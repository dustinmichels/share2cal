#![allow(dead_code)]
use regex::Regex;
use serde::{Deserialize, Serialize};
use share2cal_lib::EventDetails;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

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

pub fn get_sample_path(filename: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .join("samples")
        .join(filename)
}

pub fn load_parsed_json_manifest() -> HashMap<String, Vec<ParsedJsonEvent>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.parent().unwrap().join("samples").join("parsed.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read parsed.json at {:?}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse parsed.json: {}", e))
}

/// Fuzzy matching for title: checks substring or significant keyword overlap
pub fn fuzzy_match_title(actual: &str, expected: &str) -> bool {
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

/// Extracts significant tokens from an expected location string
pub fn extract_significant_location_tokens(expected_loc: &str) -> Vec<String> {
    let stop_words = [
        "room", "hall", "street", "st", "ave", "avenue", "wing", "the", "and",
        "in", "at", "for", "of", "to", "ma", "multipurpose",
    ];
    expected_loc
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.trim().to_lowercase())
        .filter(|t| t.len() >= 2 && !stop_words.contains(&t.as_str()))
        .collect()
}

/// Checks containment of significant ground-truth location tokens against the flat location field
pub fn check_location(actual: Option<&str>, expected: Option<&str>) -> Result<(), String> {
    match (actual, expected) {
        (None, None) => Ok(()),
        (Some(_), None) => Ok(()), // Tolerant if extra location parsed
        (None, Some(e)) if e.trim().is_empty() => Ok(()),
        (None, Some(e)) => Err(format!("Expected location containing {:?}, but got None", e)),
        (Some(a), Some(e)) => {
            let al = a.to_lowercase();
            let el = e.to_lowercase();
            if al.contains(&el) || el.contains(&al) {
                return Ok(());
            }
            let tokens = extract_significant_location_tokens(e);
            if tokens.is_empty() {
                return Ok(());
            }
            if tokens.iter().any(|t| al.contains(t)) {
                Ok(())
            } else {
                Err(format!(
                    "Location mismatch: expected tokens {:?} from {:?}, got {:?}",
                    tokens, e, a
                ))
            }
        }
    }
}

/// Fuzzy matching for location: checks substring or keyword overlap
pub fn fuzzy_match_location(actual: Option<&str>, expected: Option<&str>) -> bool {
    check_location(actual, expected).is_ok()
}

pub fn parse_time_to_24h(time_str: &str) -> Option<String> {
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

/// Checks date and time according to strict date/time rules against parsed.json
pub fn check_date_and_time(
    actual_start: Option<&str>,
    actual_end: Option<&str>,
    actual_is_all_day: bool,
    expected: &ParsedJsonEvent,
) -> Result<(), String> {
    if actual_is_all_day != expected.is_all_day {
        return Err(format!(
            "is_all_day mismatch: actual={}, expected={}",
            actual_is_all_day, expected.is_all_day
        ));
    }
    if expected.is_all_day {
        if let Some(e_date) = expected.date.as_deref() {
            if let Some(a_st) = actual_start {
                if !a_st.starts_with(e_date) {
                    return Err(format!(
                        "All-day start_time {:?} does not start with expected date {:?}",
                        a_st, e_date
                    ));
                }
            } else {
                return Err(format!(
                    "Expected all-day start_time starting with {:?}, got None",
                    e_date
                ));
            }
        }
        return Ok(());
    }

    if let Some(e_date) = expected.date.as_deref() {
        if let Some(a_st) = actual_start {
            if !a_st.starts_with(e_date) {
                return Err(format!(
                    "start_time {:?} does not start with expected date {:?}",
                    a_st, e_date
                ));
            }
        } else {
            return Err(format!(
                "Expected start_time starting with {:?}, got None",
                e_date
            ));
        }
    }

    if let Some(e_st) = expected.start_time.as_deref() {
        if let Some(a_st) = actual_start {
            if let Some(exp_24) = parse_time_to_24h(e_st) {
                if !a_st.contains(&exp_24) {
                    return Err(format!(
                        "start_time {:?} does not contain expected 24h time {:?} (from {:?})",
                        a_st, exp_24, e_st
                    ));
                }
            } else {
                return Err(format!("Failed to parse expected start_time {:?} to 24h format", e_st));
            }
        } else {
            return Err(format!("Expected start_time containing {:?}, got None", e_st));
        }
    }

    if let Some(e_et) = expected.end_time.as_deref() {
        if let Some(a_et) = actual_end {
            if let Some(exp_24) = parse_time_to_24h(e_et) {
                if !a_et.contains(&exp_24) {
                    return Err(format!(
                        "end_time {:?} does not contain expected 24h time {:?} (from {:?})",
                        a_et, exp_24, e_et
                    ));
                }
            } else {
                return Err(format!("Failed to parse expected end_time {:?} to 24h format", e_et));
            }
        } else {
            return Err(format!("Expected end_time containing {:?}, got None", e_et));
        }
    }

    Ok(())
}

/// Matches date and time according to strict date/time rules against parsed.json
pub fn match_date_and_time(
    actual_start: Option<&str>,
    actual_end: Option<&str>,
    actual_is_all_day: bool,
    expected: &ParsedJsonEvent,
) -> bool {
    check_date_and_time(actual_start, actual_end, actual_is_all_day, expected).is_ok()
}

pub fn parse_actual_byday_set(rrule: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let re = Regex::new(r"(?i)\bBYDAY=([A-Z,]+)").unwrap();
    if let Some(caps) = re.captures(rrule) {
        if let Some(matched) = caps.get(1) {
            for day in matched.as_str().split(',') {
                let trimmed = day.trim().to_uppercase();
                if !trimmed.is_empty() {
                    set.insert(trimmed);
                }
            }
        }
    }
    set
}

pub fn expected_byday_set(expected: &ParsedJsonEvent) -> Option<BTreeSet<String>> {
    let is_repeating = expected.repeating.unwrap_or(false) || expected.days.is_some();
    if !is_repeating {
        return None;
    }
    let mut set = BTreeSet::new();
    if let Some(days) = &expected.days {
        for d in days {
            let token = match d.trim().to_lowercase().as_str() {
                "monday" | "mon" | "mo" => "MO",
                "tuesday" | "tue" | "tu" => "TU",
                "wednesday" | "wed" | "we" => "WE",
                "thursday" | "thu" | "th" => "TH",
                "friday" | "fri" | "fr" => "FR",
                "saturday" | "sat" | "sa" => "SA",
                "sunday" | "sun" | "su" => "SU",
                _ => "",
            };
            if !token.is_empty() {
                set.insert(token.to_string());
            }
        }
    }
    Some(set)
}

/// Checks recurrence rule checking frequency and by_day tokens
pub fn check_recurrence_rule(actual: Option<&str>, expected: &ParsedJsonEvent) -> Result<(), String> {
    let expected_days = expected_byday_set(expected);
    match (actual, expected_days) {
        (None, None) => Ok(()),
        (Some(a), None) => Err(format!("Expected no recurrence rule, but got: {:?}", a)),
        (None, Some(exp_days)) => Err(format!(
            "Expected weekly recurrence with BYDAY={:?}, but got None",
            exp_days
        )),
        (Some(a), Some(exp_days)) => {
            if !a.contains("FREQ=WEEKLY") {
                return Err(format!("Recurrence rule missing FREQ=WEEKLY: {:?}", a));
            }
            if !exp_days.is_empty() {
                let actual_days = parse_actual_byday_set(a);
                if actual_days != exp_days {
                    return Err(format!(
                        "BYDAY mismatch: expected {:?}, got {:?}",
                        exp_days, actual_days
                    ));
                }
            }
            Ok(())
        }
    }
}

/// Matches recurrence rule checking frequency and by_day tokens
pub fn match_recurrence_rule(actual: Option<&str>, expected: &ParsedJsonEvent) -> bool {
    check_recurrence_rule(actual, expected).is_ok()
}

#[derive(Debug, Clone)]
pub struct SampleMismatch {
    pub sample_path: String,
    pub event_index: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for SampleMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(idx) = self.event_index {
            write!(
                f,
                "[{}][Event #{}] {}",
                self.sample_path,
                idx + 1,
                self.message
            )
        } else {
            write!(f, "[{}] {}", self.sample_path, self.message)
        }
    }
}

pub fn validate_event_against_parsed_json(
    actual: &EventDetails,
    expected: &ParsedJsonEvent,
    sample_name: &str,
    event_idx: usize,
) -> Vec<SampleMismatch> {
    let mut mismatches = Vec::new();

    if !fuzzy_match_title(&actual.title, &expected.title) {
        mismatches.push(SampleMismatch {
            sample_path: sample_name.to_string(),
            event_index: Some(event_idx),
            message: format!(
                "Title mismatch. Actual: {:?}, Expected: {:?}",
                actual.title, expected.title
            ),
        });
    }

    if actual.is_all_day != expected.is_all_day {
        mismatches.push(SampleMismatch {
            sample_path: sample_name.to_string(),
            event_index: Some(event_idx),
            message: format!(
                "is_all_day mismatch. Actual: {:?}, Expected: {:?}",
                actual.is_all_day, expected.is_all_day
            ),
        });
    }

    if let Err(err) = check_date_and_time(
        actual.start_time.as_deref(),
        actual.end_time.as_deref(),
        actual.is_all_day,
        expected,
    ) {
        mismatches.push(SampleMismatch {
            sample_path: sample_name.to_string(),
            event_index: Some(event_idx),
            message: format!("Date/Time mismatch: {}", err),
        });
    }

    if let Err(err) = check_location(actual.location.as_deref(), expected.location.as_deref()) {
        mismatches.push(SampleMismatch {
            sample_path: sample_name.to_string(),
            event_index: Some(event_idx),
            message: format!("Location mismatch: {}", err),
        });
    }

    if let Err(err) = check_recurrence_rule(actual.recurrence_rule.as_deref(), expected) {
        mismatches.push(SampleMismatch {
            sample_path: sample_name.to_string(),
            event_index: Some(event_idx),
            message: format!("Recurrence rule mismatch: {}", err),
        });
    }

    mismatches
}

pub fn assert_event_matches_parsed_json(
    actual: &EventDetails,
    expected: &ParsedJsonEvent,
    sample_name: &str,
) {
    let mismatches = validate_event_against_parsed_json(actual, expected, sample_name, 0);
    if !mismatches.is_empty() {
        let msg = mismatches
            .into_iter()
            .map(|m| m.message)
            .collect::<Vec<_>>()
            .join("; ");
        panic!("[{}] Event validation failed: {}", sample_name, msg);
    }
}

pub fn get_test_model_path() -> Option<PathBuf> {
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
