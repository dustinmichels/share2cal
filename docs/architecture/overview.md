# Architecture

One Tauri process does almost everything. The Vue frontend is a thin client; the Rust library owns OCR, parsing, inference, downloads and calendar access, and reaches Apple frameworks through Objective-C.

<ArchDiagram />

## Layers

### Frontend

`src/App.vue` drives the user flow: intake, scanning, the event summary, the editor and settings. It never calls `invoke` directly; each area has a service module in `src/services/` that wraps the commands with TypeScript types and, where it helps development, a browser fallback. See [Frontend](/architecture/frontend).

### IPC

Communication is plain Tauri 2:

- **Commands.** The frontend calls `invoke("command_name", { camelCaseArgs })`. Tauri converts the arguments to the Rust function's snake_case parameters. All 31 commands are registered in one `generate_handler!` list in `lib.rs` and documented in [IPC commands](/reference/commands).
- **Events.** One event flows the other way: `model_download_progress`, emitted by `model.rs` during downloads and consumed by `onModelDownloadProgress()` in `model.ts`.
- **Errors.** Every fallible command returns `Result<T, AppError>`. `AppError` serialises to its display string, so the frontend receives a readable message as the rejected promise value.

### Rust core

`share2cal_lib` is split by responsibility. `commands/` holds only the thin `#[tauri::command]` functions; the logic lives in sibling modules.

| Module      | Responsibility                                                              |
| ----------- | --------------------------------------------------------------------------- |
| `share`     | Reads, stages and clears pending shared images                              |
| `ocr`       | Calls Vision, rebuilds rows from bounding boxes, filters QR links           |
| `parser`    | Deterministic event extraction, the LLM prompt and grammar, `EventDetails`  |
| `inference` | Loads GGUF weights once, runs generation with a deadline, repairs JSON      |
| `model`     | Reads the embedded manifest, downloads and verifies models, reports storage |
| `calendar`  | EventKit access, batch validation, an in-memory mock for tests              |

### Native bridges

`build.rs` runs only when the target OS is macOS or iOS. It compiles `ocr_apple.m`, `share_apple.m` and `calendar_apple.m` into a static library with ARC, then links Vision, CoreGraphics, ImageIO, Foundation, EventKit, Metal, MetalKit and Accelerate.

Each bridge returns heap-allocated C strings, usually JSON. Rust wraps them in `NativeStringGuard` (`ffi/guard.rs`), which calls the matching free function on drop, so early returns and panics cannot leak native memory.

On other platforms each module compiles a `fallback` submodule behind `#[cfg(not(any(target_os = "macos", target_os = "ios")))]`. See the platform table in the [Introduction](/guide/introduction#platform-support).

## Startup

`main.rs` calls `share2cal_lib::run()`, which builds the app:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .manage(model::DownloadState::default())
    .manage(inference::InferenceEngineManager::default())
    .invoke_handler(tauri::generate_handler![ /* 31 commands */ ])
    .run(tauri::generate_context!())
```

Two pieces of state live for the life of the process:

- **`DownloadState`** maps a model id to an `AtomicBool` cancel flag. `cancel_model_download` flips it, and the streaming loop checks it between chunks.
- **`InferenceEngineManager`** holds at most one loaded model behind a `tokio` mutex. The first enhanced parse pays the load cost; later parses reuse the same `Arc<LlamaModel>` until `unload_inference_model` drops it.

The llama.cpp backend is a process-wide `OnceLock<Arc<LlamaBackend>>`, initialised on first use.

When the WebView mounts, `App.vue` registers paste, focus, visibility and drag listeners, checks for a pending share, loads the calendar list and starts the default model download if needed.

## Design decisions

**Rules first, model second.** A small model on a phone is slow and sometimes wrong. The rule-based parser gives an answer in milliseconds and is the fallback for every model failure, so the model can only improve a result, never block one.

**Rust owns the logic.** OCR text cleanup, parsing and prompt building are all in Rust, where `cargo test` can run them against `samples/` without a WebView. The frontend formats and edits; it does not interpret.

**Objective-C over Swift.** The bridges are plain C functions in `.m` files built by the `cc` crate. That avoids a Swift toolchain step in `build.rs` and keeps the FFI surface to `char*` in, `char*` out.

**Models are downloaded, not bundled.** The smallest model is about 105 MB. Bundling would bloat every install, including users who only want Simple mode.
