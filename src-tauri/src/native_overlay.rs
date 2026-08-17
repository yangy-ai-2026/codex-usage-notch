#[cfg(windows)]
use crate::window_geometry::{
    read_window_geometry, scale_size_for_dpi, NotchPlacement, Rect, WindowSize,
};

#[cfg(windows)]
use crate::window_tracker::WindowDiscovery;

#[cfg(windows)]
use crate::engine::{CreditSnapshot, CreditStatus, Engine, RuntimeStatus, UsageSnapshot};

#[cfg(windows)]
use crate::native_renderer::{
    build_render_model, compact_width_for_snapshot, expanded_height_for_window_count,
    render_layered_window, NativeRenderModel,
};

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

#[cfg(windows)]
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
static TRACKER_DIRTY: AtomicBool = AtomicBool::new(true);

#[cfg(windows)]
static TRACKER_STOP: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static MOVE_SIZE_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static CODEX_FOREGROUND_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static ATTACHED_TARGET: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static OVERLAY_THREAD_ID: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
static OVERLAY_THREAD: OnceLock<Mutex<Option<std::thread::JoinHandle<()>>>> = OnceLock::new();

#[cfg(windows)]
static DETAILS_EXPANDED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static CHEVRON_HIT_LEFT: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static CHEVRON_HIT_TOP: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static CHEVRON_HIT_RIGHT: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static CHEVRON_HIT_BOTTOM: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
static CHEVRON_ACTION: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
static CHEVRON_CLICKED: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
static CHEVRON_PRESSED: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
const NATIVE_SIZE: WindowSize = WindowSize {
    width: 300,
    height: 40,
};

#[cfg(windows)]
const EXPANDED_SIZE: WindowSize = WindowSize {
    width: 320,
    height: 92,
};

#[cfg(windows)]
const HOVER_EXPAND_DELAY_MS: u64 = 125;

#[cfg(windows)]
const HOVER_COLLAPSE_DELAY_MS: u64 = 175;

#[cfg(windows)]
const TRANSITION_DURATION_MS: u64 = 180;

#[cfg(windows)]
const REVEAL_DURATION_MS: u64 = 160;

#[cfg(windows)]
const HIDDEN_VISIBLE_HEIGHT: i32 = 6;

#[cfg(windows)]
const HOVER_ZONE_PADDING_X: i32 = 24;

#[cfg(windows)]
const HOVER_ZONE_PADDING_TOP: i32 = 8;

#[cfg(windows)]
const HOVER_ZONE_PADDING_BOTTOM: i32 = 18;

#[cfg(windows)]
const CHEVRON_DOWN: u32 = 1;

#[cfg(windows)]
const CHEVRON_UP: u32 = 2;

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverState {
    Collapsed,
    HoverPending,
    Compact,
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
}

#[cfg(windows)]
impl HoverController {
    fn new() -> Self {
        Self {
            state: HoverState::Collapsed,
            transition_started: std::time::Instant::now(),
        }
    }

    fn update(&mut self, inside_activation_area: bool, now: std::time::Instant) -> Option<bool> {
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
                self.state = HoverState::Compact;
                return Some(true);
            }
            HoverState::Compact if !inside_activation_area => {
                self.state = HoverState::CollapsePending;
                self.transition_started = now;
            }
            HoverState::CollapsePending if inside_activation_area => {
                self.state = HoverState::Compact;
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
        let was_visible = matches!(
            self.state,
            HoverState::Compact | HoverState::CollapsePending
        );
        self.state = HoverState::Collapsed;
        self.transition_started = std::time::Instant::now();
        was_visible
    }

    fn pause_for_move_size(&mut self, now: std::time::Instant) {
        if matches!(
            self.state,
            HoverState::HoverPending | HoverState::CollapsePending
        ) {
            self.transition_started = now;
        }
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
    MOVE_SIZE_ACTIVE.store(false, Ordering::Release);
    CODEX_FOREGROUND_ACTIVE.store(false, Ordering::Release);
    DETAILS_EXPANDED.store(false, Ordering::Release);
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
    DETAILS_EXPANDED.store(expanded, Ordering::Release);
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
    MOVE_SIZE_ACTIVE.store(false, Ordering::Release);
    CODEX_FOREGROUND_ACTIVE.store(false, Ordering::Release);
    DETAILS_EXPANDED.store(false, Ordering::Release);
    clear_chevron_hit_region();
    reset_chevron_interaction();

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
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
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
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
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
    let mut hover = HoverController::new();
    let mut transition_target = false;
    let mut transition_from = 0.0;
    let mut transition_progress = transition_from;
    let mut transition_started = Instant::now();
    let mut reveal_target = 0.0;
    let mut reveal_from = reveal_target;
    let mut reveal_progress = reveal_target;
    let mut reveal_started = Instant::now();
    let mut hover_zone = None;
    let mut last_placement = sync_overlay(
        overlay,
        discovery,
        attached_target,
        &snapshot,
        &mut rendered_model,
        &mut hover_zone,
        transition_target,
        transition_progress,
        reveal_progress,
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

        let clicked_chevron = CHEVRON_CLICKED.swap(0, Ordering::AcqRel);
        match clicked_chevron {
            CHEVRON_DOWN if !DETAILS_EXPANDED.load(Ordering::Acquire) => {
                DETAILS_EXPANDED.store(true, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
            CHEVRON_UP if DETAILS_EXPANDED.load(Ordering::Acquire) => {
                DETAILS_EXPANDED.store(false, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
            _ => {}
        }

        let mut usage_changed = false;
        while let Ok(next_snapshot) = runtime_receiver.try_recv() {
            snapshot = next_snapshot;
            usage_changed = true;
        }

        let now = Instant::now();
        let target_expanded = DETAILS_EXPANDED.load(Ordering::Acquire);
        let transition_target_changed = target_expanded != transition_target;
        if transition_target_changed {
            transition_target = target_expanded;
            transition_from = transition_progress;
            transition_started = now;
        }
        let previous_progress = transition_progress;
        let (next_progress, transition_active) = transition_value(
            transition_from,
            if transition_target { 1.0 } else { 0.0 },
            now.duration_since(transition_started),
            TRANSITION_DURATION_MS,
        );
        transition_progress = next_progress;
        let transition_changed = (transition_progress - previous_progress).abs() > f32::EPSILON;
        let move_size_active = MOVE_SIZE_ACTIVE.load(Ordering::Acquire);
        if move_size_active {
            hover.pause_for_move_size(now);
        }
        let activation_inside = if move_size_active {
            false
        } else {
            cursor_in_activation_area(hover_zone)
        };
        let reveal_engaged = if move_size_active {
            matches!(
                hover.state,
                HoverState::Compact | HoverState::CollapsePending
            )
        } else {
            !matches!(hover.state, HoverState::Collapsed)
        };
        let next_reveal_target =
            if transition_target || transition_progress > f32::EPSILON || reveal_engaged {
                1.0
            } else if activation_inside {
                1.0
            } else {
                0.0
            };
        if (next_reveal_target - reveal_target).abs() > f32::EPSILON {
            reveal_target = next_reveal_target;
            reveal_from = reveal_progress;
            reveal_started = now;
        }
        let previous_reveal_progress = reveal_progress;
        let (next_reveal_progress, reveal_active) = transition_value(
            reveal_from,
            reveal_target,
            now.duration_since(reveal_started),
            REVEAL_DURATION_MS,
        );
        reveal_progress = next_reveal_progress;
        let reveal_changed = (reveal_progress - previous_reveal_progress).abs() > f32::EPSILON;
        let event_dirty = TRACKER_DIRTY.swap(false, Ordering::AcqRel);
        let fallback_due = last_sync.elapsed() >= Duration::from_millis(500);
        if event_dirty || fallback_due || usage_changed || transition_changed || reveal_changed {
            let previous_target = *attached_target;
            last_placement = sync_overlay(
                overlay,
                discovery,
                attached_target,
                &snapshot,
                &mut rendered_model,
                &mut hover_zone,
                transition_target,
                transition_progress,
                reveal_progress,
            );
            let target_changed = previous_target.is_some() && previous_target != *attached_target;
            if (target_changed || last_placement.is_none()) && hover.reset_collapsed() {
                DETAILS_EXPANDED.store(false, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
            last_sync = Instant::now();
        }

        if event_dirty
            && CODEX_FOREGROUND_ACTIVE.load(Ordering::Acquire)
            && last_placement.is_some()
        {
            unsafe {
                let _ = reassert_overlay_z_order(overlay);
            }
        }

        if !move_size_active {
            if last_placement.is_some() {
                let inside = cursor_in_activation_area(hover_zone);
                if let Some(expanded) = hover.update(inside, Instant::now()) {
                    if !expanded {
                        DETAILS_EXPANDED.store(false, Ordering::Release);
                    }
                    TRACKER_DIRTY.store(true, Ordering::Release);
                }
            } else if hover.reset_collapsed() {
                DETAILS_EXPANDED.store(false, Ordering::Release);
                TRACKER_DIRTY.store(true, Ordering::Release);
            }
        }

        std::thread::sleep(Duration::from_millis(
            if transition_active || reveal_active {
                16
            } else {
                40
            },
        ));
    }

    hide_overlay(overlay);
    clear_attached_target(attached_target);
}

#[cfg(windows)]
unsafe extern "system" fn native_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, HTCLIENT, HTTRANSPARENT, MA_NOACTIVATE, WM_ERASEBKGND, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_NCHITTEST,
    };

    match message {
        WM_ERASEBKGND => windows::Win32::Foundation::LRESULT(1),
        WM_MOUSEACTIVATE => windows::Win32::Foundation::LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => {
            let x = (lparam.0 as i32) as i16 as i32;
            let y = ((lparam.0 >> 16) as i32) as i16 as i32;
            let action = chevron_action_at(x, y);
            if action != 0 {
                windows::Win32::Foundation::LRESULT(HTCLIENT as isize)
            } else {
                windows::Win32::Foundation::LRESULT(HTTRANSPARENT as isize)
            }
        }
        WM_LBUTTONDOWN => {
            let point = client_message_point(hwnd, lparam);
            let action = point
                .map(|point| chevron_action_at(point.x, point.y))
                .unwrap_or(0);
            if action != 0 {
                CHEVRON_PRESSED.store(action, Ordering::Release);
                let _ = SetCapture(hwnd);
                windows::Win32::Foundation::LRESULT(0)
            } else {
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
        }
        WM_LBUTTONUP => {
            let pressed = CHEVRON_PRESSED.swap(0, Ordering::AcqRel);
            let _ = ReleaseCapture();
            if let Some(point) = client_message_point(hwnd, lparam) {
                let action = chevron_action_at(point.x, point.y);
                if pressed != 0 && action == pressed {
                    CHEVRON_CLICKED.store(pressed, Ordering::Release);
                    return windows::Win32::Foundation::LRESULT(0);
                }
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_CAPTURECHANGED => {
            if CHEVRON_PRESSED.swap(0, Ordering::AcqRel) != 0 {
                CHEVRON_CLICKED.store(0, Ordering::Release);
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[cfg(windows)]
unsafe fn client_message_point(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> Option<windows::Win32::Foundation::POINT> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut point = POINT {
        x: (lparam.0 as i32 as i16) as i32,
        y: ((lparam.0 as i32 >> 16) as i16) as i32,
    };
    let mut bounds = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut bounds) }.is_err() {
        return None;
    }
    point.x += bounds.left;
    point.y += bounds.top;
    Some(point)
}

#[cfg(windows)]
fn chevron_action_at(x: i32, y: i32) -> u32 {
    let action = CHEVRON_ACTION.load(Ordering::Acquire);
    if action == 0 {
        return 0;
    }

    let left = CHEVRON_HIT_LEFT.load(Ordering::Acquire) as i32;
    let top = CHEVRON_HIT_TOP.load(Ordering::Acquire) as i32;
    let right = CHEVRON_HIT_RIGHT.load(Ordering::Acquire) as i32;
    let bottom = CHEVRON_HIT_BOTTOM.load(Ordering::Acquire) as i32;
    if x >= left && x < right && y >= top && y < bottom {
        action
    } else {
        0
    }
}

#[cfg(windows)]
fn clear_chevron_hit_region() {
    CHEVRON_HIT_LEFT.store(0, Ordering::Release);
    CHEVRON_HIT_TOP.store(0, Ordering::Release);
    CHEVRON_HIT_RIGHT.store(0, Ordering::Release);
    CHEVRON_HIT_BOTTOM.store(0, Ordering::Release);
    CHEVRON_ACTION.store(0, Ordering::Release);
}

#[cfg(windows)]
fn reset_chevron_interaction() {
    CHEVRON_CLICKED.store(0, Ordering::Release);
    CHEVRON_PRESSED.store(0, Ordering::Release);
}

#[cfg(windows)]
fn set_chevron_hit_rect(bounds: Rect, expanded: bool, compact_design_width: i32) {
    let hit_rect = chevron_hit_rect(bounds, expanded, compact_design_width);

    CHEVRON_HIT_LEFT.store(hit_rect.left as isize, Ordering::Release);
    CHEVRON_HIT_TOP.store(hit_rect.top as isize, Ordering::Release);
    CHEVRON_HIT_RIGHT.store(hit_rect.right as isize, Ordering::Release);
    CHEVRON_HIT_BOTTOM.store(hit_rect.bottom as isize, Ordering::Release);
    CHEVRON_ACTION.store(
        if expanded { CHEVRON_UP } else { CHEVRON_DOWN },
        Ordering::Release,
    );
}

#[cfg(windows)]
fn chevron_hit_rect(bounds: Rect, expanded: bool, compact_design_width: i32) -> Rect {
    let (design_width, design_height, hit_top, hit_bottom) = if expanded {
        (320, 92, 0, 28)
    } else {
        let center = NATIVE_SIZE.height / 2;
        (
            compact_design_width,
            NATIVE_SIZE.height,
            (center - 14).max(0),
            (center + 14).min(NATIVE_SIZE.height),
        )
    };
    let (hit_left_design, hit_right_design) = if expanded {
        (design_width - 30, design_width - 2)
    } else {
        (design_width - 32, design_width - 4)
    };
    let hit_left = bounds.left + (bounds.width() * hit_left_design / design_width);
    let hit_right = bounds.left + (bounds.width() * hit_right_design / design_width);
    let top = bounds.top + (bounds.height() * hit_top / design_height);
    let bottom = bounds.top + (bounds.height() * hit_bottom / design_height);
    Rect::new(hit_left, top, hit_right, bottom)
}

#[cfg(windows)]
unsafe extern "system" fn win_event_callback(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    event: u32,
    hwnd: windows::Win32::Foundation::HWND,
    _object: i32,
    _child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::{
        EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART,
    };

    let attached_target = ATTACHED_TARGET.load(Ordering::Acquire);
    let is_attached_target = attached_target != 0 && hwnd.0 as isize == attached_target;
    if event == EVENT_SYSTEM_FOREGROUND {
        CODEX_FOREGROUND_ACTIVE.store(is_attached_target, Ordering::Release);
    }
    if is_attached_target {
        if event == EVENT_SYSTEM_MOVESIZESTART {
            MOVE_SIZE_ACTIVE.store(true, Ordering::Release);
        } else if event == EVENT_SYSTEM_MOVESIZEEND {
            MOVE_SIZE_ACTIVE.store(false, Ordering::Release);
        }
    }
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
fn cursor_in_activation_area(hover_zone: Option<Rect>) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let Some(hover_zone) = hover_zone else {
        return false;
    };

    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return false;
    }

    cursor.x >= hover_zone.left
        && cursor.x < hover_zone.right
        && cursor.y >= hover_zone.top
        && cursor.y < hover_zone.bottom
}

#[cfg(windows)]
fn sync_overlay(
    overlay: windows::Win32::Foundation::HWND,
    discovery: &mut WindowDiscovery,
    attached_target: &mut Option<u64>,
    snapshot: &NativeRuntimeSnapshot,
    rendered_model: &mut Option<NativeRenderModel>,
    hover_zone: &mut Option<Rect>,
    expanded: bool,
    transition_progress: f32,
    reveal_progress: f32,
) -> Option<NotchPlacement> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, IsWindow, IsWindowVisible,
    };

    *hover_zone = None;
    clear_chevron_hit_region();

    if let Some(attached_hwnd) = *attached_target {
        let attached = HWND(attached_hwnd as *mut c_void);
        unsafe {
            if !IsWindow(Some(attached)).as_bool() {
                clear_attached_target(attached_target);
            } else if IsIconic(attached).as_bool() {
                hide_overlay(overlay);
                return None;
            }
        }
    }

    let Ok(discovery_snapshot) = discovery.refresh() else {
        hide_overlay_and_clear_target(overlay, attached_target);
        return None;
    };
    let Some(target) = discovery_snapshot.target else {
        hide_overlay_and_clear_target(overlay, attached_target);
        return None;
    };
    let target_hwnd = HWND(target.hwnd as *mut c_void);

    unsafe {
        if !IsWindow(Some(target_hwnd)).as_bool() {
            hide_overlay_and_clear_target(overlay, attached_target);
            return None;
        }
        if IsIconic(target_hwnd).as_bool() {
            hide_overlay(overlay);
            return None;
        }
    }
    let codex_foreground = unsafe { GetForegroundWindow().0 == target_hwnd.0 };
    CODEX_FOREGROUND_ACTIVE.store(codex_foreground, Ordering::Release);
    if !codex_foreground {
        hide_overlay(overlay);
        return None;
    }

    let Ok(geometry) = read_window_geometry(&target) else {
        hide_overlay(overlay);
        return None;
    };
    let compact_design_width = compact_width_for_snapshot(&snapshot.usage, &snapshot.credits);
    let collapsed_size = scale_size_for_dpi(
        WindowSize {
            width: compact_design_width,
            height: NATIVE_SIZE.height,
        },
        geometry.dpi,
    );
    let expanded_size = scale_size_for_dpi(
        WindowSize {
            width: EXPANDED_SIZE.width,
            height: expanded_height_for_window_count(snapshot.usage.windows.len()),
        },
        geometry.dpi,
    );
    let anchor = codex_top_center_anchor(geometry.frame_bounds, geometry.work_area, collapsed_size);
    let notch_size = interpolate_size(collapsed_size, expanded_size, transition_progress);
    let normal_placement = anchored_placement(
        anchor,
        geometry.frame_bounds,
        geometry.work_area,
        notch_size,
    );
    let expanded_anchor = anchored_placement(
        anchor,
        geometry.frame_bounds,
        geometry.work_area,
        expanded_size,
    );
    *hover_zone = Some(Rect::new(
        expanded_anchor.bounds.left - HOVER_ZONE_PADDING_X,
        anchor.bounds.top - HOVER_ZONE_PADDING_TOP,
        expanded_anchor.bounds.right + HOVER_ZONE_PADDING_X,
        expanded_anchor.bounds.bottom + HOVER_ZONE_PADDING_BOTTOM,
    ));
    let edge_tab =
        !expanded && transition_progress <= f32::EPSILON && reveal_progress <= f32::EPSILON;
    let placement = if expanded || transition_progress > f32::EPSILON {
        normal_placement
    } else {
        partially_hidden_placement(normal_placement, reveal_progress)
    };
    let mut model = build_render_model(&snapshot.usage, &snapshot.credits, expanded, notch_size);
    model.edge_tab = edge_tab;
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
            *attached_target = Some(target.hwnd);
            ATTACHED_TARGET.store(target_hwnd.0 as isize, Ordering::Release);
        }
        if IsWindowVisible(overlay).as_bool() {
            position_overlay(overlay, &placement)
        } else {
            show_overlay(overlay, &placement)
        }
    };
    if !positioned {
        hide_overlay(overlay);
        None
    } else {
        let interactive_surface =
            reveal_progress > f32::EPSILON || transition_progress > f32::EPSILON || expanded;
        if interactive_surface {
            set_chevron_hit_rect(placement.bounds, expanded, compact_design_width);
        }
        Some(placement)
    }
}

#[cfg(windows)]
fn codex_top_center_anchor(
    target_frame: Rect,
    work_area: Rect,
    size: WindowSize,
) -> NotchPlacement {
    let width = size.width.clamp(1, work_area.width().max(1));
    let center_x = target_frame.left + target_frame.width() / 2;
    let left = (center_x - width / 2).clamp(work_area.left, work_area.right - width);
    let bounds = Rect::new(
        left,
        target_frame.top,
        left + width,
        target_frame.top + size.height,
    );
    NotchPlacement {
        outside_target_frame: false,
        bounds,
    }
}

#[cfg(windows)]
fn anchored_placement(
    anchor: NotchPlacement,
    target_frame: Rect,
    work_area: Rect,
    size: WindowSize,
) -> NotchPlacement {
    let center_x = anchor.bounds.left + anchor.bounds.width() / 2;
    let width = size.width.clamp(1, work_area.width().max(1));
    let height = size.height.clamp(1, work_area.height().max(1));
    let left = (center_x - width / 2).clamp(work_area.left, work_area.right - width);
    let top = anchor.bounds.top;
    let bounds = Rect::new(left, top, left + width, top + height);
    NotchPlacement {
        outside_target_frame: bounds.bottom <= target_frame.top,
        bounds,
    }
}

#[cfg(windows)]
fn transition_value(
    from: f32,
    to: f32,
    elapsed: std::time::Duration,
    duration_ms: u64,
) -> (f32, bool) {
    let linear = (elapsed.as_millis() as f32 / duration_ms as f32).clamp(0.0, 1.0);
    let eased = linear * linear * (3.0 - 2.0 * linear);
    (from + (to - from) * eased, linear < 1.0)
}

#[cfg(windows)]
fn partially_hidden_placement(placement: NotchPlacement, progress: f32) -> NotchPlacement {
    let height = placement.bounds.height();
    let hidden_top = placement.bounds.top - height + HIDDEN_VISIBLE_HEIGHT.min(height);
    let top = interpolate_coordinate(hidden_top, placement.bounds.top, progress);
    NotchPlacement {
        bounds: Rect::new(
            placement.bounds.left,
            top,
            placement.bounds.right,
            top + height,
        ),
        outside_target_frame: placement.outside_target_frame,
    }
}

#[cfg(windows)]
fn interpolate_coordinate(from: i32, to: i32, progress: f32) -> i32 {
    (from as f32 + (to - from) as f32 * progress.clamp(0.0, 1.0)).round() as i32
}

#[cfg(windows)]
fn interpolate_size(collapsed: WindowSize, expanded: WindowSize, progress: f32) -> WindowSize {
    fn interpolate(from: i32, to: i32, progress: f32) -> i32 {
        (from as f32 + (to - from) as f32 * progress.clamp(0.0, 1.0)).round() as i32
    }

    WindowSize {
        width: interpolate(collapsed.width, expanded.width, progress),
        height: interpolate(collapsed.height, expanded.height, progress),
    }
}

#[cfg(windows)]
unsafe fn show_overlay(
    overlay: windows::Win32::Foundation::HWND,
    placement: &NotchPlacement,
) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_TOP, SWP_NOACTIVATE, SW_SHOWNOACTIVATE,
    };

    if SetWindowPos(
        overlay,
        Some(HWND_TOP),
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
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE};

    SetWindowPos(
        overlay,
        Some(HWND_TOP),
        placement.bounds.left,
        placement.bounds.top,
        placement.bounds.width(),
        placement.bounds.height(),
        SWP_NOACTIVATE,
    )
    .is_ok()
}

#[cfg(windows)]
unsafe fn reassert_overlay_z_order(overlay: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    SetWindowPos(
        overlay,
        Some(HWND_TOP),
        0,
        0,
        0,
        0,
        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
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
fn clear_attached_target(attached_target: &mut Option<u64>) {
    if attached_target.take().is_some() {
        ATTACHED_TARGET.store(0, Ordering::Release);
        MOVE_SIZE_ACTIVE.store(false, Ordering::Release);
    }
}

#[cfg(windows)]
fn hide_overlay_and_clear_target(
    overlay: windows::Win32::Foundation::HWND,
    attached_target: &mut Option<u64>,
) {
    hide_overlay(overlay);
    clear_attached_target(attached_target);
}

#[cfg(all(test, windows))]
mod tests {
    use super::{
        anchored_placement, chevron_hit_rect, codex_top_center_anchor, interpolate_size,
        partially_hidden_placement, transition_value, HoverController, HoverState,
        NativeRuntimeSnapshot, HIDDEN_VISIBLE_HEIGHT, HOVER_COLLAPSE_DELAY_MS,
        HOVER_EXPAND_DELAY_MS, REVEAL_DURATION_MS, TRANSITION_DURATION_MS,
    };
    use crate::engine::{CreditSnapshot, CreditStatus, RuntimeStatus, UsageSnapshot};
    use crate::window_geometry::{NotchPlacement, Rect, WindowSize};
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
    fn transition_easing_preserves_endpoints_and_is_bounded() {
        let (start, active) = transition_value(0.0, 1.0, Duration::ZERO, TRANSITION_DURATION_MS);
        assert_eq!(start, 0.0);
        assert!(active);

        let (middle, active) =
            transition_value(0.0, 1.0, Duration::from_millis(90), TRANSITION_DURATION_MS);
        assert!(middle > 0.0 && middle < 1.0);
        assert!(active);

        let (end, active) = transition_value(
            0.0,
            1.0,
            Duration::from_millis(TRANSITION_DURATION_MS),
            TRANSITION_DURATION_MS,
        );
        assert_eq!(end, 1.0);
        assert!(!active);
    }

    #[test]
    fn collapsed_reveal_keeps_only_a_bottom_strip_at_codex_attachment() {
        let normal = NotchPlacement {
            bounds: Rect::new(770, 100, 1150, 140),
            outside_target_frame: true,
        };
        let hidden = partially_hidden_placement(normal, 0.0);
        assert_eq!(hidden.bounds.top, 100 - 40 + HIDDEN_VISIBLE_HEIGHT);
        assert_eq!(hidden.bounds.bottom, 100 + HIDDEN_VISIBLE_HEIGHT);

        let revealed = partially_hidden_placement(normal, 1.0);
        assert_eq!(revealed.bounds, normal.bounds);
    }

    #[test]
    fn codex_anchor_uses_outer_frame_top_without_work_area_y_clamp() {
        let anchor = codex_top_center_anchor(
            Rect::new(100, 300, 1500, 1200),
            Rect::new(0, 0, 1920, 1080),
            WindowSize {
                width: 380,
                height: 40,
            },
        );

        assert_eq!(anchor.bounds, Rect::new(610, 300, 990, 340));
    }

    #[test]
    fn expanded_placement_keeps_the_collapsed_top_anchor() {
        let anchor = NotchPlacement {
            bounds: Rect::new(770, 100, 1150, 140),
            outside_target_frame: true,
        };
        let expanded = anchored_placement(
            anchor,
            Rect::new(100, 200, 1500, 1200),
            Rect::new(0, 0, 1920, 1080),
            WindowSize {
                width: 320,
                height: 92,
            },
        );

        assert_eq!(expanded.bounds.top, anchor.bounds.top);
        assert_eq!(
            (expanded.bounds.left + expanded.bounds.right) / 2,
            (anchor.bounds.left + anchor.bounds.right) / 2
        );
    }

    #[test]
    fn chevron_hit_region_stays_small_and_on_the_right_edge() {
        let compact = chevron_hit_rect(Rect::new(770, 100, 1150, 140), false, 380);
        assert_eq!(compact, Rect::new(1118, 106, 1146, 134));

        let expanded = chevron_hit_rect(Rect::new(800, 100, 1120, 192), true, 380);
        assert_eq!(expanded, Rect::new(1090, 100, 1118, 128));
    }

    #[test]
    fn reveal_transition_uses_a_lightweight_duration() {
        let (value, active) = transition_value(
            0.0,
            1.0,
            Duration::from_millis(REVEAL_DURATION_MS),
            REVEAL_DURATION_MS,
        );
        assert_eq!(value, 1.0);
        assert!(!active);
    }

    #[test]
    fn transition_size_preserves_collapsed_and_expanded_footprints() {
        let collapsed = WindowSize {
            width: 380,
            height: 40,
        };
        let expanded = WindowSize {
            width: 320,
            height: 92,
        };

        assert_eq!(interpolate_size(collapsed, expanded, 0.0), collapsed);
        assert_eq!(interpolate_size(collapsed, expanded, 1.0), expanded);
        assert_eq!(interpolate_size(collapsed, expanded, 0.5).height, 66);
    }

    #[test]
    fn hover_state_machine_delays_expand_and_collapse_and_cancels_leave() {
        let start = Instant::now();
        let mut controller = HoverController::new();

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
        assert_eq!(controller.state, HoverState::Compact);

        assert_eq!(
            controller.update(false, start + Duration::from_millis(200)),
            None
        );
        assert_eq!(controller.state, HoverState::CollapsePending);
        assert_eq!(
            controller.update(true, start + Duration::from_millis(250)),
            None
        );
        assert_eq!(controller.state, HoverState::Compact);

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

    #[test]
    fn move_size_pause_preserves_hover_delay_after_drag() {
        let start = Instant::now();
        let mut controller = HoverController::new();

        assert_eq!(controller.update(true, start), None);
        controller.pause_for_move_size(start + Duration::from_millis(100));

        assert_eq!(
            controller.update(
                true,
                start + Duration::from_millis(100 + HOVER_EXPAND_DELAY_MS - 1)
            ),
            None
        );
        assert_eq!(
            controller.update(
                true,
                start + Duration::from_millis(100 + HOVER_EXPAND_DELAY_MS)
            ),
            Some(true)
        );
    }
}
