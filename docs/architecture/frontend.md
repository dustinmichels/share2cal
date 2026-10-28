# Frontend

A Vue 3 single-file-component app with no router and no store library. `App.vue` holds the state, the components render it, and `src/services/` is the only code that talks to Rust.

## Views

`App.vue` switches between three views with a `currentView` ref:

| View       | Shown when                          | Main components                                                              |
| ---------- | ----------------------------------- | ---------------------------------------------------------------------------- |
| `main`     | Nothing scanned yet, or after reset | `UploadHub`, then `ImagePreviewCard` once an image is chosen                 |
| `summary`  | After the first parse returns       | `ImageReferenceCard` or `TextReferenceCard`, `EventPreviewCard`, `OcrDrawer` |
| `settings` | The gear button                     | `SettingsView`                                                               |

Editing is an overlay on the summary rather than a view. Setting `selectedEventIndex` opens `EventFormCard` with a flattened copy of the event (`eventForm`: date, start and end times, all-day flag, location, notes, URL, recurrence); saving writes it back into `eventsList`.

`EventPreviewCard` has two layouts, chosen by `previewMode`: a list of event cards (`details`) and a week grid (`week`).

## State

The important refs in `App.vue`:

| Ref                                                              | Holds                                                   |
| ---------------------------------------------------------------- | ------------------------------------------------------- |
| `selectedFile`, `previewUrl`                                     | The chosen image and its object URL                     |
| `ocrResult`                                                      | The last `OcrResult`, plus merged QR and text links     |
| `eventsList`                                                     | The `EventDetails[]` being reviewed                     |
| `parsingMode`                                                    | `simple` or `enhanced`, from settings                   |
| `isProcessing`                                                   | OCR or first parse in progress                          |
| `isReparsingWithLlm`                                             | The enhanced pass is running in the background          |
| `addedEventIndices`                                              | Which events were already saved, so buttons can show it |
| `selectedCalendarId`, `availableCalendars`                       | Calendar picker state                                   |
| `errorMessage`, `calendarErrorMessage`, `calendarSuccessMessage` | User-facing feedback                                    |

## Components

| Component                 | Role                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `UploadHub.vue`           | Landing drop zone with buttons for library, camera and text entry. Emits `selectFile`, `chooseFile`, `takePhoto`, `pasteText`. |
| `ImagePreviewCard.vue`    | The chosen image, file details, the Simple/Enhanced toggle and the Scan button.                                                |
| `EventPreviewCard.vue`    | Extracted events as cards or a week grid, confidence badges, add-all, export-all and per-event edit and remove.                |
| `EventFormCard.vue`       | Field-by-field editor for one event, with save, add, export and Google actions.                                                |
| `OcrDrawer.vue`           | Collapsible raw OCR text with line confidences and detected links, plus copy.                                                  |
| `ImageReferenceCard.vue`  | Thumbnail of the source image on the summary view.                                                                             |
| `TextReferenceCard.vue`   | The pasted text, when the events came from text instead of an image.                                                           |
| `MobileFloatingPanel.vue` | Bottom action bar on narrow screens.                                                                                           |
| `SettingsView.vue`        | Parsing mode, default target and calendar, calendar permission, and the model manager.                                         |

## Services

Each service file wraps one area's commands with types that mirror the Rust structs.

| File          | Commands                                                                                                                                 | Also contains                                                           |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `share.ts`    | `get_pending_shared_image`, `clear_pending_shared_image`, `stage_shared_image`, `load_image_from_path`                                   | `payloadToFile()`                                                       |
| `ocr.ts`      | `extract_text_from_image`, `extract_text_from_image_bytes`                                                                               | `isWebLink`, `extractUrlsFromText`                                      |
| `event.ts`    | The parse and extract commands, `get_event_schema`, `get_event_gbnf_grammar`, `generate_event_prompt`                                    | Date and time formatting, RRULE helpers, `.ics` generation and download |
| `calendar.ts` | `check_calendar_permission`, `request_calendar_permission`, `get_available_calendars`, `create_calendar_event`, `create_calendar_events` | Permission-aware add helpers, Google Calendar URL                       |
| `model.ts`    | All model and inference commands                                                                                                         | `onModelDownloadProgress`, auto-download, byte and speed formatting     |
| `settings.ts` | None                                                                                                                                     | `localStorage` getters and setters                                      |

### Browser fallbacks

`ocr.ts` and `event.ts` catch `invoke` failures and, when `isTauri()` is false, return a fixed "Weekly Pottery Class" sample. That keeps `bun run dev` usable in a normal browser for layout work. Inside Tauri the error is rethrown.

### Argument naming

Tauri maps camelCase keys in the `invoke` payload to snake_case Rust parameters. Services always send camelCase:

```ts
invoke("parse_events_from_text", {
  text,
  referenceTime, // reference_time
  timezoneOffsetMinutes, // timezone_offset_minutes
  modelId,
  timeoutSecs,
  mode, // "simple" | "enhanced"
});
```

Return values are not renamed, which is why TypeScript interfaces use snake_case fields such as `start_time` and `is_all_day`.

## Settings

All preferences live in `localStorage` through `settings.ts`, with an in-memory fallback when storage is unavailable.

| Key                                 | Values                            | Default               |
| ----------------------------------- | --------------------------------- | --------------------- |
| `share2cal_parsing_mode`            | `simple`, `enhanced`              | `enhanced`            |
| `share2cal_default_calendar_target` | `native`, `google`, `ics`         | `native`              |
| `share2cal_default_calendar_id`     | EventKit calendar id              | none (system default) |
| `share2cal_calendar_preference`     | JSON `{ id, title, sourceTitle }` | none                  |

Turning on Enhanced mode in Settings with no model downloaded first checks free space against 1.5× the default model's size and warns if it is short.

## Styling

`src/styles/shared.css` defines the app's tokens and shared classes: grouped iOS-style surfaces (`.surface-card`), touch-sized buttons (`.btn-touch` and its variants, at least 48 px tall), and badges. It follows the system font stack and switches to a dark palette with `prefers-color-scheme`. Component-specific styles are scoped inside each `.vue` file. Icons come from `lucide-vue-next`.
