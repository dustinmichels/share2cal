# Data types

The structs that cross the IPC boundary. Rust is the source of truth; the TypeScript interfaces in `src/services/` mirror them field for field, in snake_case.

## EventDetails

Defined in `src-tauri/src/parser/mod.rs`, mirrored in `src/services/event.ts`. Every parser, the model, the editor and the calendar code speak this type.

```rust
pub struct EventDetails {
    pub title: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
    pub recurrence_rule: Option<String>,   // omitted from JSON when None
    pub url: Option<String>,               // omitted from JSON when None
    pub confidence: f32,
    pub source: String,
}
```

| Field                    | Format                                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| `start_time`, `end_time` | RFC 3339 with offset for timed events (`2026-11-12T18:00:00-05:00`); `YYYY-MM-DD` for all-day events |
| `recurrence_rule`        | RRULE body without the `RRULE:` prefix, e.g. `FREQ=WEEKLY;BYDAY=TU,TH`                               |
| `confidence`             | 0 to 0.95. Rule-based events are scored by which fields were found; model events are fixed at 0.95   |
| `source`                 | Which code produced the event, see below                                                             |

### Source values

| `source`                          | Produced by                                                                               |
| --------------------------------- | ----------------------------------------------------------------------------------------- |
| `deterministic`                   | Single-event rule parser                                                                  |
| `deterministic_agenda`            | Agenda parser                                                                             |
| `deterministic_schedule`          | Row-layout schedule table parser                                                          |
| `deterministic_schedule_columnar` | Columnar schedule table parser                                                            |
| `llm`                             | The local model                                                                           |
| `deterministic_fallback`          | Rule-based result returned because the model was unavailable or failed                    |
| `empty`                           | Placeholder from the singular `extract_event_from_image*` commands when nothing was found |

## OcrResult

`src-tauri/src/ocr.rs`, mirrored in `src/services/ocr.ts`.

```rust
pub struct OcrResult {
    pub text: String,
    pub lines: Vec<OcrLine>,
    pub qr_codes: Vec<String>,
}

pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
    pub bounding_box: Option<BoundingBox>,
}

pub struct BoundingBox { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
```

Bounding boxes are normalised to 0–1 with the origin at the bottom-left, as Vision reports them.

## SharedImagePayload

`src-tauri/src/share.rs`, mirrored in `src/services/share.ts`.

```rust
pub struct SharedImagePayload {
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub timestamp: u64,          // Unix seconds
    pub source: String,
    pub bytes: Option<Vec<u8>>,  // omitted from JSON when None
}
```

`source` is `ios_share_extension` or `native_apple_share` for App Group shares, `fallback_shared_dir` or `fallback_staging` for the staging folder, and `drag_and_drop` for `load_image_from_path`. Bytes arrive in JavaScript as a plain `number[]`; `payloadToFile()` turns them into a `File`.

## CalendarInfo

`src-tauri/src/calendar.rs`, mirrored in `src/services/calendar.ts`.

```rust
pub struct CalendarInfo {
    pub id: String,             // EKCalendar.calendarIdentifier
    pub title: String,
    pub source_title: String,   // account name, e.g. "iCloud"
    pub color: String,          // hex
    pub is_default: bool,
    pub allows_modifications: bool,
}
```

## Model types

`src-tauri/src/model.rs`, mirrored in `src/services/model.ts`.

```rust
pub struct ModelManifestEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub repo: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub quantization: String,
    pub description: String,
    pub recommended_ram: String,
    pub is_default: bool,
}

pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub file_path: Option<String>,
    pub storage_dir: String,
    pub sha256: String,
    pub is_verified: bool,
    pub error: Option<String>,
    pub quantization: String,
    pub description: String,
    pub recommended_ram: String,
    pub is_default: bool,
}

pub struct DownloadProgressPayload {
    pub model_id: String,
    pub received_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: f64,
    pub status: String,   // downloading | verifying | completed | error | cancelled
    pub error: Option<String>,
}

pub struct ModelsStorageInfo {
    pub storage_dir: String,
    pub total_models_downloaded: usize,
    pub total_models_size_bytes: u64,
    pub free_disk_space_bytes: Option<u64>,
}
```

## AppError

`src-tauri/src/error.rs`. Serialised as its display string, so the frontend sees only the message.

| Variant                                               | Message                                                               |
| ----------------------------------------------------- | --------------------------------------------------------------------- |
| `Ocr(String)`                                         | The inner message                                                     |
| `Calendar(String)`                                    | The inner message                                                     |
| `CalendarPermissionDenied`                            | Calendar access is denied. Please enable Calendar access in Settings. |
| `ModelNotFound(String)`                               | Model '…' not found in manifest                                       |
| `InsufficientDiskSpace { required_mb, available_mb }` | Insufficient disk space: … MB free, but … MB required.                |
| `Download(String)`                                    | The inner message                                                     |
| `ChecksumMismatch(String)`                            | SHA-256 checksum mismatch! Expected: …                                |
| `Inference(String)`                                   | The inner message                                                     |
| `InferenceTimeout(u64)`                               | Inference timed out after … seconds                                   |
| `Io(String)`                                          | The inner message                                                     |
