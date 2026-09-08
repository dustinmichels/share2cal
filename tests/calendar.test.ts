import { describe, it, expect, beforeEach } from "bun:test";
import {
  formatGoogleCalendarDate,
  getGoogleCalendarEndTimestamp,
  generateGoogleCalendarUrl,
} from "../src/services/calendar";
import {
  getStoredDefaultCalendarId,
  setStoredDefaultCalendarId,
  getStoredCalendarPreference,
  setStoredCalendarPreference,
  getStoredDefaultTarget,
  setStoredDefaultTarget,
  type StoredCalendarPreference,
} from "../src/services/settings";
import type { EventDetails } from "../src/services/event";

// Polyfill localStorage in test runtime if missing
if (typeof globalThis.localStorage === "undefined") {
  const store = new Map<string, string>();
  globalThis.localStorage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, String(value)),
    removeItem: (key: string) => store.delete(key),
    clear: () => store.clear(),
    key: (index: number) => Array.from(store.keys())[index] ?? null,
    get length() {
      return store.size;
    },
  };
}

describe("Calendar Service & Google Calendar Integration", () => {
  const sampleEvent: EventDetails = {
    title: "Live Jazz Night & Jam Session",
    start_time: "2026-09-12T19:00:00Z",
    end_time: "2026-09-12T22:30:00Z",
    is_all_day: false,
    location: "Blue Note Jazz Club, NYC",
    description: "Featuring guest artists and student showcase.\nCover: $15.",
    recurrence_rule: "FREQ=WEEKLY;BYDAY=SA;UNTIL=20261231T235959Z",
    confidence: 0.98,
    source: "flyer_scan",
  };

  const allDayEvent: EventDetails = {
    title: "Fall Hackathon 2026",
    start_time: "2026-10-15",
    end_time: "2026-10-17",
    is_all_day: true,
    location: "Student Center Hall",
    description: "48-hour hackathon competition.",
    confidence: 0.95,
    source: "flyer_scan",
  };

  describe("formatGoogleCalendarDate", () => {
    it("formats ISO timestamps to compact UTC format", () => {
      const formatted = formatGoogleCalendarDate("2026-09-12T19:00:00Z", false);
      expect(formatted).toBe("20260912T190000Z");
    });

    it("formats date-only strings for all-day events to YYYYMMDD", () => {
      const formatted = formatGoogleCalendarDate("2026-10-15", true);
      expect(formatted).toBe("20261015");
    });

    it("handles null or empty dates gracefully by providing a valid timestamp", () => {
      const formatted = formatGoogleCalendarDate(null, false);
      expect(formatted).toMatch(/^\d{8}T\d{6}Z$/);

      const allDayFormatted = formatGoogleCalendarDate("", true);
      expect(allDayFormatted).toMatch(/^\d{8}$/);
    });
  });

  describe("getGoogleCalendarEndTimestamp", () => {
    it("returns formatted end timestamp when provided", () => {
      const end = getGoogleCalendarEndTimestamp(
        "2026-09-12T19:00:00Z",
        "2026-09-12T22:30:00Z",
        false,
      );
      expect(end).toBe("20260912T223000Z");
    });

    it("calculates +1 hour duration when end_time is missing for timed events", () => {
      const end = getGoogleCalendarEndTimestamp("2026-09-12T19:00:00Z", null, false);
      expect(end).toBe("20260912T200000Z");
    });

    it("calculates next day (+24h) for all-day events when end_time is same as start", () => {
      const end = getGoogleCalendarEndTimestamp("2026-10-15", "2026-10-15", true);
      expect(end).toBe("20261016");
    });

    it("calculates exclusive end timestamp (+1 day from inclusive end) for multi-day all-day events", () => {
      const end = getGoogleCalendarEndTimestamp("2026-10-15", "2026-10-17", true);
      expect(end).toBe("20261018");
    });
  });
  describe("generateGoogleCalendarUrl", () => {
    it("generates a valid Google Calendar template URL with all parameters", () => {
      const urlString = generateGoogleCalendarUrl(sampleEvent);
      expect(urlString.startsWith("https://calendar.google.com/calendar/render?")).toBe(true);

      const url = new URL(urlString);
      expect(url.searchParams.get("action")).toBe("TEMPLATE");
      expect(url.searchParams.get("text")).toBe("Live Jazz Night & Jam Session");
      expect(url.searchParams.get("dates")).toBe("20260912T190000Z/20260912T223000Z");
      expect(url.searchParams.get("location")).toBe("Blue Note Jazz Club, NYC");
      expect(url.searchParams.get("details")).toContain("Featuring guest artists");
      expect(url.searchParams.get("recur")).toBe(
        "RRULE:FREQ=WEEKLY;BYDAY=SA;UNTIL=20261231T235959Z",
      );
    });

    it("generates a valid URL for all-day events", () => {
      const urlString = generateGoogleCalendarUrl(allDayEvent);
      const url = new URL(urlString);
      expect(url.searchParams.get("text")).toBe("Fall Hackathon 2026");
      expect(url.searchParams.get("dates")).toBe("20261015/20261018");
      expect(url.searchParams.get("location")).toBe("Student Center Hall");
    });

    it("correctly adds RRULE prefix if missing from recurrence rule", () => {
      const eventWithRawRrule: EventDetails = {
        title: "Weekly Standup",
        start_time: "2026-09-08T09:00:00Z",
        end_time: "2026-09-08T09:30:00Z",
        is_all_day: false,
        location: null,
        description: null,
        recurrence_rule: "FREQ=WEEKLY;BYDAY=TU",
        confidence: 0.9,
        source: "test",
      };

      const urlString = generateGoogleCalendarUrl(eventWithRawRrule);
      const url = new URL(urlString);
      expect(url.searchParams.get("recur")).toBe("RRULE:FREQ=WEEKLY;BYDAY=TU");
    });

    it("generates a valid Google Calendar URL for UEP Ice Cream Social Instagram sample", () => {
      const event: EventDetails = {
        title: "UEP ICE CREAM SOCIAL",
        start_time: "2026-09-09T12:00:00-04:00",
        end_time: "2026-09-09T13:00:00-04:00",
        is_all_day: false,
        location: "BP LAWN",
        description:
          "We invite you to have dessert with us. Don't like ice cream? We have iced coffee, iced tea, and fruit too!",
        confidence: 0.95,
        source: "instagram_sample",
      };
      const urlString = generateGoogleCalendarUrl(event);
      const url = new URL(urlString);
      expect(url.searchParams.get("text")).toBe("UEP ICE CREAM SOCIAL");
      expect(url.searchParams.get("dates")).toBe("20260909T160000Z/20260909T170000Z");
      expect(url.searchParams.get("location")).toBe("BP LAWN");
      expect(url.searchParams.get("details")).toContain("We invite you to have dessert with us");
    });

    it("generates a valid Google Calendar URL for Squirrel Flower concert tour sample", () => {
      const event: EventDetails = {
        title: "Squirrel Flower (with You Bet)",
        start_time: "2026-09-26",
        end_time: "2026-09-26",
        is_all_day: true,
        location: "Crystal Ballroom, Somerville, MA",
        description: "2026 Tour",
        confidence: 0.95,
        source: "flyer_scan",
      };
      const urlString = generateGoogleCalendarUrl(event);
      const url = new URL(urlString);
      expect(url.searchParams.get("text")).toBe("Squirrel Flower (with You Bet)");
      expect(url.searchParams.get("dates")).toBe("20260926/20260927");
      expect(url.searchParams.get("location")).toBe("Crystal Ballroom, Somerville, MA");
      expect(url.searchParams.get("details")).toBe("2026 Tour");
    });
  });

  describe("Settings & Calendar Preferences Persistence", () => {
    beforeEach(() => {
      localStorage.clear();
    });
    it("persists and retrieves default calendar ID", () => {
      expect(getStoredDefaultCalendarId()).toBeNull();
      setStoredDefaultCalendarId("cal_work_123");
      expect(getStoredDefaultCalendarId()).toBe("cal_work_123");

      setStoredDefaultCalendarId(null);
      expect(getStoredDefaultCalendarId()).toBeNull();
    });

    it("persists and retrieves structured calendar preference", () => {
      const pref: StoredCalendarPreference = {
        id: "cal_personal_456",
        title: "Personal",
        sourceTitle: "iCloud",
      };

      expect(getStoredCalendarPreference()).toBeNull();
      setStoredCalendarPreference(pref);

      const retrieved = getStoredCalendarPreference();
      expect(retrieved).not.toBeNull();
      expect(retrieved?.id).toBe("cal_personal_456");
      expect(retrieved?.title).toBe("Personal");
      expect(retrieved?.sourceTitle).toBe("iCloud");

      expect(getStoredDefaultCalendarId()).toBe("cal_personal_456");

      setStoredCalendarPreference(null);
      expect(getStoredCalendarPreference()).toBeNull();
      expect(getStoredDefaultCalendarId()).toBeNull();
    });

    it("persists and retrieves default export target preference", () => {
      expect(getStoredDefaultTarget()).toBe("native");

      setStoredDefaultTarget("google");
      expect(getStoredDefaultTarget()).toBe("google");

      setStoredDefaultTarget("ics");
      expect(getStoredDefaultTarget()).toBe("ics");
    });
  });
});
