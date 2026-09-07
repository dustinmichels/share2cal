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
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple {
    use super::OcrResult;
    use std::ffi::{CStr, CString};
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

    struct AutoCString(*mut c_char);

    impl Drop for AutoCString {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { ocr_apple_free_string(self.0) };
            }
        }
    }

    pub fn extract_from_path(path: &str) -> Result<OcrResult, String> {
        let c_path = CString::new(path).map_err(|e| format!("Invalid path string: {}", e))?;
        let mut out_json: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let ret = unsafe { ocr_apple_from_file(c_path.as_ptr(), &mut out_json, &mut out_error) };

        let _guard_json = AutoCString(out_json);
        let _guard_err = AutoCString(out_error);

        if ret != 0 || out_json.is_null() {
            let err_msg = if !out_error.is_null() {
                unsafe { CStr::from_ptr(out_error).to_string_lossy().into_owned() }
            } else {
                "Unknown OCR error".to_string()
            };
            return Err(err_msg);
        }

        let json_str = unsafe { CStr::from_ptr(out_json).to_string_lossy() };
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse OCR JSON response: {}", e))
    }

    pub fn extract_from_bytes(bytes: &[u8]) -> Result<OcrResult, String> {
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

        let _guard_json = AutoCString(out_json);
        let _guard_err = AutoCString(out_error);

        if ret != 0 || out_json.is_null() {
            let err_msg = if !out_error.is_null() {
                unsafe { CStr::from_ptr(out_error).to_string_lossy().into_owned() }
            } else {
                "Unknown OCR error".to_string()
            };
            return Err(err_msg);
        }

        let json_str = unsafe { CStr::from_ptr(out_json).to_string_lossy() };
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse OCR JSON response: {}", e))
    }
}

pub fn extract_text_from_path(path: &str) -> Result<OcrResult, String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        apple::extract_from_path(path)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        let _ = path;
        Err("OCR is only supported on Apple platforms (macOS/iOS) or Android ML Kit.".to_string())
    }
}

pub fn extract_text_from_bytes(bytes: &[u8]) -> Result<OcrResult, String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        apple::extract_from_bytes(bytes)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        let _ = bytes;
        Err("OCR is only supported on Apple platforms (macOS/iOS) or Android ML Kit.".to_string())
    }
}

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
    fn test_ocr_heif_sample() {
        let sample = get_sample_path("gilman_flyer.heif");
        assert!(sample.exists(), "Sample file {:?} should exist", sample);

        let result = extract_text_from_path(sample.to_str().unwrap()).expect("OCR should succeed on HEIF");
        assert!(!result.text.is_empty(), "Extracted text should not be empty");
        assert!(
            result.text.contains("GILMAN SQUARE"),
            "Expected text to contain 'GILMAN SQUARE'"
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
    fn test_ocr_invalid_file() {
        let res = extract_text_from_path("/nonexistent/file/path.png");
        assert!(res.is_err());
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_ocr_invalid_bytes() {
        let invalid_bytes = b"not an image at all";
        let res = extract_text_from_bytes(invalid_bytes);
        assert!(res.is_err());
    }
}
