//! 显示器几何：按屏幕坐标点查它所在的那块显示器。
//!
//! 低级鼠标钩子给的坐标、`GetCursorPos` 拿到的坐标都是**虚拟屏幕坐标**——
//! 主显示器左上角是原点，排在它左边或上边的显示器坐标为负。用
//! `GetSystemMetrics(SM_CXSCREEN)`（只有主显示器的宽高）或 `SPI_GETWORKAREA`
//! （只有主显示器的工作区）去解释这些坐标，在多显示器下必然出错。

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    MonitorFromPoint,
};

/// 一块矩形屏幕区域，虚拟屏幕坐标。`right` / `bottom` 是开区间端点（Win32 约定）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    fn from_win32(r: RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

/// 一块显示器的两个矩形。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    /// 整块显示器，含任务栏所占的部分。
    pub bounds: ScreenRect,
    /// 去掉任务栏等应用栏后的可用区域。
    pub work: ScreenRect,
}

fn query(monitor: windows::Win32::Graphics::Gdi::HMONITOR) -> Option<MonitorInfo> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() }.then(|| MonitorInfo {
        bounds: ScreenRect::from_win32(info.rcMonitor),
        work: ScreenRect::from_win32(info.rcWork),
    })
}

/// 包含该点的显示器；点落在显示器之间的空隙时取最近的一块。
///
/// 每次调用都实地查询，所以分辨率调整、显示器插拔、投影切换都无需另行通知——
/// 这也是不缓存的理由：缓存就得处理 `WM_DISPLAYCHANGE`，而钩子线程收不到它。
pub fn at_point(x: i32, y: i32) -> Option<MonitorInfo> {
    let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
    query(monitor)
}

/// 主显示器。
pub fn primary() -> Option<MonitorInfo> {
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    query(monitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_primary_monitor_starts_at_the_virtual_origin() {
        // 虚拟屏幕坐标系就是以主显示器左上角为原点定义的。
        let primary = primary().expect("应能查到主显示器");
        assert_eq!(
            (primary.bounds.left, primary.bounds.top),
            (0, 0),
            "主显示器左上角按定义是原点: {:?}",
            primary.bounds
        );
        assert!(
            primary.bounds.width() > 0 && primary.bounds.height() > 0,
            "主显示器应有正的宽高: {:?}",
            primary.bounds
        );
    }

    #[test]
    fn the_work_area_sits_inside_the_monitor_bounds() {
        let m = primary().unwrap();
        assert!(
            m.work.left >= m.bounds.left
                && m.work.top >= m.bounds.top
                && m.work.right <= m.bounds.right
                && m.work.bottom <= m.bounds.bottom,
            "工作区应被显示器矩形包住: work={:?} bounds={:?}",
            m.work,
            m.bounds
        );
    }

    #[test]
    fn a_point_inside_the_primary_monitor_resolves_to_it() {
        let primary = primary().unwrap();
        let inside = at_point(primary.bounds.width() / 2, primary.bounds.height() / 2)
            .expect("屏幕中心应能查到显示器");
        assert_eq!(inside.bounds, primary.bounds);
    }

    #[test]
    fn a_far_away_point_still_resolves_to_the_nearest_monitor() {
        // MONITOR_DEFAULTTONEAREST：坐标落在所有显示器之外也要给出一块，
        // 否则钩子里就得为「查不到」单独兜底。
        let far = at_point(-500_000, -500_000).expect("远点也应回退到最近的显示器");
        assert!(far.bounds.width() > 0, "回退结果应是真实显示器: {far:?}");
    }
}
