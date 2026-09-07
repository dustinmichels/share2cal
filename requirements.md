# Requirements

I am making a native, cross-platform mobile app using Tauri 2 (bun, TS, vue, rust.)

The app allows users to share screenshots of text or images of flyers and automatically parse the text to create a calendar event. Requirements:

1. Receive an image via the share button
1. Native OCR (iOS: Apple Vision Framework, Android: Google ML Kit Text Recognition) to extract text from the image.
1. A tiny LLM to extract relevant information such as the event title, date, time, and location from the shared screenshot into a structured format to create a calendar event.
1. Native calendar APIs (iOS: EventKit, Android: Calendar Provider) to create the event in the user's calendar.

## Progress

### 1. Receive Image via Share Button

- [x] File picker upload (`<input type="file">` for PNG, JPEG, HEIF, WebP, etc.)
- [x] Camera capture input (`capture="environment"`) and iOS camera/photo library usage descriptions (`NSCameraUsageDescription`, `NSPhotoLibraryUsageDescription`, `NSPhotoLibraryAddUsageDescription`, `NSMicrophoneUsageDescription` in `src-tauri/gen/apple/project.yml` and `src-tauri/gen/apple/share2cal_iOS/Info.plist`)
- [x] Drag-and-drop file upload support in UI
- [x] Clipboard image paste support (`handlePaste`)
- [x] Native iOS Share Extension to receive images directly from the system share sheet (`src-tauri/gen/apple/ShareExtension`, App Group `group.com.dustinmichels.share2cal`, custom URL scheme `share2cal://share`, Rust bridge `src-tauri/src/share.rs`, and Vue service `src/services/share.ts`)
- [ ] Native Android Intent Filter (`ACTION_SEND` with `image/*`)

### 2. Native OCR (Apple Vision Framework / Android ML Kit)

- [x] **iOS / macOS:** Apple Vision Framework integration (`VNRecognizeTextRequest` with `.accurate` recognition and language correction) in `src-tauri/src/ocr_apple.m`
- [x] Objective-C to Rust FFI bridge (`src-tauri/src/ocr.rs`) with memory-safe buffer handling
- [x] Tauri IPC commands (`extract_text_from_image`, `extract_text_from_image_bytes`) and TypeScript service (`src/services/ocr.ts`)
- [x] UI display for OCR results, confidence metrics, word counts, and line-level bounding box details (`src/App.vue`)
- [x] Unit test suite verifying OCR on sample PNG and HEIF flyer images (`src-tauri/src/ocr.rs`)
- [ ] **Android:** Google ML Kit Text Recognition integration via JNI/NDK

### 3. Tiny LLM & Heuristic Event Extraction

- [ ] Local LLM inference engine integration (`llama.cpp` / GGUF model support)
- [ ] Structured JSON extraction with GBNF grammar / JSON schema (schema & grammar defined in `src-tauri/src/parser.rs`; pending llama.cpp engine)
- [x] Dynamic context injection (reference timestamps for relative date/time resolution) (`src-tauri/src/parser.rs`)
- [ ] On-device model download manager & persistent storage cache
- [x] Deterministic fallback parser (regex/heuristic date-time parsing) (`src-tauri/src/parser.rs`)

### 4. Native Calendar Integration

- [x] **iOS / macOS:** Native Apple EventKit integration (`EKEventStore` authorization status check, write-only/full permission requests, default/writable calendar selection, and `EKEvent` creation) in `src-tauri/src/calendar_apple.m` and `src-tauri/src/calendar_apple.h`
- [x] Objective-C to Rust FFI bridge (`src-tauri/src/calendar.rs`) with timezone-aware ISO/RFC3339 date parsing and fallback stubs for non-Apple targets
- [x] Tauri IPC commands (`create_calendar_event`, `check_calendar_permission`, `request_calendar_permission`) in `src-tauri/src/lib.rs` and TypeScript service (`src/services/calendar.ts`, `src/services/event.ts`)
- [x] iOS configuration (`EventKit.framework` dependency, `NSCalendarsUsageDescription`, and `NSCalendarsWriteOnlyAccessUsageDescription` in `src-tauri/gen/apple/project.yml` and `src-tauri/gen/apple/share2cal_iOS/Info.plist`)
- [x] UI integration with direct native "Add to Calendar" button, loading state, permission handling, success/warning feedback, and `.ics` export backup (`src/App.vue`)
- [ ] **Android:** Calendar Provider integration (`READ_CALENDAR`/`WRITE_CALENDAR`)
- [x] Event review and edit modal in UI before committing to calendar (`src/App.vue`, `src/services/event.ts`)
