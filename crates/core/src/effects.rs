use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{audio, efficiency, freeze, input, log_warn, logging};

/// 隐藏 / 恢复的副作用（静音、冻结、暂停键）。
/// 实现可以是异步的，调用方只能依赖调用顺序与执行顺序一致（FIFO）。
pub trait Effects {
    fn mute(&self, pid: u32, mute: bool);
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
    /// 发送媒体「播放/暂停」键，仅在检测到有音视频正在播放时才发送。
    fn send_pause(&self);
}

/// 冻结前的静置时长：留给隐藏动作画完、媒体暂停键被目标程序处理掉。
const FREEZE_SETTLE_DELAY: Duration = Duration::from_millis(200);

pub struct WinEffects {
    exe_dir: PathBuf,
    /// 「已开增强冻结但缺 pssuspend」是否已记过，每次运行只记一条。
    missing_tool_logged: AtomicBool,
}

impl WinEffects {
    pub fn new(exe_dir: PathBuf) -> Self {
        Self {
            exe_dir,
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
    fn mute(&self, pid: u32, mute: bool) {
        audio::set_mute(pid, mute);
    }

    fn settle_before_freeze(&self) {
        std::thread::sleep(FREEZE_SETTLE_DELAY);
    }

    fn suspend(&self, pid: u32, enhanced: bool) {
        if self.enhanced_ready(enhanced) {
            match freeze::suspend_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::debug(&format!("增强冻结成功 (pid={pid})"));
                    return;
                }
                Err(e) => log_warn!("增强冻结失败，回退普通冻结 (pid={pid}): {e}"),
            }
        }
        match freeze::suspend_process(pid) {
            Ok(()) => logging::debug(&format!("普通冻结成功 (pid={pid})")),
            Err(e) => log_warn!("冻结失败，该进程未被冻结 (pid={pid}): {e}"),
        }
    }

    fn resume(&self, pid: u32, enhanced: bool) {
        if self.enhanced_ready(enhanced) {
            match freeze::resume_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::debug(&format!("增强解冻成功 (pid={pid})"));
                    return;
                }
                Err(e) => log_warn!("增强解冻失败，回退普通解冻 (pid={pid}): {e}"),
            }
        }
        match freeze::resume_process(pid) {
            Ok(()) => logging::debug(&format!("普通解冻成功 (pid={pid})")),
            // 不升级为 error：身份校验后仍可能竞态。
            Err(e) => log_warn!("解冻失败 (pid={pid}): {e}"),
        }
    }

    fn trim_working_set(&self, pid: u32) {
        match freeze::trim_working_set(pid) {
            Ok(()) => logging::debug(&format!("已清空工作集 (pid={pid})")),
            // 不升级为 error：受保护进程拿不到 PROCESS_SET_QUOTA 是可预期的。
            Err(e) => log_warn!("清空工作集失败，该进程的内存占用不会下降 (pid={pid}): {e}"),
        }
    }

    fn send_pause(&self) {
        // 没有音视频在播放时不发键。
        if audio::is_audio_playing() {
            input::send_media_pause();
        }
    }

    fn set_efficiency(&self, pid: u32) {
        match efficiency::enable(pid) {
            Ok(()) => logging::debug(&format!("已开启效率模式 (pid={pid})")),
            // 不升级为 error：拿不到 PROCESS_SET_INFORMATION 是可预期的。
            Err(e) => log_warn!("开启效率模式失败，该进程的能耗不会下降 (pid={pid}): {e}"),
        }
    }

    fn clear_efficiency(&self, pid: u32) {
        match efficiency::disable(pid) {
            Ok(()) => logging::debug(&format!("已撤销效率模式 (pid={pid})")),
            Err(e) => log_warn!("撤销效率模式失败 (pid={pid}): {e}"),
        }
    }
}
