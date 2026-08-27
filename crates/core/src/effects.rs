use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::stats::PowerStatsStore;
use crate::{audio, efficiency, freeze, input, log_warn, logging, media};

/// 一个要暂停媒体播放的目标。`path` 查不到时为空，那时只能按 PID 判断出声。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PauseTarget {
    pub pid: u32,
    pub path: String,
}

/// 隐藏 / 恢复的副作用（静音、冻结、暂停媒体）。
/// 实现可以是异步的，调用方只能依赖调用顺序与执行顺序一致（FIFO）。
pub trait Effects {
    /// 静音进程及同映像的会话；`path` 为目标的映像路径。
    fn mute(&self, pid: u32, path: &str);
    /// 取消静音。`path` 是静音时记下的映像路径，目标进程已退出时靠它找回残留会话。
    fn unmute(&self, pid: u32, path: &str);
    /// 冻结整批进程前静置一次，见 [`FREEZE_SETTLE_DELAY`]。
    fn settle_before_freeze(&self);
    fn suspend(&self, pid: u32, enhanced: bool);
    fn resume(&self, pid: u32, enhanced: bool);
    /// 清空进程工作集，压低其内存占用；只对已挂起的进程有意义，须排在
    /// [`Effects::suspend`] 之后。
    fn trim_working_set(&self, pid: u32);
    /// 把进程降到效率模式（EcoQoS + 低优先级），进程继续运行但只吃能效核心。
    fn set_efficiency(&self, pid: u32);
    /// 撤销效率模式。无状态，重复调用无副作用。
    fn clear_efficiency(&self, pid: u32);
    /// 暂停目标的媒体播放。优先逐个会话精确暂停，够不着的才回退全局媒体键，
    /// 且仅在目标确实在出声时才发。
    fn pause_media(&self, targets: &[PauseTarget]);
    /// 续播隐藏时暂停过的媒体。只在隐藏那一刻就定下要续播时才调用，
    /// 且只动本程序暂停过、如今仍是暂停态的会话。
    fn resume_media(&self, targets: &[PauseTarget]);
    /// 丢掉暂停记账。这一轮不打算续播时调用，免得记录留到下一轮。
    fn forget_paused_media(&self) {}
    /// 把攒下的能效统计落盘。由 [`crate::effects_worker::EffectsWorker`] 在队列排空后
    /// 调用：一次隐藏会连着上报十几个进程，合并成一次写盘。不记账的实现无事可做。
    fn flush_stats(&self) {}
}

/// 目标里去重后的映像路径，空路径不计。
fn distinct_paths(targets: &[PauseTarget]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for t in targets.iter().filter(|t| !t.path.is_empty()) {
        if !paths
            .iter()
            .any(|p: &String| p.as_str().eq_ignore_ascii_case(&t.path))
        {
            paths.push(t.path.clone());
        }
    }
    paths
}

/// 冻结前的静置时长：留给隐藏动作画完、媒体暂停命令被目标程序处理掉。
const FREEZE_SETTLE_DELAY: Duration = Duration::from_millis(200);

pub struct WinEffects {
    exe_dir: PathBuf,
    /// 能效成绩单；只在副作用真正生效时记账。
    stats: Arc<PowerStatsStore>,
    /// 「已开增强冻结但缺 pssuspend」是否已记过，每次运行只记一条。
    missing_tool_logged: AtomicBool,
}

impl WinEffects {
    pub fn new(exe_dir: PathBuf, stats: Arc<PowerStatsStore>) -> Self {
        Self {
            exe_dir,
            stats,
            missing_tool_logged: AtomicBool::new(false),
        }
    }

    /// 增强冻结是否可用；因缺少 pssuspend 而不可用时，每次运行提醒一次。
    fn enhanced_ready(&self, enhanced: bool) -> bool {
        if !enhanced {
            return false;
        }
        if freeze::pssuspend_available(&self.exe_dir) {
            return true;
        }
        if !self.missing_tool_logged.swap(true, Ordering::Relaxed) {
            log_warn!(
                "已启用增强冻结，但核心所在目录下没有 {}，本次运行一律改用普通冻结",
                freeze::PSSUSPEND_EXE
            );
        }
        false
    }
}

impl Effects for WinEffects {
    fn mute(&self, pid: u32, path: &str) {
        audio::mute(pid, path);
    }

    fn unmute(&self, pid: u32, path: &str) {
        audio::unmute(pid, path);
    }

    fn settle_before_freeze(&self) {
        std::thread::sleep(FREEZE_SETTLE_DELAY);
    }

    fn suspend(&self, pid: u32, enhanced: bool) {
        if self.enhanced_ready(enhanced) {
            match freeze::suspend_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::debug(&format!("增强冻结成功 (pid={pid})"));
                    self.stats.on_suspend(pid);
                    return;
                }
                Err(e) => log_warn!("增强冻结失败，回退普通冻结 (pid={pid}): {e}"),
            }
        }
        match freeze::suspend_process(pid) {
            Ok(()) => {
                logging::debug(&format!("普通冻结成功 (pid={pid})"));
                self.stats.on_suspend(pid);
            }
            Err(e) => log_warn!("冻结失败，该进程未被冻结 (pid={pid}): {e}"),
        }
    }

    fn resume(&self, pid: u32, enhanced: bool) {
        if self.enhanced_ready(enhanced) {
            match freeze::resume_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::debug(&format!("增强解冻成功 (pid={pid})"));
                    self.stats.on_resume(pid);
                    return;
                }
                Err(e) => log_warn!("增强解冻失败，回退普通解冻 (pid={pid}): {e}"),
            }
        }
        match freeze::resume_process(pid) {
            Ok(()) => {
                logging::debug(&format!("普通解冻成功 (pid={pid})"));
                self.stats.on_resume(pid);
            }
            // 不升级为 error：身份校验后仍可能竞态。
            Err(e) => log_warn!("解冻失败 (pid={pid}): {e}"),
        }
    }

    fn trim_working_set(&self, pid: u32) {
        // 清空前后各量一次，差值即真正换出去的物理内存；读不到就只是不记账。
        let before = freeze::working_set(pid);
        match freeze::trim_working_set(pid) {
            Ok(()) => {
                logging::debug(&format!("已清空工作集 (pid={pid})"));
                if let (Some(before), Some(after)) = (before, freeze::working_set(pid)) {
                    self.stats.on_trim(before.saturating_sub(after));
                }
            }
            // 不升级为 error：受保护进程拿不到 PROCESS_SET_QUOTA 是可预期的。
            Err(e) => log_warn!("清空工作集失败，该进程的内存占用不会下降 (pid={pid}): {e}"),
        }
    }

    fn pause_media(&self, targets: &[PauseTarget]) {
        if targets.is_empty() {
            return;
        }
        let paths = distinct_paths(targets);

        // 先走 SMTC 精确暂停：只停目标自己的会话，不惊动别的播放器。
        let paused = media::pause_sessions(&paths);

        // 剩下的够不着 SMTC——没注册媒体会话，或 AUMID 与映像对不上。
        // 只能回退全局媒体键，且仅在它们确实在出声时才发：那个键是播放/暂停
        // 切换键，目标没在播时发过去反而会把它切成播放。
        let rest: Vec<&PauseTarget> = targets
            .iter()
            .filter(|t| {
                !paused
                    .iter()
                    .any(|p: &String| p.as_str().eq_ignore_ascii_case(&t.path))
            })
            .collect();
        if rest.is_empty() {
            return;
        }
        let rest_pids: Vec<u32> = rest.iter().map(|t| t.pid).collect();
        let rest_paths: Vec<String> = rest
            .iter()
            .filter(|t| !t.path.is_empty())
            .map(|t| t.path.clone())
            .collect();
        if audio::any_target_playing(&rest_pids, &rest_paths) {
            input::send_media_pause();
        }
    }

    fn resume_media(&self, targets: &[PauseTarget]) {
        let paths = distinct_paths(targets);
        if paths.is_empty() {
            return;
        }
        // 只走 SMTC：全局媒体键既认不准目标，又可能把没在播的切成播放。
        // 续播是锦上添花，不值得冒那个险，够不着就算了。
        let resumed = media::resume_sessions(&paths);
        if resumed > 0 {
            logging::debug(&format!("已续播 {resumed} 个媒体会话"));
        }
    }

    fn forget_paused_media(&self) {
        media::forget_paused();
    }

    fn set_efficiency(&self, pid: u32) {
        match efficiency::enable(pid) {
            Ok(()) => {
                logging::debug(&format!("已开启效率模式 (pid={pid})"));
                self.stats.on_efficiency_on(pid);
            }
            // 不升级为 error：拿不到 PROCESS_SET_INFORMATION 是可预期的。
            Err(e) => log_warn!("开启效率模式失败，该进程的能耗不会下降 (pid={pid}): {e}"),
        }
    }

    fn clear_efficiency(&self, pid: u32) {
        match efficiency::disable(pid) {
            Ok(()) => {
                logging::debug(&format!("已撤销效率模式 (pid={pid})"));
                self.stats.on_efficiency_off(pid);
            }
            Err(e) => log_warn!("撤销效率模式失败 (pid={pid}): {e}"),
        }
    }

    fn flush_stats(&self) {
        self.stats.flush();
    }
}
