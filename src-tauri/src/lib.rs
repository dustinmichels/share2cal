pub mod calendar;
pub mod inference;
pub mod model;
pub mod ocr;
pub mod parser;
pub mod share;
use ocr::{extract_text_from_bytes, extract_text_from_path, OcrResult};
use parser::{
    generate_extraction_prompt, get_gbnf_grammar, get_json_schema, parse_event_deterministic,
    parse_events_deterministic, EventDetails, ReferenceContext,
};
use share::{
    clear_pending_shared_image as clear_shared, get_pending_shared_image as get_shared,
    load_image_from_path as load_image, stage_shared_image as stage_shared, SharedImagePayload,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn extract_text_from_image(path: String) -> Result<OcrResult, String> {
    let mut ocr_res = extract_text_from_path(&path)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext::default();
    if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        ocr_res.text = spatial_text;
    }
    Ok(ocr_res)
}

#[tauri::command]
fn extract_text_from_image_bytes(bytes: Vec<u8>) -> Result<OcrResult, String> {
    let mut ocr_res = extract_text_from_bytes(&bytes)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext::default();
    if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        ocr_res.text = spatial_text;
    }
    Ok(ocr_res)
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
    let context = if reference_time.is_some() || timezone_offset_minutes.is_some() {
        ReferenceContext {
            reference_time,
            timezone_offset_minutes,
        }
    } else {
        ReferenceContext::now()
    };

    // If explicit mode is "simple", bypass LLM inference and use deterministic rule-based parser directly
    if let Some(m) = mode {
        if m.eq_ignore_ascii_case("simple") {
            return parser::parse_events_deterministic(text, &context);
        }
    }

    let schedule_events = parser::parse_schedule_table_events(text, &context);
    if schedule_events.len() >= 2 {
        return schedule_events;
    }

    if let Some(app_handle) = app {
        inference::extract_events_orchestrated(app_handle, text, &context, model_id, timeout_secs)
            .await
    } else {
        parse_events_deterministic(text, &context)
    }
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
    let events = parse_events_internal(
        app,
        text,
        reference_time.clone(),
        timezone_offset_minutes,
        model_id,
        timeout_secs,
        mode,
    )
    .await;
    events.into_iter().next().unwrap_or_else(|| {
        let context = ReferenceContext {
            reference_time,
            timezone_offset_minutes,
        };
        parse_event_deterministic(text, &context)
    })
}

#[tauri::command]
async fn parse_events_from_text(
    app: tauri::AppHandle,
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, String> {
    Ok(parse_events_internal(
        Some(&app),
        &text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
        mode.as_deref(),
    )
    .await)
}

#[tauri::command]
async fn parse_event_from_text(
    app: tauri::AppHandle,
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, String> {
    Ok(parse_event_internal(
        Some(&app),
        &text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
        mode.as_deref(),
    )
    .await)
}

#[tauri::command]
async fn extract_events_from_image(
    app: tauri::AppHandle,
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, String> {
    let ocr_res = extract_text_from_path(&path)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext {
        reference_time: reference_time.clone(),
        timezone_offset_minutes,
    };

    let effective_text = if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        &spatial_text
    } else {
        &ocr_res.text
    };

    Ok(parse_events_internal(
        Some(&app),
        effective_text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
        mode.as_deref(),
    )
    .await)
}

#[tauri::command]
async fn extract_event_from_image(
    app: tauri::AppHandle,
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, String> {
    let events = extract_events_from_image(
        app,
        path,
        reference_time,
        timezone_offset_minutes,
        model_id,
        timeout_secs,
        mode,
    )
    .await?;
    Ok(events.into_iter().next().unwrap_or_else(|| EventDetails {
        title: "New Event".to_string(),
        start_time: None,
        end_time: None,
        is_all_day: true,
        location: None,
        description: None,
        recurrence_rule: None,
        confidence: 0.5,
        source: "empty".to_string(),
    }))
}

#[tauri::command]
async fn extract_events_from_image_bytes(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, String> {
    let ocr_res = extract_text_from_bytes(&bytes)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext {
        reference_time: reference_time.clone(),
        timezone_offset_minutes,
    };

    let effective_text = if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        &spatial_text
    } else {
        &ocr_res.text
    };

    Ok(parse_events_internal(
        Some(&app),
        effective_text,
        reference_time,
        timezone_offset_minutes,
        model_id.as_deref(),
        timeout_secs,
        mode.as_deref(),
    )
    .await)
}

#[tauri::command]
async fn extract_event_from_image_bytes(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, String> {
    let events = extract_events_from_image_bytes(
        app,
        bytes,
        reference_time,
        timezone_offset_minutes,
        model_id,
        timeout_secs,
        mode,
    )
    .await?;
    Ok(events.into_iter().next().unwrap_or_else(|| EventDetails {
        title: "New Event".to_string(),
        start_time: None,
        end_time: None,
        is_all_day: true,
        location: None,
        description: None,
        recurrence_rule: None,
        confidence: 0.5,
        source: "empty".to_string(),
    }))
}

#[tauri::command]
async fn unload_inference_model() -> Result<(), String> {
    inference::InferenceEngineManager::global()
        .unload_model()
        .await;
    Ok(())
}

#[tauri::command]
async fn is_inference_model_loaded(model_id: String) -> Result<bool, String> {
    Ok(inference::InferenceEngineManager::global()
        .is_model_loaded(&model_id)
        .await)
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
fn get_pending_shared_image(
    include_bytes: Option<bool>,
) -> Result<Option<SharedImagePayload>, String> {
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
fn load_image_from_path(path: String) -> Result<SharedImagePayload, String> {
    load_image(&path)
}

#[tauri::command]
fn get_available_calendars() -> Result<Vec<calendar::CalendarInfo>, String> {
    calendar::list_calendars()
}

#[tauri::command]
fn create_calendar_event(
    event: EventDetails,
    calendar_id: Option<String>,
    calendar_title: Option<String>,
    calendar_source_title: Option<String>,
) -> Result<String, String> {
    calendar::create_event(
        &event,
        calendar_id.as_deref(),
        calendar_title.as_deref(),
        calendar_source_title.as_deref(),
    )
}

#[tauri::command]
fn create_calendar_events(
    events: Vec<EventDetails>,
    calendar_id: Option<String>,
    calendar_title: Option<String>,
    calendar_source_title: Option<String>,
) -> Result<Vec<String>, String> {
    calendar::create_events(
        &events,
        calendar_id.as_deref(),
        calendar_title.as_deref(),
        calendar_source_title.as_deref(),
    )
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
#[tauri::command]
fn open_models_directory(app: tauri::AppHandle) -> Result<(), String> {
    model::open_models_directory(&app)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_sample_path(filename: &str) -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .unwrap()
            .join("samples")
            .join(filename)
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_sample_image() {
        let sample = get_sample_path("gilman_flyer.png");
        assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample flyer");
        assert!(!ocr_res.text.is_empty(), "OCR text should not be empty");
        let event = parse_event_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;
        assert!(
            event.title.contains("GILMAN SQUARE") && event.title.contains("ARTS & MUSIC FESTIVAL"),
            "Title should contain event name and subtitle, got: {}",
            event.title
        );

        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-12T12:00:00-04:00"),
            "Start time must match Saturday Sep 12 2026 at 12:00 PM EDT"
        );

        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-12T17:00:00-04:00"),
            "End time must match Saturday Sep 12 2026 at 5:00 PM EDT"
        );

        assert!(
            !event.is_all_day,
            "Flyer with 12-5pm hours is not an all-day event"
        );

        assert!(event.location.is_some(), "Location should be extracted");
        let loc = event.location.as_deref().unwrap();
        assert!(
            loc.contains("Ed Leathers Park"),
            "Location must contain Ed Leathers Park, got: {}",
            loc
        );
        assert!(
            loc.contains("Walnut Street"),
            "Location must contain Walnut Street, got: {}",
            loc
        );
        assert!(
            loc.contains("Skilton Ave"),
            "Location must contain Skilton Ave, got: {}",
            loc
        );

        assert!(
            event.description.is_some(),
            "Description should be extracted"
        );
        let desc = event.description.as_deref().unwrap();
        assert!(
            desc.contains("Rain Date") && desc.contains("09/13/26"),
            "Description must contain rain date, got: {}",
            desc
        );
        assert!(
            desc.contains("LIVE MUSIC") && desc.contains("PERFORMANCES"),
            "Description must contain live music & performances, got: {}",
            desc
        );
        assert!(
            desc.contains("BEER GARDEN"),
            "Description must contain beer garden, got: {}",
            desc
        );
        assert!(
            desc.contains("FOOD VENDORS"),
            "Description must contain food vendors, got: {}",
            desc
        );
        assert!(
            desc.contains("ARTISTS&MAKERS"),
            "Description must contain artists & makers, got: {}",
            desc
        );
        assert!(
            desc.contains("KIDS ACTIVITIES"),
            "Description must contain kids activities, got: {}",
            desc
        );

        assert!(
            event.confidence >= 0.9,
            "Confidence score should be high, got: {}",
            event.confidence
        );
    }
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_instagram_sample_image() {
        let sample = get_sample_path("instagram.png");
        assert!(sample.exists(), "Sample instagram {:?} must exist", sample);

        let bytes = std::fs::read(&sample).expect("Should read sample instagram image bytes");
        let ocr_res = extract_text_from_image_bytes(bytes)
            .expect("Should run OCR on sample instagram image bytes");
        assert!(!ocr_res.text.is_empty(), "OCR text should not be empty");

        let event = parse_event_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            Some("simple"),
        )
        .await;

        assert_eq!(
            event.title, "UEP ICE CREAM SOCIAL",
            "Title must match UEP ICE CREAM SOCIAL"
        );
        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-09T12:00:00-04:00"),
            "Start time must match September 9, 2026 at 12:00 PM EDT"
        );

        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-09T13:00:00-04:00"),
            "End time must match September 9, 2026 at 1:00 PM EDT"
        );

        assert!(
            !event.is_all_day,
            "Instagram event with 12:00 PM - 1:00 PM is not an all-day event"
        );

        assert_eq!(
            event.location.as_deref(),
            Some("BP LAWN"),
            "Location must match BP LAWN"
        );

        assert_eq!(
            event.description.as_deref(),
            Some("We invite you to have dessert with us. Don't like ice cream? We have iced coffee, iced tea, and fruit too!"),
            "Description must match the complete user-supplied details sentence"
        );
        assert!(
            event.confidence >= 0.9,
            "Confidence score should be high, got: {}",
            event.confidence
        );
    }
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_squirrel_flower_sample_image() {
        let sample = get_sample_path("squirrel_flower.jpg");
        assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample image");
        assert!(!ocr_res.text.is_empty(), "OCR text should not be empty");

        let event = parse_event_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;

        assert!(
            event.title.to_lowercase().contains("flower"),
            "Title should contain artist name, got: {}",
            event.title
        );
        assert!(
            event.title.contains("(with") || event.title.contains("youbef") || event.title.contains("you bet"),
            "Title should contain supporting act, got: {}",
            event.title
        );

        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-26"),
            "Start time must match Saturday Sep 26 2026"
        );

        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-26"),
            "End time must match Saturday Sep 26 2026"
        );

        assert!(
            event.is_all_day,
            "Tour poster with date only is an all-day event"
        );

        assert!(event.location.is_some(), "Location should be extracted");
        let loc = event.location.as_deref().unwrap();
        assert!(
            loc.contains("CRYSTAL BALLROOM"),
            "Location must contain Crystal Ballroom, got: {}",
            loc
        );
        assert!(
            loc.contains("SOMERVILLE, MA") || loc.contains("SOMERVILLE"),
            "Location must contain Somerville, MA, got: {}",
            loc
        );

        assert!(
            event.description.is_some(),
            "Description should be extracted"
        );
        let desc = event.description.as_deref().unwrap();
        assert!(
            desc.contains("2026 TOUR") || desc.contains("TOUR"),
            "Description must contain tour info, got: {}",
            desc
        );

        assert!(
            event.confidence >= 0.8,
            "Confidence score should be high, got: {}",
            event.confidence
        );
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_events_plural_from_squirrel_flower_flyer() {
        let sample = get_sample_path("squirrel_flower.jpg");
        assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample flyer");
        let events = parse_events_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;

        assert_eq!(events.len(), 1, "Expected single event from tour poster");
        assert!(
            events[0].title.to_lowercase().contains("flower"),
            "Title should contain artist name"
        );
        assert_eq!(events[0].start_time.as_deref(), Some("2026-09-26"));
        assert!(events[0].is_all_day);
        assert!(events[0].location.as_deref().unwrap().contains("CRYSTAL BALLROOM"));
    }


    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_events_plural_from_sample_flyer() {
        let sample = get_sample_path("gilman_flyer.png");
        assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample flyer");
        let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
        let context = ReferenceContext {
            reference_time: Some("2026-09-06T12:00:00-04:00".to_string()),
            timezone_offset_minutes: Some(-240),
        };
        let effective_text = if !spatial_text.trim().is_empty()
            && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
        {
            &spatial_text
        } else {
            &ocr_res.text
        };
        let events = parse_events_internal(
            None,
            effective_text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;
        assert!(
            events[0].title.contains("GILMAN SQUARE")
                && events[0].title.contains("ARTS & MUSIC FESTIVAL"),
            "Title should contain event name and subtitle, got: {}",
            events[0].title
        );
        assert_eq!(
            events[0].start_time.as_deref(),
            Some("2026-09-12T12:00:00-04:00")
        );
        assert_eq!(
            events[0].end_time.as_deref(),
            Some("2026-09-12T17:00:00-04:00")
        );
        assert!(!events[0].is_all_day);
        let loc = events[0].location.as_deref().unwrap();
        assert!(
            loc.contains("Ed Leathers Park")
                && loc.contains("Walnut Street")
                && loc.contains("Skilton Ave")
        );
        let desc = events[0].description.as_deref().unwrap();
        assert!(
            desc.contains("Rain Date")
                && desc.contains("LIVE MUSIC")
                && desc.contains("BEER GARDEN")
                && desc.contains("FOOD VENDORS")
                && desc.contains("KIDS ACTIVITIES")
        );
    }
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_events_from_classes_sample_image() {
        let sample = get_sample_path("classes.png");
        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample classes.png");
        let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
        let context = ReferenceContext {
            reference_time: Some("2026-09-06T12:00:00-04:00".to_string()),
            timezone_offset_minutes: Some(-240),
        };
        let effective_text = if !spatial_text.trim().is_empty()
            && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
        {
            &spatial_text
        } else {
            &ocr_res.text
        };
        let events = parse_events_internal(
            None,
            effective_text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;
        assert_eq!(
            events.len(),
            6,
            "Expected exactly 6 parsed events from classes.png"
        );

        // Event 1: CEE 0154-03 (80513)
        assert_eq!(
            events[0].title,
            "CEE 0154-03 (80513) Principles Epidemiology (Lecture)"
        );
        assert_eq!(
            events[0].start_time.as_deref(),
            Some("2026-09-07T15:00:00-04:00")
        );
        assert_eq!(
            events[0].end_time.as_deref(),
            Some("2026-09-07T16:15:00-04:00")
        );
        assert_eq!(
            events[0].location.as_deref(),
            Some("Anderson Wing TTC, Room 306")
        );
        assert_eq!(
            events[0].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=MO,WE")
        );
        assert_eq!(
            events[0].description.as_deref(),
            Some("Days: Mo, We | Faculty: L. Abrams | Units: 3.00")
        );

        // Event 2: CS 0150-09 (84779)
        assert_eq!(
            events[1].title,
            "CS 0150-09 (84779) Special Topics - Analysis Mthds Images, Text & (Lecture)"
        );
        assert_eq!(
            events[1].start_time.as_deref(),
            Some("2026-09-11T14:00:00-04:00")
        );
        assert_eq!(
            events[1].end_time.as_deref(),
            Some("2026-09-11T16:30:00-04:00")
        );
        assert_eq!(events[1].location.as_deref(), Some("Online"));
        assert_eq!(
            events[1].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=FR")
        );
        assert_eq!(
            events[1].description.as_deref(),
            Some("Days: Fr | Faculty: J. Skripchuk | Units: 3.00")
        );

        // Event 3: CSHD 0166-01 (82454)
        assert_eq!(
            events[2].title,
            "CSHD 0166-01 (82454) Children's Play (Lecture)"
        );
        assert_eq!(
            events[2].start_time.as_deref(),
            Some("2026-09-10T13:30:00-04:00")
        );
        assert_eq!(
            events[2].end_time.as_deref(),
            Some("2026-09-10T16:00:00-04:00")
        );
        assert_eq!(
            events[2].location.as_deref(),
            Some("Eliot-Pearson, Room 157")
        );
        assert_eq!(
            events[2].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=TH")
        );
        assert_eq!(
            events[2].description.as_deref(),
            Some("Days: Th | Faculty: W. Scarlett | Units: 3.00")
        );

        // Event 4: CSHD 0167-01 (80739)
        assert_eq!(
            events[3].title,
            "CSHD 0167-01 (80739) Children & Media (Lecture)"
        );
        assert_eq!(
            events[3].start_time.as_deref(),
            Some("2026-09-11T09:00:00-04:00")
        );
        assert_eq!(
            events[3].end_time.as_deref(),
            Some("2026-09-11T11:30:00-04:00")
        );
        assert_eq!(events[3].location.as_deref(), Some("Eaton Hall, 201"));
        assert_eq!(
            events[3].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=FR")
        );
        assert_eq!(
            events[3].description.as_deref(),
            Some("Days: Fr | Faculty: J. Dobrow | Units: 3.00")
        );

        // Event 5: UEP 0254-01 (81300)
        assert_eq!(
            events[4].title,
            "UEP 0254-01 (81300) Quantitative Reasoning (Lecture)"
        );
        assert_eq!(
            events[4].start_time.as_deref(),
            Some("2026-09-08T09:00:00-04:00")
        );
        assert_eq!(
            events[4].end_time.as_deref(),
            Some("2026-09-08T10:15:00-04:00")
        );
        assert_eq!(
            events[4].location.as_deref(),
            Some("Joyce Cummings Center, 302")
        );
        assert_eq!(
            events[4].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=TU,TH")
        );
        assert_eq!(
            events[4].description.as_deref(),
            Some("Days: Tu, Th | Faculty: S. Shamsuddin | Units: 3.00")
        );

        // Event 6: UEP 0262-01 (82571)
        assert_eq!(
            events[5].title,
            "UEP 0262-01 (82571) Solidarity Economy Movements (Seminar)"
        );
        assert_eq!(
            events[5].start_time.as_deref(),
            Some("2026-09-08T12:00:00-04:00")
        );
        assert_eq!(
            events[5].end_time.as_deref(),
            Some("2026-09-08T14:30:00-04:00")
        );
        assert_eq!(
            events[5].location.as_deref(),
            Some("Bromfield-Pearson, Room 006")
        );
        assert_eq!(
            events[5].recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=TU")
        );
        assert_eq!(
            events[5].description.as_deref(),
            Some("Days: Tu | Faculty: P. Loh | Units: 3.00")
        );
    }
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_text_and_parse_events_from_classes_image_bytes() {
        let sample = get_sample_path("classes.png");
        let bytes = std::fs::read(&sample).expect("Should read classes.png sample file");
        let ocr_res = extract_text_from_image_bytes(bytes).expect("Should run OCR on sample bytes");

        // ocr_res.text should be spatially reconstructed row-by-row
        assert!(
            ocr_res.text.contains("CEE 0154-03") && ocr_res.text.contains("Principles Epidemiology"),
            "OCR text should contain course code and description in reconstructed rows"
        );
        let events = parse_events_internal(
            None,
            &ocr_res.text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            None,
        )
        .await;
        assert_eq!(events.len(), 6, "Expected exactly 6 parsed events");
        assert_eq!(events[0].title, "CEE 0154-03 (80513) Principles Epidemiology (Lecture)");
        assert_eq!(events[0].start_time.as_deref(), Some("2026-09-07T15:00:00-04:00"));
        assert_eq!(events[0].end_time.as_deref(), Some("2026-09-07T16:15:00-04:00"));
        assert_eq!(events[0].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));

        assert_eq!(events[5].title, "UEP 0262-01 (82571) Solidarity Economy Movements (Seminar)");
        assert_eq!(events[5].start_time.as_deref(), Some("2026-09-08T12:00:00-04:00"));
        assert_eq!(events[5].end_time.as_deref(), Some("2026-09-08T14:30:00-04:00"));
        assert_eq!(events[5].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TU"));
    }

    #[test]
    fn test_share_commands_flow() {
        let _guard = crate::share::TEST_SHARE_MUTEX.lock().unwrap();
        let sample = get_sample_path("gilman_flyer.png");
        if sample.exists() {
            let bytes = std::fs::read(&sample).expect("Should read sample file");
            let staged = stage_shared_image(
                bytes.clone(),
                "flyer_share_test.png".to_string(),
                Some("image/png".to_string()),
            )
            .expect("Stage command should succeed");
            assert_eq!(staged.file_name, "flyer_share_test.png");

            let pending =
                get_pending_shared_image(Some(true)).expect("Get pending command should succeed");
            assert!(pending.is_some());
            let p = pending.unwrap();
            assert_eq!(p.file_name, "flyer_share_test.png");
            assert_eq!(p.bytes.unwrap(), bytes);

            clear_pending_shared_image().expect("Clear pending command should succeed");
            let empty = get_pending_shared_image(Some(false)).expect("Get pending should succeed");
            assert!(empty.is_none());
        }
    }

    #[tokio::test]
    async fn test_parse_events_simple_mode_forces_deterministic() {
        let text = "Team Standup Meeting\nFriday, Sep 18, 2026\n10:00 AM - 11:00 AM\nRoom 4B";
        let events = parse_events_internal(
            None,
            text,
            Some("2026-09-06T12:00:00-04:00".to_string()),
            Some(-240),
            None,
            None,
            Some("simple"),
        )
        .await;

        assert_eq!(events.len(), 1);
        assert!(events[0].title.contains("Team Standup Meeting") || events[0].title.contains("Standup"));
        assert_eq!(events[0].start_time.as_deref(), Some("2026-09-18T10:00:00-04:00"));
        assert_eq!(events[0].end_time.as_deref(), Some("2026-09-18T11:00:00-04:00"));
        assert_eq!(events[0].source, "deterministic");
    }
}
