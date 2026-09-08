use crate::parser::EventDetails;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};

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
    use super::{parse_date_to_epoch, EventDetails};
    use chrono::Local;
    use std::ffi::{CStr, CString};
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

        fn calendar_apple_create_event(
            title: *const c_char,
            start_epoch: c_double,
            end_epoch: c_double,
            is_all_day: c_int,
            location: *const c_char,
            notes: *const c_char,
            url: *const c_char,
            recurrence_rule: *const c_char,
            out_event_id: *mut *mut c_char,
            out_error: *mut *mut c_char,
        ) -> c_int;
        fn calendar_apple_free_string(ptr: *mut c_char);
    }

    struct AutoCString(*mut c_char);

    impl Drop for AutoCString {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    calendar_apple_free_string(self.0);
                }
            }
        }
    }

    pub fn check_permission() -> Result<String, String> {
        let mut out_status: *mut c_char = std::ptr::null_mut();
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let code = unsafe { calendar_apple_check_permission(&mut out_status, &mut out_error) };
        let _status_guard = AutoCString(out_status);
        let _error_guard = AutoCString(out_error);

        if code != 0 || out_status.is_null() {
            let err_msg = if !out_error.is_null() {
                unsafe { CStr::from_ptr(out_error).to_string_lossy().into_owned() }
            } else {
                "Failed to check calendar permission.".to_string()
            };
            return Err(err_msg);
        }

        let status = unsafe { CStr::from_ptr(out_status).to_string_lossy().into_owned() };
        Ok(status)
    }

    pub fn request_permission() -> Result<bool, String> {
        let mut out_granted: c_int = 0;
        let mut out_error: *mut c_char = std::ptr::null_mut();

        let code = unsafe { calendar_apple_request_permission(&mut out_granted, &mut out_error) };
        let _error_guard = AutoCString(out_error);

        if code != 0 {
            let err_msg = if !out_error.is_null() {
                unsafe { CStr::from_ptr(out_error).to_string_lossy().into_owned() }
            } else {
                "Failed to request calendar permission.".to_string()
            };
            return Err(err_msg);
        }

        Ok(out_granted == 1)
    }

    pub fn create_event(event: &EventDetails) -> Result<String, String> {
        let trimmed_title = event.title.trim();
        if trimmed_title.is_empty() {
            return Err("Event title cannot be empty.".to_string());
        }

        let title_c = CString::new(trimmed_title).map_err(|e| e.to_string())?;

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
            Some(s) => Some(CString::new(s).map_err(|e| e.to_string())?),
            None => None,
        };

        let notes_c = match event.description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(CString::new(s).map_err(|e| e.to_string())?),
            None => None,
        };
        let url_c: Option<CString> = None; // Reserved for URL if extended

        let recurrence_rule_c = match event.recurrence_rule.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => {
                let parsed = crate::parser::RecurrenceRule::parse_rrule(s)
                    .map_err(|e| format!("Invalid recurrence rule: {}", e))?;
                let normalized = parsed.to_rrule_string();
                Some(CString::new(normalized).map_err(|e| e.to_string())?)
            }
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
                &mut out_event_id,
                &mut out_error,
            )
        };

        let _event_id_guard = AutoCString(out_event_id);
        let _error_guard = AutoCString(out_error);

        if code != 0 || out_event_id.is_null() {
            let err_msg = if !out_error.is_null() {
                unsafe { CStr::from_ptr(out_error).to_string_lossy().into_owned() }
            } else {
                "Failed to create event in calendar.".to_string()
            };
            return Err(err_msg);
        }

        let event_id = unsafe { CStr::from_ptr(out_event_id).to_string_lossy().into_owned() };
        Ok(event_id)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod fallback {
    use super::EventDetails;

    pub fn check_permission() -> Result<String, String> {
        Ok("authorized".to_string())
    }

    pub fn request_permission() -> Result<bool, String> {
        Ok(true)
    }

    pub fn create_event(event: &EventDetails) -> Result<String, String> {
        if event.title.trim().is_empty() {
            return Err("Event title cannot be empty.".to_string());
        }
        Ok(format!("mock_event_{}", event.title.trim()))
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use apple::*;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub use fallback::*;

/// Batch helper to create multiple calendar events
pub fn create_events(events: &[EventDetails]) -> Result<Vec<String>, String> {
    let mut event_ids = Vec::new();
    for event in events {
        let id = create_event(event)?;
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
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event);
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
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event);
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
            confidence: 0.95,
            source: "test".to_string(),
        };

        let result = create_event(&event);
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
}
