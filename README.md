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

### Standalone Install via Xcode (No Mac dev server needed)

To install and run the standalone production app directly on a connected iPhone:

1. **Build and open in Xcode**:
   ```sh
   bun run tauri ios build --open
   ```
2. **Select your iPhone as the destination**:
   - In Xcode’s top toolbar (next to the Play/Stop button), click the destination dropdown and choose your connected physical iPhone (instead of _Any iOS Device_ or a simulator).
3. **Set scheme to Release**:
   - Go to **Product > Scheme > Edit Scheme...** (`Cmd + <`).
   - Select **Run** on the left sidebar -> **Info** tab.
   - Set **Build Configuration** to **Release**.
   - Click **Close**.
4. **Check Signing**:
   - Select the root `share2cal` project in Xcode's project navigator.
   - Under **Targets**, verify the **Signing & Capabilities** tab for both `share2cal_iOS` and `ShareExtension` (ensure your Team is selected and automatic signing is valid).
5. **Deploy**:
   - Press **Run** (`Cmd + R` or ▶).
   - Xcode will compile, sign, install, and launch the standalone app on your iPhone. Once installed, it runs completely independently of your Mac.

> **First-time iPhone setup**:
>
> - **Developer Mode**: On iOS 16+, enable via **Settings > Privacy & Security > Developer Mode** (requires device restart).
> - **Trust Developer**: If prompted with "Untrusted Developer", go to **Settings > General > VPN & Device Management**, tap your developer certificate, and tap **Trust**.

```sh
# generate icons
bun tauri icon path/to/icon.svg
```
