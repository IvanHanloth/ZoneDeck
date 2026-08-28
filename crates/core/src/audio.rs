use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use windows::Win32::Media::Audio::{
    AudioSessionStateActive, IAudioSessionControl, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::core::{Interface, Result};

use crate::platform::win32::process_path;
use crate::{log_warn, logging};

/// 本次运行静音过的会话：会话所属 pid → 当时是否真的改动了它。
/// 原本就静音的记 false，[`unmute`] 据此不去解除用户自己设的静音。
static TOUCHED: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

/// 记账失败不该拖垮静音，锁中毒后照常沿用里面的数据。
fn touched() -> MutexGuard<'static, BTreeMap<u32, bool>> {
    TOUCHED.lock().unwrap_or_else(|e| e.into_inner())
}

/// 在初始化好的 COM 单元里跑一段；本线程原本就初始化过的不重复卸载。
fn with_com<T>(f: impl FnOnce() -> T) -> T {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let should_uninit = hr.is_ok();
        let out = f();
        if should_uninit {
            CoUninitialize();
        }
        out
    }
}

/// 遍历默认播放设备上每个查得到宿主进程的会话。
unsafe fn for_each_session(
    mut visit: impl FnMut(&IAudioSessionControl, u32) -> Result<()>,
) -> Result<()> {
    unsafe {
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
            visit(&control, session_pid)?;
        }
        Ok(())
    }
}

/// 会话属于目标进程，或与目标出自同一个可执行文件。
/// 按映像匹配是必须的：Chrome 这类多进程程序的声音由子进程发出。
fn same_target(session_pid: u32, session_path: &str, pid: u32, path: &str) -> bool {
    session_pid == pid
        || (!path.is_empty() && !session_path.is_empty() && session_path.eq_ignore_ascii_case(path))
}

/// 静音目标进程及同映像的全部会话。
///
/// 原本就静音的会话不动，但照样记一笔；[`unmute`] 据此只解除本程序真正改过的那些。
pub fn mute(pid: u32, path: &str) {
    if pid == 0 {
        return;
    }
    with_com(|| {
        if let Err(e) = unsafe { apply_mute(pid, path) } {
            log_warn!(
                "静音失败，该进程的声音未受影响 (pid={pid}): {}",
                crate::util::win_err(&e)
            );
        }
    });
}

unsafe fn apply_mute(pid: u32, path: &str) -> Result<()> {
    unsafe {
        // 路径可能为空（任务栏这类补不齐身份的目标），现查一次兜底。
        let target = if path.is_empty() {
            process_path(pid)
        } else {
            path.to_string()
        };
        for_each_session(|control, session_pid| {
            let session_path = process_path(session_pid);
            if !same_target(session_pid, &session_path, pid, &target) {
                return Ok(());
            }
            let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
                return Ok(());
            };
            let already = volume.GetMute().map(|m| m.as_bool()).unwrap_or(false);
            if !already {
                volume.SetMute(true, std::ptr::null())?;
            }
            touched().insert(session_pid, !already);
            Ok(())
        })
    }
}

/// 取消静音：只解除本程序真正静音过的会话。
///
/// `path` 是静音时记下的映像路径。目标进程可能已经退出，而静音波及的是同映像的
/// 全部会话，靠它才能把剩下那些解出来。
pub fn unmute(pid: u32, path: &str) {
    with_com(|| {
        if let Err(e) = unsafe { apply_unmute(pid, path) } {
            log_warn!(
                "取消静音失败，该进程可能仍处于静音 (pid={pid}): {}",
                crate::util::win_err(&e)
            );
        }
    });
}

unsafe fn apply_unmute(pid: u32, path: &str) -> Result<()> {
    unsafe {
        for_each_session(|control, session_pid| {
            let session_path = process_path(session_pid);
            if !same_target(session_pid, &session_path, pid, path) {
                return Ok(());
            }
            // 查不到记账只可能来自崩溃恢复（上一轮的记账随进程一起没了）。
            // 那时快照里留有这条目标，本身就是「本程序静音过它」的凭据，照解不误。
            let should = touched().remove(&session_pid).unwrap_or(true);
            if !should {
                return Ok(());
            }
            if let Ok(volume) = control.cast::<ISimpleAudioVolume>() {
                volume.SetMute(false, std::ptr::null())?;
            }
            Ok(())
        })
    }
}

/// `pids` 或 `paths` 命中的进程里，有没有会话正在出声。
///
/// 判定必须限定在目标身上：拿全局「有没有声音」来决定发不发媒体暂停键，
/// 会把与本次隐藏无关的后台播放器一并停掉。
pub fn any_target_playing(pids: &[u32], paths: &[String]) -> bool {
    if pids.is_empty() && paths.is_empty() {
        return false;
    }
    with_com(|| {
        let mut playing = false;
        // 查不出播放状态时按「没在播放」处理，宁可不发也不误发。
        let result = unsafe {
            for_each_session(|control, session_pid| {
                if playing {
                    return Ok(());
                }
                let hit = pids.contains(&session_pid) || {
                    let session_path = process_path(session_pid);
                    !session_path.is_empty()
                        && paths
                            .iter()
                            .any(|p: &String| p.as_str().eq_ignore_ascii_case(&session_path))
                };
                if hit && control.GetState()? == AudioSessionStateActive {
                    playing = true;
                }
                Ok(())
            })
        };
        if let Err(e) = result {
            logging::debug(&format!(
                "枚举音频会话失败，本次按「目标没在播放」处理，不发送媒体暂停键: {}",
                crate::util::win_err(&e)
            ));
        }
        playing
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mute_on_process_without_audio_session_is_a_noop() {
        let pid = std::process::id();
        mute(pid, "");
        unmute(pid, "");
    }

    #[test]
    fn mute_on_zero_pid_returns_immediately() {
        mute(0, "");
    }

    #[test]
    fn session_matches_by_pid_or_by_image_path() {
        assert!(same_target(100, "C:\\a\\WeChat.exe", 100, ""));
        // 同一个 exe 的别的进程也算命中：多进程程序的声音由子进程发出。
        assert!(same_target(
            200,
            "C:\\a\\WeChat.exe",
            100,
            "C:\\a\\WeChat.exe"
        ));
        assert!(same_target(
            200,
            "C:\\a\\wechat.exe",
            100,
            "C:\\A\\WeChat.exe"
        ));
    }

    #[test]
    fn session_of_another_program_never_matches() {
        assert!(!same_target(200, "C:\\a\\QQ.exe", 100, "C:\\a\\WeChat.exe"));
        // 路径两边有一边查不到就只能靠 pid，不能凭空匹配。
        assert!(!same_target(200, "", 100, "C:\\a\\WeChat.exe"));
        assert!(!same_target(200, "C:\\a\\WeChat.exe", 100, ""));
    }

    #[test]
    fn playing_check_without_targets_does_not_touch_the_system() {
        assert!(!any_target_playing(&[], &[]));
    }

    #[test]
    fn playing_check_does_not_panic() {
        // 环境不确定，只验证能安全求值。
        let _ = any_target_playing(&[std::process::id()], &[]);
    }
}
