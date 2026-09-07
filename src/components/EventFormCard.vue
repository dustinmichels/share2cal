<script setup lang="ts">
import { computed } from "vue";
import type { EventFormData } from "../services/event";

const model = defineModel<EventFormData>({ required: true });

const props = defineProps<{
  confidence: number;
  isAddingToCalendar?: boolean;
  copiedSummary?: boolean;
}>();

const emit = defineEmits<{
  (e: "addToCalendar"): void;
  (e: "exportIcs"): void;
  (e: "copySummary"): void;
}>();

const confidencePercent = computed(() => Math.round(props.confidence * 100));
</script>

<template>
  <div class="surface-card event-section">
    <div class="section-header">
      <div class="section-title-wrap">
        <div class="section-icon-bubble">
          <svg
            class="section-icon"
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
        </div>
        <div>
          <h2 class="section-heading">Event Details</h2>
          <p class="section-subheading">Review and adjust before adding to calendar</p>
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
    </div>

    <!-- Event Action Cluster -->
    <div class="event-actions-flow">
      <button
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
        <button type="button" class="btn-touch btn-touch-outline" @click="emit('exportIcs')">
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
            <span>Copy Summary</span>
          </template>
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
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}
</style>
