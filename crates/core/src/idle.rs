use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

pub fn idle_millis() -> Option<u32> {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if GetLastInputInfo(&mut info).as_bool() {
            let now = GetTickCount64() as u32;
            Some(now.wrapping_sub(info.dwTime))
        } else {
            None
        }
    }
}

pub fn should_auto_hide(idle_ms: u32, threshold_minutes: u32, currently_hidden: bool) -> bool {
    !currently_hidden
        && threshold_minutes > 0
        && u64::from(idle_ms) >= u64::from(threshold_minutes) * 60_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_millis_returns_a_value() {
        assert!(idle_millis().is_some(), "GetLastInputInfo 应返回有效值");
    }

    #[test]
    fn should_auto_hide_respects_threshold() {
        assert!(!should_auto_hide(4 * 60_000, 5, false), "未达阈值不应隐藏");
        assert!(should_auto_hide(5 * 60_000, 5, false), "达到阈值应隐藏");
        assert!(should_auto_hide(6 * 60_000, 5, false), "超过阈值应隐藏");
    }

    #[test]
    fn should_auto_hide_skips_when_already_hidden() {
        assert!(
            !should_auto_hide(10 * 60_000, 5, true),
            "已隐藏时不应重复隐藏"
        );
    }

    #[test]
    fn should_auto_hide_zero_threshold_never_triggers() {
        assert!(!should_auto_hide(u32::MAX, 0, false));
    }
}
