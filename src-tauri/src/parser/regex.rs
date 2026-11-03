use regex::Regex;
use std::sync::LazyLock;

pub(crate) static COURSE_CODE_ANCHORED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{2,6}[-\s]\d{3,4}").unwrap());
pub(crate) static COURSE_CODE_ROW_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z]{2,6}\s+\d{3,4}(?:-\d{2,3})?(?:\s*\(\d+\))?)").unwrap());
pub(crate) static COURSE_CODE_COLUMNAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z]{2,6}\s+\d{3,4}(?:-\d{2,3})?(?:\s*\(\d+\))?)\b").unwrap());

pub(crate) static DAY_PATTERN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b((?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun)(?:\s*,\s*(?:Mo|Tu|We|Th|Fr|Sa|Su|Mon|Tue|Tues|Wed|Thu|Thur|Thurs|Fri|Sat|Sun))*)\b").unwrap()
});

pub(crate) static TIME_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?)\s*(?:-|–|—|to)\s*(\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.))\b").unwrap()
});

pub(crate) static YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(20\d{2})\b").unwrap());

pub(crate) static FACULTY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([A-Z]\.\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\b").unwrap());

pub(crate) static UNITS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d+\.\d{2})\b").unwrap());

pub(crate) static SECTION_NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d{4,6})\)").unwrap());

pub(crate) static COURSE_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\((Lecture|Seminar|Lab|Discussion|Recitation|Studio|Practicum|Workshop|Lecture/Lab)\)").unwrap()
});

pub(crate) static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(Description|Class\s+Description|Course\s+Name|Title)$").unwrap()
});

pub(crate) static PAREN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(([^)]+)\)").unwrap());

pub(crate) static REL_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(today|tomorrow|this\s+(?:mon|tues|wed|thu|thur|thurs|fri|sat|sun)(?:day)?|next\s+(?:mon|tues|wed|thu|thur|thurs|fri|sat|sun)(?:day)?)\b").unwrap()
});

pub(crate) static DAY_MONTH_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:(mon|tue|wed|thu|fri|sat|sun)[a-z]*\s+)?(\d{1,2})(?:st|nd|rd|th)?\s+(?:of\s+)?(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*(?:\s*,?\s*(\d{4}|\d{2}))?\b").unwrap()
});

pub(crate) static MONTH_DAY_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:(mon|tue|wed|thu|fri|sat|sun)[a-z]*\s*,?\s*)?(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+(\d{1,2})(?:st|nd|rd|th)?(?:\s*,?\s*(\d{4}|\d{2}))?\b").unwrap()
});

pub(crate) static NUMERIC_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d{4})[/\.-](\d{1,2})[/\.-](\d{1,2})\b|\b(?:(mon|tue|wed|thu|fri|sat|sun)[a-z.]*\s*,?\s*)?(\d{1,2})[/\.](\d{1,2})(?:[/\.](\d{2,4}))?\b").unwrap()
});

pub(crate) static TIME_RANGE_EXTRACT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?\s*(?:-|–|—|to|until)\s*(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)\b").unwrap()
});

pub(crate) static TIME_24H_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{1,2}):(\d{2})\s*(?:-|–|—|to)\s*(\d{1,2}):(\d{2})\b").unwrap()
});

/// "noon" / "midnight" written as a range endpoint ("Noon-1pm", "10pm-midnight")
pub(crate) static NOON_MIDNIGHT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(noon|midnight)\b").unwrap());

/// A line that is only a date, optionally with a weekday before it or in parentheses after it
/// ("Oct 26 (Mon)", "Thursday, November 5", "16 Nov 2026")
pub(crate) static STANDALONE_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?,?\s+)?(?:(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}(?:st|nd|rd|th)?\s+(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?)(?:,?\s+\d{4})?\s*(?:\(\s*(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?\s*\))?$").unwrap()
});

/// A line that opens with a time range ("4-5pm", "Noon-1pm", "7:00 PM – 9:00 PM: Talk")
pub(crate) static LEADING_TIME_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?|noon|midnight)\s*(?:-|–|—|to)\s*(?:\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)?|noon|midnight)\b").unwrap()
});

/// "Rain Date", "Snow date", "Save the Date": a label for a date, not an event title
pub(crate) static DATE_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bdates?\b").unwrap());

pub(crate) static TIME_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:at|@|starts?|begins?|time:?)\s*(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)\b|\b(\d{1,2})(?::(\d{2}))\s*(am|pm|a\.m\.|p\.m\.)\b").unwrap()
});

pub(crate) static EXPLICIT_MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:this year at|held at|venue:|location:|where:|place:|live at|takes place at)\s*(.*)").unwrap()
});

pub(crate) static CONTINUATION_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:st|street|ave|avenue|blvd|boulevard|rd|road|dr|drive|way|lane|ln|ct|court|pl|plaza|pkwy|parkway|park|room|hall|building|center|centre|auditorium|and)\b").unwrap()
});

pub(crate) static STRONG_VENUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:park|square|hall|center|centre|auditorium|ballroom|plaza|room|club|theatre|theater|bar|grill|cafe|brewery|church|library|museum|garden|gardens|stadium|arena|field|house|lounge|hq|studio|building|bldg|tower|gallery|pavilion|lawn|tavern|pub|amphitheater|stage|zoom|teams|webex|discord)\b").unwrap()
});

pub(crate) static STREET_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:street|st\.?|avenue|ave\.?|boulevard|blvd\.?|road|rd\.?|drive|dr\.?|lane|ln\.?|way|court|ct\.?|place|pl\.?|highway|hwy\.?|parkway|pkwy\.?)\b").unwrap()
});

pub(crate) static CITY_STATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z][a-zA-Z\s]+,\s*[A-Z]{2}\b").unwrap());

pub(crate) static ROOM_PATTERN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:room|suite|ste|rm|bldg|building|hall|wing|apt)\.?\s+[A-Za-z0-9#-]+|\b\d{1,4}[A-Z]?\b").unwrap()
});

pub(crate) static TIME_12H_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\d{1,2}(?::\d{2})?\s*(?:am|pm|a\.m\.|p\.m\.)\b|\b\d{1,2}\s*(?:-|–|—|to)\s*\d{1,2}\s*(?:am|pm|a\.m\.|p\.m\.)\b").unwrap()
});

pub(crate) static TIME_24H_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{1,2}:\d{2}\b").unwrap());

pub(crate) static NUM_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{1,2}/\d{1,2}(?:/\d{2,4})?\b|\b\d{4}-\d{1,2}-\d{1,2}\b").unwrap()
});

pub(crate) static MONTH_DAY_PATTERN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?\b|\b\d{1,2}(?:st|nd|rd|th)?\s+(?:of\s+)?(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\b").unwrap()
});

pub(crate) static WEEKDAY_DAY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?\b").unwrap()
});

pub(crate) static PHONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}[-.)\s]+\d{3}[-.\s]+\d{4}\b|\b311\b").unwrap());

pub(crate) static NOTE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:rain date|note|info|details|rsvp|featuring|special guests?|doors|remote viewers|faculty|bio|about|admission|all ages|free admission):?\s*|\b(?:rain date)\b").unwrap()
});

pub(crate) static SLOGAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]{3,}\.\s+[A-Z]{3,}\.").unwrap());

pub(crate) static TOUR_SUBTITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:tour|summit|conference|symposium|workshop|webinar|expo|exhibition|series|session|festival)\b").unwrap()
});

pub(crate) static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bhttps?://[^\s<>"{}|\\^`\[\]]+"#).unwrap()
});

pub(crate) static WWW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bwww\.[a-z0-9\-]+(\.[a-z0-9\-]+)+[^\s<>"{}|\\^`\[\]]*"#).unwrap()
});
