pub mod calendar;
pub mod inference;
pub mod model;
pub mod ocr;
pub mod parser;
pub mod share;
use ocr::{extract_text_from_bytes, extract_text_from_path, OcrResult};
use parser::{
    generate_extraction_prompt, get_gbnf_grammar, get_json_schema, parse_event_deterministic,
    EventDetails, ReferenceContext,
};
use share::{
    clear_pending_shared_image as clear_shared,
    get_pending_shared_image as get_shared,
    stage_shared_image as stage_shared,
    SharedImagePayload,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn extract_text_from_image(path: String) -> Result<OcrResult, String> {
    extract_text_from_path(&path)
}

#[tauri::command]
fn extract_text_from_image_bytes(bytes: Vec<u8>) -> Result<OcrResult, String> {
    extract_text_from_bytes(&bytes)
}

pub async fn parse_event_internal(
    app: Option<&tauri::AppHandle>,
    text: &str,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<&str>,
    timeout_secs: Option<u64>,
) -> EventDetails {
    let context = if reference_time.is_some() || timezone_offset_minutes.is_some() {
        ReferenceContext {
            reference_time,
            timezone_offset_minutes,
        }
    } else {
        ReferenceContext::now()
    };

    if let Some(app_handle) = app {
        inference::extract_event_orchestrated(
            app_handle,
            text,
            &context,
            model_id,
            timeout_secs,
        ).await
    } else {
        parse_event_deterministic(text, &context)
    }
}

#[tauri::command]
async fn parse_event_from_text(
    app: tauri::AppHandle,
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<EventDetails, String> {
    Ok(parse_event_internal(
        Some(&app),
        &text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
    ).await)
}

#[tauri::command]
async fn extract_event_from_image(
    app: tauri::AppHandle,
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<EventDetails, String> {
    let ocr_res = extract_text_from_path(&path)?;
    Ok(parse_event_internal(
        Some(&app),
        &ocr_res.text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
    ).await)
}

#[tauri::command]
async fn extract_event_from_image_bytes(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<EventDetails, String> {
    let ocr_res = extract_text_from_bytes(&bytes)?;
    Ok(parse_event_internal(
        Some(&app),
        &ocr_res.text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
    ).await)
}
#[tauri::command]
async fn unload_inference_model() -> Result<(), String> {
    inference::InferenceEngineManager::global().unload_model().await;
    Ok(())
}

#[tauri::command]
async fn is_inference_model_loaded(model_id: String) -> Result<bool, String> {
    Ok(inference::InferenceEngineManager::global().is_model_loaded(&model_id).await)
}

#[tauri::command]
fn get_event_schema() -> serde_json::Value {
    get_json_schema()
}

#[tauri::command]
fn get_event_gbnf_grammar() -> String {
    get_gbnf_grammar().to_string()
}

#[tauri::command]
fn generate_event_prompt(
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
) -> String {
    let context = ReferenceContext {
        reference_time,
        timezone_offset_minutes,
    };
    generate_extraction_prompt(&text, &context)
}
#[tauri::command]
fn get_pending_shared_image(include_bytes: Option<bool>) -> Result<Option<SharedImagePayload>, String> {
    get_shared(include_bytes.unwrap_or(true))
}

#[tauri::command]
fn clear_pending_shared_image() -> Result<(), String> {
    clear_shared()
}

#[tauri::command]
fn stage_shared_image(
    bytes: Vec<u8>,
    file_name: String,
    mime_type: Option<String>,
) -> Result<SharedImagePayload, String> {
    stage_shared(&bytes, &file_name, mime_type.as_deref())
}

#[tauri::command]
fn create_calendar_event(event: EventDetails) -> Result<String, String> {
    calendar::create_event(&event)
}

#[tauri::command]
fn check_calendar_permission() -> Result<String, String> {
    calendar::check_permission()
}

#[tauri::command]
fn request_calendar_permission() -> Result<bool, String> {
    calendar::request_permission()
}

#[tauri::command]
fn get_model_manifest() -> Result<model::ModelManifest, String> {
    model::get_manifest()
}

#[tauri::command]
fn get_model_statuses(app: tauri::AppHandle) -> Result<Vec<model::ModelStatus>, String> {
    model::get_all_model_statuses(&app)
}

#[tauri::command]
fn get_model_status(app: tauri::AppHandle, model_id: String) -> Result<model::ModelStatus, String> {
    model::get_single_model_status(&app, &model_id)
}

#[tauri::command]
async fn download_model(app: tauri::AppHandle, model_id: String) -> Result<(), String> {
    model::start_model_download(app, model_id).await
}

#[tauri::command]
fn cancel_model_download(app: tauri::AppHandle, model_id: String) -> Result<(), String> {
    model::cancel_download(&app, &model_id)
}

#[tauri::command]
async fn delete_model(app: tauri::AppHandle, model_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || model::delete_model_file(&app, &model_id))
        .await
        .map_err(|e| format!("Delete task failed: {}", e))?
}

#[tauri::command]
async fn verify_model_hash(app: tauri::AppHandle, model_id: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || model::verify_model(&app, &model_id))
        .await
        .map_err(|e| format!("Verification task failed: {}", e))?
}

#[tauri::command]
fn get_models_storage_info(app: tauri::AppHandle) -> Result<model::ModelsStorageInfo, String> {
    model::get_models_storage_info(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(model::DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            extract_text_from_image,
            extract_text_from_image_bytes,
            parse_event_from_text,
            extract_event_from_image,
            extract_event_from_image_bytes,
            get_event_schema,
            get_event_gbnf_grammar,
            generate_event_prompt,
            get_pending_shared_image,
            clear_pending_shared_image,
            stage_shared_image,
            create_calendar_event,
            check_calendar_permission,
            request_calendar_permission,
            get_model_manifest,
            get_model_statuses,
            get_model_status,
            download_model,
            cancel_model_download,
            delete_model,
            verify_model_hash,
            get_models_storage_info,
            unload_inference_model,
            is_inference_model_loaded,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_sample_path(filename: &str) -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.parent().unwrap().join("samples").join(filename)
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_sample_image() {
        let sample = get_sample_path("gilman_flyer.png");
        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample flyer");
        let event = parse_event_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
        )
        .await;
        assert!(event.title.contains("GILMAN SQUARE") || event.title.contains("FESTIVAL"));
        assert!(event.start_time.is_some());
        assert!(event.start_time.unwrap().starts_with("2026-09-12T12:00:00"));
        assert!(event.end_time.is_some());
        assert!(event.end_time.unwrap().starts_with("2026-09-12T17:00:00"));
        assert!(event.location.is_some());
        assert!(event.description.is_some());
    }

    #[test]
    fn test_share_commands_flow() {
        let _guard = crate::share::TEST_SHARE_MUTEX.lock().unwrap();
        let sample = get_sample_path("gilman_flyer.png");
        if sample.exists() {
            let bytes = std::fs::read(&sample).expect("Should read sample file");
            let staged = stage_shared_image(bytes.clone(), "flyer_share_test.png".to_string(), Some("image/png".to_string()))
                .expect("Stage command should succeed");
            assert_eq!(staged.file_name, "flyer_share_test.png");

            let pending = get_pending_shared_image(Some(true))
                .expect("Get pending command should succeed");
            assert!(pending.is_some());
            let p = pending.unwrap();
            assert_eq!(p.file_name, "flyer_share_test.png");
            assert_eq!(p.bytes.unwrap(), bytes);

            clear_pending_shared_image().expect("Clear pending command should succeed");
            let empty = get_pending_shared_image(Some(false)).expect("Get pending should succeed");
            assert!(empty.is_none());
        }
    }
}
