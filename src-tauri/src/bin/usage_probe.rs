fn main() {
    match codex_usage_notch_lib::engine::Engine::default().read_with_recovery(None) {
        Ok(snapshot) => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).expect("snapshot serialization")
        ),
        Err(error) => {
            eprintln!("usage_probe error: {error}");
            std::process::exit(1);
        }
    }
}
