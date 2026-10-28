# IPC commands

Every command registered in `src-tauri/src/lib.rs`, grouped by the file in `src-tauri/src/commands/` that defines it. Argument names are shown as the frontend sends them (camelCase). All `Result` errors arrive in JavaScript as the `AppError` message string.

## Text and events

`commands/ocr.rs` and `commands/events.rs`.

| Command                           | Arguments                                                                               | Returns                           |
| --------------------------------- | --------------------------------------------------------------------------------------- | --------------------------------- |
| `extract_text_from_image`         | `path`                                                                                  | `OcrResult`                       |
| `extract_text_from_image_bytes`   | `bytes: number[]`                                                                       | `OcrResult`                       |
| `parse_events_from_text`          | `text`, `referenceTime?`, `timezoneOffsetMinutes?`, `modelId?`, `timeoutSecs?`, `mode?` | `EventDetails[]`                  |
| `parse_event_from_text`           | same as above                                                                           | `EventDetails` (first event)      |
| `extract_events_from_image`       | `path`, then the parse options                                                          | `EventDetails[]`                  |
| `extract_event_from_image`        | `path`, then the parse options                                                          | `EventDetails`                    |
| `extract_events_from_image_bytes` | `bytes`, then the parse options                                                         | `EventDetails[]`                  |
| `extract_event_from_image_bytes`  | `bytes`, then the parse options                                                         | `EventDetails`                    |
| `get_event_schema`                | none                                                                                    | JSON Schema for the event payload |
| `get_event_gbnf_grammar`          | none                                                                                    | GBNF grammar string               |
| `generate_event_prompt`           | `text`, `referenceTime?`, `timezoneOffsetMinutes?`                                      | The exact LLM prompt              |

Notes:

- `mode: "simple"` skips the model entirely. Any other value, including omitting it, allows the model. See the [decision order](/architecture/parsing#who-decides-rules-or-model).
- Both OCR commands replace `text` with spatially rebuilt rows when those parse as a schedule table.
- The `extract_*_from_image*` commands run OCR and parsing together and copy the first web QR link into events without a URL. The singular forms return a placeholder titled "New Event" with `source: "empty"` when nothing is found.
- Parse commands never fail because of the model; model errors fall back to the rule-based parser.

## Sharing

`commands/share.rs`.

| Command                      | Arguments                        | Returns                         |
| ---------------------------- | -------------------------------- | ------------------------------- |
| `get_pending_shared_image`   | `includeBytes?` (default `true`) | `SharedImagePayload \| null`    |
| `clear_pending_shared_image` | none                             | `void`                          |
| `stage_shared_image`         | `bytes`, `fileName`, `mimeType?` | `SharedImagePayload`            |
| `load_image_from_path`       | `path`                           | `SharedImagePayload` with bytes |

## Calendar

`commands/calendar.rs`.

| Command                       | Arguments                                                         | Returns                                                                        |
| ----------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `check_calendar_permission`   | none                                                              | `"not_determined" \| "authorized" \| "write_only" \| "denied" \| "restricted"` |
| `request_calendar_permission` | none                                                              | `boolean` granted                                                              |
| `get_available_calendars`     | none                                                              | `CalendarInfo[]` (writable only; empty without access)                         |
| `create_calendar_event`       | `event`, `calendarId?`, `calendarTitle?`, `calendarSourceTitle?`  | Event identifier                                                               |
| `create_calendar_events`      | `events`, `calendarId?`, `calendarTitle?`, `calendarSourceTitle?` | Event identifiers; validates all before writing any                            |

## Models

`commands/models.rs`.

| Command                   | Arguments | Returns                                                    |
| ------------------------- | --------- | ---------------------------------------------------------- |
| `get_model_manifest`      | none      | `ModelManifest`                                            |
| `get_model_statuses`      | none      | `ModelStatus[]`                                            |
| `get_model_status`        | `modelId` | `ModelStatus`                                              |
| `download_model`          | `modelId` | Resolves when the download finishes, fails or is cancelled |
| `cancel_model_download`   | `modelId` | `void`                                                     |
| `delete_model`            | `modelId` | `void`                                                     |
| `verify_model_hash`       | `modelId` | `boolean` hash matches the manifest                        |
| `get_models_storage_info` | none      | `ModelsStorageInfo`                                        |
| `open_models_directory`   | none      | `void`; errors on mobile                                   |

`download_model` also emits `model_download_progress` events while it runs.

## Inference

`commands/inference.rs`.

| Command                     | Arguments | Returns                        |
| --------------------------- | --------- | ------------------------------ |
| `unload_inference_model`    | none      | `void`; frees the cached model |
| `is_inference_model_loaded` | `modelId` | `boolean`                      |

## Adding a command

1. Write the logic in the relevant module (`ocr.rs`, `calendar.rs`, …) and return `Result<T, AppError>`.
2. Add a thin `#[tauri::command]` wrapper in the matching `commands/*.rs` file. Use `async` and `spawn_blocking` for anything slow so the IPC thread stays free.
3. Register it in `generate_handler![…]` in `lib.rs`.
4. Wrap it in the matching `src/services/*.ts` file with typed arguments and return value.
5. If it touches the OS on Apple platforms, add the C function to the right `*_apple.h` / `*_apple.m` pair, return strings through `NativeStringGuard`, and give non-Apple targets a `fallback` implementation.

The default capability (`capabilities/default.json`) grants `core:default` and `opener:default`. Custom commands registered with `generate_handler!` need no extra permission entries.
