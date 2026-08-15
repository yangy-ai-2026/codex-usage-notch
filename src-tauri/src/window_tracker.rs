use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowCandidate {
    pub(crate) hwnd: u64,
    pub(crate) process_id: u32,
    pub(crate) is_foreground: bool,
    pub(crate) area: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTarget {
    pub hwnd: u64,
    pub process_id: u32,
}

impl From<&WindowCandidate> for WindowTarget {
    fn from(candidate: &WindowCandidate) -> Self {
        Self {
            hwnd: candidate.hwnd,
            process_id: candidate.process_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowDiscoveryStatus {
    NoTarget,
    Attached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowDiscoverySnapshot {
    pub status: WindowDiscoveryStatus,
    pub target: Option<WindowTarget>,
    pub candidate_count: usize,
}

#[derive(Debug, Error)]
pub enum WindowDiscoveryError {
    #[error("Windows window discovery is unavailable on this platform")]
    UnsupportedPlatform,
    #[cfg(windows)]
    #[error("failed to enumerate top-level windows: {0}")]
    Enumeration(#[source] windows::core::Error),
}

#[derive(Debug, Default)]
pub struct WindowDiscovery {
    attached_hwnd: Option<u64>,
}

impl WindowDiscovery {
    pub fn refresh(&mut self) -> Result<WindowDiscoverySnapshot, WindowDiscoveryError> {
        let candidates = platform::enumerate_codex_windows()?;
        Ok(self.apply_candidates(&candidates))
    }

    fn apply_candidates(&mut self, candidates: &[WindowCandidate]) -> WindowDiscoverySnapshot {
        let target = select_candidate(candidates, self.attached_hwnd);
        self.attached_hwnd = target.as_ref().map(|candidate| candidate.hwnd);

        WindowDiscoverySnapshot {
            status: if target.is_some() {
                WindowDiscoveryStatus::Attached
            } else {
                WindowDiscoveryStatus::NoTarget
            },
            target: target.as_ref().map(WindowTarget::from),
            candidate_count: candidates.len(),
        }
    }
}

pub(crate) fn select_candidate(
    candidates: &[WindowCandidate],
    attached_hwnd: Option<u64>,
) -> Option<WindowCandidate> {
    if let Some(attached) = attached_hwnd {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.hwnd == attached)
        {
            return Some(candidate.clone());
        }
    }

    candidates
        .iter()
        .filter(|candidate| candidate.is_foreground)
        .min_by_key(|candidate| candidate.hwnd)
        .cloned()
        .or_else(|| {
            candidates
                .iter()
                .min_by_key(|candidate| (std::cmp::Reverse(candidate.area), candidate.hwnd))
                .cloned()
        })
}

#[cfg(windows)]
mod platform {
    use super::{WindowCandidate, WindowDiscoveryError};
    use std::ffi::c_void;
    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetAncestor, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
        IsWindowVisible, GA_ROOTOWNER,
    };

    pub(super) fn enumerate_codex_windows() -> Result<Vec<WindowCandidate>, WindowDiscoveryError> {
        let foreground_hwnd = unsafe { root_owner(GetForegroundWindow()) };
        let mut candidates: Vec<WindowCandidate> = Vec::new();

        unsafe {
            EnumWindows(
                Some(enum_window_callback),
                LPARAM(&mut candidates as *mut Vec<WindowCandidate> as isize),
            )
            .map_err(WindowDiscoveryError::Enumeration)?;
        }

        for candidate in &mut candidates {
            candidate.is_foreground = candidate.hwnd == hwnd_key(foreground_hwnd);
        }

        Ok(candidates)
    }

    unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let candidates = &mut *(lparam.0 as *mut Vec<WindowCandidate>);
        if let Some(candidate) = inspect_window(hwnd) {
            candidates.push(candidate);
        }
        true.into()
    }

    unsafe fn inspect_window(hwnd: HWND) -> Option<WindowCandidate> {
        if hwnd.0.is_null()
            || !IsWindowVisible(hwnd).as_bool()
            || root_owner(hwnd).0 != hwnd.0
            || is_cloaked(hwnd)
        {
            return None;
        }

        let mut process_id = 0;
        if GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32)) == 0
            || !is_codex_process(process_id)
        {
            return None;
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return None;
        }

        let width = i64::from(rect.right - rect.left).max(0) as u64;
        let height = i64::from(rect.bottom - rect.top).max(0) as u64;

        Some(WindowCandidate {
            hwnd: hwnd_key(hwnd),
            process_id,
            is_foreground: false,
            area: width.saturating_mul(height),
        })
    }

    unsafe fn is_codex_process(process_id: u32) -> bool {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) else {
            return false;
        };

        let mut buffer = [0u16; 32_768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);

        let Ok(()) = result else {
            return false;
        };

        let path = String::from_utf16_lossy(&buffer[..length as usize]).to_ascii_lowercase();
        path.contains("\\windowsapps\\openai.codex_") && path.ends_with("\\chatgpt.exe")
    }

    unsafe fn is_cloaked(hwnd: HWND) -> bool {
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .map(|_| cloaked != 0)
        .unwrap_or(false)
    }

    unsafe fn root_owner(hwnd: HWND) -> HWND {
        GetAncestor(hwnd, GA_ROOTOWNER)
    }

    fn hwnd_key(hwnd: HWND) -> u64 {
        hwnd.0 as usize as u64
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{WindowCandidate, WindowDiscoveryError};

    pub(super) fn enumerate_codex_windows() -> Result<Vec<WindowCandidate>, WindowDiscoveryError> {
        Err(WindowDiscoveryError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::{select_candidate, WindowCandidate, WindowDiscovery, WindowDiscoveryStatus};

    fn candidate(hwnd: u64, is_foreground: bool, area: u64) -> WindowCandidate {
        WindowCandidate {
            hwnd,
            process_id: hwnd as u32,
            is_foreground,
            area,
        }
    }

    #[test]
    fn no_candidates_produce_no_target() {
        assert_eq!(select_candidate(&[], None), None);
    }

    #[test]
    fn existing_valid_target_wins_over_foreground_candidate() {
        let candidates = [candidate(10, false, 100), candidate(20, true, 900)];

        assert_eq!(select_candidate(&candidates, Some(10)).unwrap().hwnd, 10);
    }

    #[test]
    fn foreground_candidate_wins_when_attached_target_is_gone() {
        let candidates = [candidate(10, false, 900), candidate(20, true, 100)];

        assert_eq!(select_candidate(&candidates, Some(30)).unwrap().hwnd, 20);
    }

    #[test]
    fn largest_candidate_is_deterministic_fallback() {
        let candidates = [candidate(20, false, 100), candidate(10, false, 900)];

        assert_eq!(select_candidate(&candidates, None).unwrap().hwnd, 10);
    }

    #[test]
    fn destroyed_target_is_cleared_until_a_new_candidate_is_found() {
        let mut discovery = WindowDiscovery::default();
        let first = discovery.apply_candidates(&[candidate(10, false, 100)]);
        assert_eq!(first.status, WindowDiscoveryStatus::Attached);

        let absent = discovery.apply_candidates(&[]);
        assert_eq!(absent.status, WindowDiscoveryStatus::NoTarget);
        assert_eq!(absent.target, None);

        let relaunched = discovery.apply_candidates(&[candidate(20, true, 100)]);
        assert_eq!(relaunched.status, WindowDiscoveryStatus::Attached);
        assert_eq!(relaunched.target.unwrap().hwnd, 20);
    }
}
