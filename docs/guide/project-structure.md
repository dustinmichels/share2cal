# Project structure

Two halves: a Vue app in `src/` and a Rust crate in `src-tauri/`. Tests for each sit beside them, and `samples/` holds the real flyers both test suites run against.

```text
share2cal/
├── src/                        Vue 3 frontend (runs in the WebView)
│   ├── App.vue                 Root component: views, intake, scan flow
│   ├── main.ts                 createApp(App), loads styles/shared.css
│   ├── components/             UI pieces (see Frontend)
│   ├── services/               Typed wrappers around Tauri commands
│   │   ├── share.ts            Pending shares, drag-and-drop loading
│   │   ├── ocr.ts              OCR calls, URL helpers
│   │   ├── event.ts            Parse calls, date helpers, .ics, Google link
│   │   ├── calendar.ts         Permissions, calendar list, event creation
│   │   ├── model.ts            Model manifest, downloads, auto-download
│   │   └── settings.ts         localStorage-backed preferences
│   └── styles/shared.css       Design tokens and shared component classes
│
├── src-tauri/                  Rust crate share2cal_lib + Tauri config
│   ├── src/
│   │   ├── lib.rs              Builder: plugins, managed state, command list
│   │   ├── main.rs             Desktop entry, calls share2cal_lib::run()
│   │   ├── commands/           #[tauri::command] functions, grouped by area
│   │   ├── share.rs            App Group / staging folder intake
│   │   ├── ocr.rs              Vision bridge, spatial line rebuilding
│   │   ├── parser/             Deterministic parser, prompt, grammar
│   │   ├── inference.rs        llama.cpp engine, prompt run, JSON repair
│   │   ├── model.rs            Manifest, downloads, SHA-256, storage
│   │   ├── calendar.rs         EventKit bridge, mock store, batch create
│   │   ├── error.rs            AppError, serialised as a plain string
│   │   ├── ffi/guard.rs        RAII guard for C strings from Objective-C
│   │   └── *_apple.{h,m}       Objective-C bridges for share, OCR, calendar
│   ├── tests/                  Integration tests + samples validation
│   ├── gen/apple/              Xcode project, iOS app + ShareExtension
│   ├── capabilities/           Tauri permission sets
│   ├── build.rs                Compiles the .m files, links frameworks
│   ├── model-manifest.json     Models compiled into the binary
│   └── tauri.conf.json         App id, window, build hooks, icons
│
├── tests/                      bun tests for the frontend services
├── samples/                    Real flyers + parsed.json ground truth
├── docs/                       This site (VitePress)
├── vite.config.ts              Dev server on 1420, HMR for --host
└── mise.toml                   bun, rust, cmake
```

## Files worth knowing first

| File                               | Why it matters                                                                                                               |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/lib.rs`             | Lists every command the frontend can call and the two pieces of managed state: `DownloadState` and `InferenceEngineManager`. |
| `src-tauri/src/commands/events.rs` | The parse entry points. `parse_events_internal` decides between rules and the model.                                         |
| `src/App.vue`                      | Owns the whole user flow, from intake to the summary view. About 2,000 lines.                                                |
| `src-tauri/src/parser/mod.rs`      | `EventDetails`, the type that crosses every boundary, and the deterministic parser's top level.                              |
| `src-tauri/gen/apple/project.yml`  | XcodeGen spec: targets, URL scheme `share2cal`, App Group entitlements, extension activation rules.                          |

## Two manifests

`model-manifest.json` exists at the repo root and in `src-tauri/`. Only `src-tauri/model-manifest.json` is used: `model.rs` embeds it with `include_str!("../model-manifest.json")`. Edit that one, then rebuild.

## Other files in the root

- `requirements.md` is the original four-line brief.
- `parsed.md` lists the expected events for each image in `samples/`, the human-readable twin of `samples/parsed.json`.
- `changelog.md` records what changed and where.
