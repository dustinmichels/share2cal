<script setup lang="ts">
import { computed } from "vue";
import type { CalendarInfo } from "../services/calendar";
import type { CalendarTarget } from "../services/settings";
import {
  extractDateInput,
  formatDateForDisplay,
  formatTimeForDisplay,
  formatRecurrenceForDisplay,
  type EventDetails,
} from "../services/event";

const selectedCalendarId = defineModel<string>("selectedCalendarId", { default: "" });

const props = withDefaults(
  defineProps<{
    events: EventDetails[];
    isAddingToCalendar?: boolean;
    copiedSummary?: boolean;
    addedIndices?: Set<number>;
    availableCalendars?: CalendarInfo[];
    defaultTarget?: CalendarTarget;
    hideHeader?: boolean;
  }>(),
  {
    defaultTarget: "native",
    hideHeader: false,
  },
);

const emit = defineEmits<{
  (e: "editEvent", index: number): void;
  (e: "addToCalendar"): void;
  (e: "openGoogleCalendar", index?: number): void;
  (e: "exportIcs"): void;
  (e: "copySummary"): void;
  (e: "removeEvent", index: number): void;
}>();
interface WeekPreviewEvent {
  event: EventDetails;
  index: number;
  timeLabel: string;
  recurrenceLabel: string | null;
}

interface WeekPreviewDay {
  key: string;
  weekdayLabel: string;
  dateLabel: string;
  events: WeekPreviewEvent[];
}

const eventCount = computed(() => props.events.length);
const isMultiple = computed(() => eventCount.value > 1);

const overallConfidence = computed(() => {
  if (props.events.length === 0) return 0;
  const sum = props.events.reduce((acc, curr) => acc + (curr.confidence || 0.8), 0);
  return Math.round((sum / props.events.length) * 100);
});

const previewMode = defineModel<"week" | "details">("previewMode", { default: "details" });

const weekStartKey = computed(() => {
  const eventDateKeys = props.events
    .map(getEventDateKey)
    .filter((key): key is string => Boolean(key));
  if (eventDateKeys.length === 0) return null;

  const firstEventDate = dateFromLocalKey(eventDateKeys.sort()[0]);
  const weekStart = new Date(firstEventDate);
  weekStart.setDate(firstEventDate.getDate() - firstEventDate.getDay());
  return formatLocalDateKey(weekStart);
});

const weekDays = computed<WeekPreviewDay[]>(() => {
  if (!weekStartKey.value) return [];

  const weekStart = dateFromLocalKey(weekStartKey.value);
  const days: WeekPreviewDay[] = [];

  for (let dayOffset = 0; dayOffset < 7; dayOffset++) {
    const date = new Date(weekStart);
    date.setDate(weekStart.getDate() + dayOffset);
    days.push({
      key: formatLocalDateKey(date),
      weekdayLabel: date.toLocaleDateString(undefined, { weekday: "short" }),
      dateLabel: date.toLocaleDateString(undefined, { month: "short", day: "numeric" }),
      events: [],
    });
  }

  const dayByKey = new Map(days.map((day) => [day.key, day]));
  props.events.forEach((event, index) => {
    const eventDateKey = getEventDateKey(event);
    const day = eventDateKey ? dayByKey.get(eventDateKey) : null;
    if (!day) return;
    day.events.push({
      event,
      index,
      timeLabel: formatWeekEventTime(event),
      recurrenceLabel: formatRecurrenceForDisplay(event.recurrence_rule),
    });
  });

  for (const day of days) {
    day.events.sort((a, b) => getEventSortValue(a.event) - getEventSortValue(b.event));
  }

  return days;
});

const weekDayKeys = computed(() => new Set(weekDays.value.map((day) => day.key)));

const eventsOutsidePreviewWeek = computed(() => {
  if (!weekStartKey.value) return 0;
  return props.events.filter((event) => {
    const eventDateKey = getEventDateKey(event);
    return eventDateKey && !weekDayKeys.value.has(eventDateKey);
  }).length;
});

const unscheduledEventCount = computed(
  () => props.events.filter((event) => !getEventDateKey(event)).length,
);

const hasRepeatingEvents = computed(() =>
  props.events.some((event) => Boolean(formatRecurrenceForDisplay(event.recurrence_rule))),
);

function getEventDateKey(event: EventDetails): string | null {
  if (!event.start_time) return null;
  return extractDateInput(event.start_time) || null;
}

function dateFromLocalKey(dateKey: string): Date {
  return new Date(`${dateKey}T12:00:00`);
}

function formatLocalDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function getEventSortValue(event: EventDetails): number {
  if (event.is_all_day) return -1;
  if (!event.start_time) return Number.MAX_SAFE_INTEGER;
  const time = new Date(event.start_time).getTime();
  return Number.isFinite(time) ? time : Number.MAX_SAFE_INTEGER;
}

function formatWeekEventTime(event: EventDetails): string {
  if (event.is_all_day) return "All day";

  const startTimeStr = formatTimeForDisplay(event.start_time);
  const endTimeStr = formatTimeForDisplay(event.end_time);
  if (startTimeStr && endTimeStr) return `${startTimeStr}–${endTimeStr}`;
  if (startTimeStr) return startTimeStr;
  return "Time TBD";
}

function formatEventTiming(event: EventDetails): string {
  const dateStr = formatDateForDisplay(event.start_time);
  if (event.is_all_day) {
    return `${dateStr} • All day`;
  }
  const startTimeStr = formatTimeForDisplay(event.start_time);
  const endTimeStr = formatTimeForDisplay(event.end_time);
  if (startTimeStr && endTimeStr) {
    return `${dateStr} • ${startTimeStr} – ${endTimeStr}`;
  } else if (startTimeStr) {
    return `${dateStr} • ${startTimeStr}`;
  }
  return dateStr;
}
</script>

<template>
  <div class="surface-card preview-section">
    <!-- Section Header -->
    <div v-if="!hideHeader" class="section-header">
      <div class="section-title-wrap">
        <div class="section-icon-badge">
          <svg
            class="section-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="4" width="18" height="18" rx="3" ry="3"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="3" y1="10" x2="21" y2="10"></line>
            <path d="M8 14h.01"></path>
            <path d="M12 14h.01"></path>
            <path d="M16 14h.01"></path>
          </svg>
        </div>
        <div>
          <h2 class="section-heading">
            {{ isMultiple ? `Extracted Events (${eventCount})` : "Event Summary" }}
          </h2>
          <p class="section-subheading">
            {{
              isMultiple
                ? "Tap any event to review and edit details"
                : "Tap the preview to edit details before adding"
            }}
          </p>
        </div>
      </div>

      <div class="section-header-actions">
        <div class="preview-mode-toggle" aria-label="Preview mode">
          <button
            type="button"
            class="preview-mode-button"
            :class="{ active: previewMode === 'details' }"
            @click="previewMode = 'details'"
          >
            Detail View
          </button>
          <button
            type="button"
            class="preview-mode-button"
            :class="{ active: previewMode === 'week' }"
            @click="previewMode = 'week'"
          >
            Calendar View
          </button>
        </div>
        <span class="badge-pill badge-pill-confidence">{{ overallConfidence }}% match</span>
      </div>
    </div>

    <!-- One-week calendar-style preview -->
    <div
      v-if="previewMode === 'week'"
      class="week-preview-card"
      aria-label="One week calendar preview"
    >
      <div class="week-preview-heading">
        <div>
          <h3 class="week-preview-title">Calendar View</h3>
          <p class="week-preview-subtitle">
            A calendar view of where the event dates land.
          </p>
        </div>
        <span class="badge-pill week-preview-count"
          >{{ eventCount }} {{ eventCount === 1 ? "event" : "events" }}</span
        >
      </div>

      <div v-if="weekDays.length" class="week-preview-grid" role="list">
        <section
          v-for="day in weekDays"
          :key="day.key"
          class="week-day-column"
          :class="{ 'has-events': day.events.length > 0 }"
          role="listitem"
        >
          <header class="week-day-header">
            <span class="week-day-name">{{ day.weekdayLabel }}</span>
            <span class="week-day-date">{{ day.dateLabel }}</span>
          </header>

          <div class="week-day-events">
            <button
              v-for="previewEvent in day.events"
              :key="`${day.key}-${previewEvent.index}`"
              type="button"
              class="week-event-block"
              :aria-label="`Edit event: ${previewEvent.event.title || 'Untitled event'}`"
              @click="emit('editEvent', previewEvent.index)"
            >
              <span class="week-event-time">{{ previewEvent.timeLabel }}</span>
              <span class="week-event-title">{{
                previewEvent.event.title || "Untitled Event"
              }}</span>
              <span v-if="previewEvent.event.location" class="week-event-location">
                {{ previewEvent.event.location }}
              </span>
              <span v-if="previewEvent.recurrenceLabel" class="week-event-repeat">Repeats</span>
            </button>

            <span v-if="day.events.length === 0" class="week-empty-slot">No events</span>
          </div>
        </section>
      </div>
      <div v-else class="week-empty-notice">
        <p class="week-empty-text">No scheduled dates found in event(s). Switch to Detail View to review dates.</p>
        <button type="button" class="btn-touch btn-touch-outline btn-switch-details" @click="previewMode = 'details'">
          Switch to Detail View
        </button>
      </div>

      <p
        v-if="eventsOutsidePreviewWeek || unscheduledEventCount || hasRepeatingEvents"
        class="week-preview-note"
      >
        <template v-if="eventsOutsidePreviewWeek">
          {{ eventsOutsidePreviewWeek }}
          {{ eventsOutsidePreviewWeek === 1 ? "event is" : "events are" }}
          outside this preview week.
        </template>
        <template v-if="unscheduledEventCount">
          {{ unscheduledEventCount }}
          {{ unscheduledEventCount === 1 ? "event has" : "events have" }}
          no date yet.
        </template>
        <template v-if="hasRepeatingEvents">
          Repeating events are shown on their start day only.
        </template>
      </p>
    </div>

    <!-- Detailed events list -->
    <div v-else class="events-preview-list">
      <div
        v-for="(event, index) in events"
        :key="index"
        class="event-preview-item"
        role="button"
        tabindex="0"
        :aria-label="`Edit event: ${event.title || 'Untitled event'}`"
        @click="emit('editEvent', index)"
        @keydown.enter.prevent="emit('editEvent', index)"
        @keydown.space.prevent="emit('editEvent', index)"
      >
        <div class="preview-item-header">
          <div class="title-with-badge">
            <span v-if="isMultiple" class="event-index-chip">#{{ index + 1 }}</span>
            <h3 class="event-preview-title">{{ event.title || "Untitled Event" }}</h3>
          </div>
          <div class="header-right-meta">
            <span v-if="addedIndices?.has(index)" class="badge-pill badge-pill-added">
              Added ✓
            </span>
            <span class="edit-cue-btn" title="Edit details">
              <svg
                class="chevron-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <polyline points="9 18 15 12 9 6"></polyline>
              </svg>
            </span>
          </div>
        </div>

        <!-- Event Metadata Details -->
        <div class="preview-meta-rows">
          <!-- Timing row -->
          <div class="meta-row">
            <svg
              class="meta-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10"></circle>
              <polyline points="12 6 12 12 16 14"></polyline>
            </svg>
            <span class="meta-text">{{ formatEventTiming(event) }}</span>
          </div>

          <!-- Recurrence row (if repeating) -->
          <div
            v-if="formatRecurrenceForDisplay(event.recurrence_rule)"
            class="meta-row meta-row-recurrence"
          >
            <svg
              class="meta-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="17 1 21 5 17 9"></polyline>
              <path d="M3 11V9a4 4 0 0 1 4-4h14"></path>
              <polyline points="7 23 3 19 7 15"></polyline>
              <path d="M21 13v2a4 4 0 0 1-4 4H3"></path>
            </svg>
            <span class="meta-text meta-recurrence-text">{{
              formatRecurrenceForDisplay(event.recurrence_rule)
            }}</span>
          </div>

          <!-- Location row (if available) -->
          <div v-if="event.location" class="meta-row">
            <svg
              class="meta-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"></path>
              <circle cx="12" cy="10" r="3"></circle>
            </svg>
            <span class="meta-text">{{ event.location }}</span>
          </div>

          <!-- Description snippet (if available) -->
          <div v-if="event.description" class="meta-row meta-row-desc">
            <svg
              class="meta-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
              <polyline points="14 2 14 8 20 8"></polyline>
              <line x1="16" y1="13" x2="8" y2="13"></line>
              <line x1="16" y1="17" x2="8" y2="17"></line>
            </svg>
            <span class="meta-text meta-desc-text">{{ event.description }}</span>
          </div>
        </div>

        <div class="preview-item-footer">
          <span class="tap-hint">Tap to edit details</span>
          <button
            v-if="isMultiple"
            type="button"
            class="btn-remove-event"
            title="Remove from schedule"
            @click.stop="emit('removeEvent', index)"
          >
            <svg
              class="trash-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="3 6 5 6 21 6"></polyline>
              <path
                d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
              ></path>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Batch / Primary Actions -->
    <div class="preview-actions-flow">
      <!-- Destination Calendar Selection -->
      <div
        v-if="availableCalendars && availableCalendars.length > 0"
        class="calendar-destination-row"
      >
        <label for="preview-target-cal" class="destination-label">
          <svg
            class="dest-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="3" y1="10" x2="21" y2="10"></line>
          </svg>
          <span>Save to:</span>
        </label>
        <select id="preview-target-cal" v-model="selectedCalendarId" class="destination-select">
          <option value="">Default Calendar (System)</option>
          <option v-for="cal in availableCalendars" :key="cal.id" :value="cal.id">
            {{ cal.title }} {{ cal.source_title ? `· ${cal.source_title}` : "" }}
          </option>
        </select>
      </div>

      <!-- Primary Action (based on defaultTarget preference) -->
      <button
        v-if="defaultTarget === 'google'"
        type="button"
        class="btn-touch btn-touch-calendar"
        @click="emit('openGoogleCalendar')"
      >
        <svg
          class="btn-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
          <polyline points="15 3 21 3 21 9"></polyline>
          <line x1="10" y1="14" x2="21" y2="3"></line>
        </svg>
        <span>{{ isMultiple ? `Open First Event in Google Cal` : "Add to Google Calendar" }}</span>
      </button>

      <button
        v-else-if="defaultTarget === 'ics'"
        type="button"
        class="btn-touch btn-touch-calendar"
        @click="emit('exportIcs')"
      >
        <svg
          class="btn-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
          <polyline points="7 10 12 15 17 10"></polyline>
          <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>
        <span>{{ isMultiple ? `Export All (${eventCount}) to .ics` : "Export .ics File" }}</span>
      </button>

      <button
        v-else
        type="button"
        class="btn-touch btn-touch-calendar"
        :disabled="isAddingToCalendar"
        @click="emit('addToCalendar')"
      >
        <template v-if="!isAddingToCalendar">
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
            <polyline points="17 21 17 13 7 13 7 21"></polyline>
            <polyline points="7 3 7 8 15 8"></polyline>
          </svg>
          <span>{{ isMultiple ? `Add All (${eventCount}) to Calendar` : "Add to Calendar" }}</span>
        </template>
        <template v-else>
          <div class="spinner-circle"></div>
          <span>Adding to Calendar...</span>
        </template>
      </button>

      <div class="secondary-actions-grid">
        <button
          v-if="defaultTarget !== 'native'"
          type="button"
          class="btn-touch btn-touch-outline"
          :disabled="isAddingToCalendar"
          @click="emit('addToCalendar')"
        >
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
          </svg>
          <span>{{ isMultiple ? "Add to Native" : "Apple Cal" }}</span>
        </button>

        <button
          v-if="defaultTarget !== 'google'"
          type="button"
          class="btn-touch btn-touch-outline"
          title="Open in Google Calendar"
          @click="emit('openGoogleCalendar')"
        >
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
            <polyline points="15 3 21 3 21 9"></polyline>
            <line x1="10" y1="14" x2="21" y2="3"></line>
          </svg>
          <span>Google Cal</span>
        </button>

        <button
          v-if="defaultTarget !== 'ics'"
          type="button"
          class="btn-touch btn-touch-outline"
          @click="emit('exportIcs')"
        >
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
          <span>{{ isMultiple ? "Export All" : "Export .ics" }}</span>
        </button>
        <button
          type="button"
          class="btn-touch btn-touch-outline"
          :class="{ 'is-copied': copiedSummary }"
          @click="emit('copySummary')"
        >
          <svg
            v-if="!copiedSummary"
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
          <svg
            v-else
            class="btn-icon check-animated"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span>{{ copiedSummary ? "Copied!" : isMultiple ? "Copy All" : "Copy Details" }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.preview-section {
  animation: fadeIn 0.25s ease-out;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  padding-bottom: 0.25rem;
}

.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.section-icon-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: var(--accent-primary-light);
  color: var(--accent-primary);
  flex-shrink: 0;
}

.section-icon {
  width: 20px;
  height: 20px;
}

.section-heading {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.section-subheading {
  font-size: 0.84rem;
  color: var(--text-secondary);
  margin: 0.15rem 0 0 0;
}

.section-header-actions {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  flex-shrink: 0;
}

.preview-mode-toggle {
  display: flex;
  align-items: center;
  gap: 0.15rem;
  padding: 0.18rem;
  border: 1px solid var(--border-input);
  border-radius: 999px;
  background: var(--bg-input);
}

.preview-mode-button {
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.72rem;
  font-weight: 800;
  line-height: 1;
  padding: 0.38rem 0.55rem;
  transition:
    background 0.15s ease,
    color 0.15s ease,
    box-shadow 0.15s ease;
}

.preview-mode-button.active {
  background: var(--bg-card);
  color: var(--accent-primary);
  box-shadow: 0 1px 4px rgba(15, 23, 42, 0.08);
}

.week-preview-card {
  background: linear-gradient(180deg, var(--bg-card-elevated) 0%, var(--bg-input) 100%);
  border: 1px solid var(--border-input);
  border-radius: var(--radius-card);
  padding: 0.75rem;
}

.week-preview-heading {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.7rem;
}

.week-preview-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.week-preview-subtitle {
  font-size: 0.78rem;
  color: var(--text-secondary);
  margin: 0.15rem 0 0 0;
  line-height: 1.35;
}

.week-preview-count {
  background: var(--accent-primary-light);
  color: var(--accent-primary);
  white-space: nowrap;
}

.week-preview-grid {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 0.35rem;
}

.week-empty-notice {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 2rem 1rem;
  text-align: center;
}

.week-empty-text {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin: 0;
}

.btn-switch-details {
  width: auto;
  min-height: 38px;
  padding: 0.5rem 1rem;
  font-size: 0.84rem;
}

.week-day-column {
  min-height: 128px;
  background: var(--bg-card);
  border: 1px solid var(--border-card-subtle);
  border-radius: 10px;
  padding: 0.42rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
}

.week-day-column.has-events {
  border-color: rgba(0, 122, 255, 0.28);
  box-shadow: 0 4px 12px rgba(0, 122, 255, 0.07);
}

.week-day-header {
  display: flex;
  flex-direction: column;
  gap: 0.05rem;
}

.week-day-name {
  font-size: 0.62rem;
  font-weight: 800;
  color: var(--text-tertiary);
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.week-day-date {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.15;
}

.week-day-events {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  flex: 1;
  min-width: 0;
}

.week-event-block {
  width: 100%;
  min-width: 0;
  border: 0;
  border-left: 3px solid var(--accent-primary);
  border-radius: 8px;
  background: var(--accent-primary-light);
  color: var(--text-primary);
  padding: 0.38rem 0.42rem;
  text-align: left;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 0.14rem;
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease,
    background 0.15s ease;
}

.week-event-block:hover {
  transform: translateY(-1px);
  box-shadow: 0 5px 12px rgba(0, 122, 255, 0.12);
}

.week-event-time {
  font-size: 0.56rem;
  font-weight: 800;
  color: var(--accent-primary);
  line-height: 1.15;
  overflow-wrap: anywhere;
}

.week-event-title {
  font-size: 0.68rem;
  font-weight: 700;
  line-height: 1.15;
  overflow: hidden;
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
}

.week-event-location {
  font-size: 0.6rem;
  color: var(--text-secondary);
  line-height: 1.15;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.week-event-repeat {
  align-self: flex-start;
  margin-top: 0.1rem;
  font-size: 0.56rem;
  font-weight: 800;
  color: #8a5a00;
  background: rgba(255, 204, 0, 0.28);
  border-radius: 999px;
  padding: 0.1rem 0.3rem;
}

.week-empty-slot {
  margin-top: auto;
  font-size: 0.62rem;
  color: var(--text-tertiary);
  line-height: 1.2;
}

.week-preview-note {
  margin: 0.7rem 0 0 0;
  font-size: 0.74rem;
  color: var(--text-tertiary);
  line-height: 1.4;
}

@media (prefers-color-scheme: dark) {
  .week-event-repeat {
    color: #ffd60a;
    background: rgba(255, 214, 10, 0.16);
  }
}

@media (max-width: 430px) {
  .section-header {
    align-items: flex-start;
  }

  .section-header-actions {
    align-items: flex-end;
    flex-direction: column-reverse;
  }

  .week-preview-grid {
    grid-template-columns: 1fr;
  }

  .week-day-column {
    min-height: auto;
  }
}

.events-preview-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.event-preview-item {
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: var(--radius-card);
  padding: 1rem 1.15rem;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  position: relative;
}

.event-preview-item:hover {
  background: var(--bg-card-elevated);
  border-color: var(--accent-primary);
  box-shadow: 0 4px 14px rgba(0, 122, 255, 0.08);
  transform: translateY(-1px);
}

.event-preview-item:active {
  transform: scale(0.99);
}

.preview-item-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.title-with-badge {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
}

.event-index-chip {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--accent-primary);
  background: var(--accent-primary-light);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.event-preview-title {
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-right-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
}

.badge-pill-added {
  background: rgba(52, 199, 89, 0.15);
  color: #15803d;
  font-size: 0.74rem;
}

@media (prefers-color-scheme: dark) {
  .badge-pill-added {
    background: rgba(48, 209, 88, 0.2);
    color: #4ade80;
  }
}

.edit-cue-btn {
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition:
    color 0.15s ease,
    transform 0.15s ease;
}

.event-preview-item:hover .edit-cue-btn {
  color: var(--accent-primary);
  transform: translateX(2px);
}

.chevron-icon {
  width: 18px;
  height: 18px;
}

.preview-meta-rows {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.86rem;
  color: var(--text-secondary);
}

.meta-row-desc {
  align-items: flex-start;
}

.meta-icon {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  color: var(--text-tertiary);
  margin-top: 1px;
}

.meta-text {
  line-height: 1.35;
}

.meta-desc-text {
  font-size: 0.82rem;
  color: var(--text-tertiary);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.preview-item-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 0.35rem;
  border-top: 1px solid var(--border-card-subtle);
}

.tap-hint {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--accent-primary);
}

.btn-remove-event {
  background: transparent;
  border: none;
  color: var(--text-tertiary);
  padding: 0.25rem;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.btn-remove-event:hover {
  color: #ff3b30;
  background: rgba(255, 59, 48, 0.1);
}

.trash-icon {
  width: 14px;
  height: 14px;
}

.calendar-destination-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.6rem;
  padding: 0.55rem 0.85rem;
  background: var(--bg-input);
  border: 1px solid var(--border-card-subtle);
  border-radius: 10px;
  margin-bottom: 0.25rem;
}

.destination-label {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}

.dest-icon {
  width: 15px;
  height: 15px;
  color: var(--accent-primary);
}

.destination-select {
  flex: 1;
  min-width: 0;
  max-width: 220px;
  padding: 0.35rem 0.6rem;
  font-size: 0.82rem;
  font-weight: 500;
  color: var(--text-primary);
  background: var(--bg-card);
  border: 1px solid var(--border-input);
  border-radius: 6px;
  outline: none;
  cursor: pointer;
}

.preview-actions-flow {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-top: 0.25rem;
}

.secondary-actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 0.5rem;
}
.btn-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.spinner-circle {
  width: 18px;
  height: 18px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
