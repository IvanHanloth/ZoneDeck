use windows::Win32::Media::Audio::{
    AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
    ISimpleAudioVolume, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::core::{Interface, Result};

use crate::platform::win32::process_path;

pub fn set_mute(pid: u32, mute: bool) {
    if pid == 0 {
        return;
    }
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let should_uninit = hr.is_ok();
        if let Err(e) = mute_matching_sessions(pid, mute) {
            let action = if mute { "静音" } else { "取消静音" };
            crate::log_warn!(
                "{action}失败，该进程的声音未受影响 (pid={pid}): {}",
                crate::util::win_err(&e)
            );
        }
        if should_uninit {
            CoUninitialize();
        }
    }
}

unsafe fn mute_matching_sessions(pid: u32, mute: bool) -> Result<()> {
    unsafe {
        let target_path = process_path(pid);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let sessions = manager.GetSessionEnumerator()?;
        let count = sessions.GetCount()?;

        for i in 0..count {
            let control = sessions.GetSession(i)?;
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                continue;
            };
            let session_pid = control2.GetProcessId().unwrap_or(0);
            if session_pid == 0 {
                continue;
            }

            let same_process = session_pid == pid;
            let same_executable =
                !target_path.is_empty() && process_path(session_pid) == target_path;

            if (same_process || same_executable)
                && let Ok(volume) = control.cast::<ISimpleAudioVolume>()
            {
                let _ = volume.SetMute(mute, std::ptr::null());
            }
        }
        Ok(())
    }
}

/// 默认播放设备上是否有音频会话正处于「活动播放」状态。
/// 媒体「播放/暂停」是切换键，没在播放时发送反而会把播放器切成播放。
pub fn is_audio_playing() -> bool {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let should_uninit = hr.is_ok();
        // 查不出播放状态时按「没在播放」处理。
        let playing = match any_active_session() {
            Ok(playing) => playing,
            Err(e) => {
                crate::logging::debug(&format!(
                    "枚举音频会话失败，本次按「无音频播放」处理，不发送媒体暂停键: {}",
                    crate::util::win_err(&e)
                ));
                false
            }
        };
        if should_uninit {
            CoUninitialize();
        }
        playing
    }
}

unsafe fn any_active_session() -> Result<bool> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let sessions = manager.GetSessionEnumerator()?;
        let count = sessions.GetCount()?;

        for i in 0..count {
            let control = sessions.GetSession(i)?;
            if control.GetState()? == AudioSessionStateActive {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_mute_on_process_without_audio_session_is_a_noop() {
        let pid = std::process::id();
        set_mute(pid, true);
        set_mute(pid, false);
    }

    #[test]
    fn set_mute_on_zero_pid_returns_immediately() {
        set_mute(0, true);
    }

    #[test]
    fn is_audio_playing_does_not_panic() {
        // 环境不确定，只验证能安全求值。
        let _ = is_audio_playing();
    }
}
