pub mod calendar;
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

#[tauri::command]
fn parse_event_from_text(
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
) -> Result<EventDetails, String> {
    let context = if reference_time.is_some() || timezone_offset_minutes.is_some() {
        ReferenceContext {
            reference_time,
            timezone_offset_minutes,
        }
    } else {
        ReferenceContext::now()
    };

    Ok(parse_event_deterministic(&text, &context))
}

#[tauri::command]
fn extract_event_from_image(
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
) -> Result<EventDetails, String> {
    let ocr_res = extract_text_from_path(&path)?;
    parse_event_from_text(ocr_res.text, reference_time, timezone_offset_minutes)
}

#[tauri::command]
fn extract_event_from_image_bytes(
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
) -> Result<EventDetails, String> {
    let ocr_res = extract_text_from_bytes(&bytes)?;
    parse_event_from_text(ocr_res.text, reference_time, timezone_offset_minutes)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            request_calendar_permission
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

    #[test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn test_extract_event_from_sample_image() {
        let sample = get_sample_path("gilman_flyer.png");
        let event = extract_event_from_image(
            sample.to_str().unwrap().to_string(),
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
        )
        .expect("Should extract event from image");

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
