use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
    pub bounding_box: Option<BoundingBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrResult {
    pub text: String,
    pub lines: Vec<OcrLine>,
    #[serde(default)]
    pub qr_codes: Vec<String>,
}

/// Returns true if the string starts with http:// or https:// (case-insensitive)
pub fn is_web_link(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Reconstructs line text by clustering 2D bounding boxes into horizontal rows.
/// In Apple Vision / normalized image coordinates, y=0.0 is bottom and y=1.0 is top.
pub fn reconstruct_spatial_lines(lines: &[OcrLine]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let has_boxes = lines.iter().any(|l| l.bounding_box.is_some());
    if !has_boxes {
        return lines
            .iter()
            .map(|l| l.text.as_str())
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }

    #[derive(Clone)]
    struct LineItem<'a> {
        text: &'a str,
        min_x: f64,
        min_y: f64,
        max_y: f64,
        center_y: f64,
        height: f64,
    }
    let mut items: Vec<LineItem> = Vec::new();
    let mut unboxed_items: Vec<&str> = Vec::new();
    for l in lines {
        let t = l.text.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(bb) = &l.bounding_box {
            let height = bb.height.max(0.005);
            let min_y = bb.y;
            let max_y = bb.y + height;
            let center_y = min_y + (height / 2.0);
            let min_x = bb.x;

            items.push(LineItem {
                text: t,
                min_x,
                min_y,
                max_y,
                center_y,
                height,
            });
        } else {
            unboxed_items.push(t);
        }
    }

    if items.is_empty() {
        return unboxed_items.join("\n");
    }
    // Sort items top-to-bottom (center_y descending)
    items.sort_by(|a, b| {
        b.center_y
            .partial_cmp(&a.center_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    struct RowCluster<'a> {
        items: Vec<LineItem<'a>>,
        avg_center_y: f64,
        avg_height: f64,
    }

    let mut clusters: Vec<RowCluster> = Vec::new();

    for item in items {
        let mut matched_idx = None;
        let mut best_overlap = 0.0;

        for (c_idx, cluster) in clusters.iter().enumerate() {
            let ref_h = item.height.min(cluster.avg_height);
            let vert_dist = (item.center_y - cluster.avg_center_y).abs();
            let c_min_y = cluster.avg_center_y - (cluster.avg_height / 2.0);
            let c_max_y = cluster.avg_center_y + (cluster.avg_height / 2.0);
            let overlap_y = item.max_y.min(c_max_y) - item.min_y.max(c_min_y);

            if (overlap_y > (0.35 * ref_h) || vert_dist < (0.35 * ref_h))
                && (overlap_y > best_overlap || matched_idx.is_none())
            {
                best_overlap = overlap_y;
                matched_idx = Some(c_idx);
            }
        }

        if let Some(idx) = matched_idx {
            let cluster = &mut clusters[idx];
            cluster.items.push(item);
            let count = cluster.items.len() as f64;
            cluster.avg_center_y = cluster.items.iter().map(|it| it.center_y).sum::<f64>() / count;
            cluster.avg_height = cluster.items.iter().map(|it| it.height).sum::<f64>() / count;
        } else {
            clusters.push(RowCluster {
                avg_center_y: item.center_y,
                avg_height: item.height,
                items: vec![item],
            });
        }
    }

    // Sort rows from top to bottom (avg_center_y descending)
    clusters.sort_by(|a, b| {
        b.avg_center_y
            .partial_cmp(&a.avg_center_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut row_strings = Vec::new();
    for mut cluster in clusters {
        // Sort left-to-right (min_x ascending)
        cluster.items.sort_by(|a, b| {
            a.min_x
                .partial_cmp(&b.min_x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let row_text = cluster.items.iter().map(|it| it.text).collect::<Vec<_>>().join(" ");
        if !row_text.is_empty() {
            row_strings.push(row_text);
        }
    }

    for unboxed in unboxed_items {
        row_strings.push(unboxed.to_string());
    }

    row_strings.join("\n")
}
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple {
    use super::{AppError, OcrResult};
    use crate::ffi::NativeStringGuard;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    extern "C" {
        fn ocr_apple_from_file(
            path: *const c_char,
            out_json: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;

        fn ocr_apple_from_bytes(
            bytes: *const u8,
            length: usize,
            out_json: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;

        fn ocr_apple_free_string(ptr: *mut c_char);
    }

    pub fn extract_text_from_path(path: &str) -> Result<OcrResult, AppError> {
        let c_path = CString::new(path).map_err(|e| AppError::Ocr(format!("Invalid path string: {}", e)))?;
        let mut out_json: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let ret = unsafe { ocr_apple_from_file(c_path.as_ptr(), &mut out_json, &mut out_error) };

        let guard_json = NativeStringGuard::new(out_json, ocr_apple_free_string);
        let guard_err = NativeStringGuard::new(out_error, ocr_apple_free_string);

        if ret != 0 || guard_json.is_null() {
            let err_msg = guard_err
                .to_string_lossy()
                .unwrap_or_else(|| "Unknown OCR error".to_string());
            return Err(AppError::Ocr(err_msg));
        }

        let json_str = guard_json
            .to_string_lossy()
            .ok_or_else(|| AppError::Ocr("OCR JSON output is null".to_string()))?;
        serde_json::from_str(&json_str)
            .map_err(|e| AppError::Ocr(format!("Failed to parse OCR JSON response: {}", e)))
    }

    pub fn extract_text_from_bytes(bytes: &[u8]) -> Result<OcrResult, AppError> {
        let mut out_json: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let ret = unsafe {
            ocr_apple_from_bytes(
                bytes.as_ptr(),
                bytes.len(),
                &mut out_json,
                &mut out_error,
            )
        };

        let guard_json = NativeStringGuard::new(out_json, ocr_apple_free_string);
        let guard_err = NativeStringGuard::new(out_error, ocr_apple_free_string);

        if ret != 0 || guard_json.is_null() {
            let err_msg = guard_err
                .to_string_lossy()
                .unwrap_or_else(|| "Unknown OCR error".to_string());
            return Err(AppError::Ocr(err_msg));
        }

        let json_str = guard_json
            .to_string_lossy()
            .ok_or_else(|| AppError::Ocr("OCR JSON output is null".to_string()))?;
        serde_json::from_str(&json_str)
            .map_err(|e| AppError::Ocr(format!("Failed to parse OCR JSON response: {}", e)))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod fallback {
    use super::{AppError, OcrResult};

    pub fn extract_text_from_path(_path: &str) -> Result<OcrResult, AppError> {
        Err(AppError::Ocr("OCR is only supported on Apple platforms (macOS/iOS) or Android ML Kit.".to_string()))
    }

    pub fn extract_text_from_bytes(_bytes: &[u8]) -> Result<OcrResult, AppError> {
        Err(AppError::Ocr("OCR is only supported on Apple platforms (macOS/iOS) or Android ML Kit.".to_string()))
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use apple::*;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub use fallback::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_sample_path(filename: &str) -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.parent().unwrap().join("samples").join(filename)
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_png_sample() {
        let sample = get_sample_path("gilman_flyer.png");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed");
        println!("--- Extracted OCR Text for gilman_flyer.png ---\n{}\n---------------------------------------------", result.text);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert!(!result.lines.is_empty(), "Lines should not be empty");
        assert!(
            result.text.contains("GILMAN SQUARE"),
            "Expected text to contain 'GILMAN SQUARE', got:\n{}",
            result.text
        );
        assert!(
            result.text.contains("ARTS & MUSIC FESTIVAL") || result.text.contains("FESTIVAL"),
            "Expected text to contain festival info"
        );
    }
    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_squirrel_flower_sample() {
        let sample = get_sample_path("squirrel_flower.jpg");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed");
        println!("--- Extracted OCR Text for squirrel_flower.jpg ---\n{}\n---------------------------------------------", result.text);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert!(!result.lines.is_empty(), "Lines should not be empty");
        let upper = result.text.to_uppercase();
        assert!(
            upper.contains("FLOWER") || upper.contains("SQUIRRE"),
            "Expected text to contain artist name, got:\n{}",
            result.text
        );
        assert!(
            upper.contains("2026 TOUR") || upper.contains("TOUR"),
            "Expected text to contain tour info, got:\n{}",
            result.text
        );
        assert!(
            upper.contains("SEPTEMBER 26") || upper.contains("SEPTEMBER"),
            "Expected text to contain concert date, got:\n{}",
            result.text
        );
        assert!(
            upper.contains("CRYSTAL BALLROOM"),
            "Expected text to contain venue Crystal Ballroom, got:\n{}",
            result.text
        );
        assert!(
            upper.contains("SOMERVILLE, MA") || upper.contains("SOMERVILLE"),
            "Expected text to contain city Somerville, got:\n{}",
            result.text
        );
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_from_squirrel_flower_bytes() {
        let sample = get_sample_path("squirrel_flower.jpg");
        let bytes = std::fs::read(&sample).expect("Should read squirrel_flower.jpg sample file");

        let result = extract_text_from_bytes(&bytes).expect("OCR from bytes should succeed");
        assert!(!result.text.is_empty());
        let upper = result.text.to_uppercase();
        assert!(upper.contains("FLOWER") || upper.contains("SQUIRRE"));
        assert!(upper.contains("CRYSTAL BALLROOM"));
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_instagram_sample() {
        let sample = get_sample_path("instagram.png");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed");
        println!("--- Extracted OCR Text for instagram.png ---\n{}\n---------------------------------------------", result.text);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert!(
            result.text.contains("UEP ICE CREAM SOCIAL"),
            "Expected text to contain 'UEP ICE CREAM SOCIAL', got:\n{}",
            result.text
        );
        assert!(
            result.text.contains("BP LAWN"),
            "Expected text to contain 'BP LAWN', got:\n{}",
            result.text
        );
        assert!(
            result.text.contains("12:00 PM - 1:00 PM") || result.text.contains("12:00 PM"),
            "Expected text to contain time range, got:\n{}",
            result.text
        );
        assert!(
            result.text.contains("09 Sept") || result.text.contains("Sept"),
            "Expected text to contain date, got:\n{}",
            result.text
        );
        assert!(
            result.text.contains("dessert with us") || result.text.contains("ice cream"),
            "Expected text to contain details, got:\n{}",
            result.text
        );
    }
    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_class_sample() {
        let sample = get_sample_path("class.png");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed on class.png");
        println!("--- Extracted OCR Text for class.png ---\n{}\n---------------------------------------------", result.text);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
    }
    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_ride_for_life_sample() {
        let sample = get_sample_path("ride_for_life.png");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed on ride_for_life.png");
        println!("--- Extracted OCR Text for ride_for_life.png ---\n{}\n---------------------------------------------", result.text);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        let upper = result.text.to_uppercase();
        assert!(
            upper.contains("RIDE") || upper.contains("LIFE"),
            "Expected text to contain 'RIDE' or 'LIFE', got:\n{}",
            result.text
        );
    }
    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_commons_sample() {
        let sample = get_sample_path("commons.jpg");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed on commons.jpg");
        println!("--- Extracted OCR Text for commons.jpg ---\n{}\n---------------------------------------------", result.text);
        println!("--- Extracted QR Codes for commons.jpg ---\n{:?}\n---------------------------------------------", result.qr_codes);
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert_eq!(
            result.qr_codes,
            vec!["https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw"],
            "QR code should be correctly decoded from commons.jpg"
        );
    }
    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_from_bytes() {
        let sample = get_sample_path("gilman_flyer.png");
        let bytes = std::fs::read(&sample).expect("Should read sample file");

        let result = extract_text_from_bytes(&bytes).expect("OCR from bytes should succeed");
        assert!(!result.text.is_empty());
        assert!(result.text.contains("GILMAN SQUARE"));
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_qr_code_from_bytes() {
        let sample = get_sample_path("commons.jpg");
        let bytes = std::fs::read(&sample).expect("Should read commons.jpg sample file");

        let result = extract_text_from_bytes(&bytes).expect("OCR from bytes should succeed on commons.jpg");
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert_eq!(
            result.qr_codes,
            vec!["https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw"],
            "QR code should be correctly decoded from commons.jpg bytes"
        );
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_invalid_file() {
        let res = extract_text_from_path("/nonexistent/file/path.png");
        assert!(res.is_err());
    }

    #[test]
    fn test_reconstruct_spatial_lines_multi_column_table() {
        // Simulate columnar OCR output where columns were recognized separately:
        // Row 1 (top, y=0.80): "CEE 0154-03" (x=0.05), "Principles Epidemiology" (x=0.25), "Mo, We 3:00PM - 4:15PM" (x=0.55)
        // Row 2 (bottom, y=0.60): "CS 0150-09" (x=0.05), "Special Topics" (x=0.25), "Fr 2:00PM - 4:30PM" (x=0.55)
        // Passed in out-of-order column order (all code lines first, then descriptions, then times)
        let lines = vec![
            OcrLine {
                text: "CEE 0154-03".to_string(),
                confidence: 0.99,
                bounding_box: Some(BoundingBox { x: 0.05, y: 0.80, width: 0.15, height: 0.03 }),
            },
            OcrLine {
                text: "CS 0150-09".to_string(),
                confidence: 0.99,
                bounding_box: Some(BoundingBox { x: 0.05, y: 0.60, width: 0.15, height: 0.03 }),
            },
            OcrLine {
                text: "Principles Epidemiology (Lecture)".to_string(),
                confidence: 0.98,
                bounding_box: Some(BoundingBox { x: 0.25, y: 0.80, width: 0.25, height: 0.03 }),
            },
            OcrLine {
                text: "Special Topics (Lecture)".to_string(),
                confidence: 0.98,
                bounding_box: Some(BoundingBox { x: 0.25, y: 0.60, width: 0.25, height: 0.03 }),
            },
            OcrLine {
                text: "Mo, We 3:00PM - 4:15PM".to_string(),
                confidence: 0.95,
                bounding_box: Some(BoundingBox { x: 0.55, y: 0.80, width: 0.30, height: 0.03 }),
            },
            OcrLine {
                text: "Fr 2:00PM - 4:30PM".to_string(),
                confidence: 0.95,
                bounding_box: Some(BoundingBox { x: 0.55, y: 0.60, width: 0.30, height: 0.03 }),
            },
        ];

        let reconstructed = reconstruct_spatial_lines(&lines);
        let rows: Vec<&str> = reconstructed.lines().collect();
        assert_eq!(rows.len(), 2);

        assert_eq!(
            rows[0],
            "CEE 0154-03 Principles Epidemiology (Lecture) Mo, We 3:00PM - 4:15PM"
        );
        assert_eq!(
            rows[1],
            "CS 0150-09 Special Topics (Lecture) Fr 2:00PM - 4:30PM"
        );
    }

    #[test]
    fn test_reconstruct_spatial_lines_fallback_no_boxes() {
        let lines = vec![
            OcrLine {
                text: "Line 1".to_string(),
                confidence: 0.9,
                bounding_box: None,
            },
            OcrLine {
                text: "Line 2".to_string(),
                confidence: 0.9,
                bounding_box: None,
            },
        ];
        let text = reconstruct_spatial_lines(&lines);
        assert_eq!(text, "Line 1\nLine 2");
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_invalid_bytes() {
        let invalid_bytes = b"not an image at all";
        let res = extract_text_from_bytes(invalid_bytes);
        assert!(res.is_err());
    }

    #[test]
    fn test_ocr_result_serde_backward_compatibility() {
        // 1. Deserializing legacy JSON without qr_codes field
        let legacy_json = r#"{"text":"Some flyer text","lines":[]}"#;
        let result: OcrResult = serde_json::from_str(legacy_json).expect("Legacy JSON should deserialize");
        assert_eq!(result.text, "Some flyer text");
        assert!(result.lines.is_empty());
        assert!(result.qr_codes.is_empty(), "qr_codes should default to empty vec if omitted");

        // 2. Deserializing modern JSON with qr_codes
        let modern_json = r#"{"text":"Flyer with QR","lines":[],"qr_codes":["https://example.com/rsvp"]}"#;
        let modern_result: OcrResult = serde_json::from_str(modern_json).expect("Modern JSON should deserialize");
        assert_eq!(modern_result.qr_codes, vec!["https://example.com/rsvp".to_string()]);

        // 3. Serialization round-trip
        let serialized = serde_json::to_string(&modern_result).expect("Serialization should succeed");
        let roundtrip: OcrResult = serde_json::from_str(&serialized).expect("Roundtrip should succeed");
        assert_eq!(modern_result, roundtrip);
    }
}
