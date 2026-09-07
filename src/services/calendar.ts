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

function getErrorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err && typeof (err as { message: unknown }).message === "string") {
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
 * High-level helper to add an event to the native calendar.
 * Automatically checks and requests permission if needed, then saves the event.
 */
export async function addEventToNativeCalendar(event: EventDetails): Promise<CalendarResult> {
  try {
    const status = await checkCalendarPermission();

    if (status === "denied" || status === "restricted") {
      return {
        success: false,
        error: "Calendar access is denied. Please enable Calendar access for Share2Cal in your device Settings.",
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
