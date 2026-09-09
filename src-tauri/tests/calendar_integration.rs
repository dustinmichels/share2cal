use share2cal_lib::{
    test_support::*,
    CalendarInfo, EventDetails,
};

#[test]
fn test_check_calendar_permission_command() {
    let status = check_calendar_permission();
    assert!(status.is_ok(), "check_calendar_permission command should succeed");
    let s = status.unwrap();
    assert!(!s.is_empty(), "Permission status string should not be empty");
}

#[test]
fn test_request_calendar_permission_command() {
    let result = request_calendar_permission();
    assert!(result.is_ok(), "request_calendar_permission command should return a Result");
}

#[test]
fn test_get_available_calendars_command() {
    let cals = get_available_calendars();
    assert!(cals.is_ok(), "get_available_calendars command should succeed");
}

#[test]
fn test_create_calendar_event_empty_title_rejection() {
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

    let result = create_calendar_event(event, None, None, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Event title cannot be empty.");
}

#[test]
fn test_create_calendar_event_whitespace_title_rejection() {
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

    let result = create_calendar_event(event, Some("default".to_string()), Some("Work".to_string()), Some("Exchange".to_string()));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Event title cannot be empty.");
}

#[test]
fn test_create_calendar_event_invalid_recurrence_rule() {
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

    let result = create_calendar_event(event, None, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid recurrence rule"));
}

#[test]
fn test_mock_calendar_is_active_during_integration_tests() {
    assert!(
        is_mock_calendar_active(),
        "Mock calendar must always be active during integration tests"
    );
}
static TEST_CALENDAR_MUTEX: parking_lot::Mutex<()> = parking_lot::Mutex::new(());


#[test]
fn test_create_calendar_event_mock_isolation_prevents_real_calendar_writes() {
    let _guard = TEST_CALENDAR_MUTEX.lock();
    clear_mock_created_events();
    let valid_event = EventDetails {
        title: "Integration Test Valid Event".to_string(),
        start_time: Some("2026-09-06T14:00:00Z".to_string()),
        end_time: Some("2026-09-06T15:00:00Z".to_string()),
        is_all_day: false,
        location: Some("Test Lab".to_string()),
        description: Some("Integration test event for mock calendar isolation".to_string()),
        recurrence_rule: None,
        url: Some("https://tufts.zoom.us/test".to_string()),
        confidence: 0.95,
        source: "test".to_string(),
    };

    let result = create_calendar_event(valid_event, None, None, None);
    assert!(result.is_ok(), "Creating valid event in test should succeed via mock");
    let event_id = result.unwrap();
    assert!(
        event_id.starts_with("mock_event_"),
        "Event ID must be a mock event ID, got: {}",
        event_id
    );

    let recorded = get_mock_created_events();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].event.title, "Integration Test Valid Event");
    assert_eq!(recorded[0].event.url.as_deref(), Some("https://tufts.zoom.us/test"));
    clear_mock_created_events();
}

#[test]
fn test_create_calendar_events_batch_validation() {
    let _guard = TEST_CALENDAR_MUTEX.lock();
    clear_mock_created_events();
    let valid_event = EventDetails {
        title: "Valid Event".to_string(),
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

    let result = create_calendar_events(vec![valid_event, invalid_event], None, None, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Event title cannot be empty.");

    // Prevalidation guarantees that zero events were created on batch error
    let recorded = get_mock_created_events();
    assert_eq!(
        recorded.len(),
        0,
        "No events should be created when batch validation fails"
    );
    clear_mock_created_events();
}

#[test]
fn test_calendar_info_serde_roundtrip() {
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
