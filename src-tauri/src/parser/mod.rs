use chrono::{
    DateTime, Duration, FixedOffset, Local, NaiveDateTime, Offset, TimeZone,
};
use serde::{Deserialize, Serialize};

pub mod agenda;
pub mod datetime;
pub mod heuristics;
pub mod regex;
pub mod rrule;
pub mod schedule;
pub mod schema;

pub use agenda::parse_agenda_events;
pub use datetime::{
    extract_date, extract_term_until_date, extract_times, get_weekday_date, is_date_or_time_line,
    is_date_pattern, is_time_pattern, month_to_num, parse_hour_min, resolve_year, weekday_to_num,
};
pub use heuristics::{
    clean_location_string, extract_description, extract_location, extract_title,
    is_noise_or_metadata_line,
};
pub use rrule::{parse_weekdays_to_byday, RecurrenceRule};
pub use schedule::{
    clean_course_name, extract_columnar_descriptions, parse_columnar_schedule_table_events,
    parse_row_schedule_table_events, parse_schedule_table_events,
};
pub use schema::{
    estimate_token_count, generate_extraction_prompt, get_gbnf_grammar, get_json_schema,
    trim_ocr_text_to_budget, MAX_PROMPT_TOKENS,
};

use regex::{COURSE_CODE_ANCHORED_RE, DAY_PATTERN_RE, TIME_RANGE_RE, URL_RE, WWW_RE};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub confidence: f32,
    pub source: String,
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
            *self.get_reference_datetime().offset()
        }
    }
}

/// Deterministic fallback parser implementing multi-event schedule, agenda, and single-event parsing
pub fn parse_events_deterministic(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let lines: Vec<&str> = ocr_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let doc_url = extract_url(&lines, ocr_text);

    // 1. Try class schedule table parsing
    let mut schedule_events = parse_schedule_table_events(ocr_text, context);
    if schedule_events.len() >= 2 {
        if let Some(u) = &doc_url {
            for ev in &mut schedule_events {
                if ev.url.is_none() {
                    ev.url = Some(u.clone());
                }
            }
        }
        return schedule_events;
    }

    // 2. Try generic agenda / multi-event line parsing
    let mut agenda_events = parse_agenda_events(ocr_text, context);
    if agenda_events.len() >= 2 {
        if let Some(u) = &doc_url {
            for ev in &mut agenda_events {
                if ev.url.is_none() {
                    ev.url = Some(u.clone());
                }
            }
        }
        return agenda_events;
    }

    // 3. Fallback to single event parsing
    let mut single = parse_single_event_deterministic(ocr_text, context);
    if single.url.is_none() && doc_url.is_some() {
        single.url = doc_url;
    }
    vec![single]
}

/// Compatibility wrapper returning the primary or first extracted event
pub fn parse_event_deterministic(ocr_text: &str, context: &ReferenceContext) -> EventDetails {
    let events = parse_events_deterministic(ocr_text, context);
    events.into_iter().next().unwrap_or_else(|| parse_single_event_deterministic(ocr_text, context))
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

    // Find valid weekday pattern in context (gated by time line, course code, or explicit repeat keywords)
    let mut matched_days_str: Option<String> = None;
    let mut is_on_time_line = false;

    // A. Check line with time range or time pattern
    for line in &lines {
        if TIME_RANGE_RE.is_match(line) || is_time_pattern(line) {
            if let Some(dm) = DAY_PATTERN_RE.find(line) {
                let s = dm.as_str().trim();
                let bydays = parse_weekdays_to_byday(s);
                let lower = s.to_lowercase();
                let is_ambiguous = (lower == "we" || lower == "sun" || lower == "sat" || lower == "mon" || lower == "th")
                    && line.split_whitespace().count() > 4
                    && bydays.len() == 1;
                if !is_ambiguous && !bydays.is_empty() {
                    matched_days_str = Some(s.to_string());
                    is_on_time_line = true;
                    break;
                }
            }
        }
    }

    // B. Check lines with explicit schedule context / course codes / repeat keywords
    if matched_days_str.is_none() {
        for line in &lines {
            let l_lower = line.to_lowercase();
            let is_schedule_context = COURSE_CODE_ANCHORED_RE.is_match(line)
                || l_lower.contains("days")
                || l_lower.contains("schedule")
                || l_lower.contains("every")
                || l_lower.contains("weekly")
                || l_lower.contains("repeats")
                || l_lower.contains("recurring")
                || l_lower.contains("starts")
                || l_lower.contains("faculty:")
                || l_lower.contains("units:");
            if is_schedule_context {
                if let Some(dm) = DAY_PATTERN_RE.find(line) {
                    let s = dm.as_str().trim();
                    let bydays = parse_weekdays_to_byday(s);
                    if !bydays.is_empty() {
                        matched_days_str = Some(s.to_string());
                        break;
                    }
                }
            }
        }
    }

    // C. Check multi-day pattern anywhere in ocr_text (e.g. "Mo, We")
    if matched_days_str.is_none() {
        for m in DAY_PATTERN_RE.find_iter(ocr_text) {
            let candidate = m.as_str().trim();
            let bydays = parse_weekdays_to_byday(candidate);
            if bydays.len() >= 2 {
                matched_days_str = Some(candidate.to_string());
                break;
            }
        }
    }

    // D. If date is absent, check short non-prose lines with day names
    if matched_days_str.is_none() && extracted_date.is_none() {
        for line in &lines {
            let t = line.trim();
            if let Some(dm) = DAY_PATTERN_RE.find(t) {
                let s = dm.as_str().trim();
                let bydays = parse_weekdays_to_byday(s);
                let lower = s.to_lowercase();
                let is_ambiguous = (lower == "we" || lower == "sun" || lower == "sat" || lower == "mon" || lower == "th") && t.split_whitespace().count() > 3;
                if !bydays.is_empty() && !is_ambiguous && (t.len() <= 25 || s.len() >= 4) {
                    matched_days_str = Some(s.to_string());
                    break;
                }
            }
        }
    }

    let (weekday_days_opt, single_recurrence_rule) = if let Some(days_str) = matched_days_str {
        let bydays = parse_weekdays_to_byday(&days_str);
        let first_day = days_str.split(',').next().map(|s| s.trim()).unwrap_or(&days_str).to_string();

        let text_lower = ocr_text.to_lowercase();
        let has_explicit_repeat = text_lower.contains("every")
            || text_lower.contains("weekly")
            || text_lower.contains("repeats")
            || text_lower.contains("recurring")
            || bydays.len() >= 2;

        let has_course_code = lines.iter().any(|l| COURSE_CODE_ANCHORED_RE.is_match(l));
        let has_academic_context = has_course_code
            || text_lower.contains("faculty:")
            || text_lower.contains("units:")
            || text_lower.contains("in cart")
            || text_lower.contains("cross-listed");

        let is_recurring = if extracted_date.is_some() {
            has_explicit_repeat || (has_course_code && (text_lower.contains("starts") || text_lower.contains("begins") || bydays.len() >= 2))
        } else {
            has_explicit_repeat || has_academic_context || (is_on_time_line && !bydays.is_empty())
        };
        let rrule = if is_recurring && !bydays.is_empty() {
            Some(RecurrenceRule::new_weekly(bydays, None).to_rrule_string())
        } else {
            None
        };

        (Some(first_day), rrule)
    } else {
        (None, None)
    };

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
            let target_date = if let Some(first_day) = &weekday_days_opt {
                get_weekday_date(first_day, ref_dt).unwrap_or_else(|| ref_dt.date_naive())
            } else {
                let mut d = ref_dt.date_naive();
                if start_time < ref_dt.time() {
                    d += Duration::days(1);
                }
                d
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
        }
        (None, None) => (None, None, false),
    };

    // 3. Location Extraction
    let location = extract_location(&lines, ocr_text);

    // 4. Title Extraction
    let title = extract_title(&lines, ocr_text);

    // 5. Description Extraction
    let description = extract_description(&lines, &title, location.as_deref());

    // 6. URL Extraction
    let url = extract_url(&lines, ocr_text);

    // 7. Confidence calculation
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
        recurrence_rule: single_recurrence_rule,
        url,
        confidence,
        source: "deterministic".to_string(),
    }
}

pub fn clean_extracted_url(s: &str) -> String {
    s.trim()
        .trim_end_matches(['.', ',', ';', ')', '>', ']', '\'', '"'])
        .to_string()
}

/// Extracts a web link or URL from text or labeled lines
pub fn extract_url(lines: &[&str], text: &str) -> Option<String> {
    // 1. Check labeled lines (Links / QR Codes:, URL:, Link:, Website:, Zoom:, RSVP:)
    for (i, line) in lines.iter().enumerate() {
        let lower = line.to_lowercase();
        if lower.starts_with("links / qr codes:")
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
            if let Some(pos) = line.find(':') {
                let candidate = line[pos + 1..].trim();
                if candidate.starts_with("http://") || candidate.starts_with("https://") {
                    return Some(clean_extracted_url(candidate));
                }
            }
            // If label is on its own line, check subsequent lines for a link
            if i + 1 < lines.len() {
                let next = lines[i + 1].trim();
                if next.starts_with("http://") || next.starts_with("https://") {
                    return Some(clean_extracted_url(next));
                }
            }
        }
    }
    // 2. Check if text has any explicit http:// or https:// URL
    if let Some(m) = URL_RE.find(text) {
        let candidate = clean_extracted_url(m.as_str());
        if !candidate.is_empty() {
            return Some(candidate);
        }
    }
    // 3. Check for www. web links
    if let Some(m) = WWW_RE.find(text) {
        let candidate = clean_extracted_url(m.as_str());
        if !candidate.is_empty() {
            return Some(format!("https://{}", candidate));
        }
    }
    None
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

    #[test]
    fn test_parse_recurring_card_with_explicit_date_and_weekdays() {
        let text = "CS 101 Intro to Computer Science\nStarts Sep 8 · Mo, We 1:20 PM - 4:20 PM\nScience Center 105";
        let ctx = sample_reference_context(); // Reference Sunday 2026-09-06
        let event = parse_event_deterministic(text, &ctx);

        assert!(event.title.contains("CS 101"));
        assert_eq!(event.start_time.as_deref(), Some("2026-09-08T13:20:00-04:00"));
        assert_eq!(event.end_time.as_deref(), Some("2026-09-08T16:20:00-04:00"));
        assert_eq!(event.recurrence_rule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));
    }

    #[test]
    fn test_parse_commons_flyer_deterministic() {
        let ocr_text = r#"CAMPUS AS COMMONS:
Agroforestry and Shared Stewardship at Tufts
Mary Mattingly
Visiting Artist, Center for the Humanities at Tufts
Luits
UNIVERSITY
School of Arts and Sciences
Environmental Studies
Hoch Cunningham Environmental Lectures
Architectural Studies Program
Center for the Humanities at Tufts, University Ecologies
THURS. 9/10, 12-1PM
Curtis Hall Multipurpose Room
Remote viewers: go.tufts.edu/HOCU0910"#;

        let ctx = sample_reference_context(); // Reference 2026-09-06
        let event = parse_event_deterministic(ocr_text, &ctx);

        assert!(event.title.contains("CAMPUS AS COMMONS") && event.title.contains("Agroforestry"));
        assert_eq!(event.start_time.as_deref(), Some("2026-09-10T12:00:00-04:00"));
        assert_eq!(event.end_time.as_deref(), Some("2026-09-10T13:00:00-04:00"));
        assert!(!event.is_all_day);
        assert_eq!(event.location.as_deref(), Some("Curtis Hall Multipurpose Room"));
        assert!(event.description.is_some());
        let desc = event.description.unwrap();
        assert!(desc.contains("Mary Mattingly") || desc.contains("Remote viewers"));
        assert_eq!(event.recurrence_rule, None, "One-off lecture flyer should not have recurrence");
    }

    #[test]
    fn test_day_matching_gated_against_prose() {
        let text = "Join Us for Autumn Celebration\nFriday, September 11, 2026\n7:00 PM - 10:00 PM\nLincoln Center\nWe invite all friends and family to join us on this special day. We will have food and games!";
        let ctx = sample_reference_context();
        let event = parse_event_deterministic(text, &ctx);

        assert_eq!(event.start_time.as_deref(), Some("2026-09-11T19:00:00-04:00"));
        assert_eq!(event.recurrence_rule, None, "Prose containing 'We' should not fabricate recurrence rule");
    }

    #[test]
    fn test_event_details_url_serde_backward_compatibility() {
        // Legacy JSON without url field
        let legacy_json = r#"{
            "title": "Old Event",
            "start_time": "2026-09-10T12:00:00Z",
            "end_time": "2026-09-10T13:00:00Z",
            "is_all_day": false,
            "location": "Room 101",
            "description": "Legacy event description",
            "confidence": 0.9,
            "source": "deterministic"
        }"#;
        let event: EventDetails = serde_json::from_str(legacy_json).expect("Legacy JSON should deserialize");
        assert_eq!(event.title, "Old Event");
        assert_eq!(event.url, None);

        // Serializing with url: None should omit url field (skip_serializing_if = Option::is_none)
        let serialized_none = serde_json::to_string(&event).expect("Serialize none");
        assert!(!serialized_none.contains("\"url\""));

        // Modern JSON with url field
        let modern_json = r#"{
            "title": "Webinar Event",
            "start_time": "2026-09-10T12:00:00Z",
            "end_time": "2026-09-10T13:00:00Z",
            "is_all_day": false,
            "location": "Online",
            "description": "Webinar",
            "url": "https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw",
            "confidence": 0.95,
            "source": "deterministic"
        }"#;
        let modern_event: EventDetails = serde_json::from_str(modern_json).expect("Modern JSON should deserialize");
        assert_eq!(
            modern_event.url.as_deref(),
            Some("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw")
        );

        // Round-trip serialization
        let serialized_some = serde_json::to_string(&modern_event).expect("Serialize some");
        assert!(serialized_some.contains("\"url\":\"https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw\""));
        let roundtrip: EventDetails = serde_json::from_str(&serialized_some).expect("Deserialize roundtrip");
        assert_eq!(modern_event.url, roundtrip.url);
    }

    #[test]
    fn test_parse_commons_flyer_with_qr_codes_section_deterministic() {
        let ocr_text = r#"CAMPUS AS COMMONS:
Agroforestry and Shared Stewardship at Tufts
Mary Mattingly
Visiting Artist, Center for the Humanities at Tufts
Luits
UNIVERSITY
School of Arts and Sciences
Environmental Studies
Hoch Cunningham Environmental Lectures
Architectural Studies Program
Center for the Humanities at Tufts, University Ecologies
THURS. 9/10, 12-1PM
Curtis Hall Multipurpose Room
Remote viewers: go.tufts.edu/HOCU0910

Links / QR Codes:
https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw"#;

        let ctx = sample_reference_context(); // Reference 2026-09-06
        let event = parse_event_deterministic(ocr_text, &ctx);

        assert!(event.title.contains("CAMPUS AS COMMONS") && event.title.contains("Agroforestry"));
        assert_eq!(event.start_time.as_deref(), Some("2026-09-10T12:00:00-04:00"));
        assert_eq!(event.end_time.as_deref(), Some("2026-09-10T13:00:00-04:00"));
        assert!(!event.is_all_day);
        assert_eq!(event.location.as_deref(), Some("Curtis Hall Multipurpose Room"));
        assert_eq!(
            event.url.as_deref(),
            Some("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw")
        );
        assert!(event.description.is_some());
        let desc = event.description.unwrap();
        assert!(desc.contains("Mary Mattingly") || desc.contains("Remote viewers"));
        assert!(!desc.contains("Links / QR Codes"));
    }

    #[test]
    fn test_extract_url_patterns() {
        let text1 = "Event\nLinks / QR Codes: https://example.com/webinar/register";
        let lines1: Vec<&str> = text1.lines().collect();
        assert_eq!(
            extract_url(&lines1, text1).as_deref(),
            Some("https://example.com/webinar/register")
        );

        let text2 = "Event\nLinks / QR Codes:\nhttps://example.com/zoom/meeting";
        let lines2: Vec<&str> = text2.lines().collect();
        assert_eq!(
            extract_url(&lines2, text2).as_deref(),
            Some("https://example.com/zoom/meeting")
        );

        let text3 = "Meeting in Room 3B (see https://tufts.zoom.us/j/123456789).";
        let lines3: Vec<&str> = text3.lines().collect();
        assert_eq!(
            extract_url(&lines3, text3).as_deref(),
            Some("https://tufts.zoom.us/j/123456789")
        );

        let text4 = "Register at www.communitygarden.org/events/rsvp, today!";
        let lines4: Vec<&str> = text4.lines().collect();
        assert_eq!(
            extract_url(&lines4, text4).as_deref(),
            Some("https://www.communitygarden.org/events/rsvp")
        );
    }
}
