pub mod engine;
pub mod native_overlay;
pub mod native_renderer;
pub mod settings;
pub mod startup;
pub mod window_geometry;
pub mod window_tracker;

pub use engine::UsageSnapshot;
pub use settings::{resolve_theme, AppSettings, Theme};
pub use window_geometry::{
    place_top_center, read_window_geometry, scale_size_for_dpi, NotchPlacement, Rect, WindowDpi,
    WindowGeometrySnapshot, WindowSize,
};
pub use window_tracker::{WindowDiscovery, WindowDiscoverySnapshot};

#[tauri::command]
fn read_usage() -> Result<UsageSnapshot, String> {
    engine::Engine::default()
        .read_with_recovery(None)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn discover_codex_window(
    state: tauri::State<'_, std::sync::Mutex<WindowDiscovery>>,
) -> Result<WindowDiscoverySnapshot, String> {
    state
        .lock()
        .map_err(|_| "window discovery state is unavailable".to_string())?
        .refresh()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_notch_expanded(expanded: bool) -> Result<(), String> {
    native_overlay::set_expanded(expanded);
    Ok(())
}

fn resolve_startup_settings(mut settings: AppSettings) -> AppSettings {
    settings.theme = resolve_theme(settings.theme);
    settings
}

pub fn run() {
    native_overlay::initialize_dpi_awareness();
    let loaded_settings = AppSettings::load();
    let settings = resolve_startup_settings(loaded_settings);
    if let Err(error) = startup::apply(settings.start_with_windows) {
        eprintln!("failed to update Windows startup registration: {error}");
    }
    tauri::Builder::default()
        .manage(std::sync::Mutex::new(WindowDiscovery::default()))
        .manage(std::sync::Mutex::new(settings.clone()))
        .setup(move |app| {
            #[cfg(windows)]
            {
                use tauri::Manager;

                let window = app
                    .get_webview_window("main")
                    .ok_or_else(|| "main Notch window is unavailable".to_string())?;
                window
                    .hide()
                    .map_err(|error| format!("failed to hide Tauri controller window: {error}"))?;
                native_overlay::start(settings);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_usage,
            discover_codex_window,
            set_notch_expanded
        ])
        .build(tauri::generate_context!())
        .expect("error while building Codex Usage Notch")
        .run(|_, event| {
            #[cfg(windows)]
            if matches!(event, tauri::RunEvent::Exit) {
                native_overlay::stop();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{resolve_startup_settings, AppSettings, Theme};

    #[test]
    fn startup_settings_pass_a_concrete_theme_to_runtime() {
        let mut settings = AppSettings::default();
        settings.theme = Theme::Dark;

        let resolved = resolve_startup_settings(settings);

        assert_eq!(resolved.theme, Theme::Dark);
        assert_ne!(resolved.theme, Theme::System);
    }
}
