//! 副作用专职线程：把静音 / 冻结 / 暂停键等慢操作挪出窗口消息循环。
//! 任务经 [`AsyncEffects`] 入队，由单线程按 FIFO 顺序执行。

use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::effects::Effects;
use crate::{log_error, log_warn};

enum Task {
    Mute { pid: u32, mute: bool },
    SettleBeforeFreeze,
    Suspend { pid: u32, enhanced: bool },
    Resume { pid: u32, enhanced: bool },
    TrimWorkingSet { pid: u32 },
    SendPause,
    Quit,
}

impl Task {
    /// 日志里指代该任务的写法，含目标进程。
    fn describe(&self) -> String {
        match self {
            Task::Mute { pid, mute: true } => format!("静音 (pid={pid})"),
            Task::Mute { pid, mute: false } => format!("取消静音 (pid={pid})"),
            Task::SettleBeforeFreeze => "冻结前静置".to_string(),
            Task::Suspend { pid, enhanced } => format!("冻结 (pid={pid}, 增强={enhanced})"),
            Task::Resume { pid, enhanced } => format!("解冻 (pid={pid}, 增强={enhanced})"),
            Task::TrimWorkingSet { pid } => format!("清空工作集 (pid={pid})"),
            Task::SendPause => "发送媒体暂停键".to_string(),
            Task::Quit => "结束副作用线程".to_string(),
        }
    }
}

/// 副作用线程句柄；`shutdown` 排干队列后退出。
pub struct EffectsWorker {
    tx: Sender<Task>,
    handle: Option<JoinHandle<()>>,
}

impl EffectsWorker {
    /// 启动专职线程；`inner` 是真正执行副作用的实现。
    pub fn spawn<E: Effects + Send + 'static>(inner: E) -> Self {
        let (tx, rx) = channel::<Task>();
        let handle = std::thread::Builder::new()
            .name("zonedeck-effects".into())
            .spawn(move || {
                while let Ok(task) = rx.recv() {
                    match task {
                        Task::Mute { pid, mute } => inner.mute(pid, mute),
                        Task::SettleBeforeFreeze => inner.settle_before_freeze(),
                        Task::Suspend { pid, enhanced } => inner.suspend(pid, enhanced),
                        Task::Resume { pid, enhanced } => inner.resume(pid, enhanced),
                        Task::TrimWorkingSet { pid } => inner.trim_working_set(pid),
                        Task::SendPause => inner.send_pause(),
                        Task::Quit => break,
                    }
                }
            })
            .expect("创建副作用线程失败");
        Self {
            tx,
            handle: Some(handle),
        }
    }

    /// 供 [`crate::hide::HideController`] 使用的异步 [`Effects`] 句柄。
    pub fn effects(&self) -> AsyncEffects {
        AsyncEffects {
            tx: self.tx.clone(),
        }
    }

    /// 排干队列并结束线程；超时放弃等待并 warn。
    pub fn shutdown(mut self, timeout: Duration) {
        let _ = self.tx.send(Task::Quit);
        let Some(handle) = self.handle.take() else {
            return;
        };
        let deadline = Instant::now() + timeout;
        while !handle.is_finished() {
            if Instant::now() >= deadline {
                log_warn!(
                    "副作用线程未在 {timeout:?} 内排干队列，放弃等待；本次退出可能残留未解冻或未取消静音的进程"
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = handle.join();
    }
}

/// 把每个副作用调用转成任务入队的 [`Effects`] 实现。
#[derive(Clone)]
pub struct AsyncEffects {
    tx: Sender<Task>,
}

impl AsyncEffects {
    fn send(&self, task: Task) {
        if let Err(e) = self.tx.send(task) {
            log_error!(
                "副作用线程已退出，以下副作用未执行，相关进程可能仍处于静音或冻结状态: {}",
                e.0.describe()
            );
        }
    }
}

impl Effects for AsyncEffects {
    fn mute(&self, pid: u32, mute: bool) {
        self.send(Task::Mute { pid, mute });
    }
    fn settle_before_freeze(&self) {
        self.send(Task::SettleBeforeFreeze);
    }
    fn suspend(&self, pid: u32, enhanced: bool) {
        self.send(Task::Suspend { pid, enhanced });
    }
    fn resume(&self, pid: u32, enhanced: bool) {
        self.send(Task::Resume { pid, enhanced });
    }
    fn trim_working_set(&self, pid: u32) {
        self.send(Task::TrimWorkingSet { pid });
    }
    fn send_pause(&self) {
        self.send(Task::SendPause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// 按调用顺序记下每个动作。
    #[derive(Clone, Default)]
    struct Recorder {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Effects for Recorder {
        fn mute(&self, pid: u32, mute: bool) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("mute:{pid}:{mute}"));
        }
        fn settle_before_freeze(&self) {
            self.calls.lock().unwrap().push("settle".into());
        }
        fn suspend(&self, pid: u32, _enhanced: bool) {
            self.calls.lock().unwrap().push(format!("suspend:{pid}"));
        }
        fn resume(&self, pid: u32, _enhanced: bool) {
            self.calls.lock().unwrap().push(format!("resume:{pid}"));
        }
        fn trim_working_set(&self, pid: u32) {
            self.calls.lock().unwrap().push(format!("trim:{pid}"));
        }
        fn send_pause(&self) {
            self.calls.lock().unwrap().push("pause".into());
        }
    }

    #[test]
    fn tasks_run_in_fifo_order_and_shutdown_drains_the_queue() {
        let recorder = Recorder::default();
        let worker = EffectsWorker::spawn(recorder.clone());
        let effects = worker.effects();

        // 暂停键先于冻结、静置紧挨冻结前、清空工作集排在冻结之后。
        effects.send_pause();
        effects.mute(100, true);
        effects.settle_before_freeze();
        effects.suspend(100, false);
        effects.trim_working_set(100);
        effects.resume(100, false);

        worker.shutdown(Duration::from_secs(5));

        assert_eq!(
            *recorder.calls.lock().unwrap(),
            vec![
                "pause",
                "mute:100:true",
                "settle",
                "suspend:100",
                "trim:100",
                "resume:100"
            ],
            "任务应按入队顺序全部执行完毕（shutdown 排干队列）"
        );
    }

    #[test]
    fn send_after_shutdown_does_not_panic() {
        let worker = EffectsWorker::spawn(Recorder::default());
        let effects = worker.effects();
        worker.shutdown(Duration::from_secs(5));
        // 线程已退出，入队应静默失败而非 panic。
        effects.mute(1, true);
        effects.send_pause();
    }
}
