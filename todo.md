# Share2Cal Platform Expansion TODO

Checklist and architectural specifications for adding native **Android** and **Windows** support to Share2Cal.

---

## 1. Android Support

### 1.1 Project Initialization & Build System

- [ ] Initialize Tauri Android project scaffold (`bun run tauri android init`).
- [ ] Configure `src-tauri/gen/android/` with appropriate Gradle plugins, Kotlin version, and min/target SDK (minSdk 26+, targetSdk 34+).
- [ ] Configure Android NDK toolchain and CMake flags for `llama_cpp_2` C/C++ compilation (targeting `arm64-v8a` and `armeabi-v7a`).
- [ ] Enable hardware acceleration backend in `llama.cpp` (Vulkan or OpenCL shaders for Adreno/Mali GPUs).
- [ ] Define required permissions in `AndroidManifest.xml` (`READ_CALENDAR`, `WRITE_CALENDAR`, `INTERNET` for model downloads, `CAMERA` for live scanning).

### 1.2 Native OCR (Google ML Kit Text Recognition)

- [ ] Add Google ML Kit Text Recognition dependency to `app/build.gradle.kts`:
  - `com.google.mlkit:text-recognition:16.0.1` (Bundled Latin model for 100% offline standalone execution without Google Play Services dependency).
- [ ] Create a Kotlin/Java OCR bridge or JNI layer (`OcrPlugin.kt` / `ocr_android.rs`):
  - Decode incoming image URI / file path / byte buffer into `InputImage`.
  - Process via `TextRecognizer.process(image)`.
  - Extract text blocks, lines, bounding boxes (normalized coordinates), and confidence metrics.
- [ ] Implement Rust FFI binding in `src-tauri/src/ocr.rs` (`#[cfg(target_os = "android")]`) returning `OcrResult`.
- [ ] Verify 2D spatial line clustering (`reconstruct_spatial_lines`) with ML Kit coordinate space.

### 1.3 System Share Sheet Integration

- [ ] Add `ACTION_SEND` and `ACTION_SEND_MULTIPLE` intent filters to `MainActivity` in `AndroidManifest.xml` for `image/*` MIME types.
- [ ] Handle incoming stream intents in `MainActivity.kt`:
  - Extract image URI from `Intent.EXTRA_STREAM`.
  - Copy/stage the shared image file to the app's cache directory (`cacheDir/shared/`).
  - Pass the staged file path or bytes to Tauri front-end / Rust share handler (`src-tauri/src/share.rs`).
- [ ] Add custom URL scheme or deep link handler for background/foreground handoff.

### 1.4 Native Calendar Integration (Android Calendar Provider)

- [ ] Implement Android Calendar permission flow:
  - Request runtime permissions (`Manifest.permission.READ_CALENDAR`, `Manifest.permission.WRITE_CALENDAR`).
- [ ] Implement `src-tauri/src/calendar.rs` Android backend via JNI / Tauri plugin:
  - Query writable calendars via `CalendarContract.Calendars`.
  - Insert single and batch events into `CalendarContract.Events` with `DTSTART`, `DTEND`, `ALL_DAY`, `EVENT_LOCATION`, `DESCRIPTION`, and `RRULE`.
  - Provide fallback intent launch (`Intent.ACTION_INSERT`) to pre-fill the user's default calendar app when direct write access is not granted.

### 1.5 Storage & Download Management

- [ ] Ensure model storage path (`model.rs`) resolves to internal app storage (`context.filesDir` / `no_backup` directory).
- [ ] Verify disk space checks (`get_free_disk_space`) using `StatFs` on Android paths.

---

## 2. Windows Support

### 2.1 Build System & Toolchain

- [ ] Configure MSVC C++ toolchain and Windows SDK dependencies in `src-tauri/Cargo.toml`.
- [ ] Add Windows-specific dependencies:
  - `windows = { version = "0.58", features = ["Media_Ocr", "Graphics_Imaging", "Storage_Streams", "ApplicationModel_Appointments", "Foundation", "Foundation_Collections"] }`
- [ ] Configure `llama_cpp_2` compilation flags for MSVC / Windows with AVX2/AVX-512 CPU support and optional DirectML / Vulkan GPU offloading.
- [ ] Configure Windows installer packaging in `tauri.conf.json` (NSIS / WiX MSI).

### 2.2 Native OCR (`Windows.Media.Ocr`)

- [ ] Implement `src-tauri/src/ocr_windows.rs` using WinRT `Windows.Media.Ocr`:
  - Initialize `OcrEngine::TryCreateFromUserProfileLanguages()` or specify fallback language.
  - Decode image bytes / path into a `SoftwareBitmap`.
  - Execute `ocr_engine.RecognizeAsync(&bitmap)`.
  - Extract recognized lines, words, bounding rectangles, and construct `OcrResult`.
- [ ] Connect `extract_text_from_path` and `extract_text_from_bytes` in `src-tauri/src/ocr.rs` under `#[cfg(target_os = "windows")]`.
- [ ] Add unit tests verifying Windows OCR extraction on sample image fixtures.

### 2.3 Share Target & File Ingestion

- [ ] Configure Windows file type association and "Open With" context menu handler for image formats (`.png`, `.jpg`, `.jpeg`, `.heic`, `.webp`).
- [ ] Implement CLI argument parsing in `main.rs` to open image files passed on launch.
- [ ] If packaging as MSIX / Windows App Package:
  - Declare `windows.shareTarget` extension in the package manifest for image types.
  - Handle `ShareTargetActivatedEventArgs` to stage the incoming shared image.
- [ ] Support Windows drag-and-drop file ingestion (already wired through Tauri webview).

### 2.4 Calendar Integration

- [ ] Implement `src-tauri/src/calendar_windows.rs`:
  - **Option A (Interactive Modal)**: Launch native Windows appointment composer via `Windows.ApplicationModel.Appointments.AppointmentManager.ShowAddAppointmentAsync(&appointment)`.
  - **Option B (Direct System / Shell)**: Generate floating `.ics` content and open via default system calendar handler (`ShellExecute` / `open::that`).
- [ ] Connect permission checks and calendar creation in `src-tauri/src/calendar.rs` under `#[cfg(target_os = "windows")]`.

### 2.5 Storage & Filesystem

- [ ] Map model storage directory (`model.rs`) to `%LOCALAPPDATA%\com.dustinmichels.share2cal\models`.
- [ ] Implement Windows `GetDiskFreeSpaceExW` in `get_free_disk_space` (`src-tauri/src/model.rs`).
- [ ] Implement `open_models_directory` using `explorer.exe` on Windows.

---

## 3. Cross-Platform Verification & Testing

- [ ] Add CI workflows (`.github/workflows/`) for Android APK build and Windows NSIS/MSI installer build.
- [ ] Validate end-to-end event extraction pipeline on all 4 platforms:
  1. Image Ingestion (Share / Open / Paste / Drag & Drop).
  2. Native OCR (`Apple Vision` / `Google ML Kit` / `Windows.Media.Ocr`).
  3. Spatial Line Clustering & Schedule Table Parser.
  4. Local LLM GBNF Structured Extraction with Deterministic Fallback.
  5. Native Calendar Insertion & `.ics` Backup Export.

---

## 4. QR Code & Barcode Ingestion

### 4.1 Native Detection & Ingestion Pipeline

- [ ] **Apple Vision Backend (`ocr_apple.m`)**:
  - Add `VNDetectBarcodesRequest` (`symbologies = @[VNBarcodeSymbologyQR]`) alongside `VNRecognizeTextRequest` in the `VNImageRequestHandler.performRequests` invocation.
  - Extract decoded payload strings (`obs.payloadStringValue`) and bounding boxes from `VNBarcodeObservation` results.
  - Serialize `qr_codes` array in the JSON returned across the C FFI boundary to Rust.
- [ ] **Android Backend (`ocr_android.rs` / `OcrPlugin.kt`)**:
  - Integrate Google ML Kit Barcode Scanning (`com.google.mlkit:barcode-scanning`) into the Android image processing pipeline.
  - Run barcode scanning concurrently with text recognition on the input bitmap.
- [ ] **Windows / Cross-Platform Fallback (`ocr_windows.rs` / pure Rust)**:
  - Evaluate `Windows.Media.Ocr` / `ZXing` / `rxing` crate for decoding barcodes on non-Apple/non-Android builds.

### 4.2 Data Model & First-Class URL Field

- [ ] **OCR Data Model (`src-tauri/src/ocr.rs` & `src/services/ocr.ts`)**:
  - Add `qr_codes: Vec<String>` with `#[serde(default)]` to `OcrResult` (ensures backward compatibility if omitted by any backend).
  - Mirror `qr_codes?: string[]` on TypeScript interface `OcrResult`.
- [ ] **Event Model Structs (`src-tauri/src/parser.rs` & `src/services/event.ts`)**:
  - Add `pub url: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` to `EventDetails` struct in Rust.
  - Update LLM JSON schema and GBNF grammar in `parser.rs` to include `"url": {"type": ["string", "null"]}`.
  - Add `url?: string | null` to `EventDetails` and `url: string` to `EventFormData` in TypeScript.
- [ ] **Calendar Creation Objects (`src-tauri/src/calendar.rs` & `src/services/calendar.ts`)**:
  - Add `url: Option<String>` to `CreateEventParams` struct in Rust and TypeScript interface.
  - Wire `url` to native Apple EventKit (`calendar_apple.m`: `event.URL = [NSURL URLWithString:...]`).
  - Wire `url` to Windows Appointment (`calendar_windows.rs`: `appointment.SetUri(...)`) and Android calendar provider.
  - Include `URL:<url>` in generated `.ics` calendar files (`calendar.rs` and `services/calendar.ts`).
- [ ] **Frontend Ingestion & UI (`src/App.vue`, `EventFormCard.vue`, `EventPreviewCard.vue`)**:
  - Update empty OCR extraction guard in `App.vue` (`!res.text.trim()`) to `!res.text.trim() && (!res.qr_codes || res.qr_codes.length === 0)` so QR-dominant flyers with minimal OCR text are processed.
  - Add editable "URL / Meeting Link" input field to `EventFormCard.vue` and link badge/preview to `EventPreviewCard.vue`.
  - Display detected QR links as interactive chips or clickable previews in `ImagePreviewCard.vue` / `OcrDrawer.vue`.

### 4.3 Parser & LLM Orchestration

- [ ] **Parser & LLM Context (`src-tauri/src/lib.rs` & `src-tauri/src/parser.rs`)**:
  - Append detected QR links/URLs to the text prompt supplied to the LLM (e.g., `Links / QR Codes: <url>`) so the model incorporates meeting/RSVP links into the event `url`, `description`, or `location`.
  - Update deterministic parser (`parse_event_deterministic`) to populate `EventDetails.url` with the first detected QR code or web link when available.
- [ ] **Calendar Export Verification**:
  - Verify that single and batch calendar additions persist the `url` property across both native calendar store and `.ics` download flows.

### 4.4 Fixtures & Verification

- [ ] Add regression and integration test coverage for `samples/commons.jpg`:
  - Verify QR code decodes to `https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw`.
  - Verify parsed event includes title ("CAMPUS AS COMMONS: Agroforestry and Shared Stewardship at Tufts"), speaker, date/time, and the webinar link in description/location.
