use std::path::PathBuf;

use crate::{audio, freeze, input, logging};

pub trait Effects {
    fn mute(&self, pid: u32, mute: bool);
    fn suspend(&self, pid: u32, enhanced: bool);
    fn resume(&self, pid: u32, enhanced: bool);
    /// 隐藏前发送媒体「播放/暂停」键。返回是否真的发送了
    /// （仅在检测到有音视频正在播放时才发送）。
    fn send_pause(&self) -> bool;
}

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

    fn suspend(&self, pid: u32, enhanced: bool) {
        // 每一次冻结调用（含增强/普通、成功/失败）都以 info 写日志，便于定位。
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
            match freeze::suspend_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::info(&format!("增强冻结成功 (pid={pid})"));
                    return;
                }
                Err(e) => logging::warn(&format!(
                    "增强冻结失败，回退普通冻结 (pid={pid}): {e}"
                )),
            }
        }
        match freeze::suspend_process(pid) {
            Ok(()) => logging::info(&format!("普通冻结成功 (pid={pid})")),
            Err(e) => logging::warn(&format!("冻结进程失败 (pid={pid}): {e}")),
        }
    }

    fn resume(&self, pid: u32, enhanced: bool) {
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
            match freeze::resume_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::info(&format!("增强解冻成功 (pid={pid})"));
                    return;
                }
                Err(e) => logging::warn(&format!(
                    "增强解冻失败，回退普通解冻 (pid={pid}): {e}"
                )),
            }
        }
        match freeze::resume_process(pid) {
            Ok(()) => logging::info(&format!("普通解冻成功 (pid={pid})")),
            Err(e) => logging::warn(&format!("解冻进程失败 (pid={pid}): {e}")),
        }
    }

    fn send_pause(&self) -> bool {
        // 没有音视频在播放时不发键，避免把静止的播放器切成播放。
        if audio::is_audio_playing() {
            input::send_media_pause();
            true
        } else {
            false
        }
    }
}
