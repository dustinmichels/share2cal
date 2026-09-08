use crate::calendar;
use crate::error::AppError;
use crate::parser::EventDetails;

#[tauri::command]
pub fn get_available_calendars() -> Result<Vec<calendar::CalendarInfo>, AppError> {
    calendar::list_calendars()
}

#[tauri::command]
pub fn create_calendar_event(
    event: EventDetails,
    calendar_id: Option<String>,
    calendar_title: Option<String>,
    calendar_source_title: Option<String>,
) -> Result<String, AppError> {
    calendar::create_event(
        &event,
        calendar_id.as_deref(),
        calendar_title.as_deref(),
        calendar_source_title.as_deref(),
    )
}

#[tauri::command]
pub fn create_calendar_events(
    events: Vec<EventDetails>,
    calendar_id: Option<String>,
    calendar_title: Option<String>,
    calendar_source_title: Option<String>,
) -> Result<Vec<String>, AppError> {
    calendar::create_events(
        &events,
        calendar_id.as_deref(),
        calendar_title.as_deref(),
        calendar_source_title.as_deref(),
    )
}

#[tauri::command]
pub fn check_calendar_permission() -> Result<String, AppError> {
    calendar::check_permission()
}

#[tauri::command]
pub fn request_calendar_permission() -> Result<bool, AppError> {
    calendar::request_permission()
}
