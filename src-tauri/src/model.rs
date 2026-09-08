use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

pub const MANIFEST_JSON: &str = include_str!("../model-manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub version: String,
    pub updated_at: String,
    pub default_model_id: String,
    pub models: Vec<ModelManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifestEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub repo: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub quantization: String,
    pub description: String,
    pub recommended_ram: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub file_path: Option<String>,
    pub storage_dir: String,
    pub sha256: String,
    pub is_verified: bool,
    pub error: Option<String>,
    pub quantization: String,
    pub description: String,
    pub recommended_ram: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgressPayload {
    pub model_id: String,
    pub received_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: f64,
    pub status: String, // "downloading" | "verifying" | "completed" | "error" | "cancelled"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsStorageInfo {
    pub storage_dir: String,
    pub total_models_downloaded: usize,
    pub total_models_size_bytes: u64,
    pub free_disk_space_bytes: Option<u64>,
}

/// Global download manager state storing active download cancel flags
pub struct DownloadState {
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            cancel_flags: Mutex::new(HashMap::new()),
        }
    }
}

pub fn get_manifest() -> Result<ModelManifest, String> {
    serde_json::from_str(MANIFEST_JSON).map_err(|e| format!("Failed to parse model manifest: {}", e))
}

/// Gets the directory where models are stored
pub fn get_storage_directory(app: &AppHandle) -> Result<PathBuf, String> {
    // 1. If override env var is set (for tests / custom sandbox)
    if let Ok(dir_str) = std::env::var("SHARE2CAL_MODELS_DIR") {
        let dir = PathBuf::from(dir_str);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        return Ok(dir);
    }

    // 2. Use app data / local data directory from Tauri
    let base_dir = app
        .path()
        .app_data_dir()
        .or_else(|_| app.path().app_local_data_dir())
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?;

    let models_dir = base_dir.join("models");
    fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models directory: {}", e))?;
    Ok(models_dir)
}

/// Gets free disk space in bytes on Unix/macOS/iOS platforms

/// Opens the models storage directory in the native file manager on desktop platforms
pub fn open_models_directory(app: &AppHandle) -> Result<(), String> {
    let storage_dir = get_storage_directory(app)?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&storage_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory in Finder: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&storage_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory in Explorer: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&storage_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Opening directory browser is not supported on mobile operating systems due to application sandboxing.".to_string())
    }
}
pub fn get_free_disk_space(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path_c = CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(path_c.as_ptr(), &mut stat) } == 0 {
            let free = (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64);
            return Some(free);
        }
    }
    None
}

/// Returns the status of all available models in the manifest
pub fn get_all_model_statuses(app: &AppHandle) -> Result<Vec<ModelStatus>, String> {
    let manifest = get_manifest()?;
    let storage_dir = get_storage_directory(app)?;
    let storage_dir_str = storage_dir.to_string_lossy().to_string();

    let state = app.try_state::<DownloadState>();

    let mut statuses = Vec::new();
    for entry in manifest.models {
        let target_file = storage_dir.join(&entry.filename);
        let part_file = storage_dir.join(format!("{}.part", &entry.filename));

        let is_downloading = if let Some(st) = &state {
            if let Ok(guard) = st.cancel_flags.lock() {
                guard.contains_key(&entry.id)
            } else {
                false
            }
        } else {
            false
        };
        let (is_downloaded, downloaded_bytes, file_path) = if target_file.is_file() {
            let metadata = fs::metadata(&target_file).ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            (size > 0, size, Some(target_file.to_string_lossy().to_string()))
        } else if part_file.is_file() {
            let metadata = fs::metadata(&part_file).ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            (false, size, None)
        } else {
            (false, 0, None)
        };

        statuses.push(ModelStatus {
            id: entry.id,
            name: entry.name,
            filename: entry.filename,
            size_bytes: entry.size_bytes,
            downloaded_bytes,
            is_downloaded,
            is_downloading,
            file_path,
            storage_dir: storage_dir_str.clone(),
            sha256: entry.sha256,
            is_verified: is_downloaded, // Verified upon download finish
            error: None,
            quantization: entry.quantization,
            description: entry.description,
            recommended_ram: entry.recommended_ram,
            is_default: entry.is_default,
        });
    }

    Ok(statuses)
}

/// Returns the status of a specific model by ID
pub fn get_single_model_status(app: &AppHandle, model_id: &str) -> Result<ModelStatus, String> {
    let statuses = get_all_model_statuses(app)?;
    statuses
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model with id '{}' not found in manifest", model_id))
}

/// Computes the SHA-256 checksum of a file
pub fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file for SHA-256: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024]; // 64KB buffer

    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Error reading file: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verifies whether the downloaded model file matches the expected SHA-256 checksum
pub fn verify_model(app: &AppHandle, model_id: &str) -> Result<bool, String> {
    let manifest = get_manifest()?;
    let entry = manifest
        .models
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model '{}' not found in manifest", model_id))?;

    let storage_dir = get_storage_directory(app)?;
    let target_file = storage_dir.join(&entry.filename);

    if !target_file.is_file() {
        return Ok(false);
    }

    let hash = compute_file_sha256(&target_file)?;
    Ok(hash.eq_ignore_ascii_case(&entry.sha256))
}

/// Deletes a downloaded model and any partial temporary files
pub fn delete_model_file(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let manifest = get_manifest()?;
    let entry = manifest
        .models
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model '{}' not found in manifest", model_id))?;

    let storage_dir = get_storage_directory(app)?;
    let target_file = storage_dir.join(&entry.filename);
    let part_file = storage_dir.join(format!("{}.part", &entry.filename));

    if target_file.exists() {
        fs::remove_file(&target_file).map_err(|e| format!("Failed to delete model file: {}", e))?;
    }
    if part_file.exists() {
        let _ = fs::remove_file(&part_file);
    }

    Ok(())
}

/// Cancels an active download for a model
pub fn cancel_download(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let state = app
        .try_state::<DownloadState>()
        .ok_or_else(|| "Download state not initialized".to_string())?;

    if let Ok(mut guard) = state.cancel_flags.lock() {
        if let Some(flag) = guard.remove(model_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }

    let _ = app.emit(
        "model_download_progress",
        DownloadProgressPayload {
            model_id: model_id.to_string(),
            received_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            speed_bytes_per_sec: 0.0,
            status: "cancelled".to_string(),
            error: None,
        },
    );

    Ok(())
}

/// Asynchronously downloads a model from Hugging Face with progress streaming and SHA-256 verification
pub async fn start_model_download(app: AppHandle, model_id: String) -> Result<(), String> {
    let manifest = get_manifest()?;
    let entry = manifest
        .models
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model '{}' not found in manifest", model_id))?;

    let storage_dir = get_storage_directory(&app)?;
    let target_file = storage_dir.join(&entry.filename);
    let part_file = storage_dir.join(format!("{}.part", &entry.filename));

    // Pre-flight: Check free disk space (require at least 1.5x model size)
    if let Some(free_space) = get_free_disk_space(&storage_dir) {
        let required_space = (entry.size_bytes as f64 * 1.5) as u64;
        if free_space < required_space {
            let free_mb = free_space / (1024 * 1024);
            let req_mb = required_space / (1024 * 1024);
            let err_msg = format!(
                "Insufficient disk space: {} MB free, but {} MB required.",
                free_mb, req_mb
            );
            let _ = app.emit(
                "model_download_progress",
                DownloadProgressPayload {
                    model_id: entry.id.clone(),
                    received_bytes: 0,
                    total_bytes: entry.size_bytes,
                    percentage: 0.0,
                    speed_bytes_per_sec: 0.0,
                    status: "error".to_string(),
                    error: Some(err_msg.clone()),
                },
            );
            return Err(err_msg);
        }
    }

    // Set up cancel flag
    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Some(state) = app.try_state::<DownloadState>() {
        if let Ok(mut guard) = state.cancel_flags.lock() {
            guard.insert(entry.id.clone(), Arc::clone(&cancel_flag));
        }
    }

    // Check if there is an existing partial download
    let existing_bytes = if part_file.exists() {
        fs::metadata(&part_file).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client.get(&entry.url);
    if existing_bytes > 0 && existing_bytes < entry.size_bytes {
        request = request.header("Range", format!("bytes={}-", existing_bytes));
    }

    let response = match request.send().await {
        Ok(res) => res,
        Err(e) => {
            cleanup_cancel_flag(&app, &entry.id);
            let err_msg = format!("Download request failed: {}", e);
            let _ = app.emit(
                "model_download_progress",
                DownloadProgressPayload {
                    model_id: entry.id.clone(),
                    received_bytes: existing_bytes,
                    total_bytes: entry.size_bytes,
                    percentage: (existing_bytes as f64 / entry.size_bytes as f64) * 100.0,
                    speed_bytes_per_sec: 0.0,
                    status: "error".to_string(),
                    error: Some(err_msg.clone()),
                },
            );
            return Err(err_msg);
        }
    };

    let status = response.status();
    let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !status.is_success() {
        cleanup_cancel_flag(&app, &entry.id);
        let err_msg = format!("Server returned HTTP status {}", status);
        let _ = app.emit(
            "model_download_progress",
            DownloadProgressPayload {
                model_id: entry.id.clone(),
                received_bytes: existing_bytes,
                total_bytes: entry.size_bytes,
                percentage: 0.0,
                speed_bytes_per_sec: 0.0,
                status: "error".to_string(),
                error: Some(err_msg.clone()),
            },
        );
        return Err(err_msg);
    }

    // Open file for appending if partial, or create new
    let mut file = match if is_partial {
        fs::OpenOptions::new().create(true).append(true).open(&part_file)
    } else {
        fs::File::create(&part_file)
    } {
        Ok(f) => f,
        Err(e) => {
            cleanup_cancel_flag(&app, &entry.id);
            let err_msg = format!("Failed to open part file: {}", e);
            return Err(err_msg);
        }
    };

    let start_downloaded = if is_partial { existing_bytes } else { 0 };
    let mut downloaded = start_downloaded;
    let total_bytes = entry.size_bytes;

    let mut stream = response.bytes_stream();
    let _start_time = Instant::now();
    let mut last_emit = Instant::now();
    let mut bytes_since_last_emit = 0u64;

    // Initial progress event
    let _ = app.emit(
        "model_download_progress",
        DownloadProgressPayload {
            model_id: entry.id.clone(),
            received_bytes: downloaded,
            total_bytes,
            percentage: (downloaded as f64 / total_bytes as f64) * 100.0,
            speed_bytes_per_sec: 0.0,
            status: "downloading".to_string(),
            error: None,
        },
    );

    while let Some(chunk_result) = stream.next().await {
        if cancel_flag.load(Ordering::SeqCst) {
            cleanup_cancel_flag(&app, &entry.id);
            let _ = app.emit(
                "model_download_progress",
                DownloadProgressPayload {
                    model_id: entry.id.clone(),
                    received_bytes: downloaded,
                    total_bytes,
                    percentage: (downloaded as f64 / total_bytes as f64) * 100.0,
                    speed_bytes_per_sec: 0.0,
                    status: "cancelled".to_string(),
                    error: None,
                },
            );
            return Ok(());
        }

        let chunk = match chunk_result {
            Ok(c) => c,
            Err(e) => {
                cleanup_cancel_flag(&app, &entry.id);
                let err_msg = format!("Stream error during download: {}", e);
                let _ = app.emit(
                    "model_download_progress",
                    DownloadProgressPayload {
                        model_id: entry.id.clone(),
                        received_bytes: downloaded,
                        total_bytes,
                        percentage: (downloaded as f64 / total_bytes as f64) * 100.0,
                        speed_bytes_per_sec: 0.0,
                        status: "error".to_string(),
                        error: Some(err_msg.clone()),
                    },
                );
                return Err(err_msg);
            }
        };

        if let Err(e) = file.write_all(&chunk) {
            cleanup_cancel_flag(&app, &entry.id);
            let err_msg = format!("Failed to write chunk to disk: {}", e);
            return Err(err_msg);
        }

        downloaded += chunk.len() as u64;
        bytes_since_last_emit += chunk.len() as u64;

        if last_emit.elapsed().as_millis() >= 200 {
            let elapsed_sec = last_emit.elapsed().as_secs_f64();
            let speed = if elapsed_sec > 0.0 {
                bytes_since_last_emit as f64 / elapsed_sec
            } else {
                0.0
            };

            let _ = app.emit(
                "model_download_progress",
                DownloadProgressPayload {
                    model_id: entry.id.clone(),
                    received_bytes: downloaded,
                    total_bytes,
                    percentage: (downloaded as f64 / total_bytes as f64) * 100.0,
                    speed_bytes_per_sec: speed,
                    status: "downloading".to_string(),
                    error: None,
                },
            );

            last_emit = Instant::now();
            bytes_since_last_emit = 0;
        }
    }

    if let Err(e) = file.flush() {
        cleanup_cancel_flag(&app, &entry.id);
        return Err(format!("Failed to flush file to disk: {}", e));
    }
    drop(file);

    // Status: Verifying SHA-256
    let _ = app.emit(
        "model_download_progress",
        DownloadProgressPayload {
            model_id: entry.id.clone(),
            received_bytes: downloaded,
            total_bytes,
            percentage: 100.0,
            speed_bytes_per_sec: 0.0,
            status: "verifying".to_string(),
            error: None,
        },
    );

    let actual_hash = match compute_file_sha256(&part_file) {
        Ok(h) => h,
        Err(e) => {
            cleanup_cancel_flag(&app, &entry.id);
            let err_msg = format!("Failed to compute SHA-256 of downloaded file: {}", e);
            let _ = app.emit(
                "model_download_progress",
                DownloadProgressPayload {
                    model_id: entry.id.clone(),
                    received_bytes: downloaded,
                    total_bytes,
                    percentage: 100.0,
                    speed_bytes_per_sec: 0.0,
                    status: "error".to_string(),
                    error: Some(err_msg.clone()),
                },
            );
            return Err(err_msg);
        }
    };

    if !actual_hash.eq_ignore_ascii_case(&entry.sha256) {
        cleanup_cancel_flag(&app, &entry.id);
        let _ = fs::remove_file(&part_file);
        let err_msg = format!(
            "SHA-256 checksum mismatch! Expected: {}, Computed: {}",
            entry.sha256, actual_hash
        );
        let _ = app.emit(
            "model_download_progress",
            DownloadProgressPayload {
                model_id: entry.id.clone(),
                received_bytes: downloaded,
                total_bytes,
                percentage: 100.0,
                speed_bytes_per_sec: 0.0,
                status: "error".to_string(),
                error: Some(err_msg.clone()),
            },
        );
        return Err(err_msg);
    }

    // Rename part file to final model destination
    if target_file.exists() {
        let _ = fs::remove_file(&target_file);
    }
    if let Err(e) = fs::rename(&part_file, &target_file) {
        cleanup_cancel_flag(&app, &entry.id);
        let err_msg = format!("Failed to move verified model into place: {}", e);
        return Err(err_msg);
    }

    cleanup_cancel_flag(&app, &entry.id);

    // Final completed event
    let _ = app.emit(
        "model_download_progress",
        DownloadProgressPayload {
            model_id: entry.id.clone(),
            received_bytes: downloaded,
            total_bytes,
            percentage: 100.0,
            speed_bytes_per_sec: 0.0,
            status: "completed".to_string(),
            error: None,
        },
    );

    Ok(())
}

fn cleanup_cancel_flag(app: &AppHandle, model_id: &str) {
    if let Some(state) = app.try_state::<DownloadState>() {
        if let Ok(mut guard) = state.cancel_flags.lock() {
            guard.remove(model_id);
        }
    }
}

pub fn get_models_storage_info(app: &AppHandle) -> Result<ModelsStorageInfo, String> {
    let storage_dir = get_storage_directory(app)?;
    let statuses = get_all_model_statuses(app)?;

    let total_models_downloaded = statuses.iter().filter(|m| m.is_downloaded).count();
    let total_models_size_bytes = statuses
        .iter()
        .filter(|m| m.is_downloaded)
        .map(|m| m.downloaded_bytes)
        .sum();

    let free_disk_space_bytes = get_free_disk_space(&storage_dir);

    Ok(ModelsStorageInfo {
        storage_dir: storage_dir.to_string_lossy().to_string(),
        total_models_downloaded,
        total_models_size_bytes,
        free_disk_space_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parses_cleanly() {
        let manifest = get_manifest().expect("manifest should parse");
        assert!(!manifest.models.is_empty());
        assert_eq!(manifest.default_model_id, "smollm2-360m-instruct-q4_k_m");
        let default_model = manifest
            .models
            .iter()
            .find(|m| m.id == manifest.default_model_id)
            .expect("default model should exist");
        assert!(default_model.is_default);
        assert_eq!(default_model.filename, "SmolLM2-360M-Instruct-Q4_K_M.gguf");
        assert!(default_model.url.starts_with("https://huggingface.co/"));
    }

    #[test]
    fn test_compute_file_sha256() {
        let temp_dir = std::env::temp_dir().join(format!("share2cal_test_sha_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("test_file.txt");
        fs::write(&test_file, b"hello world\n").unwrap();

        let hash = compute_file_sha256(&test_file).unwrap();
        // SHA-256 of "hello world\n" is a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447
        assert_eq!(hash, "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447");
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_all_manifest_entries_have_valid_structure() {
        let manifest = get_manifest().unwrap();
        for model in manifest.models {
            assert!(!model.id.is_empty());
            assert!(!model.name.is_empty());
            assert!(model.filename.ends_with(".gguf"));
            assert!(model.size_bytes > 50_000_000);
            assert_eq!(model.sha256.len(), 64);
            assert!(model.url.starts_with("https://huggingface.co/"));
        }
    }
}
