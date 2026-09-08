# Share2Cal

See [./requirements.md](./requirements.md).

```sh
bun install
bun run tauri android init
bun run tauri ios init

# For Desktop development, run:
bun run tauri dev

# For Android development, run:
bun run tauri android dev

# For iOS development, run:
bun run tauri ios dev
```

## Dev

```sh
bun tauri dev

# android / apple
bun tauri android dev
bun tauri ios dev
```

## Images

```sh
sips -s format png samples/gilman_flyer.heif --out samples/gilman_flyer.png
sips -s format png samples/ride_for_life.heif --out samples/ride_for_life.png
```

## Running on iphone

```sh
# Add iOS Rust targets (if not already installed)
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# open Xcode
bun tauri ios dev --open
bun run tauri ios dev --host

# or,
bun run tauri ios dev --host --open

# build
bun run tauri ios build --open
```

### Standalone Run (No Mac dev server needed)

If you want the app installed on your iPhone running purely standalone (using the bundled static frontend):

1.  In Xcode, go to Product > Scheme > Edit Scheme... (Cmd + <).
2.  Select Run on the left sidebar.
3.  Change Build Configuration from Debug to Release.
4.  Click Close and press Run (▶) to build and deploy to your iPhone.

```sh
# generate icons
bun tauri icon path/to/icon.svg
```
