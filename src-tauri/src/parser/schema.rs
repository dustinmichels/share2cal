use crate::inference::{DEFAULT_CONTEXT_WINDOW, MAX_OUTPUT_TOKENS};
use crate::parser::datetime::is_date_or_time_line;
use crate::parser::heuristics::is_noise_or_metadata_line;
use crate::parser::regex::COURSE_CODE_ANCHORED_RE;
use crate::parser::ReferenceContext;
use chrono::Datelike;

/// Maximum prompt token budget reserved for input context (context window - max output tokens)
pub const MAX_PROMPT_TOKENS: usize = (DEFAULT_CONTEXT_WINDOW as usize) - MAX_OUTPUT_TOKENS;

/// Generates a strict GBNF grammar for llama.cpp structured event extraction
pub fn get_gbnf_grammar() -> &'static str {
    r#"root ::= ws "{" ws "\"events\":" ws "[" ws event-list? ws "]" ws "}" ws
event-list ::= event ("," ws event)*
event ::= "{" ws "\"title\":" ws string "," ws "\"start_time\":" ws optstring "," ws "\"end_time\":" ws optstring "," ws "\"is_all_day\":" ws boolean "," ws "\"location\":" ws optstring "," ws "\"description\":" ws optstring "," ws "\"recurrence_rule\":" ws optstring "," ws "\"url\":" ws optstring "}"
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
                        },
                        "url": {
                            "type": ["string", "null"],
                            "description": "Web link, webinar, meeting, or RSVP URL associated with the event (e.g. Zoom or event registration link)."
                        }
                    },
                    "required": ["title", "start_time", "end_time", "is_all_day", "location", "description", "recurrence_rule", "url"],
                }
            }
        },
        "required": ["events"],
        "additionalProperties": false
    })
}

/// Estimates the number of tokens in a string using a conservative character and structure heuristic
pub fn estimate_token_count(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }
    // Conservative estimate: ~3.0 characters per token for OCR/prompt text, rounding up
    trimmed.chars().count().div_ceil(3)
}

/// Scores an OCR line to prioritize retention of high-signal calendar event details during adaptive prompt trimming
pub(crate) fn score_ocr_line(line: &str, line_idx: usize, _total_lines: usize) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return -100;
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("www.")
        || lower.starts_with("links / qr codes:")
        || lower.starts_with("qr code:")
        || lower.starts_with("qr codes:")
        || lower.starts_with("qr:")
        || lower.starts_with("url:")
        || lower.starts_with("link:")
        || lower.starts_with("links:")
        || lower.starts_with("website:")
        || lower.starts_with("zoom:")
        || lower.starts_with("rsvp:")
    {
        return 40;
    }
    if is_noise_or_metadata_line(trimmed) {
        return -50;
    }
    if lower.starts_with('@') || lower.starts_with('#') {
        return -30;
    }
    if trimmed.len() < 3 && !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return -25;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_punctuation() || c.is_whitespace() || c.is_ascii_digit())
        && !is_date_or_time_line(trimmed)
    {
        return -20;
    }

    let mut score = 10;

    // Header boost for top lines (often title/artist)
    if line_idx < 3 {
        score += match line_idx {
            0 => 30,
            1 => 20,
            _ => 10,
        };
    }

    // High signal: Date or time patterns
    if is_date_or_time_line(trimmed) {
        score += 50;
    }

    // High signal: Course codes / Schedule indicators (e.g. CS 0150, MATH 101)
    if COURSE_CODE_ANCHORED_RE.is_match(trimmed) {
        score += 40;
    }

    // High signal: Common event keywords
    let event_keywords = [
        "FESTIVAL", "CONCERT", "PARTY", "CELEBRATION", "MEETUP", "MEETING",
        "CONFERENCE", "SUMMIT", "WORKSHOP", "SYMPOSIUM", "WEBINAR", "SHOW",
        "EXHIBITION", "FAIR", "GALA", "DINNER", "BRUNCH", "FUNDRAISER",
        "PARADE", "MARKET", "BLOCK PARTY", "OPEN MIC", "GAME NIGHT", "TRIVIA",
        "BBQ", "COOKOUT", "LAUNCH", "BIRTHDAY", "WEDDING", "SOCIAL", "RECEPTION",
        "RIDE", "RALLY", "WALK", "RUN", "MARATHON", "TOUR", "RACE",
        "SEMINAR", "LECTURE", "CLASS", "COURSE", "TALK", "PANEL",
    ];
    let upper = trimmed.to_uppercase();
    if event_keywords.iter().any(|&kw| upper.contains(kw)) {
        score += 30;
    }

    // Location / Venue keywords
    let location_keywords = [
        "ROOM", "HALL", "STREET", "ST", "AVE", "AVENUE", "BLVD", "BLDG",
        "BUILDING", "CENTER", "CENTRE", "AUDITORIUM", "LAB", "SQUARE", "PARK",
        "CAMPUS", "THEATER", "THEATRE", "LIBRARY", "PLAZA", "SUITE", "FLOOR",
    ];
    if location_keywords.iter().any(|&kw| {
        let words: Vec<&str> = upper.split_whitespace().collect();
        words.contains(&kw)
    }) {
        score += 30;
    }

    // Bullet points / descriptions
    if trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•') {
        score += 15;
    }

    score
}

/// Adaptively trims OCR text to fit within a given token budget by iteratively dropping low-signal lines
pub fn trim_ocr_text_to_budget(ocr_text: &str, max_tokens: usize) -> String {
    let trimmed = ocr_text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if estimate_token_count(trimmed) <= max_tokens {
        return trimmed.to_string();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() <= 1 {
        let char_limit = max_tokens * 3;
        if trimmed.chars().count() > char_limit {
            return format!("{}...", trimmed.chars().take(char_limit).collect::<String>());
        }
        return trimmed.to_string();
    }

    let total_lines = lines.len();
    let scored: Vec<(usize, &str, i32)> = lines
        .iter()
        .enumerate()
        .map(|(idx, &line)| (idx, line, score_ocr_line(line, idx, total_lines)))
        .collect();

    // Track token weight per line (token estimate of trimmed line + 1 for newline separator)
    let line_tokens: Vec<usize> = lines
        .iter()
        .map(|l| estimate_token_count(l) + 1)
        .collect();
    let mut current_estimated_tokens: usize = line_tokens.iter().sum();

    let mut included = vec![true; total_lines];

    // Candidate drop order: lowest score first; break ties by dropping later lines first
    let mut drop_order: Vec<usize> = (0..total_lines).collect();
    drop_order.sort_by(|&a, &b| {
        let score_a = scored[a].2;
        let score_b = scored[b].2;
        if score_a != score_b {
            score_a.cmp(&score_b)
        } else {
            b.cmp(&a)
        }
    });

    for &drop_idx in &drop_order {
        if current_estimated_tokens <= max_tokens {
            break;
        }
        included[drop_idx] = false;
        current_estimated_tokens = current_estimated_tokens.saturating_sub(line_tokens[drop_idx]);
    }

    let mut remaining: String = scored
        .iter()
        .filter(|(idx, _, _)| included[*idx])
        .map(|(_, line, _)| *line)
        .collect::<Vec<_>>()
        .join("\n");

    if remaining.trim().is_empty() {
        remaining = lines.into_iter().take(3).collect::<Vec<_>>().join("\n");
    }

    let char_limit = max_tokens * 3;
    if remaining.chars().count() > char_limit {
        remaining = format!("{}...", remaining.chars().take(char_limit).collect::<String>());
    }

    remaining
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

    let template_prefix = format!(
        "<|im_start|>system\nYou are a calendar assistant. Extract all events from the OCR text into a JSON object with an \"events\" array.\n\
        Each event must have fields: \"title\" (string), \"start_time\" (ISO-8601 or YYYY-MM-DD or null), \"end_time\" (ISO-8601 or YYYY-MM-DD or null), \"is_all_day\" (boolean), \"location\" (string or null), \"description\" (string or null), \"recurrence_rule\" (string or null, e.g. FREQ=WEEKLY;BYDAY=MO,WE), \"url\" (string or null, e.g. meeting/RSVP link or web URL). Output ONLY raw JSON.\n\
        Reference Time: {} ({})<|im_end|>\n\
        <|im_start|>user\n",
        ref_str, day_name
    );
    let template_suffix = "\n<|im_end|>\n<|im_start|>assistant\n{\"events\": [";
    let (main_text, links_block) = if let Some(idx) = ocr_text.find("Links / QR Codes:") {
        (&ocr_text[..idx], Some(&ocr_text[idx..]))
    } else {
        (ocr_text, None)
    };

    let links_tokens = links_block.map(|l| estimate_token_count(l) + 2).unwrap_or(0);
    let template_tokens = estimate_token_count(&template_prefix) + estimate_token_count(template_suffix) + links_tokens;
    let ocr_budget = MAX_PROMPT_TOKENS.saturating_sub(template_tokens);

    let effective_ocr_text = trim_ocr_text_to_budget(main_text, ocr_budget);

    let user_prompt = match links_block {
        Some(lb) if !effective_ocr_text.trim().is_empty() => {
            format!("{}\n\n{}", effective_ocr_text.trim(), lb.trim())
        }
        Some(lb) => lb.trim().to_string(),
        None => effective_ocr_text.trim().to_string(),
    };

    format!(
        "{}{}{}",
        template_prefix,
        user_prompt,
        template_suffix
    )
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
        let prompt = generate_extraction_prompt("Concert tomorrow at 8pm\nLinks / QR Codes: https://example.com/rsvp", &ctx);
        assert!(prompt.contains("Reference Time: 2026-09-06T10:57:00-04:00 (Sunday)"));
        assert!(prompt.contains("Concert tomorrow at 8pm"));
        assert!(prompt.contains("\"url\" (string or null"));
        assert!(prompt.contains("https://example.com/rsvp"));
    }

    #[test]
    fn test_adaptive_prompt_trimming_preserves_high_signal_lines() {
        let ctx = sample_reference_context();
        let mut lines = vec![
            "CAMPUS FESTIVAL 2026",
            "Saturday, September 19, 2026",
            "12:00 PM - 6:00 PM",
            "Academic Quad, Student Center Room 204",
            "follow",
            "liked by user123 and 45 others",
            "@campuslife_official",
            "https://example.com/tickets/long/url/path",
            "---",
        ];
        for i in 0..100 {
            lines.push(match i % 3 {
                0 => "Random background sponsor boilerplate with low signal text description",
                1 => "Some additional arbitrary filler line repeating general notices",
                _ => "UI noise artifact 12345 67890",
            });
        }
        lines.push("* Free food and live music performances");
        let raw_ocr = lines.join("\n");
        let prompt = generate_extraction_prompt(&raw_ocr, &ctx);

        // The prompt must stay within budget
        assert!(estimate_token_count(&prompt) <= MAX_PROMPT_TOKENS);
        // High-signal items must be preserved
        assert!(prompt.contains("CAMPUS FESTIVAL"));
        assert!(prompt.contains("September 19, 2026"));
        assert!(prompt.contains("12:00 PM - 6:00 PM"));
        assert!(prompt.contains("Academic Quad"));
        assert!(prompt.contains("https://example.com/tickets/long/url/path"));
        // Low-signal noise must be dropped
        assert!(!prompt.contains("@campuslife_official"));
        assert!(!prompt.contains("liked by user123"));
    }

    #[test]
    fn test_token_estimation_and_single_line_truncation() {
        assert_eq!(estimate_token_count(""), 0);
        assert_eq!(estimate_token_count("   "), 0);
        assert!(estimate_token_count("Hello world") > 0);

        let long_line = "A".repeat(5000);
        let trimmed = trim_ocr_text_to_budget(&long_line, 100);
        assert!(estimate_token_count(&trimmed) <= 120);
        assert!(trimmed.ends_with("..."));
    }
}
