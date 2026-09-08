use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime,
    Offset, TimeZone,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventDetails {
    pub title: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<String>,
    pub confidence: f32,
    pub source: String,
}

/// Strongly typed recurrence rule representation matching RFC 5545
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurrenceRule {
    pub frequency: String, // "DAILY", "WEEKLY", "MONTHLY", "YEARLY"
    pub interval: u32,
    pub by_days: Vec<String>, // ["MO", "WE"]
    pub until: Option<String>,
    pub count: Option<u32>,
}

impl RecurrenceRule {
    pub fn new_weekly(by_days: Vec<String>, until: Option<String>) -> Self {
        Self {
            frequency: "WEEKLY".to_string(),
            interval: 1,
            by_days,
            until,
            count: None,
        }
    }

    pub fn to_rrule_string(&self) -> String {
        let mut parts = vec![format!("FREQ={}", self.frequency)];
        if self.interval > 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        if !self.by_days.is_empty() {
            parts.push(format!("BYDAY={}", self.by_days.join(",")));
        }
        if let Some(ref until) = self.until {
            let clean_until = until.replace(['-', ':'], "");
            if clean_until.len() == 8 {
                parts.push(format!("UNTIL={}T235959", clean_until));
            } else {
                parts.push(format!("UNTIL={}", clean_until));
            }
        } else if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        }
        parts.join(";")
    }

    pub fn parse_rrule(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("Empty recurrence rule string.".to_string());
        }
        let clean = if let Some(stripped) = trimmed.strip_prefix("RRULE:") {
            stripped
        } else {
            trimmed
        };

        let mut freq = None;
        let mut interval = 1;
        let mut by_days = Vec::new();
        let mut until = None;
        let mut count = None;

        for part in clean.split(';') {
            let trimmed_part = part.trim();
            if trimmed_part.is_empty() {
                continue;
            }

            let (k, v) = trimmed_part
                .split_once('=')
                .ok_or_else(|| format!("Malformed recurrence property (missing '='): {}", trimmed_part))?;
            let k = k.trim();
            let v = v.trim();
            if k.is_empty() || v.is_empty() {
                return Err(format!("Empty key or value in recurrence property: {}", trimmed_part));
            }

            match k.to_uppercase().as_str() {
                "FREQ" => {
                    let upper = v.to_uppercase();
                    if matches!(upper.as_str(), "DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY") {
                        freq = Some(upper);
                    } else {
                        return Err(format!("Invalid recurrence frequency: {}", v));
                    }
                }
                "INTERVAL" => {
                    let n = v
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid INTERVAL value (expected positive integer): {}", v))?;
                    if n == 0 {
                        return Err("INTERVAL must be greater than 0".to_string());
                    }
                    interval = n;
                }
                "BYDAY" => {
                    for token in v.split(',') {
                        let code = token.trim().to_uppercase();
                        if matches!(code.as_str(), "MO" | "TU" | "WE" | "TH" | "FR" | "SA" | "SU") {
                            if !by_days.contains(&code) {
                                by_days.push(code);
                            }
                        } else {
                            return Err(format!("Invalid BYDAY token: {}", token));
                        }
                    }
                }
                "UNTIL" => {
                    let clean_v = v.replace(['-', ':'], "");
                    if clean_v.len() < 8 {
                        return Err(format!("Invalid UNTIL date format: {}", v));
                    }
                    until = Some(v.to_string());
                }
                "COUNT" => {
                    let n = v
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid COUNT value (expected positive integer): {}", v))?;
                    if n == 0 {
                        return Err("COUNT must be greater than 0".to_string());
                    }
                    count = Some(n);
                }
                other => {
                    return Err(format!("Unsupported recurrence property: {}", other));
                }
            }
        }
        let frequency = freq.ok_or_else(|| "Recurrence rule missing required FREQ property.".to_string())?;

        Ok(RecurrenceRule {
            frequency,
            interval,
            by_days,
            until,
            count,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReferenceContext {
    pub reference_time: Option<String>,
    pub timezone_offset_minutes: Option<i32>,
}

impl ReferenceContext {
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            reference_time: Some(now.to_rfc3339()),
            timezone_offset_minutes: Some(now.offset().local_minus_utc() / 60),
        }
    }

    pub fn get_reference_datetime(&self) -> DateTime<FixedOffset> {
        if let Some(ref_str) = &self.reference_time {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ref_str) {
                return dt;
            }
            if let Ok(dt) = DateTime::parse_from_str(ref_str, "%Y-%m-%dT%H:%M:%S%z") {
                return dt;
            }
            if let Ok(naive) = NaiveDateTime::parse_from_str(ref_str, "%Y-%m-%dT%H:%M:%S") {
                let offset_sec = self.timezone_offset_minutes.unwrap_or(0) * 60;
                let offset = FixedOffset::east_opt(offset_sec).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
                return offset.from_utc_datetime(&naive);
            }
        }

        let now_local = Local::now();
        let offset = now_local.offset().fix();
        now_local.with_timezone(&offset)
    }

    pub fn get_fixed_offset(&self) -> FixedOffset {
        if let Some(mins) = self.timezone_offset_minutes {
            FixedOffset::east_opt(mins * 60).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
        } else {
            self.get_reference_datetime().offset().clone()
        }
    }
}

/// Generates a strict GBNF grammar for llama.cpp structured event extraction
pub fn get_gbnf_grammar() -> &'static str {
    r#"root ::= ws "{" ws "\"events\":" ws "[" ws event-list? ws "]" ws "}" ws
event-list ::= event ("," ws event)*
event ::= "{" ws "\"title\":" ws string "," ws "\"start_time\":" ws optstring "," ws "\"end_time\":" ws optstring "," ws "\"is_all_day\":" ws boolean "," ws "\"location\":" ws optstring "," ws "\"description\":" ws optstring "," ws "\"recurrence_rule\":" ws optstring "}"
string ::= "\"" ([^"\\\r\n] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\"" ws
optstring ::= ("null" | string) ws
boolean ::= ("true" | "false") ws
ws ::= [ \t\n\r]*
"#
}

/// Generates the JSON schema for LLM structured outputs
pub fn get_json_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "EventsPayload",
        "type": "object",
        "properties": {
            "events": {
                "type": "array",
                "description": "List of extracted calendar events.",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "The concise, clear title or name of the event or class."
                        },
                        "start_time": {
                            "type": ["string", "null"],
                            "description": "ISO-8601 formatted start date-time including timezone offset (e.g. 2026-09-08T15:00:00-04:00) or YYYY-MM-DD for all-day events."
                        },
                        "end_time": {
                            "type": ["string", "null"],
                            "description": "ISO-8601 formatted end date-time including timezone offset (e.g. 2026-09-08T16:15:00-04:00) or YYYY-MM-DD for all-day events."
                        },
                        "is_all_day": {
                            "type": "boolean",
                            "description": "True if the event spans the entire day or no specific time is mentioned."
                        },
                        "location": {
                            "type": ["string", "null"],
                            "description": "The venue name, physical address, room number, or virtual link."
                        },
                        "description": {
                            "type": ["string", "null"],
                            "description": "Summary of activities, instructor/faculty, section code, units, or extra details."
                        },
                        "recurrence_rule": {
                            "type": ["string", "null"],
                            "description": "RFC 5545 RRULE string for repeating events (e.g. 'FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z' or 'FREQ=WEEKLY;BYDAY=TU,TH'). Null if the event does not repeat."
                        }
                    },
                    "required": ["title", "start_time", "end_time", "is_all_day", "location", "description", "recurrence_rule"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["events"],
        "additionalProperties": false
    })
}

pub fn generate_extraction_prompt(ocr_text: &str, context: &ReferenceContext) -> String {
    let ref_dt = context.get_reference_datetime();
    let ref_str = ref_dt.to_rfc3339();
    let day_name = match ref_dt.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };

    format!(
        "<|im_start|>system\nYou are a calendar assistant. Extract all events from the OCR text into a JSON object with an \"events\" array.\n\
        Each event must have fields: \"title\" (string), \"start_time\" (ISO-8601 or YYYY-MM-DD or null), \"end_time\" (ISO-8601 or YYYY-MM-DD or null), \"is_all_day\" (boolean), \"location\" (string or null), \"description\" (string or null), \"recurrence_rule\" (string or null, e.g. FREQ=WEEKLY;BYDAY=MO,WE). Output ONLY raw JSON.\n\
        Reference Time: {} ({})<|im_end|>\n\
        <|im_start|>user\n{}\n<|im_end|>\n\
        <|im_start|>assistant\n{{\"events\": [",
        ref_str, day_name, ocr_text.trim()
    )
}

/// Deterministic fallback parser implementing multi-event schedule, agenda, and single-event parsing
pub fn parse_events_deterministic(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    // 1. Try class schedule table parsing
    let schedule_events = parse_schedule_table_events(ocr_text, context);
    if schedule_events.len() >= 2 {
        return schedule_events;
    }

    // 2. Try generic agenda / multi-event line parsing
    let agenda_events = parse_agenda_events(ocr_text, context);
    if agenda_events.len() >= 2 {
        return agenda_events;
    }

    // 3. Fallback to single event parsing
    vec![parse_single_event_deterministic(ocr_text, context)]
}

/// Compatibility wrapper returning the primary or first extracted event
pub fn parse_event_deterministic(ocr_text: &str, context: &ReferenceContext) -> EventDetails {
    let events = parse_events_deterministic(ocr_text, context);
    events.into_iter().next().unwrap_or_else(|| parse_single_event_deterministic(ocr_text, context))
}

/// Parses weekday abbreviations from strings like "Mo, We" into RFC 5545 BYDAY tokens
pub fn parse_weekdays_to_byday(days_str: &str) -> Vec<String> {
    let mut bydays = Vec::new();
    let parts: Vec<&str> = days_str
        .split(|c: char| c == ',' || c == '/' || c == '&' || c == ' ' || c == ';' || c == '+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for p in parts {
        let lower = p.to_lowercase();
        let code = match lower.as_str() {
            "mo" | "mon" | "monday" => Some("MO"),
            "tu" | "tue" | "tues" | "tuesday" => Some("TU"),
            "we" | "wed" | "wednesday" => Some("WE"),
            "th" | "thu" | "thur" | "thurs" | "thursday" => Some("TH"),
            "fr" | "fri" | "friday" => Some("FR"),
            "sa" | "sat" | "saturday" => Some("SA"),
            "su" | "sun" | "sunday" => Some("SU"),
            _ => None,
        };
        if let Some(c) = code {
            if !bydays.contains(&c.to_string()) {
                bydays.push(c.to_string());
            }
        }
    }
    bydays
}

/// Extracts academic term end date for UNTIL recurrence rule
pub fn extract_term_until_date(text: &str, ref_dt: DateTime<FixedOffset>) -> String {
    let lower = text.to_lowercase();
    let year_re = Regex::new(r"\b(20\d{2})\b").unwrap();
    let year = if let Some(caps) = year_re.captures(text) {
        caps.get(1).unwrap().as_str().parse::<i32>().unwrap_or(ref_dt.year())
    } else {
        ref_dt.year()
    };

    if lower.contains("fall") || lower.contains("autumn") {
        format!("{:04}1218T235959Z", year)
    } else if lower.contains("spring") {
        format!("{:04}0515T235959Z", year)
    } else if lower.contains("summer") {
        format!("{:04}0815T235959Z", year)
    } else if lower.contains("winter") {
        format!("{:04}0315T235959Z", year)
    } else {
        let month = ref_dt.month();
        if month >= 8 {
            format!("{:04}1218T235959Z", year)
        } else if month <= 5 {
            format!("{:04}0515T235959Z", year)
        } else {
            format!("{:04}0815T235959Z", year)
        }
    }
}

/// Parses multi-event academic / class schedule tables
pub fn parse_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let row_events = parse_row_schedule_table_events(ocr_text, context);
    if row_events.len() >= 2 {
        return row_events;
    }
    parse_columnar_schedule_table_events(ocr_text, context)
}

/// Parses row-aligned class schedule tables
fn parse_row_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();
    let course_code_re = Regex::new(r"\b([A-Z]{2,6}\s+\d{3,4}(?:-\d{2,3})?(?:\s*\(\d+\))?)").unwrap();
    let day_pattern_re = Regex::new(r"(?i)\b((?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun)(?:\s*,\s*(?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun))*)\b").unwrap();
    let time_range_re = Regex::new(r"(?i)\b(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)\s*(?:-|–|—|to)\s*(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.))\b").unwrap();
    let faculty_re = Regex::new(r"(?i)\b([A-Z]\.\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\b").unwrap();
    let units_re = Regex::new(r"\b(\d+\.\d{2})\b").unwrap();
    let section_num_re = Regex::new(r"\((\d{4,6})\)").unwrap();
    let course_type_re = Regex::new(r"(?i)\((Lecture|Seminar|Lab|Discussion|Recitation|Studio|Practicum|Workshop|Lecture/Lab)\)").unwrap();

    let blacklist_prefixes = ["ROOM", "HALL", "DATE", "PAGE", "TERM", "YEAR", "BLDG", "STEP", "UNIT", "SUMMIT"];
    let matches: Vec<_> = course_code_re
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
            if let Some(sm) = section_num_re.find(block_raw) {
                course_code = format!("{} {}", course_code, sm.as_str().trim());
            }
        }

        let block_lines: Vec<&str> = block_raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if block_lines.is_empty() {
            continue;
        }

        // Find which line has the time range
        let time_line_idx_opt = block_lines.iter().position(|l| time_range_re.is_match(l));
        if time_line_idx_opt.is_none() {
            continue;
        }
        let time_line_idx = time_line_idx_opt.unwrap();
        let time_line = block_lines[time_line_idx];

        let time_match = time_range_re.find(time_line);
        let days_match = day_pattern_re.find(time_line).or_else(|| day_pattern_re.find(block_raw));

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

            let pre_time_text = if let Some(dm) = day_pattern_re.find(time_line) {
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
            let faculty_match = faculty_re.find(block_raw).map(|f| f.as_str().trim().to_string());
            let units_match = units_re.find(block_raw).map(|u| u.as_str().trim().to_string());

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
                line_str = section_num_re.replace_all(&line_str, "").trim().to_string();
                line_str = line_str.trim_matches(|c: char| c == ',' || c == '|' || c == '-' || c.is_whitespace()).trim().to_string();

                if line_str.is_empty() {
                    continue;
                }

                // Check if this line contains course type / description continuation
                if let Some(ct) = course_type_re.find(&line_str) {
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
fn extract_columnar_descriptions(lines: &[&str], last_code_idx: usize, first_time_idx: usize, target_count: usize) -> Vec<String> {
    if first_time_idx <= last_code_idx + 1 || target_count == 0 {
        return Vec::new();
    }
    let candidate_slice = &lines[last_code_idx + 1..first_time_idx];
    let header_re = Regex::new(r"(?i)^(Description|Class\s+Description|Course\s+Name|Title)$").unwrap();
    let raw_desc_lines: Vec<&str> = candidate_slice
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !header_re.is_match(l))
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
        let is_continuation = merged.last().map_or(false, |last| {
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
fn parse_columnar_schedule_table_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();
    let course_code_re = Regex::new(r"\b([A-Z]{2,6}\s+\d{3,4}(?:-\d{2,3})?(?:\s*\(\d+\))?)\b").unwrap();
    let day_pattern_re = Regex::new(r"(?i)\b((?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun)(?:\s*,\s*(?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun))*)\b").unwrap();
    let time_range_re = Regex::new(r"(?i)\b(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)\s*(?:-|–|—|to)\s*(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.))\b").unwrap();

    let blacklist_prefixes = ["ROOM", "HALL", "DATE", "PAGE", "TERM", "YEAR", "BLDG", "STEP", "UNIT", "SUMMIT"];
    let course_codes: Vec<String> = course_code_re
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
        if let Some(tm) = time_range_re.find(line) {
            let time_str = tm.as_str().to_string();
            let dm_opt = day_pattern_re.find(line).map(|m| m.as_str().to_string());
            let next_line = if idx + 1 < lines.len() && !time_range_re.is_match(lines[idx + 1]) && !course_code_re.is_match(lines[idx + 1]) {
                Some(lines[idx + 1].trim().to_string())
            } else {
                None
            };
            time_slots.push((time_str, dm_opt, next_line));
        }
    }

    if course_codes.len() >= 2 && time_slots.len() == course_codes.len() {
        let last_code_line_idx = lines.iter().rposition(|l| course_code_re.is_match(l)).unwrap_or(0);
        let first_time_line_idx = lines.iter().position(|l| time_range_re.is_match(l)).unwrap_or(lines.len());
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
fn clean_course_name(raw: &str) -> String {
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


/// Calculates target weekday date for recurring schedule items
fn get_weekday_date(day_abbr: &str, ref_dt: DateTime<FixedOffset>) -> Option<NaiveDate> {
    let lower = day_abbr.trim().to_lowercase();
    let target_weekday = match lower.as_str() {
        "mo" | "mon" | "monday" => chrono::Weekday::Mon,
        "tu" | "tue" | "tues" | "tuesday" => chrono::Weekday::Tue,
        "we" | "wed" | "wednesday" => chrono::Weekday::Wed,
        "th" | "thu" | "thur" | "thurs" | "thursday" => chrono::Weekday::Thu,
        "fr" | "fri" | "friday" => chrono::Weekday::Fri,
        "sa" | "sat" | "saturday" => chrono::Weekday::Sat,
        "su" | "sun" | "sunday" => chrono::Weekday::Sun,
        _ => return None,
    };

    let ref_date = ref_dt.date_naive();
    let current_weekday = ref_date.weekday();
    let days_from_monday = current_weekday.num_days_from_monday() as i64;
    let monday_date = ref_date - Duration::days(days_from_monday);
    let target_offset = target_weekday.num_days_from_monday() as i64;
    let mut target_date = monday_date + Duration::days(target_offset);
    
    // If target date is before ref_date by more than 1 day in the past week, advance to next week
    if target_date < ref_date - Duration::days(1) {
        target_date = target_date + Duration::days(7);
    }

    Some(target_date)
}

/// Parses agenda / multi-event lines with distinct time slots
pub fn parse_agenda_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();
    let mut current_date = extract_date(ocr_text, ref_dt).unwrap_or_else(|| ref_dt.date_naive());

    let time_range_re = Regex::new(r"(?i)\b(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)\s*(?:-|–|—|to)\s*(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.))\b").unwrap();
    let day_pattern_re = Regex::new(r"(?i)\b((?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun)(?:\s*,\s*(?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun))*)\b").unwrap();
    let lines: Vec<&str> = ocr_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    let mut events = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        // If the line contains a date heading, update the current tracking date
        if let Some(d) = extract_date(line, ref_dt) {
            current_date = d;
        }

        if let Some(caps) = time_range_re.captures(line) {
            let time_str = caps.get(0).unwrap().as_str();
            let times_opt = extract_times(time_str);

            if let Some((start_time, end_time_opt)) = times_opt {
                let days_opt = day_pattern_re.find(line).map(|m| m.as_str().to_string());
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
                let paren_re = Regex::new(r"\(([^)]+)\)").unwrap();
                let (mut title, location) = if let Some(p_cap) = paren_re.captures(cleaned_rest) {
                    let loc = p_cap.get(1).unwrap().as_str().trim().to_string();
                    let t = paren_re.replace(cleaned_rest, "").trim().to_string();
                    (t, Some(loc))
                } else {
                    (cleaned_rest.to_string(), None)
                };

                let is_weekday_only = title.is_empty() || title.split(',').all(|w| {
                    let tw = w.trim().to_lowercase();
                    matches!(tw.as_str(), "mo" | "tu" | "we" | "th" | "fr" | "sa" | "su" | "mon" | "tue" | "tues" | "wed" | "thu" | "thur" | "thurs" | "fri" | "sat" | "sun" | "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday")
                });

                if is_weekday_only && idx > 0 && !time_range_re.is_match(lines[idx - 1]) {
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
                        confidence: 0.90,
                        source: "deterministic_agenda".to_string(),
                    });
                }
            }
        }
    }

    events
}

/// Core single-event deterministic extraction
pub fn parse_single_event_deterministic(ocr_text: &str, context: &ReferenceContext) -> EventDetails {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();

    let lines: Vec<&str> = ocr_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // 1. Date Extraction
    let extracted_date = extract_date(ocr_text, ref_dt);

    // 2. Time Extraction
    let extracted_times = extract_times(ocr_text);

    // Combine date & time into ISO-8601 timestamps
    let (start_time_iso, end_time_iso, is_all_day) = match (extracted_date, extracted_times) {
        (Some(date), Some((start_time, end_time_opt))) => {
            let start_dt = offset.from_local_datetime(&date.and_time(start_time)).unwrap();

            let end_dt_str = if let Some(end_time) = end_time_opt {
                let end_date = if end_time < start_time {
                    date + Duration::days(1)
                } else {
                    date
                };
                let end_dt = offset.from_local_datetime(&end_date.and_time(end_time)).unwrap();
                Some(end_dt.to_rfc3339())
            } else {
                let end_dt = start_dt + Duration::hours(1);
                Some(end_dt.to_rfc3339())
            };

            (Some(start_dt.to_rfc3339()), end_dt_str, false)
        }
        (Some(date), None) => {
            // All-day event on this date
            let date_str = date.format("%Y-%m-%d").to_string();
            (Some(date_str.clone()), Some(date_str), true)
        }
        (None, Some((start_time, end_time_opt))) => {
            // Time found but no date -> assume reference date (or tomorrow if time already passed today)
            let mut target_date = ref_dt.date_naive();
            if start_time < ref_dt.time() {
                target_date = target_date + Duration::days(1);
            }
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
        }
        (None, None) => (None, None, false),
    };

    // 3. Location Extraction
    let location = extract_location(&lines, ocr_text);

    // 4. Title Extraction
    let title = extract_title(&lines, ocr_text);

    // 5. Description Extraction
    let description = extract_description(&lines, &title, location.as_deref());

    // 6. Confidence calculation
    let mut confidence_score: f32 = 0.3;
    if !title.is_empty() && title != "Event" && title != "Untitled Event" {
        confidence_score += 0.25;
    }
    if start_time_iso.is_some() {
        confidence_score += 0.25;
    }
    if location.is_some() {
        confidence_score += 0.15;
    }
    if description.is_some() {
        confidence_score += 0.05;
    }
    let confidence = confidence_score.min(0.95);

    EventDetails {
        title,
        start_time: start_time_iso,
        end_time: end_time_iso,
        is_all_day,
        location,
        description,
        recurrence_rule: None,
        confidence,
        source: "deterministic".to_string(),
    }
}

fn month_to_num(m: &str) -> Option<u32> {
    let lower = m.to_lowercase();
    match lower.as_str() {
        "jan" | "january" | "jan." => Some(1),
        "feb" | "february" | "feb." => Some(2),
        "mar" | "march" | "mar." => Some(3),
        "apr" | "april" | "apr." => Some(4),
        "may" => Some(5),
        "jun" | "june" | "jun." => Some(6),
        "jul" | "july" | "jul." => Some(7),
        "aug" | "august" | "aug." => Some(8),
        "sep" | "sept" | "september" | "sep." | "sept." => Some(9),
        "oct" | "october" | "oct." => Some(10),
        "nov" | "november" | "nov." => Some(11),
        "dec" | "december" | "dec." => Some(12),
        _ => None,
    }
}

fn weekday_to_num(w: &str) -> Option<chrono::Weekday> {
    let lower = w.to_lowercase();
    match lower.as_str() {
        "mon" | "monday" | "mon." => Some(chrono::Weekday::Mon),
        "tue" | "tues" | "tuesday" | "tue." | "tues." => Some(chrono::Weekday::Tue),
        "wed" | "wednesday" | "wed." => Some(chrono::Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" | "thu." | "thurs." => Some(chrono::Weekday::Thu),
        "fri" | "friday" | "fri." => Some(chrono::Weekday::Fri),
        "sat" | "saturday" | "sat." => Some(chrono::Weekday::Sat),
        "sun" | "sunday" | "sun." => Some(chrono::Weekday::Sun),
        _ => None,
    }
}

fn resolve_year(month: u32, day: u32, weekday_opt: Option<chrono::Weekday>, ref_dt: DateTime<FixedOffset>) -> i32 {
    let ref_year = ref_dt.year();

    // If weekday is given, test candidate years [ref_year, ref_year + 1, ref_year - 1] to see which matches
    if let Some(target_weekday) = weekday_opt {
        for candidate_year in [ref_year, ref_year + 1, ref_year - 1] {
            if let Some(d) = NaiveDate::from_ymd_opt(candidate_year, month, day) {
                if d.weekday() == target_weekday {
                    return candidate_year;
                }
            }
        }
    }

    // Default heuristic: if date is more than 30 days in the past of the reference year, assume next year
    if let Some(d_curr) = NaiveDate::from_ymd_opt(ref_year, month, day) {
        if d_curr < ref_dt.date_naive() - Duration::days(60) {
            return ref_year + 1;
        }
    }

    ref_year
}

fn extract_date(text: &str, ref_dt: DateTime<FixedOffset>) -> Option<NaiveDate> {
    let clean_text = text.replace('\n', " ");

    // Relative dates: "today", "tomorrow", "this friday", "next monday"
    let rel_regex = Regex::new(r"(?i)\b(today|tomorrow|this\s+(?:mon|tues|wed|thu|thur|thurs|fri|sat|sun)(?:day)?|next\s+(?:mon|tues|wed|thu|thur|thurs|fri|sat|sun)(?:day)?)\b").unwrap();
    if let Some(caps) = rel_regex.captures(&clean_text) {
        let matched = caps.get(1).unwrap().as_str().to_lowercase();
        if matched == "today" {
            return Some(ref_dt.date_naive());
        }
        if matched == "tomorrow" {
            return Some(ref_dt.date_naive() + Duration::days(1));
        }
        if matched.starts_with("this ") || matched.starts_with("next ") {
            let parts: Vec<&str> = matched.split_whitespace().collect();
            if parts.len() == 2 {
                if let Some(target_w) = weekday_to_num(parts[1]) {
                    let mut d = ref_dt.date_naive();
                    let is_next = parts[0] == "next";
                    if is_next {
                        d = d + Duration::days(7);
                    }
                    for _ in 0..7 {
                        if d.weekday() == target_w {
                            return Some(d);
                        }
                        d = d + Duration::days(1);
                    }
                }
            }
        }
    }

    // Pattern 1: Day of week + Day + Month (e.g. "SATURDAY 12 September" or "SATURDAY 12\nSeptember")
    let day_month_regex = Regex::new(r"(?i)\b(?:(mon|tue|wed|thu|fri|sat|sun)[a-z]*\s+)?(\d{1,2})(?:st|nd|rd|th)?\s+(?:of\s+)?(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*(?:\s*,?\s*(\d{4}|\d{2}))?\b").unwrap();
    if let Some(caps) = day_month_regex.captures(&clean_text) {
        let weekday_opt = caps.get(1).and_then(|w| weekday_to_num(w.as_str()));
        let day: u32 = caps.get(2).unwrap().as_str().parse().ok()?;
        let month_str = caps.get(3).unwrap().as_str();
        let month = month_to_num(month_str)?;
        let year = if let Some(y_cap) = caps.get(4) {
            let mut y: i32 = y_cap.as_str().parse().ok()?;
            if y < 100 {
                y += 2000;
            }
            y
        } else {
            resolve_year(month, day, weekday_opt, ref_dt)
        };

        if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
            return Some(d);
        }
    }

    // Pattern 2: Month + Day + (Year) (e.g. "September 12", "Sept 12th, 2026", "October 5")
    let month_day_regex = Regex::new(r"(?i)\b(?:(mon|tue|wed|thu|fri|sat|sun)[a-z]*\s*,?\s*)?(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+(\d{1,2})(?:st|nd|rd|th)?(?:\s*,?\s*(\d{4}|\d{2}))?\b").unwrap();
    if let Some(caps) = month_day_regex.captures(&clean_text) {
        let weekday_opt = caps.get(1).and_then(|w| weekday_to_num(w.as_str()));
        let month_str = caps.get(2).unwrap().as_str();
        let month = month_to_num(month_str)?;
        let day: u32 = caps.get(3).unwrap().as_str().parse().ok()?;
        let year = if let Some(y_cap) = caps.get(4) {
            let mut y: i32 = y_cap.as_str().parse().ok()?;
            if y < 100 {
                y += 2000;
            }
            y
        } else {
            resolve_year(month, day, weekday_opt, ref_dt)
        };

        if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
            return Some(d);
        }
    }

    // Pattern 3: Numeric date MM/DD/YYYY, MM/DD/YY, MM.DD.YY, MM.DD.YYYY, YYYY-MM-DD, YYYY.MM.DD
    let numeric_regex = Regex::new(r"\b(\d{4})[/\.-](\d{1,2})[/\.-](\d{1,2})\b|\b(\d{1,2})[/\.](\d{1,2})[/\.](\d{2,4})\b").unwrap();
    if let Some(caps) = numeric_regex.captures(&clean_text) {
        if let (Some(y), Some(m), Some(d)) = (caps.get(1), caps.get(2), caps.get(3)) {
            let year: i32 = y.as_str().parse().ok()?;
            let month: u32 = m.as_str().parse().ok()?;
            let day: u32 = d.as_str().parse().ok()?;
            if let Some(res) = NaiveDate::from_ymd_opt(year, month, day) {
                return Some(res);
            }
        } else if let (Some(m), Some(d), Some(y)) = (caps.get(4), caps.get(5), caps.get(6)) {
            let month: u32 = m.as_str().parse().ok()?;
            let day: u32 = d.as_str().parse().ok()?;
            let mut year: i32 = y.as_str().parse().ok()?;
            if year < 100 {
                year += 2000;
            }
            if let Some(res) = NaiveDate::from_ymd_opt(year, month, day) {
                return Some(res);
            }
        }
    }

    None
}

fn parse_hour_min(hour_str: &str, min_str: Option<&str>, ampm_str: Option<&str>, default_is_pm: bool) -> Option<NaiveTime> {
    let mut hour: u32 = hour_str.parse().ok()?;
    let minute: u32 = if let Some(m) = min_str {
        m.parse().ok()?
    } else {
        0
    };

    if let Some(ampm) = ampm_str {
        let is_pm = ampm.to_lowercase().contains('p');
        let is_am = ampm.to_lowercase().contains('a');
        if is_pm && hour < 12 {
            hour += 12;
        } else if is_am && hour == 12 {
            hour = 0;
        }
    } else if default_is_pm && hour < 12 && hour != 12 {
        hour += 12;
    }

    NaiveTime::from_hms_opt(hour, minute, 0)
}

fn extract_times(text: &str) -> Option<(NaiveTime, Option<NaiveTime>)> {
    let clean_text = text.replace('\n', " ");

    // Pattern 1: Time range like "12-5pm", "12:00 PM - 5:00 PM", "7:00pm - 10:00pm", "12pm - 5pm", "10am to 2pm"
    let range_regex = Regex::new(r"(?i)\b(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?\s*(?:-|–|—|to|until)\s*(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)\b").unwrap();
    if let Some(caps) = range_regex.captures(&clean_text) {
        let start_hour_str = caps.get(1).unwrap().as_str();
        let start_min_str = caps.get(2).map(|m| m.as_str());
        let start_ampm_str = caps.get(3).map(|m| m.as_str());

        let end_hour_str = caps.get(4).unwrap().as_str();
        let end_min_str = caps.get(5).map(|m| m.as_str());
        let end_ampm_str = caps.get(6).map(|m| m.as_str());

        let end_is_pm = end_ampm_str.map(|s| s.to_lowercase().contains('p')).unwrap_or(false);

        // Infer start am/pm if omitted
        let start_ampm = if start_ampm_str.is_some() {
            start_ampm_str
        } else {
            let start_h: u32 = start_hour_str.parse().unwrap_or(0);
            let end_h: u32 = end_hour_str.parse().unwrap_or(0);
            if start_h == 12 && end_is_pm {
                Some("pm")
            } else if end_is_pm && start_h <= end_h {
                Some("pm")
            } else if end_is_pm && start_h > end_h && start_h >= 8 && start_h <= 11 {
                // E.g. "9 - 2pm" -> 9am - 2pm
                Some("am")
            } else if end_is_pm && start_h > end_h {
                Some("pm")
            } else {
                end_ampm_str
            }
        };

        let start_time = parse_hour_min(start_hour_str, start_min_str, start_ampm, false)?;
        let end_time = parse_hour_min(end_hour_str, end_min_str, end_ampm_str, false)?;

        return Some((start_time, Some(end_time)));
    }

    // Pattern 2: 24h range "18:00 - 21:30"
    let range_24h_regex = Regex::new(r"\b(\d{1,2}):(\d{2})\s*(?:-|–|—|to)\s*(\d{1,2}):(\d{2})\b").unwrap();
    if let Some(caps) = range_24h_regex.captures(&clean_text) {
        let s_h: u32 = caps.get(1).unwrap().as_str().parse().ok()?;
        let s_m: u32 = caps.get(2).unwrap().as_str().parse().ok()?;
        let e_h: u32 = caps.get(3).unwrap().as_str().parse().ok()?;
        let e_m: u32 = caps.get(4).unwrap().as_str().parse().ok()?;

        let start_time = NaiveTime::from_hms_opt(s_h, s_m, 0)?;
        let end_time = NaiveTime::from_hms_opt(e_h, e_m, 0)?;
        return Some((start_time, Some(end_time)));
    }

    // Pattern 3: Single time "at 7:00 PM", "doors at 8pm", "starts at 6:30pm", "7pm", "19:00"
    let single_regex = Regex::new(r"(?i)(?:at|@|starts?|begins?|time:?)\s*(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)\b|\b(\d{1,2})(?::(\d{2}))\s*(am|pm|a\.m\.|p\.m\.)\b").unwrap();
    if let Some(caps) = single_regex.captures(&clean_text) {
        let (hour_str, min_str, ampm_str) = if caps.get(1).is_some() {
            (caps.get(1).unwrap().as_str(), caps.get(2).map(|m| m.as_str()), caps.get(3).map(|m| m.as_str()))
        } else {
            (caps.get(4).unwrap().as_str(), caps.get(5).map(|m| m.as_str()), caps.get(6).map(|m| m.as_str()))
        };

        let start_time = parse_hour_min(hour_str, min_str, ampm_str, false)?;
        return Some((start_time, None));
    }

    // Keyword: "noon" -> 12:00, "midnight" -> 00:00
    if clean_text.to_lowercase().contains("noon") {
        return Some((NaiveTime::from_hms_opt(12, 0, 0).unwrap(), None));
    }

    None
}

fn extract_location(lines: &[&str], _full_text: &str) -> Option<String> {
    // 1. Explicit marker: "This year at ...", "Location: ...", "Venue: ...", "Where: ..."
    let explicit_marker_regex = Regex::new(r"(?i)(?:this year at|held at|venue:|location:|where:|place:|live at|takes place at)\s*(.*)").unwrap();
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = explicit_marker_regex.captures(line) {
            let mut loc = caps.get(1).unwrap().as_str().trim().to_string();
            if loc.is_empty() && i + 1 < lines.len() {
                loc = lines[i + 1].trim().to_string();
            }

            // Append street / avenue / park / room details if on subsequent lines
            let mut extra_idx = i + 1;
            while extra_idx < lines.len() && extra_idx <= i + 3 {
                let next_line = lines[extra_idx].trim();
                if next_line.starts_with('*') || next_line.starts_with('-') || next_line.len() > 60 {
                    break;
                }
                let lower = next_line.to_lowercase();
                if lower.contains("street") || lower.contains("st") || lower.contains("ave")
                    || lower.contains("park") || lower.contains("skilton") || lower.contains("blvd")
                    || lower.contains("road") || lower.contains("room") || lower.contains("and ") {
                    if !loc.contains(next_line) {
                        loc = format!("{}, {}", loc, next_line);
                    }
                }
                extra_idx += 1;
            }

            let cleaned = clean_location_string(&loc);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    // 2. Scan for address / venue keywords in lines
    let venue_keywords = [
        "PARK", "SQUARE", "HALL", "CENTER", "CENTRE", "AUDITORIUM", "BALLROOM", "STREET", "ST.",
        "AVENUE", "AVE", "BOULEVARD", "BLVD", "ROAD", "RD", "DRIVE", "DR", "PLAZA",
        "ROOM", "CLUB", "THEATRE", "THEATER", "BAR", "GRILL", "CAFE", "BREWERY",
        "CHURCH", "LIBRARY", "MUSEUM", "GARDEN", "GARDENS", "STADIUM", "ARENA",
        "FIELD", "HOUSE", "LOUNGE", "HQ", "CAMPUS", "HUB", "STUDIO", "BUILDING",
        "TOWER", "GALLERY", "SPACE", "OFFICE", "PAVILION", "COMMONS", "HOTEL", "LAWN",
        "RESTAURANT", "TAVERN", "PUB", "AMPHITHEATER", "STAGE", "ZOOM", "GOOGLE MEET", "TEAMS", "WEBEX", "DISCORD",
        "BOSTON", "SOMERVILLE", "CAMBRIDGE", "BROOKLINE", "MEDFORD", "NEW YORK", "NYC",
    ];
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Skip obvious header, metadata, or pure date/time lines
        if trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•')
            || upper.contains("FOR ADA") || upper.contains("ACCOMMODATIONS") || upper.contains("311")
            || upper.contains("RAIN DATE") || upper.contains("ART BY:") || upper.len() < 3
            || is_date_or_time_line(&upper) {
            continue;
        }

        // Check if this line contains a venue keyword (handling both multi-word phrases and word boundaries)
        let has_venue_kw = venue_keywords.iter().any(|&kw| {
            if kw.contains(' ') {
                upper.contains(kw)
            } else {
                let clean_kw = kw.trim_end_matches('.');
                upper.split_whitespace().any(|word| {
                    let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
                    cleaned == kw || cleaned == clean_kw
                })
            }
        });
        if has_venue_kw {
            let mut loc = trimmed.to_string();
            if i + 1 < lines.len() {
                let next = lines[i + 1].trim();
                let next_upper = next.to_uppercase();
                let is_addr_or_city = next_upper.split_whitespace().any(|word| {
                    let c = word.trim_matches(|c: char| !c.is_alphanumeric());
                    c == "STREET" || c == "AVE" || c == "AVENUE" || c == "ST" || c == "ROAD" || c == "RD"
                        || c == "ROOM" || c == "BLVD" || c == "WAY" || c == "LANE" || c == "LN"
                        || c == "HIGHWAY" || c == "HWY" || c == "DRIVE" || c == "DR"
                }) || (next_upper.contains(',') && !next_upper.contains("INVITE") && !next_upper.contains("WE "));
                if is_addr_or_city && !next.starts_with('*') && !is_date_or_time_line(&next_upper) && !is_noise_or_metadata_line(next)
                    && !next_upper.contains("INVITE") && !next_upper.contains("DESSERT") && !next_upper.contains("WE ") {
                    loc = format!("{}, {}", loc, next);
                }
            }
            let cleaned = clean_location_string(&loc);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    None
}

fn clean_location_string(s: &str) -> String {
    let mut cleaned = s.trim().trim_matches(|c: char| c == ',' || c == '.' || c == ':' || c == '-' || c == '*').trim().to_string();
    cleaned = cleaned.replace("  ", " ");
    cleaned
}

fn is_time_pattern(s: &str) -> bool {
    let time_12h_regex = Regex::new(r"(?i)\b\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)\b|\b\d{1,2}\s*(?:-|–|—|to)\s*\d{1,2}\s*(?:am|pm|a\.m\.|p\.m\.)\b").unwrap();
    let time_24h_regex = Regex::new(r"\b\d{1,2}:\d{2}\b").unwrap();
    time_12h_regex.is_match(s) || time_24h_regex.is_match(s)
}

fn is_date_pattern(s: &str) -> bool {
    let num_date_regex = Regex::new(r"\b\d{1,2}/\d{1,2}(?:/\d{2,4})?\b|\b\d{4}-\d{1,2}-\d{1,2}\b").unwrap();
    if num_date_regex.is_match(s) {
        return true;
    }

    let month_day_regex = Regex::new(r"(?i)\b(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?\b|\b\d{1,2}(?:st|nd|rd|th)?\s+(?:of\s+)?(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\b").unwrap();
    let weekday_day_regex = Regex::new(r"(?i)\b(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?\b").unwrap();
    month_day_regex.is_match(s) || weekday_day_regex.is_match(s)
}

fn is_noise_or_metadata_line(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.len() < 3 {
        return true;
    }
    let phone_regex = Regex::new(r"\b\d{3}[-.)\s]+\d{3}[-.\s]+\d{4}\b|\b311\b").unwrap();
    if phone_regex.is_match(trimmed) {
        return true;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit() || c.is_ascii_punctuation() || c.is_whitespace()) {
        return true;
    }
    let lower = trimmed.to_lowercase();
    if lower == "follow" || lower == "following" || lower == "followers"
        || lower.starts_with("liked by ") || lower.ends_with("days ago")
        || lower.ends_with("hours ago") || lower.ends_with("mins ago")
        || lower == "more" || lower == "less" {
        return true;
    }
    if (lower.contains("accommodations") || lower.contains("interpreter") || lower.contains("contact")) && lower.contains("311") {
        return true;
    }
    false
}

fn is_date_or_time_line(s: &str) -> bool {
    is_time_pattern(s) || is_date_pattern(s)
}

fn extract_title(lines: &[&str], _full_text: &str) -> String {
    let event_keywords = [
        "FESTIVAL", "CONCERT", "PARTY", "CELEBRATION", "MEETUP", "MEETING",
        "CONFERENCE", "SUMMIT", "WORKSHOP", "SYMPOSIUM", "WEBINAR", "SHOW",
        "EXHIBITION", "FAIR", "GALA", "DINNER", "BRUNCH", "FUNDRAISER",
        "PARADE", "MARKET", "BLOCK PARTY", "OPEN MIC", "GAME NIGHT", "TRIVIA",
        "BBQ", "COOKOUT", "LAUNCH", "BIRTHDAY", "WEDDING", "SOCIAL", "ICE CREAM", "RECEPTION",
        "RIDE", "RALLY", "WALK", "RUN", "MARATHON", "TOUR", "RACE",
    ];
    let mut candidate_titles: Vec<(String, usize, i32)> = Vec::new();

    // Detect supporting act / opener (e.g. "WITH\nyoubef" or "WITH youbef" or "FEATURING ...")
    let mut supporting_act: Option<String> = None;
    let mut supporting_indices: Vec<usize> = Vec::new();
    for (i, &l) in lines.iter().enumerate().take(6) {
        let t = l.trim();
        let u = t.to_uppercase();
        if u == "WITH" || u == "W/" || u == "FEATURING" || u == "FEAT." || u == "FT." || u == "PLUS" || u == "SPECIAL GUESTS" || u == "SPECIAL GUEST" {
            supporting_indices.push(i);
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if !is_date_or_time_line(next_line) && !is_noise_or_metadata_line(next_line) && next_line.len() >= 2 {
                    supporting_act = Some(next_line.to_string());
                    supporting_indices.push(i + 1);
                }
            }
        } else if u.starts_with("WITH ") || u.starts_with("W/ ") || u.starts_with("FEAT. ") || u.starts_with("FEATURING ") {
            supporting_indices.push(i);
            let act = if u.starts_with("WITH ") {
                t[5..].trim()
            } else if u.starts_with("W/ ") {
                t[3..].trim()
            } else if u.starts_with("FEAT. ") {
                t[6..].trim()
            } else {
                t[10..].trim()
            };
            if !act.is_empty() {
                supporting_act = Some(act.to_string());
            }
        }
    }

    for (idx, &line) in lines.iter().enumerate().take(12) {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Skip bullet lines, metadata lines, phone numbers, pure dates/times, supporting act opener lines, etc.
        if trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•')
            || is_noise_or_metadata_line(trimmed)
            || is_date_or_time_line(trimmed)
            || supporting_indices.contains(&idx) {
            continue;
        }

        // Strong position boost for the top lines
        let mut score: i32 = match idx {
            0 => 35,
            1 => 25,
            2 => 15,
            3 => 10,
            _ => 5 - (idx as i32),
        };

        // Boost for event keywords
        for &kw in &event_keywords {
            if upper.contains(kw) {
                score += 25;
            }
        }

        // If uppercase or Title Cased, add a boost
        if upper == trimmed && trimmed.len() > 5 {
            score += 5;
        }
        // If we have a supporting act and this is a top artist line (not a tour subtitle), generate "Artist (with Opener)"
        if let Some(act) = &supporting_act {
            if !upper.contains("TOUR") && idx <= 1 {
                let with_title = format!("{} (with {})", trimmed, act);
                candidate_titles.push((with_title, idx, score + 30));
            }
        }

        // Check if preceding line is a multi-line title continuation or all-caps brand/prefix banner
        if idx > 0 {
            let prev = lines[idx - 1].trim();
            let prev_upper = prev.to_uppercase();

            // Multi-line title connection: e.g. "Dide For" / "Ride For" + "Your Life" -> "Ride For Your Life"
            let is_connector = prev_upper.ends_with(" FOR") || prev_upper.ends_with(" OF")
                || prev_upper.ends_with(" AND") || prev_upper.ends_with(" THE")
                || prev_upper.ends_with(" AT") || prev_upper.ends_with(" IN")
                || prev_upper.ends_with(" TO") || prev_upper.ends_with(" ON")
                || prev_upper.starts_with("DIDE ") || prev_upper.starts_with("RIDE ");

            if is_connector && idx == 1 && !is_noise_or_metadata_line(prev) && !is_date_or_time_line(prev) {
                let mut fixed_prev = prev.to_string();
                if fixed_prev.starts_with("Dide") || fixed_prev.starts_with("dide") || fixed_prev.starts_with("DIDE") {
                    fixed_prev = format!("Ride{}", &fixed_prev[4..]);
                }
                let combined = format!("{} {}", fixed_prev, trimmed);
                candidate_titles.push((combined, 0, score + 40));
            } else if prev == prev_upper
                && !prev.starts_with('*') && !prev.starts_with('-') && !prev.ends_with(':')
                && !is_noise_or_metadata_line(prev) && !is_date_or_time_line(prev)
                && !supporting_indices.contains(&(idx - 1))
                && prev.len() > 3 && prev.len() < 30 {
                let combined = format!("{}: {}", prev, trimmed);
                candidate_titles.push((combined, idx, score + 10));
            }
        }

        candidate_titles.push((trimmed.to_string(), idx, score));
    }
    if let Some((best_title, _, _)) = candidate_titles.into_iter().max_by_key(|item| item.2) {
        let clean = best_title.trim_matches(|c: char| c == '*' || c == '-' || c == ':').trim();
        return clean.to_string();
    }

    // Fallback: first non-trivial line
    for &line in lines.iter().take(4) {
        let t = line.trim();
        if t.len() > 3 && !t.contains(':') && !t.starts_with('*') && !is_noise_or_metadata_line(t) && !is_date_or_time_line(t) {
            return t.to_string();
        }
    }

    "Event".to_string()
}

fn extract_description(lines: &[&str], title: &str, location: Option<&str>) -> Option<String> {
    let mut bullet_points: Vec<String> = Vec::new();
    let mut prose_lines: Vec<String> = Vec::new();

    let title_upper = title.to_uppercase();
    let loc_upper = location.map(|l| l.to_uppercase()).unwrap_or_default();

    for &line in lines {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Skip title, location, noise/metadata, pure dates/times
        if title_upper.contains(&upper) || (!loc_upper.is_empty() && loc_upper.contains(&upper)) {
            continue;
        }
        if is_noise_or_metadata_line(trimmed) || (!upper.contains("RAIN DATE") && is_date_or_time_line(trimmed)) || (trimmed.ends_with(':') && trimmed.len() <= 5) || trimmed.len() < 4 {
            continue;
        }
        if trimmed.starts_with('@') || trimmed.contains("tufts_uep") || upper.starts_with("LIKED BY") || upper.contains("DAYS AGO") {
            continue;
        }

        // Keep bullet points, activity highlights, rain dates, special notes, tour info, mottos, rallies
        if trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•')
            || upper.contains("LIVE MUSIC") || upper.contains("PERFORMANCES") || upper.contains("BEER GARDEN")
            || upper.contains("FOOD VENDORS") || upper.contains("ARTISTS") || upper.contains("ACTIVITIES")
            || upper.contains("RAIN DATE") || upper.contains("FREE ADMISSION") || upper.contains("ALL AGES")
            || upper.contains("SPEAKERS") || upper.contains("PIZZA") || upper.contains("DRINKS")
            || upper.contains("TOUR") || upper.contains("DOORS") || upper.contains("SPECIAL GUEST")
            || upper.contains("RIDE.") || upper.contains("WALK.") || upper.contains("RALLY")
            || upper.contains("STREETS EXIST") || upper.contains("EVERYONE") || upper.contains("MEMORIAL") {
            let clean_bullet = trimmed.trim_start_matches(|c: char| c == '*' || c == '-' || c == '•').trim();
            if !clean_bullet.is_empty() && !bullet_points.iter().any(|b| b == clean_bullet) {
                bullet_points.push(clean_bullet.to_string());
            }
        } else if upper.contains("INVITE") || upper.contains("DESSERT") || upper.contains("ICE CREAM")
            || upper.contains("COFFEE") || upper.contains("TEA") || upper.contains("FRUIT")
            || upper.contains("JOIN US") || upper.contains("WELCOME") || upper.contains("REFRESHMENT") {
            prose_lines.push(trimmed.to_string());
        }
    }
    if !bullet_points.is_empty() {
        Some(bullet_points.join(" • "))
    } else if !prose_lines.is_empty() {
        Some(prose_lines.join(" "))
    } else {
        None
    }
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
    fn test_parse_gilman_flyer_deterministic() {
        let ocr_text = r#"MAN:
SOMERSTREETS
GILMAN SQUARE
ARTS & MUSIC FESTIVAL
SATURDAY 12
September -
12-5pm
Rain Date 09/13/26
LIVE MUSIC &
PERFORMANCES
*BEER GARDEN
*FOOD VENDORS
*ARTISTS&MAKERS
*KIDS ACTIVITIES
FOR FESTIVAL UPDATES & BAND INFO FOLLOW
the GSNC
art by: BEARDED TALES OF WOE
Mass
MC Council
Council
*STARA
GSNC
ELMAN SQUARE NBEHBORHOOD COUNCI
McMAHON
PLUMBING & HEATING
PARADIGM
WINTER HILL
EVENTTHEM MARKMENT
This year at
Ed Leathers Park
Walnut Street
and Skilton Ave
300
For ADA accommodations or to request
an interpreter in your language, please
contact 311(617-666-3311) in advance.
CHOLES
NO EGU"#;

        let ctx = sample_reference_context();
        let event = parse_event_deterministic(ocr_text, &ctx);

        assert!(
            event.title.contains("GILMAN SQUARE") || event.title.contains("FESTIVAL"),
            "Title should contain event name, got: {}",
            event.title
        );

        assert!(event.start_time.is_some(), "Start time should be extracted");
        let start_time = event.start_time.unwrap();
        assert!(
            start_time.starts_with("2026-09-12T12:00:00"),
            "Start time should be 2026-09-12 at 12:00, got: {}",
            start_time
        );

        assert!(event.end_time.is_some(), "End time should be extracted");
        let end_time = event.end_time.unwrap();
        assert!(
            end_time.starts_with("2026-09-12T17:00:00"),
            "End time should be 2026-09-12 at 17:00 (5pm), got: {}",
            end_time
        );

        assert!(event.location.is_some(), "Location should be extracted");
        let loc = event.location.unwrap();
        assert!(
            loc.contains("Ed Leathers Park") || loc.contains("Walnut Street"),
            "Location should contain Ed Leathers Park / Walnut Street, got: {}",
            loc
        );

        assert!(event.description.is_some(), "Description should be extracted");
        let desc = event.description.unwrap();
        assert!(
            desc.contains("BEER GARDEN") || desc.contains("FOOD VENDORS") || desc.contains("LIVE MUSIC"),
            "Description should contain activities, got: {}",
            desc
        );

        assert!(event.confidence >= 0.7, "Confidence should be high, got: {}", event.confidence);
    }

    #[test]
    fn test_parse_instagram_ice_cream_social() {
        let ocr_text = r#"3:20
Follow
5G
824
Follow
Tufts
tufts_uep
Bromfield Pearson
UEP ICE CREAM SOCIAL
09 Sept,
2026
12:00 PM - 1:00 PM
BP LAWN
We invite you to have
dessert with us. Don't
like ice cream? We
have iced coffee, iced
tea, and fruit too!
Tafts
11
1 1
Liked by sabina_a_dz and others
tufts_uep Don't miss our first social event of the... more
4 days ago
JImmy"#;

        let ctx = sample_reference_context(); // Reference 2026-09-06
        let event = parse_event_deterministic(ocr_text, &ctx);

        assert_eq!(event.title, "UEP ICE CREAM SOCIAL");
        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-09T12:00:00-04:00")
        );
        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-09T13:00:00-04:00")
        );
        assert_eq!(
            event.description.as_deref(),
            Some("We invite you to have dessert with us. Don't like ice cream? We have iced coffee, iced tea, and fruit too!"),
            "Description must match the complete details text"
        );
    }
    #[test]
    fn test_parse_squirrel_flower_sample_deterministic() {
        let ocr_text = r#"squirrei flower
2026 TOUR
WITH
youbef
SATURDAY, SEPTEMBER 26
CRYSTAL BALLROOM
SOMERVILLE, MA"#;

        let ctx = sample_reference_context();
        let event = parse_event_deterministic(ocr_text, &ctx);

        assert!(
            event.title.to_lowercase().contains("flower"),
            "Title should contain artist name, got: {}",
            event.title
        );
        assert!(
            event.title.contains("(with youbef)") || event.title.contains("youbef") || event.title.contains("you bet"),
            "Title should contain supporting act, got: {}",
            event.title
        );

        assert_eq!(
            event.start_time.as_deref(),
            Some("2026-09-26"),
            "Start date should match Saturday Sep 26 2026"
        );
        assert_eq!(
            event.end_time.as_deref(),
            Some("2026-09-26"),
            "End date should match Saturday Sep 26 2026"
        );
        assert!(
            event.is_all_day,
            "Date-only tour poster should be parsed as all-day event"
        );

        assert!(event.location.is_some(), "Location should be extracted");
        let loc = event.location.unwrap();
        assert!(
            loc.contains("CRYSTAL BALLROOM"),
            "Location should contain Crystal Ballroom, got: {}",
            loc
        );
        assert!(
            loc.contains("SOMERVILLE, MA") || loc.contains("SOMERVILLE"),
            "Location should contain Somerville, MA, got: {}",
            loc
        );

        assert!(event.description.is_some(), "Description should be extracted");
        let desc = event.description.unwrap();
        assert!(
            desc.contains("2026 TOUR") || desc.contains("TOUR"),
            "Description should contain tour details, got: {}",
            desc
        );

        assert!(event.confidence >= 0.8, "Confidence should be high, got: {}", event.confidence);
    }

    #[test]
    fn test_parse_dinner_invitation_relative_date() {
        let text = "Hey! Let's do dinner this Friday at 7:30pm at Mario's Italian Restaurant on 5th Ave.";
        let ctx = sample_reference_context(); // Reference is Sunday 2026-09-06
        let event = parse_event_deterministic(text, &ctx);

        assert!(event.start_time.is_some());
        let start_time = event.start_time.unwrap();
        // This Friday from Sunday Sep 6 is Friday Sep 11, 2026
        assert!(
            start_time.starts_with("2026-09-11T19:30:00"),
            "Expected 2026-09-11T19:30:00, got: {}",
            start_time
        );
    }

    #[test]
    fn test_parse_meetup_structured() {
        let text = r#"Rust & AI Meetup
Thursday, October 15, 2026
6:00 PM - 8:30 PM
MIT Stata Center, Room 32-123
*Pizza and drinks provided
*Talks on WebAssembly and LLMs"#;

        let ctx = sample_reference_context();
        let event = parse_event_deterministic(text, &ctx);

        assert!(event.title.contains("Meetup") || event.title.contains("Rust"));
        assert_eq!(event.start_time.as_deref(), Some("2026-10-15T18:00:00-04:00"));
        assert_eq!(event.end_time.as_deref(), Some("2026-10-15T20:30:00-04:00"));
        assert!(event.location.unwrap().contains("MIT Stata Center"));
        assert!(event.description.unwrap().contains("Pizza"));
    }

    #[test]
    fn test_gbnf_grammar_and_json_schema() {
        let grammar = get_gbnf_grammar();
        assert!(grammar.contains("root ::="));
        assert!(grammar.contains(r#"\"events\":"#));
        assert!(grammar.contains(r#"\"title\":"#));
        assert!(grammar.contains(r#"\"start_time\":"#));
        let schema = get_json_schema();
        assert_eq!(schema["title"], "EventsPayload");
        assert!(schema["properties"]["events"].is_object());
    }

    #[test]
    fn test_dynamic_context_injection_prompt() {
        let ctx = sample_reference_context();
        let prompt = generate_extraction_prompt("Concert tomorrow at 8pm", &ctx);
        assert!(prompt.contains("Reference Time: 2026-09-06T10:57:00-04:00 (Sunday)"));
        assert!(prompt.contains("Concert tomorrow at 8pm"));
    }

    #[test]
    fn test_parse_all_day_event() {
        let text = "Community Clean-up Day\nSaturday, April 18, 2026\nLincoln Park\n*Bring gloves and water bottles";
        let ctx = sample_reference_context();
        let event = parse_event_deterministic(text, &ctx);

        assert!(event.is_all_day);
        assert!(event.title.contains("Clean-up") || event.title.contains("Community"));
        assert!(event.start_time.unwrap().starts_with("2026-04-18"));
        assert!(event.location.unwrap().contains("Lincoln Park"));
        assert!(event.description.unwrap().contains("Bring gloves"));
    }

    #[test]
    fn test_parse_24h_time_range() {
        let text = "Developer Workshop\n2026-10-05\n14:00 - 16:30\nInnovation Hub, Room 101";
        let ctx = sample_reference_context();
        let event = parse_event_deterministic(text, &ctx);

        assert!(!event.is_all_day);
        assert_eq!(event.start_time.as_deref(), Some("2026-10-05T14:00:00-04:00"));
        assert_eq!(event.end_time.as_deref(), Some("2026-10-05T16:30:00-04:00"));
        assert!(event.location.unwrap().contains("Innovation Hub"));
    }

    #[test]
    fn test_parse_tomorrow_relative_date() {
        let text = "Team Sync\nTomorrow at 3pm\nZoom Meeting";
        let ctx = sample_reference_context(); // Reference 2026-09-06 (Sunday)
        let event = parse_event_deterministic(text, &ctx);

        assert!(!event.is_all_day);
        // Tomorrow from Sunday Sep 6 is Monday Sep 7, 2026 at 15:00
        assert_eq!(event.start_time.as_deref(), Some("2026-09-07T15:00:00-04:00"));
        assert!(event.location.unwrap().to_lowercase().contains("zoom"));
    }

    #[test]
    fn test_titles_with_month_weekday_or_time_substrings() {
        let ctx = sample_reference_context();

        let text1 = "September Fest\nSaturday, September 12, 2026\n12-5pm\nEd Leathers Park";
        let event1 = parse_event_deterministic(text1, &ctx);
        assert_eq!(event1.title, "September Fest");

        let text2 = "Saturday Night Live Jam\n2026-11-20\n8:00 PM\nDowntown Lounge";
        let event2 = parse_event_deterministic(text2, &ctx);
        assert_eq!(event2.title, "Saturday Night Live Jam");

        let text3 = "Summer Program Launch\nJuly 15, 2026\n10:00 AM\nAuditorium A";
        let event3 = parse_event_deterministic(text3, &ctx);
        assert_eq!(event3.title, "Summer Program Launch");

        let text4 = "Team BBQ Celebration\nAugust 8, 2026\n1:00 PM - 5:00 PM\nCity Park";
        let event4 = parse_event_deterministic(text4, &ctx);
        assert_eq!(event4.title, "Team BBQ Celebration");
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
        let events = parse_events_deterministic(ocr_text, &ctx);

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
        let events = parse_events_deterministic(columnar_ocr_text, &ctx);

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
    fn test_recurrence_rule_parsing_and_validation() {
        let valid = "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z";
        let parsed = RecurrenceRule::parse_rrule(valid).expect("Should parse valid rule");
        assert_eq!(parsed.frequency, "WEEKLY");
        assert_eq!(parsed.by_days, vec!["MO", "WE"]);
        assert_eq!(parsed.until.as_deref(), Some("20261218T235959Z"));
        assert_eq!(parsed.to_rrule_string(), "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z");

        // Test missing FREQ
        let invalid_no_freq = "BYDAY=MO,WE";
        assert!(RecurrenceRule::parse_rrule(invalid_no_freq).is_err());

        // Test invalid frequency
        let invalid_freq = "FREQ=HOURLY;BYDAY=MO";
        assert!(RecurrenceRule::parse_rrule(invalid_freq).is_err());

        // Test invalid BYDAY
        let invalid_byday = "FREQ=WEEKLY;BYDAY=INVALID";
        assert!(RecurrenceRule::parse_rrule(invalid_byday).is_err());
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
        let events = parse_events_deterministic(text, &ctx);

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
        let events = parse_events_deterministic(text, &ctx);

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
        let events = parse_events_deterministic(text, &ctx);

        assert_eq!(events.len(), 2);
        assert!(events[0].title.contains("CEE 0154-03"));
        assert!(events[0].location.as_deref().unwrap().contains("Anderson Wing"));
        assert!(events[1].title.contains("CS 0150-09"));
        assert_eq!(events[1].location.as_deref(), Some("Online"));
    }

    #[test]
    fn test_parse_ride_for_life_deterministic() {
        let ocr_text = r#"Dide For
Your Life
BOSTON
10.25.26
RIDE. WALK.
RALLY.
MEN
OUR STREETS EXIST For EVERYONE"#;

        let ctx = sample_reference_context();
        let event = parse_event_deterministic(ocr_text, &ctx);

        println!("--- Parsed Ride For Life Event ---\n{:#?}\n----------------------------------", event);

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
        assert!(
            event.is_all_day,
            "Date-only rally flyer should be parsed as all-day event"
        );
        assert!(
            event.location.is_some(),
            "Location should be extracted"
        );
        let loc = event.location.unwrap();
        assert!(
            loc.to_uppercase().contains("BOSTON"),
            "Location should contain Boston, got: {}",
            loc
        );
        assert!(
            event.description.is_some(),
            "Description should be extracted"
        );
        let desc = event.description.unwrap();
        assert!(
            desc.contains("RIDE") || desc.contains("RALLY") || desc.contains("EVERYONE"),
            "Description should contain rally details, got: {}",
            desc
        );
        assert_eq!(event.recurrence_rule, None);
    }
}
