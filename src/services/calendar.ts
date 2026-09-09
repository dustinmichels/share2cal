import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { EventDetails } from "./event";

export type CalendarPermissionStatus =
  | "not_determined"
  | "restricted"
  | "denied"
  | "authorized"
  | "write_only"
  | "unknown";

export interface CalendarInfo {
  id: string;
  title: string;
  source_title: string;
  color: string;
  is_default: boolean;
  allows_modifications: boolean;
}

export interface CalendarTargetOptions {
  id?: string | null;
  title?: string | null;
  sourceTitle?: string | null;
}

export interface CalendarResult {
  success: boolean;
  eventId?: string;
  error?: string;
}

export interface BatchCalendarResult {
  success: boolean;
  total: number;
  addedCount: number;
  eventIds: string[];
  errors?: string[];
}

function getErrorMessage(err: unknown): string {
  if (typeof err === "string") {
    return err;
  }
  if (err instanceof Error) {
    return err.message;
  }
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "An unknown calendar error occurred.";
}

/**
 * Checks the current calendar permission status on the device.
 */
export async function checkCalendarPermission(): Promise<CalendarPermissionStatus> {
  try {
    return await invoke<CalendarPermissionStatus>("check_calendar_permission");
  } catch (err) {
    console.warn("Failed to check calendar permission:", err);
    return "unknown";
  }
}

/**
 * Prompts the user for calendar access.
 */
export async function requestCalendarPermission(): Promise<boolean> {
  try {
    return await invoke<boolean>("request_calendar_permission");
  } catch (err) {
    console.warn("Failed to request calendar permission:", err);
    return false;
  }
}

/**
 * Retrieves all available writable calendars from the device (EventKit / system).
 * If permissions are not determined or denied, returns [] safely without prompting.
 */
export async function getAvailableCalendars(): Promise<CalendarInfo[]> {
  try {
    return await invoke<CalendarInfo[]>("get_available_calendars");
  } catch (err) {
    console.warn("Failed to retrieve available calendars:", err);
    return [];
  }
}

/**
 * Creates an event in the native device calendar via EventKit.
 * Accepts optional calendar target (id, title, sourceTitle) for multi-level resolution.
 */
export async function createCalendarEvent(
  event: EventDetails,
  calendarTarget?: string | CalendarTargetOptions | null,
): Promise<string> {
  const targetObj = typeof calendarTarget === "string" ? { id: calendarTarget } : calendarTarget;
  return await invoke<string>("create_calendar_event", {
    event,
    calendarId: targetObj?.id || null,
    calendarTitle: targetObj?.title || null,
    calendarSourceTitle: targetObj?.sourceTitle || null,
  });
}

/**
 * Creates multiple events in the native device calendar.
 */
export async function createCalendarEvents(
  events: EventDetails[],
  calendarTarget?: string | CalendarTargetOptions | null,
): Promise<string[]> {
  const targetObj = typeof calendarTarget === "string" ? { id: calendarTarget } : calendarTarget;
  return await invoke<string[]>("create_calendar_events", {
    events,
    calendarId: targetObj?.id || null,
    calendarTitle: targetObj?.title || null,
    calendarSourceTitle: targetObj?.sourceTitle || null,
  });
}

/**
 * High-level helper to add an event to the native calendar.
 * Automatically checks and requests permission if needed, then saves the event to the chosen calendar.
 * Preserves default calendar creation for write_only status even if full access was not granted.
 */
export async function addEventToNativeCalendar(
  event: EventDetails,
  calendarTarget?: string | CalendarTargetOptions | null,
): Promise<CalendarResult> {
  try {
    const status = await checkCalendarPermission();

    if (status === "denied" || status === "restricted") {
      return {
        success: false,
        error:
          "Calendar access is denied. Please enable Calendar access for Share2Cal in your device Settings.",
      };
    }

    if (status === "not_determined") {
      const granted = await requestCalendarPermission();
      if (!granted) {
        const postStatus = await checkCalendarPermission();
        if (postStatus !== "authorized" && postStatus !== "write_only") {
          return {
            success: false,
            error: "Calendar access was not granted.",
          };
        }
      }
    }

    const eventId = await createCalendarEvent(event, calendarTarget);
    return {
      success: true,
      eventId,
    };
  } catch (err: unknown) {
    return {
      success: false,
      error: getErrorMessage(err),
    };
  }
}

/**
 * High-level helper to add a list of events to the native calendar.
 */
export async function addEventsToNativeCalendar(
  events: EventDetails[],
  calendarTarget?: string | CalendarTargetOptions | null,
): Promise<BatchCalendarResult> {
  if (!events || events.length === 0) {
    return { success: true, total: 0, addedCount: 0, eventIds: [] };
  }

  try {
    const status = await checkCalendarPermission();

    if (status === "denied" || status === "restricted") {
      return {
        success: false,
        total: events.length,
        addedCount: 0,
        eventIds: [],
        errors: ["Calendar access is denied. Please enable Calendar access in Settings."],
      };
    }

    if (status === "not_determined") {
      const granted = await requestCalendarPermission();
      if (!granted) {
        const postStatus = await checkCalendarPermission();
        if (postStatus !== "authorized" && postStatus !== "write_only") {
          return {
            success: false,
            total: events.length,
            addedCount: 0,
            eventIds: [],
            errors: ["Calendar access was not granted."],
          };
        }
      }
    }

    const eventIds: string[] = [];
    const errors: string[] = [];

    for (const event of events) {
      try {
        const id = await createCalendarEvent(event, calendarTarget);
        eventIds.push(id);
      } catch (err) {
        errors.push(`${event.title}: ${getErrorMessage(err)}`);
      }
    }

    return {
      success: errors.length === 0,
      total: events.length,
      addedCount: eventIds.length,
      eventIds,
      errors: errors.length > 0 ? errors : undefined,
    };
  } catch (err: unknown) {
    return {
      success: false,
      total: events.length,
      addedCount: 0,
      eventIds: [],
      errors: [getErrorMessage(err)],
    };
  }
}

function parseLocalDateOnly(dateStr: string): Date | null {
  const match = dateStr.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (match) {
    const y = parseInt(match[1], 10);
    const m = parseInt(match[2], 10) - 1;
    const d = parseInt(match[3], 10);
    return new Date(y, m, d, 12, 0, 0);
  }
  const d = new Date(dateStr);
  return isNaN(d.getTime()) ? null : d;
}

function formatDateToCompact(d: Date): string {
  const y = d.getFullYear().toString().padStart(4, "0");
  const m = (d.getMonth() + 1).toString().padStart(2, "0");
  const day = d.getDate().toString().padStart(2, "0");
  return `${y}${m}${day}`;
}

function formatUtcToCompact(d: Date): string {
  const y = d.getUTCFullYear().toString().padStart(4, "0");
  const m = (d.getUTCMonth() + 1).toString().padStart(2, "0");
  const day = d.getUTCDate().toString().padStart(2, "0");
  const h = d.getUTCHours().toString().padStart(2, "0");
  const min = d.getUTCMinutes().toString().padStart(2, "0");
  const s = d.getUTCSeconds().toString().padStart(2, "0");
  return `${y}${m}${day}T${h}${min}${s}Z`;
}

/**
 * Formats a date string into Google Calendar compact ISO format (YYYYMMDDTHHmmssZ or YYYYMMDD).
 */
export function formatGoogleCalendarDate(
  dateStr: string | null | undefined,
  isAllDay: boolean = false,
): string {
  if (!dateStr || !dateStr.trim()) {
    const now = new Date();
    return isAllDay ? formatDateToCompact(now) : formatUtcToCompact(now);
  }

  const trimmed = dateStr.trim();

  // If date-only format (e.g. YYYY-MM-DD)
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
    const clean = trimmed.replace(/-/g, "");
    if (isAllDay) {
      return clean;
    }
    return `${clean}T000000Z`;
  }

  if (isAllDay) {
    const parsed = parseLocalDateOnly(trimmed);
    return parsed ? formatDateToCompact(parsed) : trimmed.replace(/[-:]/g, "");
  }

  const parsed = new Date(trimmed);
  if (isNaN(parsed.getTime())) {
    return trimmed.replace(/[-:]/g, "");
  }

  return formatUtcToCompact(parsed);
}

/**
 * Calculates Google Calendar end date/time string.
 * - Timed event: end_time in UTC, or start + 1 hour if missing/equal.
 * - All-day event: Google requires non-inclusive end date (e.g. 1-day event on Oct 15 -> end is Oct 16).
 */
export function getGoogleCalendarEndTimestamp(
  startStr: string | null | undefined,
  endStr: string | null | undefined,
  isAllDay: boolean = false,
): string {
  if (isAllDay) {
    if (!endStr || !endStr.trim() || endStr.trim() === startStr?.trim()) {
      const startDate = startStr ? parseLocalDateOnly(startStr.trim()) : new Date();
      const nextDay = new Date((startDate || new Date()).getTime() + 24 * 60 * 60 * 1000);
      return formatDateToCompact(nextDay);
    }

    const endDate = parseLocalDateOnly(endStr.trim());
    if (endDate) {
      const nextDay = new Date(endDate.getTime() + 24 * 60 * 60 * 1000);
      return formatDateToCompact(nextDay);
    }
    return formatGoogleCalendarDate(endStr, true);
  }

  if (endStr && endStr.trim() && endStr.trim() !== startStr?.trim()) {
    return formatGoogleCalendarDate(endStr, false);
  }

  const startDate = startStr ? new Date(startStr.trim()) : new Date();
  if (isNaN(startDate.getTime())) {
    return formatGoogleCalendarDate(startStr, false);
  }

  const oneHourLater = new Date(startDate.getTime() + 60 * 60 * 1000);
  return formatUtcToCompact(oneHourLater);
}

/**
 * Generates a Google Calendar direct web template URL for an event.
 */
export function generateGoogleCalendarUrl(event: EventDetails): string {
  const start = formatGoogleCalendarDate(event.start_time, event.is_all_day);
  const end = getGoogleCalendarEndTimestamp(event.start_time, event.end_time, event.is_all_day);

  const params = new URLSearchParams();
  params.set("action", "TEMPLATE");
  params.set("text", event.title || "Untitled Event");
  params.set("dates", `${start}/${end}`);

  let details = event.description && event.description.trim() ? event.description.trim() : "";
  if (event.url && event.url.trim()) {
    const trimmedUrl = event.url.trim();
    if (!details.includes(trimmedUrl)) {
      details = details ? `${details}\n\n${trimmedUrl}` : trimmedUrl;
    }
  }

  if (details) {
    params.set("details", details);
  }

  if (event.location && event.location.trim()) {
    params.set("location", event.location.trim());
  }

  if (event.recurrence_rule && event.recurrence_rule.trim()) {
    let rrule = event.recurrence_rule.trim();
    if (!rrule.startsWith("RRULE:")) {
      rrule = `RRULE:${rrule}`;
    }
    params.set("recur", rrule);
  }

  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (tz) {
      params.set("ctz", tz);
    }
  } catch {
    // Ignore timezone resolution error
  }

  return `https://calendar.google.com/calendar/render?${params.toString()}`;
}

/**
 * Opens an event directly in Google Calendar in the default web browser.
 */
export async function openInGoogleCalendar(event: EventDetails): Promise<void> {
  const url = generateGoogleCalendarUrl(event);
  try {
    await openUrl(url);
  } catch {
    if (typeof window !== "undefined") {
      window.open(url, "_blank");
    }
  }
}
