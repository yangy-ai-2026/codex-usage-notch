pub mod engine;

pub use engine::UsageSnapshot;

#[tauri::command]
fn read_usage() -> Result<UsageSnapshot, String> {
    engine::Engine::default()
        .read_with_recovery(None)
        .map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_usage])
        .run(tauri::generate_context!())
        .expect("error while running Codex Usage Notch");
}
