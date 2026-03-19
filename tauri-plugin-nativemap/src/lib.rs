// lib.rs — tauri-plugin-nativemap
//
// This Tauri plugin provides platform-native map functionality:
//   iOS    → Apple Maps (MKMapView) via Swift code in /ios/Sources/NativeMapPlugin.swift
//   Android → Google Maps SDK via Kotlin code in /android/src/.../NativeMapPlugin.kt
//   Desktop → No-op (emits map-ready immediately)
//
// The plugin exposes Tauri commands for map operations:
//   show_map, update_pins, hide_map, pan_to_location, set_zoom
//
// When the user taps a pin, the Swift/Kotlin layer emits "map-pin-tapped"
// which is forwarded to the app state.

mod commands;
mod models;

pub use models::{MapConfig, MapError, MapOutput, MapPin};

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

/// Build and register the plugin with the Tauri application.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("nativemap")
        .invoke_handler(tauri::generate_handler![
            commands::show_map,
            commands::update_pins,
            commands::hide_map,
            commands::pan_to_location,
            commands::set_zoom,
        ])
        .setup(|app, _api| {
            tracing::info!("🗺️ tauri-plugin-nativemap initialised");
            app.manage(NativeMapState::default());
            Ok(())
        })
        .build()
}

/// Plugin-level state for tracking map visibility
#[derive(Default)]
pub struct NativeMapState {
    pub is_visible: std::sync::Mutex<bool>,
}
