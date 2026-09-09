<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { CalendarInfo } from "../services/calendar";
import type { CalendarTarget } from "../services/settings";
import {
  parseRecurrenceRule,
  buildRecurrenceRule,
  type EventFormData,
  type ParsedRecurrence,
} from "../services/event";

const model = defineModel<EventFormData>({ required: true });
const selectedCalendarId = defineModel<string>("selectedCalendarId", { default: "" });

const props = withDefaults(
  defineProps<{
    confidence: number;
    isAddingToCalendar?: boolean;
    isReparsing?: boolean;
    copiedSummary?: boolean;
    currentIndex?: number;
    totalEvents?: number;
    availableCalendars?: CalendarInfo[];
    defaultTarget?: CalendarTarget;
  }>(),
  {
    isAddingToCalendar: false,
    isReparsing: false,
    copiedSummary: false,
    currentIndex: 0,
    totalEvents: 1,
    defaultTarget: "native",
  },
);
const emit = defineEmits<{
  (e: "addToCalendar"): void;
  (e: "openGoogleCalendar"): void;
  (e: "exportIcs"): void;
  (e: "copySummary"): void;
  (e: "back"): void;
  (e: "remove"): void;
  (e: "tryAgain"): void;
}>();

const confidencePercent = computed(() => Math.round(props.confidence * 100));

const isRepeating = ref(Boolean(model.value.recurrenceRule?.trim()));
const parsedRecurrence = ref<ParsedRecurrence>(
  parseRecurrenceRule(model.value.recurrenceRule) || {
    frequency: "WEEKLY",
    interval: 1,
    byDays: [],
    until: null,
    count: null,
  },
);

const repeatDays = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
const dayLabels: Record<string, string> = {
  MO: "Mon",
  TU: "Tue",
  WE: "Wed",
  TH: "Thu",
  FR: "Fri",
  SA: "Sat",
  SU: "Sun",
};

function toggleDay(day: string) {
  const current = parsedRecurrence.value.byDays;
  if (current.includes(day)) {
    parsedRecurrence.value.byDays = current.filter((d) => d !== day);
  } else {
    parsedRecurrence.value.byDays = [...current, day];
  }
  syncRecurrenceToModel();
}

function syncRecurrenceToModel() {
  if (!isRepeating.value) {
    model.value.recurrenceRule = "";
    return;
  }
  model.value.recurrenceRule = buildRecurrenceRule(parsedRecurrence.value) || "";
}

watch(
  () => model.value.recurrenceRule,
  (newVal) => {
    if (newVal) {
      isRepeating.value = true;
      const parsed = parseRecurrenceRule(newVal);
      if (parsed) parsedRecurrence.value = parsed;
    }
  },
);

watch(isRepeating, (newVal) => {
  if (newVal && parsedRecurrence.value.byDays.length === 0) {
    if (model.value.date) {
      const d = new Date(model.value.date + "T12:00:00");
      const jsDay = d.getDay();
      const dayCodes = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];
      parsedRecurrence.value.byDays = [dayCodes[jsDay]];
    }
  }
  syncRecurrenceToModel();
});
</script>

<template>
  <div class="surface-card event-section">
    <div class="section-header">
      <div class="section-title-wrap">
        <button
          type="button"
          class="btn-back-nav"
          title="Back to event summary"
          @click="emit('back')"
        >
          <svg
            class="back-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="19" y1="12" x2="5" y2="12"></line>
            <polyline points="12 19 5 12 12 5"></polyline>
          </svg>
          <span>Back</span>
        </button>

        <div>
          <div class="heading-with-index">
            <h2 class="section-heading">Edit Event Details</h2>
            <span v-if="totalEvents && totalEvents > 1" class="event-counter-chip">
              {{ (currentIndex ?? 0) + 1 }} of {{ totalEvents }}
            </span>
          </div>
          <p class="section-subheading">Adjust fields and save or add to calendar</p>
        </div>
      </div>

      <div class="confidence-pill-wrap">
        <span class="badge-pill badge-pill-confidence">{{ confidencePercent }}% match</span>
      </div>
    </div>

    <!-- Grouped Form Fields -->
    <div class="grouped-form">
      <!-- Event Title Field -->
      <div class="field-item">
        <label class="field-label" for="event-title">Title</label>
        <input
          id="event-title"
          v-model="model.title"
          type="text"
          class="field-input field-input-bold"
          placeholder="Event name"
        />
      </div>

      <!-- Date & All-Day Switch Row -->
      <div class="field-row">
        <div class="field-item flex-grow">
          <label class="field-label" for="event-date">Date</label>
          <input
            id="event-date"
            v-model="model.date"
            type="date"
            class="field-input field-input-date"
          />
        </div>

        <div class="field-item field-item-toggle">
          <label class="toggle-control" for="event-allday">
            <span class="toggle-label-text">All-day</span>
            <div class="switch-wrap">
              <input
                id="event-allday"
                v-model="model.isAllDay"
                type="checkbox"
                class="switch-input"
              />
              <span class="switch-slider"></span>
            </div>
          </label>
        </div>
      </div>

      <!-- Start / End Time Row (if not all-day) -->
      <div v-if="!model.isAllDay" class="time-grid">
        <div class="field-item">
          <label class="field-label" for="event-start">Starts</label>
          <input
            id="event-start"
            v-model="model.startTime"
            type="time"
            class="field-input field-input-time"
          />
        </div>

        <div class="field-item">
          <label class="field-label" for="event-end">Ends</label>
          <input
            id="event-end"
            v-model="model.endTime"
            type="time"
            class="field-input field-input-time"
          />
        </div>
      </div>

      <!-- Location Field -->
      <div class="field-item">
        <label class="field-label" for="event-location">Location</label>
        <div class="input-icon-shell">
          <svg
            class="input-inline-icon"
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
          <input
            id="event-location"
            v-model="model.location"
            type="text"
            class="field-input field-input-with-icon"
            placeholder="Venue, address, or link"
          />
        </div>
      </div>
      <!-- URL / Link Field -->
      <div class="field-item">
        <label class="field-label" for="event-url">URL / Meeting Link</label>
        <div class="input-icon-shell">
          <svg
            class="input-leading-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
          </svg>
          <input
            id="event-url"
            v-model="model.url"
            type="url"
            class="field-input field-input-with-icon"
            placeholder="e.g. https://tufts.zoom.us/..."
          />
        </div>
      </div>
      <!-- Recurrence / Repeat Section -->
      <div class="field-item recurrence-card-section">
        <div class="toggle-control-row">
          <label class="toggle-control" for="event-repeats-toggle">
            <span class="toggle-label-text">Repeating Class / Event</span>
            <div class="switch-wrap">
              <input
                id="event-repeats-toggle"
                v-model="isRepeating"
                type="checkbox"
                class="switch-input"
              />
              <span class="switch-slider"></span>
            </div>
          </label>
        </div>

        <div v-if="isRepeating" class="recurrence-subform">
          <div class="days-selector-label">Repeats on days:</div>
          <div class="days-chip-group">
            <button
              v-for="d in repeatDays"
              :key="d"
              type="button"
              class="day-chip-btn"
              :class="{ 'day-chip-active': parsedRecurrence.byDays.includes(d) }"
              @click="toggleDay(d)"
            >
              {{ dayLabels[d] }}
            </button>
          </div>

          <div class="field-item until-field-item">
            <label class="field-label" for="event-until-date">End Repeat Date (Optional)</label>
            <input
              id="event-until-date"
              v-model="parsedRecurrence.until"
              type="date"
              class="field-input field-input-date"
              @change="syncRecurrenceToModel"
            />
          </div>
        </div>
      </div>

      <!-- Description & Notes Field -->
      <div class="field-item">
        <label class="field-label" for="event-description">Notes & Description</label>
        <textarea
          id="event-description"
          v-model="model.description"
          class="field-textarea"
          rows="3"
          placeholder="Performers, details, notes..."
        ></textarea>
      </div>

      <!-- Destination Calendar Selection -->
      <div v-if="availableCalendars && availableCalendars.length > 0" class="field-item">
        <label class="field-label" for="target-calendar">Save to Calendar</label>
        <div class="select-wrapper">
          <select
            id="target-calendar"
            v-model="selectedCalendarId"
            class="field-input field-select"
          >
            <option value="">Default Calendar (System)</option>
            <option v-for="cal in availableCalendars" :key="cal.id" :value="cal.id">
              {{ cal.title }} {{ cal.source_title ? `(${cal.source_title})` : "" }}
            </option>
          </select>
        </div>
      </div>
    </div>

    <!-- Event Action Cluster -->
    <!-- Event Action Cluster -->
    <div class="event-actions-flow">
      <!-- Primary Action Button (routed by defaultTarget preference) -->
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
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
          <polyline points="15 3 21 3 21 9"></polyline>
          <line x1="10" y1="14" x2="21" y2="3"></line>
        </svg>
        <span>Add to Google Calendar</span>
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
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
          <polyline points="7 10 12 15 17 10"></polyline>
          <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>
        <span>Export .ics File</span>
      </button>

      <button
        v-else
        type="button"
        class="btn-touch btn-touch-calendar"
        :disabled="isAddingToCalendar"
        @click="emit('addToCalendar')"
      >
        <template v-if="isAddingToCalendar">
          <div class="spinner-circle spinner-light"></div>
          <span>Adding to Calendar...</span>
        </template>
        <template v-else>
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="12" y1="11" x2="12" y2="17"></line>
            <line x1="9" y1="14" x2="15" y2="14"></line>
          </svg>
          <span>Add to Calendar</span>
        </template>
      </button>

      <div class="secondary-button-row">
        <button
          v-if="defaultTarget !== 'native'"
          type="button"
          class="btn-touch btn-touch-outline"
          :disabled="isAddingToCalendar"
          @click="emit('addToCalendar')"
        >
          <svg
            class="btn-icon-sm"
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
          <span>Add to Calendar</span>
        </button>

        <button
          v-if="defaultTarget !== 'google'"
          type="button"
          class="btn-touch btn-touch-outline btn-google-cal"
          title="Add to Google Calendar in browser"
          @click="emit('openGoogleCalendar')"
        >
          <svg
            class="btn-icon-sm"
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
            class="btn-icon-sm"
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
          <span>Export .ics</span>
        </button>

        <button
          type="button"
          class="btn-touch btn-touch-outline btn-done-editing"
          @click="emit('back')"
        >
          <svg
            class="btn-icon-sm"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span>Done</span>
        </button>

        <button
          type="button"
          class="btn-touch btn-touch-outline"
          :class="{ 'is-copied': copiedSummary }"
          @click="emit('copySummary')"
        >
          <template v-if="copiedSummary">
            <svg
              class="btn-icon-sm"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
            <span>Copied!</span>
          </template>
          <template v-else>
            <svg
              class="btn-icon-sm"
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
            <span>Copy</span>
          </template>
        </button>
        <button
          type="button"
          class="btn-touch btn-touch-outline btn-try-again"
          :disabled="isReparsing"
          title="Re-parse with AI model"
          @click="emit('tryAgain')"
        >
          <template v-if="isReparsing">
            <div class="spinner-circle spinner-dark"></div>
            <span>Trying again...</span>
          </template>
          <template v-else>
            <svg
              class="btn-icon-sm"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"></path>
              <path d="M3 3v5h5"></path>
              <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"></path>
              <path d="M16 21h5v-5"></path>
            </svg>
            <span>Try again</span>
          </template>
        </button>
      </div>
      <div v-if="totalEvents && totalEvents > 1" class="delete-action-row">
        <button type="button" class="btn-delete-event" @click="emit('remove')">
          <svg
            class="delete-icon"
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
          <span>Remove this event from list</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid var(--border-card-subtle);
}

.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}

.section-icon-bubble {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.section-icon {
  width: 20px;
  height: 20px;
}

.section-heading {
  font-size: 1.15rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-primary);
  letter-spacing: -0.015em;
  line-height: 1.2;
}

.section-subheading {
  font-size: 0.82rem;
  color: var(--text-secondary);
  margin: 0.15rem 0 0;
  line-height: 1.3;
}

.confidence-pill-wrap {
  flex-shrink: 0;
}

/* Grouped Form Fields */
.grouped-form {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.field-item {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.field-row {
  display: flex;
  align-items: flex-end;
  gap: 0.75rem;
}

.flex-grow {
  flex: 1;
  min-width: 0;
}

.time-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.field-label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.01em;
}

.field-input,
.field-textarea {
  width: 100%;
  box-sizing: border-box;
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-input);
  border-radius: var(--radius-input);
  padding: 0.75rem 0.9rem;
  font-size: 0.95rem;
  font-family: inherit;
  transition: all 0.15s ease;
  -webkit-appearance: none;
  appearance: none;
}

.field-input-bold {
  font-size: 1.05rem;
  font-weight: 600;
}

.field-input-date,
.field-input-time {
  min-height: 44px;
  color-scheme: light dark;
}

.field-input:focus,
.field-textarea:focus {
  outline: none;
  border-color: var(--border-input-focus);
  background: var(--bg-input-focus);
  box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.12);
}

.field-textarea {
  resize: vertical;
  line-height: 1.45;
}

.input-icon-shell {
  position: relative;
  display: flex;
  align-items: center;
}

.input-inline-icon {
  position: absolute;
  left: 0.85rem;
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  pointer-events: none;
}

.field-input-with-icon {
  padding-left: 2.35rem;
}

/* iOS-Style Toggle Switch */
.field-item-toggle {
  justify-content: flex-end;
  padding-bottom: 0.35rem;
  flex-shrink: 0;
}

.toggle-control {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  cursor: pointer;
  user-select: none;
}

.toggle-label-text {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.switch-wrap {
  position: relative;
  width: 44px;
  height: 26px;
}

.switch-input {
  opacity: 0;
  width: 0;
  height: 0;
  position: absolute;
}

.switch-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border-input);
  transition: 0.25s ease;
  border-radius: 34px;
}

.switch-slider:before {
  position: absolute;
  content: "";
  height: 20px;
  width: 20px;
  left: 3px;
  bottom: 3px;
  background-color: #ffffff;
  transition: 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.switch-input:checked + .switch-slider {
  background-color: var(--accent-primary);
}

.switch-input:checked + .switch-slider:before {
  transform: translateX(18px);
}

/* Action Buttons Cluster */
.event-actions-flow {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-top: 0.25rem;
}

.secondary-button-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 0.5rem;
}

@media (max-width: 480px) {
  .secondary-button-row {
    grid-template-columns: 1fr;
  }
}

.btn-back-nav {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--accent-primary);
  padding: 0.4rem 0.65rem;
  border-radius: 8px;
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.btn-back-nav:hover {
  background: var(--border-input);
}

.back-icon {
  width: 14px;
  height: 14px;
}

.heading-with-index {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.event-counter-chip {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--accent-primary);
  background: var(--accent-primary-light);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
}

.recurrence-card-section {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-card);
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.toggle-control-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.recurrence-subform {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding-top: 0.5rem;
  border-top: 1px dashed var(--border-subtle);
}

.days-selector-label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.days-chip-group {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.day-chip-btn {
  padding: 0.35rem 0.65rem;
  font-size: 0.8rem;
  font-weight: 600;
  border-radius: 999px;
  border: 1px solid var(--border-input);
  background: var(--bg-input);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.day-chip-btn:hover {
  background: var(--bg-card-elevated);
  border-color: var(--accent-primary);
  color: var(--text-primary);
}

.day-chip-btn.day-chip-active {
  background: var(--accent-primary);
  border-color: var(--accent-primary);
  color: #fff;
}

.until-field-item {
  margin-top: 0.25rem;
}
.btn-done-editing {
  color: var(--accent-primary);
  font-weight: 700;
}

.delete-action-row {
  display: flex;
  justify-content: center;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border-card-subtle);
}

.btn-delete-event {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: transparent;
  border: none;
  color: #ff3b30;
  font-size: 0.82rem;
  font-weight: 600;
  padding: 0.4rem 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-delete-event:hover {
  background: rgba(255, 59, 48, 0.1);
}

.delete-icon {
  width: 14px;
  height: 14px;
}
</style>
