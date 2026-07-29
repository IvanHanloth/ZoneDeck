use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{audio, freeze, input, log_warn, logging};

/// 隐藏 / 恢复的副作用（静音、冻结、暂停键）。
///
/// 实现可以是异步的（如 [`crate::effects_worker::AsyncEffects`]）：调用方
/// 不得依赖「方法返回即动作已生效」，只能依赖调用顺序与执行顺序一致（FIFO）。
pub trait Effects {
    fn mute(&self, pid: u32, mute: bool);
    /// 冻结整批进程前静置一次，见 [`FREEZE_SETTLE_DELAY`]。
    fn settle_before_freeze(&self);
    fn suspend(&self, pid: u32, enhanced: bool);
    fn resume(&self, pid: u32, enhanced: bool);
    /// 发送媒体「播放/暂停」键，仅在检测到有音视频正在播放时才发送。检测由实现负责。
    fn send_pause(&self);
}

/// 冻结前的静置时长。
///
/// 冻结让进程彻底停止响应消息：隐藏动作没画完就冻结会留下残影，已发出的媒体
/// 暂停键也需要这段时间被目标程序处理掉。静音走音频会话，不受影响，故不等。
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
            // 不升级为 error：身份校验后仍可能竞态（解冻前进程恰好退出）。
            Err(e) => log_warn!("解冻失败 (pid={pid}): {e}"),
        }
    }

    fn send_pause(&self) {
        // 没有音视频在播放时不发键，避免把静止的播放器切成播放。
        if audio::is_audio_playing() {
            input::send_media_pause();
        }
    }
}
