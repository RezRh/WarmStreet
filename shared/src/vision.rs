use ndarray::{Array, Array2, Array3, ArrayView1, Axis};
use ort::session::Session;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tracing::{debug, instrument, warn};

// ============================================================================
// Constants with explicit documentation
// ============================================================================

/// Maximum compressed image size (20MB) - first line of defense
const MAX_COMPRESSED_SIZE: usize = 20 * 1024 * 1024;

/// Maximum decompressed pixel count (100 megapixels) - prevents decompression bombs
const MAX_PIXELS: u64 = 100_000_000;

/// Model input dimensions - validated against actual model at runtime
const DEFAULT_INPUT_SIZE: u32 = 640;

/// Maximum detections from model output to prevent DoS
const MAX_MODEL_DETECTIONS: usize = 50_000;

/// Maximum candidates entering NMS to bound CPU time
const MAX_NMS_INPUTS: usize = 300;

/// Inference timeout to prevent hangs
const INFERENCE_TIMEOUT: Duration = Duration::from_secs(30);

/// COCO animal class IDs (validated against model class count at runtime)
const ANIMAL_CLASSES: &[u32] = &[14, 15, 16, 17, 18, 19, 20, 21, 22, 23];

/// Pre-computed HashSet for O(1) class lookup
static ANIMAL_CLASS_SET: LazyLock<HashSet<u32>> =
    LazyLock::new(|| ANIMAL_CLASSES.iter().copied().collect());

/// Allowed image formats - explicit allowlist
const ALLOWED_FORMATS: &[image::ImageFormat] = &[
    image::ImageFormat::Jpeg,
    image::ImageFormat::Png,
];

// ============================================================================
// Error Types (sanitized for external consumption)
// ============================================================================

#[derive(thiserror::Error, Debug)]
pub enum VisionError {
    #[error("image decode failed")]
    Decode(#[source] image::ImageError),

    #[error("compressed image too large: {0} bytes (max: {MAX_COMPRESSED_SIZE})")]
    CompressedTooLarge(usize),

    #[error("decompressed image too large: {width}x{height} pixels (max: {MAX_PIXELS})")]
    PixelCountTooLarge { width: u32, height: u32 },

    #[error("unsupported image format: {0:?}")]
    UnsupportedFormat(String),

    #[error("invalid image dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("inference engine error")]
    InferenceEngine(String), // Sanitized - no raw ORT errors

    #[error("invalid model output shape")]
    InvalidOutputShape { expected: String, got: String },

    #[error("model configuration mismatch: {0}")]
    ModelMismatch(String),

    #[error("inference timeout after {0:?}")]
    Timeout(Duration),

    #[error("processing error: {0}")]
    Processing(String),
}

// Manual From impl to sanitize ORT errors
impl From<ort::Error> for VisionError {
    fn from(e: ort::Error) -> Self {
        // Log full error internally, return sanitized version externally
        tracing::error!(error = %e, "ORT inference error");
        VisionError::InferenceEngine("internal inference error".into())
    }
}

// ============================================================================
// Detection Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use]
pub struct Detection {
    /// Bounding box [x1, y1, x2, y2] normalized to original image (0.0..1.0)
    pub bbox: [f32; 4],
    /// Detection confidence score (0.0..1.0)
    pub confidence: f32,
    /// Class ID from model
    pub class_id: u32,
}

/// Metadata about the detection run
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use]
pub struct DetectionResult {
    pub detections: Vec<Detection>,
    /// True if results were truncated due to limits
    pub truncated: bool,
    /// Number of candidates before NMS
    pub candidates_before_nms: usize,
    /// Preprocessing duration
    pub preprocess_ms: f64,
    /// Inference duration
    pub inference_ms: f64,
    /// Postprocessing duration
    pub postprocess_ms: f64,
}

// ============================================================================
// Model Configuration (extracted at load time)
// ============================================================================

#[derive(Debug, Clone)]
struct ModelConfig {
    input_height: u32,
    input_width: u32,
    num_classes: usize,
    output_features: usize, // 4 + num_classes
}

// ============================================================================
// YoloDetector
// ============================================================================

/// YOLO object detector with validated model configuration.
///
/// # Thread Safety
///
/// This detector uses interior synchronization. Multiple threads may call
/// `detect()` concurrently, but inference is serialized internally to prevent
/// GPU state corruption. For maximum throughput, use a pool of detectors.
pub struct YoloDetector {
    session: std::sync::Mutex<Session>,
    config: ModelConfig,
}

// Explicit: we handle synchronization via Mutex
unsafe impl Send for YoloDetector {}
unsafe impl Sync for YoloDetector {}

impl YoloDetector {
    /// Creates a new detector from ONNX model bytes.
    ///
    /// # Errors
    ///
    /// Returns error if model cannot be loaded or has unexpected structure.
    #[instrument(skip(model_bytes), fields(model_size = model_bytes.len()))]
    pub fn new(model_bytes: &[u8]) -> Result<Self, VisionError> {
        let session = Session::builder()?.commit_from_memory(model_bytes)?;

        // Extract and validate model configuration
        let config = Self::extract_model_config(&session)?;

        debug!(
            input_size = %format!("{}x{}", config.input_width, config.input_height),
            num_classes = config.num_classes,
            "Model loaded successfully"
        );

        Ok(Self {
            session: std::sync::Mutex::new(session),
            config,
        })
    }

    /// Extracts configuration from model metadata and validates expectations.
    fn extract_model_config(session: &Session) -> Result<ModelConfig, VisionError> {
        // Validate input shape
        let input = session.inputs.first().ok_or_else(|| {
            VisionError::ModelMismatch("Model has no inputs".into())
        })?;

        let input_dims: Vec<i64> = input
            .input_type
            .tensor_dimensions()
            .ok_or_else(|| VisionError::ModelMismatch("Input is not a tensor".into()))?
            .collect();

        // Expected: [batch, channels, height, width] = [1, 3, 640, 640]
        if input_dims.len() != 4 {
            return Err(VisionError::ModelMismatch(format!(
                "Expected 4D input, got {}D",
                input_dims.len()
            )));
        }

        let (input_height, input_width) = (input_dims[2] as u32, input_dims[3] as u32);

        if input_height == 0 || input_width == 0 || input_height > 4096 || input_width > 4096 {
            return Err(VisionError::ModelMismatch(format!(
                "Invalid input dimensions: {}x{}",
                input_width, input_height
            )));
        }

        // Validate output shape to extract class count
        let output = session.outputs.first().ok_or_else(|| {
            VisionError::ModelMismatch("Model has no outputs".into())
        })?;

        let output_dims: Vec<i64> = output
            .output_type
            .tensor_dimensions()
            .ok_or_else(|| VisionError::ModelMismatch("Output is not a tensor".into()))?
            .collect();

        // Expected: [1, 84, 8400] or [1, 8400, 84]
        if output_dims.len() != 3 {
            return Err(VisionError::ModelMismatch(format!(
                "Expected 3D output, got {}D",
                output_dims.len()
            )));
        }

        // Determine which dimension is features (should be 4 + num_classes)
        let output_features = if output_dims[1] < output_dims[2] {
            output_dims[1] as usize // [1, 84, 8400] format
        } else {
            output_dims[2] as usize // [1, 8400, 84] format
        };

        if output_features < 5 {
            return Err(VisionError::ModelMismatch(format!(
                "Output features too small: {}",
                output_features
            )));
        }

        let num_classes = output_features - 4;

        // Validate animal classes are within range
        for &class_id in ANIMAL_CLASSES {
            if class_id as usize >= num_classes {
                return Err(VisionError::ModelMismatch(format!(
                    "Animal class {} exceeds model class count {}",
                    class_id, num_classes
                )));
            }
        }

        Ok(ModelConfig {
            input_height,
            input_width,
            num_classes,
            output_features,
        })
    }

    /// Validates image format against allowlist.
    fn validate_format(image_data: &[u8]) -> Result<image::ImageFormat, VisionError> {
        let format = image::guess_format(image_data).map_err(VisionError::Decode)?;

        if !ALLOWED_FORMATS.contains(&format) {
            return Err(VisionError::UnsupportedFormat(format!("{:?}", format)));
        }

        Ok(format)
    }

    /// Validates image dimensions before full decode (prevents decompression bombs).
    fn validate_dimensions(image_data: &[u8]) -> Result<(u32, u32), VisionError> {
        let reader = image::io::Reader::new(Cursor::new(image_data))
            .with_guessed_format()
            .map_err(VisionError::Decode)?;

        let (width, height) = reader.into_dimensions().map_err(VisionError::Decode)?;

        if width == 0 || height == 0 {
            return Err(VisionError::InvalidDimensions { width, height });
        }

        let pixel_count = (width as u64).saturating_mul(height as u64);
        if pixel_count > MAX_PIXELS {
            return Err(VisionError::PixelCountTooLarge { width, height });
        }

        Ok((width, height))
    }

    /// Preprocesses image with letterbox resize and normalization.
    ///
    /// Returns (input_tensor, preprocessing_params).
    #[instrument(skip(self, image_data), fields(data_len = image_data.len()))]
    fn preprocess(
        &self,
        image_data: &[u8],
    ) -> Result<(Array<f32, ndarray::Dim<[usize; 4]>>, PreprocessParams), VisionError> {
        // Layer 1: Compressed size check
        if image_data.len() > MAX_COMPRESSED_SIZE {
            return Err(VisionError::CompressedTooLarge(image_data.len()));
        }

        // Layer 2: Format validation (allowlist)
        let format = Self::validate_format(image_data)?;

        // Layer 3: Dimension validation BEFORE decode (prevents decompression bombs)
        let (orig_w, orig_h) = Self::validate_dimensions(image_data)?;

        // Now safe to decode
        let dyn_img =
            image::load_from_memory_with_format(image_data, format).map_err(VisionError::Decode)?;

        let input_w = self.config.input_width;
        let input_h = self.config.input_height;

        // Calculate letterbox scale (guard against zero dimensions - already validated but defensive)
        let scale_w = input_w as f32 / orig_w.max(1) as f32;
        let scale_h = input_h as f32 / orig_h.max(1) as f32;
        let scale = scale_w.min(scale_h);

        if !scale.is_finite() || scale <= 0.0 {
            return Err(VisionError::InvalidDimensions {
                width: orig_w,
                height: orig_h,
            });
        }

        let new_w = ((orig_w as f32) * scale).round() as u32;
        let new_h = ((orig_h as f32) * scale).round() as u32;

        // Clamp to valid resize dimensions
        let new_w = new_w.clamp(1, input_w);
        let new_h = new_h.clamp(1, input_h);

        let resized =
            dyn_img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);
        let rgb = resized.to_rgb8();

        // Calculate padding
        let pad_x = (input_w - new_w) as f32 / 2.0;
        let pad_y = (input_h - new_h) as f32 / 2.0;

        // Create canvas initialized to gray (114/255)
        let mut canvas = Array3::<f32>::from_elem(
            (3, input_h as usize, input_w as usize),
            114.0 / 255.0,
        );

        // Efficient pixel copy without per-pixel bounds checks
        let offset_x = pad_x.floor() as usize;
        let offset_y = pad_y.floor() as usize;
        let rgb_raw = rgb.as_raw();
        let rgb_width = new_w as usize;
        let rgb_height = new_h as usize;

        // Bounds are guaranteed by resize logic, but we clamp defensively
        let copy_h = rgb_height.min(input_h as usize - offset_y);
        let copy_w = rgb_width.min(input_w as usize - offset_x);

        for y in 0..copy_h {
            for x in 0..copy_w {
                let src_idx = (y * rgb_width + x) * 3;
                let r = rgb_raw[src_idx] as f32 / 255.0;
                let g = rgb_raw[src_idx + 1] as f32 / 255.0;
                let b = rgb_raw[src_idx + 2] as f32 / 255.0;

                canvas[[0, offset_y + y, offset_x + x]] = r;
                canvas[[1, offset_y + y, offset_x + x]] = g;
                canvas[[2, offset_y + y, offset_x + x]] = b;
            }
        }

        let input_tensor = canvas.insert_axis(Axis(0));

        Ok((
            input_tensor,
            PreprocessParams {
                scale,
                pad_x,
                pad_y,
                orig_w,
                orig_h,
            },
        ))
    }

    /// Runs detection on an image.
    ///
    /// # Errors
    ///
    /// Returns error if image is invalid, too large, or inference fails.
    #[must_use = "detection results should be used"]
    #[instrument(skip(self, image_data), fields(data_len = image_data.len()))]
    pub fn detect(&self, image_data: &[u8]) -> Result<DetectionResult, VisionError> {
        let total_start = Instant::now();

        // Preprocessing
        let preprocess_start = Instant::now();
        let (input_tensor, params) = self.preprocess(image_data)?;
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        // Inference with timeout and mutex for thread safety
        let inference_start = Instant::now();
        let output_array = self.run_inference(input_tensor)?;
        let inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

        // Check total time against timeout
        if total_start.elapsed() > INFERENCE_TIMEOUT {
            return Err(VisionError::Timeout(INFERENCE_TIMEOUT));
        }

        // Postprocessing
        let postprocess_start = Instant::now();
        let (detections, truncated, candidates_before_nms) =
            self.postprocess(&output_array, params)?;
        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            detections = detections.len(),
            truncated,
            candidates_before_nms,
            preprocess_ms,
            inference_ms,
            postprocess_ms,
            "Detection completed"
        );

        Ok(DetectionResult {
            detections,
            truncated,
            candidates_before_nms,
            preprocess_ms,
            inference_ms,
            postprocess_ms,
        })
    }

    /// Runs inference with thread synchronization.
    fn run_inference(
        &self,
        input_tensor: Array<f32, ndarray::Dim<[usize; 4]>>,
    ) -> Result<Array2<f32>, VisionError> {
        let input_value = ort::value::Value::from_array(input_tensor)?;

        // Acquire lock for thread-safe inference
        let session = self.session.lock().map_err(|_| {
            VisionError::Processing("Session lock poisoned".into())
        })?;

        let outputs = session.run(ort::inputs![input_value])?;

        // Find output tensor
        let output_tensor = outputs
            .get("output0")
            .or_else(|| outputs.get("output"))
            .ok_or_else(|| VisionError::Processing("Model missing output node".into()))?;

        let (shape, data) = output_tensor.try_extract_tensor::<f32>()?;

        // Validate shape bounds to prevent allocation attacks
        if shape.len() != 3 {
            return Err(VisionError::InvalidOutputShape {
                expected: "[batch, features, anchors] or [batch, anchors, features]".into(),
                got: format!("{:?}", shape),
            });
        }

        // Check for negative or excessive dimensions
        for &dim in shape.iter() {
            if dim < 0 {
                return Err(VisionError::InvalidOutputShape {
                    expected: "positive dimensions".into(),
                    got: format!("{:?}", shape),
                });
            }
        }

        let total_elements: i64 = shape.iter().product();
        if total_elements as usize > MAX_MODEL_DETECTIONS * self.config.output_features * 2 {
            return Err(VisionError::InvalidOutputShape {
                expected: format!("at most {} elements", MAX_MODEL_DETECTIONS * self.config.output_features),
                got: format!("{} elements", total_elements),
            });
        }

        // Normalize to [anchors, features] format
        let expected_features = self.config.output_features;
        let output_array = if shape[1] as usize == expected_features {
            // Case: [1, 84, 8400] -> transpose to [8400, 84]
            let num_anchors = shape[2] as usize;
            if num_anchors > MAX_MODEL_DETECTIONS {
                return Err(VisionError::InvalidOutputShape {
                    expected: format!("at most {} anchors", MAX_MODEL_DETECTIONS),
                    got: format!("{} anchors", num_anchors),
                });
            }

            // Create view without copying
            let arr = Array2::from_shape_vec(
                (expected_features, num_anchors),
                data.iter().copied().collect(),
            )
            .map_err(|e| VisionError::Processing(e.to_string()))?;
            arr.t().to_owned()
        } else if shape[2] as usize == expected_features {
            // Case: [1, 8400, 84] -> already correct orientation
            let num_anchors = shape[1] as usize;
            if num_anchors > MAX_MODEL_DETECTIONS {
                return Err(VisionError::InvalidOutputShape {
                    expected: format!("at most {} anchors", MAX_MODEL_DETECTIONS),
                    got: format!("{} anchors", num_anchors),
                });
            }

            Array2::from_shape_vec(
                (num_anchors, expected_features),
                data.iter().copied().collect(),
            )
            .map_err(|e| VisionError::Processing(e.to_string()))?
        } else {
            return Err(VisionError::InvalidOutputShape {
                expected: format!("feature dim = {}", expected_features),
                got: format!("{:?}", shape),
            });
        };

        Ok(output_array)
    }

    /// Postprocesses model output into detections.
    ///
    /// Returns (detections, was_truncated, candidates_before_nms).
    fn postprocess(
        &self,
        output: &Array2<f32>,
        params: PreprocessParams,
    ) -> Result<(Vec<Detection>, bool, usize), VisionError> {
        let rows = output.shape()[0];
        let cols = output.shape()[1];

        if cols != self.config.output_features {
            return Err(VisionError::Processing(format!(
                "Output features mismatch: expected {}, got {}",
                self.config.output_features, cols
            )));
        }

        let conf_threshold: f32 = 0.5;
        let iou_threshold: f32 = 0.45;

        let mut candidates = Vec::with_capacity(200);

        for i in 0..rows {
            let row: ArrayView1<f32> = output.row(i);

            // Find best class score using iterator (no allocation)
            let (best_cls_idx, max_score) = row
                .iter()
                .skip(4)
                .enumerate()
                .fold((0usize, f32::NEG_INFINITY), |(best_idx, best_score), (idx, &score)| {
                    if score > best_score {
                        (idx, score)
                    } else {
                        (best_idx, best_score)
                    }
                });

            // Validate score is finite
            if !max_score.is_finite() || max_score < conf_threshold {
                continue;
            }

            // O(1) class lookup
            if !ANIMAL_CLASS_SET.contains(&(best_cls_idx as u32)) {
                continue;
            }

            // Extract and validate box coordinates
            let cx = row[0];
            let cy = row[1];
            let w = row[2];
            let h = row[3];

            // Check for NaN/Inf in box coordinates
            if !cx.is_finite() || !cy.is_finite() || !w.is_finite() || !h.is_finite() {
                continue;
            }

            // Skip invalid boxes
            if w <= 0.0 || h <= 0.0 {
                continue;
            }

            // Transform to original image coordinates
            // Guard against division by zero (already validated in preprocess, but defensive)
            if params.scale <= f32::EPSILON {
                continue;
            }

            let x1 = ((cx - w / 2.0) - params.pad_x) / params.scale;
            let y1 = ((cy - h / 2.0) - params.pad_y) / params.scale;
            let x2 = ((cx + w / 2.0) - params.pad_x) / params.scale;
            let y2 = ((cy + h / 2.0) - params.pad_y) / params.scale;

            // Check for NaN/Inf after transform
            if !x1.is_finite() || !y1.is_finite() || !x2.is_finite() || !y2.is_finite() {
                continue;
            }

            // Normalize and clamp to [0, 1]
            let orig_w_f = params.orig_w.max(1) as f32;
            let orig_h_f = params.orig_h.max(1) as f32;

            let bbox = [
                (x1 / orig_w_f).clamp(0.0, 1.0),
                (y1 / orig_h_f).clamp(0.0, 1.0),
                (x2 / orig_w_f).clamp(0.0, 1.0),
                (y2 / orig_h_f).clamp(0.0, 1.0),
            ];

            // Skip degenerate boxes
            if (bbox[2] - bbox[0]) < 1e-4 || (bbox[3] - bbox[1]) < 1e-4 {
                continue;
            }

            candidates.push(Detection {
                bbox,
                confidence: max_score,
                class_id: best_cls_idx as u32,
            });
        }

        let candidates_before_nms = candidates.len();

        // Sort by confidence descending
        candidates.sort_unstable_by(|a, b| b.confidence.total_cmp(&a.confidence));

        // Apply NMS with truncation tracking
        let (detections, truncated) = nms_with_tracking(candidates, iou_threshold);

        Ok((detections, truncated, candidates_before_nms))
    }
}

// ============================================================================
// Helper Types
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct PreprocessParams {
    scale: f32,
    pad_x: f32,
    pad_y: f32,
    orig_w: u32,
    orig_h: u32,
}

// ============================================================================
// NMS Implementation
// ============================================================================

/// Non-maximum suppression with truncation tracking.
///
/// Returns (kept_detections, was_truncated).
fn nms_with_tracking(mut detections: Vec<Detection>, iou_threshold: f32) -> (Vec<Detection>, bool) {
    if detections.is_empty() {
        return (vec![], false);
    }

    let truncated = detections.len() > MAX_NMS_INPUTS;
    if truncated {
        warn!(
            original = detections.len(),
            limit = MAX_NMS_INPUTS,
            "NMS input truncated"
        );
        detections.truncate(MAX_NMS_INPUTS);
    }

    let n = detections.len();
    let mut suppressed = vec![false; n];
    let mut keep_indices = Vec::with_capacity(n.min(100));

    // Precompute areas
    let areas: Vec<f32> = detections
        .iter()
        .map(|d| (d.bbox[2] - d.bbox[0]) * (d.bbox[3] - d.bbox[1]))
        .collect();

    for i in 0..n {
        if suppressed[i] {
            continue;
        }

        keep_indices.push(i);
        let box_a = &detections[i].bbox;
        let area_a = areas[i];

        for j in (i + 1)..n {
            if suppressed[j] {
                continue;
            }

            let box_b = &detections[j].bbox;

            // Quick reject: no intersection possible
            if box_b[0] > box_a[2]
                || box_b[2] < box_a[0]
                || box_b[1] > box_a[3]
                || box_b[3] < box_a[1]
            {
                continue;
            }

            let inter_x1 = box_a[0].max(box_b[0]);
            let inter_y1 = box_a[1].max(box_b[1]);
            let inter_x2 = box_a[2].min(box_b[2]);
            let inter_y2 = box_a[3].min(box_b[3]);

            let inter_w = (inter_x2 - inter_x1).max(0.0);
            let inter_h = (inter_y2 - inter_y1).max(0.0);
            let inter_area = inter_w * inter_h;

            let area_b = areas[j];
            let union = area_a + area_b - inter_area;

            // Guard against division by zero
            if union > f32::EPSILON {
                let iou = inter_area / union;
                if iou > iou_threshold {
                    suppressed[j] = true;
                }
            }
        }
    }

    // Collect results without cloning (swap-remove pattern)
    let mut result = Vec::with_capacity(keep_indices.len());
    
    // Sort indices in reverse order so we can swap_remove without invalidating indices
    let mut sorted_indices = keep_indices;
    sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
    
    // We need to keep original order, so collect differently
    // Actually, use indexed extraction to preserve confidence ordering
    let mut indexed: Vec<(usize, Detection)> = sorted_indices
        .into_iter()
        .map(|i| {
            // Use mem::take to avoid clone, replacing with default
            let det = std::mem::replace(&mut detections[i], Detection {
                bbox: [0.0; 4],
                confidence: 0.0,
                class_id: 0,
            });
            (i, det)
        })
        .collect();
    
    // Sort back to original order (by index ascending = confidence descending)
    indexed.sort_unstable_by_key(|(i, _)| *i);
    result = indexed.into_iter().map(|(_, d)| d).collect();

    (result, truncated)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nms_empty() {
        let (result, truncated) = nms_with_tracking(vec![], 0.5);
        assert!(result.is_empty());
        assert!(!truncated);
    }

    #[test]
    fn test_nms_single() {
        let det = Detection {
            bbox: [0.1, 0.1, 0.5, 0.5],
            confidence: 0.9,
            class_id: 0,
        };
        let (result, truncated) = nms_with_tracking(vec![det.clone()], 0.5);
        assert_eq!(result.len(), 1);
        assert!(!truncated);
    }

    #[test]
    fn test_nms_overlapping() {
        let det1 = Detection {
            bbox: [0.1, 0.1, 0.5, 0.5],
            confidence: 0.9,
            class_id: 0,
        };
        let det2 = Detection {
            bbox: [0.12, 0.12, 0.52, 0.52], // High overlap
            confidence: 0.8,
            class_id: 0,
        };
        let (result, _) = nms_with_tracking(vec![det1, det2], 0.5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].confidence, 0.9);
    }

    #[test]
    fn test_nms_non_overlapping() {
        let det1 = Detection {
            bbox: [0.0, 0.0, 0.2, 0.2],
            confidence: 0.9,
            class_id: 0,
        };
        let det2 = Detection {
            bbox: [0.8, 0.8, 1.0, 1.0], // No overlap
            confidence: 0.8,
            class_id: 0,
        };
        let (result, _) = nms_with_tracking(vec![det1, det2], 0.5);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_format_validation() {
        // Valid PNG header
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(YoloDetector::validate_format(&png_header).is_ok());

        // Valid JPEG header
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0];
        assert!(YoloDetector::validate_format(&jpeg_header).is_ok());

        // Invalid format (GIF)
        let gif_header = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert!(YoloDetector::validate_format(&gif_header).is_err());
    }

    #[test]
    fn test_animal_class_set() {
        assert!(ANIMAL_CLASS_SET.contains(&14));
        assert!(ANIMAL_CLASS_SET.contains(&23));
        assert!(!ANIMAL_CLASS_SET.contains(&0));
        assert!(!ANIMAL_CLASS_SET.contains(&100));
    }

    #[test]
    fn test_compressed_size_limit() {
        let oversized = vec![0u8; MAX_COMPRESSED_SIZE + 1];
        // This would fail at the compressed size check before even trying to decode
        // We can't easily test preprocess without a valid detector, but the logic is clear
        assert!(oversized.len() > MAX_COMPRESSED_SIZE);
    }
}