<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { ArrowLeft, LoaderCircle } from "lucide-vue-next";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { extractTextFromBytes, type OcrResult } from "./services/ocr";
import {
  parseEventsFromText,
  downloadIcsFile,
  downloadMultiIcsFile,
  addEventToNativeCalendar,
  addEventsToNativeCalendar,
  getAvailableCalendars,
  openInGoogleCalendar,
  formatDateForDisplay,
  formatTimeForDisplay,
  extractDateInput,
  extractTimeInput,
  buildIsoFromDateTime,
  type EventDetails,
  type EventFormData,
  type CalendarInfo,
} from "./services/event";
import {
  getPendingSharedImage,
  clearPendingSharedImage,
  payloadToFile,
  loadImageFromPath,
} from "./services/share";
import { getModelStatuses, type ModelStatus } from "./services/model";
import {
  getStoredParsingMode,
  setStoredParsingMode,
  getStoredDefaultCalendarId,
  getStoredDefaultTarget,
  type ParsingMode,
  type CalendarTarget,
} from "./services/settings";
import UploadHub from "./components/UploadHub.vue";
import ImagePreviewCard from "./components/ImagePreviewCard.vue";
import EventPreviewCard from "./components/EventPreviewCard.vue";
import EventFormCard from "./components/EventFormCard.vue";
import OcrDrawer from "./components/OcrDrawer.vue";
import SettingsNavCard from "./components/SettingsNavCard.vue";
import SettingsView from "./components/SettingsView.vue";
const currentView = ref<"main" | "summary" | "settings">("main");
const parsingMode = ref<ParsingMode>(getStoredParsingMode());

function updateParsingMode(mode: ParsingMode) {
  parsingMode.value = mode;
  setStoredParsingMode(mode);
}

const modelStatuses = ref<ModelStatus[]>([]);
const hasLocalModel = computed(() => modelStatuses.value.some((m) => m.is_downloaded));
const defaultModel = computed(
  () =>
    modelStatuses.value.find((m) => m.is_downloaded) ||
    modelStatuses.value.find((m) => m.is_default),
);
const defaultModelName = computed(() => defaultModel.value?.name || null);

async function refreshModelStatus() {
  try {
    modelStatuses.value = await getModelStatuses();
  } catch (err) {
    console.warn("Failed to check model statuses in App:", err);
  }
}

const selectedFile = ref<File | null>(null);
const previewUrl = ref<string | null>(null);
const isProcessing = ref(false);
const errorMessage = ref<string | null>(null);
const ocrResult = ref<OcrResult | null>(null);
const eventsList = ref<EventDetails[]>([]);
const selectedEventIndex = ref<number | null>(null);
const addedEventIndices = ref<Set<number>>(new Set());

const shareNotification = ref<string | null>(null);
const isFromShareExtension = ref(false);
const copiedSummary = ref(false);

const availableCalendars = ref<CalendarInfo[]>([]);
const selectedCalendarId = ref<string>(getStoredDefaultCalendarId() || "");
const defaultCalendarTarget = ref<CalendarTarget>(getStoredDefaultTarget());

async function refreshCalendars() {
  try {
    defaultCalendarTarget.value = getStoredDefaultTarget();
    availableCalendars.value = await getAvailableCalendars();
    const storedId = getStoredDefaultCalendarId();
    if (storedId) {
      selectedCalendarId.value = storedId;
    }
  } catch (err) {
    console.warn("Failed to refresh calendars in App:", err);
  }
}

const isWindowDragging = ref(false);
let unlistenDragDrop: (() => void) | null = null;

async function handleDroppedPaths(paths: string[]) {
  if (!paths || paths.length === 0) return;
  const imageExtRegex = /\.(heic|heif|png|jpe?g|webp|bmp|gif|tiff?|svg)$/i;
  const targetPath = paths.find((p) => imageExtRegex.test(p)) || paths[0];

  try {
    const file = await loadImageFromPath(targetPath);
    if (file) {
      if (currentView.value !== "main") {
        currentView.value = "main";
      }
      setImageFile(file);
    } else {
      errorMessage.value = "Could not load the dropped file.";
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    errorMessage.value = `Failed to open dropped file: ${msg}`;
  }
}

function handleWindowDragOver(event: DragEvent) {
  event.preventDefault();
  isWindowDragging.value = true;
}

function handleWindowDragLeave(event: DragEvent) {
  event.preventDefault();
  if (!event.relatedTarget || event.relatedTarget === document.documentElement) {
    isWindowDragging.value = false;
  }
}

function handleWindowDrop(event: DragEvent) {
  event.preventDefault();
  isWindowDragging.value = false;
  const files = event.dataTransfer?.files;
  if (files && files.length > 0) {
    if (currentView.value !== "main") {
      currentView.value = "main";
    }
    setImageFile(files[0]);
  }
}

async function setupDragDrop() {
  if (isTauri()) {
    try {
      const webview = getCurrentWebview();
      unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isWindowDragging.value = true;
        } else if (event.payload.type === "leave") {
          isWindowDragging.value = false;
        } else if (event.payload.type === "drop") {
          isWindowDragging.value = false;
          await handleDroppedPaths(event.payload.paths);
        }
      });
    } catch (err) {
      console.warn("Failed to attach Tauri drag drop listener:", err);
    }
  }
}
const calendarDownloaded = ref(false);
const isAddingToCalendar = ref(false);
const calendarSuccessMessage = ref<string | null>(null);
const calendarErrorMessage = ref<string | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const cameraInputRef = ref<HTMLInputElement | null>(null);

// Editable event form model for the active edit screen
const eventForm = ref<EventFormData>({
  title: "",
  date: "",
  startTime: "",
  endTime: "",
  isAllDay: false,
  location: "",
  description: "",
  recurrenceRule: "",
});

function syncFormFromEvent(event: EventDetails) {
  eventForm.value = {
    title: event.title || "",
    date: extractDateInput(event.start_time),
    startTime: extractTimeInput(event.start_time, "12:00"),
    endTime: extractTimeInput(event.end_time, "13:00"),
    isAllDay: event.is_all_day,
    location: event.location || "",
    description: event.description || "",
    recurrenceRule: event.recurrence_rule || "",
  };
}

function openEditScreen(index: number) {
  if (index >= 0 && index < eventsList.value.length) {
    selectedEventIndex.value = index;
    syncFormFromEvent(eventsList.value[index]);
  }
}

function closeEditScreen() {
  if (selectedEventIndex.value !== null && selectedEventIndex.value < eventsList.value.length) {
    eventsList.value[selectedEventIndex.value] = getComposedEvent();
  }
  selectedEventIndex.value = null;
}

function handleRemoveEvent(index: number) {
  if (index >= 0 && index < eventsList.value.length) {
    eventsList.value.splice(index, 1);
    const nextAdded = new Set<number>();
    for (const addedIdx of addedEventIndices.value) {
      if (addedIdx < index) {
        nextAdded.add(addedIdx);
      } else if (addedIdx > index) {
        nextAdded.add(addedIdx - 1);
      }
    }
    addedEventIndices.value = nextAdded;
    if (selectedEventIndex.value === index) {
      selectedEventIndex.value = null;
    } else if (selectedEventIndex.value !== null && selectedEventIndex.value > index) {
      selectedEventIndex.value -= 1;
    }
  }
}

function handleRemoveCurrentEvent() {
  if (selectedEventIndex.value !== null) {
    handleRemoveEvent(selectedEventIndex.value);
    selectedEventIndex.value = null;
  }
}

function isImageFile(file: File): boolean {
  if (!file) return false;
  if (file.type && file.type.startsWith("image/")) return true;
  if (file.name && file.name.match(/\.(heic|heif|png|jpe?g|webp|bmp|gif|tiff?)$/i)) return true;
  if (!file.type || file.type === "application/octet-stream") {
    if (
      file.name &&
      file.name.match(/\.(pdf|txt|json|doc|docx|csv|zip|gz|tar|mp3|mp4|mov|avi)$/i)
    ) {
      return false;
    }
    return file.size > 0;
  }
  return false;
}

function setImageFile(file: File) {
  if (!isImageFile(file)) {
    errorMessage.value = "Please select a valid image file (PNG, JPEG, HEIF, WebP, etc.).";
    return;
  }
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }

  selectedFile.value = file;
  previewUrl.value = URL.createObjectURL(file);
  errorMessage.value = null;
  ocrResult.value = null;
  eventsList.value = [];
  selectedEventIndex.value = null;
  addedEventIndices.value = new Set();
  calendarDownloaded.value = false;
}

function handleFileInput(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    setImageFile(target.files[0]);
    target.value = "";
  }
}

function handleCameraInput(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    setImageFile(target.files[0]);
    target.value = "";
  }
}

function triggerFileUpload() {
  fileInputRef.value?.click();
}

function triggerCameraCapture() {
  cameraInputRef.value?.click();
}

function handlePaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items;
  if (!items) return;

  for (let i = 0; i < items.length; i++) {
    if (items[i].type.startsWith("image/")) {
      const file = items[i].getAsFile();
      if (file) {
        setImageFile(file);
        break;
      }
    }
  }
}

async function handleGo() {
  if (!selectedFile.value) return;

  currentView.value = "summary";
  errorMessage.value = null;
  isProcessing.value = true;
  calendarDownloaded.value = false;
  ocrResult.value = null;
  eventsList.value = [];
  selectedEventIndex.value = null;
  addedEventIndices.value = new Set();

  try {
    const arrayBuffer = await selectedFile.value.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);
    const res = await extractTextFromBytes(bytes);
    ocrResult.value = res;

    if (!res.text.trim()) {
      errorMessage.value =
        "No text was detected in this image. Try another photo with clearer text.";
    } else {
      // Parse multiple or single event details from the extracted OCR text
      const parsed = await parseEventsFromText(res.text);
      eventsList.value = parsed;
    }
  } catch (err: unknown) {
    errorMessage.value =
      err instanceof Error
        ? err.message
        : String(err) || "Failed to process OCR and event extraction on the selected image.";
  } finally {
    isProcessing.value = false;
  }
}

async function handleReparse() {
  if (!ocrResult.value?.text) return;
  try {
    const parsed = await parseEventsFromText(ocrResult.value.text);
    eventsList.value = parsed;
    selectedEventIndex.value = null;
    addedEventIndices.value = new Set();
  } catch (err: unknown) {
    errorMessage.value =
      err instanceof Error ? err.message : String(err) || "Failed to re-parse event details.";
  }
}

function getComposedEvent(): EventDetails {
  const startIso = eventForm.value.isAllDay
    ? `${eventForm.value.date}T00:00:00Z`
    : buildIsoFromDateTime(eventForm.value.date, eventForm.value.startTime);

  const endIso = eventForm.value.isAllDay
    ? `${eventForm.value.date}T23:59:59Z`
    : buildIsoFromDateTime(eventForm.value.date, eventForm.value.endTime);

  return {
    title: eventForm.value.title.trim() || "New Event",
    start_time: startIso || null,
    end_time: endIso || null,
    is_all_day: eventForm.value.isAllDay,
    location: eventForm.value.location.trim() || null,
    description: eventForm.value.description.trim() || null,
    recurrence_rule: eventForm.value.recurrenceRule?.trim() || null,
    confidence:
      selectedEventIndex.value !== null
        ? (eventsList.value[selectedEventIndex.value]?.confidence ?? 0.8)
        : 0.8,
    source:
      selectedEventIndex.value !== null
        ? (eventsList.value[selectedEventIndex.value]?.source ?? "deterministic")
        : "deterministic",
  };
}

async function handleBatchAddToCalendar() {
  if (eventsList.value.length === 0) return;
  isAddingToCalendar.value = true;
  calendarSuccessMessage.value = null;
  calendarErrorMessage.value = null;
  calendarDownloaded.value = false;

  try {
    const result = await addEventsToNativeCalendar(
      eventsList.value,
      selectedCalendarId.value || undefined,
    );
    if (result.success || result.addedCount > 0) {
      const newSet = new Set<number>();
      for (let i = 0; i < eventsList.value.length; i++) {
        newSet.add(i);
      }
      addedEventIndices.value = newSet;

      if (result.addedCount === eventsList.value.length) {
        calendarSuccessMessage.value =
          eventsList.value.length === 1
            ? `Event "${eventsList.value[0].title}" was added directly to your Calendar!`
            : `All ${result.addedCount} events were added directly to your Calendar!`;
      } else {
        calendarSuccessMessage.value = `Added ${result.addedCount} of ${eventsList.value.length} events to your Calendar.`;
      }

      setTimeout(() => {
        calendarSuccessMessage.value = null;
      }, 5000);
    } else {
      calendarErrorMessage.value =
        result.errors?.join("; ") || "Failed to add events to native calendar.";
      downloadMultiIcsFile(eventsList.value);
      setTimeout(() => {
        calendarErrorMessage.value = null;
      }, 6000);
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    calendarErrorMessage.value = msg;
    downloadMultiIcsFile(eventsList.value);
  } finally {
    isAddingToCalendar.value = false;
  }
}

async function handleSingleAddToCalendar() {
  const event = getComposedEvent();
  if (selectedEventIndex.value !== null) {
    eventsList.value[selectedEventIndex.value] = event;
  }
  isAddingToCalendar.value = true;
  calendarSuccessMessage.value = null;
  calendarErrorMessage.value = null;
  calendarDownloaded.value = false;

  try {
    const result = await addEventToNativeCalendar(
      event,
      selectedCalendarId.value || undefined,
    );
    if (result.success) {
      if (selectedEventIndex.value !== null) {
        addedEventIndices.value.add(selectedEventIndex.value);
      }
      calendarSuccessMessage.value = `Event "${event.title}" was added directly to your Calendar!`;
      setTimeout(() => {
        calendarSuccessMessage.value = null;
      }, 5000);
    } else {
      calendarErrorMessage.value = result.error || "Failed to add event to native calendar.";
      downloadIcsFile(event);
      setTimeout(() => {
        calendarErrorMessage.value = null;
      }, 6000);
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    calendarErrorMessage.value = msg;
    downloadIcsFile(event);
  } finally {
    isAddingToCalendar.value = false;
  }
}

async function handleOpenGoogleCalendar(index?: number) {
  const targetEvent =
    index !== undefined && eventsList.value[index]
      ? eventsList.value[index]
      : selectedEventIndex.value !== null
        ? getComposedEvent()
        : eventsList.value[0];
  if (!targetEvent) return;

  try {
    await openInGoogleCalendar(targetEvent);
    calendarSuccessMessage.value = `Opening "${targetEvent.title}" in Google Calendar...`;
    setTimeout(() => {
      calendarSuccessMessage.value = null;
    }, 4000);
  } catch (err) {
    calendarErrorMessage.value = `Could not open Google Calendar: ${String(err)}`;
    setTimeout(() => {
      calendarErrorMessage.value = null;
    }, 5000);
  }
}

function handleExportIcs() {
  if (eventsList.value.length === 0) return;
  downloadMultiIcsFile(eventsList.value);
  calendarDownloaded.value = true;
  setTimeout(() => {
    calendarDownloaded.value = false;
  }, 4000);
}

function handleSingleExportIcs() {
  const event = getComposedEvent();
  if (selectedEventIndex.value !== null) {
    eventsList.value[selectedEventIndex.value] = event;
  }
  downloadIcsFile(event);
  calendarDownloaded.value = true;
  setTimeout(() => {
    calendarDownloaded.value = false;
  }, 4000);
}

async function copySummary() {
  if (eventsList.value.length === 0) return;
  const lines: string[] = [];

  if (eventsList.value.length === 1) {
    const event = selectedEventIndex.value !== null ? getComposedEvent() : eventsList.value[0];
    lines.push(`📅 ${event.title}`);
    lines.push(
      event.is_all_day
        ? `Date: ${formatDateForDisplay(event.start_time)} (All day)`
        : `Date & Time: ${formatDateForDisplay(event.start_time)} (${formatTimeForDisplay(event.start_time)} - ${formatTimeForDisplay(event.end_time)})`,
    );
    if (event.location) lines.push(`📍 Location: ${event.location}`);
    if (event.description) lines.push(`📝 Notes: ${event.description}`);
  } else {
    lines.push(`📋 Schedule Summary (${eventsList.value.length} Events)`);
    lines.push("");
    eventsList.value.forEach((ev, idx) => {
      lines.push(`${idx + 1}. ${ev.title}`);
      lines.push(
        ev.is_all_day
          ? `   Date: ${formatDateForDisplay(ev.start_time)} (All day)`
          : `   Time: ${formatDateForDisplay(ev.start_time)} (${formatTimeForDisplay(ev.start_time)} - ${formatTimeForDisplay(ev.end_time)})`,
      );
      if (ev.location) lines.push(`   Location: ${ev.location}`);
      if (ev.description) lines.push(`   Notes: ${ev.description}`);
      lines.push("");
    });
  }

  try {
    await navigator.clipboard.writeText(lines.join("\n").trim());
    copiedSummary.value = true;
    setTimeout(() => {
      copiedSummary.value = false;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy summary:", err);
  }
}

async function copySingleSummary() {
  const event = getComposedEvent();
  const lines = [
    `📅 ${event.title}`,
    event.is_all_day
      ? `Date: ${eventForm.value.date} (All day)`
      : `Date & Time: ${eventForm.value.date} (${eventForm.value.startTime} - ${eventForm.value.endTime})`,
  ];
  if (event.location) lines.push(`📍 Location: ${event.location}`);
  if (event.description) lines.push(`📝 Notes: ${event.description}`);

  try {
    await navigator.clipboard.writeText(lines.join("\n"));
    copiedSummary.value = true;
    setTimeout(() => {
      copiedSummary.value = false;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy single summary:", err);
  }
}

function handleReset() {
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
  selectedFile.value = null;
  previewUrl.value = null;
  ocrResult.value = null;
  eventsList.value = [];
  selectedEventIndex.value = null;
  addedEventIndices.value = new Set();
  errorMessage.value = null;
  calendarDownloaded.value = false;
  calendarSuccessMessage.value = null;
  calendarErrorMessage.value = null;
  isFromShareExtension.value = false;
  shareNotification.value = null;
}

async function checkPendingShare() {
  try {
    const pending = await getPendingSharedImage(true);
    if (pending && pending.bytes && pending.bytes.length > 0) {
      const file = payloadToFile(pending);
      if (file) {
        isFromShareExtension.value = true;
        shareNotification.value = `Received flyer from iOS Share Sheet: ${pending.file_name}`;
        setImageFile(file);
        await clearPendingSharedImage();
        // Automatically process OCR and event extraction for smooth mobile experience
        await handleGo();
      }
    }
  } catch (err) {
    console.warn("Could not check pending shared image:", err);
  }
}

function handleVisibilityChange() {
  if (document.visibilityState === "visible") {
    checkPendingShare();
  }
}

onMounted(() => {
  window.addEventListener("paste", handlePaste);
  window.addEventListener("focus", checkPendingShare);
  document.addEventListener("visibilitychange", handleVisibilityChange);
  window.addEventListener("dragover", handleWindowDragOver);
  window.addEventListener("dragleave", handleWindowDragLeave);
  window.addEventListener("drop", handleWindowDrop);
  setupDragDrop();
  checkPendingShare();
  refreshModelStatus();
  refreshCalendars();
});

onUnmounted(() => {
  window.removeEventListener("paste", handlePaste);
  window.removeEventListener("focus", checkPendingShare);
  document.removeEventListener("visibilitychange", handleVisibilityChange);
  window.removeEventListener("dragover", handleWindowDragOver);
  window.removeEventListener("dragleave", handleWindowDragLeave);
  window.removeEventListener("drop", handleWindowDrop);
  if (unlistenDragDrop) {
    unlistenDragDrop();
    unlistenDragDrop = null;
  }
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
});
</script>

<template>
  <div class="app-layout">
    <main class="app-container">
      <!-- App Header -->
      <header v-if="currentView === 'main'" class="app-header">
        <div class="logo-badge">
          <svg
            class="logo-icon"
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
            <path d="M8 18h.01"></path>
            <path d="M12 18h.01"></path>
          </svg>
        </div>
        <h1 class="app-title">Share2Cal</h1>
        <p class="app-tagline">Turn flyers and invitations into calendar events in seconds</p>
      </header>

      <!-- Share Notification Banner -->
      <div v-if="shareNotification" class="toast-banner toast-info">
        <div class="toast-icon-wrap">
          <svg
            class="toast-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path>
            <polyline points="16 6 12 2 8 6"></polyline>
            <line x1="12" y1="2" x2="12" y2="15"></line>
          </svg>
        </div>
        <span class="toast-text">{{ shareNotification }}</span>
        <button
          type="button"
          class="btn-toast-close"
          aria-label="Close notification"
          @click="shareNotification = null"
        >
          ✕
        </button>
      </div>

      <!-- Native Calendar Success Toast -->
      <div v-if="calendarSuccessMessage" class="toast-banner toast-success">
        <div class="toast-icon-wrap">
          <svg
            class="toast-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Added to Calendar</span>
          <span class="toast-text">{{ calendarSuccessMessage }}</span>
        </div>
        <button
          type="button"
          class="btn-toast-close"
          aria-label="Close"
          @click="calendarSuccessMessage = null"
        >
          ✕
        </button>
      </div>

      <!-- Calendar Warning / Fallback Toast -->
      <div v-if="calendarErrorMessage" class="toast-banner toast-warning">
        <div class="toast-icon-wrap">
          <svg
            class="toast-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Calendar Notice</span>
          <span class="toast-text">{{ calendarErrorMessage }} (.ics exported)</span>
        </div>
        <button
          type="button"
          class="btn-toast-close"
          aria-label="Close"
          @click="calendarErrorMessage = null"
        >
          ✕
        </button>
      </div>

      <!-- ICS Export Success Toast -->
      <div v-if="calendarDownloaded" class="toast-banner toast-success">
        <div class="toast-icon-wrap">
          <svg
            class="toast-icon"
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
        </div>
        <div class="toast-body">
          <span class="toast-heading">Calendar File (.ics) Exported</span>
          <span class="toast-text"
            >Open the downloaded file to add to Apple Calendar, Google, or Outlook.</span
          >
        </div>
        <button
          type="button"
          class="btn-toast-close"
          aria-label="Close"
          @click="calendarDownloaded = false"
        >
          ✕
        </button>
      </div>

      <!-- General Error Toast -->
      <div v-if="errorMessage" class="toast-banner toast-error">
        <div class="toast-icon-wrap">
          <svg
            class="toast-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Processing Notice</span>
          <span class="toast-text">{{ errorMessage }}</span>
        </div>
        <button
          type="button"
          class="btn-toast-close"
          aria-label="Close"
          @click="errorMessage = null"
        >
          ✕
        </button>
      </div>

      <!-- Hidden file inputs -->
      <input
        ref="fileInputRef"
        type="file"
        accept="image/*,.heic,.heif,.png,.jpg,.jpeg,.webp,.tiff"
        class="hidden-input"
        @change="handleFileInput"
      />
      <input
        ref="cameraInputRef"
        type="file"
        accept="image/*"
        capture="environment"
        class="hidden-input"
        @change="handleCameraInput"
      />

      <!-- VIEW 1: Image Upload -->
      <div v-if="currentView === 'main'" class="main-view-flow">
        <div v-if="!selectedFile" class="empty-hub-flow">
          <UploadHub
            :is-dragging="isWindowDragging"
            @select-file="setImageFile"
            @choose-file="triggerFileUpload"
            @take-photo="triggerCameraCapture"
          />
        </div>

        <section v-else class="content-flow">
          <ImagePreviewCard
            :file="selectedFile"
            :preview-url="previewUrl"
            :is-processing="isProcessing"
            :is-from-share-extension="isFromShareExtension"
            :has-event="eventsList.length > 0"
            @scan="handleGo"
            @remove="handleReset"
            @choose-another="triggerFileUpload"
            @take-photo="triggerCameraCapture"
          />
        </section>

        <!-- Prominent Settings Navigation Button (Below Main Image & Action Area) -->
        <SettingsNavCard
          :parsing-mode="parsingMode"
          :has-local-model="hasLocalModel"
          :default-model-name="defaultModelName"
          @open-settings="currentView = 'settings'"
        />
      </div>

      <!-- VIEW 2: Event Summary -->
      <section v-else-if="currentView === 'summary'" class="summary-view-flow">
        <div class="summary-nav">
          <button
            type="button"
            class="btn-touch btn-touch-outline summary-back-button"
            @click="currentView = 'main'"
          >
            <ArrowLeft class="btn-icon" :stroke-width="2.2" />
            <span>Back to image upload</span>
          </button>
        </div>

        <section
          v-if="isProcessing"
          class="surface-card extraction-loading-card"
          aria-live="polite"
          aria-busy="true"
        >
          <LoaderCircle class="summary-loading-icon" :stroke-width="2" aria-hidden="true" />
          <div class="loading-copy">
            <h2 class="section-heading">Scanning and extracting</h2>
            <p class="section-subheading">Reading the image and building your event summary.</p>
          </div>
        </section>

        <!-- PREVIEW VIEW: Event Summary / List of Events -->
        <EventPreviewCard
          v-else-if="eventsList.length > 0 && selectedEventIndex === null"
          v-model:selected-calendar-id="selectedCalendarId"
          :events="eventsList"
          :available-calendars="availableCalendars"
          :default-target="defaultCalendarTarget"
          :is-adding-to-calendar="isAddingToCalendar"
          :copied-summary="copiedSummary"
          :added-indices="addedEventIndices"
          @edit-event="openEditScreen"
          @add-to-calendar="handleBatchAddToCalendar"
          @open-google-calendar="handleOpenGoogleCalendar"
          @export-ics="handleExportIcs"
          @copy-summary="copySummary"
          @remove-event="handleRemoveEvent"
        />

        <!-- EDIT VIEW: Detailed Event Editing Screen -->
        <EventFormCard
          v-else-if="eventsList.length > 0 && selectedEventIndex !== null"
          v-model="eventForm"
          v-model:selected-calendar-id="selectedCalendarId"
          :confidence="eventsList[selectedEventIndex]?.confidence ?? 0.8"
          :available-calendars="availableCalendars"
          :default-target="defaultCalendarTarget"
          :is-adding-to-calendar="isAddingToCalendar"
          :copied-summary="copiedSummary"
          :current-index="selectedEventIndex"
          :total-events="eventsList.length"
          @back="closeEditScreen"
          @add-to-calendar="handleSingleAddToCalendar"
          @open-google-calendar="() => handleOpenGoogleCalendar(selectedEventIndex ?? undefined)"
          @export-ics="handleSingleExportIcs"
          @copy-summary="copySingleSummary"
          @remove="handleRemoveCurrentEvent"
        />

        <section v-else class="surface-card extraction-empty-card">
          <h2 class="section-heading">No event summary yet</h2>
          <p class="section-subheading">
            Go back to image upload and scan a clearer flyer or screenshot.
          </p>
        </section>

        <!-- Collapsible Raw OCR Diagnostics Drawer -->
        <OcrDrawer
          v-if="!isProcessing && ocrResult"
          :ocr-result="ocrResult"
          @reparse="handleReparse"
        />
      </section>

      <!-- VIEW 3: Dedicated Settings Page -->
      <SettingsView
        v-else-if="currentView === 'settings'"
        :parsing-mode="parsingMode"
        @update:parsing-mode="updateParsingMode"
        @back="currentView = 'main'; refreshCalendars();"
        @models-updated="() => { refreshModelStatus(); refreshCalendars(); }"
      />

      <!-- Drop Overlay for when an image is already selected or on other views -->
      <div v-if="isWindowDragging && selectedFile" class="drop-target-overlay">
        <div class="drop-target-content">
          <div class="drop-target-icon-bubble">
            <svg
              class="drop-target-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="17 8 12 3 7 8"></polyline>
              <line x1="12" y1="3" x2="12" y2="15"></line>
            </svg>
          </div>
          <h3 class="drop-target-title">Drop new image to replace</h3>
          <p class="drop-target-desc">Supports PNG, JPG, HEIF, WebP</p>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
/* Theme Variables & Layout Foundation */
.app-layout {
  --bg-page: #f2f2f7;
  --bg-card: #ffffff;
  --bg-card-elevated: #ffffff;
  --bg-input: #f8f9fa;
  --bg-input-focus: #ffffff;
  --border-card: rgba(0, 0, 0, 0.06);
  --border-card-subtle: rgba(0, 0, 0, 0.04);
  --border-input: #e2e8f0;
  --border-input-focus: #007aff;
  --text-primary: #111827;
  --text-secondary: #4b5563;
  --text-tertiary: #9ca3af;
  --accent-primary: #007aff;
  --accent-primary-hover: #0066d6;
  --accent-green: #34c759;
  --accent-green-hover: #2db84d;
  --accent-danger: #ff3b30;
  --accent-danger-bg: #fee2e2;
  --shadow-subtle: 0 2px 8px rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.02);
  --shadow-card: 0 4px 20px -2px rgba(0, 0, 0, 0.05), 0 2px 6px -1px rgba(0, 0, 0, 0.02);
  --shadow-primary-btn: 0 4px 14px rgba(0, 122, 255, 0.25);
  --shadow-green-btn: 0 4px 14px rgba(52, 199, 89, 0.28);
  --radius-card: 20px;
  --radius-input: 12px;
  --radius-btn: 14px;

  width: 100%;
  min-height: 100vh;
  background-color: var(--bg-page);
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  align-items: center;
}

@media (prefers-color-scheme: dark) {
  .app-layout {
    --bg-page: #0b0f17;
    --bg-card: #161b26;
    --bg-card-elevated: #1e2433;
    --bg-input: #10141d;
    --bg-input-focus: #0d1017;
    --border-card: rgba(255, 255, 255, 0.08);
    --border-card-subtle: rgba(255, 255, 255, 0.05);
    --border-input: #283141;
    --border-input-focus: #388bfd;
    --text-primary: #f3f4f6;
    --text-secondary: #9ca3af;
    --text-tertiary: #6b7280;
    --accent-primary: #0a84ff;
    --accent-primary-hover: #0071e3;
    --accent-green: #30d158;
    --accent-green-hover: #28b84d;
    --accent-danger: #ff453a;
    --accent-danger-bg: rgba(255, 69, 58, 0.15);
    --shadow-subtle: 0 2px 8px rgba(0, 0, 0, 0.3);
    --shadow-card: 0 4px 24px -2px rgba(0, 0, 0, 0.5), 0 2px 8px -1px rgba(0, 0, 0, 0.3);
    --shadow-primary-btn: 0 4px 16px rgba(10, 132, 255, 0.35);
    --shadow-green-btn: 0 4px 16px rgba(48, 209, 88, 0.35);
  }
}

.app-container {
  width: 100%;
  max-width: 580px;
  margin: 0 auto;
  padding: max(1.25rem, env(safe-area-inset-top)) max(1rem, env(safe-area-inset-right))
    max(2rem, env(safe-area-inset-bottom)) max(1rem, env(safe-area-inset-left));
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* Header */
.app-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 0.5rem 0.5rem 0.25rem;
}

/* Main View Flow */
.main-view-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  width: 100%;
}

.empty-hub-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.content-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.summary-view-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  width: 100%;
}

.summary-nav {
  display: flex;
  align-items: center;
  width: 100%;
}

.summary-back-button {
  width: auto;
  min-height: 44px;
  padding: 0.7rem 0.9rem;
}

.extraction-loading-card,
.extraction-empty-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 1rem;
  min-height: 260px;
  padding: 2.5rem 1.5rem;
}

.summary-loading-icon {
  width: 44px;
  height: 44px;
  color: var(--accent-primary);
  animation: summaryLoadingSpin 1s linear infinite;
}

.loading-copy {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.extraction-loading-card .section-heading,
.extraction-empty-card .section-heading {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  line-height: 1.25;
}

.extraction-loading-card .section-subheading,
.extraction-empty-card .section-subheading {
  font-size: 0.84rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.4;
}

@keyframes summaryLoadingSpin {
  to {
    transform: rotate(360deg);
  }
}

.logo-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, #007aff 0%, #5856d6 100%);
  border-radius: 14px;
  color: #ffffff;
  margin-bottom: 0.75rem;
  box-shadow: 0 4px 16px rgba(0, 122, 255, 0.3);
}

.logo-icon {
  width: 26px;
  height: 26px;
}

.app-title {
  font-size: 1.75rem;
  font-weight: 800;
  letter-spacing: -0.03em;
  margin: 0 0 0.35rem 0;
  color: var(--text-primary);
  line-height: 1.2;
}

.app-tagline {
  font-size: 0.92rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.4;
  max-width: 380px;
}

/* Hidden Inputs */
.hidden-input {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
  opacity: 0;
  pointer-events: none;
}

/* Toast Banners */
.toast-banner {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.85rem 1rem;
  border-radius: 14px;
  font-size: 0.88rem;
  animation: toastSlideDown 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  box-shadow: var(--shadow-subtle);
}

@keyframes toastSlideDown {
  from {
    opacity: 0;
    transform: translateY(-6px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.toast-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.toast-icon {
  width: 20px;
  height: 20px;
}

.toast-body {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  flex: 1;
  min-width: 0;
}

.toast-heading {
  font-weight: 700;
  font-size: 0.88rem;
  line-height: 1.2;
}

.toast-text {
  font-size: 0.84rem;
  line-height: 1.35;
  word-break: break-word;
}

.btn-toast-close {
  background: transparent;
  border: none;
  color: inherit;
  opacity: 0.6;
  font-size: 1rem;
  cursor: pointer;
  padding: 0.25rem 0.4rem;
  border-radius: 6px;
  margin-left: auto;
  line-height: 1;
  flex-shrink: 0;
}

.btn-toast-close:hover {
  opacity: 1;
}

.toast-info {
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  color: #1d4ed8;
}

.toast-success {
  background: #ecfdf5;
  border: 1px solid #a7f3d0;
  color: #065f46;
}

.toast-warning {
  background: #fffbeb;
  border: 1px solid #fde68a;
  color: #92400e;
}

.toast-error {
  background: #fef2f2;
  border: 1px solid #fecaca;
  color: #991b1b;
}

@media (prefers-color-scheme: dark) {
  .toast-info {
    background: #172554;
    border-color: #1e40af;
    color: #93c5fd;
  }
  .toast-success {
    background: #064e3b;
    border-color: #047857;
    color: #a7f3d0;
  }
  .toast-warning {
    background: #451a03;
    border-color: #78350f;
    color: #fde68a;
  }
  .toast-error {
    background: #450a0a;
    border-color: #7f1d1d;
    color: #fca5a5;
  }
}

/* Drop Target Overlay */
.drop-target-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
  background: rgba(15, 23, 42, 0.65);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  animation: fadeInOverlay 0.15s ease-out;
}

.drop-target-content {
  background: var(--bg-card);
  border: 2px dashed var(--accent-primary);
  border-radius: var(--radius-card);
  padding: 2.5rem 2rem;
  text-align: center;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.35);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  max-width: 360px;
  width: 90%;
}

.drop-target-icon-bubble {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.drop-target-icon {
  width: 28px;
  height: 28px;
}

.drop-target-title {
  font-size: 1.15rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-primary);
}

.drop-target-desc {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin: 0;
}

@keyframes fadeInOverlay {
  from {
    opacity: 0;
    transform: scale(0.98);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
