# Introduction

Share2Cal is a Tauri 2 app that turns a screenshot or photo of an event flyer into one or more calendar events. You share an image to it, it reads the text, and it hands you editable events to save.

## What it does

- **Takes images from anywhere.** The iOS share sheet, drag and drop on desktop, the clipboard, the camera or the photo library. Pasted plain text works too.
- **Reads the image on-device.** Apple's Vision framework recognises text line by line and decodes QR codes. Links found in QR codes become the event URL.
- **Finds the events.** A deterministic Rust parser produces a draft in milliseconds. It understands single events, multi-line agendas and class schedule tables, including recurring meetings.
- **Refines with a small local model.** In Enhanced mode, a quantised SmolLM2 or Qwen2.5 model running through llama.cpp re-reads the text and replaces the draft. Nothing leaves the device except the one-time model download.
- **Saves where you want.** Directly into a device calendar through EventKit, as an `.ics` file, or as a prefilled Google Calendar link.

## Platform support

The native pieces are Apple-only today. On other platforms the share, OCR and calendar modules compile `fallback` implementations instead, but no non-Apple build has been tested: `Cargo.toml` enables the `metal` feature of `llama-cpp-2` for every target, which is expected to break the llama.cpp build off Apple until that dependency is made Apple-specific.

| Capability                        | iOS                            | macOS                                                                                                   | Windows, Linux, Android       |
| --------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------- | ----------------------------- |
| Share sheet intake                | Share Extension + App Group    | App Group, then staging folder                                                                          | Staging folder                |
| Drag and drop, paste, file picker | Paste, picker, camera          | Yes                                                                                                     | Yes                           |
| OCR and QR codes                  | Apple Vision                   | Apple Vision                                                                                            | Returns an OCR error          |
| Device calendar                   | EventKit                       | EventKit, but no macOS usage strings yet (see [Saving to a calendar](/architecture/calendar#macos-gap)) | Two fake calendars, no writes |
| `.ics` export and Google link     | Yes                            | Yes                                                                                                     | Yes                           |
| Local LLM                         | llama.cpp, all layers on Metal | llama.cpp, all layers on Metal                                                                          | Untested; see above           |

::: info Android
The original [requirements](https://github.com/dustinmichels/share2cal/blob/main/requirements.md) call for Google ML Kit and the Android Calendar Provider. Neither is implemented yet, and `src-tauri/gen/android` has not been generated. Run `bun run tauri android init` before working on Android.
:::

## How the pieces fit

The app is a single Tauri process. A Vue 3 frontend runs in the system WebView and calls Rust through Tauri's `invoke`. The Rust library `share2cal_lib` owns all real work: OCR, parsing, inference, model downloads and calendar access. Apple frameworks are reached through three small Objective-C files that `build.rs` compiles and links.

The only other process is the iOS Share Extension. It cannot run the main app's code, so it copies the shared image into a container both targets can see and then opens the app.

Read [Architecture](/architecture/overview) for the full picture, or jump to [Getting started](/guide/getting-started) to run it.

## Tech stack

| Layer     | Choice                                                                |
| --------- | --------------------------------------------------------------------- |
| Shell     | Tauri 2 (`tauri`, `tauri-plugin-opener`)                              |
| Frontend  | Vue 3, TypeScript, Vite 8, `lucide-vue-next` icons                    |
| Backend   | Rust 2021, `tokio`, `serde`, `chrono`, `regex`, `reqwest`, `sha2`     |
| Inference | `llama-cpp-2` with the `metal` feature, GGUF models from Hugging Face |
| Native    | Objective-C against Vision, EventKit, ImageIO, Foundation             |
| Tooling   | `mise` (bun, rust, cmake), `bun test`, `cargo test`, `oxfmt`          |
