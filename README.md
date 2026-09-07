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
```

## Running on iphone

```sh
# open Xcode
bun tauri ios dev --open
bun run tauri ios dev --host

# build
bun run tauri ios build --open

# or,
bun run tauri ios dev --host --open
```

### Standalone Run (No Mac dev server needed)

If you want the app installed on your iPhone running purely standalone (using the bundled static frontend):

1.  In Xcode, go to Product > Scheme > Edit Scheme... (Cmd + <).
2.  Select Run on the left sidebar.
3.  Change Build Configuration from Debug to Release.
4.  Click Close and press Run (▶) to build and deploy to your iPhone.
