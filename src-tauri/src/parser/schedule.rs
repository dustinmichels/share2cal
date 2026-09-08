use chrono::{Duration, TimeZone};
use crate::parser::datetime::{extract_times, get_weekday_date};
use crate::parser::regex::{
    COURSE_CODE_COLUMNAR_RE, COURSE_CODE_ROW_RE, COURSE_TYPE_RE, DAY_PATTERN_RE, FACULTY_RE,
    HEADER_RE, SECTION_NUM_RE, TIME_RANGE_RE, UNITS_RE,
};
use crate::parser::rrule::{parse_weekdays_to_byday, RecurrenceRule};
use crate::parser::{EventDetails, ReferenceContext};

/// Parses multi-event academic / class schedule tables
pub fn parse_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let row_events = parse_row_schedule_table_events(ocr_text, context);
    if row_events.len() >= 2 {
        return row_events;
    }
    parse_columnar_schedule_table_events(ocr_text, context)
}

/// Parses row-aligned class schedule tables
pub fn parse_row_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();

    let blacklist_prefixes = ["ROOM", "HALL", "DATE", "PAGE", "TERM", "YEAR", "BLDG", "STEP", "UNIT", "SUMMIT"];
    let matches: Vec<_> = COURSE_CODE_ROW_RE
        .find_iter(ocr_text)
        .filter(|m| {
            let first_word = m.as_str().split_whitespace().next().unwrap_or("").to_uppercase();
            !blacklist_prefixes.contains(&first_word.as_str())
        })
        .collect();
    let mut events = Vec::new();

    for (i, m) in matches.iter().enumerate() {
        let mut course_code = m.as_str().trim().to_string();
        let start_pos = m.end();
        let end_pos = if i + 1 < matches.len() {
            matches[i + 1].start()
        } else {
            ocr_text.len()
        };

        let block_raw = &ocr_text[start_pos..end_pos];

        // If course code doesn't contain (section_number), look for one in block_raw
        if !course_code.contains('(') {
            if let Some(sm) = SECTION_NUM_RE.find(block_raw) {
                course_code = format!("{} {}", course_code, sm.as_str().trim());
            }
        }

        let block_lines: Vec<&str> = block_raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if block_lines.is_empty() {
            continue;
        }

        // Find which line has the time range
        let time_line_idx_opt = block_lines.iter().position(|l| TIME_RANGE_RE.is_match(l));
        if time_line_idx_opt.is_none() {
            continue;
        }
        let time_line_idx = time_line_idx_opt.unwrap();
        let time_line = block_lines[time_line_idx];

        let time_match = TIME_RANGE_RE.find(time_line);
        let days_match = DAY_PATTERN_RE.find(time_line).or_else(|| DAY_PATTERN_RE.find(block_raw));

        if let Some(tm) = time_match {
            let time_str = tm.as_str();
            let times_opt = extract_times(time_str);

            let (first_day_opt, all_days_str, bydays) = if let Some(dm) = days_match {
                let days_str = dm.as_str().trim();
                let first_day = days_str.split(',').next().map(|s| s.trim()).unwrap_or(days_str);
                let parsed_bydays = parse_weekdays_to_byday(days_str);
                (Some(first_day), Some(days_str.to_string()), parsed_bydays)
            } else {
                (None, None, Vec::new())
            };

            let (start_time_iso, end_time_iso, is_all_day) = if let Some((start_time, end_time_opt)) = times_opt {
                let target_date = if let Some(day_str) = first_day_opt {
                    get_weekday_date(day_str, ref_dt).unwrap_or_else(|| ref_dt.date_naive())
                } else {
                    ref_dt.date_naive()
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

                (Some(start_dt.to_rfc3339()), end_dt_str, false)
            } else {
                (None, None, true)
            };

            let mut desc_parts_accum: Vec<String> = Vec::new();
            for prev_l in &block_lines[..time_line_idx] {
                let cleaned = clean_course_name(prev_l);
                if !cleaned.is_empty() {
                    desc_parts_accum.push(cleaned);
                }
            }

            let pre_time_text = if let Some(dm) = DAY_PATTERN_RE.find(time_line) {
                if dm.start() < time_line.len() {
                    &time_line[..dm.start()]
                } else {
                    &time_line[..tm.start()]
                }
            } else {
                &time_line[..tm.start()]
            };
            let cleaned_pre_time = clean_course_name(pre_time_text);
            if !cleaned_pre_time.is_empty() {
                desc_parts_accum.push(cleaned_pre_time);
            }

            // Extract faculty and units across the whole block
            let faculty_match = FACULTY_RE.find(block_raw).map(|f| f.as_str().trim().to_string());
            let units_match = UNITS_RE.find(block_raw).map(|u| u.as_str().trim().to_string());

            // Post-time text on time_line
            let post_time_on_line = &time_line[tm.end()..];
            let mut post_time_cleaned = post_time_on_line.trim().to_string();
            if let Some(f) = &faculty_match {
                post_time_cleaned = post_time_cleaned.replace(f, "");
            }
            if let Some(u) = &units_match {
                post_time_cleaned = post_time_cleaned.replace(u, "");
            }
            post_time_cleaned = post_time_cleaned.trim_matches(|c: char| c == ',' || c == '|' || c == '-' || c.is_whitespace()).trim().to_string();

            let mut location: Option<String> = if !post_time_cleaned.is_empty() {
                Some(post_time_cleaned)
            } else {
                None
            };

            // Inspect other lines after time_line_idx
            for &post_line in &block_lines[time_line_idx + 1..] {
                let mut line_str = post_line.trim().to_string();
                if let Some(f) = &faculty_match {
                    line_str = line_str.replace(f, "");
                }
                if let Some(u) = &units_match {
                    line_str = line_str.replace(u, "");
                }
                // Strip section number like (82454) if present
                line_str = SECTION_NUM_RE.replace_all(&line_str, "").trim().to_string();
                line_str = line_str.trim_matches(|c: char| c == ',' || c == '|' || c == '-' || c.is_whitespace()).trim().to_string();

                if line_str.is_empty() {
                    continue;
                }

                // Check if this line contains course type / description continuation
                if let Some(ct) = COURSE_TYPE_RE.find(&line_str) {
                    let desc_cont = line_str[..ct.end()].trim().to_string();
                    let loc_remainder = line_str[ct.end()..].trim_matches(|c: char| c == ',' || c == '|' || c == '-' || c.is_whitespace()).trim().to_string();
                    if !desc_cont.is_empty() {
                        desc_parts_accum.push(desc_cont);
                    }
                    if !loc_remainder.is_empty() {
                        location = Some(loc_remainder);
                    }
                } else if line_str.eq_ignore_ascii_case("online") || line_str.to_lowercase().contains("online") {
                    if line_str.eq_ignore_ascii_case("online") {
                        location = Some("Online".to_string());
                    } else {
                        let lower = line_str.to_lowercase();
                        if let Some(pos) = lower.find("online") {
                            let desc_cont = line_str[..pos].trim().trim_matches(|c: char| c == ',' || c == '|' || c == '-').trim().to_string();
                            if !desc_cont.is_empty() {
                                desc_parts_accum.push(desc_cont);
                            }
                            location = Some("Online".to_string());
                        } else {
                            location = Some(line_str);
                        }
                    }
                } else if location.is_none() {
                    location = Some(line_str);
                } else {
                    desc_parts_accum.push(line_str);
                }
            }

            let full_course_desc = desc_parts_accum.join(" ");
            let cleaned_desc = clean_course_name(&full_course_desc);
            let full_title = if !cleaned_desc.is_empty() {
                format!("{} {}", course_code, cleaned_desc)
            } else {
                course_code.clone()
            };

            let mut desc_parts = Vec::new();
            if let Some(days) = all_days_str {
                desc_parts.push(format!("Days: {}", days));
            }
            if let Some(fac) = faculty_match {
                desc_parts.push(format!("Faculty: {}", fac));
            }
            if let Some(un) = units_match {
                desc_parts.push(format!("Units: {}", un));
            }
            let description = if desc_parts.is_empty() {
                None
            } else {
                Some(desc_parts.join(" | "))
            };

            let recurrence_rule = if !bydays.is_empty() {
                Some(RecurrenceRule::new_weekly(bydays, None).to_rrule_string())
            } else {
                None
            };

            events.push(EventDetails {
                title: full_title,
                start_time: start_time_iso,
                end_time: end_time_iso,
                is_all_day,
                location,
                description,
                recurrence_rule,
                confidence: 0.95,
                source: "deterministic_schedule".to_string(),
            });
        }
    }

    events
}

/// Extracts descriptions from a columnar OCR text block between course codes and time slots
pub fn extract_columnar_descriptions(lines: &[&str], last_code_idx: usize, first_time_idx: usize, target_count: usize) -> Vec<String> {
    if first_time_idx <= last_code_idx + 1 || target_count == 0 {
        return Vec::new();
    }
    let candidate_slice = &lines[last_code_idx + 1..first_time_idx];
    let raw_desc_lines: Vec<&str> = candidate_slice
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !HEADER_RE.is_match(l))
        .collect();

    if raw_desc_lines.is_empty() {
        return Vec::new();
    }

    if raw_desc_lines.len() == target_count {
        return raw_desc_lines.into_iter().map(|s| s.to_string()).collect();
    }

    // Merge multi-line descriptions (lines ending with connectors, unclosed parentheses, or continuation keywords)
    let mut merged: Vec<String> = Vec::new();
    for l in raw_desc_lines {
        let is_continuation = merged.last().is_some_and(|last| {
            let last_trimmed = last.trim();
            last_trimmed.ends_with('-')
                || last_trimmed.ends_with('&')
                || last_trimmed.ends_with(',')
                || last_trimmed.ends_with('(')
                || (!last_trimmed.ends_with(')') && (l.starts_with("Images") || l.starts_with("Text") || l.starts_with('&') || l.starts_with('(') || l.starts_with("and ") || l.starts_with("with ")))
        });

        if is_continuation && !merged.is_empty() {
            let last = merged.last_mut().unwrap();
            last.push(' ');
            last.push_str(l);
        } else {
            merged.push(l.to_string());
        }
    }

    if merged.len() == target_count {
        return merged;
    }

    let mut result = Vec::new();
    for i in 0..target_count {
        if i < merged.len() {
            result.push(merged[i].clone());
        } else {
            result.push(String::new());
        }
    }
    result
}

/// Parses columnar-oriented schedule tables where columns are output sequentially
pub fn parse_columnar_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();

    let blacklist_prefixes = ["ROOM", "HALL", "DATE", "PAGE", "TERM", "YEAR", "BLDG", "STEP", "UNIT", "SUMMIT"];
    let course_codes: Vec<String> = COURSE_CODE_COLUMNAR_RE
        .find_iter(ocr_text)
        .filter(|m| {
            let first_word = m.as_str().split_whitespace().next().unwrap_or("").to_uppercase();
            !blacklist_prefixes.contains(&first_word.as_str())
        })
        .map(|m| m.as_str().trim().to_string())
        .collect();

    let lines: Vec<&str> = ocr_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let mut time_slots: Vec<(String, Option<String>, Option<String>)> = Vec::new(); // (time_str, days_str, location)

    for (idx, line) in lines.iter().enumerate() {
        if let Some(tm) = TIME_RANGE_RE.find(line) {
            let time_str = tm.as_str().to_string();
            let dm_opt = DAY_PATTERN_RE.find(line).map(|m| m.as_str().to_string());
            let next_line = if idx + 1 < lines.len() && !TIME_RANGE_RE.is_match(lines[idx + 1]) && !COURSE_CODE_COLUMNAR_RE.is_match(lines[idx + 1]) {
                Some(lines[idx + 1].trim().to_string())
            } else {
                None
            };
            time_slots.push((time_str, dm_opt, next_line));
        }
    }

    if course_codes.len() >= 2 && time_slots.len() == course_codes.len() {
        let last_code_line_idx = lines.iter().rposition(|l| COURSE_CODE_COLUMNAR_RE.is_match(l)).unwrap_or(0);
        let first_time_line_idx = lines.iter().position(|l| TIME_RANGE_RE.is_match(l)).unwrap_or(lines.len());
        let descriptions = extract_columnar_descriptions(&lines, last_code_line_idx, first_time_line_idx, course_codes.len());

        let mut events = Vec::new();
        for (i, code) in course_codes.iter().enumerate() {
            let (time_str, days_opt, next_line_opt) = &time_slots[i];
            let times_opt = extract_times(time_str);
            let first_day_opt = days_opt.as_ref().and_then(|d| d.split(',').next().map(|s| s.trim()));
            let bydays = days_opt.as_ref().map(|d| parse_weekdays_to_byday(d)).unwrap_or_default();

            let (start_time_iso, end_time_iso, is_all_day) = if let Some((start_time, end_time_opt)) = times_opt {
                let target_date = if let Some(day_str) = first_day_opt {
                    get_weekday_date(day_str, ref_dt).unwrap_or_else(|| ref_dt.date_naive())
                } else {
                    ref_dt.date_naive()
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

                (Some(start_dt.to_rfc3339()), end_dt_str, false)
            } else {
                (None, None, true)
            };

            let desc_str = if i < descriptions.len() { &descriptions[i] } else { "" };
            let full_title = if !desc_str.is_empty() {
                format!("{} {}", code, desc_str)
            } else {
                code.clone()
            };

            let recurrence_rule = if !bydays.is_empty() {
                Some(RecurrenceRule::new_weekly(bydays, None).to_rrule_string())
            } else {
                None
            };

            let description = days_opt.as_ref().map(|d| format!("Days: {}", d));

            events.push(EventDetails {
                title: full_title,
                start_time: start_time_iso,
                end_time: end_time_iso,
                is_all_day,
                location: next_line_opt.clone(),
                description,
                recurrence_rule,
                confidence: 0.90,
                source: "deterministic_schedule_columnar".to_string(),
            });
        }
        return events;
    }

    Vec::new()
}

/// Helper to clean extracted course descriptions from schedule blocks
pub fn clean_course_name(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    let header_prefixes = ["Class Description", "Description", "Course Name", "Title"];
    for hp in header_prefixes {
        if s.starts_with(hp) {
            s = s[hp.len()..].trim().to_string();
        }
    }
    // Trim leading punctuation (including stray closing paren from preceding section number or stray dots)
    s = s.trim_start_matches(|c: char| c == '-' || c == ':' || c == '|' || c == ')' || c == '.' || c == ',' || c == '\n' || c.is_whitespace()).to_string();
    // Trim trailing punctuation (colons, dashes, commas, dots, pipes - but preserve trailing ')' if balanced)
    s = s.trim_end_matches(|c: char| c == '-' || c == ':' || c == '|' || c == ',' || c == '.' || c == '\n' || c.is_whitespace()).to_string();
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    s
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
    fn test_parse_fall_2026_class_schedule_6_events() {
        let ocr_text = r#"My Fall Term 2026 Class Schedule
Class Description Days, Times, & Locations Faculty Units Status
CEE 0154-03 (80513) Principles Epidemiology (Lecture) Mo, We 3:00PM - 4:15PM Anderson Wing TTC, Room 306 L. Abrams 3.00
CS 0150-09 (84779) Special Topics - Analysis Mthds Images, Text & (Lecture) Fr 2:00PM - 4:30PM Online J. Skripchuk 3.00
CSHD 0166-01 (82454) Children's Play (Lecture) Th 1:30PM - 4:00PM Eliot-Pearson, Room 157 W. Scarlett 3.00
CSHD 0167-01 (80739) Children & Media (Lecture) Fr 9:00AM - 11:30AM Eaton Hall, 201 J. Dobrow 3.00
UEP 0254-01 (81300) Quantitative Reasoning (Lecture) Tu, Th 9:00AM - 10:15AM Joyce Cummings Center, 302 S. Shamsuddin 3.00
UEP 0262-01 (82571) Solidarity Economy Movements (Seminar) Tu 12:00PM - 2:30PM Bromfield-Pearson, Room 006 P. Loh 3.00"#;

        let ctx = sample_reference_context(); // Reference is Sunday 2026-09-06
        let events = parse_schedule_table_events(ocr_text, &ctx);

        assert_eq!(events.len(), 6, "Expected exactly 6 parsed events from class schedule, got {}", events.len());

        // 1. CEE 0154-03
        assert!(events[0].title.contains("CEE 0154-03") && events[0].title.contains("Principles Epidemiology"));
        assert!(events[0].start_time.as_ref().unwrap().starts_with("2026-09-07T15:00:00"));
        assert!(events[0].end_time.as_ref().unwrap().starts_with("2026-09-07T16:15:00"));
        assert!(events[0].location.as_deref().unwrap().contains("Anderson Wing TTC"));
        assert!(events[0].description.as_deref().unwrap().contains("L. Abrams"));
        assert_eq!(events[0].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));

        // 2. CS 0150-09
        assert!(events[1].title.contains("CS 0150-09") && events[1].title.contains("Special Topics"));
        assert!(events[1].start_time.as_ref().unwrap().starts_with("2026-09-11T14:00:00"));
        assert!(events[1].end_time.as_ref().unwrap().starts_with("2026-09-11T16:30:00"));
        assert_eq!(events[1].location.as_deref(), Some("Online"));
        assert!(events[1].description.as_deref().unwrap().contains("J. Skripchuk"));
        assert_eq!(events[1].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=FR"));

        // 3. CSHD 0166-01
        assert!(events[2].title.contains("CSHD 0166-01") && events[2].title.contains("Children's Play"));
        assert!(events[2].start_time.as_ref().unwrap().starts_with("2026-09-10T13:30:00"));
        assert!(events[2].end_time.as_ref().unwrap().starts_with("2026-09-10T16:00:00"));
        assert!(events[2].location.as_deref().unwrap().contains("Eliot-Pearson"));
        assert!(events[2].description.as_deref().unwrap().contains("W. Scarlett"));
        assert_eq!(events[2].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TH"));

        // 4. CSHD 0167-01
        assert!(events[3].title.contains("CSHD 0167-01") && events[3].title.contains("Children & Media"));
        assert!(events[3].start_time.as_ref().unwrap().starts_with("2026-09-11T09:00:00"));
        assert!(events[3].end_time.as_ref().unwrap().starts_with("2026-09-11T11:30:00"));
        assert!(events[3].location.as_deref().unwrap().contains("Eaton Hall"));
        assert!(events[3].description.as_deref().unwrap().contains("J. Dobrow"));
        assert_eq!(events[3].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=FR"));

        // 5. UEP 0254-01
        assert!(events[4].title.contains("UEP 0254-01") && events[4].title.contains("Quantitative Reasoning"));
        assert!(events[4].start_time.as_ref().unwrap().starts_with("2026-09-08T09:00:00"));
        assert!(events[4].end_time.as_ref().unwrap().starts_with("2026-09-08T10:15:00"));
        assert!(events[4].location.as_deref().unwrap().contains("Joyce Cummings Center"));
        assert!(events[4].description.as_deref().unwrap().contains("S. Shamsuddin"));
        assert_eq!(events[4].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TU,TH"));

        // 6. UEP 0262-01
        assert!(events[5].title.contains("UEP 0262-01") && events[5].title.contains("Solidarity Economy"));
        assert!(events[5].start_time.as_ref().unwrap().starts_with("2026-09-08T12:00:00"));
        assert!(events[5].end_time.as_ref().unwrap().starts_with("2026-09-08T14:30:00"));
        assert!(events[5].location.as_deref().unwrap().contains("Bromfield-Pearson"));
        assert!(events[5].description.as_deref().unwrap().contains("P. Loh"));
        assert_eq!(events[5].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TU"));
    }

    #[test]
    fn test_parse_columnar_fall_2026_class_schedule_all_titles() {
        let columnar_ocr_text = r#"My Fall Term 2026 Class Schedule
Class
CEE 0154-03 (80513)
CS 0150-09 (84779)
CSHD 0166-01 (82454)
CSHD 0167-01 (80739)
UEP 0254-01 (81300)
UEP 0262-01 (82571)
Description
Principles Epidemiology (Lecture)
Special Topics - Analysis Mthds Images, Text & (Lecture)
Children's Play (Lecture)
Children & Media (Lecture)
Quantitative Reasoning (Lecture)
Solidarity Economy Movements (Seminar)
Days, Times, & Locations
Mo, We 3:00PM - 4:15PM
Anderson Wing TTC, Room 306
Fr 2:00PM - 4:30PM
Online
Th 1:30PM - 4:00PM
Eliot-Pearson, Room 157
Fr 9:00AM - 11:30AM
Eaton Hall, 201
Tu, Th 9:00AM - 10:15AM
Joyce Cummings Center, 302
Tu 12:00PM - 2:30PM
Bromfield-Pearson, Room 006
Faculty
L. Abrams
J. Skripchuk
W. Scarlett
J. Dobrow
S. Shamsuddin
P. Loh
Units
3.00
3.00
3.00
3.00
3.00
3.00
Status"#;

        let ctx = sample_reference_context();
        let events = parse_schedule_table_events(columnar_ocr_text, &ctx);

        assert_eq!(events.len(), 6, "Expected exactly 6 parsed events from columnar schedule, got {}", events.len());

        assert!(events[0].title.contains("CEE 0154-03") && events[0].title.contains("Principles Epidemiology"));
        assert_eq!(events[0].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));

        assert!(events[1].title.contains("CS 0150-09") && events[1].title.contains("Special Topics"));
        assert_eq!(events[1].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=FR"));

        assert!(events[2].title.contains("CSHD 0166-01") && events[2].title.contains("Children's Play"));
        assert_eq!(events[2].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TH"));

        assert!(events[3].title.contains("CSHD 0167-01") && events[3].title.contains("Children & Media"));
        assert_eq!(events[3].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=FR"));

        assert!(events[4].title.contains("UEP 0254-01") && events[4].title.contains("Quantitative Reasoning"));
        assert_eq!(events[4].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TU,TH"));

        assert!(events[5].title.contains("UEP 0262-01") && events[5].title.contains("Solidarity Economy Movements"));
        assert_eq!(events[5].recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TU"));
    }

    #[test]
    fn test_parse_multi_line_class_schedule() {
        let text = r#"My Fall Term 2026 Class Schedule
Class Description Days, Times, & Locations Faculty Units Status
CEE 0154-03 (80513)
Principles Epidemiology (Lecture)
Mo, We 3:00PM - 4:15PM
Anderson Wing TTC, Room 306
L. Abrams
3.00
CS 0150-09 (84779)
Special Topics - Analysis Mthds Images, Text & (Lecture)
Fr 2:00PM - 4:30PM
Online
J. Skripchuk
3.00"#;

        let ctx = sample_reference_context();
        let events = parse_schedule_table_events(text, &ctx);

        assert_eq!(events.len(), 2);
        assert!(events[0].title.contains("CEE 0154-03"));
        assert!(events[0].location.as_deref().unwrap().contains("Anderson Wing"));
        assert!(events[1].title.contains("CS 0150-09"));
        assert_eq!(events[1].location.as_deref(), Some("Online"));
    }
}
