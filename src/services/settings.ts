export type ParsingMode = "simple" | "enhanced";

const PARSING_MODE_KEY = "share2cal_parsing_mode";

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
