#[cfg(windows)]
use crate::window_geometry::{
    place_top_center, read_window_geometry, scale_size_for_dpi, NotchPlacement, WindowSize,
};

#[cfg(windows)]
use crate::window_tracker::WindowDiscovery;

#[cfg(windows)]
static TRACKER_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[cfg(windows)]
static TRACKER_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
static EXPANDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
const COLLAPSED_SIZE: WindowSize = WindowSize {
    width: 480,
    height: 64,
};

#[cfg(windows)]
const EXPANDED_SIZE: WindowSize = WindowSize {
    width: 640,
    height: 240,
};

#[cfg(windows)]
pub fn start(overlay_hwnd: u64) {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    TRACKER_STOP.store(false, std::sync::atomic::Ordering::Release);
    EXPANDED.store(false, std::sync::atomic::Ordering::Release);
    let overlay = HWND(overlay_hwnd as *mut c_void);
    unsafe {
        let _ = ShowWindow(overlay, SW_HIDE);
    }
    configure_overlay_window(overlay);
    std::thread::Builder::new()
        .name("codex-notch-window-tracker".to_string())
        .spawn(move || run(overlay_hwnd))
        .expect("failed to start Codex Notch window tracker");
}

#[cfg(not(windows))]
pub fn start(_overlay_hwnd: u64) {}

#[cfg(windows)]
pub fn initialize_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(windows))]
pub fn initialize_dpi_awareness() {}

#[cfg(windows)]
pub fn set_expanded(expanded: bool) {
    EXPANDED.store(expanded, std::sync::atomic::Ordering::Release);
    TRACKER_DIRTY.store(true, std::sync::atomic::Ordering::Release);
}

#[cfg(not(windows))]
pub fn set_expanded(_expanded: bool) {}

#[cfg(windows)]
pub fn stop() {
    TRACKER_STOP.store(true, std::sync::atomic::Ordering::Release);
    TRACKER_DIRTY.store(true, std::sync::atomic::Ordering::Release);
}

#[cfg(not(windows))]
pub fn stop() {}

#[cfg(windows)]
fn run(overlay_hwnd: u64) {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND,
        WINEVENT_OUTOFCONTEXT,
    };

    let mut discovery = WindowDiscovery::default();
    let mut attached_target = None;
    sync_overlay(overlay_hwnd, &mut discovery, &mut attached_target);

    let hooks = unsafe {
        [
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_MINIMIZEEND,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            ),
            SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            ),
        ]
    };
    let mut last_sync = Instant::now();
    while !TRACKER_STOP.load(Ordering::Acquire) {
        pump_messages();
        let event_dirty = TRACKER_DIRTY.swap(false, Ordering::AcqRel);
        let fallback_due = last_sync.elapsed() >= Duration::from_millis(500);
        if event_dirty || fallback_due {
            sync_overlay(overlay_hwnd, &mut discovery, &mut attached_target);
            last_sync = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(80));
    }

    unsafe {
        use windows::Win32::UI::Accessibility::UnhookWinEvent;
        for hook in hooks {
            if !hook.is_invalid() {
                let _ = UnhookWinEvent(hook);
            }
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn win_event_callback(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    _hwnd: windows::Win32::Foundation::HWND,
    _object: i32,
    _child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    use std::sync::atomic::Ordering;
    TRACKER_DIRTY.store(true, Ordering::Release);
}

#[cfg(windows)]
fn pump_messages() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };

    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageW(&message);
        }
    }
}

#[cfg(windows)]
fn sync_overlay(
    overlay_hwnd: u64,
    discovery: &mut WindowDiscovery,
    attached_target: &mut Option<u64>,
) {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindow, IsWindowVisible, SetWindowLongPtrW, GWLP_HWNDPARENT,
    };

    let overlay = HWND(overlay_hwnd as *mut c_void);
    let Ok(snapshot) = discovery.refresh() else {
        hide_overlay(overlay, attached_target);
        return;
    };
    let Some(target) = snapshot.target else {
        hide_overlay(overlay, attached_target);
        return;
    };
    let target_hwnd = HWND(target.hwnd as *mut c_void);

    unsafe {
        if !IsWindow(Some(target_hwnd)).as_bool()
            || !IsWindowVisible(target_hwnd).as_bool()
            || IsIconic(target_hwnd).as_bool()
        {
            hide_overlay(overlay, attached_target);
            return;
        }
    }

    let Ok(geometry) = read_window_geometry(&target) else {
        hide_overlay(overlay, attached_target);
        return;
    };
    let design_size = if EXPANDED.load(std::sync::atomic::Ordering::Acquire) {
        EXPANDED_SIZE
    } else {
        COLLAPSED_SIZE
    };
    let notch_size = scale_size_for_dpi(design_size, geometry.dpi);
    let placement = place_top_center(geometry.frame_bounds, geometry.work_area, notch_size, 8);

    unsafe {
        if *attached_target != Some(target.hwnd) {
            let _ = SetWindowLongPtrW(overlay, GWLP_HWNDPARENT, target_hwnd.0 as isize);
            *attached_target = Some(target.hwnd);
        }
        if !show_overlay(overlay, target_hwnd, &placement) {
            hide_overlay(overlay, attached_target);
        }
    }
}

#[cfg(windows)]
unsafe fn show_overlay(
    overlay: windows::Win32::Foundation::HWND,
    target: windows::Win32::Foundation::HWND,
    placement: &NotchPlacement,
) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_SHOWWINDOW,
        SW_SHOWNOACTIVATE,
    };

    let was_visible = IsWindowVisible(overlay).as_bool();
    let mut flags = SWP_NOACTIVATE;
    if !was_visible {
        flags |= SWP_SHOWWINDOW;
    }
    if SetWindowPos(
        overlay,
        Some(target),
        placement.bounds.left,
        placement.bounds.top,
        placement.bounds.width(),
        placement.bounds.height(),
        flags,
    )
    .is_err()
    {
        return false;
    }
    if !was_visible {
        let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
    }
    true
}

#[cfg(windows)]
fn configure_overlay_window(overlay: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SIZEBOX,
        WS_SYSMENU,
    };

    unsafe {
        let style = GetWindowLongPtrW(overlay, GWL_STYLE) as u32;
        let frame_bits =
            WS_CAPTION.0 | WS_SIZEBOX.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0;
        let borderless_style = (style & !frame_bits) | WS_POPUP.0;
        if borderless_style != style {
            let _ = SetWindowLongPtrW(overlay, GWL_STYLE, borderless_style as isize);
            let _ = SetWindowPos(
                overlay,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        let ex_style = GetWindowLongPtrW(overlay, GWL_EXSTYLE);
        let configured_ex_style = ex_style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        if configured_ex_style != ex_style {
            let _ = SetWindowLongPtrW(overlay, GWL_EXSTYLE, configured_ex_style);
        }
    }
}

#[cfg(windows)]
fn hide_overlay(overlay: windows::Win32::Foundation::HWND, attached_target: &mut Option<u64>) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowLongPtrW, ShowWindow, GWLP_HWNDPARENT, SW_HIDE,
        };
        if attached_target.take().is_some() {
            let _ = SetWindowLongPtrW(overlay, GWLP_HWNDPARENT, 0);
        }
        let _ = ShowWindow(overlay, SW_HIDE);
    }
}
