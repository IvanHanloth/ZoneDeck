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
        // 每一次冻结调用都写日志：成功记 info，失败记 warn，便于事后按 pid 追溯。
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
            match freeze::suspend_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::info(&format!("增强冻结成功 (pid={pid})"));
                    return;
                }
                Err(e) => logging::warn(&format!("增强冻结失败，回退普通冻结 (pid={pid}): {e}")),
            }
        }
        match freeze::suspend_process(pid) {
            Ok(()) => logging::info(&format!("普通冻结成功 (pid={pid})")),
            Err(e) => logging::warn(&format!("普通冻结失败，该进程未被冻结 (pid={pid}): {e}")),
        }
    }

    fn resume(&self, pid: u32, enhanced: bool) {
        if enhanced && freeze::pssuspend_available(&self.exe_dir) {
            match freeze::resume_enhanced(&self.exe_dir, pid) {
                Ok(()) => {
                    logging::info(&format!("增强解冻成功 (pid={pid})"));
                    return;
                }
                Err(e) => logging::warn(&format!("增强解冻失败，回退普通解冻 (pid={pid}): {e}")),
            }
        }
        match freeze::resume_process(pid) {
            Ok(()) => logging::info(&format!("普通解冻成功 (pid={pid})")),
            // 不升级为 error：进程在隐藏期间正常退出时同样会走到这里（OpenProcess 失败），
            // 那是常态而非故障。具体是「已退出」还是「权限不足」由错误码分辨。
            Err(e) => logging::warn(&format!("普通解冻失败 (pid={pid}): {e}")),
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
