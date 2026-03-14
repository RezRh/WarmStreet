// lib.rs — tauri-plugin-nativemap
//
// This Tauri plugin bridges the Crux MapCapability to platform-native map SDKs:
//   iOS    → Apple Maps (MKMapView) via Swift code in /ios/Sources/NativeMapPlugin.swift
//   Android → Google Maps SDK via Kotlin code in /android/src/.../NativeMapPlugin.kt
//   Desktop → No-op (the UI shows the static placeholder image)
//
// The plugin exposes three Tauri commands that the Tauri shell calls when Crux
// emits a MapOperation effect:
//   show_map(config: MapConfig) → MapResult
//   update_pins(pins: [MapPin]) → MapResult
//   hide_map()                  → MapResult
//
// The Swift / Kotlin code emits a "map-pin-tapped" Tauri event back to Rust
// when the user taps a pin, and Rust forwards it into Crux as a MapPinTapped event.

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
            // Store a handle so commands can emit events back.
            app.manage(NativeMapState::default());
            Ok(())
        })
        .build()
}

/// Plugin-level state (currently a placeholder; future: track shown/hidden status).
#[derive(Default)]
pub struct NativeMapState {
    pub is_visible: std::sync::Mutex<bool>,
}
