const MUTEX_NAME: &str = r"Local\QuotaStrip.SingleInstance";

#[cfg(windows)]
pub struct SingleInstanceGuard(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
pub fn acquire() -> Result<Option<SingleInstanceGuard>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    let name = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR::from_raw(name.as_ptr())) }
        .map_err(|error| format!("CreateMutexW failed: {error}"))?;

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }
        return Ok(None);
    }

    Ok(Some(SingleInstanceGuard(handle)))
}

#[cfg(not(windows))]
pub struct SingleInstanceGuard;

#[cfg(not(windows))]
pub fn acquire() -> Result<Option<SingleInstanceGuard>, String> {
    Ok(Some(SingleInstanceGuard))
}

#[cfg(test)]
mod tests {
    use super::MUTEX_NAME;

    #[test]
    fn mutex_name_is_stable_and_quotastrip_specific() {
        assert_eq!(MUTEX_NAME, r"Local\QuotaStrip.SingleInstance");
    }
}
