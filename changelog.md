# Changelog

## [Unreleased] - 2026-09-07

### Added
- **On-Device LLM Inference (`src-tauri/src/inference.rs`, `src/services/event.ts`)**:
  - Integrated `llama-cpp-2` with Metal GPU acceleration on macOS and iOS for fast local text processing.
  - Structured `CalendarEventDraft` extraction from OCR text with JSON schema prompting, grammar constraints, and timeout controls.
  - Deterministic regex and heuristic fallback parser for handling inference timeouts, missing models, or non-JSON outputs.
  - Added inference lifecycle management with active inference tracking and cancellation support (`extract_event_with_model`, `cancel_inference`).
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
