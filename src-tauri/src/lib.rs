mod types;

use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_plugin_nativemap;

/// Initialize and run the Tauri application
pub fn run_app() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    tracing::info!("🚀 Starting WarmStreet - Native Backend");

    tauri::Builder::default()
        .plugin(tauri_plugin_stronghold::Builder::new(|_app_handle| {
            // Warning: For production, this secret should be retrieved from a secure 
            // OS keychain or provided by a user PIN.
            Ok("warmstreet-vault-enclave-secret-123".to_string())
        }).build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_nativemap::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Listen for "map-ready" event from native map layer
            let app_handle_clone = app_handle.clone();
            app_handle.listen("map-ready", move |_| {
                tracing::info!("🗺️ Native map initialized");

                // Emit to frontend that map is ready
                let _ = app_handle_clone.emit("map-ready", ());
            });

            // Listen for "map-pin-tapped" events from native map layer
            let app_handle_clone = app_handle.clone();
            app_handle.listen("map-pin-tapped", move |event| {
                let payload = event.payload();
                let case_id = payload.trim_matches('"').to_string();
                tracing::info!("📍 Map pin tapped: {}", case_id);

                // Forward to frontend
                let _ = app_handle_clone.emit("map-pin-tapped", case_id);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running WarmStreet");
}
