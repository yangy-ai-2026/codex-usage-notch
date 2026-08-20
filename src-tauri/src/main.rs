#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    let _instance_guard = match quotastrip_lib::single_instance::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => return,
        Err(error) => {
            eprintln!("failed to acquire QuotaStrip single-instance guard: {error}");
            return;
        }
    };

    quotastrip_lib::run();
}
