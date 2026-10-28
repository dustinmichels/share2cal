# Getting started

Install the toolchain, run the desktop build, then move to the iOS simulator or a phone. Desktop is the fastest loop for frontend and parser work; the share sheet only exists on iOS.

## Prerequisites

The repo pins its tools with [mise](https://mise.jdx.dev). From the repo root:

```sh
mise install
```

That installs the three tools listed in `mise.toml`:

| Tool    | Why                                                     |
| ------- | ------------------------------------------------------- |
| `bun`   | Package manager, script runner and frontend test runner |
| `rust`  | Builds the Tauri backend                                |
| `cmake` | `llama-cpp-2` compiles llama.cpp from source            |

You also need:

- **macOS with Xcode** for the Apple builds. `build.rs` compiles the Objective-C bridges and links Vision, EventKit, Metal and friends, so the full feature set only builds on a Mac.
- **iOS Rust targets.** `src-tauri/rust-toolchain.toml` requests `aarch64-apple-ios`, `aarch64-apple-ios-sim` and `x86_64-apple-ios`, and rustup adds them on first build. To add them by hand:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

## Install

```sh
bun install
```

The Xcode project in `src-tauri/gen/apple` is checked in, including the Share Extension target. Only run `bun run tauri ios init` if you are recreating it, and expect to re-add the extension afterwards.

## Run on desktop

```sh
bun tauri dev
```

Tauri runs `bun run dev` first (`beforeDevCommand` in `tauri.conf.json`), which starts Vite on port **1420** with `strictPort`, then compiles the Rust crate and opens an 800 × 600 window. The first build compiles llama.cpp and takes several minutes; later builds are incremental.

On macOS you get real OCR. Drag an image from `samples/` onto the window to try it. Saving to the device calendar may be refused on macOS, because the desktop build has no calendar usage strings yet; `.ics` export works. See [the macOS gap](/architecture/calendar#macos-gap).

::: tip Frontend only
`bun run dev` serves the Vue app in a normal browser. With no Tauri runtime, `src/services/ocr.ts` and `src/services/event.ts` return a canned "Weekly Pottery Class" result, so the UI is clickable without Rust.
:::

## Run on iOS

```sh
# Simulator or device chosen interactively
bun tauri ios dev

# Open the project in Xcode instead
bun tauri ios dev --open

# Physical device on the same network as your Mac
bun run tauri ios dev --host
```

`--host` binds the Vite dev server to your LAN address. `vite.config.ts` reads `TAURI_DEV_HOST` and moves HMR to a websocket on port 1421 so the phone can reach it.

To install a build that runs without your Mac, see [Running on iPhone](/guide/iphone).

## First run checklist

1. **Model download starts by itself.** With the default Enhanced mode, `App.vue` calls `checkDiskSpaceAndAutoDownloadDefaultModel()` on mount. It downloads SmolLM2 360M (about 270 MB) if there is at least 1.5× that free. Progress shows in Settings.
2. **Calendar permission is requested on first save,** not at launch. Listing calendars never prompts.
3. **Simple mode needs no model.** Switch to it in Settings if you want to iterate on the rule-based parser without waiting for downloads.

## Everyday commands

| Command                           | What it does                                                      |
| --------------------------------- | ----------------------------------------------------------------- |
| `bun tauri dev`                   | Desktop app with hot reload                                       |
| `bun tauri ios dev`               | iOS simulator or device                                           |
| `bun run build`                   | `vue-tsc --noEmit` then `vite build` into `dist/`                 |
| `bun test`                        | Frontend tests in `tests/`                                        |
| `cargo test` (in `src-tauri/`)    | Rust unit and integration tests                                   |
| `bun run fmt`                     | Format with `oxfmt`                                               |
| `bun tauri icon path/to/icon.svg` | Regenerate every app icon size                                    |
| `bun run docs:dev`                | This documentation site (run `bun install` in `docs/` once first) |

## Working on these docs

The site is a separate VitePress package in `docs/` with its own dependencies, so it never touches the app's Vite build.

```sh
cd docs && bun install && cd ..
bun run docs:dev       # live preview
bun run docs:build     # static site in docs/.vitepress/dist
```

Pages are Markdown under `docs/guide`, `docs/architecture` and `docs/reference`. The diagrams are Vue components in `docs/.vitepress/theme/components/`, and the theme lives in `docs/.vitepress/theme/custom.css`.

## Converting sample images

Some samples arrive from the phone as HEIF. Convert them with macOS's `sips` before committing:

```sh
sips -s format png samples/gilman_flyer.heif --out samples/gilman_flyer.png
```
