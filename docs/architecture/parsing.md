# Rule-based parsing

The deterministic parser in `src-tauri/src/parser/` turns OCR text into `EventDetails` with regular expressions and scoring heuristics. It is the first answer for every scan and the safety net for every model failure.

## Who decides: rules or model

Both scan passes call `parse_events_from_text`, which forwards to `parse_events_internal` in `commands/events.rs`. The `mode` argument and a few checks decide what runs:

<ParseFlow />

The schedule-table check runs before the model on purpose. A class schedule has rigid, regular structure that the rules handle reliably, and it would overflow a small model's output budget.

## Reference time

Relative dates need an anchor. Every parse takes a `ReferenceContext`:

```rust
pub struct ReferenceContext {
    pub reference_time: Option<String>,          // RFC 3339
    pub timezone_offset_minutes: Option<i32>,
}
```

If the caller passes neither, `ReferenceContext::now()` uses the device's local time and offset. Tests pin it, for example to `2026-09-06T10:57:00-04:00`, so expectations do not drift. All produced timestamps are RFC 3339 in that fixed offset; all-day events use bare `YYYY-MM-DD`.

## Strategy order

`parse_events_deterministic` tries three strategies and returns the first that produces enough events:

| Order | Strategy                       | Accepts when     | `source`                                                      |
| ----- | ------------------------------ | ---------------- | ------------------------------------------------------------- |
| 1     | Schedule table (`schedule.rs`) | 2 or more events | `deterministic_schedule` or `deterministic_schedule_columnar` |
| 2     | Agenda (`agenda.rs`)           | 2 or more events | `deterministic_agenda`                                        |
| 3     | Single event (`mod.rs`)        | Always           | `deterministic`                                               |

A URL found anywhere in the document (`extract_url`) is copied onto every event that does not have one.

### Schedule tables

Built for course registration screenshots such as `samples/classes.png`. Two layouts are recognised:

- **Row layout** (`parse_row_schedule_table_events`): each class is on one row, anchored by a course code like `CVS 0188-01 (83710)`.
- **Columnar layout** (`parse_columnar_schedule_table_events`): codes, titles, days and times are in separate columns that OCR returns as separate runs of lines. The parser pairs them up by order.

Each class becomes a weekly recurring event, starting on the first matching weekday on or after the reference date. Day lists like `Mo, We` or `Tu, Th` become `BYDAY`. Rules are open-ended: `RecurrenceRule::new_weekly` is always called without `UNTIL`. (`datetime.rs` has an unused `extract_term_until_date` helper that guesses a term end date.) Row-layout events carry the instructor and units in the description; columnar events carry the days.

This is also the check that decides whether the OCR commands use the [spatially rebuilt rows](/architecture/ocr#rebuilding-rows).

### Agendas

For programmes and line-ups where each line carries its own time range (`7:00 PM - 8:30 PM Opening talk`). The parser walks the lines, tracks the most recent date heading, and emits an event per time range. A line without its own date but with a day name (`Wed 6-8pm`) lands on the first matching weekday on or after the reference date.

### Single event

The fallback, and the path most flyers take. It extracts each field independently:

| Field       | How                                                                                                                                                                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date        | Month-day, day-month, numeric and relative forms (`Nov 12`, `12th of November`, `11/12`, `2026-11-12`, `tomorrow`, `next Friday`). A weekday next to the date picks the year whose calendar matches; otherwise a date more than 60 days in the past rolls to next year.                        |
| Time        | Ranges in 12- or 24-hour form (`6-8pm`, `6:00 PM – 8:00 PM`, `18:00-20:00`) or a single time (`at 7pm`, `doors 7:30 PM`). An end time earlier than the start rolls to the next day. No end time means one hour.                                                                                |
| All day     | A date with no time.                                                                                                                                                                                                                                                                           |
| Recurrence  | A day pattern on a time line, near words like `every`, `weekly`, `schedule`, or two or more days (`Tu, Th`) produces `FREQ=WEEKLY;BYDAY=…`. Ambiguous short tokens such as `We` inside a long sentence are ignored.                                                                            |
| Title       | Scores the first 12 lines. Earlier lines score higher; event words (`CONCERT`, `WORKSHOP`, `MARKET`, `RIDE`, …) add points; dates, times, URLs, phone numbers, bullets and metadata are skipped. Openers like `WITH` or `FEATURING` are recognised so a support act does not become the title. |
| Location    | Explicit markers (`Location:`, `Venue:`, `held at`), venue words (`hall`, `studio`, `park`, `zoom`), street suffixes and `City, ST` patterns. Continuation lines with address words are joined.                                                                                                |
| Description | Remaining informative lines that are not the title or location.                                                                                                                                                                                                                                |
| URL         | `http(s)://` links first, then bare `www.` hosts, with trailing punctuation stripped.                                                                                                                                                                                                          |

Confidence starts at 0.30 and adds 0.25 for a real title, 0.25 for a start time, 0.15 for a location and 0.05 for a description, capped at 0.95.

## File map

| File                   | Contents                                                                                |
| ---------------------- | --------------------------------------------------------------------------------------- |
| `parser/mod.rs`        | `EventDetails`, `ReferenceContext`, strategy order, single-event parser, URL extraction |
| `parser/schedule.rs`   | Row and columnar schedule-table parsers, course name cleanup                            |
| `parser/agenda.rs`     | Multi-line agenda parser                                                                |
| `parser/heuristics.rs` | Title, location and description scoring, noise filters                                  |
| `parser/datetime.rs`   | Date and time extraction, year resolution, weekday lookup                               |
| `parser/rrule.rs`      | `RecurrenceRule`: build, serialise and validate RRULE strings                           |
| `parser/regex.rs`      | Every regular expression, compiled once with `LazyLock`                                 |
| `parser/schema.rs`     | JSON schema, GBNF grammar and the LLM prompt builder                                    |

## Recurrence rules

`RecurrenceRule` understands `FREQ` (`DAILY`, `WEEKLY`, `MONTHLY`, `YEARLY`), `INTERVAL`, `BYDAY`, `UNTIL` and `COUNT`. The parser only ever produces weekly rules. `parse_rrule` also validates rules coming back from the model or the edit form before they reach EventKit.

## Changing the parser

1. Add or update the image in `samples/` and its expected events in `samples/parsed.json` (and `parsed.md`).
2. Write a unit test next to the code you change. Each parser file has a `#[cfg(test)]` module with realistic OCR strings.
3. Run `cargo test` in `src-tauri/`. `tests/samples_validation.rs` OCRs every sample on macOS and checks the parser against `parsed.json`.

See [Testing](/reference/testing) for the full list.
