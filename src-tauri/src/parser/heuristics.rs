use crate::parser::datetime::is_date_or_time_line;
use crate::parser::regex::{
    CITY_STATE_RE, CONTINUATION_TOKEN_RE, COURSE_CODE_ANCHORED_RE, EXPLICIT_MARKER_RE,
    NOTE_PREFIX_RE, PHONE_RE, ROOM_PATTERN_RE, SLOGAN_RE, STREET_SUFFIX_RE, STRONG_VENUE_RE,
    TOUR_SUBTITLE_RE,
};

pub fn clean_location_string(s: &str) -> String {
    let mut cleaned = s.trim().trim_matches(|c: char| c == ',' || c == '.' || c == ':' || c == '-' || c == '*').trim().to_string();
    cleaned = cleaned.replace("  ", " ");
    cleaned
}

pub fn is_noise_or_metadata_line(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.len() < 3 {
        return true;
    }
    if PHONE_RE.is_match(trimmed) {
        return true;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit() || c.is_ascii_punctuation() || c.is_whitespace()) {
        return true;
    }
    let lower = trimmed.to_lowercase();
    if lower == "follow" || lower == "following" || lower == "followers"
        || lower.starts_with("liked by ") || lower.ends_with("days ago")
        || lower.ends_with("hours ago") || lower.ends_with("mins ago")
        || lower == "more" || lower == "less"
        || lower.ends_with("... more") || lower.ends_with("… more") || lower.ends_with(" more")
        || lower.ends_with("...more") || lower.ends_with("…more") {
        return true;
    }
    if (lower.contains("accommodations") || lower.contains("interpreter") || lower.contains("contact")) && lower.contains("311") {
        return true;
    }
    false
}

pub fn extract_location(lines: &[&str], _full_text: &str) -> Option<String> {
    // 1. Explicit marker: "This year at ...", "Location: ...", "Venue: ...", "Where: ..."
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = EXPLICIT_MARKER_RE.captures(line) {
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
                if CONTINUATION_TOKEN_RE.is_match(next_line) && !loc.contains(next_line) {
                    loc = format!("{}, {}", loc, next_line);
                }
                extra_idx += 1;
            }

            let cleaned = clean_location_string(&loc);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    // 2. Structural & keyword candidate scoring for venue / room / address lines
    let known_cities = ["BOSTON", "SOMERVILLE", "CAMBRIDGE", "BROOKLINE", "MEDFORD", "NEW YORK", "NYC"];

    let mut candidate_locations: Vec<(String, usize, i32)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Skip obvious header, metadata, or pure date/time lines
        if trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•')
            || upper.contains("FOR ADA") || upper.contains("ACCOMMODATIONS") || upper.contains("311")
            || upper.contains("RAIN DATE") || upper.contains("ART BY:") || upper.len() < 3
            || is_date_or_time_line(&upper) || is_noise_or_metadata_line(trimmed) {
            continue;
        }

        let has_strong_venue = STRONG_VENUE_RE.is_match(trimmed);
        let has_street_suffix = STREET_SUFFIX_RE.is_match(trimmed);
        let has_city_state = CITY_STATE_RE.is_match(trimmed) || known_cities.iter().any(|&c| upper.contains(c));
        let has_room = ROOM_PATTERN_RE.is_match(trimmed);

        if !has_strong_venue && !has_street_suffix && !has_city_state && !has_room {
            continue;
        }

        let mut score: i32 = 0;
        if has_strong_venue {
            score += 30;
        }
        if has_room {
            score += 30;
        }
        if has_street_suffix {
            score += 25;
        }
        if has_city_state {
            score += 25;
        }

        // Boost if immediately following a date or time line
        if i > 0 && is_date_or_time_line(lines[i - 1]) {
            score += 30;
        }

        // Penalize top header lines (idx 0 or 1) that are all-caps or end with a colon (e.g. "CAMPUS AS COMMONS:" or "SOMERSTREETS")
        if i <= 1 && (trimmed.ends_with(':') || (!has_room && !has_street_suffix && !CITY_STATE_RE.is_match(trimmed))) {
            score -= 40;
        }

        let mut loc = trimmed.to_string();

        // Check if next line is an address, room, or city continuation
        if i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            let next_upper = next.to_uppercase();
            let is_next_location_part = STREET_SUFFIX_RE.is_match(next)
                || CITY_STATE_RE.is_match(next)
                || ROOM_PATTERN_RE.is_match(next)
                || known_cities.iter().any(|&c| next_upper.contains(c))
                || (next_upper.contains(',') && !next_upper.contains("INVITE") && !next_upper.contains("WE "));

            if is_next_location_part
                && !next.starts_with('*')
                && !is_date_or_time_line(&next_upper)
                && !is_noise_or_metadata_line(next)
                && !next_upper.contains("INVITE")
                && !next_upper.contains("DESSERT") {
                loc = format!("{}, {}", loc, next);
                score += 20;
            }
        }

        let cleaned = clean_location_string(&loc);
        if !cleaned.is_empty() {
            candidate_locations.push((cleaned, i, score));
        }
    }

    if let Some((best_loc, _, _)) = candidate_locations.into_iter().max_by_key(|item| item.2) {
        return Some(best_loc);
    }

    None
}

pub fn extract_title(lines: &[&str], _full_text: &str) -> String {
    let event_keywords = [
        "FESTIVAL", "CONCERT", "PARTY", "CELEBRATION", "MEETUP", "MEETING",
        "CONFERENCE", "SUMMIT", "WORKSHOP", "SYMPOSIUM", "WEBINAR", "SHOW",
        "EXHIBITION", "FAIR", "GALA", "DINNER", "BRUNCH", "FUNDRAISER",
        "PARADE", "MARKET", "BLOCK PARTY", "OPEN MIC", "GAME NIGHT", "TRIVIA",
        "BBQ", "COOKOUT", "LAUNCH", "BIRTHDAY", "WEDDING", "SOCIAL", "ICE CREAM", "RECEPTION",
        "RIDE", "RALLY", "WALK", "RUN", "MARATHON", "TOUR", "RACE",
        "SEMINAR", "LECTURE", "CLASS", "COURSE", "TALK", "PANEL",
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
        // Boost for course codes at start of line
        if COURSE_CODE_ANCHORED_RE.is_match(trimmed) {
            score += 35;
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
            } else {
                let prev_clean = prev.trim_end_matches(':');
                if (prev == prev_upper || prev.ends_with(':'))
                    && !prev.starts_with('*') && !prev.starts_with('-')
                    && !is_noise_or_metadata_line(prev) && !is_date_or_time_line(prev)
                    && !supporting_indices.contains(&(idx - 1))
                    && prev_clean.len() > 3 && prev_clean.len() < 40 {
                    let combined = format!("{}: {}", prev_clean, trimmed);
                    candidate_titles.push((combined, idx, score + 15));
                }
            }
        }

        candidate_titles.push((trimmed.to_string(), idx, score));
    }
    if let Some((best_title, _, _)) = candidate_titles.into_iter().max_by_key(|item| item.2) {
        let clean = best_title.trim_matches(['*', '-', ':']).trim();
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

pub fn extract_description(lines: &[&str], title: &str, location: Option<&str>) -> Option<String> {
    let mut bullet_points: Vec<String> = Vec::new();
    let mut prose_lines: Vec<String> = Vec::new();

    let title_upper = title.to_uppercase();
    let loc_upper = location.map(|l| l.to_uppercase()).unwrap_or_default();

    // Preprocess lines: merge lines ending with connectors (&, +, /, ,)
    let mut merged_lines: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let mut curr = lines[i].trim().to_string();
        while (curr.ends_with('&') || curr.ends_with('+') || curr.ends_with('/') || curr.ends_with(',')) && i + 1 < lines.len() {
            i += 1;
            curr.push(' ');
            curr.push_str(lines[i].trim());
        }
        merged_lines.push(curr);
        i += 1;
    }

    for line in &merged_lines {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Skip title, location, noise/metadata, pure dates/times
        if title_upper.contains(&upper) || (!loc_upper.is_empty() && loc_upper.contains(&upper)) {
            continue;
        }
        if is_noise_or_metadata_line(trimmed) || (!NOTE_PREFIX_RE.is_match(trimmed) && is_date_or_time_line(trimmed)) || (trimmed.ends_with(':') && trimmed.len() <= 5) || trimmed.len() < 3 {
            continue;
        }

        // Structural check 1: List items starting with bullet markers
        let is_bullet = trimmed.starts_with('*') || trimmed.starts_with('-') || trimmed.starts_with('•') || trimmed.starts_with('+');
        if is_bullet {
            let clean_bullet = trimmed.trim_start_matches(['*', '-', '•', '+']).trim();
            if !clean_bullet.is_empty() && !bullet_points.iter().any(|b| b == clean_bullet) {
                bullet_points.push(clean_bullet.to_string());
            }
            continue;
        }

        // Structural check 2: Explicit note / annotation lines (e.g. Rain Date, Remote viewers, Faculty)
        if NOTE_PREFIX_RE.is_match(trimmed) {
            let clean = trimmed.trim_matches(['*', '-', '•']).trim();
            if !clean.is_empty() && !bullet_points.iter().any(|b| b == clean) {
                bullet_points.push(clean.to_string());
            }
            continue;
        }

        // Structural check 3: Slogans / punctuated uppercase phrases (e.g. "RIDE. WALK. RALLY.", "OUR STREETS EXIST For EVERYONE", "LIVE MUSIC & PERFORMANCES")
        if SLOGAN_RE.is_match(trimmed) || (upper == trimmed && trimmed.len() >= 10 && (trimmed.contains(' ') || trimmed.contains('&') || trimmed.contains('.'))) {
            let clean = trimmed.trim_matches(['*', '-', '•']).trim();
            if !clean.is_empty() && !bullet_points.iter().any(|b| b == clean) {
                bullet_points.push(clean.to_string());
            }
            continue;
        }
        // Structural check 4: Tour / subtitle / event classification line (e.g. "2026 TOUR")
        if TOUR_SUBTITLE_RE.is_match(trimmed) {
            let clean = trimmed.trim_matches(['*', '-', '•']).trim();
            if !clean.is_empty() && !bullet_points.iter().any(|b| b == clean) {
                bullet_points.push(clean.to_string());
            }
            continue;
        }

        // Structural check 5: Prose / sentence content (contains punctuation or conversational sentence flow)
        let has_sentence_punctuation = trimmed.contains('.') || trimmed.contains('!') || trimmed.contains('?') || trimmed.contains(',');
        let word_count = trimmed.split_whitespace().count();
        if (has_sentence_punctuation && word_count >= 3) || word_count >= 5 {
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
