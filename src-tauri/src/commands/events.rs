use tauri::Manager;
use crate::error::AppError;
use crate::inference;
use crate::ocr::{self, extract_text_from_bytes, extract_text_from_path};
use crate::parser::{
    self, generate_extraction_prompt, get_gbnf_grammar, get_json_schema,
    parse_event_deterministic, parse_events_deterministic, EventDetails, ReferenceContext,
};
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
        if let Some(engine) = app_handle.try_state::<inference::InferenceEngineManager>() {
            inference::extract_events_orchestrated(app_handle, &engine, text, &context, model_id, timeout_secs)
                .await
        } else {
            let engine = inference::InferenceEngineManager::default();
            inference::extract_events_orchestrated(app_handle, &engine, text, &context, model_id, timeout_secs)
                .await
        }
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
pub async fn parse_events_from_text(
    app: tauri::AppHandle,
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, AppError> {
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
pub async fn parse_event_from_text(
    app: tauri::AppHandle,
    text: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, AppError> {
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
pub async fn extract_events_from_image(
    app: tauri::AppHandle,
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, AppError> {
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
pub async fn extract_event_from_image(
    app: tauri::AppHandle,
    path: String,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, AppError> {
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
pub async fn extract_events_from_image_bytes(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<Vec<EventDetails>, AppError> {
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
pub async fn extract_event_from_image_bytes(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    reference_time: Option<String>,
    timezone_offset_minutes: Option<i32>,
    model_id: Option<String>,
    timeout_secs: Option<u64>,
    mode: Option<String>,
) -> Result<EventDetails, AppError> {
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
pub fn get_event_schema() -> serde_json::Value {
    get_json_schema()
}

#[tauri::command]
pub fn get_event_gbnf_grammar() -> String {
    get_gbnf_grammar().to_string()
}

#[tauri::command]
pub fn generate_event_prompt(
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
