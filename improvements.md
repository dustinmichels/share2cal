# Rust Codebase Improvements & Refactoring Plan

A structured, actionable checklist of architectural, performance, and code hygiene improvements for `src-tauri/`.

---

## 1. Parsing Accuracy & Deterministic Heuristics

- [ ] **Weekday rollover for single recurring event cards**
  - [ ] Ensure single-event recurring logic (e.g. `class.png`) advances the event start date (`DTSTART`) forward to the first matching weekday relative to the reference timestamp, matching the behavior in multi-event schedule tables.
- [ ] **Generalize entity and location extraction patterns**
  - [ ] Replace sample-specific literal string checks in `extract_location` and `extract_description` with generalizable heuristic rules (e.g., room/hall/building patterns, venue + city/state delimiters, street address matching).
- [ ] **Scope day name / abbreviation matching to time and header contexts**
  - [ ] Narrow weekly day abbreviation regex matching to lines containing time ranges or class meeting patterns to prevent normal English prose (such as "we invite you" or "sun") from triggering recurrence rules on non-recurring flyers.

---

## 2. Test Harness Rigor & Ground-Truth Validation

- [ ] **Enforce strict date, time, and location assertions across all test modes**
  - [ ] Eliminate title-only `||` fallback escape hatches in test assertions for both simple and enhanced test suites.
  - [ ] Directly assert `is_all_day`, normalized 24-hour start/end times, location entity tokens (venue, room, city), and `BYDAY` recurrence rules against `samples/parsed.json`.
- [ ] **Aggregated sample mismatch reporting in integration test loops**
  - [ ] Collect all sample image validation mismatches into a single structured summary report per test run rather than aborting immediately on the first mismatch.
- [ ] **Explicit enhanced mode execution verification**
  - [ ] Ensure enhanced-mode test suites explicitly assert the presence of model weights and fail or log explicit skip diagnostics rather than silently returning early.

---

## 3. Enhanced LLM Inference & GBNF Grammar Robustness

- [ ] **Enforce strict JSON quote escaping in GBNF grammar**
  - [ ] Update the `string` production rule in `get_gbnf_grammar()` to prohibit raw unescaped double quotes (`"`) within JSON strings, enforcing proper `\"` escape sequences for text descriptions.
- [ ] **Prompt token budgeting and context trimming for dense OCR text**
  - [ ] Add adaptive OCR text trimming and token budgeting in `generate_extraction_prompt()` to ensure dense flyer paragraphs do not saturate the context window or trigger decoding loops.

---

## 4. Regex Hoisting & Parsing Performance

- [ ] **Define static regex definitions in `parser.rs` with `std::sync::LazyLock`**
  - [ ] Hoist `DAY_PATTERN_RE` (`(?i)\b(Mo(?:n)?|Tu(?:e)?|...)\b`) into `static LazyLock<Regex>`
  - [ ] Hoist `TIME_RANGE_RE` (`(?i)(\d{1,2}(?::\d{2})?...)\s*(?:-|–|—|to)\s*...`) into `static LazyLock<Regex>`
  - [ ] Hoist `COURSE_CODE_RE` (`\b([A-Z]{2,6}\s+\d{3,4}...)\b`) into `static LazyLock<Regex>`
  - [ ] Hoist `YEAR_RE` (`\b(20\d{2})\b`) into `static LazyLock<Regex>`
  - [ ] Hoist `EXPLICIT_MARKER_RE` (`(?i)(?:this year at|held at|venue:|location:|where:|place:|live at|takes place at)\s*(.*)`) into `static LazyLock<Regex>`
  - [ ] Hoist `TIME_12H_RE` and `TIME_24H_RE` into `static LazyLock<Regex>`
  - [ ] Hoist `NUM_DATE_RE` into `static LazyLock<Regex>`
  - [ ] Hoist time range regexes in `extract_times` (`lines 1244, 1284, 1297`) into `static LazyLock<Regex>`
- [ ] **Deduplicate regex call sites and eliminate drift risk**
  - [ ] Replace `day_pattern_re` compilation in `parse_row_schedule_table_events` (line 388)
  - [ ] Replace `day_pattern_re` compilation in `parse_columnar_schedule_table_events` (line 679)
  - [ ] Replace `day_pattern_re` compilation in `parse_agenda_events` (line 835)
  - [ ] Replace `day_pattern_re` compilation in `parse_single_event_deterministic` (line 948)
  - [ ] Replace `time_range_re` compilation across all 4 corresponding locations (lines 389, 679, 835, 948)
  - [ ] Replace `course_code_re` compilation across lines 387 and 678
- [ ] **Fix regex compilation in loops**
  - [ ] Fix `clippy::regex_creation_in_loops` at `src/parser.rs:840` by reusing the static regex instance outside the line loop

---

## 5. Test Architecture & Integration Test Extraction

- [ ] **Extract integration test harness from `lib.rs` into `src-tauri/tests/`**
  - [ ] Create `tests/samples_validation.rs` for sample flyer and `parsed.json` gold-standard accuracy tests
  - [ ] Move `load_parsed_json_manifest()`, `fuzzy_match_title()`, `fuzzy_match_location()`, and `match_date_and_time()` from `lib.rs:496-735` to a shared test helper in `tests/common/mod.rs`
  - [ ] Move `test_parsed_json_manifest_loads_all_samples` to `tests/samples_validation.rs`
  - [ ] Move `test_all_samples_simple_deterministic_against_parsed_json` to `tests/samples_validation.rs`
  - [ ] Move `test_all_samples_llm_inference_enhanced_against_parsed_json` to `tests/samples_validation.rs`
  - [ ] Move flyer-specific tests (`test_extract_event_from_sample_image`, `test_extract_event_from_instagram_sample_image`, `test_extract_event_from_squirrel_flower_sample_image`, `test_extract_events_plural_from_squirrel_flower_flyer`, `test_extract_events_plural_from_sample_flyer`, `test_extract_events_from_classes_sample_image`, `test_extract_event_from_ride_for_life_sample_image`) to `tests/samples_validation.rs`
- [ ] **Extract domain integration tests into separate test files**
  - [ ] Create `tests/calendar_integration.rs` for calendar creation, permission, and date parsing tests
  - [ ] Create `tests/share_integration.rs` for `test_share_commands_flow` and App Group staging tests
- [ ] **Retain fast unit tests in source files**
  - [ ] Keep fast unit tests in `parser.rs` (or `parser/*`), `ocr.rs`, `model.rs`, and `calendar.rs` within `#[cfg(test)] mod tests`
  - [ ] Ensure `cargo test` runs all integration and unit tests identically

---

## 6. Tauri Command Layer Modularization

- [ ] **Delete dead starter scaffold**
  - [ ] Remove unused `fn greet(name: &str)` command from `src/lib.rs`
- [ ] **Create modular command hierarchy in `src-tauri/src/commands/`**
  - [ ] Create `src/commands/mod.rs` to re-export all domain command modules
  - [ ] Create `src/commands/ocr.rs` and migrate:
    - [ ] `extract_text_from_image`
    - [ ] `extract_text_from_image_bytes`
  - [ ] Create `src/commands/events.rs` and migrate:
    - [ ] `parse_events_from_text`
    - [ ] `parse_event_from_text`
    - [ ] `extract_events_from_image`
    - [ ] `extract_event_from_image`
    - [ ] `extract_events_from_image_bytes`
    - [ ] `extract_event_from_image_bytes`
    - [ ] `generate_event_prompt`
    - [ ] `get_event_schema`
    - [ ] `get_event_gbnf_grammar`
    - [ ] Internal orchestration helpers (`parse_events_internal`, `parse_event_internal`)
  - [ ] Create `src/commands/models.rs` and migrate:
    - [ ] `get_model_manifest`
    - [ ] `get_model_statuses`
    - [ ] `get_model_status`
    - [ ] `download_model`
    - [ ] `cancel_model_download`
    - [ ] `delete_model`
    - [ ] `verify_model_hash`
    - [ ] `get_models_storage_info`
    - [ ] `open_models_directory`
  - [ ] Create `src/commands/inference.rs` and migrate:
    - [ ] `unload_inference_model`
    - [ ] `is_inference_model_loaded`
  - [ ] Create `src/commands/calendar.rs` and migrate:
    - [ ] `get_available_calendars`
    - [ ] `create_calendar_event`
    - [ ] `create_calendar_events`
    - [ ] `check_calendar_permission`
    - [ ] `request_calendar_permission`
  - [ ] Create `src/commands/share.rs` and migrate:
    - [ ] `get_pending_shared_image`
    - [ ] `clear_pending_shared_image`
    - [ ] `stage_shared_image`
    - [ ] `load_image_from_path`
- [ ] **Simplify `src/lib.rs` app bootstrap**
  - [ ] Shrink `lib.rs` to ~80 LOC focusing strictly on module declarations and `tauri::Builder` setup
  - [ ] Cleanly register the 31 domain commands in `tauri::generate_handler![...]` via `commands::*`
- [ ] **Optimize command parameter passing**
  - [ ] Pass borrowed `&str` / `&[u8]` or references where parameters are not consumed by value (fixing clippy warnings on lines 24, 37, 325, 349, 368, 383, 410, 415, 425)

---

## 7. Parser Decomposition & Architecture

- [ ] **Split `src/parser.rs` (2,271 LOC) into `src-tauri/src/parser/` submodules**
  - [ ] Create `src/parser/mod.rs`: Re-export `EventDetails`, `ReferenceContext`, and entrypoints `parse_events_deterministic` / `parse_event_deterministic`
  - [ ] Create `src/parser/rrule.rs`:
    - [ ] Move `struct RecurrenceRule`
    - [ ] Move `RecurrenceRule::new_weekly`, `to_rrule_string`, and `parse_rrule`
    - [ ] Move `parse_weekdays_to_byday`
  - [ ] Create `src/parser/schema.rs`:
    - [ ] Move `get_gbnf_grammar()`
    - [ ] Move `get_json_schema()`
    - [ ] Move `generate_extraction_prompt()`
  - [ ] Create `src/parser/schedule.rs`:
    - [ ] Move `parse_schedule_table_events`
    - [ ] Move `parse_row_schedule_table_events`
    - [ ] Move `parse_columnar_schedule_table_events`
    - [ ] Move `extract_columnar_descriptions` and `clean_course_name`
  - [ ] Create `src/parser/agenda.rs`:
    - [ ] Move `parse_agenda_events`
  - [ ] Create `src/parser/datetime.rs`:
    - [ ] Move `extract_date`, `extract_times`, `parse_hour_min`, `resolve_year`
    - [ ] Move `month_to_num`, `weekday_to_num`, `get_weekday_date`
    - [ ] Move `extract_term_until_date`
    - [ ] Move `is_date_pattern`, `is_time_pattern`, `is_date_or_time_line`
  - [ ] Create `src/parser/heuristics.rs`:
    - [ ] Move `extract_title` and title keyword scoring logic
    - [ ] Move `extract_location` and `clean_location_string`
    - [ ] Move `extract_description`
    - [ ] Move `is_noise_or_metadata_line`

---

## 8. Strongly-Typed Domain Error Handling

- [ ] **Introduce structured error enum in `src-tauri/src/error.rs`**
  - [ ] Add `thiserror` to `src-tauri/Cargo.toml`
  - [ ] Define `AppError` enum with typed variants:
    - [ ] `AppError::Ocr(String)`
    - [ ] `AppError::Calendar(String)`
    - [ ] `AppError::CalendarPermissionDenied`
    - [ ] `AppError::ModelNotFound(String)`
    - [ ] `AppError::InsufficientDiskSpace { required_mb: u64, available_mb: u64 }`
    - [ ] `AppError::Download(String)`
    - [ ] `AppError::ChecksumMismatch(String)`
    - [ ] `AppError::Inference(String)`
    - [ ] `AppError::InferenceTimeout(u64)`
    - [ ] `AppError::Io(String)`
  - [ ] Implement `serde::Serialize` on `AppError` for Tauri IPC compatibility
  - [ ] Implement `From<std::io::Error>`, `From<serde_json::Error>`, and `From<reqwest::Error>` for `AppError`
- [ ] **Migrate internal functions from `Result<T, String>` to `Result<T, AppError>`**
  - [ ] Update `ocr::extract_text_from_path` and `ocr::extract_text_from_bytes`
  - [ ] Update `calendar::list_calendars`, `calendar::create_event`, and `calendar::check_permission`
  - [ ] Update `model::get_manifest`, `model::start_model_download`, and `model::verify_model`
  - [ ] Update `inference::InferenceEngineManager` methods

---

## 9. FFI Safety & Memory Guard Consolidation

- [ ] **Centralize native C string memory management**
  - [ ] Create `src-tauri/src/ffi/mod.rs` (or `src/ffi/guard.rs`)
  - [ ] Implement reusable `NativeStringGuard` struct with `Drop` calling a generic `unsafe extern "C" fn(*mut c_char)`
  - [ ] Implement safe conversion methods (`to_string_lossy() -> Option<String>`, `into_string() -> Result<String, ...>`)
- [ ] **Refactor module FFI calls to use shared guard**
  - [ ] Replace `AutoCString` in `src-tauri/src/ocr.rs` with `NativeStringGuard`
  - [ ] Replace `AutoCString` in `src-tauri/src/calendar.rs` with `NativeStringGuard`
  - [ ] Replace manual `ffi::free_share_string` calls in `src-tauri/src/share.rs` with `NativeStringGuard`

---

## 10. State Management & Dependency Injection

- [ ] **Unify global state under Tauri managed state**
  - [ ] Implement `Default` for `InferenceEngineManager` (resolves `clippy::new_without_default`)
  - [ ] Register `InferenceEngineManager` via `app.manage(inference::InferenceEngineManager::default())` in `lib.rs`
  - [ ] Remove `static LazyLock<InferenceEngineManager> ENGINE_MANAGER` singleton
  - [ ] Inject `state: tauri::State<InferenceEngineManager>` into inference commands
  - [ ] Keep `BACKEND: OnceLock<Arc<LlamaBackend>>` safely isolated within the inference subsystem

---

## 11. Observability & Structured Logging

- [ ] **Replace stray `eprintln!` statements with structured logging**
  - [ ] Add `tracing` or `log` dependency (or use `tauri-plugin-log`)
  - [ ] Replace `eprintln!("[Debug] Sampler initialized before prompt decoding")` at `inference.rs:188` with `tracing::debug!`
  - [ ] Replace `eprintln!("[Inference] Failed to load manifest: ...")` at `inference.rs:374` with `tracing::warn!`
  - [ ] Replace `eprintln!("[Inference] Storage directory error: ...")` at `inference.rs:388` with `tracing::error!`
  - [ ] Replace `eprintln!("[Inference] Model ID not found ...")` at `inference.rs:403` with `tracing::warn!`
  - [ ] Replace `eprintln!("[Inference] Model file does not exist ...")` at `inference.rs:414` with `tracing::info!`
  - [ ] Replace `eprintln!("[Inference] Failed to load model weights: ...")` at `inference.rs:426` with `tracing::error!`

---

## 12. Platform Provider Traits & Cross-Platform Abstraction

- [ ] **Define platform capability traits**
  - [ ] Create `CalendarProvider` trait (`check_permission`, `request_permission`, `list_calendars`, `create_event`)
  - [ ] Create `OcrProvider` trait (`extract_from_path`, `extract_from_bytes`)
  - [ ] Create `ShareProvider` trait (`get_pending_share`, `clear_pending_share`, `stage_share`)
- [ ] **Separate platform implementations cleanly**
  - [ ] Implement `AppleCalendarProvider`, `AppleOcrProvider`, and `AppleShareProvider` for `target_os = "macos"` / `target_os = "ios"`
  - [ ] Implement `StubCalendarProvider`, `StubOcrProvider`, and `StubShareProvider` (or mock providers) for non-Apple targets and unit tests without scattered inline `#[cfg(...)]` blocks

---

## 13. Clippy Diagnostics & Code Hygiene Fixes

- [ ] **Apply Clippy suggestions across `src-tauri/`**
  - [ ] Fix `clippy::clone_on_copy`: Remove redundant `.clone()` on `FixedOffset` at `parser.rs:198`
  - [ ] Fix `clippy::manual_clamp`: Use `.clamp(1, 4)` for thread count selection at `inference.rs:81`
  - [ ] Fix `clippy::collapsible_if`: Collapse nested `if` statements at `inference.rs:252` and `ocr.rs:106`
  - [ ] Fix `clippy::if_same_then_else`: Clean up duplicate branches at `parser.rs:1265`
  - [ ] Fix `clippy::manual_range_contains`: Use `(min..=max).contains(&val)` at `parser.rs:1267`
  - [ ] Fix `clippy::empty_line_after_doc_comments` at `model.rs:116`
  - [ ] Fix `clippy::needless_return` at `model.rs:128`
  - [ ] Fix `clippy::unnecessary_map_or` at `parser.rs:641`
  - [ ] Fix `clippy::assign_op_pattern` at `parser.rs:822, 998, 1135, 1141`
  - [ ] Fix redundant string closures and format string captures throughout the crate
