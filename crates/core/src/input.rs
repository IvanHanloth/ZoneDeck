use windows::Win32::UI::Input::KeyboardAndMouse::{
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, keybd_event,
};

const VK_MEDIA_STOP: u8 = 0xB2;

pub fn send_media_stop() {
    unsafe {
        keybd_event(VK_MEDIA_STOP, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_MEDIA_STOP, 0, KEYEVENTF_KEYUP, 0);
    }
}
