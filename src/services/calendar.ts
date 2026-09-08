import { invoke } from "@tauri-apps/api/core";
import type { EventDetails } from "./event";

export type CalendarPermissionStatus =
  | "not_determined"
  | "restricted"
  | "denied"
  | "authorized"
  | "write_only"
  | "unknown";

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
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (
    err &&
    typeof err === "object" &&
    "message" in err &&
    typeof (err as { message: unknown }).message === "string"
  ) {
    return (err as { message: string }).message;
  }
  return "Unknown calendar error";
}

/**
 * Checks the current calendar permission status on the device.
 */
export async function checkCalendarPermission(): Promise<CalendarPermissionStatus> {
  try {
    const status = await invoke<string>("check_calendar_permission");
    return status as CalendarPermissionStatus;
  } catch (err: unknown) {
    console.error("Failed to check calendar permission:", err);
    return "unknown";
  }
}

/**
 * Prompts the user for calendar write permission.
 */
export async function requestCalendarPermission(): Promise<boolean> {
  try {
    const granted = await invoke<boolean>("request_calendar_permission");
    return Boolean(granted);
  } catch (err: unknown) {
    console.error("Failed to request calendar permission:", err);
    return false;
  }
}

/**
 * Creates an event in the native device calendar via EventKit.
 * Returns the created event identifier.
 */
export async function createCalendarEvent(event: EventDetails): Promise<string> {
  return await invoke<string>("create_calendar_event", { event });
}

/**
 * Creates multiple events in the native device calendar.
 */
export async function createCalendarEvents(events: EventDetails[]): Promise<string[]> {
  return await invoke<string[]>("create_calendar_events", { events });
}

/**
 * High-level helper to add an event to the native calendar.
 * Automatically checks and requests permission if needed, then saves the event.
 */
export async function addEventToNativeCalendar(event: EventDetails): Promise<CalendarResult> {
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
        return {
          success: false,
          error: "Calendar access was not granted.",
        };
      }
    }

    const eventId = await createCalendarEvent(event);
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
        return {
          success: false,
          total: events.length,
          addedCount: 0,
          eventIds: [],
          errors: ["Calendar access was not granted."],
        };
      }
    }

    const eventIds: string[] = [];
    const errors: string[] = [];

    for (const event of events) {
      try {
        const id = await createCalendarEvent(event);
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
