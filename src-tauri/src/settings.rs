use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SETTINGS_DIRECTORY: &str = "QuotaStrip";
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::System
    }
}

pub fn resolve_theme(theme: Theme) -> Theme {
    match theme {
        Theme::System => system_theme(),
        explicit => explicit,
    }
}

#[cfg(windows)]
fn system_theme() -> Theme {
    use windows::core::w;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
        REG_DWORD, REG_VALUE_TYPE,
    };

    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            None,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if status != ERROR_SUCCESS {
        return Theme::Dark;
    }

    let mut value = 0u32;
    let mut value_type = REG_VALUE_TYPE::default();
    let mut value_size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegQueryValueExW(
            key,
            w!("AppsUseLightTheme"),
            None,
            Some(&mut value_type),
            Some((&mut value as *mut u32).cast()),
            Some(&mut value_size),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status == ERROR_SUCCESS
        && value_type == REG_DWORD
        && value_size == std::mem::size_of::<u32>() as u32
    {
        theme_from_apps_use_light_theme(value)
    } else {
        Theme::Dark
    }
}

#[cfg(not(windows))]
fn system_theme() -> Theme {
    Theme::Dark
}

fn theme_from_apps_use_light_theme(value: u32) -> Theme {
    if value == 1 {
        Theme::Light
    } else {
        Theme::Dark
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub start_with_windows: bool,
    pub auto_hide_when_codex_inactive: bool,
    pub hover_expansion_enabled: bool,
    pub position_x_offset: i32,
    pub position_y_offset: i32,
    pub theme: Theme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            auto_hide_when_codex_inactive: true,
            hover_expansion_enabled: true,
            position_x_offset: 0,
            position_y_offset: 0,
            theme: Theme::System,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        load_from_path(&settings_path())
    }

    pub fn save(&self) -> io::Result<()> {
        save_to_path(&settings_path(), self)
    }
}

pub fn settings_path() -> PathBuf {
    config_root().join(SETTINGS_DIRECTORY).join(SETTINGS_FILE)
}

fn load_from_path(path: &Path) -> AppSettings {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let contents = contents.strip_prefix('\u{feff}').unwrap_or(&contents);
            serde_json::from_str(contents).unwrap_or_default()
        }
        Err(_) => AppSettings::default(),
    }
}

fn save_to_path(path: &Path, settings: &AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_vec_pretty(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, contents)
}

fn config_root() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data);
        }
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            return PathBuf::from(user_profile).join("AppData").join("Local");
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(config_home);
        }
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(".config");
        }
    }

    env::temp_dir()
}

#[cfg(test)]
mod tests {
    use super::{
        load_from_path, resolve_theme, save_to_path, theme_from_apps_use_light_theme, AppSettings,
        Theme,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "quotastrip-settings-{label}-{}",
                std::process::id()
            ))
            .join(format!("{unique}.json"))
    }

    #[test]
    fn defaults_are_safe_and_preserve_current_behavior() {
        let settings = AppSettings::default();

        assert!(!settings.start_with_windows);
        assert!(settings.auto_hide_when_codex_inactive);
        assert!(settings.hover_expansion_enabled);
        assert_eq!(settings.position_x_offset, 0);
        assert_eq!(settings.position_y_offset, 0);
        assert_eq!(settings.theme, Theme::System);
    }

    #[test]
    fn explicit_themes_are_preserved_during_resolution() {
        assert_eq!(resolve_theme(Theme::Dark), Theme::Dark);
        assert_eq!(resolve_theme(Theme::Light), Theme::Light);
    }

    #[test]
    fn windows_app_theme_value_maps_to_matching_theme() {
        assert_eq!(theme_from_apps_use_light_theme(0), Theme::Dark);
        assert_eq!(theme_from_apps_use_light_theme(1), Theme::Light);
        assert_eq!(theme_from_apps_use_light_theme(2), Theme::Dark);
    }

    #[test]
    fn missing_config_falls_back_to_defaults() {
        let path = test_path("missing");
        assert_eq!(load_from_path(&path), AppSettings::default());
    }

    #[test]
    fn malformed_config_falls_back_to_defaults() {
        let path = test_path("malformed");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create test config directory");
        }
        fs::write(&path, b"{not-json").expect("write malformed config");

        assert_eq!(load_from_path(&path), AppSettings::default());
        let _ = fs::remove_dir_all(path.parent().expect("test config parent"));
    }

    #[test]
    fn utf8_bom_config_preserves_position_offsets() {
        let path = test_path("utf8-bom");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create test config directory");
        }
        fs::write(
            &path,
            b"\xEF\xBB\xBF{\"position_x_offset\":1500,\"position_y_offset\":80}",
        )
        .expect("write UTF-8 BOM config");

        let loaded = load_from_path(&path);
        assert_eq!(loaded.position_x_offset, 1500);
        assert_eq!(loaded.position_y_offset, 80);
        let _ = fs::remove_dir_all(path.parent().expect("test config parent"));
    }

    #[test]
    fn missing_fields_use_defaults_and_theme_serializes_as_wire_value() {
        let path = test_path("partial");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create test config directory");
        }
        fs::write(&path, br#"{"theme":"dark","position_x_offset":12}"#)
            .expect("write partial config");

        let loaded = load_from_path(&path);
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.position_x_offset, 12);
        assert_eq!(loaded.position_y_offset, 0);
        assert!(loaded.hover_expansion_enabled);
        assert_eq!(serde_json::to_string(&loaded.theme).unwrap(), "\"dark\"");
        let _ = fs::remove_dir_all(path.parent().expect("test config parent"));
    }

    #[test]
    fn save_then_load_preserves_settings_across_restart_boundary() {
        let path = test_path("round-trip");
        let expected = AppSettings {
            start_with_windows: true,
            auto_hide_when_codex_inactive: false,
            hover_expansion_enabled: false,
            position_x_offset: -8,
            position_y_offset: 14,
            theme: Theme::Light,
        };

        save_to_path(&path, &expected).expect("save settings");
        assert_eq!(load_from_path(&path), expected);
        let _ = fs::remove_dir_all(path.parent().expect("test config parent"));
    }
}
