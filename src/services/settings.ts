export type ParsingMode = "simple" | "enhanced";
export type CalendarTarget = "native" | "google" | "ics";

export interface StoredCalendarPreference {
  id: string;
  title: string;
  sourceTitle: string;
}

const PARSING_MODE_KEY = "share2cal_parsing_mode";
const DEFAULT_CALENDAR_KEY = "share2cal_default_calendar_id";
const CALENDAR_PREFERENCE_KEY = "share2cal_calendar_preference";
const DEFAULT_TARGET_KEY = "share2cal_default_calendar_target";

/**
 * Retrieves the persisted parsing mode preference, defaulting to 'simple'.
 */
export function getStoredParsingMode(): ParsingMode {
  try {
    const stored = localStorage.getItem(PARSING_MODE_KEY);
    if (stored === "enhanced" || stored === "simple") {
      return stored;
    }
  } catch (err) {
    console.warn("Failed to read parsing mode from localStorage:", err);
  }
  return "simple";
}

/**
 * Persists the chosen parsing mode preference.
 */
export function setStoredParsingMode(mode: ParsingMode): void {
  try {
    localStorage.setItem(PARSING_MODE_KEY, mode);
  } catch (err) {
    console.warn("Failed to persist parsing mode to localStorage:", err);
  }
}

/**
 * Retrieves the persisted default calendar identifier.
 */
export function getStoredDefaultCalendarId(): string | null {
  try {
    return localStorage.getItem(DEFAULT_CALENDAR_KEY);
  } catch (err) {
    console.warn("Failed to read default calendar from localStorage:", err);
    return null;
  }
}

/**
 * Persists the default calendar identifier.
 */
export function setStoredDefaultCalendarId(calendarId: string | null): void {
  try {
    if (calendarId) {
      localStorage.setItem(DEFAULT_CALENDAR_KEY, calendarId);
    } else {
      localStorage.removeItem(DEFAULT_CALENDAR_KEY);
    }
  } catch (err) {
    console.warn("Failed to persist default calendar to localStorage:", err);
  }
}

/**
 * Retrieves the persisted structured calendar preference (id, title, sourceTitle).
 */
export function getStoredCalendarPreference(): StoredCalendarPreference | null {
  try {
    const raw = localStorage.getItem(CALENDAR_PREFERENCE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.id === "string" && typeof parsed.title === "string") {
        return {
          id: parsed.id,
          title: parsed.title,
          sourceTitle: typeof parsed.sourceTitle === "string" ? parsed.sourceTitle : "",
        };
      }
    }
  } catch (err) {
    console.warn("Failed to read calendar preference from localStorage:", err);
  }
  return null;
}

/**
 * Persists the structured calendar preference.
 */
export function setStoredCalendarPreference(pref: StoredCalendarPreference | null): void {
  try {
    if (pref) {
      localStorage.setItem(CALENDAR_PREFERENCE_KEY, JSON.stringify(pref));
      setStoredDefaultCalendarId(pref.id);
    } else {
      localStorage.removeItem(CALENDAR_PREFERENCE_KEY);
      setStoredDefaultCalendarId(null);
    }
  } catch (err) {
    console.warn("Failed to persist calendar preference to localStorage:", err);
  }
}

/**
 * Retrieves the preferred default calendar action/target ('native', 'google', or 'ics').
 */
export function getStoredDefaultTarget(): CalendarTarget {
  try {
    const stored = localStorage.getItem(DEFAULT_TARGET_KEY);
    if (stored === "native" || stored === "google" || stored === "ics") {
      return stored;
    }
  } catch (err) {
    console.warn("Failed to read default calendar target from localStorage:", err);
  }
  return "native";
}

/**
 * Persists the default calendar action/target.
 */
export function setStoredDefaultTarget(target: CalendarTarget): void {
  try {
    localStorage.setItem(DEFAULT_TARGET_KEY, target);
  } catch (err) {
    console.warn("Failed to persist default calendar target to localStorage:", err);
  }
}
