use std::path::Path;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE_NAME: &str = "Codex Usage Notch";

#[derive(Debug, PartialEq, Eq)]
enum StartupAction {
    Register(String),
    Remove,
}

fn startup_action(enabled: bool, executable: &Path) -> StartupAction {
    if enabled {
        StartupAction::Register(format!("\"{}\"", executable.to_string_lossy()))
    } else {
        StartupAction::Remove
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn apply(enabled: bool) -> Result<(), String> {
    if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("failed to locate current executable: {error}"))?;
        register(&executable)
    } else {
        unregister()
    }
}

#[cfg(not(windows))]
pub fn apply(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn register(executable: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_SZ,
    };

    let key_path = wide(RUN_KEY_PATH);
    let value_name = wide(RUN_VALUE_NAME);
    let mut key = HKEY::default();
    let status = unsafe {
        RegCreateKeyW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(key_path.as_ptr()),
            &mut key,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(registry_error("opening startup registry key", status.0));
    }

    let value = match startup_action(true, executable) {
        StartupAction::Register(value) => value,
        StartupAction::Remove => unreachable!(),
    };
    let value_data: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let value_bytes = unsafe {
        std::slice::from_raw_parts(value_data.as_ptr().cast::<u8>(), value_data.len() * 2)
    };
    let status = unsafe {
        RegSetValueExW(
            key,
            PCWSTR::from_raw(value_name.as_ptr()),
            None,
            REG_SZ,
            Some(value_bytes),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(registry_error("writing startup registry value", status.0))
    }
}

#[cfg(windows)]
fn unregister() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    let key_path = wide(RUN_KEY_PATH);
    let value_name = wide(RUN_VALUE_NAME);
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(key_path.as_ptr()),
            None,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    if status != ERROR_SUCCESS {
        return Err(registry_error("opening startup registry key", status.0));
    }

    let status = unsafe { RegDeleteValueW(key, PCWSTR::from_raw(value_name.as_ptr())) };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(registry_error("deleting startup registry value", status.0))
    }
}

#[cfg(windows)]
fn registry_error(operation: &str, status: u32) -> String {
    format!("{operation} failed with Win32 error {status}")
}

#[cfg(test)]
mod tests {
    use super::{startup_action, StartupAction, RUN_KEY_PATH, RUN_VALUE_NAME};
    use std::path::Path;

    #[test]
    fn enabled_startup_registers_quoted_current_executable() {
        let executable = Path::new(r"C:\Program Files\Codex Usage Notch\notch.exe");

        assert_eq!(
            startup_action(true, executable),
            StartupAction::Register(
                r#""C:\Program Files\Codex Usage Notch\notch.exe""#.to_string()
            )
        );
    }

    #[test]
    fn disabled_startup_removes_the_existing_value() {
        assert_eq!(
            startup_action(false, Path::new(r"C:\notch.exe")),
            StartupAction::Remove
        );
    }

    #[test]
    fn uses_the_current_user_run_key_and_stable_value_name() {
        assert_eq!(
            RUN_KEY_PATH,
            r"Software\Microsoft\Windows\CurrentVersion\Run"
        );
        assert_eq!(RUN_VALUE_NAME, "Codex Usage Notch");
    }
}
