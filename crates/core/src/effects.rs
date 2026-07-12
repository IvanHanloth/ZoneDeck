use std::path::PathBuf;

use crate::{audio, freeze, input};

pub trait Effects {
    fn mute(&self, pid: u32, mute: bool);
    fn suspend(&self, pid: u32, enhanced: bool);
    fn resume(&self, pid: u32, enhanced: bool);
    fn send_pause(&self);
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
        if enhanced
            && freeze::pssuspend_available(&self.exe_dir)
            && freeze::suspend_enhanced(&self.exe_dir, pid).is_ok()
        {
            return;
        }
        if let Err(e) = freeze::suspend_process(pid) {
            eprintln!("冻结进程失败 (pid={pid}): {e}");
        }
    }

    fn resume(&self, pid: u32, enhanced: bool) {
        if enhanced
            && freeze::pssuspend_available(&self.exe_dir)
            && freeze::resume_enhanced(&self.exe_dir, pid).is_ok()
        {
            return;
        }
        if let Err(e) = freeze::resume_process(pid) {
            eprintln!("解冻进程失败 (pid={pid}): {e}");
        }
    }

    fn send_pause(&self) {
        input::send_media_stop();
    }
}
