use chrono::{Duration, TimeZone};
use crate::parser::datetime::{extract_date, extract_times, get_weekday_date};
use crate::parser::regex::{DAY_PATTERN_RE, PAREN_RE, TIME_RANGE_RE};
use crate::parser::rrule::{parse_weekdays_to_byday, RecurrenceRule};
use crate::parser::{EventDetails, ReferenceContext};

/// Parses agenda / multi-event lines with distinct time slots
pub fn parse_agenda_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();
    let mut current_date = extract_date(ocr_text, ref_dt).unwrap_or_else(|| ref_dt.date_naive());

    let lines: Vec<&str> = ocr_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    let mut events = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        // If the line contains a date heading, update the current tracking date
        if let Some(d) = extract_date(line, ref_dt) {
            current_date = d;
        }

        if let Some(caps) = TIME_RANGE_RE.captures(line) {
            let time_str = caps.get(0).unwrap().as_str();
            let times_opt = extract_times(time_str);

            if let Some((start_time, end_time_opt)) = times_opt {
                let days_opt = DAY_PATTERN_RE.find(line).and_then(|m| {
                    let s = m.as_str().trim();
                    let bydays = parse_weekdays_to_byday(s);
                    if bydays.is_empty() {
                        return None;
                    }
                    let lower = s.to_lowercase();
                    let is_ambiguous = (lower == "we" || lower == "sun" || lower == "sat" || lower == "mon" || lower == "th")
                        && line.split_whitespace().count() > 4
                        && bydays.len() == 1;
                    if is_ambiguous && !line.to_lowercase().contains("every") && !line.to_lowercase().contains("weekly") {
                        return None;
                    }
                    Some(s.to_string())
                });
                let target_date = if let Some(line_date) = extract_date(line, ref_dt) {
                    line_date
                } else if let Some(d) = &days_opt {
                    let first_d = d.split(',').next().map(|s| s.trim()).unwrap_or(d.as_str());
                    get_weekday_date(first_d, ref_dt).unwrap_or(current_date)
                } else {
                    current_date
                };
                let start_dt = offset.from_local_datetime(&target_date.and_time(start_time)).unwrap();
                let end_dt_str = if let Some(end_time) = end_time_opt {
                    let end_date = if end_time < start_time {
                        target_date + Duration::days(1)
                    } else {
                        target_date
                    };
                    let end_dt = offset.from_local_datetime(&end_date.and_time(end_time)).unwrap();
                    Some(end_dt.to_rfc3339())
                } else {
                    let end_dt = start_dt + Duration::hours(1);
                    Some(end_dt.to_rfc3339())
                };

                // Remove time string from line to extract title and location
                let mut rest = line.replace(time_str, "");
                if let Some(d) = &days_opt {
                    rest = rest.replace(d, "");
                }
                let cleaned_rest = rest.trim().trim_matches(|c: char| c == ':' || c == '-' || c == '|' || c == '*' || c == ',').trim();

                // Extract location if in parentheses e.g. "(Room 204)" or "(Main Auditorium)"
                let (mut title, location) = if let Some(p_cap) = PAREN_RE.captures(cleaned_rest) {
                    let loc = p_cap.get(1).unwrap().as_str().trim().to_string();
                    let t = PAREN_RE.replace(cleaned_rest, "").trim().to_string();
                    (t, Some(loc))
                } else {
                    (cleaned_rest.to_string(), None)
                };

                let is_weekday_only = title.is_empty() || title.split(',').all(|w| {
                    let tw = w.trim().to_lowercase();
                    matches!(tw.as_str(), "mo" | "tu" | "we" | "th" | "fr" | "sa" | "su" | "mon" | "tue" | "tues" | "wed" | "thu" | "thur" | "thurs" | "fri" | "sat" | "sun" | "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday")
                });

                if is_weekday_only && idx > 0 && !TIME_RANGE_RE.is_match(lines[idx - 1]) {
                    title = lines[idx - 1].trim().to_string();
                }
                let bydays = days_opt.as_ref().map(|d| parse_weekdays_to_byday(d)).unwrap_or_default();
                let line_lower = line.to_lowercase();
                let has_explicit_repeat = line_lower.contains("every")
                    || line_lower.contains("weekly")
                    || line_lower.contains("repeats")
                    || line_lower.contains("recurring")
                    || bydays.len() >= 2;
                let recurrence_rule = if has_explicit_repeat && !bydays.is_empty() {
                    Some(RecurrenceRule::new_weekly(bydays, None).to_rrule_string())
                } else {
                    None
                };
                if !title.is_empty() {
                    events.push(EventDetails {
                        title,
                        start_time: Some(start_dt.to_rfc3339()),
                        end_time: end_dt_str,
                        is_all_day: false,
                        location,
                        description: None,
                        recurrence_rule,
                        url: None,
                        confidence: 0.90,
                        source: "deterministic_agenda".to_string(),
                    });
                }
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reference_context() -> ReferenceContext {
        ReferenceContext {
            reference_time: Some("2026-09-06T10:57:00-04:00".to_string()),
            timezone_offset_minutes: Some(-240), // EDT
        }
    }

    #[test]
    fn test_parse_conference_agenda_multiple_events() {
        let text = r#"AI & Cloud Summit 2026
Friday, October 16, 2026
9:00 AM - 10:00 AM: Keynote Speech (Main Auditorium)
10:30 AM - 12:00 PM: Machine Learning Workshop (Room 204)
1:00 PM - 2:30 PM: WebAssembly Panel (Hall B)
3:00 PM - 4:30 PM: Networking & Drinks (Rooftop Lounge)"#;
        let ctx = sample_reference_context();
        let events = parse_agenda_events(text, &ctx);

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].title, "Keynote Speech");
        assert_eq!(events[0].start_time.as_deref(), Some("2026-10-16T09:00:00-04:00"));
        assert_eq!(events[0].end_time.as_deref(), Some("2026-10-16T10:00:00-04:00"));
        assert_eq!(events[0].location.as_deref(), Some("Main Auditorium"));
        assert!(events[0].recurrence_rule.is_none(), "One-off conference event should not have recurrence");

        assert_eq!(events[1].title, "Machine Learning Workshop");
        assert_eq!(events[1].location.as_deref(), Some("Room 204"));
        assert!(events[1].recurrence_rule.is_none());

        assert_eq!(events[2].title, "WebAssembly Panel");
        assert_eq!(events[2].location.as_deref(), Some("Hall B"));
        assert!(events[2].recurrence_rule.is_none());

        assert_eq!(events[3].title, "Networking & Drinks");
        assert_eq!(events[3].location.as_deref(), Some("Rooftop Lounge"));
        assert!(events[3].recurrence_rule.is_none());
    }

    #[test]
    fn test_parse_multi_day_conference_non_recurring() {
        let text = r#"Developer Summit 2026
Thursday, October 15, 2026
9:00 AM - 10:30 AM: Opening Keynote (Auditorium)
2:00 PM - 4:00 PM: Rust Deep Dive (Room 101)
Friday, October 16, 2026
10:00 AM - 11:30 AM: Closing Remarks (Auditorium)"#;

        let ctx = sample_reference_context();
        let events = parse_agenda_events(text, &ctx);

        assert_eq!(events.len(), 3);

        // Day 1: Thursday, Oct 15
        assert_eq!(events[0].title, "Opening Keynote");
        assert!(events[0].start_time.as_ref().unwrap().starts_with("2026-10-15T09:00:00"));
        assert!(events[0].end_time.as_ref().unwrap().starts_with("2026-10-15T10:30:00"));
        assert!(events[0].recurrence_rule.is_none());

        assert_eq!(events[1].title, "Rust Deep Dive");
        assert!(events[1].start_time.as_ref().unwrap().starts_with("2026-10-15T14:00:00"));
        assert!(events[1].end_time.as_ref().unwrap().starts_with("2026-10-15T16:00:00"));
        assert!(events[1].recurrence_rule.is_none());

        // Day 2: Friday, Oct 16
        assert_eq!(events[2].title, "Closing Remarks");
        assert!(events[2].start_time.as_ref().unwrap().starts_with("2026-10-16T10:00:00"));
        assert!(events[2].end_time.as_ref().unwrap().starts_with("2026-10-16T11:30:00"));
        assert!(events[2].recurrence_rule.is_none());
    }
}
