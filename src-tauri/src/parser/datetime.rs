use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveTime,
};
use crate::parser::regex::{
    DAY_MONTH_DATE_RE, MONTH_DAY_DATE_RE, MONTH_DAY_PATTERN_RE, NUMERIC_DATE_RE, NUM_DATE_RE,
    REL_DATE_RE, TIME_12H_RE, TIME_24H_RANGE_RE, TIME_24H_RE, TIME_RANGE_EXTRACT_RE,
    TIME_SINGLE_RE, WEEKDAY_DAY_RE, YEAR_RE,
};

pub fn month_to_num(m: &str) -> Option<u32> {
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

pub fn weekday_to_num(w: &str) -> Option<chrono::Weekday> {
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

pub fn resolve_year(month: u32, day: u32, weekday_opt: Option<chrono::Weekday>, ref_dt: DateTime<FixedOffset>) -> i32 {
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

pub fn extract_date(text: &str, ref_dt: DateTime<FixedOffset>) -> Option<NaiveDate> {
    let clean_text = text.replace('\n', " ");

    // Relative dates: "today", "tomorrow", "this friday", "next monday"
    if let Some(caps) = REL_DATE_RE.captures(&clean_text) {
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
                        d += Duration::days(7);
                    }
                    for _ in 0..7 {
                        if d.weekday() == target_w {
                            return Some(d);
                        }
                        d += Duration::days(1);
                    }
                }
            }
        }
    }

    // Pattern 1: Day of week + Day + Month (e.g. "SATURDAY 12 September" or "SATURDAY 12\nSeptember")
    if let Some(caps) = DAY_MONTH_DATE_RE.captures(&clean_text) {
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
    if let Some(caps) = MONTH_DAY_DATE_RE.captures(&clean_text) {
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

    // Pattern 3: Numeric date YYYY-MM-DD, MM/DD/YYYY, MM/DD/YY, MM.DD.YY, or MM/DD (e.g. "9/10", "THURS. 9/10")
    if let Some(caps) = NUMERIC_DATE_RE.captures(&clean_text) {
        if let (Some(y), Some(m), Some(d)) = (caps.get(1), caps.get(2), caps.get(3)) {
            let year: i32 = y.as_str().parse().ok()?;
            let month: u32 = m.as_str().parse().ok()?;
            let day: u32 = d.as_str().parse().ok()?;
            if let Some(res) = NaiveDate::from_ymd_opt(year, month, day) {
                return Some(res);
            }
        } else if let (Some(m_cap), Some(d_cap)) = (caps.get(5), caps.get(6)) {
            let month: u32 = m_cap.as_str().parse().ok()?;
            let day: u32 = d_cap.as_str().parse().ok()?;
            let weekday_opt = caps.get(4).and_then(|w| weekday_to_num(w.as_str()));
            if (1..=12).contains(&month) && (1..=31).contains(&day) {
                let year = if let Some(y_cap) = caps.get(7) {
                    let mut y: i32 = y_cap.as_str().parse().ok()?;
                    if y < 100 {
                        y += 2000;
                    }
                    y
                } else {
                    resolve_year(month, day, weekday_opt, ref_dt)
                };
                if let Some(res) = NaiveDate::from_ymd_opt(year, month, day) {
                    return Some(res);
                }
            }
        }
    }
    None
}

pub fn parse_hour_min(hour_str: &str, min_str: Option<&str>, ampm_str: Option<&str>, default_is_pm: bool) -> Option<NaiveTime> {
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

pub fn extract_times(text: &str) -> Option<(NaiveTime, Option<NaiveTime>)> {
    let clean_text = text.replace('\n', " ");

    // Pattern 1: Time range like "12-5pm", "12:00 PM - 5:00 PM", "7:00pm - 10:00pm", "12pm - 5pm", "10am to 2pm"
    if let Some(caps) = TIME_RANGE_EXTRACT_RE.captures(&clean_text) {
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
            if end_is_pm {
                if start_h > end_h && (8..=11).contains(&start_h) {
                    // E.g. "9 - 2pm" -> 9am - 2pm
                    Some("am")
                } else {
                    Some("pm")
                }
            } else {
                end_ampm_str
            }
        };

        let start_time = parse_hour_min(start_hour_str, start_min_str, start_ampm, false)?;
        let end_time = parse_hour_min(end_hour_str, end_min_str, end_ampm_str, false)?;

        return Some((start_time, Some(end_time)));
    }

    // Pattern 2: 24h range "18:00 - 21:30"
    if let Some(caps) = TIME_24H_RANGE_RE.captures(&clean_text) {
        let s_h: u32 = caps.get(1).unwrap().as_str().parse().ok()?;
        let s_m: u32 = caps.get(2).unwrap().as_str().parse().ok()?;
        let e_h: u32 = caps.get(3).unwrap().as_str().parse().ok()?;
        let e_m: u32 = caps.get(4).unwrap().as_str().parse().ok()?;

        let start_time = NaiveTime::from_hms_opt(s_h, s_m, 0)?;
        let end_time = NaiveTime::from_hms_opt(e_h, e_m, 0)?;
        return Some((start_time, Some(end_time)));
    }

    // Pattern 3: Single time "at 7:00 PM", "doors at 8pm", "starts at 6:30pm", "7pm", "19:00"
    if let Some(caps) = TIME_SINGLE_RE.captures(&clean_text) {
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

/// Calculates target weekday date for recurring schedule items
pub fn get_weekday_date(day_abbr: &str, ref_dt: DateTime<FixedOffset>) -> Option<NaiveDate> {
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
    
    // If target date is before ref_date, advance to next week
    if target_date < ref_date {
        target_date += Duration::days(7);
    }

    Some(target_date)
}

/// Extracts academic term end date for UNTIL recurrence rule
pub fn extract_term_until_date(text: &str, ref_dt: DateTime<FixedOffset>) -> String {
    let lower = text.to_lowercase();
    let year = if let Some(caps) = YEAR_RE.captures(text) {
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

pub fn is_time_pattern(s: &str) -> bool {
    TIME_12H_RE.is_match(s) || TIME_24H_RE.is_match(s)
}

pub fn is_date_pattern(s: &str) -> bool {
    if NUM_DATE_RE.is_match(s) {
        return true;
    }
    MONTH_DAY_PATTERN_RE.is_match(s) || WEEKDAY_DAY_RE.is_match(s)
}

pub fn is_date_or_time_line(s: &str) -> bool {
    is_time_pattern(s) || is_date_pattern(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_weekday_date_tight_comparison() {
        // Tuesday Sep 8, 2026 reference
        let tuesday_ref = DateTime::parse_from_rfc3339("2026-09-08T12:00:00-04:00").unwrap();

        // Monday should resolve to next Monday (Sep 14, 2026), NOT yesterday (Sep 7, 2026)
        let monday = get_weekday_date("monday", tuesday_ref).unwrap();
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 9, 14).unwrap());

        // Wednesday should resolve to tomorrow (Sep 9, 2026)
        let wednesday = get_weekday_date("wed", tuesday_ref).unwrap();
        assert_eq!(wednesday, NaiveDate::from_ymd_opt(2026, 9, 9).unwrap());
    }
}
