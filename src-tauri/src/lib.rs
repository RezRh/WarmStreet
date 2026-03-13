pub fn run_app() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::Debug)
        .init();

    tracing::info!("Starting WarmStreet Tauri Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running WarmStreet");
}
