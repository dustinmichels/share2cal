# Rust Codebase Improvements & Refactoring Plan

---

## 1. Parsing Accuracy & Deterministic Heuristics

- [ ] **Fix weekday resolution for recurring cards** — **[corrected]**
  - Original claim ("single-event path does not roll DTSTART forward, unlike the table path") is **false**. Both paths call the same helper `get_weekday_date` (`parser.rs:800`): table path at `parser.rs:439-443`, single-event path at `parser.rs:993-994`. The two real defects are:
  - [ ] `get_weekday_date` (`parser.rs:821`) only advances when `target_date < ref_date - 1 day`, so a resolved DTSTART may land one day in the past. Harmless for a weekly RRULE, wrong for a one-off weekday reference (`"this Saturday"`). Tighten to `target_date < ref_date` and let callers opt into the grace day.
  - [ ] `parse_single_event_deterministic` computes `weekday_days_opt` / `single_recurrence_rule` **only when `extracted_date.is_none()`** (`parser.rs:949`). A card carrying both an explicit date and a weekday pattern (e.g. "Starts Sep 8 · Mo, We 1:20–4:20") therefore emits **no recurrence rule at all**. Decouple RRULE derivation from date absence.
- [ ] **Remove sample-specific literals from entity extraction** — **[verified, understated]**
  - [ ] `extract_location`: drop `lower.contains("skilton")` (`parser.rs:1335`). Also fix `lower.contains("st")` on the same line — an unanchored substring test that matches almost any English word (`first`, `august`, `institute`); it must be a token/suffix match (`\b(st|street)\b`).
  - [ ] `extract_description` (`parser.rs:~1620-1640`): replace the hardcoded keyword allowlists harvested from the sample set (`"PIZZA"`, `"RIDE."`, `"STREETS EXIST"`, `"TUFFS_UEP"`, `"SPECIAL GUEST"`, …) with structural heuristics (bullet-list membership, line length, capitalisation ratio, position relative to title/time blocks).
  - [ ] Replace both with generalizable rules: room/hall/building patterns, `venue + city/state` delimiters, street-address matching.
- [ ] **Gate day-abbreviation matching on time context** — **[verified; highest value item in this document]**
  - Evidence: `parse_single_event_deterministic` runs `day_pattern_re.find(ocr_text)` across the **entire OCR text with no guard** (`parser.rs:950`). The alternation includes bare `We`, `Su`, `Sa`, `Mo`, so the prose "**we** invite you" or "in the **sun**" fabricates `FREQ=WEEKLY` on a one-off flyer.
  - The fix already exists in the table path and should be mirrored: `parse_row_schedule_table_events` only consults `day_pattern_re` inside `if let Some(tm) = time_match` (`parser.rs:435-438`), i.e. the day match is scoped to a line that also carries a time range.
  - [ ] Apply the same time-range / course-code guard to `parse_single_event_deterministic:950` and audit `parse_agenda_events:835`.

---

## 2. Test Harness Rigor & Ground-Truth Validation

- [ ] **Eliminate title-only `||` escape hatches** — **[verified]**
  - [ ] Remove the fallback disjunctions at `lib.rs:1676, 1710, 1744, 1782, 1815, 1849` (`matched || events.iter().any(|ev| ev.title.contains("gilman") || …)`), which let a run pass on a title substring alone.
- [ ] **Assert the fields `parsed.json` actually carries** — **[corrected]**
  - `samples/parsed.json` stores flat per-sample records: `title`, `date`, `days`, `start_time` (12-hour, e.g. `"1:20 PM"`), `end_time`, `is_all_day`, `repeating`, `term`, `location`, `description`. It has **no `BYDAY` field and no venue/room/city entity decomposition** — `location` is one flat string.
  - [ ] Assert `is_all_day` and both times directly, normalising the 12-hour ground truth to 24-hour before comparison.
  - [ ] Derive the expected `BYDAY` set from the `days` array (+ `repeating`) and assert it against the parsed `recurrence_rule`; do not expect a literal `BYDAY` key in the manifest.
  - [ ] For location, assert containment of each significant ground-truth token rather than inventing venue/room/city fields the manifest does not have.
- [ ] **Aggregated sample mismatch reporting** — **[verified]**
  - [ ] Collect all sample mismatches into one structured summary per run instead of aborting on the first failure.
- [ ] **Explicit enhanced-mode execution verification** — **[verified]**
  - [ ] `test_all_samples_llm_inference_enhanced_against_parsed_json` currently does a bare `return` after an `eprintln!` when weights are absent (`lib.rs:1627-1633`), which reports as a pass. Make the skip explicit (env-gated hard failure in CI, or a visible ignored-test mechanism).

---

## 3. Enhanced LLM Inference & GBNF Grammar Robustness

- ~~Enforce strict JSON quote escaping in GBNF grammar~~ — **[dropped: already implemented]**
  - `get_gbnf_grammar()` already defines `string ::= "\"" ([^"\\\r\n] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F]{4}))* "\"" ws` (`parser.rs:208`). Raw `"` (and raw newlines) are already prohibited. No change needed.
- [ ] **Reserve generation headroom and trim dense OCR text** — **[corrected]**
  - A budget check already exists, but it is the wrong shape: `inference.rs:191` hard-errors when `tokens.len() >= DEFAULT_CONTEXT_WINDOW` (2048, `inference.rs:54`). A 2047-token prompt therefore _passes_ and leaves one token for the answer, which is exactly the decode-loop / truncated-JSON case.
  - [ ] Reserve an explicit `MAX_OUTPUT_TOKENS` budget: reject/trim when `prompt_tokens + reserve > n_ctx`.
  - [ ] Replace the hard error with adaptive trimming in `generate_extraction_prompt()` (`parser.rs:267-290`, which today interpolates `ocr_text.trim()` verbatim): drop low-signal OCR lines until the prompt fits, rather than failing the extraction outright.

---

## 4. Regex Hoisting & Parsing Performance

- [ ] **Define static regexes in `parser.rs` with `std::sync::LazyLock`** — **[verified, line numbers corrected]**
  - [ ] `DAY_PATTERN_RE` — compiled 4× at `388, 679, 835, 948`
  - [ ] `TIME_RANGE_RE` — compiled 4× at `389, 680, 834` and via `extract_times`
  - [ ] `YEAR_RE` (`347`), `EXPLICIT_MARKER_RE` (`1319`), `TIME_12H_RE`/`TIME_24H_RE` (`1420, 1421`), `NUM_DATE_RE` (`1426`)
  - [ ] `extract_times` regexes at `1245` (was listed as 1244), `1284`, `1297`
  - [ ] Also unhoisted and worth including in the same pass: `faculty_re` (390), `units_re` (391), `section_num_re` (392), `course_type_re` (393), `header_re` (623), `paren_re` (882), `rel_regex` (1119), `day_month_regex` (1149), `month_day_regex` (1171, 1431), `numeric_regex` (1193), `weekday_day_regex` (1432), `phone_regex` (1441), `course_code_re` (1475)
- [ ] **Deduplicate call sites — but preserve behaviour** — **[corrected]**
  - [ ] `COURSE_CODE_RE` at `387` and `678` are **not identical**: `387` is `r"\b([A-Z]{2,6}\s+\d{3,4}(?:-\d{2,3})?(?:\s*\(\d+\))?)"` while `678` appends a trailing `\b`. Collapsing them into one static silently changes matching for one call site. Confirm which variant is correct against the class samples, or keep two named statics.
  - [ ] `DAY_PATTERN_RE` (`388, 679, 835, 948`) and `TIME_RANGE_RE` (`389, 680, 834`) are byte-identical across sites and can be deduplicated safely.
- [ ] **Fix regex compilation in a loop** — **[corrected]**
  - [ ] Clippy reports `compiling a regex in a loop` at **`parser.rs:882:32`** (the `paren_re` = `r"\(([^)]+)\)"` inside `parse_agenda_events`'s line loop), **not** at line 840. Hoist `paren_re` to a `LazyLock` static.

---

## 5. Test Architecture & Integration Test Extraction

- [ ] **Resolve the visibility blocker first** — **[new; blocks the whole section]**
  - Tests currently live in `#[cfg(test)] mod tests` inside `lib.rs:495` with `use super::*`, and they call **private** items: `extract_text_from_image_bytes` (`lib.rs:37`, used at `1071, 1483`), `stage_shared_image` (`lib.rs:348`) and `get_pending_shared_image` (`lib.rs:336`, used at `1518-1534`), plus private `parser` helpers. `src-tauri/tests/` does not exist yet.
  - [ ] Decide the mechanism before moving anything: promote the needed items to `pub` (widens the public surface), or expose a `#[doc(hidden)] pub mod test_support` re-export. Moving tests to an external `tests/` crate without this **will not compile**.
- [ ] **Extract the integration harness into `src-tauri/tests/`**
  - [ ] Create `tests/samples_validation.rs` for sample-flyer / `parsed.json` gold-standard tests
  - [ ] Move `ParsedJsonEvent`, `load_parsed_json_manifest`, `fuzzy_match_title`, `fuzzy_match_location`, `match_date_and_time` — actual range **`lib.rs:502-646`** (the plan previously said 496-735) — into `tests/common/mod.rs`
  - [ ] Move `test_parsed_json_manifest_loads_all_samples` (739), `test_all_samples_simple_deterministic_against_parsed_json` (759), `test_all_samples_llm_inference_enhanced_against_parsed_json` (1625)
  - [ ] Move flyer-specific tests: `test_extract_event_from_sample_image` (965), `test_extract_event_from_instagram_sample_image` (1067), `test_extract_event_from_squirrel_flower_sample_image` (1127), `test_extract_events_plural_from_squirrel_flower_flyer` (1207), `test_extract_events_plural_from_sample_flyer` (1237), `test_extract_events_from_classes_sample_image` (1297), `test_extract_event_from_ride_for_life_sample_image` (1542)
  - [ ] **Omitted by the original list — decide a destination for each:** `test_classes_schedule_relative_date_handling_next_weekday` (854), `test_extract_event_from_class_sample_image` (913), `test_extract_text_and_parse_events_from_classes_image_bytes` (1481), `test_parse_events_simple_mode_forces_deterministic` (1586)
- [ ] **Extract domain integration tests**
  - [ ] `tests/calendar_integration.rs` — calendar creation, permission, date-parsing tests
  - [ ] `tests/share_integration.rs` — `test_share_commands_flow` (`lib.rs:1514`) and App Group staging
- [ ] **Retain fast unit tests in source files**
  - [ ] Keep fast unit tests in `parser.rs` (or `parser/*`), `ocr.rs`, `model.rs`, `calendar.rs` under `#[cfg(test)] mod tests`
  - [ ] Verify `cargo test` still runs unit + integration tests identically

---

## 6. Tauri Command Layer Modularization

- [ ] **Delete dead starter scaffold** — **[verified]**
  - [ ] Remove `fn greet(name: &str)` (`lib.rs:19`) and its `generate_handler!` entry. `lib.rs` has 32 `#[tauri::command]` fns; removing `greet` leaves the 31 domain commands the plan enumerates.
- [ ] **Create modular command hierarchy in `src-tauri/src/commands/`**
  - [ ] `src/commands/mod.rs` re-exporting all domain command modules
  - [ ] `src/commands/ocr.rs`: `extract_text_from_image`, `extract_text_from_image_bytes`
  - [ ] `src/commands/events.rs`: `parse_events_from_text`, `parse_event_from_text`, `extract_events_from_image`, `extract_event_from_image`, `extract_events_from_image_bytes`, `extract_event_from_image_bytes`, `generate_event_prompt`, `get_event_schema`, `get_event_gbnf_grammar`, plus `parse_events_internal` / `parse_event_internal`
  - [ ] `src/commands/models.rs`: `get_model_manifest`, `get_model_statuses`, `get_model_status`, `download_model`, `cancel_model_download`, `delete_model`, `verify_model_hash`, `get_models_storage_info`, `open_models_directory`
  - [ ] `src/commands/inference.rs`: `unload_inference_model`, `is_inference_model_loaded`
  - [ ] `src/commands/calendar.rs`: `get_available_calendars`, `create_calendar_event`, `create_calendar_events`, `check_calendar_permission`, `request_calendar_permission`
  - [ ] `src/commands/share.rs`: `get_pending_shared_image`, `clear_pending_shared_image`, `stage_shared_image`, `load_image_from_path`
  - [ ] Note the ordering constraint with §5: the tests at `lib.rs:495+` reach these via `use super::*`; moving the commands out requires the visibility decision above.
- [ ] **Simplify `src/lib.rs` bootstrap**
  - [ ] Shrink non-test `lib.rs` (currently 494 LOC before `mod tests`) to module declarations plus `tauri::Builder` setup
  - [ ] Register the 31 commands via `commands::*`
- [ ] **Command parameter passing** — **[corrected; low value, do last or skip]** — **[deferred]**
  - These are **not** default clippy warnings — `cargo clippy` reports zero `lib.rs` diagnostics. They surface only under the pedantic `-W clippy::needless_pass_by_value`, which flags 22 arguments (24, 37, 325, 349, 350, 351, 357, 368-371, 383-386, 410, 415×2, 425×2, 444, 448) — more than the 9 originally listed.
  - `#[tauri::command]` arguments are deserialized from IPC. `&str` is supported via Tauri's `CommandArg`, but **`Vec<u8>` cannot become `&[u8]`** (`lib.rs:37`, `349`) and `Option<String>` rewrites buy nothing. Restrict this to the internal, non-command helpers where it is both legal and measurable.

---

## 7. Parser Decomposition & Architecture

**[verified]** — all listed symbols exist; `parser.rs` is 2271 LOC. Note that `extract_date` (1115), `extract_times` (1241), `extract_location` (1317), `extract_title` (1465) are **private**, so the split must add `pub(crate)` where a sibling submodule needs them.

- [ ] **Split `src/parser.rs` into `src-tauri/src/parser/` submodules**
  - [ ] `parser/mod.rs`: re-export `EventDetails`, `ReferenceContext`, `parse_events_deterministic`, `parse_event_deterministic`
  - [ ] `parser/rrule.rs`: `struct RecurrenceRule`, `new_weekly`, `to_rrule_string`, `parse_rrule`, `parse_weekdays_to_byday`
  - [ ] `parser/schema.rs`: `get_gbnf_grammar()`, `get_json_schema()`, `generate_extraction_prompt()`
  - [ ] `parser/schedule.rs`: `parse_schedule_table_events` (375), `parse_row_schedule_table_events`, `parse_columnar_schedule_table_events`, `extract_columnar_descriptions`, `clean_course_name`
  - [ ] `parser/agenda.rs`: `parse_agenda_events` (829)
  - [ ] `parser/datetime.rs`: `extract_date`, `extract_times`, `parse_hour_min`, `resolve_year`, `month_to_num`, `weekday_to_num`, `get_weekday_date` (800), `extract_term_until_date`, `is_date_pattern`, `is_time_pattern`, `is_date_or_time_line`
  - [ ] `parser/heuristics.rs`: `extract_title` + keyword scoring, `extract_location`, `clean_location_string`, `extract_description`, `is_noise_or_metadata_line`
  - [ ] `parser/regex.rs` (or keep in `mod.rs`): the §4 `LazyLock` statics, so submodules share one definition instead of re-forking the duplicates

---

## 8. Strongly-Typed Domain Error Handling

**[verified]** — `thiserror` is absent from `src-tauri/Cargo.toml`; `reqwest 0.12` is a direct dependency, so `From<reqwest::Error>` is available. Scope: 40-50 signatures across `lib.rs`, `ocr.rs`, `calendar.rs`, `model.rs`, `inference.rs`. Sequence this **after** §6/§7 so the migration touches already-split files once.

- [ ] **Introduce a structured error enum in `src-tauri/src/error.rs`**
  - [ ] Add `thiserror` to `src-tauri/Cargo.toml`
  - [ ] Define `AppError`: `Ocr(String)`, `Calendar(String)`, `CalendarPermissionDenied`, `ModelNotFound(String)`, `InsufficientDiskSpace { required_mb: u64, available_mb: u64 }`, `Download(String)`, `ChecksumMismatch(String)`, `Inference(String)`, `InferenceTimeout(u64)`, `Io(String)`
  - [ ] Implement `serde::Serialize` for Tauri IPC (commands may return any `Serialize` error type; the TS side currently receives plain strings — update the frontend error handling in the same change)
  - [ ] Implement `From<std::io::Error>`, `From<serde_json::Error>`, `From<reqwest::Error>`
- [ ] **Migrate internal functions from `Result<T, String>` to `Result<T, AppError>`**
  - [ ] `ocr::extract_text_from_path`, `ocr::extract_text_from_bytes`
  - [ ] `calendar::list_calendars`, `calendar::create_event`, `calendar::check_permission`
  - [ ] `model::get_manifest`, `model::start_model_download`, `model::verify_model`
  - [ ] `inference::InferenceEngineManager` methods

---

## 9. FFI Safety & Memory Guard Consolidation

**[verified]** — `AutoCString` is genuinely duplicated: `calendar.rs:96-106` (frees via `calendar_apple_free_string`) and `ocr.rs:180-190` (frees via `ocr_apple_free_string`); `share.rs:59, 78` frees manually via `ffi::free_share_string` with no guard (leaks on any early return between the call and the free). All three externs share the signature `unsafe extern "C" fn(*mut c_char)`, so a single guard parameterised by a `fn` pointer works.

- [ ] **Centralize native C string memory management**
  - [ ] Create `src-tauri/src/ffi/mod.rs` (or `src/ffi/guard.rs`)
  - [ ] Implement `NativeStringGuard` holding `*mut c_char` plus the free `fn(*mut c_char)`, with `Drop` and a null check
  - [ ] Safe conversions: `to_string_lossy() -> Option<String>`, `into_string() -> Result<String, _>`
  - [ ] Gate the module on `#[cfg(any(target_os = "macos", target_os = "ios"))]` to match the existing extern blocks
- [ ] **Refactor module FFI call sites**
  - [ ] Replace `AutoCString` in `ocr.rs`
  - [ ] Replace `AutoCString` in `calendar.rs`
  - [ ] Replace the manual `free_share_string` calls in `share.rs` (this one is a real leak fix, not just deduplication)

---

## 10. State Management & Dependency Injection

**[corrected]** — `BACKEND: OnceLock<Arc<LlamaBackend>>` is at `inference.rs:62`; `pub static ENGINE_MANAGER: LazyLock<InferenceEngineManager>` at `inference.rs:63`. `InferenceEngineManager::new()` (`inference.rs:95`) has no `Default` — this is the `clippy::new_without_default` warning at `inference.rs:95`. **Blocker the original plan missed:** `InferenceEngineManager::global()` is called outside any Tauri command — `inference.rs:610` and `:651` (unit tests) and `lib.rs:1635` (integration test). Deleting the singleton outright breaks those.

- [ ] **Unify global state under Tauri managed state**
  - [ ] Implement `Default` for `InferenceEngineManager` (clears `clippy::new_without_default`)
  - [ ] Register via `app.manage(inference::InferenceEngineManager::default())` in `lib.rs`
  - [ ] Inject `state: tauri::State<InferenceEngineManager>` into `unload_inference_model` and `is_inference_model_loaded`
  - [ ] Thread the manager explicitly through `extract_events_orchestrated` instead of its internal `global()` call
  - [ ] Only then remove `ENGINE_MANAGER` / `global()`, converting the three test call sites to construct their own instance
  - [ ] Keep `BACKEND: OnceLock<Arc<LlamaBackend>>` isolated in the inference subsystem (llama.cpp backend init is genuinely process-global)

---

## 11. Observability & Structured Logging

**[corrected]** — every line number in the original list was stale by roughly 60-70 lines, and four call sites were missing. No logging crate is present: neither `tracing`, `log`, nor `tauri-plugin-log` is in `Cargo.toml`.

- [ ] **Replace stray `eprintln!` with structured logging**
  - [ ] Add `tracing` (or `tauri-plugin-log`) to `src-tauri/Cargo.toml`
  - [ ] `inference.rs:180` `"[Debug] Sampler initialized before prompt decoding"` → `tracing::debug!` _(was listed as 188)_
  - [ ] `inference.rs:440` `"Failed to load manifest"` → `tracing::warn!` _(was 374)_
  - [ ] `inference.rs:455` `"Storage directory error"` → `tracing::error!` _(was 388)_
  - [ ] `inference.rs:468` `"Model ID not found in manifest"` → `tracing::warn!` _(was 403)_
  - [ ] `inference.rs:479` `"Model file does not exist"` → `tracing::info!` _(was 414)_
  - [ ] `inference.rs:492` `"Failed to load model weights"` → `tracing::error!` _(was 426)_
  - [ ] **Unlisted in the original plan:** `inference.rs:510`, `:519`, `:530` (further `eprintln!` fallback paths) and `lib.rs:1630`
  - [ ] `inference.rs:661` is a `println!` inside a test — leave it or convert to a test assertion; do not route it through `tracing`
  - [ ] The `println!` calls in `ocr.rs` and `parser.rs` are all test-local; out of scope

---

## 12. Platform Provider Traits & Cross-Platform Abstraction — **[deferred]**

**Rationale for deferring:** the stated benefit largely already exists and the cost is a full indirection layer. Today `calendar.rs` has a complete `#[cfg(not(any(target_os = "macos", target_os = "ios")))] mod fallback` with stub `check_permission` / `request_permission` / `list_calendars` / `create_event`; `share.rs` has filesystem fallback staging; `ocr.rs` returns an explicit "OCR is only supported on Apple platforms" error. The app ships Apple-only. Traits would add dynamic dispatch and a mock surface with no consumer.

- [ ] Revisit **only** when one of these becomes true:
  - [ ] A non-Apple target is actually planned (Android/Windows/Linux build in `tauri.conf.json`)
  - [ ] Unit tests need to inject a fake calendar/OCR provider to test orchestration without the platform — at which point start with `OcrProvider` alone (smallest surface: `extract_from_path`, `extract_from_bytes`) rather than all three at once
- [ ] Cheaper interim win: consolidate the per-file `#[cfg]` sprinkling (~8 attributes in `ocr.rs`, ~6 in `calendar.rs`, ~5 in `share.rs`) into one `apple` / `fallback` module pair per file, mirroring what `calendar.rs` already does. No trait required.

---

## 13. Clippy Diagnostics & Code Hygiene Fixes

**[corrected]** — verified against `cargo clippy --all-targets` (20 unique lib warnings). Three real warnings were missing from the original list, one line number was off by two, and "redundant string closures" does not fire anywhere.

- [ ] **Apply Clippy suggestions across `src-tauri/`**
  - [ ] `clippy::clone_on_copy` — `parser.rs:198`, use `*self.get_reference_datetime().offset()`
  - [ ] `clippy::manual_clamp` — `inference.rs:81`, use `count.clamp(1, 4)`
  - [ ] `clippy::collapsible_if` — `inference.rs:252`, `ocr.rs:106`, and **`parser.rs:1335`** _(missing from the original list)_
  - [ ] `clippy::if_same_then_else` — **`parser.rs:1263`** _(the plan said 1265)_
  - [ ] `clippy::manual_range_contains` — `parser.rs:1267`, use `(8..=11).contains(&start_h)`
  - [ ] `clippy::empty_line_after_doc_comments` — `model.rs:116`
  - [ ] `clippy::needless_return` — `model.rs:128`
  - [ ] `clippy::unnecessary_map_or` — `parser.rs:641`
  - [ ] `clippy::assign_op_pattern` — `parser.rs:822, 998, 1135, 1141`
  - [ ] `clippy::manual_pattern_char_comparison` — **`parser.rs:318`** (`[',', '/', '&', ' ', ';', '+']`) and **`parser.rs:1632`** (`['*', '-', '•']`) _(both missing from the original list)_
  - [ ] Redundant reference in `format!` argument — `model.rs:181, 287, 338` (drop the `&` on `entry.filename`). _(The original "redundant string closures and format string captures throughout the crate" matches nothing; these three are the actual diagnostics.)_
  - [ ] `clippy::new_without_default` — `inference.rs:95`; tracked as the first step of §10
  - [ ] Once §1 and §4 land, re-run `cargo clippy --all-targets` — `parser.rs` line numbers above will have shifted
