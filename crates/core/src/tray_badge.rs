//! 托盘图标状态角标：在基础图标右下角叠加一个彩色圆点，反映核心当前状态。
//!
//! 四种颜色各自绑定一个状态源（见 `TrayBadges`），多个绑定状态同时活跃时
//! 按 **红 > 绿 > 黄 > 蓝** 的优先级只显示一个圆点；置空的颜色不参与。

use std::collections::HashMap;

use bosskey_common::config::{
    TRAY_STATUS_AUTO_HIDE, TRAY_STATUS_ELEVATED, TRAY_STATUS_FREEZE, TRAY_STATUS_HIDDEN,
    TRAY_STATUS_HIDE_CURRENT, TRAY_STATUS_MONITOR_PAUSED, TrayBadges,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, GetDC, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

use crate::icon::IconRgba;

/// 角标描边：白色，用于在深浅任务栏背景上都能衬出圆点边界。
const BADGE_RING: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// 角标颜色，declaration 顺序即显示优先级（红最高）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeColor {
    Red,
    Green,
    Yellow,
    Blue,
}

impl BadgeColor {
    const fn rgba(self) -> [u8; 4] {
        match self {
            BadgeColor::Red => [0xF4, 0x43, 0x36, 0xFF],
            BadgeColor::Green => [0x4C, 0xAF, 0x50, 0xFF],
            BadgeColor::Yellow => [0xFF, 0xC1, 0x07, 0xFF],
            BadgeColor::Blue => [0x21, 0x96, 0xF3, 0xFF],
        }
    }
}

/// 核心当前的状态快照，供角标绑定判断。
#[derive(Debug, Clone, Copy, Default)]
pub struct TrayStatus {
    /// 当前有窗口被隐藏。
    pub hidden: bool,
    /// 空闲自动隐藏已启用。
    pub auto_hide: bool,
    /// 「同时隐藏当前活动窗口」已启用。
    pub hide_current: bool,
    /// 「隐藏窗口时冻结进程」已启用。
    pub freeze: bool,
    /// 核心正以管理员身份运行。
    pub elevated: bool,
    /// 热键与鼠标监控已被临时停用（见 `Command::SetHotkeys`）。
    pub monitor_paused: bool,
}

/// 某个状态源当前是否活跃；空串与未知取值一律视为不活跃。
fn status_active(key: &str, s: &TrayStatus) -> bool {
    match key {
        TRAY_STATUS_HIDDEN => s.hidden,
        TRAY_STATUS_AUTO_HIDE => s.auto_hide,
        TRAY_STATUS_HIDE_CURRENT => s.hide_current,
        TRAY_STATUS_FREEZE => s.freeze,
        TRAY_STATUS_ELEVATED => s.elevated,
        TRAY_STATUS_MONITOR_PAUSED => s.monitor_paused,
        _ => false,
    }
}

/// 按红 > 绿 > 黄 > 蓝的优先级选出当前应显示的角标颜色；都不活跃时不显示。
pub fn active_badge(badges: &TrayBadges, status: &TrayStatus) -> Option<BadgeColor> {
    [
        (BadgeColor::Red, badges.red.as_str()),
        (BadgeColor::Green, badges.green.as_str()),
        (BadgeColor::Yellow, badges.yellow.as_str()),
        (BadgeColor::Blue, badges.blue.as_str()),
    ]
    .into_iter()
    .find(|(_, key)| status_active(key, status))
    .map(|(color, _)| color)
}

/// 在顶到底 RGBA 像素的右下角叠加一个带白色描边的实心圆点。
///
/// 圆点直径约为图标边长的 44%，紧贴右下角；边缘做 1px 线性过渡抗锯齿。
/// `pixels` 长度必须等于 `width*height*4`，否则不做任何修改。
fn overlay_badge(pixels: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    if width == 0 || height == 0 || pixels.len() != (width * height * 4) as usize {
        return;
    }
    let size = width.min(height) as f32;
    let radius = (size * 0.22).max(2.5);
    let ring = (size * 0.05).max(1.0);
    let cx = width as f32 - radius - ring - 0.5;
    let cy = height as f32 - radius - ring - 0.5;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            // 覆盖率：圆内 1，圆外 0，边界 1px 内线性过渡。
            let dot = (radius + 0.5 - dist).clamp(0.0, 1.0);
            let outline = (radius + ring + 0.5 - dist).clamp(0.0, 1.0) - dot;
            if dot <= 0.0 && outline <= 0.0 {
                continue;
            }
            let i = ((y * width + x) * 4) as usize;
            let px = &mut pixels[i..i + 4];
            blend(px, BADGE_RING, outline);
            blend(px, color, dot);
        }
    }
}

/// 把 `src` 以 `coverage`（0..=1）的强度 alpha 混合到 `dst`（均为直通 alpha 的 RGBA）。
fn blend(dst: &mut [u8], src: [u8; 4], coverage: f32) {
    if coverage <= 0.0 {
        return;
    }
    let a = coverage * (src[3] as f32 / 255.0);
    for c in 0..3 {
        dst[c] = (src[c] as f32 * a + dst[c] as f32 * (1.0 - a)).round() as u8;
    }
    dst[3] = ((255.0 * a) + dst[3] as f32 * (1.0 - a)).round() as u8;
}

/// 把指定颜色的圆点叠加到基础图标像素上。
fn compose(base: &IconRgba, color: BadgeColor) -> IconRgba {
    let mut out = base.clone();
    overlay_badge(&mut out.pixels, out.width, out.height, color.rgba());
    out
}

/// 顶到底 RGBA 像素 → HICON（32 位 ARGB，直通 alpha）。
fn rgba_to_hicon(icon: &IconRgba) -> Option<HICON> {
    if icon.width == 0
        || icon.height == 0
        || icon.pixels.len() != (icon.width * icon.height * 4) as usize
    {
        return None;
    }
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: icon.width as i32,
                biHeight: -(icon.height as i32), // 负值 = 顶到底行序
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(None);
        let color = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        ReleaseDC(None, hdc);
        let color = color.ok()?;
        if bits.is_null() {
            let _ = DeleteObject(color.into());
            return None;
        }
        // RGBA → BGRA。
        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, icon.pixels.len());
        for (d, s) in dst.chunks_exact_mut(4).zip(icon.pixels.chunks_exact(4)) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            d[3] = s[3];
        }
        let mask = CreateBitmap(icon.width as i32, icon.height as i32, 1, 1, None);
        let info = ICONINFO {
            fIcon: true.into(),
            hbmMask: mask,
            hbmColor: color,
            ..Default::default()
        };
        let hicon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(color.into());
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
        hicon
    }
}

/// 基础图标 + 各颜色角标的 HICON 缓存。
///
/// 颜色数量有限（4 种），生成后缓存复用；缓存的 HICON 随进程存活，不逐个销毁。
pub struct TrayIconSet {
    base: HICON,
    base_rgba: Option<IconRgba>,
    variants: HashMap<BadgeColor, HICON>,
}

impl TrayIconSet {
    pub fn new() -> Self {
        let base = crate::tray::load_app_icon();
        Self {
            base,
            base_rgba: crate::icon::hicon_to_rgba(base),
            variants: HashMap::new(),
        }
    }

    /// 指定角标颜色的图标；无角标、像素提取或生成失败时回退基础图标。
    pub fn icon(&mut self, badge: Option<BadgeColor>) -> HICON {
        let Some(color) = badge else {
            return self.base;
        };
        if let Some(icon) = self.variants.get(&color) {
            return *icon;
        }
        let Some(base_rgba) = &self.base_rgba else {
            return self.base;
        };
        match rgba_to_hicon(&compose(base_rgba, color)) {
            Some(icon) => {
                self.variants.insert(color, icon);
                icon
            }
            None => self.base,
        }
    }
}

impl Default for TrayIconSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_bound() -> TrayBadges {
        TrayBadges::default()
    }

    #[test]
    fn default_bindings_pick_expected_colors() {
        let b = all_bound();
        let s = |st: TrayStatus| active_badge(&b, &st);
        assert_eq!(
            s(TrayStatus {
                hidden: true,
                ..Default::default()
            }),
            Some(BadgeColor::Red),
            "存在隐藏窗口时应显示红色"
        );
        assert_eq!(
            s(TrayStatus {
                auto_hide: true,
                ..Default::default()
            }),
            Some(BadgeColor::Green),
            "启用自动隐藏时应显示绿色"
        );
        assert_eq!(
            s(TrayStatus {
                hide_current: true,
                ..Default::default()
            }),
            Some(BadgeColor::Yellow),
            "启用同时隐藏当前窗口时应显示黄色"
        );
        assert_eq!(
            s(TrayStatus {
                freeze: true,
                ..Default::default()
            }),
            Some(BadgeColor::Blue),
            "启用进程冻结时应显示蓝色"
        );
        assert_eq!(s(TrayStatus::default()), None, "无活跃状态时不显示角标");
    }

    #[test]
    fn priority_red_over_green_over_yellow_over_blue() {
        let b = all_bound();
        let all_on = TrayStatus {
            hidden: true,
            auto_hide: true,
            hide_current: true,
            freeze: true,
            ..Default::default()
        };
        assert_eq!(
            active_badge(&b, &all_on),
            Some(BadgeColor::Red),
            "全部活跃时红色优先级最高"
        );
        let no_hidden = TrayStatus {
            hidden: false,
            ..all_on
        };
        assert_eq!(
            active_badge(&b, &no_hidden),
            Some(BadgeColor::Green),
            "红色不活跃时轮到绿色"
        );
        let only_low = TrayStatus {
            hide_current: true,
            freeze: true,
            ..Default::default()
        };
        assert_eq!(
            active_badge(&b, &only_low),
            Some(BadgeColor::Yellow),
            "黄色优先于蓝色"
        );
    }

    #[test]
    fn empty_binding_skips_color_and_falls_through() {
        // 红色置空：即使有隐藏窗口，也应轮到下一个活跃颜色。
        let b = TrayBadges {
            red: String::new(),
            ..TrayBadges::default()
        };
        let st = TrayStatus {
            hidden: true,
            freeze: true,
            ..Default::default()
        };
        assert_eq!(
            active_badge(&b, &st),
            Some(BadgeColor::Blue),
            "置空的颜色不显示，由更低优先级的活跃颜色顶上"
        );
    }

    #[test]
    fn extended_statuses_can_be_bound() {
        let b = TrayBadges {
            red: String::new(),
            green: TRAY_STATUS_MONITOR_PAUSED.to_string(),
            yellow: String::new(),
            blue: TRAY_STATUS_ELEVATED.to_string(),
        };
        let admin_only = TrayStatus {
            elevated: true,
            ..Default::default()
        };
        assert_eq!(
            active_badge(&b, &admin_only),
            Some(BadgeColor::Blue),
            "管理员身份可绑定为角标状态"
        );
        let paused = TrayStatus {
            elevated: true,
            monitor_paused: true,
            ..Default::default()
        };
        assert_eq!(
            active_badge(&b, &paused),
            Some(BadgeColor::Green),
            "监控暂停可绑定为角标状态，且绿色优先于蓝色"
        );
    }

    #[test]
    fn rebinding_a_color_changes_its_trigger() {
        // 绿色改绑「存在隐藏窗口」：隐藏时显示绿点（红色已置空）。
        let b = TrayBadges {
            red: String::new(),
            green: bosskey_common::config::TRAY_STATUS_HIDDEN.to_string(),
            ..TrayBadges::default()
        };
        let st = TrayStatus {
            hidden: true,
            ..Default::default()
        };
        assert_eq!(active_badge(&b, &st), Some(BadgeColor::Green));
    }

    fn blank(width: u32, height: u32) -> IconRgba {
        IconRgba {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
        }
    }

    fn pixel(icon: &IconRgba, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * icon.width + x) * 4) as usize;
        icon.pixels[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn badge_paints_bottom_right_dot_and_leaves_far_corner_untouched() {
        let icon = compose(&blank(32, 32), BadgeColor::Red);
        let px = pixel(&icon, 24, 24);
        assert_eq!(&px[..3], &BadgeColor::Red.rgba()[..3], "圆心应为角标颜色");
        assert_eq!(px[3], 255, "圆心应完全不透明");
        assert_eq!(pixel(&icon, 2, 2), [0, 0, 0, 0], "对角像素应保持原样");
    }

    #[test]
    fn badge_survives_small_icons() {
        let icon = compose(&blank(16, 16), BadgeColor::Blue);
        assert!(
            icon.pixels.chunks_exact(4).any(|px| px[3] == 255),
            "16×16 图标上也应画出角标"
        );
    }

    #[test]
    fn mismatched_buffer_is_left_untouched() {
        let mut pixels = vec![0u8; 10];
        overlay_badge(&mut pixels, 32, 32, BadgeColor::Red.rgba());
        assert_eq!(pixels, vec![0u8; 10], "长度不符时不得越界或修改");
    }

    #[test]
    fn compose_does_not_mutate_base() {
        let base = blank(32, 32);
        let _ = compose(&base, BadgeColor::Yellow);
        assert!(base.pixels.iter().all(|b| *b == 0), "compose 不得修改原图");
    }
}
