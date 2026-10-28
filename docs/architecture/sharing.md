# Getting images in

An image can arrive five ways. The iOS share sheet is the interesting one, because the Share Extension is a separate process that cannot call into the app.

## The iOS Share Extension

The extension target lives in `src-tauri/gen/apple/ShareExtension/` and is declared in `project.yml`:

| Setting         | Value                                        |
| --------------- | -------------------------------------------- |
| Bundle id       | `com.dustinmichels.share2cal.ShareExtension` |
| Extension point | `com.apple.share-services`                   |
| Principal class | `ShareViewController`                        |
| Activation      | Up to 10 images, or up to 10 files           |
| App Group       | `group.com.dustinmichels.share2cal`          |

### What the extension does

1. **Finds an image attachment.** It prefers `public.jpeg`, `public.png`, `public.heic`, `public.heif` and RAW types, then `public.image`, then file and web URLs. As a last resort it takes any attachment that conforms to `public.image`.
2. **Detects the real format** from the first bytes (`DetectExtensionFromData`), because type identifiers are not always accurate, and maps it to a MIME type.
3. **Writes into the shared container.** The image goes to `shared_images/` inside the App Group container, next to a `pending_share.json` manifest:

   ```json
   {
     "file_name": "shared_flyer_1767312000.png",
     "file_path": "/…/shared_images/shared_flyer_1767312000.png",
     "mime_type": "image/png",
     "size_bytes": 482113,
     "timestamp": 1767312000,
     "source": "ios_share_extension"
   }
   ```

   If the App Group container is unavailable, the extension writes to its own Caches directory instead. The app cannot read that location, so a missing App Group shows up as shares that never arrive.

4. **Opens the app** with the custom scheme `share2cal://share`, registered under `CFBundleURLTypes` in `project.yml`. Extensions have no public `openURL`, so it tries the extension context first, then walks the responder chain to `UIApplication`, then dismisses itself.

### How the app picks it up

The app does not handle the URL itself. Opening it just brings Share2Cal to the foreground, and `App.vue` checks for a pending share on three occasions:

- on mount,
- on window `focus`,
- on `visibilitychange` when the document becomes visible.

`checkPendingShare()` calls `get_pending_shared_image` with `includeBytes: true`. In Rust, `share.rs` asks `share_apple.m` for the manifest JSON in the App Group container. If there is no manifest, the bridge returns the most recently modified file in `shared_images/`. If the App Group is unavailable, for example in an unsigned simulator build, it falls back to the staging folder below.

When a payload with bytes comes back, the frontend:

1. converts it to a `File` with `payloadToFile()`,
2. calls `clear_pending_shared_image`, which deletes `shared_images/` so the same image is not processed twice,
3. runs `handleGo()` to scan immediately.

## The staging folder

On every platform without an App Group (and as the fallback on Apple platforms), pending shares live in a plain directory:

```text
$SHARE2CAL_SHARED_DIR             if set
<system temp>/share2cal_shared_images   otherwise
```

It uses the same `pending_share.json` layout. `stage_shared_image` writes into it, which is how the Rust tests and desktop development simulate a share without the extension.

## Desktop drag and drop

`App.vue` subscribes to `getCurrentWebview().onDragDropEvent` when running under Tauri. The `enter`, `over` and `leave` events drive a full-window drop overlay; `drop` hands the first path to `loadImageFromPath()`.

That calls `load_image_from_path`, which reads the file, infers a MIME type from the extension (`jpg`, `png`, `heic`, `heif`, `webp`, `gif`, `bmp`, `tiff`, `svg`) and returns the bytes with `source: "drag_and_drop"`. In a plain browser, standard HTML5 `dragover` and `drop` listeners do the same job with `DataTransfer.files`.

## Paste, camera and library

- **Paste.** A window-level `paste` listener takes the first `image/*` item from the clipboard. If the clipboard holds text instead, it goes straight to `handlePastedText()` and skips OCR.
- **Camera.** A hidden `<input type="file" accept="image/*" capture="environment">` opens the rear camera on phones.
- **Library.** A second hidden file input without `capture` opens the photo picker or file dialog.
- **Typed text.** `UploadHub` has a dialog for pasting or typing event text when clipboard access is unavailable.

Only the share sheet scans automatically. The other image paths show `ImagePreviewCard`, where you choose the parsing mode and press Scan.
