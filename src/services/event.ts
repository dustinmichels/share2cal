import { invoke } from "@tauri-apps/api/core";

export * from "./calendar";

export interface EventDetails {
  title: string;
  start_time: string | null;
  end_time: string | null;
  is_all_day: boolean;
  location: string | null;
  description: string | null;
  recurrence_rule?: string | null;
  confidence: number;
  source: string;
}

export interface EventFormData {
  title: string;
  date: string;
  startTime: string;
  endTime: string;
  isAllDay: boolean;
  location: string;
  description: string;
  recurrenceRule?: string;
}

export function getCurrentReferenceTime(): {
  referenceTime: string;
  timezoneOffsetMinutes: number;
} {
  const now = new Date();
  const timezoneOffsetMinutes = -now.getTimezoneOffset(); // e.g. -240 for EDT
  return {
    referenceTime: now.toISOString(),
    timezoneOffsetMinutes,
  };
}

export async function parseEventsFromText(
  text: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails[]> {
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails[]>("parse_events_from_text", {
    text,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  });
}

export async function parseEventFromText(
  text: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails> {
  const events = await parseEventsFromText(
    text,
    referenceTime,
    timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  );
  if (events && events.length > 0) {
    return events[0];
  }
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails>("parse_event_from_text", {
    text,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  });
}

export async function extractEventsFromImage(
  path: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails[]> {
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails[]>("extract_events_from_image", {
    path,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  });
}

export async function extractEventFromImage(
  path: string,
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails> {
  const events = await extractEventsFromImage(
    path,
    referenceTime,
    timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  );
  if (events && events.length > 0) {
    return events[0];
  }
  const ref = getCurrentReferenceTime();
  return await invoke<EventDetails>("extract_event_from_image", {
    path,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  });
}

export async function extractEventsFromImageBytes(
  bytes: Uint8Array | number[],
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails[]> {
  const ref = getCurrentReferenceTime();
  const payload = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  return await invoke<EventDetails[]>("extract_events_from_image_bytes", {
    bytes: payload,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  });
}

export async function extractEventFromImageBytes(
  bytes: Uint8Array | number[],
  referenceTime?: string,
  timezoneOffsetMinutes?: number,
  modelId?: string,
  timeoutSecs?: number,
): Promise<EventDetails> {
  const events = await extractEventsFromImageBytes(
    bytes,
    referenceTime,
    timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
  );
  if (events && events.length > 0) {
    return events[0];
  }
  const ref = getCurrentReferenceTime();
  const payload = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  return await invoke<EventDetails>("extract_event_from_image_bytes", {
    bytes: payload,
    referenceTime: referenceTime ?? ref.referenceTime,
    timezoneOffsetMinutes: timezoneOffsetMinutes ?? ref.timezoneOffsetMinutes,
    modelId,
    timeoutSecs,
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
  timezoneOffsetMinutes?: number,
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
    const year = today.getFullYear();
    const month = String(today.getMonth() + 1).padStart(2, "0");
    const day = String(today.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }
  const match = isoString.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (match) {
    return `${match[1]}-${match[2]}-${match[3]}`;
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
  const match = isoString.match(/T(\d{2}):(\d{2})/);
  if (match) {
    return `${match[1]}:${match[2]}`;
  }
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
  const cleanTime = timeStr ? (timeStr.length === 5 ? `${timeStr}:00` : timeStr) : "12:00:00";
  return `${dateStr}T${cleanTime}`;
}

export interface ParsedRecurrence {
  frequency: "DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY";
  interval: number;
  byDays: string[];
  until: string | null;
  count: number | null;
}
export function parseRecurrenceRule(rrule?: string | null): ParsedRecurrence | null {
  if (!rrule) return null;
  const clean = rrule.replace(/^RRULE:/i, "").trim();
  if (!clean) return null;

  let frequency: "DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY" | null = null;
  let interval = 1;
  const byDays: string[] = [];
  let until: string | null = null;
  let count: number | null = null;

  const validDays = new Set(["MO", "TU", "WE", "TH", "FR", "SA", "SU"]);

  for (const part of clean.split(";")) {
    const trimmedPart = part.trim();
    if (!trimmedPart) continue;

    const eqIdx = trimmedPart.indexOf("=");
    if (eqIdx === -1) return null; // Reject malformed (missing '=')

    const key = trimmedPart.slice(0, eqIdx).trim().toUpperCase();
    const val = trimmedPart.slice(eqIdx + 1).trim();
    if (!key || !val) return null; // Reject empty key or value

    if (key === "FREQ") {
      const upper = val.toUpperCase();
      if (upper === "DAILY" || upper === "WEEKLY" || upper === "MONTHLY" || upper === "YEARLY") {
        frequency = upper;
      } else {
        return null; // Reject invalid frequency
      }
    } else if (key === "INTERVAL") {
      if (!/^\d+$/.test(val)) return null; // Reject non-digit like "2oops"
      const num = parseInt(val, 10);
      if (num <= 0) return null;
      interval = num;
    } else if (key === "BYDAY") {
      const tokens = val.toUpperCase().split(",").map((s) => s.trim());
      if (tokens.length === 0) return null;
      for (const t of tokens) {
        if (!validDays.has(t)) return null; // Reject invalid day token
        if (!byDays.includes(t)) byDays.push(t);
      }
    } else if (key === "UNTIL") {
      const cleanUntil = val.replace(/[-:]/g, "");
      if (cleanUntil.length < 8) return null; // Reject malformed UNTIL
      until = val;
    } else if (key === "COUNT") {
      if (!/^\d+$/.test(val)) return null; // Reject non-digit
      const num = parseInt(val, 10);
      if (num <= 0) return null;
      count = num;
    } else {
      return null; // Reject unknown/unsupported properties
    }
  }

  if (!frequency) {
    return null; // Strictly require FREQ
  }

  return { frequency, interval, byDays, until, count };
}
export function buildRecurrenceRule(parsed: ParsedRecurrence | null): string | null {
  if (!parsed) return null;
  const parts: string[] = [`FREQ=${parsed.frequency}`];
  if (parsed.interval > 1) parts.push(`INTERVAL=${parsed.interval}`);
  if (parsed.byDays.length > 0) parts.push(`BYDAY=${parsed.byDays.join(",")}`);
  if (parsed.until) {
    const raw = parsed.until.replace(/[-:]/g, "");
    if (raw.length === 8) {
      parts.push(`UNTIL=${raw}T235959`);
    } else {
      parts.push(`UNTIL=${raw}`);
    }
  } else if (parsed.count) {
    parts.push(`COUNT=${parsed.count}`);
  }
  return parts.join(";");
}

/**
 * Formats an RFC 5545 RRULE string into human-readable text for display
 */
export function formatRecurrenceForDisplay(rrule?: string | null): string | null {
  const parsed = parseRecurrenceRule(rrule);
  if (!parsed) return null;

  const dayMap: Record<string, string> = {
    MO: "Mon",
    TU: "Tue",
    WE: "Wed",
    TH: "Thu",
    FR: "Fri",
    SA: "Sat",
    SU: "Sun",
  };

  const daysStr = parsed.byDays.map((d) => dayMap[d] || d).join(", ");

  let desc = "";
  if (parsed.frequency === "WEEKLY" && daysStr) {
    desc = `Repeats: ${daysStr}`;
  } else if (parsed.frequency === "DAILY") {
    desc = "Repeats daily";
  } else if (parsed.frequency === "WEEKLY") {
    desc = "Repeats weekly";
  } else if (parsed.frequency === "MONTHLY") {
    desc = "Repeats monthly";
  } else {
    desc = `Repeats (${parsed.frequency.toLowerCase()})`;
  }

  if (parsed.until) {
    const match = parsed.until.match(/^(\d{4})-?(\d{2})-?(\d{2})/);
    if (match) {
      const year = match[1];
      const monthIdx = parseInt(match[2], 10) - 1;
      const day = parseInt(match[3], 10);
      const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
      desc += ` until ${monthNames[monthIdx]} ${day}, ${year}`;
    }
  }

  return desc;
}

function formatIcsFloatingDateTime(isoString: string | null, fallbackDate: Date): string {
  if (!isoString) {
    return formatLocalDateToIcsFloating(fallbackDate);
  }

  // 1. If it has a trailing 'Z' (e.g. from buildIsoFromDateTime):
  // Parse with Date and use device-local getters to convert UTC instant back to intended local wall-clock
  if (isoString.endsWith("Z") || isoString.endsWith("z")) {
    const d = new Date(isoString);
    if (!isNaN(d.getTime())) {
      return formatLocalDateToIcsFloating(d);
    }
  }

  // 2. If it has an explicit numeric timezone offset (e.g. "2026-09-07T15:00:00-04:00")
  // or is a naive datetime string (e.g. "2026-09-07T15:00:00"):
  // The clock fields before the offset represent the exact intended local wall-clock time
  const match = isoString.match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?/);
  if (match) {
    const y = match[1];
    const m = match[2];
    const d = match[3];
    const h = match[4];
    const min = match[5];
    const s = match[6] || "00";
    return `${y}${m}${d}T${h}${min}${s}`;
  }

  const d = new Date(isoString);
  if (isNaN(d.getTime())) {
    return formatLocalDateToIcsFloating(fallbackDate);
  }
  return formatLocalDateToIcsFloating(d);
}

function formatLocalDateToIcsFloating(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  const h = String(d.getHours()).padStart(2, "0");
  const min = String(d.getMinutes()).padStart(2, "0");
  const s = String(d.getSeconds()).padStart(2, "0");
  return `${y}${m}${day}T${h}${min}${s}`;
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

    const endDate = event.end_time
      ? new Date(event.end_time)
      : new Date(startDate.getTime() + 86400000);
    const endStr = formatIcsDateOnly(endDate);
    dtEnd = `DTEND;VALUE=DATE:${endStr}`;
  } else if (event.recurrence_rule) {
    // Recurring event: use floating local wall-clock datetime to preserve time across DST transitions
    const startStr = formatIcsFloatingDateTime(event.start_time, now);
    dtStart = `DTSTART:${startStr}`;
    const endFallback = new Date((event.start_time ? new Date(event.start_time).getTime() : now.getTime()) + 3600000);
    const endStr = formatIcsFloatingDateTime(event.end_time, endFallback);
    dtEnd = `DTEND:${endStr}`;
  } else {
    const startDate = event.start_time ? new Date(event.start_time) : now;
    const endDate = event.end_time
      ? new Date(event.end_time)
      : new Date(startDate.getTime() + 3600000);
    dtStart = `DTSTART:${formatIcsDate(startDate)}`;
    dtEnd = `DTEND:${formatIcsDate(endDate)}`;
  }

  const uid = `share2cal-${Date.now()}-${Math.random().toString(36).substring(2, 9)}@share2cal.app`;
  const summary = escapeIcsText(event.title || "New Event");
  const location = event.location ? `LOCATION:${escapeIcsText(event.location)}\r\n` : "";
  const description = event.description
    ? `DESCRIPTION:${escapeIcsText(event.description)}\r\n`
    : "";
  const validatedRecurrence = event.recurrence_rule
    ? buildRecurrenceRule(parseRecurrenceRule(event.recurrence_rule))
    : null;
  const rrule = validatedRecurrence ? `RRULE:${validatedRecurrence}` : null;

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
    rrule,
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

/**
 * Generates an RFC 5545 iCalendar (.ics) string containing multiple events
 */
export function generateMultiIcsCalendarContent(events: EventDetails[]): string {
  const now = new Date();
  const dtStamp = formatIcsDate(now);
  const vevents = events.map((event, index) => {
    let dtStart = "";
    let dtEnd = "";

    if (event.is_all_day) {
      const startDate = event.start_time ? new Date(event.start_time) : now;
      const startStr = formatIcsDateOnly(startDate);
      dtStart = `DTSTART;VALUE=DATE:${startStr}`;

      const endDate = event.end_time
        ? new Date(event.end_time)
        : new Date(startDate.getTime() + 86400000);
      const endStr = formatIcsDateOnly(endDate);
      dtEnd = `DTEND;VALUE=DATE:${endStr}`;
    } else if (event.recurrence_rule) {
      // Recurring event: use floating local wall-clock datetime to preserve time across DST transitions
      const startStr = formatIcsFloatingDateTime(event.start_time, now);
      dtStart = `DTSTART:${startStr}`;
      const endFallback = new Date((event.start_time ? new Date(event.start_time).getTime() : now.getTime()) + 3600000);
      const endStr = formatIcsFloatingDateTime(event.end_time, endFallback);
      dtEnd = `DTEND:${endStr}`;
    } else {
      const startDate = event.start_time ? new Date(event.start_time) : now;
      const endDate = event.end_time
        ? new Date(event.end_time)
        : new Date(startDate.getTime() + 3600000);
      dtStart = `DTSTART:${formatIcsDate(startDate)}`;
      dtEnd = `DTEND:${formatIcsDate(endDate)}`;
    }

    const uid = `share2cal-${Date.now()}-${index}-${Math.random().toString(36).substring(2, 9)}@share2cal.app`;
    const summary = escapeIcsText(event.title || "New Event");
    const location = event.location ? `LOCATION:${escapeIcsText(event.location)}\r\n` : "";
    const description = event.description
      ? `DESCRIPTION:${escapeIcsText(event.description)}\r\n`
      : "";
    const validatedRecurrence = event.recurrence_rule
      ? buildRecurrenceRule(parseRecurrenceRule(event.recurrence_rule))
      : null;
    const rrule = validatedRecurrence ? `RRULE:${validatedRecurrence}` : null;

    return [
      "BEGIN:VEVENT",
      `UID:${uid}`,
      `DTSTAMP:${dtStamp}`,
      dtStart,
      dtEnd,
      rrule,
      `SUMMARY:${summary}`,
      location ? location.trimEnd() : null,
      description ? description.trimEnd() : null,
      "STATUS:CONFIRMED",
      "END:VEVENT",
    ]
      .filter(Boolean)
      .join("\r\n");
  });

  return [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//Share2Cal//Event Parser//EN",
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
    ...vevents,
    "END:VCALENDAR",
  ].join("\r\n");
}

/**
 * Downloads a combined .ics calendar file containing all events in the list
 */
export function downloadMultiIcsFile(events: EventDetails[], filename?: string): void {
  if (!events || events.length === 0) return;
  if (events.length === 1) {
    downloadIcsFile(events[0], filename);
    return;
  }

  const icsContent = generateMultiIcsCalendarContent(events);
  const blob = new Blob([icsContent], { type: "text/calendar;charset=utf-8" });
  const url = URL.createObjectURL(blob);

  const safeFilename = filename || `events-schedule-${events.length}.ics`;

  const link = document.createElement("a");
  link.href = url;
  link.setAttribute("download", safeFilename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
