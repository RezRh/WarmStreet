use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------
pub const DEFAULT_MAP_ZOOM: f64 = 14.0;
pub const MIN_MAP_ZOOM: f64 = 3.0;
pub const MAX_MAP_ZOOM: f64 = 20.0;

// ---------------------------------------------------------------------------
// Capability struct
// ---------------------------------------------------------------------------

/// Crux capability that sends map operations to the native shell.
/// The shell decides which SDK to use (MKMapView on iOS, Google Maps on Android).
#[derive(Clone)]
pub struct Map<E> {
    context: CapabilityContext<MapOperation, E>,
}

impl<Ev> Capability<Ev> for Map<Ev> {
    type Operation = MapOperation;
    type MappedSelf<MappedEv> = Map<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Map::new(self.context.map_event(f))
    }
}

impl<E> Map<E>
where
    E: 'static,
{
    pub fn new(context: CapabilityContext<MapOperation, E>) -> Self {
        Self { context }
    }

    /// Ask the shell to render the native map with the given config.
    /// `callback` will be called once the map is ready (or failed).
    pub fn show_map<F>(&self, config: MapConfig, callback: F)
    where
        F: FnOnce(MapResult) -> E + Send + 'static,
    {
        let config = config.validated();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx
                .request_from_shell(MapOperation::ShowMap { config })
                .await;
            ctx.update_app(callback(result));
        });
    }

    /// Tear down the native map view (e.g. when user navigates away).
    pub fn hide_map<F>(&self, callback: F)
    where
        F: FnOnce(MapResult) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx.request_from_shell(MapOperation::HideMap).await;
            ctx.update_app(callback(result));
        });
    }

    /// Replace all pins currently shown on the map.
    pub fn update_pins<F>(&self, pins: Vec<MapPin>, callback: F)
    where
        F: FnOnce(MapResult) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx
                .request_from_shell(MapOperation::UpdatePins { pins })
                .await;
            ctx.update_app(callback(result));
        });
    }

    /// Pan the map to a new centre point.
    pub fn pan_to<F>(&self, lat: f64, lon: f64, callback: F)
    where
        F: FnOnce(MapResult) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx
                .request_from_shell(MapOperation::PanToLocation { lat, lon })
                .await;
            ctx.update_app(callback(result));
        });
    }

    /// Change the zoom level without panning.
    pub fn set_zoom<F>(&self, level: f64, callback: F)
    where
        F: FnOnce(MapResult) -> E + Send + 'static,
    {
        let level = level.clamp(MIN_MAP_ZOOM, MAX_MAP_ZOOM);
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx
                .request_from_shell(MapOperation::SetZoom { level })
                .await;
            ctx.update_app(callback(result));
        });
    }
}

// ---------------------------------------------------------------------------
// Operation (shell request) + Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MapOperation {
    /// Show the native map view with the given initial config.
    ShowMap { config: MapConfig },
    /// Tear down the native map view.
    HideMap,
    /// Replace all annotation pins on the map.
    UpdatePins { pins: Vec<MapPin> },
    /// Pan the camera to a specific lat/lon.
    PanToLocation { lat: f64, lon: f64 },
    /// Set the zoom level (platform-specific zoom scale).
    SetZoom { level: f64 },
}

impl Operation for MapOperation {
    type Output = MapResult;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MapOutput {
    /// Map view is rendered and interactive.
    Ready,
    /// Map view has been hidden / destroyed.
    Hidden,
    /// Pins were updated successfully.
    PinsUpdated,
    /// Camera moved successfully.
    CameraUpdated,
}

pub type MapResult = Result<MapOutput, MapError>;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq)]
pub enum MapError {
    #[error("map permission denied — location access is required")]
    PermissionDenied,

    #[error("map SDK not available on this platform")]
    NotAvailable,

    #[error("map SDK initialisation failed: {reason}")]
    InitFailed { reason: String },

    #[error("invalid coordinate: lat={lat}, lon={lon}")]
    InvalidCoordinate { lat: f64, lon: f64 },

    #[error("map operation failed: {message}")]
    OperationFailed { message: String },
}

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

/// Initial configuration for the native map view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapConfig {
    /// Centre of the viewport on load.
    pub center_lat: f64,
    pub center_lon: f64,
    /// Map zoom level (platform zoom units, clamped to MIN/MAX).
    pub zoom: f64,
    /// Whether to show the blue "You are here" dot.
    pub show_user_location: bool,
    /// Search radius in metres — drawn as a semi-transparent circle.
    pub search_radius_m: u32,
    /// Start with satellite imagery instead of the standard map.
    pub satellite_mode: bool,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            center_lat: 0.0,
            center_lon: 0.0,
            zoom: DEFAULT_MAP_ZOOM,
            show_user_location: true,
            search_radius_m: 5000,
            satellite_mode: false,
        }
    }
}

impl MapConfig {
    /// Builder — set the viewport centre.
    #[must_use]
    pub fn with_center(mut self, lat: f64, lon: f64) -> Self {
        self.center_lat = lat;
        self.center_lon = lon;
        self
    }

    /// Builder — set zoom level (will be clamped).
    #[must_use]
    pub fn with_zoom(mut self, zoom: f64) -> Self {
        self.zoom = zoom;
        self
    }

    /// Builder — set search radius circle.
    #[must_use]
    pub fn with_radius(mut self, metres: u32) -> Self {
        self.search_radius_m = metres;
        self
    }

    /// Clamp values to valid ranges.
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.zoom = self.zoom.clamp(MIN_MAP_ZOOM, MAX_MAP_ZOOM);
        self.center_lat = self.center_lat.clamp(-90.0, 90.0);
        self.center_lon = self.center_lon.clamp(-180.0, 180.0);
        self
    }
}

/// A single annotation pin to be placed on the map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapPin {
    /// Unique ID — matches the Crux `CaseId`.
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    /// e.g. "Critical", "High", "Moderate", "Low" — shell uses this to pick pin colour.
    pub severity: String,
    /// Short descriptive label shown in the pin callout.
    pub title: String,
    /// Optional subtitle (e.g. animal type + breed).
    pub subtitle: Option<String>,
}

impl MapPin {
    #[must_use]
    pub fn new(id: impl Into<String>, lat: f64, lon: f64, severity: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            lat,
            lon,
            severity: severity.into(),
            title: title.into(),
            subtitle: None,
        }
    }

    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_config_defaults() {
        let config = MapConfig::default();
        assert_eq!(config.zoom, DEFAULT_MAP_ZOOM);
        assert!(config.show_user_location);
        assert!(!config.satellite_mode);
    }

    #[test]
    fn test_map_config_validation_clamps_zoom() {
        let config = MapConfig::default().with_zoom(999.0).validated();
        assert_eq!(config.zoom, MAX_MAP_ZOOM);

        let config = MapConfig::default().with_zoom(-5.0).validated();
        assert_eq!(config.zoom, MIN_MAP_ZOOM);
    }

    #[test]
    fn test_map_config_serialisation() {
        let config = MapConfig::default()
            .with_center(28.6139, 77.2090)
            .with_zoom(15.0)
            .with_radius(2000)
            .validated();
        let json = serde_json::to_string(&config).expect("serialise");
        let back: MapConfig = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(config, back);
    }

    #[test]
    fn test_map_pin_builder() {
        let pin = MapPin::new("case-1", 28.6, 77.2, "Critical", "Dog injured")
            .with_subtitle("Labrador • Adult");
        assert_eq!(pin.id, "case-1");
        assert_eq!(pin.severity, "Critical");
        assert_eq!(pin.subtitle, Some("Labrador • Adult".to_string()));
    }

    #[test]
    fn test_map_pin_serialisation() {
        let pin = MapPin::new("case-42", 19.076, 72.877, "High", "Cat with wound");
        let json = serde_json::to_string(&pin).expect("serialise");
        let back: MapPin = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(pin, back);
    }

    #[test]
    fn test_map_operation_serialisation() {
        let op = MapOperation::UpdatePins {
            pins: vec![MapPin::new("x", 0.0, 0.0, "Low", "test")],
        };
        let json = serde_json::to_string(&op).expect("serialise");
        let back: MapOperation = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(op, back);
    }
}
