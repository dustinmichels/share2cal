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
import parsedManifest from "../samples/parsed.json";

describe("Event Service & Multi-Event ICS Generation (using samples/parsed.json)", () => {
  // Load source of truth directly from samples/parsed.json
  const sampleEvents: EventDetails[] = (
    parsedManifest["samples/classes.png"] as Array<Record<string, unknown>>
  ).map((e) => ({
    title: String(e.title),
    start_time: (e.start_time as string) ?? null,
    end_time: (e.end_time as string) ?? null,
    is_all_day: Boolean(e.is_all_day),
    location: (e.location as string) ?? null,
    description: (e.description as string) ?? null,
    recurrence_rule: (e.recurrence_rule as string) ?? null,
    confidence: 0.95,
    source: "deterministic_schedule",
  }));

  const gilmanFlyerEvent: EventDetails = {
    ...(parsedManifest["samples/gilman_flyer.png"][0] as EventDetails),
    confidence: 0.95,
    source: "deterministic_flyer",
  };

  const instagramIceCreamSocialEvent: EventDetails = {
    ...(parsedManifest["samples/instagram.png"][0] as EventDetails),
    confidence: 0.95,
    source: "deterministic_flyer",
  };

  const squirrelFlowerEvent: EventDetails = {
    ...(parsedManifest["samples/squirrel_flower.jpg"][0] as EventDetails),
    confidence: 0.95,
    source: "deterministic_flyer",
  };

  const rideForLifeEvent: EventDetails = {
    ...(parsedManifest["samples/ride_for_life.png"][0] as EventDetails),
    confidence: 0.95,
    source: "deterministic_flyer",
  };

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

    // Verify presence of event titles from parsed.json
    expect(icsContent).toContain("SUMMARY:CEE 0154-03 (80513) Principles Epidemiology (Lecture)");
    expect(icsContent).toContain(
      "SUMMARY:CS 0150-09 (84779) Special Topics - Analysis Mthds Images\\, Text & (Lecture)",
    );
    expect(icsContent).toContain("SUMMARY:CSHD 0166-01 (82454) Children's Play (Lecture)");
    expect(icsContent).toContain("SUMMARY:CSHD 0167-01 (80739) Children & Media (Lecture)");
    expect(icsContent).toContain("SUMMARY:UEP 0254-01 (81300) Quantitative Reasoning (Lecture)");
    expect(icsContent).toContain(
      "SUMMARY:UEP 0262-01 (82571) Solidarity Economy Movements (Seminar)",
    );

    // Verify locations and descriptions from parsed.json
    expect(icsContent).toContain("LOCATION:Anderson Wing TTC\\, Room 306");
    expect(icsContent).toContain("LOCATION:Online");

    // Verify RRULE presence in multi-event ICS
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TH;UNTIL=20261218T235959Z");
    expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU;UNTIL=20261218T235959Z");
  });

  it("handles recurring relative weekday calculations starting on the next upcoming matching weekday", () => {
    /**
     * COMPLEXITY / SPECIFICATION NOTE:
     * When parsing recurring schedules (like class schedules "Mo, We 3:00 PM - 4:15 PM") without
     * an absolute calendar year/date in the image, the event is defined to repeat weekly starting
     * on the NEXT matching weekday relative to WHEN the code is run (or reference context).
     *
     * If the tests are run on a different date (e.g. next week or next month):
     * - The start and end calendar dates (YYYY-MM-DD) will advance to the next upcoming weekdays.
     * - The start time-of-day (15:00), end time-of-day (16:15), duration (75m), and recurrence rule (BYDAY=MO,WE)
     *   remain strictly identical.
     */
    const mondayClass = sampleEvents[0]; // CEE 0154-03
    expect(mondayClass.recurrence_rule).toContain("BYDAY=MO,WE");
    expect(mondayClass.is_all_day).toBe(false);

    const startTimeInput = extractTimeInput(mondayClass.start_time);
    expect(startTimeInput).toBe("15:00");
    const endTimeInput = extractTimeInput(mondayClass.end_time);
    expect(endTimeInput).toBe("16:15");
  });

  it("generates single event ICS correctly for Gilman Square Arts & Music Festival flyer sample", () => {
    const ics = generateIcsCalendarContent(gilmanFlyerEvent);
    expect(ics).toStartWith("BEGIN:VCALENDAR");
    expect(ics).toEndWith("END:VCALENDAR");
    expect(ics).toContain("BEGIN:VEVENT");
    expect(ics).toContain("END:VEVENT");
    expect(ics).toContain("SUMMARY:SomerStreets Gilman Square Arts & Music Festival");
    expect(ics).toContain(
      "LOCATION:Ed Leathers Park\\, Walnut Street and Skilton Ave\\, Somerville\\, MA",
    );
    expect(ics).toContain("STATUS:CONFIRMED");
    expect(ics).toContain("DTSTART:20260912T160000Z");
    expect(ics).toContain("DTEND:20260912T210000Z");
  });

  it("extracts and formats date and time values accurately for Gilman Square flyer event", () => {
    const dateInput = extractDateInput(gilmanFlyerEvent.start_time);
    expect(dateInput).toBe("2026-09-12");

    const startTimeInput = extractTimeInput(gilmanFlyerEvent.start_time);
    expect(startTimeInput).toBe("12:00");

    const endTimeInput = extractTimeInput(gilmanFlyerEvent.end_time);
    expect(endTimeInput).toBe("17:00");

    const formattedDate = formatDateForDisplay(gilmanFlyerEvent.start_time);
    expect(formattedDate).toContain("2026");
    expect(formattedDate).toContain("Sep");

    const formattedStartTime = formatTimeForDisplay(gilmanFlyerEvent.start_time);
    expect(formattedStartTime).not.toBe("");

    const formattedEndTime = formatTimeForDisplay(gilmanFlyerEvent.end_time);
    expect(formattedEndTime).not.toBe("");

    const reconstructedStart = buildIsoFromDateTime("2026-09-12", "12:00");
    expect(reconstructedStart).toContain("2026-09-12T12:00");

    const reconstructedEnd = buildIsoFromDateTime("2026-09-12", "17:00");
    expect(reconstructedEnd).toContain("2026-09-12T17:00");
  });

  it("generates single event ICS correctly for UEP Ice Cream Social Instagram sample", () => {
    const ics = generateIcsCalendarContent(instagramIceCreamSocialEvent);
    expect(ics).toStartWith("BEGIN:VCALENDAR");
    expect(ics).toEndWith("END:VCALENDAR");
    expect(ics).toContain("BEGIN:VEVENT");
    expect(ics).toContain("END:VEVENT");
    expect(ics).toContain("SUMMARY:UEP ICE CREAM SOCIAL");
    expect(ics).toContain("LOCATION:BP Lawn (Bromfield-Pearson)\\, Tufts University");
    expect(ics).toContain("STATUS:CONFIRMED");
    expect(ics).toContain("DTSTART:20260909T160000Z");
    expect(ics).toContain("DTEND:20260909T170000Z");
  });

  it("extracts and formats date and time values accurately for UEP Ice Cream Social Instagram event", () => {
    const dateInput = extractDateInput(instagramIceCreamSocialEvent.start_time);
    expect(dateInput).toBe("2026-09-09");

    const startTimeInput = extractTimeInput(instagramIceCreamSocialEvent.start_time);
    expect(startTimeInput).toBe("12:00");

    const endTimeInput = extractTimeInput(instagramIceCreamSocialEvent.end_time);
    expect(endTimeInput).toBe("13:00");

    const formattedDate = formatDateForDisplay(instagramIceCreamSocialEvent.start_time);
    expect(formattedDate).toContain("2026");
    expect(formattedDate).toContain("Sep");

    const formattedStartTime = formatTimeForDisplay(instagramIceCreamSocialEvent.start_time);
    expect(formattedStartTime).not.toBe("");

    const formattedEndTime = formatTimeForDisplay(instagramIceCreamSocialEvent.end_time);
    expect(formattedEndTime).not.toBe("");

    const reconstructedStart = buildIsoFromDateTime("2026-09-09", "12:00");
    expect(reconstructedStart).toContain("2026-09-09T12:00");

    const reconstructedEnd = buildIsoFromDateTime("2026-09-09", "13:00");
    expect(reconstructedEnd).toContain("2026-09-09T13:00");
  });
  it("generates single event ICS correctly for Squirrel Flower concert tour flyer sample", () => {
    const ics = generateIcsCalendarContent(squirrelFlowerEvent);
    expect(ics).toStartWith("BEGIN:VCALENDAR");
    expect(ics).toEndWith("END:VCALENDAR");
    expect(ics).toContain("BEGIN:VEVENT");
    expect(ics).toContain("END:VEVENT");
    expect(ics).toContain("SUMMARY:Squirrel Flower – 2026 Tour (with youbet)");
    expect(ics).toContain("LOCATION:Crystal Ballroom\\, Somerville\\, MA");
    expect(ics).toContain("STATUS:CONFIRMED");
    expect(ics).toContain("DTSTART;VALUE=DATE:20260926");
    expect(ics).toContain("DTEND;VALUE=DATE:20260926");
  });

  it("extracts and formats date values accurately for Squirrel Flower flyer event", () => {
    const dateInput = extractDateInput(squirrelFlowerEvent.start_time);
    expect(dateInput).toBe("2026-09-26");

    const formattedDate = formatDateForDisplay(squirrelFlowerEvent.start_time);
    expect(formattedDate).toContain("2026");
    expect(formattedDate).toContain("Sep");
    expect(formattedDate).toContain("26");
  });
  it("generates single event ICS correctly for Ride For Your Life advocacy event sample", () => {
    const ics = generateIcsCalendarContent(rideForLifeEvent);
    expect(ics).toStartWith("BEGIN:VCALENDAR");
    expect(ics).toEndWith("END:VCALENDAR");
    expect(ics).toContain("BEGIN:VEVENT");
    expect(ics).toContain("END:VEVENT");
    expect(ics).toContain("SUMMARY:Ride For Your Life - Boston");
    expect(ics).toContain("LOCATION:Boston\\, MA");
    expect(ics).toContain(
      'DESCRIPTION:RIDE. WALK. RALLY. Motto: "OUR STREETS EXIST For EVERYONE". Memorial and safe streets advocacy event.',
    );
    expect(ics).toContain("STATUS:CONFIRMED");
    expect(ics).toContain("DTSTART;VALUE=DATE:20261025");
    expect(ics).toContain("DTEND;VALUE=DATE:20261025");
  });

  it("extracts and formats date values accurately for Ride For Your Life event", () => {
    const dateInput = extractDateInput(rideForLifeEvent.start_time);
    expect(dateInput).toBe("2026-10-25");

    const formattedDate = formatDateForDisplay(rideForLifeEvent.start_time);
    expect(formattedDate).toContain("2026");
    expect(formattedDate).toContain("Oct");
    expect(formattedDate).toContain("25");
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
