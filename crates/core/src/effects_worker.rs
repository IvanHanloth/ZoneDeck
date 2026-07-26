//! 副作用专职线程：把静音 / 冻结 / 暂停键等慢操作挪出窗口消息循环。
//! 任务经 [`AsyncEffects`] 入队，由单线程按 FIFO 顺序执行；通道无界，事件不丢。

use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::effects::Effects;
use crate::{log_error, log_warn};

enum Task {
    Mute { pid: u32, mute: bool },
    Suspend { pid: u32, enhanced: bool },
    Resume { pid: u32, enhanced: bool },
    SendPause,
    Quit,
}

/// 副作用线程句柄。`shutdown` 排干队列后退出，保证退出前解冻 / 取消静音已生效。
pub struct EffectsWorker {
    tx: Sender<Task>,
    handle: Option<JoinHandle<()>>,
}

impl EffectsWorker {
    /// 启动专职线程；`inner` 是真正执行副作用的实现（生产为 `WinEffects`）。
    pub fn spawn<E: Effects + Send + 'static>(inner: E) -> Self {
        let (tx, rx) = channel::<Task>();
        let handle = std::thread::Builder::new()
            .name("bosskey-effects".into())
            .spawn(move || {
                while let Ok(task) = rx.recv() {
                    match task {
                        Task::Mute { pid, mute } => inner.mute(pid, mute),
                        Task::Suspend { pid, enhanced } => inner.suspend(pid, enhanced),
                        Task::Resume { pid, enhanced } => inner.resume(pid, enhanced),
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
                log_warn!("副作用线程未在 {timeout:?} 内排干队列，放弃等待");
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = handle.join();
    }
}

/// 把每个副作用调用转成任务入队的 [`Effects`] 实现。克隆廉价。
#[derive(Clone)]
pub struct AsyncEffects {
    tx: Sender<Task>,
}

impl AsyncEffects {
    fn send(&self, task: Task) {
        if self.tx.send(task).is_err() {
            log_error!("副作用线程已退出，本次副作用未执行");
        }
    }
}

impl Effects for AsyncEffects {
    fn mute(&self, pid: u32, mute: bool) {
        self.send(Task::Mute { pid, mute });
    }
    fn suspend(&self, pid: u32, enhanced: bool) {
        self.send(Task::Suspend { pid, enhanced });
    }
    fn resume(&self, pid: u32, enhanced: bool) {
        self.send(Task::Resume { pid, enhanced });
    }
    fn send_pause(&self) {
        self.send(Task::SendPause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// 线程安全的记录用 Effects：按调用顺序记下每个动作。
    #[derive(Clone, Default)]
    struct Recorder {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Effects for Recorder {
        fn mute(&self, pid: u32, mute: bool) {
            self.calls.lock().unwrap().push(format!("mute:{pid}:{mute}"));
        }
        fn suspend(&self, pid: u32, _enhanced: bool) {
            self.calls.lock().unwrap().push(format!("suspend:{pid}"));
        }
        fn resume(&self, pid: u32, _enhanced: bool) {
            self.calls.lock().unwrap().push(format!("resume:{pid}"));
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

        // 暂停键必须先于冻结执行（冻结后的进程收不到按键）。
        effects.send_pause();
        effects.mute(100, true);
        effects.suspend(100, false);
        effects.resume(100, false);

        worker.shutdown(Duration::from_secs(5));

        assert_eq!(
            *recorder.calls.lock().unwrap(),
            vec!["pause", "mute:100:true", "suspend:100", "resume:100"],
            "任务应按入队顺序全部执行完毕（shutdown 排干队列）"
        );
    }

    #[test]
    fn send_after_shutdown_does_not_panic() {
        let worker = EffectsWorker::spawn(Recorder::default());
        let effects = worker.effects();
        worker.shutdown(Duration::from_secs(5));
        // 线程已退出，入队应静默失败（记日志）而非 panic。
        effects.mute(1, true);
        effects.send_pause();
    }
}
