use windows::Win32::UI::Input::KeyboardAndMouse::{
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, keybd_event,
};

/// 媒体「播放/暂停」键的虚拟键码；主流播放器普遍只响应这个硬件媒体键。
const VK_MEDIA_PLAY_PAUSE: u8 = 0xB3;

/// 模拟按下一次「媒体 播放/暂停」键，用于隐藏前暂停正在播放的音视频。
pub fn send_media_pause() {
    unsafe {
        keybd_event(VK_MEDIA_PLAY_PAUSE, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_MEDIA_PLAY_PAUSE, 0, KEYEVENTF_KEYUP, 0);
    }
}
