import { describe, it, expect } from "bun:test";
import {
  parseEventsFromText,
  parseEventsProgressive,
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
  type EventFormData,
} from "../src/services/event";
import { isWebLink, extractUrlsFromText } from "../src/services/ocr";
import parsedManifest from "../samples/parsed.json";

export interface ParsedJsonEntry {
  title: string;
  date: string | null;
  days: string[] | null;
  start_time: string | null;
  end_time: string | null;
  is_all_day: boolean;
  repeating?: boolean;
  location: string | null;
  description: string | null;
  url?: string | null;
}

function parseTimeTo24h(timeStr: string | null | undefined): string | null {
  if (!timeStr) return null;
  const match = timeStr.trim().match(/^(\d{1,2}):(\d{2})\s*(AM|PM)?$/i);
  if (!match) return null;
  let hour = parseInt(match[1], 10);
  const minute = match[2];
  const ampm = match[3]?.toUpperCase();
  if (ampm === "PM" && hour < 12) hour += 12;
  else if (ampm === "AM" && hour === 12) hour = 0;
  return `${String(hour).padStart(2, "0")}:${minute}`;
}

function convertDayNamesToByDay(days: string[] | null | undefined): string[] {
  if (!days || days.length === 0) return [];
  const map: Record<string, string> = {
    monday: "MO",
    mon: "MO",
    mo: "MO",
    tuesday: "TU",
    tue: "TU",
    tu: "TU",
    wednesday: "WE",
    wed: "WE",
    we: "WE",
    thursday: "TH",
    thu: "TH",
    th: "TH",
    friday: "FR",
    fri: "FR",
    fr: "FR",
    saturday: "SA",
    sat: "SA",
    sa: "SA",
    sunday: "SU",
    sun: "SU",
    su: "SU",
  };
  return days.map((d) => map[d.toLowerCase()] || d).filter(Boolean);
}

function getUpcomingWeekdayDate(dayName: string, refDateStr = "2026-09-06"): string {
  const dayMap: Record<string, number> = {
    sunday: 0,
    su: 0,
    monday: 1,
    mo: 1,
    tuesday: 2,
    tu: 2,
    wednesday: 3,
    we: 3,
    thursday: 4,
    th: 4,
    friday: 5,
    fr: 5,
    saturday: 6,
    sa: 6,
  };
  const targetDay = dayMap[dayName.toLowerCase()] ?? 1;
  const d = new Date(refDateStr + "T12:00:00Z");
  const currentDay = d.getUTCDay();
  let diff = targetDay - currentDay;
  if (diff <= 0) diff += 7;
  d.setUTCDate(d.getUTCDate() + diff);
  return d.toISOString().split("T")[0];
}

function parsedJsonEntryToEventDetails(raw: ParsedJsonEntry, refDate = "2026-09-06"): EventDetails {
  const isAllDay = raw.is_all_day;
  const byDays = convertDayNamesToByDay(raw.days);
  const isRepeating = Boolean(raw.repeating || byDays.length > 0);
  const startTime24 = parseTimeTo24h(raw.start_time);
  const endTime24 = parseTimeTo24h(raw.end_time);

  let dateStr = raw.date;
  if (!dateStr && byDays.length > 0 && raw.days && raw.days.length > 0) {
    dateStr = getUpcomingWeekdayDate(raw.days[0], refDate);
  }

  let startTimeIso: string | null = null;
  let endTimeIso: string | null = null;

  if (isAllDay) {
    startTimeIso = raw.date || refDate;
    endTimeIso = raw.date || refDate;
  } else if (dateStr && startTime24) {
    startTimeIso = `${dateStr}T${startTime24}:00Z`;
    endTimeIso = endTime24 ? `${dateStr}T${endTime24}:00Z` : `${dateStr}T${startTime24}:00Z`;
  }

  let recurrenceRule: string | null = null;
  if (isRepeating && byDays.length > 0) {
    recurrenceRule = `FREQ=WEEKLY;BYDAY=${byDays.join(",")};UNTIL=20261218T235959Z`;
  }

  return {
    title: raw.title,
    start_time: startTimeIso,
    end_time: endTimeIso,
    is_all_day: isAllDay,
    location: raw.location ?? null,
    description: raw.description ?? null,
    url: raw.url ?? null,
    recurrence_rule: recurrenceRule,
    confidence: 0.95,
  };
}

describe("Event Service & Multi-Method ICS Generation (using samples/parsed.json)", () => {
  const samplesManifest = parsedManifest as Record<string, ParsedJsonEntry[]>;

  const classEntries = samplesManifest["samples/class.png"];
  const classesEntries = samplesManifest["samples/classes.png"];
  const commonsEntries = samplesManifest["samples/commons.jpg"];
  const gilmanFlyerEntries = samplesManifest["samples/gilman_flyer.png"];
  const instagramEntries = samplesManifest["samples/instagram.png"];
  const rideForLifeEntries = samplesManifest["samples/ride_for_life.png"];
  const squirrelFlowerEntries = samplesManifest["samples/squirrel_flower.jpg"];

  it("loads all 7 sample image ground truths from samples/parsed.json", () => {
    expect(classEntries.length).toBe(1);
    expect(classesEntries.length).toBe(6);
    expect(commonsEntries.length).toBe(1);
    expect(gilmanFlyerEntries.length).toBe(1);
    expect(instagramEntries.length).toBe(1);
    expect(rideForLifeEntries.length).toBe(1);
    expect(squirrelFlowerEntries.length).toBe(1);
  });
  describe("Multi-Event & Single-Event ICS Generation from parsed.json", () => {
    it("generates a multi-event RFC 5545 iCalendar (.ics) string for classes.png with 6 distinct VEVENT blocks", () => {
      const sampleEvents: EventDetails[] = classesEntries.map((e) =>
        parsedJsonEntryToEventDetails(e),
      );
      const icsContent = generateMultiIcsCalendarContent(sampleEvents);

      expect(icsContent).toStartWith("BEGIN:VCALENDAR");
      expect(icsContent).toEndWith("END:VCALENDAR");

      const beginMatches = icsContent.match(/BEGIN:VEVENT/g);
      const endMatches = icsContent.match(/END:VEVENT/g);

      expect(beginMatches).not.toBeNull();
      expect(beginMatches!.length).toBe(6);
      expect(endMatches).not.toBeNull();
      expect(endMatches!.length).toBe(6);

      // Verify titles from parsed.json
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

      // Verify locations from parsed.json
      expect(icsContent).toContain("LOCATION:Anderson Wing TTC\\, Room 306");
      expect(icsContent).toContain("LOCATION:Online");
      expect(icsContent).toContain("LOCATION:Eliot-Pearson\\, Room 157");
      expect(icsContent).toContain("LOCATION:Eaton Hall\\, 201");
      expect(icsContent).toContain("LOCATION:Joyce Cummings Center\\, 302");
      expect(icsContent).toContain("LOCATION:Bromfield-Pearson\\, Room 006");

      // Verify RRULE presence in multi-event ICS
      expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959Z");
      expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU,TH;UNTIL=20261218T235959Z");
      expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=FR;UNTIL=20261218T235959Z");
      expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TH;UNTIL=20261218T235959Z");
      expect(icsContent).toContain("RRULE:FREQ=WEEKLY;BYDAY=TU;UNTIL=20261218T235959Z");
    });

    it("generates single event ICS correctly for class.png (CVS-0188 Seminar)", () => {
      const event = parsedJsonEntryToEventDetails(classEntries[0]);
      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain("SUMMARY:CVS-0188 Children and Media Seminar");
      expect(ics).toContain("LOCATION:Eliot-Pearson\\, Room 157");
      expect(ics).toContain("RRULE:FREQ=WEEKLY;BYDAY=WE;UNTIL=20261218T235959Z");
    });

    it("generates single event ICS correctly for Gilman Square Arts & Music Festival flyer sample", () => {
      const event = parsedJsonEntryToEventDetails(gilmanFlyerEntries[0]);
      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain("BEGIN:VEVENT");
      expect(ics).toContain("END:VEVENT");
      expect(ics).toContain("SUMMARY:SomerStreets Gilman Square Arts & Music Festival");
      expect(ics).toContain("LOCATION:Ed Leathers Park\\, Walnut Street and Skilton Ave");
      expect(ics).toContain("STATUS:CONFIRMED");
      expect(ics).toContain("DTSTART:20260912T120000Z");
      expect(ics).toContain("DTEND:20260912T170000Z");
    });

    it("generates single event ICS correctly for UEP Ice Cream Social Instagram sample", () => {
      const event = parsedJsonEntryToEventDetails(instagramEntries[0]);
      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain("SUMMARY:UEP ICE CREAM SOCIAL");
      expect(ics).toContain("LOCATION:BP Lawn");
      expect(ics).toContain("DTSTART:20260909T120000Z");
      expect(ics).toContain("DTEND:20260909T130000Z");
    });

    it("generates single event ICS correctly for Squirrel Flower concert tour flyer sample", () => {
      const event = parsedJsonEntryToEventDetails(squirrelFlowerEntries[0]);
      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain("SUMMARY:Squirrel Flower – 2026 Tour (with youbet)");
      expect(ics).toContain("LOCATION:Crystal Ballroom\\, Somerville\\, MA");
      expect(ics).toContain("DTSTART;VALUE=DATE:20260926");
      expect(ics).toContain("DTEND;VALUE=DATE:20260926");
    });

    it("generates single event ICS correctly for Ride For Your Life advocacy event sample", () => {
      const event = parsedJsonEntryToEventDetails(rideForLifeEntries[0]);
      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain("SUMMARY:Ride For Your Life");
      expect(ics).toContain("LOCATION:Boston\\, MA");
      expect(ics).toContain("DTSTART;VALUE=DATE:20261025");
      expect(ics).toContain("DTEND;VALUE=DATE:20261025");
    });

    it("generates single event ICS correctly for commons.jpg (Campus as Commons with QR code webinar URL)", () => {
      const event = parsedJsonEntryToEventDetails(commonsEntries[0]);
      expect(event.url).toBe("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");

      const ics = generateIcsCalendarContent(event);
      expect(ics).toStartWith("BEGIN:VCALENDAR");
      expect(ics).toEndWith("END:VCALENDAR");
      expect(ics).toContain(
        "SUMMARY:CAMPUS AS COMMONS: Agroforestry and Shared Stewardship at Tufts",
      );
      expect(ics).toContain("LOCATION:Curtis Hall Multipurpose Room");
      expect(ics).toContain("DTSTART:20260910T120000Z");
      expect(ics).toContain("DTEND:20260910T130000Z");
      expect(ics).toContain("URL:https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");
    });

    it("generates multi-event ICS correctly including URL fields for events with URLs", () => {
      const event1: EventDetails = {
        title: "Webinar Session 1",
        start_time: "2026-09-10T12:00:00Z",
        end_time: "2026-09-10T13:00:00Z",
        is_all_day: false,
        url: "https://zoom.us/j/session1",
        confidence: 0.95,
      };
      const event2: EventDetails = {
        title: "Session 2 (In Person)",
        start_time: "2026-09-10T14:00:00Z",
        end_time: "2026-09-10T15:00:00Z",
        is_all_day: false,
        location: "Hall A",
        confidence: 0.95,
      };
      const ics = generateMultiIcsCalendarContent([event1, event2]);
      expect(ics).toContain("URL:https://zoom.us/j/session1");
      expect(ics).toContain("SUMMARY:Webinar Session 1");
      expect(ics).toContain("SUMMARY:Session 2 (In Person)");
    });
  });

  describe("Formatting & Date/Time Form Helpers", () => {
    it("extracts and formats date and time values accurately for Gilman Square flyer event", () => {
      const event = parsedJsonEntryToEventDetails(gilmanFlyerEntries[0]);
      const dateInput = extractDateInput(event.start_time);
      expect(dateInput).toBe("2026-09-12");

      const startTimeInput = extractTimeInput(event.start_time);
      expect(startTimeInput).toBe("12:00");

      const endTimeInput = extractTimeInput(event.end_time);
      expect(endTimeInput).toBe("17:00");

      const formattedDate = formatDateForDisplay(event.start_time);
      expect(formattedDate).toContain("2026");
      expect(formattedDate).toContain("Sep");

      const formattedStartTime = formatTimeForDisplay(event.start_time);
      expect(formattedStartTime).not.toBe("");

      const formattedEndTime = formatTimeForDisplay(event.end_time);
      expect(formattedEndTime).not.toBe("");

      const reconstructedStart = buildIsoFromDateTime("2026-09-12", "12:00");
      expect(reconstructedStart).toContain("2026-09-12T12:00");

      const reconstructedEnd = buildIsoFromDateTime("2026-09-12", "17:00");
      expect(reconstructedEnd).toContain("2026-09-12T17:00");
    });

    it("extracts and formats date and time values accurately for UEP Ice Cream Social event", () => {
      const event = parsedJsonEntryToEventDetails(instagramEntries[0]);
      const dateInput = extractDateInput(event.start_time);
      expect(dateInput).toBe("2026-09-09");

      const startTimeInput = extractTimeInput(event.start_time);
      expect(startTimeInput).toBe("12:00");

      const endTimeInput = extractTimeInput(event.end_time);
      expect(endTimeInput).toBe("13:00");

      const formattedDate = formatDateForDisplay(event.start_time);
      expect(formattedDate).toContain("2026");
      expect(formattedDate).toContain("Sep");

      const reconstructedStart = buildIsoFromDateTime("2026-09-09", "12:00");
      expect(reconstructedStart).toContain("2026-09-09T12:00");

      const reconstructedEnd = buildIsoFromDateTime("2026-09-09", "13:00");
      expect(reconstructedEnd).toContain("2026-09-09T13:00");
    });

    it("extracts date values accurately for all-day events (Squirrel Flower & Ride For Life)", () => {
      const squirrel = parsedJsonEntryToEventDetails(squirrelFlowerEntries[0]);
      expect(extractDateInput(squirrel.start_time)).toBe("2026-09-26");
      expect(formatDateForDisplay(squirrel.start_time)).toContain("2026");
      expect(formatDateForDisplay(squirrel.start_time)).toContain("Sep");

      const ride = parsedJsonEntryToEventDetails(rideForLifeEntries[0]);
      expect(extractDateInput(ride.start_time)).toBe("2026-10-25");
      expect(formatDateForDisplay(ride.start_time)).toContain("2026");
      expect(formatDateForDisplay(ride.start_time)).toContain("Oct");
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
      expect(icsOffset).toContain("DTSTART:20260907T150000");
      expect(icsOffset).toContain("DTEND:20260907T161500");
      expect(icsOffset).toContain("RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261218T235959");
    });

    it("supports EventFormData and EventDetails models with first-class URL field", () => {
      const formData: EventFormData = {
        title: "Campus as Commons Agroforestry Lecture",
        date: "2026-09-10",
        startTime: "12:00",
        endTime: "13:00",
        isAllDay: false,
        location: "Curtis Hall Multipurpose Room",
        description: "Visiting Artist lecture and Q&A.",
        url: "https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw",
      };

      expect(formData.url).toBe("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");

      const details: EventDetails = {
        title: formData.title,
        start_time: "2026-09-10T12:00:00-04:00",
        end_time: "2026-09-10T13:00:00-04:00",
        is_all_day: false,
        location: formData.location,
        description: formData.description,
        url: formData.url,
        confidence: 0.95,
        source: "ocr",
      };

      expect(details.url).toBe("https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");
      const ics = generateIcsCalendarContent(details);
      expect(ics).toContain("URL:https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");
    });

    it("persists URL across single and batch calendar additions in ICS generation", () => {
      const event1: EventDetails = {
        title: "Webinar Lecture",
        start_time: "2026-09-10T12:00:00-04:00",
        end_time: "2026-09-10T13:00:00-04:00",
        is_all_day: false,
        location: "Curtis Hall",
        description: "Keynote presentation",
        url: "https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw",
        confidence: 0.95,
        source: "ocr",
      };

      const event2: EventDetails = {
        title: "Evening Concert",
        start_time: "2026-09-12T19:00:00-04:00",
        end_time: "2026-09-12T22:00:00-04:00",
        is_all_day: false,
        location: "Main Stage",
        description: "Live band performance",
        url: "https://example.com/tickets",
        confidence: 0.95,
        source: "ocr",
      };

      const eventWithoutUrl: EventDetails = {
        title: "Morning Walk",
        start_time: "2026-09-13T09:00:00-04:00",
        end_time: "2026-09-13T10:00:00-04:00",
        is_all_day: false,
        location: "Park Trail",
        description: "Casual community walk",
        url: null,
        confidence: 0.90,
        source: "ocr",
      };

      // Single event ICS
      const singleIcs = generateIcsCalendarContent(event1);
      expect(singleIcs).toContain("URL:https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");

      // Batch / Multi-event ICS
      const batchIcs = generateMultiIcsCalendarContent([event1, event2, eventWithoutUrl]);
      expect(batchIcs).toContain("URL:https://tufts.zoom.us/webinar/register/WN_trzRawg4RbKfQBvJ5ylTDw");
      expect(batchIcs).toContain("URL:https://example.com/tickets");

      // Verify VEVENT separation and count
      const veventCount = (batchIcs.match(/BEGIN:VEVENT/g) || []).length;
      expect(veventCount).toBe(3);

      const urlCount = (batchIcs.match(/URL:/g) || []).length;
      expect(urlCount).toBe(2);
    });

    it("correctly identifies web URLs vs non-web QR payloads with isWebLink", () => {
      expect(isWebLink("https://tufts.zoom.us/webinar/123")).toBe(true);
      expect(isWebLink("http://example.com/tickets")).toBe(true);
      expect(isWebLink("HTTPS://SUBDOMAIN.EXAMPLE.ORG/PATH?Q=1")).toBe(true);
      expect(isWebLink("  https://whitespace.com/link  ")).toBe(true);

      expect(isWebLink("WIFI:S:MyNetwork;T:WPA;P:secret;;")).toBe(false);
      expect(isWebLink("BEGIN:VCARD\nVERSION:3.0\nFN:John Doe\nEND:VCARD")).toBe(false);
      expect(isWebLink("mailto:contact@example.com")).toBe(false);
      expect(isWebLink("tel:+16175551234")).toBe(false);
      expect(isWebLink("Just some raw flyer text")).toBe(false);
      expect(isWebLink("")).toBe(false);
      expect(isWebLink(null)).toBe(false);
      expect(isWebLink(undefined)).toBe(false);
    });

    it("supports simple and enhanced parsing modes for progressive retry flow", async () => {
      const sampleText = "Weekly Pottery Class\nTuesdays & Thursdays 6:00 PM - 8:00 PM\nCommunity Arts Center";

      // In browser/node test environment, verify parseEventsFromText returns events
      const simpleEvents = await parseEventsFromText(sampleText, undefined, undefined, undefined, undefined, "simple");
      expect(simpleEvents.length).toBeGreaterThan(0);
      expect(simpleEvents[0].title).toBe("Weekly Pottery Class");

      const enhancedEvents = await parseEventsFromText(sampleText, undefined, undefined, undefined, undefined, "enhanced");
      expect(enhancedEvents.length).toBeGreaterThan(0);
      expect(enhancedEvents[0].title).toBe("Weekly Pottery Class");
    });

    it("extracts and cleans URLs from pasted text with extractUrlsFromText", () => {
      const textWithLinks = `
        Tech Talk & Networking
        Friday, Oct 24 at 5:00 PM
        Location: Main Auditorium
        RSVP here: https://meetup.example.com/events/123.
        More info at http://events.example.org/details, or visit https://meetup.example.com/events/123
        Contact us at contact@example.com or call tel:+1234567890
      `;

      const urls = extractUrlsFromText(textWithLinks);
      expect(urls).toHaveLength(2);
      expect(urls).toContain("https://meetup.example.com/events/123");
      expect(urls).toContain("http://events.example.org/details");

      // Edge cases
      expect(extractUrlsFromText("")).toEqual([]);
      expect(extractUrlsFromText(null)).toEqual([]);
      expect(extractUrlsFromText(undefined)).toEqual([]);
      expect(extractUrlsFromText("No links in this text at all")).toEqual([]);
      expect(extractUrlsFromText("Visit (https://subdomain.example.com/path?a=1&b=2)")).toEqual([
        "https://subdomain.example.com/path?a=1&b=2",
      ]);
    });

    it("supports parseEventsProgressive with onSimpleResult callback and enhanced completion", async () => {
      const pastedEventText = "Weekly Pottery Class\nTuesdays & Thursdays 6:00 PM - 8:00 PM\nCommunity Arts Center";

      let simpleCallbackEvents: EventDetails[] | null = null;
      const result = await parseEventsProgressive(pastedEventText, {
        onSimpleResult: (events) => {
          simpleCallbackEvents = events;
        },
      });

      // Simple result callback was called immediately
      expect(simpleCallbackEvents).not.toBeNull();
      expect(simpleCallbackEvents!.length).toBeGreaterThan(0);
      expect(simpleCallbackEvents![0].title).toBe("Weekly Pottery Class");

      // Returned both simple and enhanced
      expect(result.simple.length).toBeGreaterThan(0);
      expect(result.simple[0].title).toBe("Weekly Pottery Class");
      expect(result.enhanced).toBeDefined();
      expect(result.enhanced!.length).toBeGreaterThan(0);
      expect(result.enhanced![0].title).toBe("Weekly Pottery Class");
    });

    it("supports parseEventsProgressive with skipEnhanced: true", async () => {
      const pastedEventText = "Weekly Pottery Class\nTuesdays & Thursdays 6:00 PM - 8:00 PM\nCommunity Arts Center";

      let simpleCallbackEvents: EventDetails[] | null = null;
      const result = await parseEventsProgressive(pastedEventText, {
        skipEnhanced: true,
        onSimpleResult: (events) => {
          simpleCallbackEvents = events;
        },
      });

      expect(simpleCallbackEvents).not.toBeNull();
      expect(result.simple.length).toBeGreaterThan(0);
      expect(result.enhanced).toBeUndefined();
    });
  });
});
