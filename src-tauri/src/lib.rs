use shared::{App, Model};
use std::sync::Mutex;

/// Global state: holds the Crux App instance and current model
pub struct CruxState {
    pub app: App,
    pub model: Mutex<Model>,
}

impl CruxState {
    pub fn new() -> Self {
        Self {
            app: App::default(),
            model: Mutex::new(Model::default()),
        }
    }
}

// TODO: Implement dispatch_event command handler
// This requires fixing the tauri::command macro duplication issue
// The command should:
// 1. Receive event string + payload from UI
// 2. Convert to shared::Event
// 3. Call app.update(event, &mut model, &capabilities)
// 4. Emit "crux-update" event with ViewModel to UI

/// Initialize and run the Tauri application
pub fn run_app() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("Starting WarmStreet Tauri Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(CruxState::new())
        // TODO: Add invoke handler for Crux event dispatching
        // .invoke_handler(tauri::generate_handler![dispatch_event])
        .run(tauri::generate_context!())
        .expect("error while running WarmStreet");
}
