# Project completeness

Status of Share2Cal against [`requirements.md`](../requirements.md), as of 2026-10-27.

iOS is mostly complete. Android has not been started. On other platforms the share, OCR and calendar modules compile `fallback` implementations that either error out or fake success.

## Requirements coverage

| Requirement                 | iOS                                     | macOS                                                     | Android / Windows / Linux                                      |
| --------------------------- | --------------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------- |
| 1. Receive image via share  | Share Extension + App Group             | Staging folder; drag-drop, paste and file picker work     | No `src-tauri/gen/android`; staging-folder fallback only       |
| 2. Native OCR               | Apple Vision, including QR codes        | Apple Vision                                              | `ocr.rs` fallback returns an error                             |
| 3. Tiny LLM extraction      | llama.cpp on Metal, rule-based parser first | Same                                                  | Build likely broken (Metal enabled for every target)           |
| 4. Native calendar          | EventKit, single and batch, recurring events | EventKit code present, likely blocked by permissions | Fake calendars that report success without writing (see P0)    |

Tests at time of writing: `bun test` 58/58 pass; `cargo test` 108 pass, 1 ignored.

## P0: Calendar fallback fakes success

On every non-Apple platform, `src-tauri/src/calendar.rs:268-311`:

- `check_permission` returns `"authorized"`, `request_permission` returns `true`.
- `list_calendars` returns two made-up calendars ("Personal", "Work").
- `create_event` returns `Ok("mock_event_<title>")` and writes nothing.

That `Ok` becomes `result.success` in `App.vue:724-727`, which marks the event as added and shows the "Added to Calendar" toast (`App.vue:1038`). The batch path (`App.vue:673`) goes through the same fallback via `create_calendar_events`. Users are told their event was saved when it was not. OCR's fallback (`ocr.rs:249-255`), by contrast, returns an error.

Fix:

- [ ] Make the fallback's `check_permission`, `request_permission`, `list_calendars` and `create_event` return `AppError::Calendar("Device calendar is not supported on this platform")`.
- [ ] In the UI, hide or disable "Add to Calendar" when unsupported and point to `.ics` export or the Google Calendar link.
- [ ] Use the existing `calendar::mock` module (`calendar.rs:320`) for tests instead of fallback fake data. Before changing anything, check that `tests/calendar.test.ts` and `src-tauri/tests/calendar_integration.rs` don't rely on the fallback succeeding.

## P1: Android (requirements unmet)

- [ ] Run `bun run tauri android init` to generate `src-tauri/gen/android`.
- [ ] Write a Google ML Kit Text Recognition bridge to replace the OCR fallback in `src-tauri/src/ocr.rs`. The current error message mentions "Android ML Kit", but no such code exists.
- [ ] Write an Android Calendar Provider (`CalendarContract`) bridge to replace the fallback in `src-tauri/src/calendar.rs`.
- [ ] Handle the Android share intent (`ACTION_SEND` with `image/*`) to replace the staging-folder fallback in `src-tauri/src/share.rs`.
- [ ] Make the Metal GPU setting Apple-only. `src-tauri/Cargo.toml:37` enables `llama-cpp-2`'s `metal` feature for every target, so non-Apple builds will most likely fail. Move it under `[target.'cfg(any(target_os = "macos", target_os = "ios"))'.dependencies]` and choose a CPU (or Vulkan) configuration for other platforms. Not yet verified with an actual non-Apple build.

## P2: macOS calendar permissions

- [ ] Add a macOS `Info.plist` (or `bundle.macOS` config in `tauri.conf.json`) with `NSCalendarsUsageDescription` / `NSCalendarsFullAccessUsageDescription`. Without these, macOS will likely refuse calendar access or quit the app when it asks. See `docs/architecture/calendar.md#macos-gap`.

## P3: Quality and loose ends

- [ ] **LLM accuracy against sample answers.** `test_all_samples_llm_inference_enhanced_against_parsed_json` (`src-tauri/tests/samples_validation.rs:987`) is ignored because the model's times don't match the expected values in `samples/parsed.json`. Its ignore message cites "Section 3", which seems to be in the deleted `improvements.md`. Update the message and track the normalization work here.
- [ ] **Recurring classes never end.** `RecurrenceRule::new_weekly` is always called without an end date (`UNTIL`). `extract_term_until_date` in `src-tauri/src/parser/datetime.rs` exists but isn't used. Either connect it or delete it.
- [ ] **Unused commands.** `extract_events_from_image` and `extract_events_from_image_bytes` are no longer called from `App.vue`; only tests use the `src/services/event.ts` wrappers (`docs/architecture/pipeline.md`). Either remove them or document why they stay.
- [ ] **`src/App.vue` is ~58 KB** even after being split into components. Split it further before adding Android-specific screens.
- [ ] **Noisy test output.** `tests/model.test.ts` passes but prints `ReferenceError: window is not defined` because the background model download calls Tauri's `invoke` without mocking it.
- [ ] **README duplication.** The top-level `README.md` repeats dev commands that `docs/guide/getting-started.md` covers in more detail. Trim the README to link to the docs.
- [ ] If Android is no longer a goal, update `requirements.md` to say so.
