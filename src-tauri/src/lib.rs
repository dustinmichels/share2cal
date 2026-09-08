pub mod calendar;
pub mod commands;
pub mod error;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod ffi;
pub mod inference;
pub mod model;
pub mod ocr;
pub mod parser;
pub mod share;

pub use error::AppError;
pub use calendar::CalendarInfo;
pub use commands::*;
pub use ocr::{extract_text_from_bytes, extract_text_from_path, OcrResult};
pub use parser::{
    generate_extraction_prompt, get_gbnf_grammar, get_json_schema, parse_event_deterministic,
    parse_events_deterministic, EventDetails, ReferenceContext,
};
pub use share::{
    clear_pending_shared_image as clear_shared, get_pending_shared_image as get_shared,
    load_image_from_path as load_image, stage_shared_image as stage_shared, SharedImagePayload,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(model::DownloadState::default())
        .manage(inference::InferenceEngineManager::default())
        .invoke_handler(tauri::generate_handler![
            extract_text_from_image,
            extract_text_from_image_bytes,
            parse_events_from_text,
            parse_event_from_text,
            extract_events_from_image,
            extract_event_from_image,
            extract_events_from_image_bytes,
            extract_event_from_image_bytes,
            get_event_schema,
            get_event_gbnf_grammar,
            generate_event_prompt,
            get_pending_shared_image,
            clear_pending_shared_image,
            stage_shared_image,
            get_available_calendars,
            create_calendar_event,
            load_image_from_path,
            create_calendar_events,
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
            open_models_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[doc(hidden)]
pub mod test_support {
    use super::*;

    pub fn extract_text_from_image(path: String) -> Result<OcrResult, AppError> {
        commands::extract_text_from_image(path)
    }

    pub fn extract_text_from_image_bytes(bytes: Vec<u8>) -> Result<OcrResult, AppError> {
        commands::extract_text_from_image_bytes(bytes)
    }

    pub fn stage_shared_image(
        bytes: Vec<u8>,
        file_name: String,
        mime_type: Option<String>,
    ) -> Result<SharedImagePayload, AppError> {
        commands::stage_shared_image(bytes, file_name, mime_type)
    }

    pub fn get_pending_shared_image(
        include_bytes: Option<bool>,
    ) -> Result<Option<SharedImagePayload>, AppError> {
        commands::get_pending_shared_image(include_bytes)
    }

    pub fn clear_pending_shared_image() -> Result<(), AppError> {
        commands::clear_pending_shared_image()
    }

    pub fn load_image_from_path(path: String) -> Result<SharedImagePayload, AppError> {
        commands::load_image_from_path(path)
    }

    pub fn get_available_calendars() -> Result<Vec<calendar::CalendarInfo>, AppError> {
        commands::get_available_calendars()
    }

    pub fn create_calendar_event(
        event: EventDetails,
        calendar_id: Option<String>,
        calendar_title: Option<String>,
        calendar_source_title: Option<String>,
    ) -> Result<String, AppError> {
        commands::create_calendar_event(event, calendar_id, calendar_title, calendar_source_title)
    }

    pub fn create_calendar_events(
        events: Vec<EventDetails>,
        calendar_id: Option<String>,
        calendar_title: Option<String>,
        calendar_source_title: Option<String>,
    ) -> Result<Vec<String>, AppError> {
        commands::create_calendar_events(events, calendar_id, calendar_title, calendar_source_title)
    }

    pub fn check_calendar_permission() -> Result<String, AppError> {
        commands::check_calendar_permission()
    }

    pub fn request_calendar_permission() -> Result<bool, AppError> {
        commands::request_calendar_permission()
    }

    pub async fn parse_events_internal(
        app: Option<&tauri::AppHandle>,
        text: &str,
        reference_time: Option<String>,
        timezone_offset_minutes: Option<i32>,
        model_id: Option<&str>,
        timeout_secs: Option<u64>,
        mode: Option<&str>,
    ) -> Vec<EventDetails> {
        commands::parse_events_internal(
            app,
            text,
            reference_time,
            timezone_offset_minutes,
            model_id,
            timeout_secs,
            mode,
        )
        .await
    }

    pub async fn parse_event_internal(
        app: Option<&tauri::AppHandle>,
        text: &str,
        reference_time: Option<String>,
        timezone_offset_minutes: Option<i32>,
        model_id: Option<&str>,
        timeout_secs: Option<u64>,
        mode: Option<&str>,
    ) -> EventDetails {
        commands::parse_event_internal(
            app,
            text,
            reference_time,
            timezone_offset_minutes,
            model_id,
            timeout_secs,
            mode,
        )
        .await
    }
}
