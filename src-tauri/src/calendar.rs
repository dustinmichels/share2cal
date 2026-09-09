use crate::error::AppError;
use crate::parser::EventDetails;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    pub source_title: String,
    pub color: String,
    pub is_default: bool,
    pub allows_modifications: bool,
}
/// Parses various date-time string formats into epoch seconds (as f64).
pub fn parse_date_to_epoch(date_str: &str) -> Option<f64> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. Try RFC 3339 / ISO 8601 with timezone (e.g., "2026-09-06T14:00:00Z", "2026-09-06T14:00:00-07:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.timestamp() as f64 + (dt.timestamp_subsec_nanos() as f64 / 1_000_000_000.0));
    }

    // 2. Try ISO format without seconds or with standard format (e.g. "2026-09-06T14:00:00")
    if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        if let Some(local_dt) = Local.from_local_datetime(&ndt).single() {
            return Some(local_dt.timestamp() as f64);
        }
        return Some(ndt.and_utc().timestamp() as f64);
    }

    // 3. Try ISO format without seconds (e.g. "2026-09-06T14:00")
    if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M") {
        if let Some(local_dt) = Local.from_local_datetime(&ndt).single() {
            return Some(local_dt.timestamp() as f64);
        }
        return Some(ndt.and_utc().timestamp() as f64);
    }

    // 4. Try Date-only format (e.g. "2026-09-06")
    if let Ok(nd) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
            if let Some(local_dt) = Local.from_local_datetime(&ndt).single() {
                return Some(local_dt.timestamp() as f64);
            }
            return Some(ndt.and_utc().timestamp() as f64);
        }
    }

    None
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple {
    use super::{parse_date_to_epoch, AppError, CalendarInfo, EventDetails};
    use crate::ffi::NativeStringGuard;
    use chrono::Local;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_double, c_int};

    extern "C" {
        fn calendar_apple_check_permission(
            out_status: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;

        fn calendar_apple_request_permission(
            out_granted: *mut c_int,
            out_error: *mut *mut c_char,
        ) -> c_int;

        fn calendar_apple_list_calendars(
            out_json: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;

        fn calendar_apple_create_event(
            title: *const c_char,
            start_epoch: c_double,
            end_epoch: c_double,
            is_all_day: c_int,
            location: *const c_char,
            notes: *const c_char,
            url: *const c_char,
            recurrence_rule: *const c_char,
            calendar_id: *const c_char,
            calendar_title: *const c_char,
            calendar_source_title: *const c_char,
            out_event_id: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;
        fn calendar_apple_free_string(ptr: *mut c_char);
    }

    pub fn check_permission() -> Result<String, AppError> {
        let mut out_status: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let code = unsafe { calendar_apple_check_permission(&mut out_status, &mut out_error) };
        let guard_status = NativeStringGuard::new(out_status, calendar_apple_free_string);
        let guard_error = NativeStringGuard::new(out_error, calendar_apple_free_string);

        if code != 0 || guard_status.is_null() {
            let err_msg = guard_error
                .to_string_lossy()
                .unwrap_or_else(|| "Failed to check calendar permission.".to_string());
            return Err(AppError::Calendar(err_msg));
        }

        let status = guard_status
            .to_string_lossy()
            .ok_or_else(|| AppError::Calendar("Permission status is null.".to_string()))?;
        Ok(status)
    }

    pub fn request_permission() -> Result<bool, AppError> {
        let mut out_granted: c_int = 0;
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let code = unsafe { calendar_apple_request_permission(&mut out_granted, &mut out_error) };
        let guard_error = NativeStringGuard::new(out_error, calendar_apple_free_string);

        if code != 0 {
            let err_msg = guard_error
                .to_string_lossy()
                .unwrap_or_else(|| "Failed to request calendar permission.".to_string());
            return Err(AppError::Calendar(err_msg));
        }

        Ok(out_granted == 1)
    }

    pub fn list_calendars() -> Result<Vec<CalendarInfo>, AppError> {
        let mut out_json: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let code = unsafe { calendar_apple_list_calendars(&mut out_json, &mut out_error) };
        let guard_json = NativeStringGuard::new(out_json, calendar_apple_free_string);
        let guard_error = NativeStringGuard::new(out_error, calendar_apple_free_string);

        if code != 0 || guard_json.is_null() {
            let err_msg = guard_error
                .to_string_lossy()
                .unwrap_or_else(|| "Failed to retrieve available calendars.".to_string());
            return Err(AppError::Calendar(err_msg));
        }

        let json_str = guard_json
            .to_string_lossy()
            .ok_or_else(|| AppError::Calendar("Calendar list JSON is null.".to_string()))?;
        serde_json::from_str(&json_str).map_err(|e| AppError::Calendar(format!("Failed to parse calendar list: {}", e)))
    }

    pub fn create_event(
        event: &EventDetails,
        calendar_id: Option<&str>,
        calendar_title: Option<&str>,
        calendar_source_title: Option<&str>,
    ) -> Result<String, AppError> {
        let trimmed_title = event.title.trim();
        if trimmed_title.is_empty() {
            return Err(AppError::Calendar("Event title cannot be empty.".to_string()));
        }

        let title_c = CString::new(trimmed_title).map_err(|e| AppError::Calendar(e.to_string()))?;

        // Calculate start epoch
        let start_epoch = match event.start_time.as_deref().and_then(parse_date_to_epoch) {
            Some(epoch) => epoch,
            None => {
                // Default to now
                Local::now().timestamp() as f64
            }
        };

        // Calculate end epoch
        let end_epoch = match event.end_time.as_deref().and_then(parse_date_to_epoch) {
            Some(epoch) => epoch,
            None => {
                if event.is_all_day {
                    start_epoch
                } else {
                    start_epoch + 3600.0 // 1 hour duration
                }
            }
        };

        let is_all_day_c: c_int = if event.is_all_day { 1 } else { 0 };

        let location_c = match event.location.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };

        let notes_c = match event.description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };
        let url_c = match event.url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };

        let recurrence_rule_c = match event.recurrence_rule.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => {
                let parsed = crate::parser::RecurrenceRule::parse_rrule(s)
                    .map_err(|e| AppError::Calendar(format!("Invalid recurrence rule: {}", e)))?;
                let normalized = parsed.to_rrule_string();
                Some(CString::new(normalized).map_err(|e| AppError::Calendar(e.to_string()))?)
            }
            None => None,
        };

        let calendar_id_c = match calendar_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };

        let calendar_title_c = match calendar_title.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };

        let calendar_source_title_c = match calendar_source_title.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| AppError::Calendar(e.to_string()))?),
            None => None,
        };

        let mut out_event_id: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            calendar_apple_create_event(
                title_c.as_ptr(),
                start_epoch,
                end_epoch,
                is_all_day_c,
                location_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                notes_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                url_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                recurrence_rule_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                calendar_id_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                calendar_title_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                calendar_source_title_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                &mut out_event_id,
                &mut out_error,
            )
        };

        let guard_event_id = NativeStringGuard::new(out_event_id, calendar_apple_free_string);
        let guard_error = NativeStringGuard::new(out_error, calendar_apple_free_string);

        if code != 0 || guard_event_id.is_null() {
            let err_msg = guard_error
                .to_string_lossy()
                .unwrap_or_else(|| "Failed to create event in calendar.".to_string());
            return Err(AppError::Calendar(err_msg));
        }

        let event_id = guard_event_id
            .to_string_lossy()
            .ok_or_else(|| AppError::Calendar("Event ID is null.".to_string()))?;
        Ok(event_id)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod fallback {
    use super::{AppError, CalendarInfo, EventDetails};

    pub fn check_permission() -> Result<String, AppError> {
        Ok("authorized".to_string())
    }

    pub fn request_permission() -> Result<bool, AppError> {
        Ok(true)
    }

    pub fn list_calendars() -> Result<Vec<CalendarInfo>, AppError> {
        Ok(vec![
            CalendarInfo {
                id: "default".to_string(),
                title: "Personal".to_string(),
                source_title: "Default Account".to_string(),
                color: "#3B82F6".to_string(),
                is_default: true,
                allows_modifications: true,
            },
            CalendarInfo {
                id: "work".to_string(),
                title: "Work".to_string(),
                source_title: "Work Account".to_string(),
                color: "#10B981".to_string(),
                is_default: false,
                allows_modifications: true,
            },
        ])
    }

    pub fn create_event(
        event: &EventDetails,
        _calendar_id: Option<&str>,
        _calendar_title: Option<&str>,
        _calendar_source_title: Option<&str>,
    ) -> Result<String, AppError> {
        if event.title.trim().is_empty() {
            return Err(AppError::Calendar("Event title cannot be empty.".to_string()));
        }
        Ok(format!("mock_event_{}", event.title.trim()))
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
use apple as native;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
use fallback as native;

pub mod mock {
    use super::*;
    use parking_lot::RwLock;
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct MockCreatedEvent {
        pub id: String,
        pub event: EventDetails,
        pub calendar_id: Option<String>,
        pub calendar_title: Option<String>,
        pub calendar_source_title: Option<String>,
    }

    static MOCK_EVENTS: RwLock<Vec<MockCreatedEvent>> = RwLock::new(Vec::new());

    pub fn get_created_events() -> Vec<MockCreatedEvent> {
        MOCK_EVENTS.read().clone()
    }

    pub fn clear_created_events() {
        MOCK_EVENTS.write().clear();
    }

    pub fn check_permission() -> Result<String, AppError> {
        Ok("authorized".to_string())
    }

    pub fn request_permission() -> Result<bool, AppError> {
        Ok(true)
    }

    pub fn list_calendars() -> Result<Vec<CalendarInfo>, AppError> {
        Ok(vec![
            CalendarInfo {
                id: "mock_default".to_string(),
                title: "Personal".to_string(),
                source_title: "iCloud".to_string(),
                color: "#4285F4".to_string(),
                is_default: true,
                allows_modifications: true,
            },
            CalendarInfo {
                id: "mock_work".to_string(),
                title: "Work".to_string(),
                source_title: "Work Account".to_string(),
                color: "#10B981".to_string(),
                is_default: false,
                allows_modifications: true,
            },
        ])
    }

    pub fn create_event(
        event: &EventDetails,
        calendar_id: Option<&str>,
        calendar_title: Option<&str>,
        calendar_source_title: Option<&str>,
    ) -> Result<String, AppError> {
        let trimmed_title = event.title.trim();
        if trimmed_title.is_empty() {
            return Err(AppError::Calendar("Event title cannot be empty.".to_string()));
        }

        if let Some(rrule) = event.recurrence_rule.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            crate::parser::RecurrenceRule::parse_rrule(rrule)
                .map_err(|e| AppError::Calendar(format!("Invalid recurrence rule: {}", e)))?;
        }

        let mut events = MOCK_EVENTS.write();
        let event_id = format!("mock_event_{}_{}", trimmed_title.replace(' ', "_"), events.len() + 1);
        events.push(MockCreatedEvent {
            id: event_id.clone(),
            event: event.clone(),
            calendar_id: calendar_id.map(str::to_string),
            calendar_title: calendar_title.map(str::to_string),
            calendar_source_title: calendar_source_title.map(str::to_string),
        });
        Ok(event_id)
    }
}

static MOCK_OVERRIDE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_mock_mode(enabled: bool) {
    MOCK_OVERRIDE.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_mock_active() -> bool {
    if MOCK_OVERRIDE.load(std::sync::atomic::Ordering::SeqCst) {
        return true;
    }
    if cfg!(test) {
        return true;
    }
    if let Ok(val) = std::env::var("SHARE2CAL_MOCK_CALENDAR") {
        if val != "0" && !val.eq_ignore_ascii_case("false") {
            return true;
        }
    }
    if std::env::var("RUST_TEST_NOCAPTURE").is_ok() {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("/target/") && (exe_str.contains("/deps/") || exe_str.contains("test")) {
            return true;
        }
    }
    false
}

pub fn check_permission() -> Result<String, AppError> {
    if is_mock_active() {
        return mock::check_permission();
    }
    native::check_permission()
}

pub fn request_permission() -> Result<bool, AppError> {
    if is_mock_active() {
        return mock::request_permission();
    }
    native::request_permission()
}

pub fn list_calendars() -> Result<Vec<CalendarInfo>, AppError> {
    if is_mock_active() {
        return mock::list_calendars();
    }
    native::list_calendars()
}

pub fn create_event(
    event: &EventDetails,
    calendar_id: Option<&str>,
    calendar_title: Option<&str>,
    calendar_source_title: Option<&str>,
) -> Result<String, AppError> {
    if is_mock_active() {
        return mock::create_event(event, calendar_id, calendar_title, calendar_source_title);
    }
    native::create_event(event, calendar_id, calendar_title, calendar_source_title)
}

/// Batch helper to create multiple calendar events.
/// Pre-validates all events to ensure none are created if any event is invalid.
pub fn create_events(
    events: &[EventDetails],
    calendar_id: Option<&str>,
    calendar_title: Option<&str>,
    calendar_source_title: Option<&str>,
) -> Result<Vec<String>, AppError> {
    // 1. Pre-validate all events before creating any
    for event in events {
        let trimmed_title = event.title.trim();
        if trimmed_title.is_empty() {
            return Err(AppError::Calendar("Event title cannot be empty.".to_string()));
        }

        if let Some(rrule) = event.recurrence_rule.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            crate::parser::RecurrenceRule::parse_rrule(rrule)
                .map_err(|e| AppError::Calendar(format!("Invalid recurrence rule: {}", e)))?;
        }
    }

    // 2. Create each event
    let mut event_ids = Vec::with_capacity(events.len());
    for event in events {
        let id = create_event(event, calendar_id, calendar_title, calendar_source_title)?;
        event_ids.push(id);
    }
    Ok(event_ids)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_to_epoch_rfc3339() {
        let epoch = parse_date_to_epoch("2026-09-06T14:00:00Z");
        assert!(epoch.is_some());
        assert_eq!(epoch.unwrap() as i64, 1788703200);
    }

    #[test]
    fn test_parse_date_to_epoch_naive_iso() {
        let epoch = parse_date_to_epoch("2026-09-06T14:00:00");
        assert!(epoch.is_some());
    }

    #[test]
    fn test_parse_date_to_epoch_date_only() {
        let epoch = parse_date_to_epoch("2026-09-06");
        assert!(epoch.is_some());
    }
    #[test]
    fn test_parse_date_to_epoch_with_offset() {
        let epoch = parse_date_to_epoch("2026-09-06T14:00:00-04:00");
        assert!(epoch.is_some());
        assert_eq!(epoch.unwrap() as i64, 1788717600);
    }

    #[test]
    fn test_parse_date_to_epoch_without_seconds() {
        let epoch = parse_date_to_epoch("2026-09-06T14:00");
        assert!(epoch.is_some());
    }

    #[test]
    fn test_parse_date_to_epoch_invalid() {
        assert_eq!(parse_date_to_epoch("not-a-date"), None);
        assert_eq!(parse_date_to_epoch(""), None);
        assert_eq!(parse_date_to_epoch("   "), None);
    }

    #[test]
    fn test_create_event_empty_title() {
        let event = EventDetails {
            title: "".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: None,
            description: None,
            recurrence_rule: None,
            url: None,
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event, None, None, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event title cannot be empty.");
    }

    #[test]
    fn test_create_event_whitespace_title() {
        let event = EventDetails {
            title: "   ".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: None,
            description: None,
            recurrence_rule: None,
            url: None,
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event, Some("default"), Some("Work"), Some("Exchange"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event title cannot be empty.");
    }

    #[test]
    fn test_create_event_invalid_recurrence_rule() {
        let event = EventDetails {
            title: "Test Event".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: None,
            description: None,
            recurrence_rule: Some("INTERVAL=2;BYDAY=MO".to_string()), // Missing required FREQ
            url: None,
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid recurrence rule"));
    }

    #[test]
    fn test_check_permission_returns_result() {
        let status = check_permission();
        assert!(status.is_ok());
        let s = status.unwrap();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_calendar_info_serde() {
        let info = CalendarInfo {
            id: "cal-123".to_string(),
            title: "Personal".to_string(),
            source_title: "iCloud".to_string(),
            color: "#4285F4".to_string(),
            is_default: true,
            allows_modifications: true,
        };
        let serialized = serde_json::to_string(&info).expect("serialize");
        let deserialized: CalendarInfo = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(info, deserialized);
    }

    #[test]
    fn test_list_calendars_returns_result() {
        let cals = list_calendars();
        assert!(cals.is_ok());
    }

    #[test]
    fn test_mock_mode_active_in_unit_tests() {
        assert!(is_mock_active(), "Mock mode must be active during tests");
    }
    static TEST_CALENDAR_MUTEX: parking_lot::Mutex<()> = parking_lot::Mutex::new(());


    #[test]
    fn test_create_event_valid_mock_isolation() {
        let _guard = TEST_CALENDAR_MUTEX.lock();
        mock::clear_created_events();
        let event = EventDetails {
            title: "Test Isolation Event".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: Some("Virtual".to_string()),
            description: Some("Test description".to_string()),
            recurrence_rule: None,
            url: Some("https://example.com/event".to_string()),
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event, None, None, None);
        assert!(result.is_ok(), "Creating valid event in mock mode should succeed");
        let event_id = result.unwrap();
        assert!(event_id.starts_with("mock_event_"), "Event ID should be a mock ID");

        let created = mock::get_created_events();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].event.title, "Test Isolation Event");
        assert_eq!(created[0].event.url.as_deref(), Some("https://example.com/event"));
        mock::clear_created_events();
    }

    #[test]
    fn test_create_events_batch_prevalidation_atomicity() {
        let _guard = TEST_CALENDAR_MUTEX.lock();
        mock::clear_created_events();
        let valid_event = EventDetails {
            title: "Valid Batch Event".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: None,
            description: None,
            recurrence_rule: None,
            url: None,
            confidence: 0.95,
            source: "test".to_string(),
        };
        let invalid_event = EventDetails {
            title: "".to_string(),
            start_time: Some("2026-09-06T14:00:00Z".to_string()),
            end_time: Some("2026-09-06T15:00:00Z".to_string()),
            is_all_day: false,
            location: None,
            description: None,
            recurrence_rule: None,
            url: None,
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_events(&[valid_event, invalid_event], None, None, None);
        assert!(result.is_err(), "Batch with invalid event should be rejected");
        assert_eq!(result.unwrap_err(), "Event title cannot be empty.");

        // Prevalidation ensures the first valid event was never created
        let created = mock::get_created_events();
        assert_eq!(created.len(), 0, "No event should have been created on batch validation failure");
        mock::clear_created_events();
    }

    #[test]
    fn test_create_events_batch_url_persistence() {
        let _guard = TEST_CALENDAR_MUTEX.lock();
        mock::clear_created_events();

        let event1 = EventDetails {
            title: "Webinar Event".to_string(),
            start_time: Some("2026-09-10T12:00:00-04:00".to_string()),
            end_time: Some("2026-09-10T13:00:00-04:00".to_string()),
            is_all_day: false,
            location: Some("Curtis Hall".to_string()),
            description: Some("Visiting Artist lecture".to_string()),
            recurrence_rule: None,
            url: Some("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw".to_string()),
            confidence: 0.95,
            source: "ocr".to_string(),
        };

        let event2 = EventDetails {
            title: "Concert Event".to_string(),
            start_time: Some("2026-09-12T19:00:00-04:00".to_string()),
            end_time: Some("2026-09-12T22:00:00-04:00".to_string()),
            is_all_day: false,
            location: Some("Main Stage".to_string()),
            description: Some("Live Performance".to_string()),
            recurrence_rule: None,
            url: Some("https://example.com/tickets".to_string()),
            confidence: 0.95,
            source: "ocr".to_string(),
        };

        let result = create_events(&[event1, event2], None, None, None);
        assert!(result.is_ok(), "Batch creation should succeed");
        let ids = result.unwrap();
        assert_eq!(ids.len(), 2);

        let created = mock::get_created_events();
        assert_eq!(created.len(), 2);
        assert_eq!(created[0].event.title, "Webinar Event");
        assert_eq!(
            created[0].event.url.as_deref(),
            Some("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw")
        );
        assert_eq!(created[1].event.title, "Concert Event");
        assert_eq!(
            created[1].event.url.as_deref(),
            Some("https://example.com/tickets")
        );

        mock::clear_created_events();
    }
}
