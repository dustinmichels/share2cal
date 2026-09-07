<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { extractTextFromBytes, type OcrResult } from "./services/ocr";
import {
  parseEventFromText,
  downloadIcsFile,
  addEventToNativeCalendar,
  extractDateInput,
  extractTimeInput,
  buildIsoFromDateTime,
  type EventDetails,
} from "./services/event";
import {
  getPendingSharedImage,
  clearPendingSharedImage,
  payloadToFile,
} from "./services/share";
import ModelSettingsModal from "./components/ModelSettingsModal.vue";
import { getModelStatuses, type ModelStatus } from "./services/model";

const isSettingsOpen = ref(false);
const modelStatuses = ref<ModelStatus[]>([]);
const hasLocalModel = computed(() => modelStatuses.value.some((m) => m.is_downloaded));
const defaultModel = computed(() => modelStatuses.value.find((m) => m.is_downloaded) || modelStatuses.value.find((m) => m.is_default));
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
const isDragging = ref(false);
const isProcessing = ref(false);
const errorMessage = ref<string | null>(null);
const ocrResult = ref<OcrResult | null>(null);
const eventDetails = ref<EventDetails | null>(null);

const shareNotification = ref<string | null>(null);
const isFromShareExtension = ref(false);
const copiedOcr = ref(false);
const copiedSummary = ref(false);
const calendarDownloaded = ref(false);
const isAddingToCalendar = ref(false);
const calendarSuccessMessage = ref<string | null>(null);
const calendarErrorMessage = ref<string | null>(null);
const showOcrSection = ref(false);
const showLineDetails = ref(false);
const fileInputRef = ref<HTMLInputElement | null>(null);
const cameraInputRef = ref<HTMLInputElement | null>(null);

// Editable event form model
const eventForm = ref({
  title: "",
  date: "",
  startTime: "",
  endTime: "",
  isAllDay: false,
  location: "",
  description: "",
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
  };
}

watch(eventDetails, (newVal) => {
  if (newVal) {
    syncFormFromEvent(newVal);
  }
});

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const wordCount = computed(() => {
  if (!ocrResult.value?.text) return 0;
  return ocrResult.value.text.trim().split(/\s+/).filter(Boolean).length;
});

const averageConfidence = computed(() => {
  if (!ocrResult.value?.lines || ocrResult.value.lines.length === 0) return 0;
  const total = ocrResult.value.lines.reduce((sum, line) => sum + line.confidence, 0);
  return Math.round((total / ocrResult.value.lines.length) * 100);
});

const eventConfidencePercent = computed(() => {
  if (!eventDetails.value) return 0;
  return Math.round(eventDetails.value.confidence * 100);
});

function setImageFile(file: File) {
  if (!file.type.startsWith("image/") && !file.name.match(/\.(heic|heif|png|jpe?g|webp|bmp|gif)$/i)) {
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
  eventDetails.value = null;
  calendarDownloaded.value = false;
  showOcrSection.value = false;
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

function handleDragOver(event: DragEvent) {
  event.preventDefault();
  isDragging.value = true;
}

function handleDragLeave(event: DragEvent) {
  event.preventDefault();
  isDragging.value = false;
}

function handleDrop(event: DragEvent) {
  event.preventDefault();
  isDragging.value = false;

  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    setImageFile(event.dataTransfer.files[0]);
  }
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

  errorMessage.value = null;
  isProcessing.value = true;
  calendarDownloaded.value = false;

  try {
    const arrayBuffer = await selectedFile.value.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);
    const res = await extractTextFromBytes(bytes);
    ocrResult.value = res;

    if (!res.text.trim()) {
      errorMessage.value = "No text was detected in this image. Try another photo with clearer text.";
      eventDetails.value = null;
    } else {
      // Parse event details from the extracted OCR text
      const parsed = await parseEventFromText(res.text);
      eventDetails.value = parsed;
    }
  } catch (err: any) {
    errorMessage.value = err?.toString() || "Failed to process OCR and event extraction on the selected image.";
  } finally {
    isProcessing.value = false;
  }
}

async function handleReparse() {
  if (!ocrResult.value?.text) return;
  try {
    const parsed = await parseEventFromText(ocrResult.value.text);
    eventDetails.value = parsed;
  } catch (err: any) {
    errorMessage.value = err?.toString() || "Failed to re-parse event details.";
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
    confidence: eventDetails.value?.confidence ?? 0.8,
    source: eventDetails.value?.source ?? "deterministic",
  };
}

async function handleAddToCalendar() {
  const event = getComposedEvent();
  isAddingToCalendar.value = true;
  calendarSuccessMessage.value = null;
  calendarErrorMessage.value = null;
  calendarDownloaded.value = false;

  try {
    const result = await addEventToNativeCalendar(event);
    if (result.success) {
      calendarSuccessMessage.value = `Event "${event.title}" was added directly to your Calendar!`;
      setTimeout(() => {
        calendarSuccessMessage.value = null;
      }, 5000);
    } else {
      calendarErrorMessage.value = result.error || "Failed to add event to native calendar.";
      // Fallback: download .ics file so the user never loses their event
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

function handleExportIcs() {
  const event = getComposedEvent();
  downloadIcsFile(event);
  calendarDownloaded.value = true;
  setTimeout(() => {
    calendarDownloaded.value = false;
  }, 4000);
}

async function copySummary() {
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
    console.error("Failed to copy summary:", err);
  }
}

function handleReset() {
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
  selectedFile.value = null;
  previewUrl.value = null;
  ocrResult.value = null;
  eventDetails.value = null;
  errorMessage.value = null;
  showLineDetails.value = false;
  showOcrSection.value = false;
  calendarDownloaded.value = false;
  calendarSuccessMessage.value = null;
  calendarErrorMessage.value = null;
  isFromShareExtension.value = false;
  shareNotification.value = null;
}

async function copyOcrToClipboard() {
  if (!ocrResult.value?.text) return;
  try {
    await navigator.clipboard.writeText(ocrResult.value.text);
    copiedOcr.value = true;
    setTimeout(() => {
      copiedOcr.value = false;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy OCR text:", err);
  }
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
  checkPendingShare();
  refreshModelStatus();
});

onUnmounted(() => {
  window.removeEventListener("paste", handlePaste);
  window.removeEventListener("focus", checkPendingShare);
  document.removeEventListener("visibilitychange", handleVisibilityChange);
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
});
</script>

<template>
  <div class="app-layout">
    <main class="app-container">
      <!-- App Header -->
      <header class="app-header">
        <div class="header-top-bar">
          <button
            type="button"
            class="btn-settings-pill"
            :class="{ 'has-model': hasLocalModel }"
            @click="isSettingsOpen = true"
            aria-label="Model Settings"
          >
            <svg class="pill-gear-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
            <span class="pill-text">{{ hasLocalModel ? (defaultModelName || 'Model Ready') : 'AI Model Settings' }}</span>
            <span class="pill-dot" :class="hasLocalModel ? 'dot-ready' : 'dot-missing'"></span>
          </button>
        </div>
        <div class="logo-badge">
          <svg class="logo-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
          <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path>
            <polyline points="16 6 12 2 8 6"></polyline>
            <line x1="12" y1="2" x2="12" y2="15"></line>
          </svg>
        </div>
        <span class="toast-text">{{ shareNotification }}</span>
        <button type="button" class="btn-toast-close" @click="shareNotification = null" aria-label="Close notification">✕</button>
      </div>

      <!-- Native Calendar Success Toast -->
      <div v-if="calendarSuccessMessage" class="toast-banner toast-success">
        <div class="toast-icon-wrap">
          <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Added to Calendar</span>
          <span class="toast-text">{{ calendarSuccessMessage }}</span>
        </div>
        <button type="button" class="btn-toast-close" @click="calendarSuccessMessage = null" aria-label="Close">✕</button>
      </div>

      <!-- Calendar Warning / Fallback Toast -->
      <div v-if="calendarErrorMessage" class="toast-banner toast-warning">
        <div class="toast-icon-wrap">
          <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Calendar Notice</span>
          <span class="toast-text">{{ calendarErrorMessage }} (.ics exported)</span>
        </div>
        <button type="button" class="btn-toast-close" @click="calendarErrorMessage = null" aria-label="Close">✕</button>
      </div>

      <!-- ICS Export Success Toast -->
      <div v-if="calendarDownloaded" class="toast-banner toast-success">
        <div class="toast-icon-wrap">
          <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Calendar File (.ics) Exported</span>
          <span class="toast-text">Open the downloaded file to add to Apple Calendar, Google, or Outlook.</span>
        </div>
        <button type="button" class="btn-toast-close" @click="calendarDownloaded = false" aria-label="Close">✕</button>
      </div>

      <!-- General Error Toast -->
      <div v-if="errorMessage" class="toast-banner toast-error">
        <div class="toast-icon-wrap">
          <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </div>
        <div class="toast-body">
          <span class="toast-heading">Processing Notice</span>
          <span class="toast-text">{{ errorMessage }}</span>
        </div>
        <button type="button" class="btn-toast-close" @click="errorMessage = null" aria-label="Close">✕</button>
      </div>

      <!-- Hidden file inputs -->
      <input
        ref="fileInputRef"
        type="file"
        accept="image/*,.heic,.heif"
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

      <!-- STATE 1: Empty Upload Hub (No Image Selected) -->
      <div v-if="!selectedFile" class="empty-hub-flow">
        <section
        class="upload-hub"
        :class="{ 'is-dragging': isDragging }"
        @dragover="handleDragOver"
        @dragleave="handleDragLeave"
        @drop="handleDrop"
      >
        <div class="upload-hero">
          <div class="upload-icon-bubble">
            <svg class="upload-bubble-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="3" ry="3"></rect>
              <circle cx="8.5" cy="8.5" r="1.5"></circle>
              <polyline points="21 15 16 10 5 21"></polyline>
            </svg>
          </div>
          <h2 class="upload-title">Add Flyer or Screenshot</h2>
          <p class="upload-subtitle">Choose a photo or snap a picture of an event flyer, invite, or schedule.</p>
        </div>

        <div class="upload-actions">
          <button type="button" class="btn-touch btn-touch-primary" @click="triggerFileUpload">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="17 8 12 3 7 8"></polyline>
              <line x1="12" y1="3" x2="12" y2="15"></line>
            </svg>
            <span>Choose from Library</span>
          </button>

          <button type="button" class="btn-touch btn-touch-secondary" @click="triggerCameraCapture">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"></path>
              <circle cx="12" cy="13" r="4"></circle>
            </svg>
            <span>Take Photo</span>
          </button>
        </div>

        <div class="upload-footer">
          <p class="format-note">Supports PNG, JPG, HEIF • Also paste images via ⌘V</p>
        </div>
        </section>

        <!-- On-Device Model Mini Status Card -->
        <div class="surface-card model-status-mini-card" @click="isSettingsOpen = true">
          <div class="mini-card-icon-wrap" :class="hasLocalModel ? 'icon-ready' : 'icon-missing'">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z"></path>
              <path d="M6 10a6 6 0 0 0 12 0"></path>
              <line x1="12" y1="16" x2="12" y2="22"></line>
              <line x1="8" y1="22" x2="16" y2="22"></line>
            </svg>
          </div>
          <div class="mini-card-body">
            <div class="mini-card-title-row">
              <span class="mini-card-title">On-Device AI Model</span>
              <span class="mini-status-chip" :class="hasLocalModel ? 'chip-ready' : 'chip-missing'">
                {{ hasLocalModel ? 'Ready on Device' : 'Not Downloaded' }}
              </span>
            </div>
            <p class="mini-card-desc">
              {{ hasLocalModel ? `${defaultModelName || 'SmolLM2 360M'} active for 100% private, offline parsing.` : 'Download a tiny model (~270 MB) from Hugging Face for enhanced offline extraction.' }}
            </p>
          </div>
          <button type="button" class="btn-manage-model">
            {{ hasLocalModel ? 'Manage' : 'Download' }}
          </button>
        </div>
      </div>
      <!-- STATE 2: Image Selected & Event Extracted State -->
      <section v-else class="content-flow">
        <!-- Hero Preview & Scanner Card -->
        <div class="surface-card preview-card">
          <div class="preview-top-bar">
            <div class="preview-meta">
              <span class="preview-file-name" :title="selectedFile.name">{{ selectedFile.name }}</span>
              <div class="preview-chips">
                <span class="chip-size">{{ formatFileSize(selectedFile.size) }}</span>
                <span v-if="isFromShareExtension" class="chip-share">
                  <svg class="chip-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path>
                    <polyline points="16 6 12 2 8 6"></polyline>
                    <line x1="12" y1="2" x2="12" y2="15"></line>
                  </svg>
                  iOS Share
                </span>
              </div>
            </div>

            <button type="button" class="btn-pill-danger" :disabled="isProcessing" @click="handleReset" aria-label="Remove photo">
              <svg class="btn-icon-xs" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
              <span>Remove</span>
            </button>
          </div>

          <div class="image-stage">
            <img v-if="previewUrl" :src="previewUrl" alt="Selected flyer preview" class="stage-img" />
          </div>

          <div class="scanner-action-wrap">
            <button
              type="button"
              class="btn-touch btn-touch-scan"
              :class="{ 'is-loading': isProcessing }"
              :disabled="isProcessing"
              @click="handleGo"
            >
              <template v-if="!isProcessing">
                <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
                <span>{{ eventDetails ? 'Re-scan & Extract' : 'Scan Flyer & Extract Event' }}</span>
              </template>
              <template v-else>
                <div class="spinner-circle"></div>
                <span>Scanning Flyer...</span>
              </template>
            </button>

            <div class="quick-switch-bar">
              <button type="button" class="btn-subtle-link" :disabled="isProcessing" @click="triggerFileUpload">
                Choose another photo
              </button>
              <span class="quick-dot">•</span>
              <button type="button" class="btn-subtle-link" :disabled="isProcessing" @click="triggerCameraCapture">
                Take new photo
              </button>
            </div>
          </div>
        </div>

        <!-- Event Details Form Section -->
        <div v-if="eventDetails" class="surface-card event-section">
          <div class="section-header">
            <div class="section-title-wrap">
              <div class="section-icon-bubble">
                <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
              <span class="badge-pill badge-pill-confidence">{{ eventConfidencePercent }}% match</span>
            </div>
          </div>

          <!-- Grouped Form Fields -->
          <div class="grouped-form">
            <!-- Event Title Field -->
            <div class="field-item">
              <label class="field-label" for="event-title">Title</label>
              <input
                id="event-title"
                v-model="eventForm.title"
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
                  v-model="eventForm.date"
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
                      v-model="eventForm.isAllDay"
                      type="checkbox"
                      class="switch-input"
                    />
                    <span class="switch-slider"></span>
                  </div>
                </label>
              </div>
            </div>

            <!-- Start / End Time Row (if not all-day) -->
            <div v-if="!eventForm.isAllDay" class="time-grid">
              <div class="field-item">
                <label class="field-label" for="event-start">Starts</label>
                <input
                  id="event-start"
                  v-model="eventForm.startTime"
                  type="time"
                  class="field-input field-input-time"
                />
              </div>

              <div class="field-item">
                <label class="field-label" for="event-end">Ends</label>
                <input
                  id="event-end"
                  v-model="eventForm.endTime"
                  type="time"
                  class="field-input field-input-time"
                />
              </div>
            </div>

            <!-- Location Field -->
            <div class="field-item">
              <label class="field-label" for="event-location">Location</label>
              <div class="input-icon-shell">
                <svg class="input-inline-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"></path>
                  <circle cx="12" cy="10" r="3"></circle>
                </svg>
                <input
                  id="event-location"
                  v-model="eventForm.location"
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
                v-model="eventForm.description"
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
              @click="handleAddToCalendar"
            >
              <template v-if="isAddingToCalendar">
                <div class="spinner-circle spinner-light"></div>
                <span>Adding to Calendar...</span>
              </template>
              <template v-else>
                <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
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
                type="button"
                class="btn-touch btn-touch-outline"
                @click="handleExportIcs"
              >
                <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
                @click="copySummary"
              >
                <template v-if="copiedSummary">
                  <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                  <span>Copied!</span>
                </template>
                <template v-else>
                  <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                  </svg>
                  <span>Copy Summary</span>
                </template>
              </button>
            </div>
          </div>
        </div>

        <!-- Collapsible Raw OCR Diagnostics Drawer -->
        <div v-if="ocrResult" class="surface-card ocr-accordion">
          <button
            type="button"
            class="accordion-trigger"
            @click="showOcrSection = !showOcrSection"
          >
            <div class="accordion-title-wrap">
              <svg class="accordion-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="4 7 4 4 20 4 20 7"></polyline>
                <line x1="9" y1="20" x2="15" y2="20"></line>
                <line x1="12" y1="4" x2="12" y2="20"></line>
              </svg>
              <span class="accordion-title">Extracted OCR Text</span>
              <span class="chip-count">{{ ocrResult.lines.length }} lines • {{ wordCount }} words • {{ averageConfidence }}% conf</span>
            </div>

            <svg
              class="accordion-chevron"
              :class="{ 'is-open': showOcrSection }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>

          <div v-if="showOcrSection" class="accordion-content">
            <div class="ocr-toolbar">
              <button
                type="button"
                class="btn-subtle-tool"
                @click="handleReparse"
              >
                <svg class="btn-icon-xs" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10"></polyline>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                </svg>
                <span>Re-parse</span>
              </button>

              <button
                type="button"
                class="btn-subtle-tool"
                :class="{ 'is-copied': copiedOcr }"
                :disabled="!ocrResult.text.trim()"
                @click="copyOcrToClipboard"
              >
                <template v-if="copiedOcr">
                  <svg class="btn-icon-xs" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                  <span>Copied Text</span>
                </template>
                <template v-else>
                  <svg class="btn-icon-xs" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                  </svg>
                  <span>Copy OCR</span>
                </template>
              </button>
            </div>

            <textarea
              readonly
              class="ocr-raw-display"
              :value="ocrResult.text"
              rows="5"
              placeholder="No text detected."
            ></textarea>

            <!-- Line Details Toggle -->
            <div v-if="ocrResult.lines.length > 0" class="line-details-block">
              <button
                type="button"
                class="btn-line-toggle"
                @click="showLineDetails = !showLineDetails"
              >
                <span>{{ showLineDetails ? 'Hide' : 'Show' }} line-by-line confidence</span>
                <svg
                  class="btn-icon-xs chevron-sm"
                  :class="{ 'is-rotated': showLineDetails }"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>

              <div v-if="showLineDetails" class="line-breakdown-list">
                <div v-for="(line, idx) in ocrResult.lines" :key="idx" class="line-item">
                  <span class="line-idx">{{ idx + 1 }}</span>
                  <span class="line-content">{{ line.text }}</span>
                  <span
                    class="line-score"
                    :class="{
                      'score-high': line.confidence >= 0.8,
                      'score-med': line.confidence >= 0.5 && line.confidence < 0.8,
                      'score-low': line.confidence < 0.5
                    }"
                  >
                    {{ Math.round(line.confidence * 100) }}%
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </main>

      <!-- Model Settings Modal -->
      <ModelSettingsModal
        :is-open="isSettingsOpen"
        @close="isSettingsOpen = false"
        @models-updated="refreshModelStatus"
      />
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
  padding: max(1.25rem, env(safe-area-inset-top)) max(1rem, env(safe-area-inset-right)) max(2rem, env(safe-area-inset-bottom)) max(1rem, env(safe-area-inset-left));
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

.header-top-bar {
  width: 100%;
  display: flex;
  justify-content: flex-end;
  margin-bottom: 0.25rem;
}

.btn-settings-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.35rem 0.75rem;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 999px;
  color: var(--text-primary);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  box-shadow: var(--shadow-subtle);
  transition: all 0.15s ease;
}

.btn-settings-pill:hover {
  background: var(--bg-input);
  transform: translateY(-1px);
}

.pill-gear-icon {
  width: 14px;
  height: 14px;
  color: var(--text-muted);
}

.pill-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.dot-ready {
  background: #34c759;
  box-shadow: 0 0 6px rgba(52, 199, 89, 0.6);
}

.dot-missing {
  background: #ff9500;
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
  display: none;
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

/* Surface Cards (Clean iOS Grouped Surface) */
.surface-card {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card);
  padding: 1.25rem;
  box-shadow: var(--shadow-card);
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  transition: background-color 0.2s ease, border-color 0.2s ease;
}

/* Model Status Mini Card in Empty State */
.model-status-mini-card {
  cursor: pointer;
  flex-direction: row;
  align-items: center;
  gap: 1rem;
  padding: 1rem 1.25rem;
  transition: all 0.2s ease;
}

.model-status-mini-card:hover {
  border-color: rgba(0, 122, 255, 0.3);
  transform: translateY(-1px);
}

.mini-card-icon-wrap {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.mini-card-icon-wrap svg {
  width: 20px;
  height: 20px;
}

.icon-ready {
  background: rgba(52, 199, 89, 0.12);
  color: #34c759;
}

.icon-missing {
  background: rgba(0, 122, 255, 0.12);
  color: #007aff;
}

.mini-card-body {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  flex: 1;
  min-width: 0;
}

.mini-card-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.mini-card-title {
  font-size: 0.88rem;
  font-weight: 700;
  color: var(--text-primary);
}

.mini-status-chip {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.12rem 0.45rem;
  border-radius: 999px;
}

.mini-status-chip.chip-ready {
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.mini-status-chip.chip-missing {
  background: rgba(142, 142, 147, 0.15);
  color: var(--text-muted);
}

.mini-card-desc {
  font-size: 0.78rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.35;
}

.btn-manage-model {
  background: rgba(0, 122, 255, 0.1);
  color: #007aff;
  border: none;
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.4rem 0.8rem;
  border-radius: 8px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.btn-manage-model:hover {
  background: #007aff;
  color: #ffffff;
}

.empty-hub-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* Upload Hub (Empty State) */
.upload-hub {
  background: var(--bg-card);
  border: 1.5px dashed var(--border-input);
  border-radius: var(--radius-card);
  padding: 2.25rem 1.25rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  box-shadow: var(--shadow-card);
  transition: all 0.2s ease;
}

.upload-hub.is-dragging {
  border-color: var(--accent-primary);
  background: rgba(0, 122, 255, 0.05);
  transform: scale(1.01);
}

.upload-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 380px;
}

.upload-icon-bubble {
  width: 60px;
  height: 60px;
  border-radius: 18px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
}

.upload-bubble-icon {
  width: 32px;
  height: 32px;
}

.upload-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0 0 0.4rem 0;
  color: var(--text-primary);
  letter-spacing: -0.015em;
}

.upload-subtitle {
  font-size: 0.88rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.45;
}

.upload-actions {
  width: 100%;
  max-width: 380px;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.upload-footer {
  margin-top: -0.25rem;
}

.format-note {
  font-size: 0.8rem;
  color: var(--text-tertiary);
  margin: 0;
}

/* Content Flow */
.content-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* Image Preview Card */
.preview-top-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.preview-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.preview-file-name {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 220px;
}

.preview-chips {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-wrap: wrap;
}

.chip-size {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 0.15rem 0.5rem;
  border-radius: 6px;
  border: 1px solid var(--border-card);
}

.chip-share {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--accent-primary);
  background: rgba(0, 122, 255, 0.12);
  padding: 0.15rem 0.5rem;
  border-radius: 12px;
}

.chip-icon {
  width: 12px;
  height: 12px;
}

.btn-pill-danger {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: transparent;
  border: 1px solid transparent;
  color: var(--accent-danger);
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.35rem 0.7rem;
  border-radius: 20px;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.btn-pill-danger:hover:not(:disabled) {
  background: var(--accent-danger-bg);
}

.btn-pill-danger:active:not(:disabled) {
  transform: scale(0.96);
}

.image-stage {
  width: 100%;
  max-height: 320px;
  background: #090d14;
  border-radius: 14px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-card);
}

.stage-img {
  max-width: 100%;
  max-height: 320px;
  object-fit: contain;
  display: block;
}

.scanner-action-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: center;
}

.quick-switch-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.btn-subtle-link {
  background: none;
  border: none;
  color: var(--accent-primary);
  font-size: 0.86rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
}

.btn-subtle-link:hover:not(:disabled) {
  text-decoration: underline;
}

.quick-dot {
  color: var(--text-tertiary);
}

/* Event Details Section */
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

.badge-pill {
  font-size: 0.78rem;
  font-weight: 700;
  padding: 0.25rem 0.6rem;
  border-radius: 20px;
  letter-spacing: -0.01em;
  white-space: nowrap;
}

.badge-pill-confidence {
  background: rgba(52, 199, 89, 0.15);
  color: #15803d;
}

@media (prefers-color-scheme: dark) {
  .badge-pill-confidence {
    background: rgba(48, 209, 88, 0.2);
    color: #4ade80;
  }
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

/* Button System */
.btn-touch {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  width: 100%;
  min-height: 48px;
  padding: 0.75rem 1.25rem;
  border-radius: var(--radius-btn);
  font-size: 0.96rem;
  font-weight: 700;
  border: none;
  cursor: pointer;
  transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  box-sizing: border-box;
  text-decoration: none;
}

.btn-touch:active:not(:disabled) {
  transform: scale(0.98);
}

.btn-touch:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.btn-touch-primary {
  background: var(--accent-primary);
  color: #ffffff;
  box-shadow: var(--shadow-primary-btn);
}

.btn-touch-primary:hover:not(:disabled) {
  background: var(--accent-primary-hover);
}

.btn-touch-secondary {
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-input);
}

.btn-touch-secondary:hover:not(:disabled) {
  background: var(--border-input);
}

.btn-touch-scan {
  background: linear-gradient(135deg, var(--accent-primary) 0%, #0056b3 100%);
  color: #ffffff;
  box-shadow: var(--shadow-primary-btn);
  font-size: 1rem;
}

.btn-touch-scan:hover:not(:disabled) {
  background: linear-gradient(135deg, var(--accent-primary-hover) 0%, #004696 100%);
}

.btn-touch-calendar {
  background: linear-gradient(135deg, #34c759 0%, #248a3d 100%);
  color: #ffffff;
  box-shadow: var(--shadow-green-btn);
  font-size: 1.02rem;
  min-height: 52px;
}

.btn-touch-calendar:hover:not(:disabled) {
  background: linear-gradient(135deg, #2db84d 0%, #1e7534 100%);
}

.btn-touch-outline {
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-primary);
  font-size: 0.88rem;
  font-weight: 600;
  min-height: 44px;
}

.btn-touch-outline:hover:not(:disabled) {
  background: var(--border-input);
}

.btn-touch-outline.is-copied {
  background: rgba(52, 199, 89, 0.12);
  border-color: rgba(52, 199, 89, 0.4);
  color: #15803d;
}

@media (prefers-color-scheme: dark) {
  .btn-touch-outline.is-copied {
    color: #4ade80;
  }
}

.btn-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
}

.btn-icon-sm {
  width: 17px;
  height: 17px;
  flex-shrink: 0;
}

.btn-icon-xs {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

/* Spinner */
.spinner-circle {
  width: 20px;
  height: 20px;
  border: 2.5px solid rgba(255, 255, 255, 0.3);
  border-radius: 50%;
  border-top-color: #ffffff;
  animation: spinCircle 0.8s linear infinite;
  display: inline-block;
  flex-shrink: 0;
}

.spinner-light {
  border-color: rgba(255, 255, 255, 0.3);
  border-top-color: #ffffff;
}

@keyframes spinCircle {
  to {
    transform: rotate(360deg);
  }
}

/* Collapsible OCR Diagnostics */
.ocr-accordion {
  padding: 0.9rem 1.1rem;
  gap: 0.75rem;
}

.accordion-trigger {
  width: 100%;
  background: none;
  border: none;
  padding: 0.25rem 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  color: var(--text-primary);
  text-align: left;
}

.accordion-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.accordion-icon {
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
}

.accordion-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--text-primary);
}

.chip-count {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
}

.accordion-chevron {
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.accordion-chevron.is-open {
  transform: rotate(180deg);
}

.accordion-content {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border-card-subtle);
}

.ocr-toolbar {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.btn-subtle-tool {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-secondary);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.35rem 0.65rem;
  border-radius: 8px;
  transition: all 0.15s ease;
}

.btn-subtle-tool:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}

.btn-subtle-tool.is-copied {
  background: rgba(52, 199, 89, 0.12);
  border-color: rgba(52, 199, 89, 0.4);
  color: #15803d;
}

.ocr-raw-display {
  width: 100%;
  box-sizing: border-box;
  padding: 0.75rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.82rem;
  line-height: 1.5;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 10px;
  resize: vertical;
}

.ocr-raw-display:focus {
  outline: none;
  border-color: var(--accent-primary);
}

.line-details-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.btn-line-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: none;
  border: none;
  color: var(--accent-primary);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.2rem 0;
  width: fit-content;
}

.chevron-sm {
  transition: transform 0.2s ease;
}

.chevron-sm.is-rotated {
  transform: rotate(180deg);
}

.line-breakdown-list {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--border-input);
  border-radius: 8px;
  background: var(--bg-input);
}

.line-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.35rem 0.6rem;
  border-bottom: 1px solid var(--border-card-subtle);
  font-size: 0.8rem;
}

.line-item:last-child {
  border-bottom: none;
}

.line-idx {
  color: var(--text-tertiary);
  font-family: monospace;
  font-size: 0.75rem;
  min-width: 18px;
}

.line-content {
  flex: 1;
  color: var(--text-primary);
  word-break: break-word;
}

.line-score {
  font-family: monospace;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.1rem 0.3rem;
  border-radius: 4px;
}

.score-high {
  background: rgba(52, 199, 89, 0.15);
  color: #15803d;
}

.score-med {
  background: rgba(234, 179, 8, 0.15);
  color: #a16207;
}

.score-low {
  background: rgba(239, 68, 68, 0.15);
  color: #b91c1c;
}

@media (prefers-color-scheme: dark) {
  .score-high {
    background: rgba(48, 209, 88, 0.2);
    color: #4ade80;
  }
  .score-med {
    background: rgba(250, 204, 21, 0.2);
    color: #fde047;
  }
  .score-low {
    background: rgba(248, 113, 113, 0.2);
    color: #fca5a5;
  }
}

/* Small Screens Optimization (iPhone SE, 375px or narrower) */
@media (max-width: 380px) {
  .app-title {
    font-size: 1.5rem;
  }
  .preview-file-name {
    max-width: 140px;
  }
  .secondary-button-row {
    grid-template-columns: 1fr;
  }
  .field-row {
    flex-direction: column;
    align-items: stretch;
  }
  .field-item-toggle {
    justify-content: space-between;
    padding-top: 0.25rem;
  }
}
</style>
