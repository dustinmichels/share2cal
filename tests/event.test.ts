import { describe, it, expect } from "bun:test";
import {
  generateIcsCalendarContent,
  generateMultiIcsCalendarContent,
  formatDateForDisplay,
  formatTimeForDisplay,
  formatRecurrenceForDisplay,
  parseRecurrenceRule,
  buildRecurrenceRule,
  extractDateInput,
  extractTimeInput,
  buildIsoFromDateTime,
  type EventDetails,
} from "../src/services/event";

describe("Event Service & Multi-Event ICS Generation", () => {
  const sampleEvents: EventDetails[] = [
    {
      title: "CEE 0154-03 Principles Epidemiology (Lecture)",
      start_time: "2026-09-07T15:00:00-04:00",
      end_time: "2026-09-07T16:15:00-04:00",
      is_all_day: false,
      location: "Anderson Wing TTC, Room 306",
      description: "Days: Mo, We | Faculty: L. Abrams | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
    {
      title: "CS 0150-09 Special Topics - Analysis Mthds Images, Text & (Lecture)",
      start_time: "2026-09-11T14:00:00-04:00",
      end_time: "2026-09-11T16:30:00-04:00",
      is_all_day: false,
      location: "Online",
      description: "Days: Fr | Faculty: J. Skripchuk | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=FR;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
    {
      title: "CSHD 0166-01 Children's Play (Lecture)",
      start_time: "2026-09-10T13:30:00-04:00",
      end_time: "2026-09-10T16:00:00-04:00",
      is_all_day: false,
      location: "Eliot-Pearson, Room 157",
      description: "Days: Th | Faculty: W. Scarlett | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=TH;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
    {
      title: "CSHD 0167-01 Children & Media (Lecture)",
      start_time: "2026-09-11T09:00:00-04:00",
      end_time: "2026-09-11T11:30:00-04:00",
      is_all_day: false,
      location: "Eaton Hall, 201",
      description: "Days: Fr | Faculty: J. Dobrow | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=FR;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
    {
      title: "UEP 0254-01 Quantitative Reasoning (Lecture)",
      start_time: "2026-09-08T09:00:00-04:00",
      end_time: "2026-09-08T10:15:00-04:00",
      is_all_day: false,
      location: "Joyce Cummings Center, 302",
      description: "Days: Tu, Th | Faculty: S. Shamsuddin | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
    {
      title: "UEP 0262-01 Solidarity Economy Movements (Seminar)",
      start_time: "2026-09-08T12:00:00-04:00",
      end_time: "2026-09-08T14:30:00-04:00",
      is_all_day: false,
      location: "Bromfield-Pearson, Room 006",
      description: "Days: Tu | Faculty: P. Loh | Units: 3.00",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=TU;UNTIL=20261218T235959Z",
      confidence: 0.95,
      source: "deterministic_schedule",
    },
  ];

  it("generates a multi-event RFC 5545 iCalendar (.ics) string with 6 distinct VEVENT blocks", () => {
    const icsContent = generateMultiIcsCalendarContent(sampleEvents);

    expect(icsContent).toStartWith("BEGIN:VCALENDAR");
    expect(icsContent).toEndWith("END:VCALENDAR");

    // Count occurrences of BEGIN:VEVENT and END:VEVENT
    const beginMatches = icsContent.match(/BEGIN:VEVENT/g);
    const endMatches = icsContent.match(/END:VEVENT/g);

    expect(beginMatches).not.toBeNull();
    expect(beginMatches!.length).toBe(6);
    expect(endMatches).not.toBeNull();
    expect(endMatches!.length).toBe(6);

    // Verify presence of event titles
    expect(icsContent).toContain("SUMMARY:CEE 0154-03 Principles Epidemiology (Lecture)");
    expect(icsContent).toContain(
      "SUMMARY:CS 0150-09 Special Topics - Analysis Mthds Images\\, Text & (Lecture)",
    );
    expect(icsContent).toContain("SUMMARY:CSHD 0166-01 Children's Play (Lecture)");
    expect(icsContent).toContain("SUMMARY:CSHD 0167-01 Children & Media (Lecture)");
    expect(icsContent).toContain("SUMMARY:UEP 0254-01 Quantitative Reasoning (Lecture)");
    expect(icsContent).toContain("SUMMARY:UEP 0262-01 Solidarity Economy Movements (Seminar)");

    // Verify locations and descriptions
    expect(icsContent).toContain("LOCATION:Anderson Wing TTC\\, Room 306");
    expect(icsContent).toContain("LOCATION:Online");
    expect(icsContent).toContain("DESCRIPTION:Days: Mo\\, We | Faculty: L. Abrams | Units: 3.00");

    // Verify RRULE presence in multi-event ICS
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TH;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU;UNTIL=20261218T235959Z");
  });
  it("generates single event ICS correctly", () => {
    const single = sampleEvents[0];
    const ics = generateIcsCalendarContent(single);
    expect(ics).toStartWith("BEGIN:VCALENDAR");
    expect(ics).toEndWith("END:VCALENDAR");
    expect(ics).toContain("SUMMARY:CEE 0154-03 Principles Epidemiology (Lecture)");
  });

  it("formats dates and times for display accurately", () => {
    const isoString = "2026-09-11T14:00:00Z";
    const formattedDate = formatDateForDisplay(isoString);
    expect(formattedDate).toContain("2026");
    expect(formattedDate).toContain("Sep");

    const formattedTime = formatTimeForDisplay(isoString);
    expect(formattedTime).not.toBe("");

    expect(formatDateForDisplay(null)).toBe("Date not specified");
    expect(formatTimeForDisplay(null)).toBe("");
  });

  it("extracts date and time input values for editing form", () => {
    const isoString = "2026-09-11T14:30:00Z";
    const dateInput = extractDateInput(isoString);
    expect(dateInput.length).toBe(10); // YYYY-MM-DD format

    const timeInput = extractTimeInput(isoString);
    expect(timeInput).toMatch(/^\d{2}:\d{2}$/);
  });

  it("builds ISO string from separate date and time input strings", () => {
    const iso = buildIsoFromDateTime("2026-09-11", "14:30");
    expect(iso).toContain("2026-09-11");
  });

  it("formats recurrence rules into human-readable text", () => {
    expect(formatRecurrenceForDisplay("FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z")).toBe(
      "Repeats: Mon, Wed until Dec 18, 2026",
    );
    expect(formatRecurrenceForDisplay("FREQ=WEEKLY;BYDAY=TU,TH")).toBe("Repeats: Tue, Thu");
    expect(formatRecurrenceForDisplay("FREQ=DAILY")).toBe("Repeats daily");
    expect(formatRecurrenceForDisplay(null)).toBeNull();
  });

  it("parses and builds typed recurrence rules with end-of-day inclusive UNTIL", () => {
    const parsedFromPicker = parseRecurrenceRule(
      "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;UNTIL=2026-12-18",
    );
    expect(parsedFromPicker).not.toBeNull();
    expect(parsedFromPicker!.frequency).toBe("WEEKLY");
    expect(parsedFromPicker!.interval).toBe(2);
    expect(parsedFromPicker!.byDays).toEqual(["MO", "WE"]);
    expect(parsedFromPicker!.until).toBe("2026-12-18");

    // Date picker input "2026-12-18" is serialized as end-of-day DATE-TIME "20261218T235959" to match DTSTART type and include end-date afternoon classes
    const rebuilt = buildRecurrenceRule(parsedFromPicker);
    expect(rebuilt).toBe("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;UNTIL=20261218T235959");
  });

  it("emits floating local DTSTART and DTEND for recurring events to preserve wall-clock time across DST transitions", () => {
    const recurringClassOffset: EventDetails = {
      title: "CEE 0154-03 Principles Epidemiology (Lecture)",
      start_time: "2026-09-07T15:00:00-04:00",
      end_time: "2026-09-07T16:15:00-04:00",
      is_all_day: false,
      location: "Anderson Wing TTC, Room 306",
      description: "Days: Mo, We",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959",
      confidence: 0.95,
      source: "deterministic_schedule",
    };

    const icsOffset = generateIcsCalendarContent(recurringClassOffset);
    // Floating datetime format YYYYMMDDTHHMMSS without 'Z' preserves local 3:00 PM across November DST boundary
    expect(icsOffset).toContain("DTSTART:20260907T150000");
    expect(icsOffset).toContain("DTEND:20260907T161500");
    expect(icsOffset).toContain("RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959");

    const formIsoStart = buildIsoFromDateTime("2026-09-07", "15:00");
    const formIsoEnd = buildIsoFromDateTime("2026-09-07", "16:15");

    const recurringClassFromForm: EventDetails = {
      title: "CEE 0154-03 Principles Epidemiology (Lecture)",
      start_time: formIsoStart,
      end_time: formIsoEnd,
      is_all_day: false,
      location: "Anderson Wing TTC, Room 306",
      description: "Days: Mo, We",
      recurrence_rule: "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959",
      confidence: 0.95,
      source: "form",
    };

    const icsForm = generateIcsCalendarContent(recurringClassFromForm);
    expect(icsForm).toContain("DTSTART:20260907T150000");
    expect(icsForm).toContain("DTEND:20260907T161500");
  });
});
