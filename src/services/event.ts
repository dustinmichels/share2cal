import { invoke } from "@tauri-apps/api/core";

export * from "./calendar";

export interface EventDetails {
  title: string;
  start_time: string | null;
  end_time: string | null;
  is_all_day: boolean;
  location: string | null;
  description: string | null;
  confidence: number;
  source: string;
}

export function getCurrentReferenceTime(): { referenceTime: string; timezoneOffsetMinutes: number } {
  const now = new Date();
  const timezoneOffsetMinutes = -now.getTimezoneOffset(); // e.g. -240 for EDT
  return {
    referenceTime: now.toISOString(),
    timezoneOffsetMinutes,
  };
}

export async function parseEventFromText(
  text: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number
): Promise<EventDetails> {
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails>("parse_event_from_text", {
    text,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
  });
}

export async function extractEventFromImage(
  path: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number
): Promise<EventDetails> {
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails>("extract_event_from_image", {
    path,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
  });
}

export async function extractEventFromImageBytes(
  bytes: Uint8Array | number[],
  referenceTime?: string,
  timezoneOffsetMinutes?: number
): Promise<EventDetails> {
  const ref = getCurrentReferenceTime();
  const payload = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  return await invoke<EventDetails>("extract_event_from_image_bytes", {
    bytes: payload,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
  });
}

export async function getEventSchema(): Promise<any> {
  return await invoke("get_event_schema");
}

export async function getEventGbnfGrammar(): Promise<string> {
  return await invoke<string>("get_event_gbnf_grammar");
}

export async function generateEventPrompt(
  text: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number
): Promise<string> {
  const ref = getCurrentReferenceTime();
  return await invoke<string>("generate_event_prompt", {
    text,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
  });
}

// Formatting helpers
export function formatDateForDisplay(isoString: string | null): string {
  if (!isoString) return "Date not specified";
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return isoString;
    return d.toLocaleDateString(undefined, {
      weekday: "short",
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return isoString;
  }
}

export function formatTimeForDisplay(isoString: string | null): string {
  if (!isoString) return "";
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return "";
    return d.toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

export function extractDateInput(isoString: string | null): string {
  if (!isoString) {
    const today = new Date();
    return today.toISOString().split("T")[0];
  }
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return "";
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  } catch {
    return "";
  }
}

export function extractTimeInput(isoString: string | null, defaultTime = "12:00"): string {
  if (!isoString) return defaultTime;
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return defaultTime;
    const hours = String(d.getHours()).padStart(2, "0");
    const minutes = String(d.getMinutes()).padStart(2, "0");
    return `${hours}:${minutes}`;
  } catch {
    return defaultTime;
  }
}

export function buildIsoFromDateTime(dateStr: string, timeStr: string): string {
  if (!dateStr) return "";
  if (!timeStr) timeStr = "00:00";
  const [hours, minutes] = timeStr.split(":").map(Number);
  const [year, month, day] = dateStr.split("-").map(Number);

  const d = new Date(year, month - 1, day, hours || 0, minutes || 0, 0);
  return d.toISOString();
}

/**
 * Generates an RFC 5545 iCalendar (.ics) string from EventDetails
 */
export function generateIcsCalendarContent(event: EventDetails): string {
  const now = new Date();
  const dtStamp = formatIcsDate(now);

  let dtStart = "";
  let dtEnd = "";

  if (event.is_all_day) {
    const startDate = event.start_time ? new Date(event.start_time) : now;
    const startStr = formatIcsDateOnly(startDate);
    dtStart = `DTSTART;VALUE=DATE:${startStr}`;

    const endDate = event.end_time ? new Date(event.end_time) : new Date(startDate.getTime() + 86400000);
    const endStr = formatIcsDateOnly(endDate);
    dtEnd = `DTEND;VALUE=DATE:${endStr}`;
  } else {
    const startDate = event.start_time ? new Date(event.start_time) : now;
    const endDate = event.end_time ? new Date(event.end_time) : new Date(startDate.getTime() + 3600000);
    dtStart = `DTSTART:${formatIcsDate(startDate)}`;
    dtEnd = `DTEND:${formatIcsDate(endDate)}`;
  }

  const uid = `share2cal-${Date.now()}-${Math.random().toString(36).substring(2, 9)}@share2cal.app`;
  const summary = escapeIcsText(event.title || "New Event");
  const location = event.location ? `LOCATION:${escapeIcsText(event.location)}\r\n` : "";
  const description = event.description ? `DESCRIPTION:${escapeIcsText(event.description)}\r\n` : "";

  return [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//Share2Cal//Event Parser//EN",
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
    "BEGIN:VEVENT",
    `UID:${uid}`,
    `DTSTAMP:${dtStamp}`,
    dtStart,
    dtEnd,
    `SUMMARY:${summary}`,
    location ? location.trimEnd() : null,
    description ? description.trimEnd() : null,
    "STATUS:CONFIRMED",
    "END:VEVENT",
    "END:VCALENDAR",
  ]
    .filter(Boolean)
    .join("\r\n");
}

function formatIcsDate(d: Date): string {
  return d
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}/, "");
}

function formatIcsDateOnly(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}${m}${day}`;
}

function escapeIcsText(text: string): string {
  return text
    .replace(/\\/g, "\\\\")
    .replace(/;/g, "\\;")
    .replace(/,/g, "\\,")
    .replace(/\n/g, "\\n");
}

/**
 * Downloads the .ics calendar file to the user's device / triggers calendar import
 */
export function downloadIcsFile(event: EventDetails, filename?: string): void {
  const icsContent = generateIcsCalendarContent(event);
  const blob = new Blob([icsContent], { type: "text/calendar;charset=utf-8" });
  const url = URL.createObjectURL(blob);

  const cleanTitle = (event.title || "event")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

  const safeFilename = filename || `${cleanTitle || "event"}.ics`;

  const link = document.createElement("a");
  link.href = url;
  link.setAttribute("download", safeFilename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
