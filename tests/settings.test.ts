import { describe, it, expect, beforeEach } from "bun:test";
import {
  getStoredParsingMode,
  setStoredParsingMode,
  hasStoredParsingModePreference,
  getStoredDefaultTarget,
  setStoredDefaultTarget,
  getStoredDefaultCalendarId,
  setStoredDefaultCalendarId,
  clearStoredSettings,
} from "../src/services/settings";

describe("Settings Service", () => {
  beforeEach(() => {
    clearStoredSettings();
  });

  it("defaults parsing mode to 'enhanced' when not previously set", () => {
    expect(hasStoredParsingModePreference()).toBe(false);
    expect(getStoredParsingMode()).toBe("enhanced");
  });

  it("persists and retrieves 'simple' parsing mode", () => {
    setStoredParsingMode("simple");
    expect(localStorage.getItem("share2cal_parsing_mode")).toBe("simple");
    expect(getStoredParsingMode()).toBe("simple");
    expect(hasStoredParsingModePreference()).toBe(true);
  });

  it("persists and retrieves 'enhanced' parsing mode", () => {
    setStoredParsingMode("enhanced");
    expect(localStorage.getItem("share2cal_parsing_mode")).toBe("enhanced");
    expect(getStoredParsingMode()).toBe("enhanced");
    expect(hasStoredParsingModePreference()).toBe(true);
  });

  it("handles default target and calendar id settings", () => {
    expect(getStoredDefaultTarget()).toBe("native");
    setStoredDefaultTarget("google");
    expect(getStoredDefaultTarget()).toBe("google");

    expect(getStoredDefaultCalendarId()).toBeNull();
    setStoredDefaultCalendarId("work-cal-123");
    expect(getStoredDefaultCalendarId()).toBe("work-cal-123");
    setStoredDefaultCalendarId(null);
    expect(getStoredDefaultCalendarId()).toBeNull();
  });
});
