use std::io::Cursor;
use std::sync::Arc;
use std::time::Instant;

use image::codecs::webp::WebPEncoder;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ExtendedColorType, GenericImageView, ImageEncoder, Limits};
use metrics::{counter, histogram};
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{instrument, warn};

use crate::vision::Detection;

#[derive(Debug, Error)]
pub enum ImageProcessingError {
    #[error("failed to decode image: {source}")]
    Decode {
        #[from]
        source: image::ImageError,
    },

    #[error("webp encoding failed: width={width}, height={height}, reason={reason}")]
    WebpEncode {
        width: u32,
        height: u32,
        reason: String,
    },

    #[error("invalid bbox: x1={x1}, y1={y1}, x2={x2}, y2={y2}, reason={reason}")]
    InvalidBbox {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        reason: &'static str,
    },

    #[error("invalid expand factor: {value}, must be in [0.0, {max}]")]
    InvalidExpand { value: f32, max: f32 },

    #[error("crop region is zero-sized after clamping")]
    ZeroCrop,

    #[error("image too large: {width}x{height} = {pixels} pixels, max {max_pixels}")]
    ImageTooLarge {
        width: u32,
        height: u32,
        pixels: u64,
        max_pixels: u64,
    },

    #[error("input too large: {size} bytes, max {max_size}")]
    InputTooLarge { size: usize, max_size: usize },

    #[error("input bytes empty")]
    EmptyInput,

    #[error("unsupported image format")]
    UnsupportedFormat,

    #[error("processing timeout exceeded")]
    Timeout,

    #[error("service overloaded, try again later")]
    Overloaded,
}

#[derive(Clone, Debug)]
pub struct ProcessingConfig {
    pub max_image_pixels: u64,
    pub max_input_bytes: usize,
    pub max_alloc_bytes: u64,
    pub max_dimension: u32,
    pub output_size: u32,
    pub webp_quality: u8,
    pub max_expand: f32,
    pub max_concurrent_ops: usize,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            max_image_pixels: 100_000_000,
            max_input_bytes: 50 * 1024 * 1024,
            max_alloc_bytes: 512 * 1024 * 1024,
            max_dimension: 15_000,
            output_size: 512,
            webp_quality: 85,
            max_expand: 2.0,
            max_concurrent_ops: 4,
        }
    }
}

pub struct ImageProcessor {
    config: ProcessingConfig,
    semaphore: Arc<Semaphore>,
}

impl ImageProcessor {
    pub fn new(config: ProcessingConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_ops));
        Self { config, semaphore }
    }

    pub fn with_defaults() -> Self {
        Self::new(ProcessingConfig::default())
    }

    #[instrument(skip(self, raw_bytes), fields(input_size = raw_bytes.len()))]
    pub async fn crop_and_strip(
        &self,
        raw_bytes: Vec<u8>,
        bbox: NormalizedBbox,
        expand: f32,
    ) -> Result<Vec<u8>, ImageProcessingError> {
        let start = Instant::now();
        counter!("image.crop_and_strip.requests").increment(1);

        let _permit = self.semaphore.try_acquire().map_err(|_| {
            counter!("image.crop_and_strip.rejected").increment(1);
            ImageProcessingError::Overloaded
        })?;

        let config = self.config.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::crop_and_strip_sync(&config, &raw_bytes, &bbox, expand)
        })
        .await
        .map_err(|_| ImageProcessingError::Timeout)?;

        histogram!("image.crop_and_strip.duration_ms").record(start.elapsed().as_millis() as f64);

        match &result {
            Ok(data) => {
                histogram!("image.crop_and_strip.output_size").record(data.len() as f64);
            }
            Err(e) => {
                counter!("image.crop_and_strip.errors").increment(1);
                warn!(error = %e, "crop_and_strip failed");
            }
        }

        result
    }

    #[instrument(skip(self, raw_bytes), fields(input_size = raw_bytes.len()))]
    pub async fn resize_and_strip(
        &self,
        raw_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, ImageProcessingError> {
        let start = Instant::now();
        counter!("image.resize_and_strip.requests").increment(1);

        let _permit = self.semaphore.try_acquire().map_err(|_| {
            counter!("image.resize_and_strip.rejected").increment(1);
            ImageProcessingError::Overloaded
        })?;

        let config = self.config.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::resize_and_strip_sync(&config, &raw_bytes)
        })
        .await
        .map_err(|_| ImageProcessingError::Timeout)?;

        histogram!("image.resize_and_strip.duration_ms").record(start.elapsed().as_millis() as f64);

        match &result {
            Ok(data) => {
                histogram!("image.resize_and_strip.output_size").record(data.len() as f64);
            }
            Err(e) => {
                counter!("image.resize_and_strip.errors").increment(1);
                warn!(error = %e, "resize_and_strip failed");
            }
        }

        result
    }

    fn crop_and_strip_sync(
        config: &ProcessingConfig,
        raw_bytes: &[u8],
        bbox: &NormalizedBbox,
        expand: f32,
    ) -> Result<Vec<u8>, ImageProcessingError> {
        validate_expand(expand, config.max_expand)?;
        let img = decode_image(config, raw_bytes)?;
        let (w, h) = img.dimensions();

        let dx = (bbox.width() * expand / 2.0) as f64;
        let dy = (bbox.height() * expand / 2.0) as f64;

        let px_x1 = safe_coord(bbox.x1() as f64 - dx, w);
        let px_y1 = safe_coord(bbox.y1() as f64 - dy, h);
        let px_x2 = safe_coord(bbox.x2() as f64 + dx, w);
        let px_y2 = safe_coord(bbox.y2() as f64 + dy, h);

        let crop_width = px_x2.saturating_sub(px_x1);
        let crop_height = px_y2.saturating_sub(px_y1);

        if crop_width == 0 || crop_height == 0 {
            return Err(ImageProcessingError::ZeroCrop);
        }

        let safe_width = crop_width.min(w.saturating_sub(px_x1));
        let safe_height = crop_height.min(h.saturating_sub(px_y1));

        let cropped = img.crop_imm(px_x1, px_y1, safe_width, safe_height);

        let resized = cropped.resize_exact(
            config.output_size,
            config.output_size,
            image::imageops::FilterType::Triangle,
        );

        encode_webp(&resized, config.webp_quality)
    }

    fn resize_and_strip_sync(
        config: &ProcessingConfig,
        raw_bytes: &[u8],
    ) -> Result<Vec<u8>, ImageProcessingError> {
        let img = decode_image(config, raw_bytes)?;

        let resized = img.resize_exact(
            config.output_size,
            config.output_size,
            image::imageops::FilterType::Triangle,
        );

        encode_webp(&resized, config.webp_quality)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedBbox {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl NormalizedBbox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Result<Self, ImageProcessingError> {
        if [x1, y1, x2, y2].iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(ImageProcessingError::InvalidBbox {
                x1,
                y1,
                x2,
                y2,
                reason: "contains NaN or infinite value",
            });
        }
        if x1 < 0.0 || y1 < 0.0 || x2 > 1.0 || y2 > 1.0 {
            return Err(ImageProcessingError::InvalidBbox {
                x1,
                y1,
                x2,
                y2,
                reason: "coordinates outside [0, 1] range",
            });
        }
        if x2 <= x1 || y2 <= y1 {
            return Err(ImageProcessingError::InvalidBbox {
                x1,
                y1,
                x2,
                y2,
                reason: "inverted or zero-area box",
            });
        }
        Ok(Self { x1, y1, x2, y2 })
    }

    pub fn full() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        }
    }

    pub fn x1(&self) -> f32 {
        self.x1
    }

    pub fn y1(&self) -> f32 {
        self.y1
    }

    pub fn x2(&self) -> f32 {
        self.x2
    }

    pub fn y2(&self) -> f32 {
        self.y2
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }
}

impl TryFrom<&Detection> for NormalizedBbox {
    type Error = ImageProcessingError;

    fn try_from(det: &Detection) -> Result<Self, Self::Error> {
        NormalizedBbox::new(det.bbox[0], det.bbox[1], det.bbox[2], det.bbox[3])
    }
}

pub fn merge_bboxes(detections: &[Detection]) -> Result<NormalizedBbox, ImageProcessingError> {
    if detections.is_empty() {
        return Ok(NormalizedBbox::full());
    }

    if detections.len() == 1 {
        return NormalizedBbox::try_from(&detections[0]);
    }

    let mut x1 = f64::MAX;
    let mut y1 = f64::MAX;
    let mut x2 = f64::MIN;
    let mut y2 = f64::MIN;

    for det in detections {
        let bbox = NormalizedBbox::try_from(det)?;
        x1 = x1.min(bbox.x1() as f64);
        y1 = y1.min(bbox.y1() as f64);
        x2 = x2.max(bbox.x2() as f64);
        y2 = y2.max(bbox.y2() as f64);
    }

    NormalizedBbox::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32)
}

fn validate_expand(expand: f32, max: f32) -> Result<(), ImageProcessingError> {
    if expand.is_nan() || expand.is_infinite() || expand < 0.0 || expand > max {
        return Err(ImageProcessingError::InvalidExpand { value: expand, max });
    }
    Ok(())
}

fn safe_coord(normalized: f64, dimension: u32) -> u32 {
    let pixel = (normalized * dimension as f64).round() as i64;
    pixel.clamp(0, (dimension.saturating_sub(1)) as i64) as u32
}

fn decode_image(
    config: &ProcessingConfig,
    raw_bytes: &[u8],
) -> Result<DynamicImage, ImageProcessingError> {
    if raw_bytes.is_empty() {
        return Err(ImageProcessingError::EmptyInput);
    }

    if raw_bytes.len() > config.max_input_bytes {
        return Err(ImageProcessingError::InputTooLarge {
            size: raw_bytes.len(),
            max_size: config.max_input_bytes,
        });
    }

    let cursor = Cursor::new(raw_bytes);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| ImageProcessingError::Decode { source: e.into() })?;

    if reader.format().is_none() {
        return Err(ImageProcessingError::UnsupportedFormat);
    }

    let mut limits = Limits::default();
    limits.max_image_width = Some(config.max_dimension);
    limits.max_image_height = Some(config.max_dimension);
    limits.max_alloc = Some(config.max_alloc_bytes);

    let cursor = Cursor::new(raw_bytes);
    let mut reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| ImageProcessingError::Decode { source: e.into() })?;

    reader.limits(limits);

    let img = reader.decode()?;
    let (w, h) = img.dimensions();
    let pixels = w as u64 * h as u64;

    if pixels > config.max_image_pixels {
        return Err(ImageProcessingError::ImageTooLarge {
            width: w,
            height: h,
            pixels,
            max_pixels: config.max_image_pixels,
        });
    }

    Ok(img)
}

fn encode_webp(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageProcessingError> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return Err(ImageProcessingError::WebpEncode {
            width,
            height,
            reason: "zero dimension".into(),
        });
    }

    let mut buffer = Vec::with_capacity((width * height * 4) as usize / 10);
    let encoder = WebPEncoder::new_lossless(&mut buffer);

    encoder
        .write_image(
            rgba.as_raw(),
            width,
            height,
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| ImageProcessingError::WebpEncode {
            width,
            height,
            reason: e.to_string(),
        })?;

    if buffer.len() < 12 || &buffer[0..4] != b"RIFF" || &buffer[8..12] != b"WEBP" {
        return Err(ImageProcessingError::WebpEncode {
            width,
            height,
            reason: "invalid webp magic bytes".into(),
        });
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn make_detection(bbox: [f32; 4]) -> Detection {
        Detection {
            bbox,
            confidence: 0.9,
            class_id: 15,
        }
    }

    fn create_test_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(width, height, |x, y| {
                Rgba([
                    (x % 256) as u8,
                    (y % 256) as u8,
                    ((x + y) % 256) as u8,
                    255,
                ])
            });
        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder
            .write_image(
                img.as_raw(),
                width,
                height,
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        buffer
    }

    #[test]
    fn merge_bboxes_empty_returns_full() {
        let result = merge_bboxes(&[]).unwrap();
        assert_eq!(result, NormalizedBbox::full());
    }

    #[test]
    fn merge_bboxes_single_passthrough() {
        let det = make_detection([0.1, 0.2, 0.5, 0.6]);
        let result = merge_bboxes(&[det]).unwrap();
        assert_eq!(result, NormalizedBbox::new(0.1, 0.2, 0.5, 0.6).unwrap());
    }

    #[test]
    fn merge_bboxes_multiple_encompasses_all() {
        let d1 = make_detection([0.1, 0.2, 0.3, 0.4]);
        let d2 = make_detection([0.5, 0.6, 0.7, 0.8]);
        let result = merge_bboxes(&[d1, d2]).unwrap();
        assert_eq!(result, NormalizedBbox::new(0.1, 0.2, 0.7, 0.8).unwrap());
    }

    #[test]
    fn merge_bboxes_rejects_nan() {
        let det = make_detection([f32::NAN, 0.1, 0.5, 0.5]);
        assert!(merge_bboxes(&[det]).is_err());
    }

    #[test]
    fn normalized_bbox_rejects_inverted() {
        assert!(NormalizedBbox::new(0.5, 0.1, 0.2, 0.9).is_err());
        assert!(NormalizedBbox::new(0.1, 0.9, 0.5, 0.2).is_err());
    }

    #[test]
    fn normalized_bbox_rejects_out_of_range() {
        assert!(NormalizedBbox::new(-0.1, 0.0, 0.5, 0.5).is_err());
        assert!(NormalizedBbox::new(0.0, 0.0, 1.1, 0.5).is_err());
    }

    #[test]
    fn normalized_bbox_fields_are_private() {
        let bbox = NormalizedBbox::new(0.1, 0.2, 0.3, 0.4).unwrap();
        assert!((bbox.x1() - 0.1).abs() < f32::EPSILON);
        assert!((bbox.y1() - 0.2).abs() < f32::EPSILON);
        assert!((bbox.x2() - 0.3).abs() < f32::EPSILON);
        assert!((bbox.y2() - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn validate_expand_rejects_negative() {
        assert!(validate_expand(-0.1, 2.0).is_err());
    }

    #[test]
    fn validate_expand_rejects_extreme() {
        assert!(validate_expand(3.0, 2.0).is_err());
        assert!(validate_expand(f32::NAN, 2.0).is_err());
        assert!(validate_expand(f32::INFINITY, 2.0).is_err());
    }

    #[test]
    fn validate_expand_accepts_valid() {
        assert!(validate_expand(0.0, 2.0).is_ok());
        assert!(validate_expand(0.2, 2.0).is_ok());
        assert!(validate_expand(2.0, 2.0).is_ok());
    }

    #[test]
    fn decode_rejects_empty() {
        let config = ProcessingConfig::default();
        assert!(decode_image(&config, &[]).is_err());
    }

    #[test]
    fn decode_rejects_garbage() {
        let config = ProcessingConfig::default();
        assert!(decode_image(&config, &[0xFF, 0xFE, 0x00]).is_err());
    }

    #[test]
    fn decode_rejects_oversized_input() {
        let config = ProcessingConfig {
            max_input_bytes: 100,
            ..Default::default()
        };
        let data = vec![0u8; 101];
        let result = decode_image(&config, &data);
        assert!(matches!(result, Err(ImageProcessingError::InputTooLarge { .. })));
    }

    #[test]
    fn safe_coord_clamps_correctly() {
        assert_eq!(safe_coord(-0.5, 100), 0);
        assert_eq!(safe_coord(0.0, 100), 0);
        assert_eq!(safe_coord(0.5, 100), 50);
        assert_eq!(safe_coord(1.0, 100), 99);
        assert_eq!(safe_coord(1.5, 100), 99);
    }

    #[test]
    fn crop_and_strip_sync_produces_valid_webp() {
        let config = ProcessingConfig::default();
        let png = create_test_png(100, 100);
        let bbox = NormalizedBbox::new(0.25, 0.25, 0.75, 0.75).unwrap();

        let result = ImageProcessor::crop_and_strip_sync(&config, &png, &bbox, 0.1).unwrap();

        assert!(result.len() >= 12);
        assert_eq!(&result[0..4], b"RIFF");
        assert_eq!(&result[8..12], b"WEBP");

        let decoded = image::load_from_memory(&result).unwrap();
        assert_eq!(decoded.dimensions(), (512, 512));
    }

    #[test]
    fn resize_and_strip_sync_produces_valid_webp() {
        let config = ProcessingConfig::default();
        let png = create_test_png(200, 150);

        let result = ImageProcessor::resize_and_strip_sync(&config, &png).unwrap();

        assert!(result.len() >= 12);
        assert_eq!(&result[0..4], b"RIFF");
        assert_eq!(&result[8..12], b"WEBP");

        let decoded = image::load_from_memory(&result).unwrap();
        assert_eq!(decoded.dimensions(), (512, 512));
    }

    #[test]
    fn crop_rejects_zero_crop() {
        let config = ProcessingConfig::default();
        let png = create_test_png(100, 100);
        let bbox = NormalizedBbox::new(0.999, 0.999, 1.0, 1.0).unwrap();

        let result = ImageProcessor::crop_and_strip_sync(&config, &png, &bbox, 0.0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn processor_rejects_when_overloaded() {
        let config = ProcessingConfig {
            max_concurrent_ops: 1,
            ..Default::default()
        };
        let processor = ImageProcessor::new(config);
        let png = create_test_png(50, 50);

        let _permit = processor.semaphore.try_acquire().unwrap();

        let result = processor
            .resize_and_strip(png)
            .await;

        assert!(matches!(result, Err(ImageProcessingError::Overloaded)));
    }

    proptest! {
        #[test]
        fn bbox_invariants_maintained(
            x1 in 0.0f32..0.5,
            y1 in 0.0f32..0.5,
            x2 in 0.5f32..1.0,
            y2 in 0.5f32..1.0,
        ) {
            let bbox = NormalizedBbox::new(x1, y1, x2, y2).unwrap();
            prop_assert!(bbox.x1() >= 0.0);
            prop_assert!(bbox.y1() >= 0.0);
            prop_assert!(bbox.x2() <= 1.0);
            prop_assert!(bbox.y2() <= 1.0);
            prop_assert!(bbox.x2() > bbox.x1());
            prop_assert!(bbox.y2() > bbox.y1());
            prop_assert!(bbox.width() > 0.0);
            prop_assert!(bbox.height() > 0.0);
        }

        #[test]
        fn safe_coord_never_exceeds_bounds(
            normalized in -1.0f64..2.0,
            dimension in 1u32..10000,
        ) {
            let result = safe_coord(normalized, dimension);
            prop_assert!(result < dimension);
        }

        #[test]
        fn merge_preserves_bounds(
            x1a in 0.0f32..0.3,
            y1a in 0.0f32..0.3,
            x2a in 0.3f32..0.5,
            y2a in 0.3f32..0.5,
            x1b in 0.5f32..0.7,
            y1b in 0.5f32..0.7,
            x2b in 0.7f32..1.0,
            y2b in 0.7f32..1.0,
        ) {
            let d1 = make_detection([x1a, y1a, x2a, y2a]);
            let d2 = make_detection([x1b, y1b, x2b, y2b]);
            let merged = merge_bboxes(&[d1, d2]).unwrap();

            prop_assert!(merged.x1() >= 0.0);
            prop_assert!(merged.y1() >= 0.0);
            prop_assert!(merged.x2() <= 1.0);
            prop_assert!(merged.y2() <= 1.0);
            prop_assert!(merged.x1() <= x1a);
            prop_assert!(merged.y1() <= y1a);
            prop_assert!(merged.x2() >= x2b);
            prop_assert!(merged.y2() >= y2b);
        }

        #[test]
        fn expand_validation_consistent(expand in -5.0f32..5.0) {
            let max = 2.0f32;
            let result = validate_expand(expand, max);

            if expand.is_nan() || expand.is_infinite() || expand < 0.0 || expand > max {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }
    }
}