---
timeline:
  - who: Extension
    what: The user picks Share2Cal in the share sheet. <code>ShareViewController.m</code> loads the image, writes it to <code>shared_images/</code> in the App Group container with a <code>pending_share.json</code> manifest, and opens <code>share2cal://share</code>.
  - who: Frontend
    what: The app comes to the foreground. A <code>focus</code> or <code>visibilitychange</code> event runs <code>checkPendingShare()</code>.
    code: getPendingSharedImage(true) → invoke("get_pending_shared_image")
  - who: Rust
    what: <code>share.rs</code> reads the manifest through <code>share_apple.m</code> and returns the image bytes. The frontend turns them into a <code>File</code>, clears the share and calls <code>handleGo()</code>.
    code: clear_pending_shared_image
  - who: Rust
    what: The bytes go to Vision. Rust rebuilds rows from the bounding boxes, and if the rows look like a class schedule it swaps them in for the plain text.
    code: extract_text_from_image_bytes
  - who: Frontend
    what: <code>processExtractedText()</code> merges QR links with any URLs in the text and appends them under a <code>Links / QR Codes:</code> heading.
  - who: Rust
    what: First parse with <code>mode&#58; "simple"</code>. The rule-based parser answers in milliseconds and the summary view opens with these events.
    code: parse_events_from_text(mode = "simple")
  - who: Rust
    what: If the parsing mode is Enhanced and the default model is downloaded, a second parse runs in the background. The summary shows a refining indicator meanwhile.
    code: parse_events_from_text(mode = "enhanced")
  - who: Frontend
    what: If the enhanced parse returns events, they replace the draft. Any open edit form re-syncs to the new values. On error the draft stays.
  - who: You
    what: Review, edit or delete events, then save them. The summary's primary button follows the default target in Settings.
  - who: Native
    what: Each event is written through EventKit. If that fails for any reason, the app downloads an <code>.ics</code> with every event instead.
    code: create_calendar_event (once per event)
---

# The scan pipeline

What happens between sharing a flyer and seeing it in your calendar. The order below is the real call order in `App.vue`, from the iOS share sheet to EventKit.

<Timeline :items="$frontmatter.timeline" />

## Two passes, one screen

The scan deliberately parses twice:

| Pass   | Mode       | Typical time                                               | Purpose                                           |
| ------ | ---------- | ---------------------------------------------------------- | ------------------------------------------------- |
| Draft  | `simple`   | Milliseconds                                               | Show something immediately and guarantee a result |
| Refine | `enhanced` | Up to the 5 s inference timeout, plus a one-off model load | Better titles, descriptions and recurrence        |

The refine pass is skipped when the parsing mode is Simple, or when the default model is not downloaded yet. Re-running it later from the summary view (`handleTryAgain`) starts the model download if needed and asks you to try again once it finishes.

Both passes go through the same command, `parse_events_from_text`. The decision logic is described in [Rule-based parsing](/architecture/parsing) and [On-device LLM](/architecture/local-llm).

## Other ways in

Sharing is one of five entry points. They all converge on the same two functions:

| Entry                   | Handler                                   | Converges on                                    |
| ----------------------- | ----------------------------------------- | ----------------------------------------------- |
| iOS share sheet         | `checkPendingShare()`                     | `handleGo()`                                    |
| Drag and drop (desktop) | `onDragDropEvent` → `loadImageFromPath()` | `setImageFile()`, then Scan                     |
| Paste an image          | window `paste` listener                   | `setImageFile()`, then Scan                     |
| Camera or photo library | hidden `<input type="file">`              | `setImageFile()`, then Scan                     |
| Paste or type text      | `paste` listener or the text dialog       | `handlePastedText()` → `processExtractedText()` |

Text input skips OCR entirely. Shared images are scanned automatically; everything else waits for you to press Scan in `ImagePreviewCard`.

## Unused all-in-one commands

The backend also exposes `extract_events_from_image` and `extract_events_from_image_bytes`, which run OCR and parsing in one call and attach the first QR link to events without a URL. `App.vue` does not use them; it calls OCR and parsing separately so it can show the draft before the model runs. The wrappers in `src/services/event.ts` remain for tests and future callers.
