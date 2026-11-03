use chrono::{Duration, NaiveDate, NaiveTime, TimeZone};
use crate::parser::datetime::{extract_date, extract_times, is_date_or_time_line};
use crate::parser::heuristics::{clean_location_string, is_url_or_link_line};
use crate::parser::regex::{DATE_LABEL_RE, LEADING_TIME_RANGE_RE, STANDALONE_DATE_RE};
use crate::parser::{EventDetails, ReferenceContext};

/// Heading lines above a date anchor; anything longer is body text, not a title
const MAX_TITLE_LINES: usize = 3;
/// Words in a heading, title or venue line
const MAX_HEADING_WORDS: usize = 6;

/// Parses series flyers: several blocks that each carry their own title, date, time and venue.
///
/// ```text
/// PED 101           <- title: 1-3 heading lines
/// Oct 26 (Mon)      <- anchor: a line that is only a date
/// Noon-1pm          <- optional time line
/// Gund 109          <- optional venue line
/// The presentation  <- description, up to the next block's title
/// ```
///
/// Returns no events unless every anchor has a title, so ordinary flyers fall through to the
/// next strategy. A block with several lines opening with a time range is an agenda, not a series.
pub fn parse_series_events(ocr_text: &str, context: &ReferenceContext) -> Vec<EventDetails> {
    let ref_dt = context.get_reference_datetime();
    let offset = context.get_fixed_offset();
    let lines: Vec<&str> = ocr_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    let anchors: Vec<(usize, NaiveDate)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| STANDALONE_DATE_RE.is_match(l))
        .filter_map(|(i, l)| extract_date(l, ref_dt).map(|d| (i, d)))
        .collect();
    if anchors.len() < 2 {
        return Vec::new();
    }

    struct Block {
        title_start: usize,
        anchor: usize,
        date: NaiveDate,
        times: Option<(NaiveTime, Option<NaiveTime>)>,
        location: Option<String>,
        meta_end: usize,
    }

    let mut blocks: Vec<Block> = Vec::with_capacity(anchors.len());
    for (k, &(anchor, date)) in anchors.iter().enumerate() {
        // Title: heading lines directly above the anchor, never reaching into the previous block's metadata
        let floor = blocks.last().map_or(0, |b| b.meta_end);
        let mut title_start = anchor;
        while title_start > floor && anchor - title_start < MAX_TITLE_LINES {
            let candidate = lines[title_start - 1];
            if !is_heading_line(candidate) {
                break;
            }
            // Single words above the first title line are usually logo fragments ("GSD", "MOBILITY")
            if title_start < anchor && candidate.split_whitespace().count() < 2 {
                break;
            }
            title_start -= 1;
        }
        if title_start == anchor {
            return Vec::new();
        }

        // Metadata: a time line and a venue line in either order, leaving at least one line
        // before the next anchor for that block's title
        let limit = anchors.get(k + 1).map_or(lines.len(), |&(next, _)| next - 1);
        let mut times = None;
        let mut location = None;
        let mut meta_end = anchor + 1;
        while meta_end < limit {
            let line = lines[meta_end];
            if times.is_none() && is_time_line(line) {
                times = extract_times(line);
            } else if location.is_none() && is_heading_line(line) {
                location = Some(clean_location_string(line));
            } else {
                break;
            }
            meta_end += 1;
        }

        blocks.push(Block { title_start, anchor, date, times, location, meta_end });
    }

    let mut events = Vec::with_capacity(blocks.len());
    for (k, block) in blocks.iter().enumerate() {
        let body_end = blocks.get(k + 1).map_or(lines.len(), |b| b.title_start);
        let body = &lines[block.anchor + 1..body_end];
        if body.iter().filter(|l| LEADING_TIME_RANGE_RE.is_match(l)).count() > 1 {
            return Vec::new();
        }

        let description = lines[block.meta_end..body_end]
            .iter()
            .filter(|l| l.split_whitespace().count() >= 2 && !is_url_or_link_line(l))
            .copied()
            .collect::<Vec<_>>()
            .join(" ");

        let (start_time, end_time, is_all_day) = match block.times {
            Some((start, end)) => {
                let start_dt = offset.from_local_datetime(&block.date.and_time(start)).unwrap();
                let end_dt = match end {
                    Some(end) => {
                        let end_date = if end < start { block.date + Duration::days(1) } else { block.date };
                        offset.from_local_datetime(&end_date.and_time(end)).unwrap()
                    }
                    None => start_dt + Duration::hours(1),
                };
                (start_dt.to_rfc3339(), end_dt.to_rfc3339(), false)
            }
            None => {
                let day = block.date.format("%Y-%m-%d").to_string();
                (day.clone(), day, true)
            }
        };

        events.push(EventDetails {
            title: lines[block.title_start..block.anchor].join(" "),
            start_time: Some(start_time),
            end_time: Some(end_time),
            is_all_day,
            location: block.location.clone(),
            description: (!description.is_empty()).then_some(description),
            recurrence_rule: None,
            url: None,
            confidence: 0.90,
            source: "deterministic_series".to_string(),
        });
    }

    events
}

/// Short, title-cased or upper-cased line without sentence punctuation, dates, times or links.
/// "Rain Date" and similar labels name an alternate date, not another event.
fn is_heading_line(line: &str) -> bool {
    line.split_whitespace().count() <= MAX_HEADING_WORDS
        && !line.starts_with(char::is_lowercase)
        && !line.ends_with(['.', ',', ':', ';'])
        && !DATE_LABEL_RE.is_match(line)
        && !is_date_or_time_line(line)
        && !is_url_or_link_line(line)
}

fn is_time_line(line: &str) -> bool {
    LEADING_TIME_RANGE_RE.is_match(line) || (line.split_whitespace().count() <= 3 && extract_times(line).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reference_context() -> ReferenceContext {
        ReferenceContext {
            reference_time: Some("2026-09-06T12:00:00-04:00".to_string()),
            timezone_offset_minutes: Some(-240),
        }
    }

    /// Vision OCR of samples/streets_for_people.jpg, plus the QR footer the image commands append
    const STREETS_FOR_PEOPLE: &str = r#"STREETS FOR PEOPLE
Street Design, Mobility & Public Space
This mini-series explores how the built environment shapes mobility,
safety, and public life - from design principles to implementation and
politics.
Events are free and open to all members of the Harvard community.
GSD
MOBILITY
PED 101
Oct 26 (Mon)
Noon-1pm
Gund 109
The presentation is a primer on aspects of the built environment
(i.e., the streets and sidewalks) that go into creating safe,
walkable places.
walk
massachusetts
INMAN SQUARE
WALKING TOUR
Nov 5 (Thu)
4-5pm
Inman Square
TOOLE
DESIGN
Join us for a walking tour of the recently redesigned Inman
Event is followed by an informal social.
Sign up REQUIRED:
IN THE FACE
OF BIKELASH
Nov 16 (Mon)
Noon-1pm
Gund 109
LOEB
FELLOWSHIP
How do cities deliver ambitious bike infrastructure when
changes in the face of "bikelash."

Links / QR Codes:
https://partiful.com/e/WYOOJ4vsEkMDMzbYl3OB"#;

    #[test]
    fn test_parse_series_flyer_one_event_per_block() {
        let events = parse_series_events(STREETS_FOR_PEOPLE, &sample_reference_context());
        assert_eq!(events.len(), 3);

        assert_eq!(events[0].title, "PED 101");
        assert_eq!(events[0].start_time.as_deref(), Some("2026-10-26T12:00:00-04:00"));
        assert_eq!(events[0].end_time.as_deref(), Some("2026-10-26T13:00:00-04:00"));
        assert_eq!(events[0].location.as_deref(), Some("Gund 109"));
        assert_eq!(
            events[0].description.as_deref(),
            Some("The presentation is a primer on aspects of the built environment (i.e., the streets and sidewalks) that go into creating safe, walkable places.")
        );

        assert_eq!(events[1].title, "INMAN SQUARE WALKING TOUR");
        assert_eq!(events[1].start_time.as_deref(), Some("2026-11-05T16:00:00-04:00"));
        assert_eq!(events[1].end_time.as_deref(), Some("2026-11-05T17:00:00-04:00"));
        assert_eq!(events[1].location.as_deref(), Some("Inman Square"));
        assert_eq!(
            events[1].description.as_deref(),
            Some("Join us for a walking tour of the recently redesigned Inman Event is followed by an informal social. Sign up REQUIRED:")
        );

        assert_eq!(events[2].title, "IN THE FACE OF BIKELASH");
        assert_eq!(events[2].start_time.as_deref(), Some("2026-11-16T12:00:00-04:00"));
        assert_eq!(events[2].location.as_deref(), Some("Gund 109"));
        assert_eq!(
            events[2].description.as_deref(),
            Some("How do cities deliver ambitious bike infrastructure when changes in the face of \"bikelash.\"")
        );

        assert!(events.iter().all(|e| !e.is_all_day && e.recurrence_rule.is_none()));
    }

    #[test]
    fn test_parse_series_without_times_or_bodies_is_all_day() {
        let text = "Open Studio\nSat, Oct 3\nPrint Lab\nSat, Oct 10";
        let events = parse_series_events(text, &sample_reference_context());
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Open Studio");
        assert_eq!(events[0].start_time.as_deref(), Some("2026-10-03"));
        assert!(events[0].is_all_day);
        // The next block's title must not be taken as this block's venue
        assert_eq!(events[0].location, None);
        assert_eq!(events[1].title, "Print Lab");
        assert_eq!(events[1].start_time.as_deref(), Some("2026-10-10"));
    }

    #[test]
    fn test_parse_series_rejects_agenda_and_single_event_text() {
        let ctx = sample_reference_context();
        let agenda = "Developer Summit\nOctober 15\n9:00 AM - 10:30 AM Opening Keynote\n2:00 PM - 4:00 PM Rust Deep Dive\nClosing Day\nOctober 16\n10:00 AM - 11:30 AM Closing Remarks";
        assert!(parse_series_events(agenda, &ctx).is_empty());

        let rain_date = "Harvest Fair\nOct 3\n10am-2pm\nCity Park\nRain Date\nOct 4";
        assert!(parse_series_events(rain_date, &ctx).is_empty());

        let untitled_second_date = "Harvest Fair\nOct 3\n10am-2pm\nCity Park\nbring a friend.\nOct 4";
        assert!(parse_series_events(untitled_second_date, &ctx).is_empty());
    }
}
