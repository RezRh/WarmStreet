use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::event::Event;

pub const MAX_IMAGE_SIZE_BYTES: usize = 20 * 1024 * 1024;
pub const DEFAULT_JPEG_QUALITY: u8 = 85;
pub const DEFAULT_MAX_DIMENSION: u32 = 2048;

#[derive(Debug, Clone)]
pub struct Camera<E> {
    context: CapabilityContext<CameraOperation, E>,
}

impl<Ev> Capability<Ev> for Camera<Ev> {
    type Operation = CameraOperation;
    type MappedSelf<MappedEv> = Camera<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static + Send,
    {
        Camera::new(self.context.map_event(f))
    }
}

impl<E> Camera<E>
where
    E: 'static,
{
    pub fn new(context: CapabilityContext<CameraOperation, E>) -> Self {
        Self { context }
    }

    pub fn check_permission<F>(&self, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        self.context
            .request_from_shell(CameraOperation::CheckPermission, callback);
    }

    pub fn request_permission<F>(&self, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        self.context
            .request_from_shell(CameraOperation::RequestPermission, callback);
    }

    pub fn get_capabilities<F>(&self, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        self.context
            .request_from_shell(CameraOperation::GetCapabilities, callback);
    }

    pub fn capture_photo<F>(&self, config: CaptureConfig, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        let config = config.validated();
        self.context
            .request_from_shell(CameraOperation::CapturePhoto { config }, callback);
    }

    pub fn capture_photo_simple<F>(&self, facing: CameraFacing, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        self.capture_photo(CaptureConfig::default().with_facing(facing), callback);
    }

    pub fn pick_from_gallery<F>(&self, config: GalleryPickConfig, callback: F)
    where
        F: FnOnce(CameraResult) -> E + Send + 'static,
    {
        let config = config.validated();
        self.context
            .request_from_shell(CameraOperation::PickFromGallery { config }, callback);
    }

    pub fn cancel_pending(&self) {
        self.context
            .request_from_shell(CameraOperation::CancelPending, |_: CameraResult| {
                panic!("cancel should not produce event")
            });
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl<E> Default for Camera<E>
where
    E: 'static,
{
    fn default() -> Self {
        panic!("Camera::default() should only be used in test context with mocking")
    }
}

pub type CameraCapability = Camera<Event>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CameraOperation {
    CheckPermission,
    RequestPermission,
    GetCapabilities,
    CapturePhoto { config: CaptureConfig },
    PickFromGallery { config: GalleryPickConfig },
    CancelPending,
}

impl Operation for CameraOperation {
    type Output = CameraResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraFacing {
    Front,
    Back,
    External,
}

impl Default for CameraFacing {
    fn default() -> Self {
        CameraFacing::Back
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Heic,
    WebP,
}

impl ImageFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Heic => "image/heic",
            ImageFormat::WebP => "image/webp",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Heic => "heic",
            ImageFormat::WebP => "webp",
        }
    }

    pub fn supports_quality(&self) -> bool {
        matches!(self, ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Heic)
    }

    pub fn from_magic_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(ImageFormat::Jpeg);
        }

        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(ImageFormat::Png);
        }

        if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
            return Some(ImageFormat::WebP);
        }

        if data.len() >= 12 {
            let ftyp_offset = 4;
            if data.len() > ftyp_offset + 8 {
                let ftyp = &data[ftyp_offset..ftyp_offset + 4];
                if ftyp == b"ftyp" {
                    let brand = &data[ftyp_offset + 4..ftyp_offset + 8];
                    if brand == b"heic" || brand == b"heix" || brand == b"mif1" {
                        return Some(ImageFormat::Heic);
                    }
                }
            }
        }

        None
    }
}

impl Default for ImageFormat {
    fn default() -> Self {
        ImageFormat::Jpeg
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashMode {
    Off,
    On,
    Auto,
    Torch,
}

impl Default for FlashMode {
    fn default() -> Self {
        FlashMode::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectRatio {
    Square,
    Ratio4x3,
    Ratio16x9,
    Full,
}

impl Default for AspectRatio {
    fn default() -> Self {
        AspectRatio::Ratio4x3
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptureConfig {
    pub facing: CameraFacing,
    pub format: ImageFormat,
    pub quality: u8,
    pub max_width: u32,
    pub max_height: u32,
    pub flash: FlashMode,
    pub aspect_ratio: AspectRatio,
    pub strip_metadata: bool,
    pub mirror_front_camera: bool,
    pub timeout_ms: u64,
    pub max_file_size: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            facing: CameraFacing::Back,
            format: ImageFormat::Jpeg,
            quality: DEFAULT_JPEG_QUALITY,
            max_width: DEFAULT_MAX_DIMENSION,
            max_height: DEFAULT_MAX_DIMENSION,
            flash: FlashMode::Auto,
            aspect_ratio: AspectRatio::Ratio4x3,
            strip_metadata: true,
            mirror_front_camera: true,
            timeout_ms: 60_000,
            max_file_size: MAX_IMAGE_SIZE_BYTES,
        }
    }
}

impl CaptureConfig {
    pub fn with_facing(mut self, facing: CameraFacing) -> Self {
        self.facing = facing;
        self
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }

    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_width = width.max(1);
        self.max_height = height.max(1);
        self
    }

    pub fn with_flash(mut self, flash: FlashMode) -> Self {
        self.flash = flash;
        self
    }

    pub fn with_aspect_ratio(mut self, ratio: AspectRatio) -> Self {
        self.aspect_ratio = ratio;
        self
    }

    pub fn keep_metadata(mut self) -> Self {
        self.strip_metadata = false;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.max(1000).min(300_000);
        self
    }

    pub fn with_max_file_size(mut self, max_bytes: usize) -> Self {
        self.max_file_size = max_bytes.min(MAX_IMAGE_SIZE_BYTES);
        self
    }

    pub fn validated(mut self) -> Self {
        self.quality = self.quality.min(100);
        self.max_width = self.max_width.max(1);
        self.max_height = self.max_height.max(1);
        self.timeout_ms = self.timeout_ms.max(1000).min(300_000);
        self.max_file_size = self.max_file_size.min(MAX_IMAGE_SIZE_BYTES);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GalleryPickConfig {
    pub allow_multiple: bool,
    pub max_selections: u32,
    pub format: ImageFormat,
    pub quality: u8,
    pub max_width: u32,
    pub max_height: u32,
    pub strip_metadata: bool,
    pub max_file_size: usize,
}

impl Default for GalleryPickConfig {
    fn default() -> Self {
        Self {
            allow_multiple: false,
            max_selections: 1,
            format: ImageFormat::Jpeg,
            quality: DEFAULT_JPEG_QUALITY,
            max_width: DEFAULT_MAX_DIMENSION,
            max_height: DEFAULT_MAX_DIMENSION,
            strip_metadata: true,
            max_file_size: MAX_IMAGE_SIZE_BYTES,
        }
    }
}

impl GalleryPickConfig {
    pub fn single() -> Self {
        Self::default()
    }

    pub fn multiple(max: u32) -> Self {
        Self {
            allow_multiple: true,
            max_selections: max.max(1).min(50),
            ..Default::default()
        }
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }

    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_width = width.max(1);
        self.max_height = height.max(1);
        self
    }

    pub fn validated(mut self) -> Self {
        self.quality = self.quality.min(100);
        self.max_width = self.max_width.max(1);
        self.max_height = self.max_height.max(1);
        self.max_selections = self.max_selections.max(1).min(50);
        self.max_file_size = self.max_file_size.min(MAX_IMAGE_SIZE_BYTES);
        if !self.allow_multiple {
            self.max_selections = 1;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionStatus {
    Granted,
    Denied,
    DeniedPermanently,
    Restricted,
    NotDetermined,
}

impl PermissionStatus {
    pub fn is_granted(&self) -> bool {
        matches!(self, PermissionStatus::Granted)
    }

    pub fn can_request(&self) -> bool {
        matches!(self, PermissionStatus::NotDetermined | PermissionStatus::Denied)
    }

    pub fn should_show_settings_prompt(&self) -> bool {
        matches!(self, PermissionStatus::DeniedPermanently | PermissionStatus::Restricted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CameraCapabilities {
    pub has_front_camera: bool,
    pub has_back_camera: bool,
    pub has_flash: bool,
    pub has_torch: bool,
    pub supports_heic: bool,
    pub supports_video: bool,
    pub max_photo_resolution: Option<(u32, u32)>,
    pub is_simulator: bool,
    pub platform: CameraPlatform,
}

impl Default for CameraCapabilities {
    fn default() -> Self {
        Self {
            has_front_camera: true,
            has_back_camera: true,
            has_flash: true,
            has_torch: true,
            supports_heic: false,
            supports_video: true,
            max_photo_resolution: None,
            is_simulator: false,
            platform: CameraPlatform::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraPlatform {
    IOS,
    Android,
    Web,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapturedImage {
    data: Vec<u8>,
    format: ImageFormat,
    width: u32,
    height: u32,
    file_size: usize,
    capture_time_ms: u64,
}

impl CapturedImage {
    pub fn new(
        data: Vec<u8>,
        format: ImageFormat,
        width: u32,
        height: u32,
        capture_time_ms: u64,
    ) -> Result<Self, CameraError> {
        if data.is_empty() {
            return Err(CameraError::InvalidImage {
                reason: "image data is empty".to_string(),
            });
        }

        if data.len() > MAX_IMAGE_SIZE_BYTES {
            return Err(CameraError::ImageTooLarge {
                size: data.len(),
                max: MAX_IMAGE_SIZE_BYTES,
            });
        }

        if let Some(detected) = ImageFormat::from_magic_bytes(&data) {
            if detected != format {
                return Err(CameraError::InvalidImage {
                    reason: format!(
                        "format mismatch: declared {:?} but detected {:?}",
                        format, detected
                    ),
                });
            }
        }

        Ok(Self {
            file_size: data.len(),
            data,
            format,
            width,
            height,
            capture_time_ms,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }

    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn file_size(&self) -> usize {
        self.file_size
    }

    pub fn capture_time_ms(&self) -> u64 {
        self.capture_time_ms
    }

    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0 {
            return 0.0;
        }
        self.width as f64 / self.height as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CameraOutput {
    PermissionStatus(PermissionStatus),
    Capabilities(CameraCapabilities),
    Photo(CapturedImage),
    Photos(Vec<CapturedImage>),
    Cancelled,
}

impl CameraOutput {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, CameraOutput::Cancelled)
    }

    pub fn into_photo(self) -> Option<CapturedImage> {
        match self {
            CameraOutput::Photo(img) => Some(img),
            _ => None,
        }
    }

    pub fn into_photos(self) -> Option<Vec<CapturedImage>> {
        match self {
            CameraOutput::Photos(imgs) => Some(imgs),
            CameraOutput::Photo(img) => Some(vec![img]),
            _ => None,
        }
    }

    pub fn permission_status(&self) -> Option<PermissionStatus> {
        match self {
            CameraOutput::PermissionStatus(status) => Some(*status),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CameraError {
    #[error("camera permission denied")]
    PermissionDenied,

    #[error("camera permission denied permanently - user must enable in settings")]
    PermissionDeniedPermanently,

    #[error("camera unavailable: {reason}")]
    Unavailable { reason: String },

    #[error("camera {facing:?} not available on this device")]
    CameraNotFound { facing: CameraFacing },

    #[error("capture failed: {reason}")]
    CaptureFailed { reason: String },

    #[error("capture cancelled by user")]
    Cancelled,

    #[error("capture timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("image too large: {size} bytes exceeds maximum of {max} bytes")]
    ImageTooLarge { size: usize, max: usize },

    #[error("invalid image: {reason}")]
    InvalidImage { reason: String },

    #[error("format {format:?} not supported on this device")]
    FormatNotSupported { format: ImageFormat },

    #[error("flash not available")]
    FlashNotAvailable,

    #[error("camera not supported on this platform")]
    NotSupported,

    #[error("operation cancelled - another operation in progress")]
    Busy,

    #[error("app in background - camera access not allowed")]
    BackgroundRestricted,

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl CameraError {
    pub fn is_permission_error(&self) -> bool {
        matches!(
            self,
            CameraError::PermissionDenied | CameraError::PermissionDeniedPermanently
        )
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CameraError::Timeout { .. }
                | CameraError::Busy
                | CameraError::BackgroundRestricted
                | CameraError::Unavailable { .. }
        )
    }

    pub fn should_show_settings(&self) -> bool {
        matches!(self, CameraError::PermissionDeniedPermanently)
    }
}

pub type CameraResult = Result<CameraOutput, CameraError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_detection_jpeg() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
        assert_eq!(
            ImageFormat::from_magic_bytes(&jpeg_header),
            Some(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn test_image_format_detection_png() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D];
        assert_eq!(
            ImageFormat::from_magic_bytes(&png_header),
            Some(ImageFormat::Png)
        );
    }

    #[test]
    fn test_image_format_detection_webp() {
        let webp_header = [
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
        ];
        assert_eq!(
            ImageFormat::from_magic_bytes(&webp_header),
            Some(ImageFormat::WebP)
        );
    }

    #[test]
    fn test_image_format_detection_unknown() {
        let random_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B];
        assert_eq!(ImageFormat::from_magic_bytes(&random_data), None);
    }

    #[test]
    fn test_image_format_detection_too_short() {
        let short_data = [0xFF, 0xD8];
        assert_eq!(ImageFormat::from_magic_bytes(&short_data), None);
    }

    #[test]
    fn test_capture_config_defaults() {
        let config = CaptureConfig::default();
        assert_eq!(config.facing, CameraFacing::Back);
        assert_eq!(config.format, ImageFormat::Jpeg);
        assert_eq!(config.quality, DEFAULT_JPEG_QUALITY);
        assert!(config.strip_metadata);
    }

    #[test]
    fn test_capture_config_builder() {
        let config = CaptureConfig::default()
            .with_facing(CameraFacing::Front)
            .with_format(ImageFormat::Png)
            .with_quality(90)
            .with_max_dimensions(1024, 768)
            .with_flash(FlashMode::Off);

        assert_eq!(config.facing, CameraFacing::Front);
        assert_eq!(config.format, ImageFormat::Png);
        assert_eq!(config.quality, 90);
        assert_eq!(config.max_width, 1024);
        assert_eq!(config.max_height, 768);
        assert_eq!(config.flash, FlashMode::Off);
    }

    #[test]
    fn test_capture_config_validation() {
        let config = CaptureConfig::default()
            .with_quality(150)
            .with_max_dimensions(0, 0)
            .with_timeout_ms(500)
            .validated();

        assert_eq!(config.quality, 100);
        assert_eq!(config.max_width, 1);
        assert_eq!(config.max_height, 1);
        assert!(config.timeout_ms >= 1000);
    }

    #[test]
    fn test_gallery_pick_config_single() {
        let config = GalleryPickConfig::single();
        assert!(!config.allow_multiple);
        assert_eq!(config.max_selections, 1);
    }

    #[test]
    fn test_gallery_pick_config_multiple() {
        let config = GalleryPickConfig::multiple(10);
        assert!(config.allow_multiple);
        assert_eq!(config.max_selections, 10);
    }

    #[test]
    fn test_gallery_pick_config_multiple_capped() {
        let config = GalleryPickConfig::multiple(100).validated();
        assert_eq!(config.max_selections, 50);
    }

    #[test]
    fn test_permission_status() {
        assert!(PermissionStatus::Granted.is_granted());
        assert!(!PermissionStatus::Denied.is_granted());

        assert!(PermissionStatus::NotDetermined.can_request());
        assert!(!PermissionStatus::DeniedPermanently.can_request());

        assert!(PermissionStatus::DeniedPermanently.should_show_settings_prompt());
        assert!(!PermissionStatus::Granted.should_show_settings_prompt());
    }

    #[test]
    fn test_captured_image_creation_valid() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
        let result = CapturedImage::new(jpeg_data.clone(), ImageFormat::Jpeg, 100, 100, 12345);
        assert!(result.is_ok());

        let image = result.unwrap();
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 100);
        assert_eq!(image.format(), ImageFormat::Jpeg);
        assert_eq!(image.file_size(), jpeg_data.len());
    }

    #[test]
    fn test_captured_image_empty_data() {
        let result = CapturedImage::new(vec![], ImageFormat::Jpeg, 100, 100, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(CameraError::InvalidImage { .. })));
    }

    #[test]
    fn test_captured_image_format_mismatch() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
        let result = CapturedImage::new(jpeg_data, ImageFormat::Png, 100, 100, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(CameraError::InvalidImage { .. })));
    }

    #[test]
    fn test_captured_image_too_large() {
        let large_data = vec![0xFF; MAX_IMAGE_SIZE_BYTES + 1];
        let result = CapturedImage::new(large_data, ImageFormat::Jpeg, 100, 100, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(CameraError::ImageTooLarge { .. })));
    }

    #[test]
    fn test_camera_output_helpers() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
        let image = CapturedImage::new(jpeg_data, ImageFormat::Jpeg, 100, 100, 0).unwrap();

        let output = CameraOutput::Photo(image);
        assert!(!output.is_cancelled());
        assert!(output.clone().into_photo().is_some());

        let cancelled = CameraOutput::Cancelled;
        assert!(cancelled.is_cancelled());
        assert!(cancelled.into_photo().is_none());
    }

    #[test]
    fn test_camera_error_helpers() {
        assert!(CameraError::PermissionDenied.is_permission_error());
        assert!(CameraError::PermissionDeniedPermanently.is_permission_error());
        assert!(!CameraError::Cancelled.is_permission_error());

        assert!(CameraError::Timeout { timeout_ms: 1000 }.is_retryable());
        assert!(CameraError::Busy.is_retryable());
        assert!(!CameraError::PermissionDenied.is_retryable());

        assert!(CameraError::PermissionDeniedPermanently.should_show_settings());
        assert!(!CameraError::PermissionDenied.should_show_settings());
    }

    #[test]
    fn test_aspect_ratio_calculation() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
        let image = CapturedImage::new(jpeg_data, ImageFormat::Jpeg, 1920, 1080, 0).unwrap();
        let ratio = image.aspect_ratio();
        assert!((ratio - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_image_format_mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Heic.mime_type(), "image/heic");
        assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
    }

    #[test]
    fn test_image_format_extensions() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Heic.extension(), "heic");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
    }

    #[test]
    fn test_format_supports_quality() {
        assert!(ImageFormat::Jpeg.supports_quality());
        assert!(ImageFormat::WebP.supports_quality());
        assert!(ImageFormat::Heic.supports_quality());
        assert!(!ImageFormat::Png.supports_quality());
    }
}