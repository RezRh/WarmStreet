// commands.rs — Tauri command handlers for tauri-plugin-nativemap
//
// These commands handle map operations for the native map plugin.

use crate::models::{MapConfig, MapError, MapOutput, MapPin};
use tauri::{AppHandle, Emitter, Runtime, State};
use crate::NativeMapState;

// ---------------------------------------------------------------------------
// show_map
// ---------------------------------------------------------------------------

/// Render the native map view with the supplied configuration.
/// On iOS   → calls NativeMapPlugin.swift showMap()
/// On Android → calls NativeMapPlugin.kt showMap()
/// On Desktop → emits "map-ready" immediately (placeholder behaviour)
#[tauri::command]
pub async fn show_map<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, NativeMapState>,
    config: MapConfig,
) -> Result<MapOutput, MapError> {
    tracing::info!("🗺️ show_map called: center=({}, {}), zoom={}", config.center_lat, config.center_lon, config.zoom);

    {
        let mut visible = state.is_visible.lock().unwrap();
        *visible = true;
    }

    // On real mobile devices the Swift / Kotlin layer handles this command and
    // emits the "map-ready" Tauri event.  On desktop we emit it ourselves so the
    // Crux core still receives the MapReady event and sends initial pins.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        tracing::debug!("Desktop mode: emitting map-ready stub");
        app.emit("map-ready", ()).ok();
    }

    Ok(MapOutput::Ready)
}

// ---------------------------------------------------------------------------
// update_pins
// ---------------------------------------------------------------------------

/// Replace all annotation pins on the visible map.
#[tauri::command]
pub async fn update_pins<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, NativeMapState>,
    pins: Vec<MapPin>,
) -> Result<MapOutput, MapError> {
    tracing::info!("📍 update_pins called: {} pins", pins.len());

    let visible = *state.is_visible.lock().unwrap();
    if !visible {
        tracing::warn!("update_pins called while map is hidden — ignoring");
        return Ok(MapOutput::PinsUpdated);
    }

    // Emit the pin data as a Tauri event so the native layers (Swift/Kotlin)
    // can pick it up and render the annotations.
    app.emit("map-update-pins", &pins)
        .map_err(|e| MapError::OperationFailed { message: e.to_string() })?;

    Ok(MapOutput::PinsUpdated)
}

// ---------------------------------------------------------------------------
// hide_map
// ---------------------------------------------------------------------------

/// Remove the native map view.
#[tauri::command]
pub async fn hide_map<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, NativeMapState>,
) -> Result<MapOutput, MapError> {
    tracing::info!("🗺️ hide_map called");

    {
        let mut visible = state.is_visible.lock().unwrap();
        *visible = false;
    }

    app.emit("map-hide", ()).ok();
    Ok(MapOutput::Hidden)
}

// ---------------------------------------------------------------------------
// pan_to_location
// ---------------------------------------------------------------------------

/// Move the map camera to a specific coordinate.
#[tauri::command]
pub async fn pan_to_location<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, NativeMapState>,
    lat: f64,
    lon: f64,
) -> Result<MapOutput, MapError> {
    tracing::debug!("🧭 pan_to_location: ({lat}, {lon})");

    let visible = *state.is_visible.lock().unwrap();
    if !visible {
        return Ok(MapOutput::CameraUpdated);
    }

    app.emit("map-pan", serde_json::json!({ "lat": lat, "lon": lon }))
        .map_err(|e| MapError::OperationFailed { message: e.to_string() })?;

    Ok(MapOutput::CameraUpdated)
}

// ---------------------------------------------------------------------------
// set_zoom
// ---------------------------------------------------------------------------

/// Change the map zoom level without panning.
#[tauri::command]
pub async fn set_zoom<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, NativeMapState>,
    level: f64,
) -> Result<MapOutput, MapError> {
    tracing::debug!("🔍 set_zoom: {level}");

    let visible = *state.is_visible.lock().unwrap();
    if !visible {
        return Ok(MapOutput::CameraUpdated);
    }

    app.emit("map-zoom", serde_json::json!({ "level": level }))
        .map_err(|e| MapError::OperationFailed { message: e.to_string() })?;

    Ok(MapOutput::CameraUpdated)
}
