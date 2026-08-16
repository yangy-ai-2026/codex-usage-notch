#[cfg(windows)]
use crate::window_geometry::{
    place_top_center, read_window_geometry, scale_size_for_dpi, NotchPlacement, WindowSize,
};

#[cfg(windows)]
use crate::window_tracker::WindowDiscovery;

#[cfg(windows)]
use crate::engine::{CreditSnapshot, CreditStatus, Engine, RuntimeStatus, UsageSnapshot};

#[cfg(windows)]
use crate::native_renderer::{build_render_model, render_layered_window, NativeRenderModel};

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

#[cfg(windows)]
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
static TRACKER_DIRTY: AtomicBool = AtomicBool::new(true);

#[cfg(windows)]
static TRACKER_STOP: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static ATTACHED_TARGET: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static OVERLAY_THREAD_ID: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
static OVERLAY_THREAD: OnceLock<Mutex<Option<std::thread::JoinHandle<()>>>> = OnceLock::new();

#[cfg(windows)]
static EXPANDED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static FORCE_EXPANDED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
const NATIVE_SIZE: WindowSize = WindowSize {
    width: 480,
    height: 64,
};

#[cfg(windows)]
const EXPANDED_SIZE: WindowSize = WindowSize {
    width: 500,
    height: 112,
};

#[cfg(windows)]
const HOVER_EXPAND_DELAY_MS: u64 = 125;

#[cfg(windows)]
const HOVER_COLLAPSE_DELAY_MS: u64 = 175;

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverState {
    Collapsed,
    HoverPending,
    Expanded,
    CollapsePending,
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq)]
struct NativeRuntimeSnapshot {
    usage: UsageSnapshot,
    credits: CreditSnapshot,
}

#[cfg(windows)]
struct HoverController {
    state: HoverState,
    transition_started: std::time::Instant,
    forced: bool,
}

#[cfg(windows)]
impl HoverController {
    fn new(forced: bool) -> Self {
        Self {
            state: if forced {
                HoverState::Expanded
            } else {
                HoverState::Collapsed
            },
            transition_started: std::time::Instant::now(),
            forced,
        }
    }

    fn update(&mut self, inside_activation_area: bool, now: std::time::Instant) -> Option<bool> {
        if self.forced {
            return None;
        }

        match self.state {
            HoverState::Collapsed if inside_activation_area => {
                self.state = HoverState::HoverPending;
                self.transition_started = now;
            }
            HoverState::HoverPending if !inside_activation_area => {
                self.state = HoverState::Collapsed;
            }
            HoverState::HoverPending
                if now.duration_since(self.transition_started)
                    >= std::time::Duration::from_millis(HOVER_EXPAND_DELAY_MS) =>
            {
                self.state = HoverState::Expanded;
                return Some(true);
            }
            HoverState::Expanded if !inside_activation_area => {
                self.state = HoverState::CollapsePending;
                self.transition_started = now;
            }
            HoverState::CollapsePending if inside_activation_area => {
                self.state = HoverState::Expanded;
            }
            HoverState::CollapsePending
                if now.duration_since(self.transition_started)
                    >= std::time::Duration::from_millis(HOVER_COLLAPSE_DELAY_MS) =>
            {
                self.state = HoverState::Collapsed;
                return Some(false);
            }
            _ => {}
        }

        None
    }

    fn reset_collapsed(&mut self) -> bool {
        if self.forced {
            return false;
        }

        let was_expanded = matches!(
            self.state,
            HoverState::Expanded | HoverState::CollapsePending
        );
        self.state = HoverState::Collapsed;
        self.transition_started = std::time::Instant::now();
        was_expanded
    }
}

#[cfg(windows)]
fn overlay_thread() -> &'static Mutex<Option<std::thread::JoinHandle<()>>> {
    OVERLAY_THREAD.get_or_init(|| Mutex::new(None))
}

#[cfg(windows)]
pub fn start() {
    let mut thread = overlay_thread()
        .lock()
        .expect("native overlay thread state is poisoned");
    if thread.is_some() {
        return;
    }

    TRACKER_STOP.store(false, Ordering::Release);
    TRACKER_DIRTY.store(true, Ordering::Release);
    #[cfg(debug_assertions)]
    let initial_expanded = std::env::var("CODEX_NOTCH_FORCE_EXPANDED").as_deref() == Ok("1");
    #[cfg(not(debug_assertions))]
    let initial_expanded = false;
    FORCE_EXPANDED.store(initial_expanded, Ordering::Release);
    EXPANDED.store(initial_expanded, Ordering::Release);
    *thread = Some(
        std::thread::Builder::new()
            .name("codex-notch-native-overlay".to_string())
            .spawn(run)
            .expect("failed to start Codex Notch native overlay thread"),
    );
}

#[cfg(not(windows))]
pub fn start() {}

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
    EXPANDED.store(expanded, Ordering::Release);
    TRACKER_DIRTY.store(true, Ordering::Release);
}

#[cfg(not(windows))]
pub fn set_expanded(_expanded: bool) {}

#[cfg(windows)]
pub fn stop() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

    TRACKER_STOP.store(true, Ordering::Release);
    TRACKER_DIRTY.store(true, Ordering::Release);
    FORCE_EXPANDED.store(false, Ordering::Release);
    EXPANDED.store(false, Ordering::Release);

    let thread_id = OVERLAY_THREAD_ID.swap(0, Ordering::AcqRel);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }

    let handle = overlay_thread()
        .lock()
        .expect("native overlay thread state is poisoned")
        .take();
    if let Some(handle) = handle {
        let _ = handle.join();
    }
    ATTACHED_TARGET.store(0, Ordering::Release);
}

#[cfg(not(windows))]
pub fn stop() {}

#[cfg(windows)]
struct NativeWindow {
    hwnd: windows::Win32::Foundation::HWND,
    instance: windows::Win32::Foundation::HINSTANCE,
}

#[cfg(windows)]
fn run() {
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_NOREMOVE};

    unsafe {
        let mut message = MSG::default();
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }
    OVERLAY_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::Release);

    let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
        OVERLAY_THREAD_ID.store(0, Ordering::Release);
        return;
    };
    let instance = HINSTANCE(module.0);
    let Some(window) = create_native_window(instance) else {
        OVERLAY_THREAD_ID.store(0, Ordering::Release);
        return;
    };

    let mut discovery = WindowDiscovery::default();
    let mut attached_target = None;
    let (runtime_sender, runtime_receiver) = std::sync::mpsc::channel();
    let usage_worker = spawn_usage_worker(runtime_sender);
    let hooks = message_hooks();

    message_loop(
        window.hwnd,
        &mut discovery,
        &mut attached_target,
        &runtime_receiver,
    );
    let _ = usage_worker.join();

    unsafe {
        use windows::Win32::UI::Accessibility::UnhookWinEvent;
        use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, UnregisterClassW};
        for hook in hooks {
            if !hook.is_invalid() {
                let _ = UnhookWinEvent(hook);
            }
        }
        let _ = DestroyWindow(window.hwnd);
        let _ = UnregisterClassW(
            windows::core::w!("CodexUsageNotch.NativeOverlay"),
            Some(window.instance),
        );
    }
    ATTACHED_TARGET.store(0, Ordering::Release);
    OVERLAY_THREAD_ID.store(0, Ordering::Release);
}

#[cfg(windows)]
fn create_native_window(instance: windows::Win32::Foundation::HINSTANCE) -> Option<NativeWindow> {
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, RegisterClassExW, CS_HREDRAW, CS_VREDRAW, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
    };

    let class_name = windows::core::w!("CodexUsageNotch.NativeOverlay");
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(native_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: Default::default(),
        hCursor: Default::default(),
        hbrBackground: Default::default(),
        lpszMenuName: windows::core::PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: Default::default(),
    };

    unsafe {
        if RegisterClassExW(&class) == 0 {
            return None;
        }

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
            class_name,
            windows::core::w!("Codex Usage Notch Native Overlay"),
            WS_POPUP,
            0,
            0,
            NATIVE_SIZE.width,
            NATIVE_SIZE.height,
            None,
            None,
            Some(instance),
            None,
        )
        .ok()?;

        Some(NativeWindow { hwnd, instance })
    }
}

#[cfg(windows)]
fn spawn_usage_worker(
    sender: std::sync::mpsc::Sender<NativeRuntimeSnapshot>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("codex-notch-usage-reader".to_string())
        .spawn(move || {
            let engine = Engine::default();
            let mut previous: Option<NativeRuntimeSnapshot> = None;
            while !TRACKER_STOP.load(Ordering::Acquire) {
                let snapshot = engine
                    .read_usage_and_credits_with_recovery(
                        previous.as_ref().map(|snapshot| snapshot.usage.clone()),
                        previous.as_ref().map(|snapshot| snapshot.credits.clone()),
                    )
                    .map(|(usage, credits)| NativeRuntimeSnapshot { usage, credits })
                    .unwrap_or_else(|_| native_error_runtime_snapshot(previous.as_ref()));
                previous = Some(snapshot.clone());
                if sender.send(snapshot).is_err() {
                    return;
                }

                for _ in 0..600 {
                    if TRACKER_STOP.load(Ordering::Acquire) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        })
        .expect("failed to start Codex Notch usage reader")
}

#[cfg(windows)]
fn loading_snapshot() -> NativeRuntimeSnapshot {
    NativeRuntimeSnapshot {
        usage: UsageSnapshot {
            windows: Vec::new(),
            status: RuntimeStatus::Loading,
            fetched_at: None,
            last_successful_at: None,
            source: "codex-owned-local-app-server".to_string(),
            capability: "account/rateLimits/read".to_string(),
            diagnostic_code: None,
        },
        credits: CreditSnapshot::loading(),
    }
}

#[cfg(windows)]
fn native_error_runtime_snapshot(
    previous: Option<&NativeRuntimeSnapshot>,
) -> NativeRuntimeSnapshot {
    let mut snapshot = previous.cloned().unwrap_or_else(loading_snapshot);
    snapshot.usage.status = RuntimeStatus::Error;
    snapshot.usage.diagnostic_code = Some("native_usage_read_failed".to_string());
    if snapshot.credits.last_successful_at.is_some() {
        snapshot.credits.status = CreditStatus::Stale;
    } else {
        snapshot.credits.status = CreditStatus::Error;
    }
    snapshot.credits.diagnostic_code = Some("native_credit_read_failed".to_string());
    snapshot
}

#[cfg(windows)]
fn message_loop(
    overlay: windows::Win32::Foundation::HWND,
    discovery: &mut WindowDiscovery,
    attached_target: &mut Option<u64>,
    runtime_receiver: &std::sync::mpsc::Receiver<NativeRuntimeSnapshot>,
) {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
    };

    let mut snapshot = loading_snapshot();
    let mut rendered_model: Option<NativeRenderModel> = None;
    let force_expanded = FORCE_EXPANDED.load(Ordering::Acquire);
    let mut hover = HoverController::new(force_expanded);
    let mut last_placement = sync_overlay(
        overlay,
        discovery,
        attached_target,
        &snapshot,
        &mut rendered_model,
    );
    let mut last_sync = Instant::now();
    while !TRACKER_STOP.load(Ordering::Acquire) {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_QUIT {
                    TRACKER_STOP.store(true, Ordering::Release);
                    break;
                }
                let _ = TranslateMessage(&message);
                let _ = DispatchMessageW(&message);
            }
        }

        let mut usage_changed = false;
        while let Ok(next_snapshot) = runtime_receiver.try_recv() {
            snapshot = next_snapshot;
            usage_changed = true;
        }

        let event_dirty = TRACKER_DIRTY.swap(false, Ordering::AcqRel);
        let fallback_due = last_sync.elapsed() >= Duration::from_millis(500);
        if event_dirty || fallback_due || usage_changed {
            let previous_target = *attached_target;
            last_placement = sync_overlay(
                overlay,
                discovery,
                attached_target,
                &snapshot,
                &mut rendered_model,
            );
            let target_changed = previous_target.is_some() && previous_target != *attached_target;
            if !force_expanded
                && (target_changed || last_placement.is_none())
                && hover.reset_collapsed()
            {
                EXPANDED.store(false, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
            last_sync = Instant::now();
        }

        if !force_expanded {
            if let Some(placement) = last_placement {
                let inside = cursor_in_activation_area(placement, EXPANDED.load(Ordering::Acquire));
                if let Some(expanded) = hover.update(inside, Instant::now()) {
                    EXPANDED.store(expanded, Ordering::Release);
                    TRACKER_DIRTY.store(true, Ordering::Release);
                }
            } else if hover.reset_collapsed() {
                EXPANDED.store(false, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
        }

        std::thread::sleep(Duration::from_millis(40));
    }

    hide_overlay(overlay);
    clear_overlay_owner(overlay, attached_target);
}

#[cfg(windows)]
unsafe extern "system" fn native_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, HTTRANSPARENT, MA_NOACTIVATE, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST,
    };

    match message {
        WM_ERASEBKGND => windows::Win32::Foundation::LRESULT(1),
        WM_MOUSEACTIVATE => windows::Win32::Foundation::LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => windows::Win32::Foundation::LRESULT(HTTRANSPARENT as isize),
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
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
    TRACKER_DIRTY.store(true, Ordering::Release);
}

#[cfg(windows)]
fn message_hooks() -> [windows::Win32::UI::Accessibility::HWINEVENTHOOK; 2] {
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND,
        WINEVENT_OUTOFCONTEXT,
    };

    unsafe {
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
    }
}

#[cfg(windows)]
fn cursor_in_activation_area(placement: NotchPlacement, expanded: bool) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return false;
    }

    let design_width = if expanded {
        EXPANDED_SIZE.width
    } else {
        NATIVE_SIZE.width
    };
    let radius = ((20_i64 * i64::from(placement.bounds.width()) + i64::from(design_width / 2))
        / i64::from(design_width.max(1))) as i32;
    point_in_rounded_bounds(placement.bounds, cursor.x, cursor.y, radius)
}

#[cfg(windows)]
fn point_in_rounded_bounds(
    bounds: crate::window_geometry::Rect,
    x: i32,
    y: i32,
    radius: i32,
) -> bool {
    if x < bounds.left || x >= bounds.right || y < bounds.top || y >= bounds.bottom {
        return false;
    }

    let local_x = x - bounds.left;
    let local_y = y - bounds.top;
    let width = bounds.width();
    let height = bounds.height();
    let radius = radius.min(width / 2).min(height / 2).max(1);

    let in_corner = |corner_x: i32, corner_y: i32| {
        let dx = local_x - corner_x;
        let dy = local_y - corner_y;
        dx * dx + dy * dy <= radius * radius
    };

    if local_x < radius && local_y < radius {
        in_corner(radius, radius)
    } else if local_x >= width - radius && local_y < radius {
        in_corner(width - radius - 1, radius)
    } else if local_x < radius && local_y >= height - radius {
        in_corner(radius, height - radius - 1)
    } else if local_x >= width - radius && local_y >= height - radius {
        in_corner(width - radius - 1, height - radius - 1)
    } else {
        true
    }
}

#[cfg(windows)]
fn sync_overlay(
    overlay: windows::Win32::Foundation::HWND,
    discovery: &mut WindowDiscovery,
    attached_target: &mut Option<u64>,
    snapshot: &NativeRuntimeSnapshot,
    rendered_model: &mut Option<NativeRenderModel>,
) -> Option<NotchPlacement> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindow, IsWindowVisible, SetWindowLongPtrW, GWLP_HWNDPARENT,
    };

    if let Some(attached_hwnd) = *attached_target {
        let attached = HWND(attached_hwnd as *mut c_void);
        unsafe {
            if !IsWindow(Some(attached)).as_bool() {
                clear_overlay_owner(overlay, attached_target);
            } else if IsIconic(attached).as_bool() {
                hide_overlay(overlay);
                return None;
            }
        }
    }

    let Ok(discovery_snapshot) = discovery.refresh() else {
        hide_overlay_and_clear_owner(overlay, attached_target);
        return None;
    };
    let Some(target) = discovery_snapshot.target else {
        hide_overlay_and_clear_owner(overlay, attached_target);
        return None;
    };
    let target_hwnd = HWND(target.hwnd as *mut c_void);

    unsafe {
        if !IsWindow(Some(target_hwnd)).as_bool() {
            hide_overlay_and_clear_owner(overlay, attached_target);
            return None;
        }
        if IsIconic(target_hwnd).as_bool() {
            hide_overlay(overlay);
            return None;
        }
    }

    let Ok(geometry) = read_window_geometry(&target) else {
        hide_overlay(overlay);
        return None;
    };
    let design_size = if EXPANDED.load(Ordering::Acquire) {
        EXPANDED_SIZE
    } else {
        NATIVE_SIZE
    };
    let notch_size = scale_size_for_dpi(design_size, geometry.dpi);
    let placement = place_top_center(geometry.frame_bounds, geometry.work_area, notch_size, 8);
    let model = build_render_model(
        &snapshot.usage,
        &snapshot.credits,
        EXPANDED.load(Ordering::Acquire),
        notch_size,
    );
    let render_ready = if rendered_model.as_ref() == Some(&model) {
        true
    } else if render_layered_window(overlay, &model) {
        *rendered_model = Some(model);
        true
    } else {
        false
    };

    if !render_ready {
        hide_overlay(overlay);
        return None;
    }

    let positioned = unsafe {
        if *attached_target != Some(target.hwnd) {
            let _ = SetWindowLongPtrW(overlay, GWLP_HWNDPARENT, target_hwnd.0 as isize);
            *attached_target = Some(target.hwnd);
            ATTACHED_TARGET.store(target_hwnd.0 as isize, Ordering::Release);
        }
        if IsWindowVisible(overlay).as_bool() {
            position_overlay(overlay, &placement)
        } else {
            show_overlay(overlay, target_hwnd, &placement)
        }
    };
    if !positioned {
        hide_overlay(overlay);
        None
    } else {
        Some(placement)
    }
}

#[cfg(windows)]
unsafe fn show_overlay(
    overlay: windows::Win32::Foundation::HWND,
    target: windows::Win32::Foundation::HWND,
    placement: &NotchPlacement,
) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, SWP_NOACTIVATE, SW_SHOWNOACTIVATE,
    };

    if SetWindowPos(
        overlay,
        Some(target),
        placement.bounds.left,
        placement.bounds.top,
        placement.bounds.width(),
        placement.bounds.height(),
        SWP_NOACTIVATE,
    )
    .is_err()
    {
        return false;
    }
    let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
    true
}

#[cfg(windows)]
unsafe fn position_overlay(
    overlay: windows::Win32::Foundation::HWND,
    placement: &NotchPlacement,
) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER,
    };

    let mut current_bounds = RECT::default();
    if GetWindowRect(overlay, &mut current_bounds).is_ok()
        && current_bounds.left == placement.bounds.left
        && current_bounds.top == placement.bounds.top
        && current_bounds.right == placement.bounds.right
        && current_bounds.bottom == placement.bounds.bottom
    {
        return true;
    }

    SetWindowPos(
        overlay,
        None,
        placement.bounds.left,
        placement.bounds.top,
        placement.bounds.width(),
        placement.bounds.height(),
        SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .is_ok()
}

#[cfg(windows)]
fn hide_overlay(overlay: windows::Win32::Foundation::HWND) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        let _ = ShowWindow(overlay, SW_HIDE);
    }
}

#[cfg(windows)]
fn clear_overlay_owner(
    overlay: windows::Win32::Foundation::HWND,
    attached_target: &mut Option<u64>,
) {
    if attached_target.take().is_some() {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_HWNDPARENT};
            let _ = SetWindowLongPtrW(overlay, GWLP_HWNDPARENT, 0);
        }
        ATTACHED_TARGET.store(0, Ordering::Release);
    }
}

#[cfg(windows)]
fn hide_overlay_and_clear_owner(
    overlay: windows::Win32::Foundation::HWND,
    attached_target: &mut Option<u64>,
) {
    hide_overlay(overlay);
    clear_overlay_owner(overlay, attached_target);
}

#[cfg(all(test, windows))]
mod tests {
    use super::{
        HoverController, HoverState, NativeRuntimeSnapshot, HOVER_COLLAPSE_DELAY_MS,
        HOVER_EXPAND_DELAY_MS,
    };
    use crate::engine::{CreditSnapshot, CreditStatus, RuntimeStatus, UsageSnapshot};
    use std::time::{Duration, Instant};

    fn runtime_snapshot(status: CreditStatus, balance: Option<&str>) -> NativeRuntimeSnapshot {
        NativeRuntimeSnapshot {
            usage: UsageSnapshot {
                windows: Vec::new(),
                status: RuntimeStatus::Partial,
                fetched_at: Some(1),
                last_successful_at: Some(1),
                source: "test".to_string(),
                capability: "account/rateLimits/read".to_string(),
                diagnostic_code: None,
            },
            credits: CreditSnapshot {
                has_credits: matches!(status, CreditStatus::Available | CreditStatus::Unlimited),
                unlimited: status == CreditStatus::Unlimited,
                balance: balance.map(str::to_string),
                status,
                fetched_at: Some(1),
                last_successful_at: Some(1),
                diagnostic_code: None,
            },
        }
    }

    #[test]
    fn native_runtime_channel_receives_available_credit_snapshot() {
        let expected = runtime_snapshot(CreditStatus::Available, Some("841.00"));
        let (sender, receiver) = std::sync::mpsc::channel();
        sender
            .send(expected.clone())
            .expect("runtime snapshot send");

        let received = receiver.try_recv().expect("runtime snapshot receive");
        assert_eq!(received, expected);
        assert_eq!(received.credits.balance.as_deref(), Some("841.00"));
    }

    #[test]
    fn native_runtime_propagates_unlimited_unavailable_and_stale_states() {
        for (status, balance) in [
            (CreditStatus::Unlimited, None),
            (CreditStatus::Unavailable, None),
            (CreditStatus::Stale, Some("841.00")),
            (CreditStatus::Error, None),
        ] {
            let snapshot = runtime_snapshot(status.clone(), balance);
            assert_eq!(snapshot.credits.status, status);
            assert_eq!(snapshot.credits.balance.as_deref(), balance);
        }
    }

    #[test]
    fn credit_updates_do_not_change_usage_snapshot_or_require_usage_status_match() {
        let available = runtime_snapshot(CreditStatus::Available, Some("841.00"));
        assert_eq!(available.usage.status, RuntimeStatus::Partial);

        let mut updated = available.clone();
        updated.credits = runtime_snapshot(CreditStatus::Stale, Some("841.00")).credits;
        assert_eq!(updated.usage, available.usage);
        assert_eq!(updated.credits.status, CreditStatus::Stale);
    }

    #[test]
    fn hover_state_machine_delays_expand_and_collapse_and_cancels_leave() {
        let start = Instant::now();
        let mut controller = HoverController::new(false);

        assert_eq!(controller.state, HoverState::Collapsed);
        assert_eq!(controller.update(true, start), None);
        assert_eq!(controller.state, HoverState::HoverPending);
        assert_eq!(
            controller.update(
                true,
                start + Duration::from_millis(HOVER_EXPAND_DELAY_MS - 1)
            ),
            None
        );
        assert_eq!(
            controller.update(true, start + Duration::from_millis(HOVER_EXPAND_DELAY_MS)),
            Some(true)
        );
        assert_eq!(controller.state, HoverState::Expanded);

        assert_eq!(
            controller.update(false, start + Duration::from_millis(200)),
            None
        );
        assert_eq!(controller.state, HoverState::CollapsePending);
        assert_eq!(
            controller.update(true, start + Duration::from_millis(250)),
            None
        );
        assert_eq!(controller.state, HoverState::Expanded);

        assert_eq!(
            controller.update(false, start + Duration::from_millis(300)),
            None
        );
        assert_eq!(
            controller.update(
                false,
                start + Duration::from_millis(300 + HOVER_COLLAPSE_DELAY_MS)
            ),
            Some(false)
        );
        assert_eq!(controller.state, HoverState::Collapsed);
    }
}
