# Changelog

## [Unreleased] - 2026-09-07

### Added

- **Desktop Drag & Drop Image Import (`src-tauri/src/share.rs`, `src-tauri/src/lib.rs`, `src/services/share.ts`, `src/App.vue`, `src/components/UploadHub.vue`)**:
  - Added native desktop drag-and-drop support listening to Tauri webview events (`onDragDropEvent`) when files are dragged from Finder or File Explorer.
  - Added backend command `load_image_from_path` to read dropped image files from filesystem paths into memory payloads with MIME type detection.
  - Added frontend helper `loadImageFromPath` converting dropped payloads into browser `File` objects for seamless preview and OCR processing.
  - Added window-level drag overlay and dynamic visual feedback during drag operations with seamless image replacement support.
- **Multi-Event Parsing & Schedule Extraction (`src-tauri/src/parser.rs`, `src-tauri/src/inference.rs`, `src/services/event.ts`)**:
  - Added multi-event structured schema and GBNF grammar for extracting multiple distinct calendar events (e.g. academic class schedules, conference agendas, timetables) from a single flyer or screenshot.
  - Deterministic schedule table parser extracting course codes, titles, multi-day recurring meeting times (`Mo, We`, `Tu, Th`, `Fr`), locations (room/building/online), instructors, and unit details into distinct `EventDetails` with ISO-8601 timestamps.
  - Deterministic agenda parser extracting multiple timed line items with location and description metadata.
  - Orchestrated LLM inference supporting `EventsPayload` deserialization with seamless deterministic fallback.
  - Batch calendar integration (`create_calendar_events`, `addEventsToNativeCalendar`) and multi-event RFC 5545 `.ics` export (`generateMultiIcsCalendarContent`, `downloadMultiIcsFile`).
- **Event Preview Page & Two-Stage UI Workflow (`src/components/EventPreviewCard.vue`, `src/components/EventFormCard.vue`, `src/App.vue`)**:
  - Added `EventPreviewCard.vue` as an intermediate preview stage following image scan/OCR, presenting compact summary cards for single events or multi-event lists.
  - Interactive event cards showing title, timing, location, notes, and match confidence, with tap-to-edit navigation to the full event editing screen.
  - Batch actions on the preview page: "Add All to Calendar", "Export All (.ics)", "Copy All", and individual event removal.
  - Updated `EventFormCard.vue` with back navigation ("← Back to Events"), event position indicator ("Event X of Y"), and "Done Editing" actions.
- **Model Management & Downloader (`src-tauri/src/model.rs`, `model-manifest.json`, `src/services/model.ts`)**:
  - Direct Hugging Face LFS distribution for quantized GGUF models: `SmolLM2-360M-Instruct` (default, Q4_K_M, ~270 MB), `SmolLM2-135M-Instruct` (Q4_K_M, ~105 MB), and `Qwen2.5-0.5B-Instruct` (Q4_K_M, ~491 MB).
  - Resumable streaming HTTP downloader with `Range` header support, atomic `.part` staging, and graceful cancellation.
  - Pre-flight disk space checks (`statvfs`) requiring $\ge 1.5\times$ model file size before initiating downloads.
  - Post-download SHA-256 integrity verification, on-demand verification, and safe model deletion.
  - Real-time download progress events (`model_download_progress`) reporting transfer speed, downloaded bytes, total size, and percentage.
- **Modular Scan & Extraction UI (`src/components/`, `src/styles/shared.css`, `src/App.vue`)**:
  - Decomposed monolithic `App.vue` into reusable components:
    - `UploadHub.vue`: Drag-and-drop file upload, clipboard paste, and live camera capture triggers.
    - `ImagePreviewCard.vue`: Scaled flyer/document preview with quick action toolbar.
    - `OcrDrawer.vue`: Collapsible drawer to inspect, edit, and copy raw OCR text.
    - `EventFormCard.vue`: Editable structured event fields (title, start/end dates/times, all-day toggle, location, notes) with calendar export.
    - `SettingsNavCard.vue` & `SettingsView.vue`: Model manager interface showing storage breakdown, download progress, integrity checks, and deletion controls.
  - Added unified CSS design system (`shared.css`) with consistent tokens, glassmorphism cards, and responsive layouts.
- **Camera & Platform Permissions (`src-tauri/gen/apple/`, `capabilities/default.json`)**:
  - Added `NSCameraUsageDescription` and `NSPhotoLibraryUsageDescription` to iOS `Info.plist`.
  - Configured Tauri capability permissions for camera, dialog, filesystem, and notification access.
  - Updated iOS project configuration and Apple Share Extension bridging for Metal acceleration support.
- **Tauri IPC & Client Services**:
  - Registered IPC commands for model management (`get_model_manifest`, `get_model_statuses`, `get_model_status`, `download_model`, `cancel_model_download`, `delete_model`, `verify_model_hash`, `get_models_storage_info`) and inference (`extract_event_with_model`, `cancel_inference`).
  - Added typed TypeScript wrappers in `src/services/model.ts`, `src/services/event.ts`, and `src/services/settings.ts`.
