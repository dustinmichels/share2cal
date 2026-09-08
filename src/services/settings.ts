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

const memoryStore = new Map<string, string>();

function getStorageItem(key: string): string | null {
  try {
    if (typeof localStorage !== "undefined") {
      return localStorage.getItem(key);
    }
  } catch {
    // fallback
  }
  return memoryStore.get(key) ?? null;
}

function setStorageItem(key: string, value: string): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(key, value);
    }
  } catch {
    // fallback
  }
  memoryStore.set(key, value);
}

function removeStorageItem(key: string): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(key);
    }
  } catch {
    // fallback
  }
  memoryStore.delete(key);
}

export function clearStoredSettings(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(PARSING_MODE_KEY);
      localStorage.removeItem(DEFAULT_CALENDAR_KEY);
      localStorage.removeItem(CALENDAR_PREFERENCE_KEY);
      localStorage.removeItem(DEFAULT_TARGET_KEY);
    }
  } catch {
    // fallback
  }
  memoryStore.clear();
}
/**
 * Retrieves the persisted parsing mode preference, defaulting to 'enhanced'.
 */
export function getStoredParsingMode(): ParsingMode {
  const stored = getStorageItem(PARSING_MODE_KEY);
  if (stored === "enhanced" || stored === "simple") {
    return stored;
  }
  return "enhanced";
}

/**
 * Returns whether a parsing mode preference has been explicitly set in localStorage.
 */
export function hasStoredParsingModePreference(): boolean {
  const stored = getStorageItem(PARSING_MODE_KEY);
  return stored === "enhanced" || stored === "simple";
}

/**
 * Persists the chosen parsing mode preference.
 */
export function setStoredParsingMode(mode: ParsingMode): void {
  setStorageItem(PARSING_MODE_KEY, mode);
}

/**
 * Retrieves the persisted default calendar identifier.
 */
export function getStoredDefaultCalendarId(): string | null {
  return getStorageItem(DEFAULT_CALENDAR_KEY);
}

/**
 * Persists the default calendar identifier.
 */
export function setStoredDefaultCalendarId(calendarId: string | null): void {
  if (calendarId) {
    setStorageItem(DEFAULT_CALENDAR_KEY, calendarId);
  } else {
    removeStorageItem(DEFAULT_CALENDAR_KEY);
  }
}

/**
 * Retrieves the persisted structured calendar preference (id, title, sourceTitle).
 */
export function getStoredCalendarPreference(): StoredCalendarPreference | null {
  const raw = getStorageItem(CALENDAR_PREFERENCE_KEY);
  if (raw) {
    try {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.id === "string" && typeof parsed.title === "string") {
        return {
          id: parsed.id,
          title: parsed.title,
          sourceTitle: typeof parsed.sourceTitle === "string" ? parsed.sourceTitle : "",
        };
      }
    } catch {
      // ignore parse error
    }
  }
  return null;
}

/**
 * Persists the structured calendar preference.
 */
export function setStoredCalendarPreference(pref: StoredCalendarPreference | null): void {
  if (pref) {
    setStorageItem(CALENDAR_PREFERENCE_KEY, JSON.stringify(pref));
    setStoredDefaultCalendarId(pref.id);
  } else {
    removeStorageItem(CALENDAR_PREFERENCE_KEY);
    setStoredDefaultCalendarId(null);
  }
}

/**
 * Retrieves the preferred default calendar action/target ('native', 'google', or 'ics').
 */
export function getStoredDefaultTarget(): CalendarTarget {
  const stored = getStorageItem(DEFAULT_TARGET_KEY);
  if (stored === "native" || stored === "google" || stored === "ics") {
    return stored;
  }
  return "native";
}

/**
 * Persists the default calendar action/target.
 */
export function setStoredDefaultTarget(target: CalendarTarget): void {
  setStorageItem(DEFAULT_TARGET_KEY, target);
}
