// models.rs — shared data types for tauri-plugin-nativemap
//
// These types are used for serializing map data over the Tauri IPC bridge.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Config + Pin
// ---------------------------------------------------------------------------

/// Configuration sent from Rust → native to initialise the map view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    pub center_lat: f64,
    pub center_lon: f64,
    pub zoom: f64,
    pub show_user_location: bool,
    pub search_radius_m: u32,
    pub satellite_mode: bool,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            center_lat: 0.0,
            center_lon: 0.0,
            zoom: 14.0,
            show_user_location: true,
            search_radius_m: 5000,
            satellite_mode: false,
        }
    }
}

/// A single case pin to place on the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapPin {
    /// Unique case identifier
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    /// Severity level for color coding: "Critical" | "High" | "Moderate" | "Low"
    pub severity: String,
    pub title: String,
    pub subtitle: Option<String>,
}

// ---------------------------------------------------------------------------
// Output + Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MapOutput {
    Ready,
    Hidden,
    PinsUpdated,
    CameraUpdated,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum MapError {
    #[error("permission denied")]
    PermissionDenied,
    #[error("not available on this platform")]
    NotAvailable,
    #[error("init failed: {reason}")]
    InitFailed { reason: String },
    #[error("operation failed: {message}")]
    OperationFailed { message: String },
}

pub type MapResult = Result<MapOutput, MapError>;
