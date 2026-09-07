# Calendar Destination & Integration Options

## 1. Context: Why Events Default to the "Work" Calendar

In `src-tauri/src/calendar_apple.m` (`calendar_apple_create_event`), Share2Cal creates events via Apple's **EventKit** framework:

```objc
// Retrieve default calendar or find first writable calendar
EKCalendar *targetCalendar = [store defaultCalendarForNewEvents];
if (!targetCalendar || !targetCalendar.allowsContentModifications) {
    NSArray<EKCalendar *> *calendars = [store calendarsForEntityType:EKEntityTypeEvent];
    for (EKCalendar *cal in calendars) {
        if (cal.allowsContentModifications) {
            targetCalendar = cal;
            break;
        }
    }
}
```

- `[store defaultCalendarForNewEvents]` returns whichever calendar is designated as the system default for new events in macOS/iOS Settings (**System Settings → Calendar → Default Calendar** on macOS; **Settings → Calendar → Default Calendar** on iOS).
- When no calendar is explicitly chosen by the app, EventKit routes the event to this system default (which is currently "Work" on the user's device).

---

## 2. Calendar Options & Technical Requirements

### Option A: In-App Calendar Selection (Device-Synced Google, iCloud, Exchange)

Google accounts synced to macOS/iOS via **System Settings → Internet Accounts** appear inside EventKit as native `EKCalendar` instances with source type `EKSourceTypeCalDAV`. No separate Google API or OAuth integration is required to write to them.

#### Requirements & Constraints

1. **Permission Upgrade (Write-Only to Full Access)**:
   - Current implementation in `calendar_apple.m` requests write-only access on iOS 17+ / macOS 14+:
     ```objc
     [store requestWriteOnlyAccessToEventsWithCompletion:...]
     ```
   - **Constraint**: Apple's write-only access sandbox forbids reading or enumerating existing calendars (`[store calendarsForEntityType:EKEntityTypeEvent]` returns an empty array).
   - **Alignment with Original Architecture**: `plan.md` Phase 4 already originally specified *"iOS: Request EKEntityType.event full access permissions via Info.plist"*. The shipped write-only implementation was a drift from the initial design; switching to full access is a return to original intent.
   - **Change needed**:
     - Switch to `requestFullAccessToEventsWithCompletion:` on iOS 17+ / macOS 14+ (and `requestAccessToEntityType:EKEntityTypeEvent` on older versions).
     - Add `NSCalendarsFullAccessUsageDescription` to `src-tauri/gen/apple/share2cal_iOS/Info.plist` and `src-tauri/gen/apple/project.yml`.
   - **Critical Trap — Existing Installs with Write-Only Status**:
     - Current permission gates only request access when status is `not_determined` (`calendar.ts:82` and `calendar_apple.m:126`).
     - Existing installs that previously granted write-only access will persistently report `write_only` (`EKAuthorizationStatusWriteOnly`), so without an explicit upgrade check, the app will never prompt for full access and the picker will render an empty list.
     - The calendar listing logic MUST treat `write_only` as insufficient for calendar enumeration, trigger `requestFullAccessToEventsWithCompletion:`, and handle the scenario where the user declines the upgrade by falling back to the default calendar behavior instead of displaying a broken/blank picker.
2. **Calendar Identifier Stability & Fallback Resolution**:
   - `EKCalendar.calendarIdentifier` is not guaranteed to be stable across device migrations, backup restores, or account re-syncs.
   - **Storage design**: Persist `{ id: string, title: string, sourceTitle: string }` in user settings (`localStorage`).
   - **Resolution logic**:
     1. Attempt lookup by `calendarIdentifier` (`[store calendarWithIdentifier:]`).
     2. If `nil`, fall back to matching by `title` + `sourceTitle`.
     3. If no match, fall back to `defaultCalendarForNewEvents` and return metadata indicating which calendar was used rather than silently failing.

3. **Backend Implementation**:
   - Objective-C (`src-tauri/src/calendar_apple.m` / `.h`):
     - `calendar_apple_list_calendars(char **out_json, char **out_error)` returning JSON-serialized array of `{ id, title, source_title, color, is_default, allows_modifications }`.
     - Update `calendar_apple_create_event` to accept an optional `const char *calendar_id`.
   - Rust (`src-tauri/src/calendar.rs`, `src-tauri/src/lib.rs`):
     - Structs for `CalendarInfo`.
     - Tauri command `get_available_calendars() -> Result<Vec<CalendarInfo>, String>`.
     - Update `create_calendar_event(event: EventDetails, calendar_id: Option<String>)`.

4. **Frontend Implementation**:
   - Service (`src/services/calendar.ts`, `src/services/settings.ts`):
     - Helper to fetch calendars and get/set preferred default calendar.
   - UI (`src/components/EventFormCard.vue`, `src/components/SettingsView.vue`):
     - Destination calendar selector in event review card and settings view.

---

### Option B: Direct "Add to Google Calendar" (Web / URL Scheme)

For users who do not sync their Google account to macOS/iOS Apple Calendar:

#### Requirements & Implementation

1. **Google Calendar Web Template URL**:
   - Generate template link prefilled with event parameters:
     ```ts
     function getGoogleCalendarUrl(event: EventDetails): string {
       const start = formatGoogleDate(event.start_time);
       const end = formatGoogleDate(event.end_time || event.start_time);
       const params = new URLSearchParams({
         action: "TEMPLATE",
         text: event.title,
         dates: `${start}/${end}`,
         details: event.description || "",
         location: event.location || "",
       });
       return `https://calendar.google.com/calendar/render?${params.toString()}`;
     }
     ```
2. **Launch Action**:
   - Open via Tauri opener plugin or browser window (`window.open(url, '_blank')`).
   - No permissions or API keys required.

---

### Option C: iCal (.ics Export & System Share Sheet)

Share2Cal already generates RFC 5545 `.ics` content via `generateIcsCalendarContent()` in `src/services/event.ts`.

#### Requirements & Implementation

1. **Native Share Sheet Integration**:
   - Provide a native Share Sheet trigger (`UIActivityViewController` on iOS, macOS system share) passing the temporary `.ics` file.
   - Enables users to open the event in third-party calendar apps (Fantastical, Outlook, BusyCal, etc.) or send via AirDrop/Messages.
2. **Action Sheet / Split Button UI**:
   - Allow user to choose destination from the review card:
     - **Add to Calendar** (EventKit default or selected calendar)
     - **Google Calendar** (Web link)
     - **Export / Share .ics** (File / Share Sheet)

---

## 3. Implementation Tasks

- [ ] **Permissions & Upgrade Migration**:
  - Switch `calendar_apple.m` permission requests to `requestFullAccessToEventsWithCompletion:`.
  - Add `NSCalendarsFullAccessUsageDescription` in `Info.plist` and `project.yml`.
  - Handle existing `write_only` installs: treat `write_only` as insufficient for listing, request full access upgrade, and gracefully fall back to default calendar if declined.
- [ ] **Backend Calendar Listing**: Implement `calendar_apple_list_calendars` and expose `get_available_calendars` Tauri command.
- [ ] **Backend Event Creation by ID**: Update `create_calendar_event` to accept an optional `calendar_id` and implement safe fallback resolution (ID → Title + Source → System Default).
- [ ] **Google Calendar URL Generator**: Add `generateGoogleCalendarUrl` in `src/services/calendar.ts`.
- [ ] **Settings & Persistence**: Add calendar selection preferences to `src/services/settings.ts` and `src/components/SettingsView.vue`.
- [ ] **UI Integration**: Add destination calendar selector / action options in `src/components/EventFormCard.vue`.
