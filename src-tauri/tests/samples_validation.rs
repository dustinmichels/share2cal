mod common;
use common::*;
use share2cal_lib::{
    inference, ocr, parser,
    test_support::*,
    ReferenceContext,
};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_parsed_json_manifest_loads_all_samples() {
    let manifest = load_parsed_json_manifest();
    assert_eq!(manifest.len(), 7, "parsed.json must contain all 7 sample images");
    assert!(manifest.contains_key("samples/class.png"));
    assert!(manifest.contains_key("samples/classes.png"));
    assert!(manifest.contains_key("samples/commons.jpg"));
    assert!(manifest.contains_key("samples/gilman_flyer.png"));
    assert!(manifest.contains_key("samples/instagram.png"));
    assert!(manifest.contains_key("samples/ride_for_life.png"));
    assert!(manifest.contains_key("samples/squirrel_flower.jpg"));

    assert_eq!(manifest.get("samples/class.png").unwrap().len(), 1);
    assert_eq!(manifest.get("samples/classes.png").unwrap().len(), 6);
    assert_eq!(manifest.get("samples/commons.jpg").unwrap().len(), 1);
    assert_eq!(manifest.get("samples/gilman_flyer.png").unwrap().len(), 1);
    assert_eq!(manifest.get("samples/instagram.png").unwrap().len(), 1);
    assert_eq!(manifest.get("samples/ride_for_life.png").unwrap().len(), 1);
    assert_eq!(manifest.get("samples/squirrel_flower.jpg").unwrap().len(), 1);
}

#[test]
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn test_all_samples_ocr_prompts_fit_context_budget() {
    let manifest = load_parsed_json_manifest();
    let context = ReferenceContext {
        reference_time: Some("2026-09-06T12:00:00-04:00".to_string()),
        timezone_offset_minutes: Some(-240),
    };

    for sample_rel_path in manifest.keys() {
        let filename = PathBuf::from(sample_rel_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let sample_path = get_sample_path(&filename);
        if !sample_path.exists() {
            continue;
        }

        let ocr_res = ocr::extract_text_from_path(sample_path.to_str().unwrap())
            .expect("OCR extraction should succeed on sample");
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);

        // Assert prompt stays within context budget
        assert!(
            parser::estimate_token_count(&prompt) <= parser::MAX_PROMPT_TOKENS,
            "Sample {} prompt estimated tokens ({}) exceeds MAX_PROMPT_TOKENS ({})",
            sample_rel_path,
            parser::estimate_token_count(&prompt),
            parser::MAX_PROMPT_TOKENS
        );

        // Assert chat template boundaries are intact
        assert!(prompt.starts_with("<|im_start|>system"));
        assert!(prompt.ends_with("<|im_start|>assistant\n{\"events\": ["));
    }
}

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn test_all_samples_simple_deterministic_against_parsed_json() {
    let manifest = load_parsed_json_manifest();
    let reference_time = "2026-09-06T12:00:00-04:00".to_string(); // Sunday before Fall 2026 Term
    let timezone_offset_minutes = -240; // EDT
    let mut all_mismatches: Vec<SampleMismatch> = Vec::new();

    // Sort sample paths for deterministic test iteration order
    let mut sample_keys: Vec<String> = manifest.keys().cloned().collect();
    sample_keys.sort();

    for sample_rel_path in sample_keys {
        let expected_events = &manifest[&sample_rel_path];
        let filename = PathBuf::from(&sample_rel_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let sample_path = get_sample_path(&filename);
        if !sample_path.exists() {
            all_mismatches.push(SampleMismatch {
                sample_path: sample_rel_path.clone(),
                event_index: None,
                message: format!("Sample file {:?} does not exist", sample_path),
            });
            continue;
        }

        // Step 1: Run native Apple Vision OCR
        let ocr_res = match ocr::extract_text_from_path(sample_path.to_str().unwrap()) {
            Ok(res) => res,
            Err(e) => {
                all_mismatches.push(SampleMismatch {
                    sample_path: sample_rel_path.clone(),
                    event_index: None,
                    message: format!("OCR failed: {}", e),
                });
                continue;
            }
        };
        if ocr_res.text.is_empty() {
            all_mismatches.push(SampleMismatch {
                sample_path: sample_rel_path.clone(),
                event_index: None,
                message: "OCR text is empty".to_string(),
            });
            continue;
        }

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

        if actual_events.len() != expected_events.len() {
            all_mismatches.push(SampleMismatch {
                sample_path: sample_rel_path.clone(),
                event_index: None,
                message: format!(
                    "Event count mismatch: expected {}, got {}",
                    expected_events.len(),
                    actual_events.len()
                ),
            });
        }

        // Step 4: Validate each extracted event against parsed.json
        for (idx, expected) in expected_events.iter().enumerate() {
            if let Some(actual) = actual_events.get(idx) {
                let mut mismatches = validate_event_against_parsed_json(
                    actual,
                    expected,
                    &sample_rel_path,
                    idx,
                );
                all_mismatches.append(&mut mismatches);
            }
        }
    }

    if !all_mismatches.is_empty() {
        let summary = all_mismatches
            .iter()
            .map(|m| format!("  - {}", m))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "\n=== Sample Validation Mismatch Report ({} failures) ===\n{}\n=======================================================",
            all_mismatches.len(),
            summary
        );
    }
}

/// Tests relative start date calculation for recurring weekly schedules.
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn test_classes_schedule_relative_date_handling_next_weekday() {
    let sample_path = get_sample_path("classes.png");
    let ocr_res = ocr::extract_text_from_path(sample_path.to_str().unwrap()).unwrap();
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

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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
    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn test_extract_event_from_ride_for_life_sample_image() {
    let sample = get_sample_path("ride_for_life.png");
    assert!(sample.exists(), "Sample flyer {:?} must exist", sample);

    let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap())
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

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[ignore = "strict ground-truth assertions need LLM time normalization — Section 3"]
async fn test_all_samples_llm_inference_enhanced_against_parsed_json() {
    let manifest = load_parsed_json_manifest();
    let model_path = match get_test_model_path() {
        Some(p) => p,
        None => {
            let in_ci = std::env::var("CI").is_ok()
                || std::env::var("GITHUB_ACTIONS").is_ok()
                || std::env::var("REQUIRE_MODEL").is_ok();
            if in_ci {
                panic!(
                    "FATAL: CI/REQUIRE_MODEL environment detected, but SmolLM2 model weights \
                     were not found on disk. Candidate paths checked: SHARE2CAL_MODELS_DIR, \
                     ~/Library/Application Support/com.dustinmichels.share2cal/models/."
                );
            } else {
                eprintln!(
                    "[SKIPPED] test_all_samples_llm_inference_enhanced_against_parsed_json: \
                     SmolLM2 model weights not found on disk. Set REQUIRE_MODEL=1 or CI=1 to \
                     require model weights during test execution."
                );
                return;
            }
        }
    };

    let manager = inference::InferenceEngineManager::new();
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
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);
        let raw_json = inference::run_inference_async(
            Arc::clone(&model),
            prompt,
            1024,
            std::time::Duration::from_secs(30),
        )
        .await
        .expect("Inference on gilman_flyer should succeed");

        let events = inference::parse_llm_json_payload(&raw_json).expect("Should parse LLM events payload for gilman_flyer");
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
            matched,
            "LLM should extract Gilman Festival event matching parsed.json"
        );
    }

    // 2. Test Instagram Ice Cream Social
    {
        let sample_rel = "samples/instagram.png";
        let expected_list = manifest.get(sample_rel).unwrap();
        let sample = get_sample_path("instagram.png");
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);
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
                && actual.is_all_day == expected.is_all_day
                && match_date_and_time(
                    actual.start_time.as_deref(),
                    actual.end_time.as_deref(),
                    actual.is_all_day,
                    expected,
                )
        });
        assert!(
            matched,
            "LLM should extract UEP Ice Cream Social matching parsed.json"
        );
    }

    // 3. Test Squirrel Flower Tour Poster
    {
        let sample_rel = "samples/squirrel_flower.jpg";
        let expected_list = manifest.get(sample_rel).unwrap();
        let sample = get_sample_path("squirrel_flower.jpg");
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);
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
                && actual.is_all_day == expected.is_all_day
                && match_date_and_time(
                    actual.start_time.as_deref(),
                    actual.end_time.as_deref(),
                    actual.is_all_day,
                    expected,
                )
        });
        assert!(
            matched,
            "LLM should extract Squirrel Flower matching parsed.json"
        );
    }

    // 4. Test Ride For Your Life Poster
    {
        let sample_rel = "samples/ride_for_life.png";
        let expected_list = manifest.get(sample_rel).unwrap();
        let sample = get_sample_path("ride_for_life.png");
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);
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
                && actual.is_all_day == expected.is_all_day
                && match_date_and_time(
                    actual.start_time.as_deref(),
                    actual.end_time.as_deref(),
                    actual.is_all_day,
                    expected,
                )
        });
        assert!(
            matched,
            "LLM should extract Ride For Life matching parsed.json"
        );
    }

    // 5. Test Class Schedule
    {
        let sample_rel = "samples/classes.png";
        let expected_list = manifest.get(sample_rel).unwrap();
        let sample = get_sample_path("classes.png");
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
        let effective_text = if !spatial_text.trim().is_empty() {
            &spatial_text
        } else {
            &ocr_res.text
        };
        let prompt = parser::generate_extraction_prompt(effective_text, &context);
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
        let any_matched = expected_list.iter().all(|expected| {
            events.iter().any(|actual| {
                fuzzy_match_title(&actual.title, &expected.title)
                    && actual.is_all_day == expected.is_all_day
                    && match_date_and_time(
                        actual.start_time.as_deref(),
                        actual.end_time.as_deref(),
                        actual.is_all_day,
                        expected,
                    )
            })
        });
        assert!(
            any_matched,
            "LLM should extract course events from schedule matching parsed.json"
        );
    }

    // 6. Test Single Class Card
    {
        let sample_rel = "samples/class.png";
        let expected_list = manifest.get(sample_rel).unwrap();
        let sample = get_sample_path("class.png");
        let ocr_res = ocr::extract_text_from_path(sample.to_str().unwrap()).unwrap();
        let prompt = parser::generate_extraction_prompt(&ocr_res.text, &context);
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
                && actual.is_all_day == expected.is_all_day
                && match_date_and_time(
                    actual.start_time.as_deref(),
                    actual.end_time.as_deref(),
                    actual.is_all_day,
                    expected,
                )
        });
        assert!(
            matched,
            "LLM should extract CVS-0188 class matching parsed.json"
        );
    }

    drop(model);
    manager.unload_model().await;
}
