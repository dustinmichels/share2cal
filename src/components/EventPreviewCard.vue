<script setup lang="ts">
import { computed } from "vue";
import {
  formatDateForDisplay,
  formatTimeForDisplay,
  formatRecurrenceForDisplay,
  type EventDetails,
} from "../services/event";
const props = defineProps<{
  events: EventDetails[];
  isAddingToCalendar?: boolean;
  copiedSummary?: boolean;
  addedIndices?: Set<number>;
}>();

const emit = defineEmits<{
  (e: "editEvent", index: number): void;
  (e: "addToCalendar"): void;
  (e: "exportIcs"): void;
  (e: "copySummary"): void;
  (e: "removeEvent", index: number): void;
}>();

const eventCount = computed(() => props.events.length);
const isMultiple = computed(() => eventCount.value > 1);

const overallConfidence = computed(() => {
  if (props.events.length === 0) return 0;
  const sum = props.events.reduce((acc, curr) => acc + (curr.confidence || 0.8), 0);
  return Math.round((sum / props.events.length) * 100);
});

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
    <div class="section-header">
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

      <div class="confidence-pill-wrap">
        <span class="badge-pill badge-pill-confidence">{{ overallConfidence }}% match</span>
      </div>
    </div>

    <!-- Events List / Grid -->
    <div class="events-preview-list">
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
          <div v-if="formatRecurrenceForDisplay(event.recurrence_rule)" class="meta-row meta-row-recurrence">
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
            <span class="meta-text meta-recurrence-text">{{ formatRecurrenceForDisplay(event.recurrence_rule) }}</span>
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
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Batch / Primary Actions -->
    <div class="preview-actions-flow">
      <button
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
          <span>{{
            isMultiple ? `Add All (${eventCount}) to Calendar` : "Add to Calendar"
          }}</span>
        </template>
        <template v-else>
          <div class="spinner-circle"></div>
          <span>Adding to Calendar...</span>
        </template>
      </button>

      <div class="secondary-actions-grid">
        <button
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
          <span>{{ isMultiple ? "Export All (.ics)" : "Export (.ics)" }}</span>
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
  transition: color 0.15s ease, transform 0.15s ease;
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

.preview-actions-flow {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-top: 0.25rem;
}

.secondary-actions-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
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
