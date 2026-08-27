//! 副作用专职线程：把静音 / 冻结 / 暂停键等慢操作挪出窗口消息循环。
//! 任务经 [`AsyncEffects`] 入队，由单线程按 FIFO 顺序执行。

use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::effects::{Effects, PauseTarget};
use crate::{log_error, log_warn};

/// 队列静置多久就把能效统计落盘。一次隐藏 / 恢复的十几个任务连着来，
/// 排空后写一次即可；[`crate::stats`] 自己的节流会漏掉末尾那几笔，得有人补。
const FLUSH_IDLE: Duration = Duration::from_secs(2);

enum Task {
    Mute { pid: u32, path: String },
    Unmute { pid: u32, path: String },
    SettleBeforeFreeze,
    Suspend { pid: u32, enhanced: bool },
    Resume { pid: u32, enhanced: bool },
    TrimWorkingSet { pid: u32 },
    SetEfficiency { pid: u32 },
    ClearEfficiency { pid: u32 },
    PauseMedia { targets: Vec<PauseTarget> },
    ResumeMedia { targets: Vec<PauseTarget> },
    ForgetPausedMedia,
    Quit,
}

impl Task {
    /// 日志里指代该任务的写法，含目标进程。
    fn describe(&self) -> String {
        match self {
            Task::Mute { pid, .. } => format!("静音 (pid={pid})"),
            Task::Unmute { pid, .. } => format!("取消静音 (pid={pid})"),
            Task::SettleBeforeFreeze => "冻结前静置".to_string(),
            Task::Suspend { pid, enhanced } => format!("冻结 (pid={pid}, 增强={enhanced})"),
            Task::Resume { pid, enhanced } => format!("解冻 (pid={pid}, 增强={enhanced})"),
            Task::TrimWorkingSet { pid } => format!("清空工作集 (pid={pid})"),
            Task::SetEfficiency { pid } => format!("开启效率模式 (pid={pid})"),
            Task::ClearEfficiency { pid } => format!("撤销效率模式 (pid={pid})"),
            Task::PauseMedia { targets } => format!("暂停媒体播放 ({} 个目标)", targets.len()),
            Task::ResumeMedia { targets } => format!("续播媒体 ({} 个目标)", targets.len()),
            Task::ForgetPausedMedia => "丢弃媒体暂停记账".to_string(),
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
                // 干过活才等超时，否则一直阻塞在 recv 上：空闲时线程完全不醒。
                let mut worked = false;
                loop {
                    let task = if worked {
                        match rx.recv_timeout(FLUSH_IDLE) {
                            Ok(task) => task,
                            Err(RecvTimeoutError::Timeout) => {
                                inner.flush_stats();
                                worked = false;
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match rx.recv() {
                            Ok(task) => task,
                            Err(_) => break,
                        }
                    };
                    match task {
                        Task::Mute { pid, path } => inner.mute(pid, &path),
                        Task::Unmute { pid, path } => inner.unmute(pid, &path),
                        Task::SettleBeforeFreeze => inner.settle_before_freeze(),
                        Task::Suspend { pid, enhanced } => inner.suspend(pid, enhanced),
                        Task::Resume { pid, enhanced } => inner.resume(pid, enhanced),
                        Task::TrimWorkingSet { pid } => inner.trim_working_set(pid),
                        Task::SetEfficiency { pid } => inner.set_efficiency(pid),
                        Task::ClearEfficiency { pid } => inner.clear_efficiency(pid),
                        Task::PauseMedia { targets } => inner.pause_media(&targets),
                        Task::ResumeMedia { targets } => inner.resume_media(&targets),
                        Task::ForgetPausedMedia => inner.forget_paused_media(),
                        Task::Quit => break,
                    }
                    worked = true;
                }
                // 收到 Quit 或发送端全没了都会走到这里，末尾那批统计不能烂在内存里。
                if worked {
                    inner.flush_stats();
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
    fn mute(&self, pid: u32, path: &str) {
        self.send(Task::Mute {
            pid,
            path: path.to_string(),
        });
    }
    fn unmute(&self, pid: u32, path: &str) {
        self.send(Task::Unmute {
            pid,
            path: path.to_string(),
        });
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
    fn set_efficiency(&self, pid: u32) {
        self.send(Task::SetEfficiency { pid });
    }
    fn clear_efficiency(&self, pid: u32) {
        self.send(Task::ClearEfficiency { pid });
    }
    fn pause_media(&self, targets: &[PauseTarget]) {
        self.send(Task::PauseMedia {
            targets: targets.to_vec(),
        });
    }
    fn resume_media(&self, targets: &[PauseTarget]) {
        self.send(Task::ResumeMedia {
            targets: targets.to_vec(),
        });
    }
    fn forget_paused_media(&self) {
        self.send(Task::ForgetPausedMedia);
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
        fn mute(&self, pid: u32, path: &str) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("mute:{pid}:{path}"));
        }
        fn unmute(&self, pid: u32, path: &str) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("unmute:{pid}:{path}"));
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
        fn set_efficiency(&self, pid: u32) {
            self.calls.lock().unwrap().push(format!("eco_on:{pid}"));
        }
        fn clear_efficiency(&self, pid: u32) {
            self.calls.lock().unwrap().push(format!("eco_off:{pid}"));
        }
        fn pause_media(&self, targets: &[PauseTarget]) {
            let pids: Vec<String> = targets.iter().map(|t| t.pid.to_string()).collect();
            self.calls
                .lock()
                .unwrap()
                .push(format!("pause:{}", pids.join(",")));
        }
        fn resume_media(&self, targets: &[PauseTarget]) {
            let pids: Vec<String> = targets.iter().map(|t| t.pid.to_string()).collect();
            self.calls
                .lock()
                .unwrap()
                .push(format!("resume_media:{}", pids.join(",")));
        }
        fn flush_stats(&self) {
            self.calls.lock().unwrap().push("flush".into());
        }
    }

    impl Recorder {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    /// 队列排空静置后要主动落盘：核心不退出也可能几小时不再有动作，
    /// 末尾那几笔统计不能一直烂在内存里。
    #[test]
    fn stats_are_flushed_once_the_queue_goes_idle() {
        let recorder = Recorder::default();
        let worker = EffectsWorker::spawn(recorder.clone());
        let effects = worker.effects();

        effects.set_efficiency(100);
        std::thread::sleep(FLUSH_IDLE + Duration::from_millis(500));
        assert_eq!(
            recorder.calls(),
            vec!["eco_on:100", "flush"],
            "静置一轮之后该落一次盘"
        );

        // 落过盘就该回到完全阻塞，没有新任务时不再重复写。
        std::thread::sleep(FLUSH_IDLE + Duration::from_millis(500));
        assert_eq!(
            recorder.calls(),
            vec!["eco_on:100", "flush"],
            "空闲时不该反复落盘"
        );

        worker.shutdown(Duration::from_secs(5));
    }

    /// 退出时队列里还压着任务，排干之后同样要落盘。
    #[test]
    fn shutdown_flushes_what_the_last_batch_recorded() {
        let recorder = Recorder::default();
        let worker = EffectsWorker::spawn(recorder.clone());
        worker.effects().clear_efficiency(100);
        worker.shutdown(Duration::from_secs(5));

        assert_eq!(recorder.calls(), vec!["eco_off:100", "flush"]);
    }

    #[test]
    fn tasks_run_in_fifo_order_and_shutdown_drains_the_queue() {
        let recorder = Recorder::default();
        let worker = EffectsWorker::spawn(recorder.clone());
        let effects = worker.effects();

        // 暂停媒体先于冻结、静置紧挨冻结前、清空工作集排在冻结之后。
        effects.pause_media(&[PauseTarget {
            pid: 100,
            path: "C:/a.exe".into(),
        }]);
        effects.mute(100, "C:/a.exe");
        effects.set_efficiency(100);
        effects.settle_before_freeze();
        effects.suspend(100, false);
        effects.trim_working_set(100);
        effects.resume(100, false);
        effects.clear_efficiency(100);
        effects.unmute(100, "C:/a.exe");

        worker.shutdown(Duration::from_secs(5));

        // 落盘不是入队的任务，本例只看副作用的先后，另有用例专管它。
        let order: Vec<String> = recorder
            .calls()
            .into_iter()
            .filter(|c| c != "flush")
            .collect();
        assert_eq!(
            order,
            vec![
                "pause:100",
                "mute:100:C:/a.exe",
                "eco_on:100",
                "settle",
                "suspend:100",
                "trim:100",
                "resume:100",
                "eco_off:100",
                "unmute:100:C:/a.exe"
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
        effects.mute(1, "");
        effects.pause_media(&[PauseTarget::default()]);
    }
}
