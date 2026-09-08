use serde::{Deserialize, Serialize};

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
        if let Some(until) = &self.until {
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

/// Parses weekday abbreviations from strings like "Mo, We" into RFC 5545 BYDAY tokens
pub fn parse_weekdays_to_byday(days_str: &str) -> Vec<String> {
    let mut bydays = Vec::new();
    let parts: Vec<&str> = days_str
        .split([',', '/', '&', ' ', ';', '+'])
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
