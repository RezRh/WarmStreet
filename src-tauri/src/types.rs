// types.rs - Types for native feature integration
// These types are used for native map, camera, and offline storage features

#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};

// ============================================================================
// Coordinate Types (for native map integration)
// ============================================================================

/// Simple coordinate type for map pins
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}

// ============================================================================
// Native Map Types
// ============================================================================

/// Map pin data for native map layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapPin {
    pub case_id: String,
    pub lat: f64,
    pub lon: f64,
    pub severity: u8,
    pub status: String,
}

/// Map configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    pub center_lat: f64,
    pub center_lon: f64,
    pub zoom: f64,
    pub pins: Vec<MapPin>,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            center_lat: 0.0,
            center_lon: 0.0,
            zoom: 14.0,
            pins: Vec::new(),
        }
    }
}

// ============================================================================
// Camera/Image Types (for future native camera integration)
// ============================================================================

/// Captured image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedImage {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub timestamp: u64,
}

// ============================================================================
// Offline Storage Types (for future SQLite integration)
// ============================================================================

/// Cached data for offline mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData {
    pub key: String,
    pub data: String,
    pub timestamp: u64,
    pub expires_at: Option<u64>,
}
