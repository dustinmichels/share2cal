# Saving to a calendar

Events can leave the app three ways. The device calendar goes through EventKit; the other two are generated in the frontend and work on any platform.

| Target          | Setting value | Implemented in                                                                 |
| --------------- | ------------- | ------------------------------------------------------------------------------ |
| Device calendar | `native`      | `calendar.rs` → `calendar_apple.m` (EventKit)                                  |
| Google Calendar | `google`      | `generateGoogleCalendarUrl` in `calendar.ts`                                   |
| `.ics` file     | `ics`         | `generateIcsCalendarContent` / `generateMultiIcsCalendarContent` in `event.ts` |

The default target in Settings (`share2cal_default_calendar_target`, default `native`) decides which action the summary and edit views put first. The others stay available.

## Device calendar

### Permissions

Status strings come from `calendar_apple_check_permission`:

| Status                 | Meaning                                                            |
| ---------------------- | ------------------------------------------------------------------ |
| `not_determined`       | Never asked                                                        |
| `authorized`           | Full access (iOS 17+ `fullAccess`, or the legacy authorised state) |
| `write_only`           | iOS 17+ write-only access: can add events, cannot list calendars   |
| `denied`, `restricted` | No access                                                          |

Listing calendars never prompts. `get_available_calendars` returns an empty list unless access is already granted, so opening Settings does not trigger a system dialog.

The prompt appears on the first save. `addEventToNativeCalendar` and `addEventsToNativeCalendar` in `calendar.ts` check the status, call `request_calendar_permission` if it is `not_determined`, and give up with a readable error on `denied` or `restricted`. On iOS 17 and macOS 14 and later the request is `requestFullAccessToEventsWithCompletion`; older systems use `requestAccessToEntityType`.

The iOS `Info.plist` carries the three calendar usage strings (`NSCalendarsUsageDescription`, `NSCalendarsFullAccessUsageDescription`, `NSCalendarsWriteOnlyAccessUsageDescription`). They are defined in `project.yml`.

### macOS gap

The desktop build has no equivalent. There is no `src-tauri/Info.plist`, and `tauri.conf.json` sets no macOS `infoPlist` or entitlements, so the macOS app never declares calendar usage strings. Without them, macOS privacy controls normally refuse the calendar request or terminate the app when it asks. Real EventKit writes are only expected to work on iOS until a macOS `Info.plist` with the `NSCalendars*UsageDescription` keys is added. OCR and the staging-folder share path do not need these keys.

### Choosing a calendar

The create commands accept a calendar id, title and account (source) title. `calendar_apple_create_event` resolves the target in this order, skipping any calendar that does not allow modifications:

1. The calendar with that identifier.
2. A calendar with the same title **and** account (source) title.
3. A calendar with the same title.
4. The system default calendar for new events.
5. The first writable calendar.

`App.vue` currently passes only the selected id, so steps 2 and 3 apply to callers that pass a `CalendarTargetOptions` object. Settings stores the choice twice: the id in `share2cal_default_calendar_id`, and `{ id, title, sourceTitle }` in `share2cal_calendar_preference`.

### Writing the event

`calendar.rs` converts each timestamp to Unix seconds (`parse_date_to_epoch` accepts RFC 3339, naive ISO with or without seconds, and bare dates), then passes everything to Objective-C as C strings. Missing values are filled in:

| Missing                        | Becomes        |
| ------------------------------ | -------------- |
| `start_time`                   | Now            |
| `end_time` on a timed event    | Start + 1 hour |
| `end_time` on an all-day event | Same as start  |

`location`, `description` (as notes) and `url` are set when present. A `recurrence_rule` is parsed into an `EKRecurrenceRule`; an invalid rule fails the save instead of silently dropping the repeat. The event is saved with `EKSpanThisEvent` and its `eventIdentifier` is returned.

### Batches

The summary's add-all button calls `create_calendar_event` once per event and collects per-event errors, so one bad event does not block the rest. If anything fails, the app downloads an `.ics` with every event as a fallback.

The backend also has a batch command, `create_calendar_events`. It first validates every event (non-empty title, parseable RRULE) and writes nothing if any check fails. It is **not** atomic after that: events are created in order, the loop stops at the first save error, and events already saved stay saved. It is used by tests and `createCalendarEvents` in `calendar.ts`, not by the current UI.

## Mock calendar

Tests must never write to a real calendar. `calendar::is_mock_active()` returns true, routing every call to an in-memory store, when any of these hold:

- `calendar::set_mock_mode(true)` has been called (the integration tests do this through `test_support::ensure_mock_calendar`),
- the code is compiled with `cfg(test)`,
- `SHARE2CAL_MOCK_CALENDAR` is set to anything except `0` or `false`,
- `RUST_TEST_NOCAPTURE` is set,
- the executable path looks like a Cargo test binary (`/target/` plus `/deps/` or `test`).

The mock store records created events so tests can assert on them (`get_mock_created_events`, `clear_mock_created_events`).

On non-Apple platforms the separate `fallback` module reports `authorized`, lists two fake calendars (Personal and Work) and returns `mock_event_<title>` without storing anything.

## `.ics` export

`event.ts` writes RFC 5545 files with `PRODID:-//Share2Cal//Event Parser//EN`:

- **Timed events** use UTC (`DTSTART:20261112T230000Z`).
- **All-day events** use `VALUE=DATE`. In multi-event files, a missing end becomes the next day, because `DTEND` is exclusive.
- **Recurring timed events** in multi-event files use floating local time so the wall-clock time survives daylight-saving changes.
- `RRULE` is round-tripped through `parseRecurrenceRule` and `buildRecurrenceRule`, which drops anything the app cannot represent.
- Text fields escape `\`, `;`, `,` and newlines.

A single event downloads as `<slugified-title>.ics`; several download as `events-schedule-<count>.ics`. The download is a Blob URL clicked from a temporary `<a download>` link.

## Google Calendar

`generateGoogleCalendarUrl` builds a `https://calendar.google.com/calendar/render?action=TEMPLATE&…` link and `openInGoogleCalendar` opens it with `@tauri-apps/plugin-opener`, so it goes to the system browser rather than the WebView. Timed events are sent in UTC; all-day events get an exclusive end date one day after the start.
