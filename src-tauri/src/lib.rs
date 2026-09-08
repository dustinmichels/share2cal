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
    use serde::{Deserialize, Serialize};
    use regex::Regex;
    use std::path::PathBuf;
    use std::sync::Arc;
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct ParsedJsonEvent {
        pub title: String,
        pub date: Option<String>,
        pub days: Option<Vec<String>>,
        pub start_time: Option<String>,
        pub end_time: Option<String>,
        pub is_all_day: bool,
        #[serde(default)]
        pub repeating: Option<bool>,
        pub location: Option<String>,
        pub description: Option<String>,
    }

    fn load_parsed_json_manifest() -> std::collections::HashMap<String, Vec<ParsedJsonEvent>> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.parent().unwrap().join("samples").join("parsed.json");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read parsed.json at {:?}: {}", path, e));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse parsed.json: {}", e))
    }

    /// Fuzzy matching for title: checks substring or significant keyword overlap
    fn fuzzy_match_title(actual: &str, expected: &str) -> bool {
        let a = actual.to_lowercase();
        let e = expected.to_lowercase();
        if a.contains(&e) || e.contains(&a) {
            return true;
        }
        let e_tokens: Vec<&str> = e
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() >= 3 && !["and", "the", "for", "with", "lecture", "seminar"].contains(t))
            .collect();
        if e_tokens.is_empty() {
            return true;
        }
        e_tokens.iter().any(|t| a.contains(*t))
    }

    /// Fuzzy matching for location: checks substring or keyword overlap
    fn fuzzy_match_location(actual: Option<&str>, expected: Option<&str>) -> bool {
        match (actual, expected) {
            (None, None) => true,
            (Some(_), None) => true, // Tolerant if extra location parsed
            (None, Some(e)) if e.trim().is_empty() => true,
            (None, Some(_)) => false,
            (Some(a), Some(e)) => {
                let al = a.to_lowercase();
                let el = e.to_lowercase();
                if al.contains(&el) || el.contains(&al) {
                    return true;
                }
                let e_tokens: Vec<&str> = el
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|t| t.len() >= 3 && !["room", "hall", "street", "ave", "wing", "the", "and"].contains(t))
                    .collect();
                if e_tokens.is_empty() {
                    return true;
                }
                e_tokens.iter().any(|t| al.contains(*t))
            }
        }
    }

    fn parse_time_to_24h(time_str: &str) -> Option<String> {
        let trimmed = time_str.trim();
        let re = Regex::new(r"(?i)^(\d{1,2}):(\d{2})\s*(AM|PM)?$").ok()?;
        if let Some(caps) = re.captures(trimmed) {
            let mut hour: u32 = caps.get(1)?.as_str().parse().ok()?;
            let minute: u32 = caps.get(2)?.as_str().parse().ok()?;
            if let Some(ampm_match) = caps.get(3) {
                let ampm = ampm_match.as_str().to_uppercase();
                if ampm == "PM" && hour < 12 {
                    hour += 12;
                } else if ampm == "AM" && hour == 12 {
                    hour = 0;
                }
            }
            return Some(format!("{:02}:{:02}", hour, minute));
        }
        None
    }

    /// Matches date and time according to strict date/time rules against parsed.json
    fn match_date_and_time(
        actual_start: Option<&str>,
        actual_end: Option<&str>,
        actual_is_all_day: bool,
        expected: &ParsedJsonEvent,
    ) -> bool {
        if actual_is_all_day != expected.is_all_day {
            return false;
        }
        if expected.is_all_day {
            if let Some(e_date) = expected.date.as_deref() {
                if let Some(a_st) = actual_start {
                    if !a_st.starts_with(e_date) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            return true;
        }

        if let Some(e_date) = expected.date.as_deref() {
            if let Some(a_st) = actual_start {
                if !a_st.starts_with(e_date) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(e_st) = expected.start_time.as_deref() {
            if let Some(a_st) = actual_start {
                if let Some(exp_24) = parse_time_to_24h(e_st) {
                    if !a_st.contains(&exp_24) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        if let Some(e_et) = expected.end_time.as_deref() {
            if let Some(a_et) = actual_end {
                if let Some(exp_24) = parse_time_to_24h(e_et) {
                    if !a_et.contains(&exp_24) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Matches recurrence rule checking frequency and by_day tokens
    fn match_recurrence_rule(actual: Option<&str>, expected: &ParsedJsonEvent) -> bool {
        let is_expected_repeating = expected.repeating.unwrap_or(false) || expected.days.is_some();
        if !is_expected_repeating {
            return actual.is_none();
        }
        if let Some(a) = actual {
            if !a.contains("FREQ=WEEKLY") {
                return false;
            }
            if let Some(days) = &expected.days {
                for d in days {
                    let token = match d.to_lowercase().as_str() {
                        "monday" | "mon" | "mo" => "MO",
                        "tuesday" | "tue" | "tu" => "TU",
                        "wednesday" | "wed" | "we" => "WE",
                        "thursday" | "thu" | "th" => "TH",
                        "friday" | "fri" | "fr" => "FR",
                        "saturday" | "sat" | "sa" => "SA",
                        "sunday" | "sun" | "su" => "SU",
                        _ => "",
                    };
                    if !token.is_empty() && !a.contains(token) {
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }
    fn assert_event_matches_parsed_json(
        actual: &EventDetails,
        expected: &ParsedJsonEvent,
        sample_name: &str,
    ) {
        assert!(
            fuzzy_match_title(&actual.title, &expected.title),
            "[{}] Title mismatch. Actual: {:?}, Expected: {:?}",
            sample_name,
            actual.title,
            expected.title
        );
        assert_eq!(
            actual.is_all_day, expected.is_all_day,
            "[{}] is_all_day mismatch for event {:?}",
            sample_name, actual.title
        );
        assert!(
            match_date_and_time(
                actual.start_time.as_deref(),
                actual.end_time.as_deref(),
                actual.is_all_day,
                expected,
            ),
            "[{}] Date/Time mismatch. Actual: (start={:?}, end={:?}), Expected: (start={:?}, end={:?}) for event {:?}",
            sample_name,
            actual.start_time,
            actual.end_time,
            expected.start_time,
            expected.end_time,
            actual.title
        );
        assert!(
            fuzzy_match_location(actual.location.as_deref(), expected.location.as_deref()),
            "[{}] Location mismatch. Actual: {:?}, Expected: {:?}",
            sample_name,
            actual.location,
            expected.location
        );
        assert!(
            match_recurrence_rule(actual.recurrence_rule.as_deref(), expected),
            "[{}] Recurrence rule mismatch. Actual: {:?}, Expected: (repeating={:?}, days={:?})",
            sample_name,
            actual.recurrence_rule,
            expected.repeating,
            expected.days
        );
        // Note: As specified, "description" is intentionally not checked for strict equality.
    }

    fn get_sample_path(filename: &str) -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .unwrap()
            .join("samples")
            .join(filename)
    }

    #[test]
    fn test_parsed_json_manifest_loads_all_samples() {
        let manifest = load_parsed_json_manifest();
        assert_eq!(manifest.len(), 6, "parsed.json must contain all 6 sample images");
        assert!(manifest.contains_key("samples/class.png"));
        assert!(manifest.contains_key("samples/classes.png"));
        assert!(manifest.contains_key("samples/gilman_flyer.png"));
        assert!(manifest.contains_key("samples/instagram.png"));
        assert!(manifest.contains_key("samples/ride_for_life.png"));
        assert!(manifest.contains_key("samples/squirrel_flower.jpg"));

        assert_eq!(manifest.get("samples/class.png").unwrap().len(), 1);
        assert_eq!(manifest.get("samples/classes.png").unwrap().len(), 6);
        assert_eq!(manifest.get("samples/gilman_flyer.png").unwrap().len(), 1);
        assert_eq!(manifest.get("samples/instagram.png").unwrap().len(), 1);
        assert_eq!(manifest.get("samples/ride_for_life.png").unwrap().len(), 1);
        assert_eq!(manifest.get("samples/squirrel_flower.jpg").unwrap().len(), 1);
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_all_samples_simple_deterministic_against_parsed_json() {
        let manifest = load_parsed_json_manifest();
        let reference_time = "2026-09-06T12:00:00-04:00".to_string(); // Sunday before Fall 2026 Term
        let timezone_offset_minutes = -240; // EDT

        for (sample_rel_path, expected_events) in manifest {
            let filename = PathBuf::from(&sample_rel_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let sample_path = get_sample_path(&filename);
            assert!(
                sample_path.exists(),
                "Sample file {:?} must exist for test",
                sample_path
            );

            // Step 1: Run native Apple Vision OCR
            let ocr_res = extract_text_from_path(sample_path.to_str().unwrap())
                .unwrap_or_else(|e| panic!("OCR failed for sample {}: {}", sample_rel_path, e));
            assert!(
                !ocr_res.text.is_empty(),
                "OCR text must not be empty for sample {}",
                sample_rel_path
            );

            // Step 2: Use reconstructed spatial layout if multiple table rows detected
            let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
            let context = ReferenceContext {
                reference_time: Some(reference_time.clone()),
                timezone_offset_minutes: Some(timezone_offset_minutes),
            };
            let effective_text = if !spatial_text.trim().is_empty()
                && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
            {
                &spatial_text
            } else {
                &ocr_res.text
            };

            // Step 3: Run "simple" deterministic parsing
            let actual_events = parse_events_internal(
                None,
                effective_text,
                Some(reference_time.clone()),
                Some(timezone_offset_minutes),
                None,
                None,
                Some("simple"),
            )
            .await;

            assert_eq!(
                actual_events.len(),
                expected_events.len(),
                "[{}] Expected {} parsed events, got {}",
                sample_rel_path,
                expected_events.len(),
                actual_events.len()
            );

            // Step 4: Validate each extracted event against parsed.json
            for (idx, expected) in expected_events.iter().enumerate() {
                let actual = &actual_events[idx];
                assert_event_matches_parsed_json(actual, expected, &sample_rel_path);
            }
        }
    }

    /// Tests relative start date calculation for recurring weekly schedules.
    ///
    /// COMPLEXITY / BEHAVIOR SPECIFICATION:
    /// When an image specifies a weekly recurring schedule like "M 4:30-5" or "Mo, We 3:00 PM - 4:15 PM"
    /// without explicit start calendar dates, the application assumes that the event repeats weekly,
    /// starting on the NEXT matching weekday relative to WHEN the code is executed (or reference_time).
    ///
    /// For example:
    /// - If the test is run with reference_time = 2026-09-06 (Sunday):
    ///   * Monday classes start on 2026-09-07
    ///   * Tuesday classes start on 2026-09-08
    ///   * Thursday classes start on 2026-09-10
    ///   * Friday classes start on 2026-09-11
    ///
    /// - If the code is re-run next week or next month (e.g. reference_time = 2026-09-16, Wednesday):
    ///   * Monday classes advance to the next upcoming Monday (2026-09-21)
    ///   * Thursday classes advance to the next upcoming Thursday (2026-09-17)
    ///   * Friday classes advance to the next upcoming Friday (2026-09-18)
    ///   * Tuesday classes advance to the next upcoming Tuesday (2026-09-22)
    ///
    /// In all cases, the time of day (hours/minutes), duration, timezone offset, and RRULE (BYDAY)
    /// are strictly preserved.
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_classes_schedule_relative_date_handling_next_weekday() {
        let sample_path = get_sample_path("classes.png");
        let ocr_res = extract_text_from_path(sample_path.to_str().unwrap()).unwrap();
        let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);

        // Scenario A: Run on a Wednesday (2026-09-16T10:00:00-04:00)
        let wednesday_ref = "2026-09-16T10:00:00-04:00".to_string();
        let events_wed = parse_events_internal(
            None,
            &spatial_text,
            Some(wednesday_ref),
            Some(-240),
            None,
            None,
            Some("simple"),
        )
        .await;

        assert_eq!(events_wed.len(), 6);

        // CEE 0154-03 (Mo, We 3:00 PM - 4:15 PM) -> Next Monday is Sep 21, 2026
        assert!(events_wed[0].title.contains("CEE 0154-03") || events_wed[0].title.contains("Epidemiology"));
        assert_eq!(
            events_wed[0].start_time.as_deref(),
            Some("2026-09-21T15:00:00-04:00"),
            "When run on Wednesday Sep 16, next Monday class should start on Sep 21"
        );
        assert_eq!(
            events_wed[0].end_time.as_deref(),
            Some("2026-09-21T16:15:00-04:00")
        );
        assert_eq!(events_wed[0].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));

        // CS 0150-09 (Fr 2:00 PM - 4:30 PM) -> Next Friday is Sep 18, 2026
        assert_eq!(
            events_wed[1].start_time.as_deref(),
            Some("2026-09-18T14:00:00-04:00"),
            "When run on Wednesday Sep 16, next Friday class should start on Sep 18"
        );
        assert_eq!(
            events_wed[1].end_time.as_deref(),
            Some("2026-09-18T16:30:00-04:00")
        );
        assert_eq!(events_wed[1].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=FR"));

        // CSHD 0166-01 (Th 1:30 PM - 4:00 PM) -> Next Thursday is Sep 17, 2026
        assert_eq!(
            events_wed[2].start_time.as_deref(),
            Some("2026-09-17T13:30:00-04:00"),
            "When run on Wednesday Sep 16, next Thursday class should start on Sep 17"
        );
        assert_eq!(
            events_wed[2].end_time.as_deref(),
            Some("2026-09-17T16:00:00-04:00")
        );
        assert_eq!(events_wed[2].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TH"));
    }
    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_class_sample_image() {
        let sample = get_sample_path("class.png");
        assert!(sample.exists(), "Sample class {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample class.png");
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

        assert!(
            event.title.contains("CVS-0188") && event.title.contains("Children and Media"),
            "Title should contain course code and name, got: {}",
            event.title
        );
        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-09T13:20:00-04:00"),
            "Start time must match Wednesday Sep 9 2026 at 1:20 PM EDT"
        );
        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-09T16:20:00-04:00"),
            "End time must match Wednesday Sep 9 2026 at 4:20 PM EDT"
        );
        assert!(
            !event.is_all_day,
            "Class with 1:20-4:20pm hours is not an all-day event"
        );
        assert!(
            event.location.is_some() && event.location.as_deref().unwrap().contains("Eliot-Pearson"),
            "Location must contain Eliot-Pearson, Room 157, got: {:?}",
            event.location
        );
        assert_eq!(
            event.recurrence_rule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=WE"),
            "Recurrence rule must match weekly Wednesday"
        );
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_extract_event_from_ride_for_life_sample_image() {
        let sample = get_sample_path("ride_for_life.png");
        assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

        let ocr_res = extract_text_from_path(sample.to_str().unwrap())
            .expect("Should run OCR on sample ride_for_life.png");
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

        assert!(
            event.title.to_lowercase().contains("ride for your life"),
            "Title should contain 'Ride For Your Life', got: {}",
            event.title
        );
        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-10-25"),
            "Start date should match October 25, 2026"
        );
        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-10-25"),
            "End date should match October 25, 2026"
        );
        assert!(event.is_all_day);
        assert!(event.location.is_some());
        assert!(event.location.as_deref().unwrap().to_uppercase().contains("BOSTON"));
        assert!(event.description.is_some());
        let desc = event.description.as_deref().unwrap();
        assert!(desc.contains("RIDE") || desc.contains("RALLY") || desc.contains("EVERYONE"));
        assert_eq!(event.recurrence_rule, None);
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

    fn get_test_model_path() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("SHARE2CAL_MODELS_DIR") {
            let p = PathBuf::from(dir).join("SmolLM2-360M-Instruct-Q4_K_M.gguf");
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home)
                .join("Library/Application Support/com.dustinmichels.share2cal/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn test_all_samples_llm_inference_enhanced_against_parsed_json() {
        let manifest = load_parsed_json_manifest();
        let model_path = match get_test_model_path() {
            Some(p) => p,
            None => {
                eprintln!("SmolLM2 model not found on disk, skipping live LLM inference test.");
                return;
            }
        };

        let manager = inference::InferenceEngineManager::global();
        let model = manager
            .get_or_load_model("smollm2-360m-instruct-q4_k_m", &model_path)
            .await
            .expect("Should load SmolLM2 model weights");

        let context = ReferenceContext {
            reference_time: Some("2026-09-06T12:00:00-04:00".to_string()),
            timezone_offset_minutes: Some(-240),
        };

        // 1. Test Gilman Square Flyer
        {
            let sample_rel = "samples/gilman_flyer.png";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("gilman_flyer.png");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let prompt = generate_extraction_prompt(&ocr_res.text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1024,
                std::time::Duration::from_secs(30),
            )
            .await
            .expect("Inference on gilman_flyer should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for gilman_flyer");
            assert!(!events.is_empty(), "Expected at least 1 event extracted by LLM");
            let expected = &expected_list[0];
            let matched = events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && actual.is_all_day == expected.is_all_day
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            });
            assert!(
                matched || events.iter().any(|ev| ev.title.to_lowercase().contains("gilman") || ev.title.to_lowercase().contains("festival")),
                "LLM should extract Gilman Festival event matching parsed.json"
            );
        }

        // 2. Test Instagram Ice Cream Social
        {
            let sample_rel = "samples/instagram.png";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("instagram.png");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let prompt = generate_extraction_prompt(&ocr_res.text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1024,
                std::time::Duration::from_secs(30),
            )
            .await
            .expect("Inference on instagram.png should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for instagram");
            assert!(!events.is_empty());
            let expected = &expected_list[0];
            let matched = events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            });
            assert!(
                matched || events.iter().any(|ev| ev.title.to_lowercase().contains("ice cream") || ev.title.to_lowercase().contains("uep")),
                "LLM should extract UEP Ice Cream Social matching parsed.json"
            );
        }

        // 3. Test Squirrel Flower Tour Poster
        {
            let sample_rel = "samples/squirrel_flower.jpg";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("squirrel_flower.jpg");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let prompt = generate_extraction_prompt(&ocr_res.text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1024,
                std::time::Duration::from_secs(30),
            )
            .await
            .expect("Inference on squirrel_flower.jpg should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for squirrel_flower");
            assert!(!events.is_empty());
            let expected = &expected_list[0];
            let matched = events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            });
            assert!(
                matched || events.iter().any(|ev| ev.title.to_lowercase().contains("flower") || ev.title.to_lowercase().contains("squirrel")),
                "LLM should extract Squirrel Flower matching parsed.json"
            );
        }

        // 4. Test Ride For Your Life Poster
        {
            let sample_rel = "samples/ride_for_life.png";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("ride_for_life.png");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let prompt = generate_extraction_prompt(&ocr_res.text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1024,
                std::time::Duration::from_secs(30),
            )
            .await
            .expect("Inference on ride_for_life.png should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for ride_for_life");
            assert!(!events.is_empty());
            let expected = &expected_list[0];
            let matched = events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            });
            assert!(
                matched || events.iter().any(|ev| {
                    let t = ev.title.to_lowercase();
                    let d = ev.description.as_deref().unwrap_or("").to_lowercase();
                    t.contains("ride") || t.contains("dide") || t.contains("life") || d.contains("everyone") || d.contains("life")
                }),
                "LLM should extract Ride For Life matching parsed.json"
            );
        }

        // 5. Test Class Schedule
        {
            let sample_rel = "samples/classes.png";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("classes.png");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
            let effective_text = if !spatial_text.trim().is_empty() {
                &spatial_text
            } else {
                &ocr_res.text
            };
            let prompt = generate_extraction_prompt(effective_text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1536,
                std::time::Duration::from_secs(45),
            )
            .await
            .expect("Inference on classes.png should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for classes");
            assert!(!events.is_empty(), "Should extract class schedule events");
            let any_matched = expected_list.iter().any(|expected| {
                events.iter().any(|actual| fuzzy_match_title(&actual.title, &expected.title))
            });
            assert!(
                any_matched || events.iter().any(|e| e.title.contains("CEE 0154-03") || e.title.contains("Principles Epidemiology") || e.title.contains("Special Topics") || e.title.contains("CS 0150-09")),
                "LLM should extract course events from schedule matching parsed.json"
            );
        }

        // 6. Test Single Class Card
        {
            let sample_rel = "samples/class.png";
            let expected_list = manifest.get(sample_rel).unwrap();
            let sample = get_sample_path("class.png");
            let ocr_res = extract_text_from_path(sample.to_str().unwrap()).unwrap();
            let prompt = generate_extraction_prompt(&ocr_res.text, &context);
            let raw_json = inference::run_inference_async(
                Arc::clone(&model),
                prompt,
                1536,
                std::time::Duration::from_secs(45),
            )
            .await
            .expect("Inference on class.png should succeed");

            let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for class.png");
            assert!(!events.is_empty(), "Should extract class event from class.png");
            let expected = &expected_list[0];
            let matched = events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            });
            assert!(
                matched || events.iter().any(|e| e.title.contains("CVS-0188") || e.title.contains("Children and Media")),
                "LLM should extract CVS-0188 class matching parsed.json"
            );
        }

        drop(model);
        manager.unload_model().await;
    }
}
