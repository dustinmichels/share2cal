# Rust Codebase Improvements & Refactoring Plan

---

## 1. Parsing Accuracy & Deterministic Heuristics

- [x] **Fix weekday resolution for recurring cards**
  - [x] Tighten `get_weekday_date` (`parser.rs:821`) condition to `target_date < ref_date` (instead of `< ref_date - 1 day`) to prevent resolved DTSTART landing in the past for one-off weekday references, allowing callers to opt into a grace day.
  - [x] Decouple RRULE derivation from date absence in `parse_single_event_deterministic` (`parser.rs:949`) so cards with both explicit dates and weekday patterns (e.g. "Starts Sep 8 · Mo, We 1:20–4:20") emit recurrence rules.
- [x] **Remove sample-specific literals from entity extraction**
  - [x] `extract_location`: drop `lower.contains("skilton")` (`parser.rs:1335`) and replace unanchored `lower.contains("st")` with token/suffix matching (`\b(st|street)\b`).
  - [x] `extract_description` (`parser.rs:~1620-1640`): replace hardcoded keyword allowlists (`"PIZZA"`, `"RIDE."`, `"STREETS EXIST"`, `"TUFFS_UEP"`, `"SPECIAL GUEST"`, etc.) with structural heuristics (bullet-list membership, line length, capitalisation ratio, position relative to title/time blocks).
  - [x] Implement generalizable rules for entity extraction: room/hall/building patterns, `venue + city/state` delimiters, street-address matching.
- [x] **Gate day-abbreviation matching on time context**
  - [x] Guard `day_pattern_re.find(ocr_text)` in `parse_single_event_deterministic` (`parser.rs:950`) with a time-range / course-code check (matching `parse_row_schedule_table_events:435-438`), preventing bare words like "we" or "sun" from fabricating weekly recurrence rules on one-off flyers.
  - [x] Audit and apply appropriate guards to `parse_agenda_events` (`parser.rs:835`).

---

## 2. Test Harness Rigor & Ground-Truth Validation

- [x] **Eliminate title-only `||` escape hatches**
  - [x] Remove fallback disjunctions at `lib.rs:1676, 1710, 1744, 1782, 1815, 1849` (`matched || events.iter().any(|ev| ev.title.contains("gilman") || …)`), which let tests pass on title substring alone.
- [x] **Assert the fields `parsed.json` actually carries**
  - [x] Assert `is_all_day`, `start_time`, and `end_time` directly, normalising the 12-hour ground truth in `samples/parsed.json` to 24-hour before comparison.
  - [x] Derive expected `BYDAY` sets from `days` and `repeating` arrays to assert against parsed `recurrence_rule`.
  - [x] Assert containment of significant ground-truth location tokens against the flat `location` field.
- [x] **Aggregated sample mismatch reporting**
  - [x] Collect all sample mismatches into one structured summary per test run instead of aborting on the first failure.
- [x] **Explicit enhanced-mode execution verification**
  - [x] Make missing model weights in `test_all_samples_llm_inference_enhanced_against_parsed_json` (`lib.rs:1627-1633`) an explicit skip (env-gated hard failure in CI or visible ignored-test) rather than a silent pass.

---

## 3. Enhanced LLM Inference & GBNF Grammar Robustness

- [x] **Reserve generation headroom and trim dense OCR text**
  - [x] Reserve an explicit `MAX_OUTPUT_TOKENS` budget in `inference.rs:191` (reject/trim when `prompt_tokens + reserve > n_ctx` instead of checking `tokens.len() >= DEFAULT_CONTEXT_WINDOW`).
  - [x] Replace hard error with adaptive trimming in `generate_extraction_prompt()` (`parser.rs:267-290`): drop low-signal OCR lines until the prompt fits the budget rather than failing extraction.

---

## 4. Regex Hoisting & Parsing Performance

- [ ] **Define static regexes in `parser.rs` with `std::sync::LazyLock`**
  - [ ] `DAY_PATTERN_RE` — compiled 4× at `388, 679, 835, 948`
  - [ ] `TIME_RANGE_RE` — compiled 4× at `389, 680, 834` and via `extract_times`
  - [ ] `YEAR_RE` (`347`), `EXPLICIT_MARKER_RE` (`1319`), `TIME_12H_RE`/`TIME_24H_RE` (`1420, 1421`), `NUM_DATE_RE` (`1426`)
  - [ ] `extract_times` regexes at `1245`, `1284`, `1297`
  - [ ] Unhoisted regexes: `faculty_re` (390), `units_re` (391), `section_num_re` (392), `course_type_re` (393), `header_re` (623), `paren_re` (882), `rel_regex` (1119), `day_month_regex` (1149), `month_day_regex` (1171, 1431), `numeric_regex` (1193), `weekday_day_regex` (1432), `phone_regex` (1441), `course_code_re` (1475)
- [ ] **Deduplicate call sites — but preserve behaviour**
  - [ ] Verify `COURSE_CODE_RE` variations (`parser.rs:387` vs `parser.rs:678` with trailing `\b`) against class samples to unify safely or maintain distinct statics.
  - [ ] Deduplicate identical `DAY_PATTERN_RE` (`388, 679, 835, 948`) and `TIME_RANGE_RE` (`389, 680, 834`) definitions.
- [ ] **Fix regex compilation in a loop**
  - [ ] Hoist `paren_re` (`r"\(([^)]+)\)"` inside `parse_agenda_events` line loop at `parser.rs:882:32`) to a `LazyLock` static.

---

## 5. Test Architecture & Integration Test Extraction

- [ ] **Establish test helper visibility for integration tests**
  - [ ] Expose private helpers needed by tests (`extract_text_from_image_bytes`, `stage_shared_image`, `get_pending_shared_image`, and parser internals) via `pub` or a `#[doc(hidden)] pub mod test_support` module so external `tests/` can consume them.
- [ ] **Extract the integration harness into `src-tauri/tests/`**
  - [ ] Create `tests/samples_validation.rs` for sample-flyer / `parsed.json` gold-standard tests
  - [ ] Move `ParsedJsonEvent`, `load_parsed_json_manifest`, `fuzzy_match_title`, `fuzzy_match_location`, `match_date_and_time` (`lib.rs:502-646`) into `tests/common/mod.rs`
  - [ ] Move `test_parsed_json_manifest_loads_all_samples` (739), `test_all_samples_simple_deterministic_against_parsed_json` (759), `test_all_samples_llm_inference_enhanced_against_parsed_json` (1625)
  - [ ] Move flyer-specific tests: `test_extract_event_from_sample_image` (965), `test_extract_event_from_instagram_sample_image` (1067), `test_extract_event_from_squirrel_flower_sample_image` (1127), `test_extract_events_plural_from_squirrel_flower_flyer` (1207), `test_extract_events_plural_from_sample_flyer` (1237), `test_extract_events_from_classes_sample_image` (1297), `test_extract_event_from_ride_for_life_sample_image` (1542)
  - [ ] Move class and parser tests: `test_classes_schedule_relative_date_handling_next_weekday` (854), `test_extract_event_from_class_sample_image` (913), `test_extract_text_and_parse_events_from_classes_image_bytes` (1481), `test_parse_events_simple_mode_forces_deterministic` (1586)
- [ ] **Extract domain integration tests**
  - [ ] `tests/calendar_integration.rs` — calendar creation, permission, date-parsing tests
  - [ ] `tests/share_integration.rs` — `test_share_commands_flow` (`lib.rs:1514`) and App Group staging
- [ ] **Retain fast unit tests in source files**
  - [ ] Keep fast unit tests in `parser.rs` (or `parser/*`), `ocr.rs`, `model.rs`, `calendar.rs` under `#[cfg(test)] mod tests`
  - [ ] Verify `cargo test` runs unit + integration tests identically

---

## 6. Tauri Command Layer Modularization

- [ ] **Delete dead starter scaffold**
  - [ ] Remove `fn greet(name: &str)` (`lib.rs:19`) and its `generate_handler!` entry, leaving 31 domain commands.
- [ ] **Create modular command hierarchy in `src-tauri/src/commands/`**
  - [ ] `src/commands/mod.rs` re-exporting all domain command modules
  - [ ] `src/commands/ocr.rs`: `extract_text_from_image`, `extract_text_from_image_bytes`
  - [ ] `src/commands/events.rs`: `parse_events_from_text`, `parse_event_from_text`, `extract_events_from_image`, `extract_event_from_image`, `extract_events_from_image_bytes`, `extract_event_from_image_bytes`, `generate_event_prompt`, `get_event_schema`, `get_event_gbnf_grammar`, plus `parse_events_internal` / `parse_event_internal`
  - [ ] `src/commands/models.rs`: `get_model_manifest`, `get_model_statuses`, `get_model_status`, `download_model`, `cancel_model_download`, `delete_model`, `verify_model_hash`, `get_models_storage_info`, `open_models_directory`
  - [ ] `src/commands/inference.rs`: `unload_inference_model`, `is_inference_model_loaded`
  - [ ] `src/commands/calendar.rs`: `get_available_calendars`, `create_calendar_event`, `create_calendar_events`, `check_calendar_permission`, `request_calendar_permission`
  - [ ] `src/commands/share.rs`: `get_pending_shared_image`, `clear_pending_shared_image`, `stage_shared_image`, `load_image_from_path`
- [ ] **Simplify `src/lib.rs` bootstrap**
  - [ ] Shrink non-test `lib.rs` (currently 494 LOC before `mod tests`) to module declarations plus `tauri::Builder` setup
  - [ ] Register the 31 commands via `commands::*`
- [ ] **Optimize internal helper parameter passing**
  - [ ] Audit internal non-command functions flagged by `clippy::needless_pass_by_value` and pass references where appropriate (avoiding `Vec<u8>` on `#[tauri::command]` IPC handlers where ownership is required).

---

## 7. Parser Decomposition & Architecture

- [ ] **Split `src/parser.rs` into `src-tauri/src/parser/` submodules**
  - [ ] `parser/mod.rs`: re-export `EventDetails`, `ReferenceContext`, `parse_events_deterministic`, `parse_event_deterministic`
  - [ ] `parser/rrule.rs`: `struct RecurrenceRule`, `new_weekly`, `to_rrule_string`, `parse_rrule`, `parse_weekdays_to_byday`
  - [ ] `parser/schema.rs`: `get_gbnf_grammar()`, `get_json_schema()`, `generate_extraction_prompt()`
  - [ ] `parser/schedule.rs`: `parse_schedule_table_events` (375), `parse_row_schedule_table_events`, `parse_columnar_schedule_table_events`, `extract_columnar_descriptions`, `clean_course_name`
  - [ ] `parser/agenda.rs`: `parse_agenda_events` (829)
  - [ ] `parser/datetime.rs`: `extract_date`, `extract_times`, `parse_hour_min`, `resolve_year`, `month_to_num`, `weekday_to_num`, `get_weekday_date` (800), `extract_term_until_date`, `is_date_pattern`, `is_time_pattern`, `is_date_or_time_line`
  - [ ] `parser/heuristics.rs`: `extract_title` + keyword scoring, `extract_location`, `clean_location_string`, `extract_description`, `is_noise_or_metadata_line`
  - [ ] `parser/regex.rs` (or keep in `mod.rs`): §4 `LazyLock` statics shared across submodules
  - [ ] Adjust visibility (`pub(crate)`) for internal parser helpers (`extract_date`, `extract_times`, `extract_location`, `extract_title`) across submodules.

---

## 8. Strongly-Typed Domain Error Handling

- [ ] **Introduce a structured error enum in `src-tauri/src/error.rs`**
  - [ ] Add `thiserror` to `src-tauri/Cargo.toml`
  - [ ] Define `AppError`: `Ocr(String)`, `Calendar(String)`, `CalendarPermissionDenied`, `ModelNotFound(String)`, `InsufficientDiskSpace { required_mb: u64, available_mb: u64 }`, `Download(String)`, `ChecksumMismatch(String)`, `Inference(String)`, `InferenceTimeout(u64)`, `Io(String)`
  - [ ] Implement `serde::Serialize` for Tauri IPC and update frontend error handling
  - [ ] Implement `From<std::io::Error>`, `From<serde_json::Error>`, `From<reqwest::Error>`
- [ ] **Migrate internal functions from `Result<T, String>` to `Result<T, AppError>`**
  - [ ] `ocr::extract_text_from_path`, `ocr::extract_text_from_bytes`
  - [ ] `calendar::list_calendars`, `calendar::create_event`, `calendar::check_permission`
  - [ ] `model::get_manifest`, `model::start_model_download`, `model::verify_model`
  - [ ] `inference::InferenceEngineManager` methods

---

## 9. FFI Safety & Memory Guard Consolidation

- [ ] **Centralize native C string memory management**
  - [ ] Create `src-tauri/src/ffi/mod.rs` (or `src/ffi/guard.rs`)
  - [ ] Implement `NativeStringGuard` holding `*mut c_char` plus the free `fn(*mut c_char)`, with `Drop` and a null check
  - [ ] Safe conversions: `to_string_lossy() -> Option<String>`, `into_string() -> Result<String, _>`
  - [ ] Gate the module on `#[cfg(any(target_os = "macos", target_os = "ios"))]` to match existing extern blocks
- [ ] **Refactor module FFI call sites**
  - [ ] Replace `AutoCString` in `ocr.rs`
  - [ ] Replace `AutoCString` in `calendar.rs`
  - [ ] Replace manual `free_share_string` calls in `share.rs` with `NativeStringGuard` to prevent memory leaks on early returns

---

## 10. State Management & Dependency Injection

- [ ] **Unify global state under Tauri managed state**
  - [ ] Implement `Default` for `InferenceEngineManager` (clears `clippy::new_without_default`)
  - [ ] Register via `app.manage(inference::InferenceEngineManager::default())` in `lib.rs`
  - [ ] Inject `state: tauri::State<InferenceEngineManager>` into `unload_inference_model` and `is_inference_model_loaded`
  - [ ] Thread the manager explicitly through `extract_events_orchestrated` instead of internal `global()` call
  - [ ] Remove `ENGINE_MANAGER` / `global()`, updating unit/integration test call sites (`inference.rs:610, 651`, `lib.rs:1635`) to construct their own instance
  - [ ] Keep `BACKEND: OnceLock<Arc<LlamaBackend>>` isolated in the inference subsystem

---

## 11. Observability & Structured Logging

- [ ] **Replace stray `eprintln!` with structured logging**
  - [ ] Add `tracing` (or `tauri-plugin-log`) to `src-tauri/Cargo.toml`
  - [ ] `inference.rs:180` `"[Debug] Sampler initialized before prompt decoding"` → `tracing::debug!`
  - [ ] `inference.rs:440` `"Failed to load manifest"` → `tracing::warn!`
  - [ ] `inference.rs:455` `"Storage directory error"` → `tracing::error!`
  - [ ] `inference.rs:468` `"Model ID not found in manifest"` → `tracing::warn!`
  - [ ] `inference.rs:479` `"Model file does not exist"` → `tracing::info!`
  - [ ] `inference.rs:492` `"Failed to load model weights"` → `tracing::error!`
  - [ ] Update remaining runtime `eprintln!` sites: `inference.rs:510, 519, 530` and `lib.rs:1630`
  - [ ] Leave test-local output (`inference.rs:661`, `ocr.rs`, `parser.rs`) untouched or convert to test assertions

---

## 12. Platform Provider Traits & Cross-Platform Abstraction

- [ ] Consolidate per-file `#[cfg]` attributes in `ocr.rs` (~8), `calendar.rs` (~6), and `share.rs` (~5) into clean `apple` / `fallback` module pairs per file.
- [ ] Defer full dynamic provider traits (`CalendarProvider`, `OcrProvider`, `ShareProvider`) until non-Apple target support or provider mocking is required.

---

## 13. Clippy Diagnostics & Code Hygiene Fixes

- [ ] **Apply Clippy suggestions across `src-tauri/`**
  - [ ] `clippy::clone_on_copy` — `parser.rs:198`, use `*self.get_reference_datetime().offset()`
  - [ ] `clippy::manual_clamp` — `inference.rs:81`, use `count.clamp(1, 4)`
  - [ ] `clippy::collapsible_if` — `inference.rs:252`, `ocr.rs:106`, `parser.rs:1335`
  - [ ] `clippy::if_same_then_else` — `parser.rs:1263`
  - [ ] `clippy::manual_range_contains` — `parser.rs:1267`, use `(8..=11).contains(&start_h)`
  - [ ] `clippy::empty_line_after_doc_comments` — `model.rs:116`
  - [ ] `clippy::needless_return` — `model.rs:128`
  - [ ] `clippy::unnecessary_map_or` — `parser.rs:641`
  - [ ] `clippy::assign_op_pattern` — `parser.rs:822, 998, 1135, 1141`
  - [ ] `clippy::manual_pattern_char_comparison` — `parser.rs:318` (`[',', '/', '&', ' ', ';', '+']`) and `parser.rs:1632` (`['*', '-', '•']`)
  - [ ] Redundant reference in `format!` argument — `model.rs:181, 287, 338` (drop `&` on `entry.filename`)
  - [ ] `clippy::new_without_default` — `inference.rs:95`
  - [ ] Re-run `cargo clippy --all-targets` to verify clean output
