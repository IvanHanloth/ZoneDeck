//! 系统媒体传输控件（SMTC）会话：按应用精确暂停，不发全局媒体键。
//!
use std::collections::BTreeSet;
use std::sync::mpsc::channel;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::Result;

use crate::{log_warn, logging};

/// 等待 SMTC 的上限。它走跨进程 RPC，而 WinRT 的等待本身是无限阻塞的；
/// 卡在这里会连累副作用线程后面排队的解冻，必须有上限。
const SMTC_TIMEOUT: Duration = Duration::from_millis(1500);

/// 本次运行由本程序暂停的会话（按 AUMID 记）。只有记在这里的才会被重新播放，
/// 用户自己暂停的不受影响。记账随进程走，崩溃恢复后为空——那时不再续播。
static PAUSED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// 记账失败不该拖垮暂停，锁中毒后照常沿用里面的数据。
fn paused_set() -> MutexGuard<'static, BTreeSet<String>> {
    PAUSED.lock().unwrap_or_else(|e| e.into_inner())
}

/// 在临时线程里跑一次 SMTC 操作并限时等待。WinRT 的等待是无限阻塞的，
/// 卡住会连累副作用线程后面排队的解冻，只能靠丢开线程来兜超时。
fn run_limited<T: Default + Send + 'static>(
    what: &'static str,
    job: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new()
        .name("zonedeck-smtc".into())
        .spawn(move || {
            let _ = tx.send(job());
        });
    if let Err(e) = spawned {
        log_warn!("无法启动媒体会话线程，本次跳过{what}: {e}");
        return T::default();
    }
    match rx.recv_timeout(SMTC_TIMEOUT) {
        Ok(out) => out,
        Err(_) => {
            log_warn!("媒体会话在 {SMTC_TIMEOUT:?} 内没有响应，本次跳过{what}");
            T::default()
        }
    }
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

/// 取出默认播放设备上的全部媒体会话。
fn sessions() -> Result<Vec<GlobalSystemMediaTransportControlsSession>> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
    let view = manager.GetSessions()?;
    let count = view.Size()?;
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        out.push(view.GetAt(i)?);
    }
    Ok(out)
}

/// 暂停映像路径命中 `paths` 的媒体会话，返回真正被暂停的映像路径。
///
/// 只动正在播放的会话：已暂停的再暂停一次没有意义，也不该由本程序改变它的状态。
/// 整个调用扔进临时线程并限时等待，SMTC 无响应时按「一个都没暂停」处理，
/// 由调用方回退到全局媒体键。
pub fn pause_sessions(paths: &[String]) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }
    let owned: Vec<String> = paths.to_vec();
    run_limited("暂停媒体", move || {
        with_com(|| match pause_matching(&owned) {
            Ok(paused) => paused,
            Err(e) => {
                logging::debug(&format!(
                    "枚举媒体会话失败，本次改用全局媒体键: {}",
                    crate::util::win_err(&e)
                ));
                Vec::new()
            }
        })
    })
}

fn pause_matching(paths: &[String]) -> Result<Vec<String>> {
    let mut paused: Vec<String> = Vec::new();
    for session in sessions()? {
        let aumid = session.SourceAppUserModelId()?.to_string();
        let Some(path) = paths.iter().find(|p| same_app(&aumid, p)) else {
            continue;
        };
        if session.GetPlaybackInfo()?.PlaybackStatus()?
            != GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
        {
            continue;
        }
        if session.TryPauseAsync()?.join()? {
            logging::debug(&format!("已暂停媒体会话 (aumid={aumid})"));
            paused_set().insert(aumid);
            if !paused.contains(path) {
                paused.push(path.clone());
            }
        }
    }
    Ok(paused)
}

/// 重新播放此前由本程序暂停的会话，只动映像路径命中 `paths` 的那些。返回续播成功的会话数。
///
/// 三道闸：会话得在记账里（本程序暂停过它）、映像要对得上、且当下仍是暂停态。
/// 用户在隐藏期间自己点了播放或切了别的内容，就不该再被本程序摆布。
pub fn resume_sessions(paths: &[String]) -> usize {
    if paths.is_empty() || paused_set().is_empty() {
        return 0;
    }
    let owned: Vec<String> = paths.to_vec();
    run_limited("续播媒体", move || {
        with_com(|| match resume_matching(&owned) {
            Ok(resumed) => resumed,
            Err(e) => {
                logging::debug(&format!(
                    "枚举媒体会话失败，本次不续播: {}",
                    crate::util::win_err(&e)
                ));
                0
            }
        })
    })
}

fn resume_matching(paths: &[String]) -> Result<usize> {
    let mut resumed = 0;
    for session in sessions()? {
        let aumid = session.SourceAppUserModelId()?.to_string();
        let ours = paused_set().contains(&aumid);
        if !ours || !paths.iter().any(|p| same_app(&aumid, p)) {
            continue;
        }
        if session.GetPlaybackInfo()?.PlaybackStatus()?
            != GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused
        {
            continue;
        }
        if session.TryPlayAsync()?.join()? {
            logging::debug(&format!("已续播媒体会话 (aumid={aumid})"));
            paused_set().remove(&aumid);
            resumed += 1;
        }
    }
    Ok(resumed)
}

/// 丢掉暂停记账。不打算续播时得清掉，免得下一轮隐藏把陈年记录也播出来。
pub fn forget_paused() {
    paused_set().clear();
}

/// AUMID 与映像路径是否指同一个程序。
///
/// Win32 程序注册的 AUMID 多为 exe 全路径或 exe 名，UWP 则是包族名——后者与映像路径
/// 对不上，只能放过，由调用方回退到全局媒体键。
fn same_app(aumid: &str, path: &str) -> bool {
    if aumid.is_empty() || path.is_empty() {
        return false;
    }
    if aumid.eq_ignore_ascii_case(path) {
        return true;
    }
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|exe| aumid.eq_ignore_ascii_case(exe))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aumid_matches_full_path_and_bare_exe_name() {
        assert!(same_app(
            "C:\\Spotify\\Spotify.exe",
            "C:\\Spotify\\Spotify.exe"
        ));
        assert!(same_app("Spotify.exe", "C:\\Spotify\\Spotify.exe"));
        // 大小写不该影响判定：AUMID 与路径的大小写由各程序自己写，不保证一致。
        assert!(same_app("spotify.exe", "C:\\Spotify\\Spotify.exe"));
    }

    #[test]
    fn aumid_does_not_match_a_different_program() {
        assert!(!same_app("Spotify.exe", "C:\\Music\\foobar2000.exe"));
        // UWP 的包族名对不上映像路径，须判为不匹配而不是误伤。
        assert!(!same_app(
            "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic",
            "C:\\Music\\foobar2000.exe"
        ));
    }

    #[test]
    fn empty_sides_never_match() {
        assert!(!same_app("", "C:\\Spotify\\Spotify.exe"));
        assert!(!same_app("Spotify.exe", ""));
    }

    #[test]
    fn pause_with_no_targets_does_not_touch_the_system() {
        assert!(pause_sessions(&[]).is_empty());
    }
}
