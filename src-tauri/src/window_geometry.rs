use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::window_tracker::WindowTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(self) -> i32 {
        self.right - self.left
    }

    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
}

pub fn scale_size_for_dpi(size: WindowSize, dpi: WindowDpi) -> WindowSize {
    fn scale(value: i32, dpi: u32) -> i32 {
        ((i64::from(value.max(1)) * i64::from(dpi.max(1)) + 48) / 96).clamp(1, i64::from(i32::MAX))
            as i32
    }

    WindowSize {
        width: scale(size.width, dpi.x),
        height: scale(size.height, dpi.y),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowDpi {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometrySnapshot {
    pub frame_bounds: Rect,
    pub work_area: Rect,
    pub dpi: WindowDpi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotchPlacement {
    pub bounds: Rect,
    pub outside_target_frame: bool,
}

#[derive(Debug, Error)]
pub enum WindowGeometryError {
    #[error("Windows window geometry is unavailable on this platform")]
    UnsupportedPlatform,
    #[cfg(windows)]
    #[error("failed to read Windows window geometry: {0}")]
    Windows(#[source] windows::core::Error),
}

pub fn place_top_center(
    target_frame: Rect,
    work_area: Rect,
    notch_size: WindowSize,
    gap: i32,
) -> NotchPlacement {
    let width = notch_size.width.clamp(1, work_area.width().max(1));
    let height = notch_size.height.clamp(1, work_area.height().max(1));

    let desired_left = target_frame.left + (target_frame.width() - width) / 2;
    let left = desired_left.clamp(work_area.left, work_area.right - width);

    let desired_top = target_frame.top - gap - height;
    let top = desired_top
        .max(work_area.top)
        .min(work_area.bottom - height);

    let bounds = Rect::new(left, top, left + width, top + height);
    NotchPlacement {
        outside_target_frame: bounds.bottom <= target_frame.top,
        bounds,
    }
}

#[cfg(windows)]
pub fn read_window_geometry(
    target: &WindowTarget,
) -> Result<WindowGeometrySnapshot, WindowGeometryError> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;

    let hwnd = HWND(target.hwnd as *mut c_void);
    let mut frame = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(WindowGeometryError::Windows)?;
    }

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_invalid() {
        return Err(WindowGeometryError::Windows(
            windows::core::Error::from_win32(),
        ));
    }

    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let monitor_ok = unsafe { GetMonitorInfoW(monitor, &mut monitor_info).as_bool() };
    if !monitor_ok {
        return Err(WindowGeometryError::Windows(
            windows::core::Error::from_win32(),
        ));
    }

    let dpi_x = unsafe { GetDpiForWindow(hwnd) }.max(1);
    Ok(WindowGeometrySnapshot {
        frame_bounds: rect_from_windows(frame),
        work_area: rect_from_windows(monitor_info.rcWork),
        dpi: WindowDpi { x: dpi_x, y: dpi_x },
    })
}

#[cfg(not(windows))]
pub fn read_window_geometry(
    _target: &WindowTarget,
) -> Result<WindowGeometrySnapshot, WindowGeometryError> {
    Err(WindowGeometryError::UnsupportedPlatform)
}

#[cfg(windows)]
fn rect_from_windows(rect: windows::Win32::Foundation::RECT) -> Rect {
    Rect::new(rect.left, rect.top, rect.right, rect.bottom)
}

#[cfg(test)]
mod tests {
    use super::{place_top_center, scale_size_for_dpi, Rect, WindowDpi, WindowSize};

    const NOTCH: WindowSize = WindowSize {
        width: 400,
        height: 64,
    };

    #[test]
    fn places_notch_centered_above_restored_window() {
        let placement = place_top_center(
            Rect::new(100, 300, 1500, 1200),
            Rect::new(0, 0, 1920, 1080),
            NOTCH,
            8,
        );

        assert_eq!(placement.bounds, Rect::new(600, 228, 1000, 292));
        assert!(placement.outside_target_frame);
    }

    #[test]
    fn clamps_centered_notch_to_monitor_work_area() {
        let placement = place_top_center(
            Rect::new(-800, 10, 200, 900),
            Rect::new(-1280, 0, 0, 1000),
            NOTCH,
            8,
        );

        assert_eq!(placement.bounds, Rect::new(-500, 0, -100, 64));
        assert!(!placement.outside_target_frame);
    }

    #[test]
    fn preserves_physical_coordinates_on_a_150_percent_monitor() {
        let placement = place_top_center(
            Rect::new(2560, 200, 4480, 1400),
            Rect::new(1920, 0, 5120, 1800),
            WindowSize {
                width: 600,
                height: 96,
            },
            12,
        );

        assert_eq!(placement.bounds, Rect::new(3220, 92, 3820, 188));
        assert!(placement.outside_target_frame);
    }

    #[test]
    fn supports_negative_cross_monitor_coordinates() {
        let placement = place_top_center(
            Rect::new(-1800, 300, -400, 1100),
            Rect::new(-2560, 0, 0, 1440),
            NOTCH,
            8,
        );

        assert_eq!(placement.bounds, Rect::new(-1300, 228, -900, 292));
        assert!(placement.outside_target_frame);
    }

    #[test]
    fn scales_design_size_to_physical_pixels_at_150_percent_dpi() {
        assert_eq!(
            scale_size_for_dpi(
                WindowSize {
                    width: 480,
                    height: 144,
                },
                WindowDpi { x: 144, y: 144 },
            ),
            WindowSize {
                width: 720,
                height: 216,
            }
        );
    }
}
