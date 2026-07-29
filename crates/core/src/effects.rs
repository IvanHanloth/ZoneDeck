use std::path::PathBuf;
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
/// 冻结让进程彻底停止响应消息：隐藏动作若还没在屏幕上画完就冻结，被冻结的窗口
/// 会留下残影。发出去的媒体暂停键同样需要这段时间被目标程序处理掉，冻结早了就收不到。
///
/// 只在冻结前等，静音不受影响——静音走音频会话，与目标进程是否在跑无关。
const FREEZE_SETTLE_DELAY: Duration = Duration::from_millis(200);

pub struct WinEffects {
    exe_dir: PathBuf,
}

impl WinEffects {
    pub fn new(exe_dir: PathBuf) -> Self {
        Self { exe_dir }
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
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
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
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
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
