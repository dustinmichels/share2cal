use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
pub const DEFAULT_APP_GROUP_ID: &str = "group.com.dustinmichels.share2cal";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedImagePayload {
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub timestamp: u64,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

/// Fallback staging directory for cross-platform support and test isolation
fn get_fallback_shared_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SHARE2CAL_SHARED_DIR") {
        return PathBuf::from(dir);
    }
    std::env::temp_dir().join("share2cal_shared_images")
}

fn get_fallback_pending_shared_image(include_bytes: bool) -> Result<Option<SharedImagePayload>, AppError> {
    let shared_dir = get_fallback_shared_dir();
    let manifest_path = shared_dir.join("pending_share.json");

    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path).map_err(|e| AppError::Io(e.to_string()))?;
        if let Ok(mut payload) = serde_json::from_str::<SharedImagePayload>(&content) {
            if Path::new(&payload.file_path).exists() {
                if include_bytes && payload.bytes.is_none() {
                    payload.bytes = fs::read(&payload.file_path).ok();
                }
                return Ok(Some(payload));
            }
        }
    }

    if shared_dir.exists() {
        if let Ok(entries) = fs::read_dir(&shared_dir) {
            let mut latest_entry: Option<(PathBuf, fs::Metadata)> = None;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().map(|f| f == "pending_share.json").unwrap_or(false) {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        if let Some((_, ref latest_meta)) = latest_entry {
                            if let (Ok(m1), Ok(m2)) = (meta.modified(), latest_meta.modified()) {
                                if m1 > m2 {
                                    latest_entry = Some((path, meta));
                                }
                            }
                        } else {
                            latest_entry = Some((path, meta));
                        }
                    }
                }
            }

            if let Some((path, meta)) = latest_entry {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "shared_image.png".to_string());

                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                let mime_type = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "heic" | "heif" => "image/heic",
                    "webp" => "image/webp",
                    _ => "image/png",
                }
                .to_string();

                let bytes = if include_bytes {
                    Some(fs::read(&path).map_err(|e| AppError::Io(e.to_string()))?)
                } else {
                    None
                };
                let timestamp = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                return Ok(Some(SharedImagePayload {
                    file_name,
                    file_path: path.to_string_lossy().into_owned(),
                    mime_type,
                    size_bytes: meta.len() as usize,
                    timestamp,
                    source: "fallback_shared_dir".to_string(),
                    bytes,
                }));
            }
        }
    }

    Ok(None)
}

fn clear_fallback_shared_dir() -> Result<(), AppError> {
    let shared_dir = get_fallback_shared_dir();
    if shared_dir.exists() {
        if let Ok(entries) = fs::read_dir(&shared_dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

fn stage_fallback_shared_image(
    bytes: &[u8],
    file_name: &str,
    mime_type: &str,
) -> Result<SharedImagePayload, AppError> {
    let shared_dir = get_fallback_shared_dir();
    fs::create_dir_all(&shared_dir).map_err(|e| AppError::Io(e.to_string()))?;

    let file_path = shared_dir.join(file_name);
    fs::write(&file_path, bytes).map_err(|e| AppError::Io(e.to_string()))?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let payload = SharedImagePayload {
        file_name: file_name.to_string(),
        file_path: file_path.to_string_lossy().into_owned(),
        mime_type: mime_type.to_string(),
        size_bytes: bytes.len(),
        timestamp,
        source: "fallback_staging".to_string(),
        bytes: Some(bytes.to_vec()),
    };

    let manifest_path = shared_dir.join("pending_share.json");
    if let Ok(json) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(manifest_path, json);
    }

    Ok(payload)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple {
    use super::{
        clear_fallback_shared_dir, get_fallback_pending_shared_image, stage_fallback_shared_image,
        AppError, SharedImagePayload, DEFAULT_APP_GROUP_ID,
    };
    use crate::ffi::NativeStringGuard;
    use std::ffi::CString;
    use std::fs;
    use std::os::raw::{c_char, c_uchar};
    use std::path::Path;

    extern "C" {
        fn get_app_group_pending_share_json(group_id: *const c_char) -> *mut c_char;
        fn get_app_group_shared_image_path(group_id: *const c_char) -> *mut c_char;
        fn clear_app_group_shared_data(group_id: *const c_char) -> bool;
        fn save_app_group_shared_image(
            group_id: *const c_char,
            bytes: *const c_uchar,
            len: usize,
            filename: *const c_char,
            mime_type: *const c_char,
        ) -> bool;
        fn free_share_string(str: *mut c_char);
    }

    pub fn get_pending_shared_image(include_bytes: bool) -> Result<Option<SharedImagePayload>, AppError> {
        let group_c_str = CString::new(DEFAULT_APP_GROUP_ID).map_err(|e| AppError::Io(e.to_string()))?;
        // 1. Try reading the manifest JSON via native App Group FFI
        let json_ptr = unsafe { get_app_group_pending_share_json(group_c_str.as_ptr()) };
        let json_guard = NativeStringGuard::new(json_ptr, free_share_string);
        if let Some(json_str) = json_guard.to_string_lossy() {
            if let Ok(mut payload) = serde_json::from_str::<SharedImagePayload>(&json_str) {
                if include_bytes && payload.bytes.is_none() && Path::new(&payload.file_path).exists() {
                    if let Ok(data) = fs::read(&payload.file_path) {
                        payload.bytes = Some(data);
                    }
                }
                return Ok(Some(payload));
            }
        }

        // 2. Try reading shared image path directly
        let path_ptr = unsafe { get_app_group_shared_image_path(group_c_str.as_ptr()) };
        let path_guard = NativeStringGuard::new(path_ptr, free_share_string);
        if let Some(path_str) = path_guard.to_string_lossy() {
            let path = Path::new(&path_str);
            if path.exists() {
                let metadata = fs::metadata(path).map_err(|e| AppError::Io(e.to_string()))?;
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "shared_flyer.png".to_string());

                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                let mime_type = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "heic" | "heif" => "image/heic",
                    "webp" => "image/webp",
                    _ => "image/png",
                }
                .to_string();

                let bytes = if include_bytes {
                    Some(fs::read(path).map_err(|e| AppError::Io(e.to_string()))?)
                } else {
                    None
                };
                let timestamp = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                return Ok(Some(SharedImagePayload {
                    file_name,
                    file_path: path_str,
                    mime_type,
                    size_bytes: metadata.len() as usize,
                    timestamp,
                    source: "native_apple_share".to_string(),
                    bytes,
                }));
            }
        }

        // Fallback: check staging directory
        get_fallback_pending_shared_image(include_bytes)
    }

    pub fn clear_pending_shared_image() -> Result<(), AppError> {
        if let Ok(group_c_str) = CString::new(DEFAULT_APP_GROUP_ID) {
            unsafe {
                clear_app_group_shared_data(group_c_str.as_ptr());
            }
        }
        clear_fallback_shared_dir()
    }

    pub fn stage_shared_image(
        bytes: &[u8],
        file_name: &str,
        mime_type: Option<&str>,
    ) -> Result<SharedImagePayload, AppError> {
        if bytes.is_empty() {
            return Err(AppError::Io("Cannot stage empty image bytes".to_string()));
        }

        let mime = mime_type.unwrap_or("image/png");

        let group_c_str = CString::new(DEFAULT_APP_GROUP_ID).map_err(|e| AppError::Io(e.to_string()))?;
        let name_c_str = CString::new(file_name).map_err(|e| AppError::Io(e.to_string()))?;
        let mime_c_str = CString::new(mime).map_err(|e| AppError::Io(e.to_string()))?;

        let saved = unsafe {
            save_app_group_shared_image(
                group_c_str.as_ptr(),
                bytes.as_ptr(),
                bytes.len(),
                name_c_str.as_ptr(),
                mime_c_str.as_ptr(),
            )
        };

        if saved {
            if let Ok(Some(payload)) = get_pending_shared_image(true) {
                return Ok(payload);
            }
        }

        // Fallback staging
        stage_fallback_shared_image(bytes, file_name, mime)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod fallback {
    use super::{
        clear_fallback_shared_dir, get_fallback_pending_shared_image, stage_fallback_shared_image,
        AppError, SharedImagePayload,
    };

    pub fn get_pending_shared_image(include_bytes: bool) -> Result<Option<SharedImagePayload>, AppError> {
        get_fallback_pending_shared_image(include_bytes)
    }

    pub fn clear_pending_shared_image() -> Result<(), AppError> {
        clear_fallback_shared_dir()
    }

    pub fn stage_shared_image(
        bytes: &[u8],
        file_name: &str,
        mime_type: Option<&str>,
    ) -> Result<SharedImagePayload, AppError> {
        if bytes.is_empty() {
            return Err(AppError::Io("Cannot stage empty image bytes".to_string()));
        }
        let mime = mime_type.unwrap_or("image/png");
        stage_fallback_shared_image(bytes, file_name, mime)
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use apple::*;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub use fallback::*;
/// Reads an image from the filesystem given its path (e.g. from a drag-and-drop event).
pub fn load_image_from_path(path: &str) -> Result<SharedImagePayload, AppError> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(AppError::Io(format!("File does not exist at path: {}", path)));
    }
    let metadata = fs::metadata(p).map_err(|e| AppError::Io(format!("Failed to read metadata: {}", e)))?;
    if metadata.is_dir() {
        return Err(AppError::Io("Dropped item is a directory, not an image file".to_string()));
    }
    let file_name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dropped_image.png".to_string());

    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mime_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
    .to_string();

    let bytes = fs::read(p).map_err(|e| AppError::Io(format!("Failed to read file bytes: {}", e)))?;
    let timestamp = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(SharedImagePayload {
        file_name,
        file_path: path.to_string(),
        mime_type,
        size_bytes: metadata.len() as usize,
        timestamp,
        source: "drag_and_drop".to_string(),
        bytes: Some(bytes),
    })
}


#[doc(hidden)]
pub static TEST_SHARE_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_and_retrieve_shared_image() {
        let _guard = TEST_SHARE_MUTEX.lock().unwrap();
        let sample_bytes = b"fake-png-image-bytes-for-unit-test";
        let file_name = "test_flyer.png";

        // Clear any previous state
        let _ = clear_pending_shared_image();

        // Stage image
        let staged = stage_shared_image(sample_bytes, file_name, Some("image/png")).expect("Staging should succeed");
        assert_eq!(staged.file_name, file_name);
        assert_eq!(staged.mime_type, "image/png");
        assert_eq!(staged.size_bytes, sample_bytes.len());

        // Retrieve image with bytes
        let retrieved = get_pending_shared_image(true).expect("Retrieval should succeed");
        assert!(retrieved.is_some(), "Pending shared image must be found");

        let payload = retrieved.unwrap();
        assert_eq!(payload.file_name, file_name);
        assert_eq!(payload.size_bytes, sample_bytes.len());
        assert!(payload.bytes.is_some());
        assert_eq!(payload.bytes.unwrap(), sample_bytes.to_vec());

        // Clear image
        clear_pending_shared_image().expect("Clear should succeed");

        // Verify it is empty now
        let after_clear = get_pending_shared_image(false).expect("Retrieval after clear should succeed");
        assert!(after_clear.is_none(), "Pending shared image must be cleared");
    }

    #[test]
    fn test_empty_bytes_rejected() {
        let _guard = TEST_SHARE_MUTEX.lock().unwrap();
        let result = stage_shared_image(&[], "empty.png", None);
        assert!(result.is_err(), "Empty bytes must return an error");
    }

    #[test]
    fn test_load_image_from_path_success() {
        let _guard = TEST_SHARE_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("share2cal_test_load_image");
        let _ = fs::create_dir_all(&temp_dir);
        let temp_file = temp_dir.join("test_sample.png");
        let sample_bytes = b"mock-png-content-for-drag-drop-test";
        fs::write(&temp_file, sample_bytes).expect("Write temp image file");

        let loaded = load_image_from_path(&temp_file.to_string_lossy()).expect("Load image from path");
        assert_eq!(loaded.file_name, "test_sample.png");
        assert_eq!(loaded.mime_type, "image/png");
        assert_eq!(loaded.size_bytes, sample_bytes.len());
        assert_eq!(loaded.source, "drag_and_drop");
        assert_eq!(loaded.bytes.unwrap(), sample_bytes.to_vec());

        let _ = fs::remove_file(temp_file);
    }

    #[test]
    fn test_load_image_from_path_nonexistent() {
        let _guard = TEST_SHARE_MUTEX.lock().unwrap();
        let non_existent = std::env::temp_dir().join("definitely_non_existent_image_12345.png");
        let result = load_image_from_path(&non_existent.to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_load_image_from_path_directory() {
        let _guard = TEST_SHARE_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let result = load_image_from_path(&temp_dir.to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("directory"));
    }
}
