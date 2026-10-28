# Configuration

Everything you can change without editing logic: environment variables, compile-time constants, project config files and stored user preferences.

## Environment variables

| Variable                  | Read by                | Effect                                                                               |
| ------------------------- | ---------------------- | ------------------------------------------------------------------------------------ |
| `SHARE2CAL_MODELS_DIR`    | `model.rs`, Rust tests | Store and look for models here instead of `<app data>/models/`                       |
| `SHARE2CAL_SHARED_DIR`    | `share.rs`             | Staging folder for pending shares instead of `<temp>/share2cal_shared_images`        |
| `SHARE2CAL_MOCK_CALENDAR` | `calendar.rs`          | Any value except `0` or `false` routes calendar calls to the in-memory mock          |
| `TAURI_DEV_HOST`          | `vite.config.ts`       | Set by `tauri ios dev --host`; binds Vite to that address and moves HMR to port 1421 |

## Constants

| Constant                                  | Value                               | File                    |
| ----------------------------------------- | ----------------------------------- | ----------------------- |
| `DEFAULT_CONTEXT_WINDOW`                  | 2048 tokens                         | `inference.rs`          |
| `MAX_OUTPUT_TOKENS`                       | 512 tokens                          | `inference.rs`          |
| `MAX_PROMPT_TOKENS`                       | 1536 tokens (context minus output)  | `parser/schema.rs`      |
| `DEFAULT_MAX_GENERATION_TOKENS`           | 1536 tokens                         | `inference.rs`          |
| `DEFAULT_INFERENCE_TIMEOUT_SECS`          | 5                                   | `inference.rs`          |
| `get_optimal_thread_count()`              | Available cores, clamped to 1–4     | `inference.rs`          |
| `DEFAULT_APP_GROUP_ID`                    | `group.com.dustinmichels.share2cal` | `share.rs`              |
| `kAppGroupIdentifier`, `kCustomUrlScheme` | App Group, `share2cal://share`      | `ShareViewController.m` |
| Disk headroom for downloads               | 1.5 × model size                    | `model.rs`, `model.ts`  |

The App Group id appears in four places: `share.rs`, `ShareViewController.m` and both `.entitlements` files (generated from `project.yml`). Change all of them together.

## Project files

| File                                  | Controls                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/tauri.conf.json`           | Product name, version, bundle id `com.dustinmichels.share2cal`, dev URL, build hooks, window size, icons. CSP is disabled (`null`). |
| `src-tauri/capabilities/default.json` | Permissions for the `main` window: `core:default`, `opener:default`                                                                 |
| `src-tauri/model-manifest.json`       | Available models and the default; compiled in                                                                                       |
| `src-tauri/gen/apple/project.yml`     | iOS targets, deployment target (iOS 14), URL scheme, usage strings, App Group, Share Extension activation                           |
| `src-tauri/Cargo.toml`                | Rust dependencies. Release profile: LTO, `codegen-units = 1`, `opt-level = 3`, `panic = "abort"`, stripped                          |
| `src-tauri/rust-toolchain.toml`       | Extra iOS targets for rustup                                                                                                        |
| `vite.config.ts`                      | Dev server port 1420 (strict), HMR, ignores `src-tauri/` when watching                                                              |
| `mise.toml`                           | Tool versions                                                                                                                       |
| `bunfig.toml`                         | `bun test` root (`./tests`) and coverage ignores                                                                                    |
| `.oxfmtrc.json`                       | Formatter config                                                                                                                    |

## User preferences

Stored in the WebView's `localStorage` by `src/services/settings.ts`.

| Key                                 | Values                            | Default                          |
| ----------------------------------- | --------------------------------- | -------------------------------- |
| `share2cal_parsing_mode`            | `simple`, `enhanced`              | `enhanced`                       |
| `share2cal_default_calendar_target` | `native`, `google`, `ics`         | `native`                         |
| `share2cal_default_calendar_id`     | EventKit calendar id              | none, meaning the system default |
| `share2cal_calendar_preference`     | JSON `{ id, title, sourceTitle }` | none                             |

`clearStoredSettings()` removes all four.

## iOS usage strings

Set in `project.yml` and written to `share2cal_iOS/Info.plist`:

- `NSCalendarsUsageDescription`, `NSCalendarsFullAccessUsageDescription`, `NSCalendarsWriteOnlyAccessUsageDescription`
- `NSCameraUsageDescription`, `NSMicrophoneUsageDescription`
- `NSPhotoLibraryUsageDescription`, `NSPhotoLibraryAddUsageDescription`
