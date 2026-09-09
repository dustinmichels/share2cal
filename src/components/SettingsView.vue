<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import {
  getAvailableCalendars,
  checkCalendarPermission,
  requestCalendarPermission,
  type CalendarInfo,
  type CalendarPermissionStatus,
} from "../services/calendar";
import {
  getStoredDefaultCalendarId,
  setStoredDefaultCalendarId,
  setStoredCalendarPreference,
  getStoredDefaultTarget,
  setStoredDefaultTarget,
  type CalendarTarget,
  type ParsingMode,
} from "../services/settings";
import {
  getModelStatuses,
  getModelsStorageInfo,
  downloadModel,
  cancelModelDownload,
  deleteModel,
  verifyModelHash,
  onModelDownloadProgress,
  openModelsDirectory,
  isDesktopDevice,
  formatBytes,
  formatSpeed,
  type ModelStatus,
  type ModelsStorageInfo,
  type DownloadProgressPayload,
} from "../services/model";

const props = defineProps<{
  parsingMode: ParsingMode;
}>();

const emit = defineEmits<{
  (e: "update:parsingMode", mode: ParsingMode): void;
  (e: "back"): void;
  (e: "modelsUpdated"): void;
}>();

const models = ref<ModelStatus[]>([]);
const storageInfo = ref<ModelsStorageInfo | null>(null);
const isLoading = ref(true);
const actionError = ref<string | null>(null);
const actionSuccess = ref<string | null>(null);
const verifyingModelId = ref<string | null>(null);
const deletingModelId = ref<string | null>(null);
const activeDownloads = ref<
  Record<
    string,
    {
      progress: number;
      speed: number;
      received: number;
      total: number;
      status: string;
    }
  >
>({});
const copiedPath = ref(false);
const availableCalendars = ref<CalendarInfo[]>([]);
const selectedDefaultCalendarId = ref<string>(getStoredDefaultCalendarId() || "");
const defaultCalendarTarget = ref<CalendarTarget>(getStoredDefaultTarget());
const calendarPermissionStatus = ref<CalendarPermissionStatus>("unknown");
const isRequestingCalendarPermission = ref(false);
const showAlternativeModels = ref(false);
const isDesktop = ref(isDesktopDevice());

let unlistenProgress: (() => void) | null = null;

const defaultModel = computed<ModelStatus | null>(() => {
  return models.value.find((m) => m.is_default) || models.value[0] || null;
});

const alternativeModels = computed<ModelStatus[]>(() => {
  return models.value.filter((m) => !m.is_default);
});

const readyModelCount = computed(() => {
  return models.value.filter((m) => m.is_downloaded).length;
});

const downloadedAlternativeCount = computed(() => {
  return alternativeModels.value.filter((m) => m.is_downloaded).length;
});

async function loadCalendarData() {
  try {
    const [status, cals] = await Promise.all([checkCalendarPermission(), getAvailableCalendars()]);
    calendarPermissionStatus.value = status;
    availableCalendars.value = cals;
  } catch (err) {
    console.warn("Failed to load calendar data:", err);
  }
}

function handleCalendarSelectionChange(calendarId: string) {
  selectedDefaultCalendarId.value = calendarId;
  if (!calendarId) {
    setStoredCalendarPreference(null);
    actionSuccess.value = "Default calendar set to System Default.";
  } else {
    const found = availableCalendars.value.find((c) => c.id === calendarId);
    if (found) {
      setStoredCalendarPreference({
        id: found.id,
        title: found.title,
        sourceTitle: found.source_title,
      });
      actionSuccess.value = `Default calendar set to "${found.title}".`;
    } else {
      setStoredDefaultCalendarId(calendarId);
    }
  }
  setTimeout(() => {
    actionSuccess.value = null;
  }, 3000);
}

function handleTargetChange(target: CalendarTarget) {
  defaultCalendarTarget.value = target;
  setStoredDefaultTarget(target);
  actionSuccess.value = `Default destination set to ${
    target === "native"
      ? "Apple / Device Calendar"
      : target === "google"
        ? "Google Calendar"
        : ".ics Export"
  }.`;
  setTimeout(() => {
    actionSuccess.value = null;
  }, 3000);
}

async function handleGrantPermission() {
  isRequestingCalendarPermission.value = true;
  actionError.value = null;
  try {
    const granted = await requestCalendarPermission();
    if (granted) {
      actionSuccess.value = "Calendar permission granted!";
      await loadCalendarData();
    } else {
      actionError.value = "Calendar access was not granted. Please check device Settings.";
    }
  } catch (err) {
    actionError.value = err instanceof Error ? err.message : String(err);
  } finally {
    isRequestingCalendarPermission.value = false;
  }
}

async function loadData() {
  isLoading.value = true;
  actionError.value = null;
  try {
    const [statuses, info] = await Promise.all([
      getModelStatuses(),
      getModelsStorageInfo(),
      loadCalendarData(),
    ]);
    models.value = statuses;
    storageInfo.value = info;
  } catch (err) {
    actionError.value = err instanceof Error ? err.message : String(err);
  } finally {
    isLoading.value = false;
  }
}

onMounted(async () => {
  await loadData();

  try {
    unlistenProgress = await onModelDownloadProgress((payload: DownloadProgressPayload) => {
      handleProgress(payload);
    });
  } catch (err) {
    console.warn("Could not register model download progress listener:", err);
  }
});

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
});

function handleProgress(payload: DownloadProgressPayload) {
  if (payload.status === "downloading" || payload.status === "verifying") {
    activeDownloads.value[payload.model_id] = {
      progress: payload.percentage,
      speed: payload.speed_bytes_per_sec,
      received: payload.received_bytes,
      total: payload.total_bytes,
      status: payload.status,
    };

    const model = models.value.find((m) => m.id === payload.model_id);
    if (model) {
      model.is_downloading = true;
      model.downloaded_bytes = payload.received_bytes;
    }
  } else if (payload.status === "completed") {
    delete activeDownloads.value[payload.model_id];
    actionSuccess.value = "Model downloaded and verified successfully!";
    loadData();
    emit("modelsUpdated");
  } else if (payload.status === "cancelled") {
    delete activeDownloads.value[payload.model_id];
    actionError.value = "Download was cancelled.";
    loadData();
  } else if (payload.status === "error") {
    delete activeDownloads.value[payload.model_id];
    actionError.value = payload.error || "An error occurred during download.";
    loadData();
  }
}

async function handleDownload(modelId: string) {
  actionError.value = null;
  actionSuccess.value = null;

  activeDownloads.value[modelId] = {
    progress: 0,
    speed: 0,
    received: 0,
    total: 0,
    status: "downloading",
  };

  const model = models.value.find((m) => m.id === modelId);
  if (model) {
    model.is_downloading = true;
  }

  try {
    await downloadModel(modelId);
  } catch (err) {
    delete activeDownloads.value[modelId];
    if (model) model.is_downloading = false;
    const errMsg = err instanceof Error ? err.message : String(err);
    actionError.value = `Download failed: ${errMsg}`;
  }
}

async function handleCancel(modelId: string) {
  try {
    await cancelModelDownload(modelId);
  } catch (err) {
    actionError.value = `Failed to cancel download: ${err instanceof Error ? err.message : String(err)}`;
  }
}

async function handleDelete(modelId: string) {
  deletingModelId.value = modelId;
  actionError.value = null;
  actionSuccess.value = null;
  try {
    await deleteModel(modelId);
    actionSuccess.value = "Model file deleted to reclaim storage.";
    await loadData();
    emit("modelsUpdated");
  } catch (err) {
    actionError.value = `Failed to delete model: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    deletingModelId.value = null;
  }
}

async function handleVerify(modelId: string) {
  verifyingModelId.value = modelId;
  actionError.value = null;
  actionSuccess.value = null;
  try {
    const isValid = await verifyModelHash(modelId);
    if (isValid) {
      actionSuccess.value = "SHA-256 integrity check passed. Model is intact.";
    } else {
      actionError.value = "SHA-256 integrity check failed. Consider re-downloading.";
    }
    await loadData();
  } catch (err) {
    actionError.value = `Verification error: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    verifyingModelId.value = null;
  }
}

async function copyStoragePath() {
  if (storageInfo.value?.storage_dir) {
    try {
      await navigator.clipboard.writeText(storageInfo.value.storage_dir);
      copiedPath.value = true;
      setTimeout(() => {
        copiedPath.value = false;
      }, 2000);
    } catch {
      // Ignore clipboard fallback error
    }
  }
}

async function handleOpenStorageLocation() {
  actionError.value = null;
  actionSuccess.value = null;

  try {
    await openModelsDirectory();
    actionSuccess.value = "Opened models directory in file manager.";
    setTimeout(() => {
      if (actionSuccess.value === "Opened models directory in file manager.") {
        actionSuccess.value = null;
      }
    }, 4000);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (storageInfo.value?.storage_dir) {
      await copyStoragePath();
      actionError.value = `Cannot open desktop file manager directly (${msg}). Storage path has been copied to your clipboard.`;
    } else {
      actionError.value = `Failed to open models location: ${msg}`;
    }
  }
}

function setMode(mode: ParsingMode) {
  if (mode === "enhanced" && readyModelCount.value === 0 && defaultModel.value) {
    const reqBytes = Math.round(defaultModel.value.size_bytes * 1.5);
    const free = storageInfo.value?.free_disk_space_bytes;
    if (typeof free === "number" && free < reqBytes) {
      actionError.value = `Insufficient disk space to download default AI model (${formatBytes(free)} free, ${formatBytes(reqBytes)} required). Scans will fall back to Simple Mode until space is freed.`;
    } else if (!defaultModel.value.is_downloading && !defaultModel.value.is_downloaded) {
      handleDownload(defaultModel.value.id);
    }
  }
  emit("update:parsingMode", mode);
}
</script>

<template>
  <div class="settings-page">
    <!-- Top Navigation Header -->
    <header class="settings-nav-bar">
      <button type="button" class="btn-back" @click="emit('back')" aria-label="Back to Scanner">
        <svg
          class="nav-arrow-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="15 18 9 12 15 6"></polyline>
        </svg>
        <span class="back-text">Back</span>
      </button>

      <div class="header-center">
        <h1 class="settings-title">Settings</h1>
      </div>

      <div class="nav-spacer"></div>
    </header>

    <!-- Feedback Banners -->
    <div v-if="actionSuccess" class="banner banner-success">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.5"
        class="banner-icon"
      >
        <polyline points="20 6 9 17 4 12"></polyline>
      </svg>
      <span class="banner-text">{{ actionSuccess }}</span>
      <button type="button" class="banner-close" @click="actionSuccess = null" aria-label="Close">
        ✕
      </button>
    </div>

    <div v-if="actionError" class="banner banner-error">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        class="banner-icon"
      >
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
      <span class="banner-text">{{ actionError }}</span>
      <button type="button" class="banner-close" @click="actionError = null" aria-label="Close">
        ✕
      </button>
    </div>

    <!-- CALENDAR & DESTINATIONS SETTING CARD -->
    <section class="surface-card setting-section-card">
      <div class="section-top">
        <div class="section-title-wrap">
          <div class="section-icon-bubble bubble-calendar">
            <svg
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
            <h2 class="section-heading">Calendar Destinations</h2>
            <p class="section-subheading">Choose where scanned events get saved</p>
          </div>
        </div>
      </div>

      <!-- Segmented Toggle: Default Destination Target -->
      <div class="setting-subsection">
        <label class="subsection-label">Default Export Action</label>
        <div
          class="toggle-track target-toggle-track"
          role="tablist"
          aria-label="Default calendar destination"
        >
          <button
            type="button"
            class="toggle-btn"
            :class="{ 'is-active': defaultCalendarTarget === 'native' }"
            role="tab"
            :aria-selected="defaultCalendarTarget === 'native'"
            @click="handleTargetChange('native')"
          >
            <svg
              class="toggle-icon"
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
              <line x1="12" y1="11" x2="12" y2="17"></line>
              <line x1="9" y1="14" x2="15" y2="14"></line>
            </svg>
            <span class="toggle-label">Apple Cal</span>
          </button>

          <button
            type="button"
            class="toggle-btn"
            :class="{ 'is-active': defaultCalendarTarget === 'google' }"
            role="tab"
            :aria-selected="defaultCalendarTarget === 'google'"
            @click="handleTargetChange('google')"
          >
            <svg
              class="toggle-icon"
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
            <span class="toggle-label">Google Cal</span>
          </button>

          <button
            type="button"
            class="toggle-btn"
            :class="{ 'is-active': defaultCalendarTarget === 'ics' }"
            role="tab"
            :aria-selected="defaultCalendarTarget === 'ics'"
            @click="handleTargetChange('ics')"
          >
            <svg
              class="toggle-icon"
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
            <span class="toggle-label">.ics File</span>
          </button>
        </div>
      </div>

      <!-- Target Apple/Device Calendar Dropdown -->
      <div class="setting-subsection">
        <label class="subsection-label" for="default-calendar-select">
          Default Device Calendar
        </label>

        <div v-if="availableCalendars.length > 0" class="calendar-picker-row">
          <select
            id="default-calendar-select"
            class="settings-select"
            :value="selectedDefaultCalendarId"
            @change="handleCalendarSelectionChange(($event.target as HTMLSelectElement).value)"
          >
            <option value="">System Default Calendar</option>
            <option v-for="cal in availableCalendars" :key="cal.id" :value="cal.id">
              {{ cal.title }} {{ cal.source_title ? `(${cal.source_title})` : "" }}
            </option>
          </select>
        </div>

        <div v-else class="permission-prompt-box">
          <p class="prompt-text">
            {{
              calendarPermissionStatus === "denied" || calendarPermissionStatus === "restricted"
                ? "Calendar permissions are disabled. Enable access in device Settings to select specific calendars."
                : "Grant calendar access to list your synced iCloud, Google, and Exchange calendars."
            }}
          </p>
          <button
            v-if="
              calendarPermissionStatus === 'not_determined' ||
              calendarPermissionStatus === 'write_only' ||
              calendarPermissionStatus === 'unknown'
            "
            type="button"
            class="btn-grant-permission"
            :disabled="isRequestingCalendarPermission"
            @click="handleGrantPermission"
          >
            {{ isRequestingCalendarPermission ? "Requesting..." : "Grant Calendar Access" }}
          </button>
        </div>
      </div>
    </section>

    <!-- MAIN SETTING CARD: Parsing Mode Selection -->
    <section class="surface-card setting-section-card">
      <div class="section-top">
        <div class="section-title-wrap">
          <div class="section-icon-bubble bubble-mode">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="4 7 4 4 20 4 20 7"></polyline>
              <line x1="9" y1="20" x2="15" y2="20"></line>
              <line x1="12" y1="4" x2="12" y2="20"></line>
            </svg>
          </div>
          <div class="section-heading-col">
            <div class="heading-with-status">
              <h2 class="section-heading">Parsing Mode</h2>
              <span
                class="active-mode-chip"
                :class="parsingMode === 'enhanced' ? 'chip-enhanced' : 'chip-simple'"
              >
                {{ parsingMode === "enhanced" ? "Enhanced Mode" : "Simple Mode" }}
              </span>
            </div>
            <p class="section-subheading">
              Select how Share2Cal extracts event details from text and images
            </p>
          </div>
        </div>
      </div>

      <!-- Segmented Mode Selection Track -->
      <div class="mode-selector-container">
        <label class="subsection-label">Select Active Mode</label>
        <div class="mode-cards-grid" role="radiogroup" aria-label="Event parsing mode">
          <!-- Option 1: Simple Mode Card -->
          <div
            class="mode-choice-card"
            :class="{ 'is-selected': parsingMode === 'simple' }"
            role="radio"
            :aria-checked="parsingMode === 'simple'"
            tabindex="0"
            @click="setMode('simple')"
            @keydown.space.prevent="setMode('simple')"
            @keydown.enter.prevent="setMode('simple')"
          >
            <div class="choice-top">
              <div class="choice-icon-label">
                <span class="choice-symbol">⚡</span>
                <div>
                  <h3 class="choice-title">Simple Mode</h3>
                  <span class="choice-tagline">Fast & Lightweight Rules (Fallback)</span>
                </div>
              </div>

              <div class="radio-indicator" :class="{ 'is-checked': parsingMode === 'simple' }">
                <div class="radio-dot" v-if="parsingMode === 'simple'"></div>
              </div>
            </div>

            <p class="choice-desc">
              Uses instant regex pattern matching and schedule table heuristics. Quick and zero
              storage overhead.
            </p>

            <div class="choice-tags-row">
              <span class="choice-pill">⚡ Fallback</span>
              <span class="choice-pill">⚡ Instant</span>
              <span class="choice-pill">📦 0 MB Download</span>
              <span class="choice-pill">📄 Clean text & tables</span>
            </div>
          </div>

          <!-- Option 2: Enhanced Mode Card -->
          <div
            class="mode-choice-card"
            :class="{ 'is-selected': parsingMode === 'enhanced' }"
            role="radio"
            :aria-checked="parsingMode === 'enhanced'"
            tabindex="0"
            @click="setMode('enhanced')"
            @keydown.space.prevent="setMode('enhanced')"
            @keydown.enter.prevent="setMode('enhanced')"
          >
            <div class="choice-top">
              <div class="choice-icon-label">
                <span class="choice-symbol">🧠</span>
                <div>
                  <h3 class="choice-title">Enhanced Mode</h3>
                  <span class="choice-tagline">Local On-Device AI (Default)</span>
                </div>
              </div>

              <div class="radio-indicator" :class="{ 'is-checked': parsingMode === 'enhanced' }">
                <div class="radio-dot" v-if="parsingMode === 'enhanced'"></div>
              </div>
            </div>

            <p class="choice-desc">
              Uses a compact LLM running 100% on your device for high accuracy on flyers, informal
              dates, and complex schedules.
            </p>

            <div class="choice-tags-row">
              <span class="choice-pill">🧠 Default</span>
              <span class="choice-pill">🧠 AI Reasoning</span>
              <span class="choice-pill">🔒 100% Private</span>
              <span class="choice-pill">🎨 Complex flyers</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Active Mode Explanation Banner -->
      <div
        class="active-mode-callout"
        :class="parsingMode === 'enhanced' ? 'callout-enhanced' : 'callout-simple'"
      >
        <div class="callout-content">
          <div class="callout-header">
            <span class="callout-icon">{{ parsingMode === "enhanced" ? "🧠" : "⚡" }}</span>
            <span class="callout-title">
              {{
                parsingMode === "enhanced"
                  ? "Enhanced Mode is currently active"
                  : "Simple Mode is currently active"
              }}
            </span>
            <span
              v-if="parsingMode === 'enhanced' && readyModelCount > 0"
              class="ready-indicator-tag"
            >
              ✓ AI Model Ready
            </span>
          </div>
          <p class="callout-text">
            <template v-if="parsingMode === 'simple'">
              All scans will use fast, lightweight rule-based parsing. No on-device LLM model files
              or downloads are required.
            </template>
            <template v-else-if="readyModelCount > 0">
              Enhanced Mode (Default) is active. Scans will use your downloaded on-device model for
              intelligent event extraction with zero data leaving your device.
            </template>
            <template v-else>
              Enhanced Mode is the default. Scans will automatically fall back to fast Simple Mode
              until the on-device AI model finishes downloading.
            </template>
          </p>
        </div>
      </div>
    </section>

    <!-- ENHANCED MODE SECTION: Model Management -->
    <div v-if="parsingMode === 'enhanced'" class="enhanced-models-flow">
      <!-- Storage Overview Card -->
      <section class="surface-card storage-card" v-if="storageInfo">
        <div class="storage-top">
          <div class="storage-title-row">
            <span class="storage-title">Storage & Inference Engine</span>
            <span
              class="storage-status-pill"
              :class="readyModelCount > 0 ? 'pill-green' : 'pill-gray'"
            >
              {{ readyModelCount > 0 ? `${readyModelCount} Model Ready` : "No Model Downloaded" }}
            </span>
          </div>
        </div>

        <div class="storage-grid">
          <div class="storage-item">
            <span class="storage-item-label">Used by Models</span>
            <span class="storage-item-val font-semibold">
              {{ formatBytes(storageInfo.total_models_size_bytes) }}
            </span>
          </div>

          <div class="storage-item" v-if="storageInfo.free_disk_space_bytes">
            <span class="storage-item-label">Free on Device</span>
            <span class="storage-item-val">
              {{ formatBytes(storageInfo.free_disk_space_bytes) }}
            </span>
          </div>

          <div class="storage-item">
            <span class="storage-item-label">Inference Engine</span>
            <span class="storage-item-val text-brand">llama.cpp (Metal)</span>
          </div>
        </div>

        <!-- Storage Directory with Open Folder (Desktop) and Copy Path Buttons -->
        <div class="storage-path-container">
          <div class="path-details">
            <div class="path-title-row">
              <span class="path-heading">Models Storage Directory:</span>
              <span class="platform-hint" v-if="isDesktop">
                (Click to open folder in Finder / Explorer)
              </span>
            </div>
            <code
              class="path-string clickable-path"
              :title="storageInfo.storage_dir"
              @click="isDesktop ? handleOpenStorageLocation() : copyStoragePath()"
            >
              {{ storageInfo.storage_dir }}
            </code>
          </div>

          <div class="path-actions-group">
            <button
              v-if="isDesktop"
              type="button"
              class="btn-path-action btn-open-folder"
              @click="handleOpenStorageLocation"
              title="Open models folder in native file manager"
              aria-label="Open models folder"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="btn-icon-svg"
              >
                <path
                  d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                ></path>
              </svg>
              <span>Open Folder</span>
            </button>

            <button
              type="button"
              class="btn-path-action btn-copy-path"
              @click="copyStoragePath"
              aria-label="Copy models path"
            >
              <svg
                v-if="!copiedPath"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="btn-icon-svg"
              >
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>{{ copiedPath ? "✓ Copied" : "Copy Path" }}</span>
            </button>
          </div>
        </div>
      </section>

      <!-- Loading State -->
      <div v-if="isLoading" class="loading-box">
        <div class="spinner"></div>
        <span>Checking model statuses...</span>
      </div>

      <!-- Models Section: Highlighted Default Model & Collapsible Alternative Models -->
      <div v-else class="models-architecture-flow">
        <!-- 1. HIGHLIGHTED DEFAULT MODEL SECTION -->
        <section class="default-model-section" v-if="defaultModel">
          <div class="section-label-row">
            <div class="section-label-group">
              <span class="section-label-tag">PRIMARY ENGINE</span>
              <h3 class="default-model-heading">Default Model (Recommended)</h3>
            </div>
            <span class="badge-recommended-pill">★ Recommended</span>
          </div>

          <div
            class="surface-card hero-model-card"
            :class="{
              'card-is-ready': defaultModel.is_downloaded,
              'card-is-downloading':
                defaultModel.is_downloading || activeDownloads[defaultModel.id],
            }"
          >
            <!-- Hero Top Info -->
            <div class="hero-top-block">
              <div class="hero-info-col">
                <div class="hero-title-wrap">
                  <div class="hero-icon-badge">🧠</div>
                  <h4 class="hero-model-name">{{ defaultModel.name }}</h4>
                  <span class="quant-badge">{{ defaultModel.quantization }}</span>
                </div>
                <p class="hero-model-desc">{{ defaultModel.description }}</p>
              </div>

              <!-- Status Badge -->
              <div class="model-status-col">
                <span v-if="defaultModel.is_downloaded" class="status-pill status-ready">
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    class="status-check-icon"
                  >
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                  <span>Ready to Use</span>
                </span>
                <span
                  v-else-if="defaultModel.is_downloading || activeDownloads[defaultModel.id]"
                  class="status-pill status-downloading"
                >
                  <span class="pulse-dot"></span>
                  <span>Downloading</span>
                </span>
                <span v-else class="status-pill status-missing">
                  <span>Available ({{ formatBytes(defaultModel.size_bytes) }})</span>
                </span>
              </div>
            </div>

            <!-- Specs Grid -->
            <div class="model-specs-grid hero-specs-grid">
              <div class="specs-two-col">
                <div class="spec-row">
                  <span class="spec-label">Model Size:</span>
                  <span class="spec-value font-semibold">
                    {{ formatBytes(defaultModel.size_bytes) }}
                  </span>
                </div>
                <div class="spec-row">
                  <span class="spec-label">Memory Footprint:</span>
                  <span class="spec-value">{{ defaultModel.recommended_ram }}</span>
                </div>
              </div>

              <div class="spec-row spec-full">
                <span class="spec-label">Hugging Face:</span>
                <span class="spec-value code-truncate" :title="defaultModel.filename">
                  {{ defaultModel.filename }}
                </span>
              </div>

              <div class="spec-row spec-full" v-if="defaultModel.file_path">
                <span class="spec-label">Local File:</span>
                <span class="spec-value code-truncate path-accent" :title="defaultModel.file_path">
                  {{ defaultModel.file_path }}
                </span>
              </div>
            </div>

            <!-- Active Download Progress Bar -->
            <div
              v-if="activeDownloads[defaultModel.id] || defaultModel.is_downloading"
              class="active-progress-box"
            >
              <div class="progress-info-line">
                <span class="progress-label">
                  {{
                    activeDownloads[defaultModel.id]?.status === "verifying"
                      ? "Verifying Checksum..."
                      : "Downloading from Hugging Face..."
                  }}
                </span>
                <span class="progress-pct">
                  {{ Math.round(activeDownloads[defaultModel.id]?.progress || 0) }}%
                </span>
              </div>

              <div class="progress-bar-track">
                <div
                  class="progress-bar-fill"
                  :style="{
                    width: `${Math.min(
                      Math.max(activeDownloads[defaultModel.id]?.progress || 0, 0),
                      100,
                    )}%`,
                  }"
                ></div>
              </div>

              <div class="progress-stats-line">
                <span>
                  {{
                    formatBytes(
                      activeDownloads[defaultModel.id]?.received || defaultModel.downloaded_bytes,
                    )
                  }}
                  / {{ formatBytes(defaultModel.size_bytes) }}
                </span>
                <span v-if="activeDownloads[defaultModel.id]?.speed">
                  {{ formatSpeed(activeDownloads[defaultModel.id]!.speed) }}
                </span>
              </div>
            </div>

            <!-- Action Buttons for Default Model -->
            <div class="model-actions-wrap">
              <!-- Case 1: Downloading -> Cancel -->
              <template v-if="defaultModel.is_downloading || activeDownloads[defaultModel.id]">
                <button
                  type="button"
                  class="btn-model-action btn-model-cancel"
                  @click="handleCancel(defaultModel.id)"
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    class="btn-action-icon"
                  >
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                  <span>Cancel Download</span>
                </button>
              </template>

              <!-- Case 2: Not Downloaded -> Prominent Download Button -->
              <template v-else-if="!defaultModel.is_downloaded">
                <button
                  type="button"
                  class="btn-model-action btn-model-hero-primary"
                  @click="handleDownload(defaultModel.id)"
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.2"
                    class="btn-action-icon"
                  >
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="7 10 12 15 17 10"></polyline>
                    <line x1="12" y1="15" x2="12" y2="3"></line>
                  </svg>
                  <span
                    >Download Recommended Model ({{ formatBytes(defaultModel.size_bytes) }})</span
                  >
                </button>
              </template>

              <!-- Case 3: Downloaded -> Verify & Delete Buttons -->
              <template v-else>
                <div class="downloaded-btn-grid">
                  <button
                    type="button"
                    class="btn-model-action btn-model-secondary"
                    :disabled="verifyingModelId === defaultModel.id"
                    @click="handleVerify(defaultModel.id)"
                  >
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      class="btn-action-icon"
                    >
                      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
                    </svg>
                    <span>{{
                      verifyingModelId === defaultModel.id ? "Verifying..." : "Verify Checksum"
                    }}</span>
                  </button>

                  <button
                    type="button"
                    class="btn-model-action btn-model-danger"
                    :disabled="deletingModelId === defaultModel.id"
                    @click="handleDelete(defaultModel.id)"
                  >
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      class="btn-action-icon"
                    >
                      <polyline points="3 6 5 6 21 6"></polyline>
                      <path
                        d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                      ></path>
                    </svg>
                    <span>{{
                      deletingModelId === defaultModel.id ? "Deleting..." : "Delete File"
                    }}</span>
                  </button>
                </div>
              </template>
            </div>
          </div>
        </section>

        <!-- 2. COLLAPSIBLE ALTERNATIVE MODELS SECTION -->
        <section class="alternative-models-section" v-if="alternativeModels.length > 0">
          <div class="surface-card accordion-card">
            <!-- Accordion Header Button -->
            <button
              type="button"
              class="accordion-toggle-btn"
              :aria-expanded="showAlternativeModels"
              @click="showAlternativeModels = !showAlternativeModels"
            >
              <div class="accordion-left-content">
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="chevron-icon"
                  :class="{ 'is-rotated': showAlternativeModels }"
                >
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>

                <div class="accordion-title-col">
                  <div class="accordion-heading-row">
                    <h3 class="accordion-heading">Alternative Models</h3>
                    <span class="accordion-count-badge">
                      {{ alternativeModels.length }} options
                    </span>
                  </div>
                  <p class="accordion-subtext">
                    Ultra-lightweight or high-capacity options for specific devices
                  </p>
                </div>
              </div>

              <div class="accordion-right-content">
                <span
                  class="accordion-status-pill"
                  :class="downloadedAlternativeCount > 0 ? 'pill-green' : 'pill-muted'"
                >
                  {{
                    downloadedAlternativeCount > 0
                      ? `${downloadedAlternativeCount} Installed`
                      : "Optional"
                  }}
                </span>
              </div>
            </button>

            <!-- Accordion Expanded Body -->
            <div v-if="showAlternativeModels" class="accordion-body">
              <p class="accordion-intro-note">
                These optional models offer different trade-offs between memory footprint and
                reasoning power. Share2Cal will automatically load the active model.
              </p>

              <div class="alternative-models-stack">
                <div
                  v-for="model in alternativeModels"
                  :key="model.id"
                  class="surface-card alt-model-card"
                  :class="{
                    'card-is-ready': model.is_downloaded,
                    'card-is-downloading': model.is_downloading || activeDownloads[model.id],
                  }"
                >
                  <!-- Header -->
                  <div class="model-top-block">
                    <div class="model-info-col">
                      <div class="model-title-wrap">
                        <h4 class="model-name">{{ model.name }}</h4>
                        <span class="quant-badge">{{ model.quantization }}</span>
                      </div>
                      <p class="model-desc">{{ model.description }}</p>
                    </div>

                    <!-- Status Badge -->
                    <div class="model-status-col">
                      <span v-if="model.is_downloaded" class="status-pill status-ready">
                        <svg
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2.5"
                          class="status-check-icon"
                        >
                          <polyline points="20 6 9 17 4 12"></polyline>
                        </svg>
                        <span>Ready</span>
                      </span>
                      <span
                        v-else-if="model.is_downloading || activeDownloads[model.id]"
                        class="status-pill status-downloading"
                      >
                        <span class="pulse-dot"></span>
                        <span>Downloading</span>
                      </span>
                      <span v-else class="status-pill status-missing">
                        <span>Available</span>
                      </span>
                    </div>
                  </div>

                  <!-- Specs Grid -->
                  <div class="model-specs-grid">
                    <div class="specs-two-col">
                      <div class="spec-row">
                        <span class="spec-label">Size:</span>
                        <span class="spec-value font-semibold">
                          {{ formatBytes(model.size_bytes) }}
                        </span>
                      </div>
                      <div class="spec-row">
                        <span class="spec-label">RAM:</span>
                        <span class="spec-value">{{ model.recommended_ram }}</span>
                      </div>
                    </div>

                    <div class="spec-row spec-full">
                      <span class="spec-label">Hugging Face:</span>
                      <span class="spec-value code-truncate" :title="model.filename">
                        {{ model.filename }}
                      </span>
                    </div>

                    <div class="spec-row spec-full" v-if="model.file_path">
                      <span class="spec-label">Local File:</span>
                      <span class="spec-value code-truncate path-accent" :title="model.file_path">
                        {{ model.file_path }}
                      </span>
                    </div>
                  </div>

                  <!-- Download Progress Bar -->
                  <div
                    v-if="activeDownloads[model.id] || model.is_downloading"
                    class="active-progress-box"
                  >
                    <div class="progress-info-line">
                      <span class="progress-label">
                        {{
                          activeDownloads[model.id]?.status === "verifying"
                            ? "Verifying Checksum..."
                            : "Downloading from Hugging Face..."
                        }}
                      </span>
                      <span class="progress-pct">
                        {{ Math.round(activeDownloads[model.id]?.progress || 0) }}%
                      </span>
                    </div>

                    <div class="progress-bar-track">
                      <div
                        class="progress-bar-fill"
                        :style="{
                          width: `${Math.min(
                            Math.max(activeDownloads[model.id]?.progress || 0, 0),
                            100,
                          )}%`,
                        }"
                      ></div>
                    </div>

                    <div class="progress-stats-line">
                      <span>
                        {{
                          formatBytes(activeDownloads[model.id]?.received || model.downloaded_bytes)
                        }}
                        / {{ formatBytes(model.size_bytes) }}
                      </span>
                      <span v-if="activeDownloads[model.id]?.speed">
                        {{ formatSpeed(activeDownloads[model.id]!.speed) }}
                      </span>
                    </div>
                  </div>

                  <!-- Actions -->
                  <div class="model-actions-wrap">
                    <template v-if="model.is_downloading || activeDownloads[model.id]">
                      <button
                        type="button"
                        class="btn-model-action btn-model-cancel"
                        @click="handleCancel(model.id)"
                      >
                        <svg
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          class="btn-action-icon"
                        >
                          <line x1="18" y1="6" x2="6" y2="18"></line>
                          <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                        <span>Cancel Download</span>
                      </button>
                    </template>

                    <template v-else-if="!model.is_downloaded">
                      <button
                        type="button"
                        class="btn-model-action btn-model-primary"
                        @click="handleDownload(model.id)"
                      >
                        <svg
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2.2"
                          class="btn-action-icon"
                        >
                          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                          <polyline points="7 10 12 15 17 10"></polyline>
                          <line x1="12" y1="15" x2="12" y2="3"></line>
                        </svg>
                        <span>Download ({{ formatBytes(model.size_bytes) }})</span>
                      </button>
                    </template>

                    <template v-else>
                      <div class="downloaded-btn-grid">
                        <button
                          type="button"
                          class="btn-model-action btn-model-secondary"
                          :disabled="verifyingModelId === model.id"
                          @click="handleVerify(model.id)"
                        >
                          <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            class="btn-action-icon"
                          >
                            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
                          </svg>
                          <span>{{
                            verifyingModelId === model.id ? "Verifying..." : "Verify"
                          }}</span>
                        </button>

                        <button
                          type="button"
                          class="btn-model-action btn-model-danger"
                          :disabled="deletingModelId === model.id"
                          @click="handleDelete(model.id)"
                        >
                          <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            class="btn-action-icon"
                          >
                            <polyline points="3 6 5 6 21 6"></polyline>
                            <path
                              d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                            ></path>
                          </svg>
                          <span>{{ deletingModelId === model.id ? "Deleting..." : "Delete" }}</span>
                        </button>
                      </div>
                    </template>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>

    <!-- Bottom Privacy & Safety Note -->
    <footer class="settings-footer">
      <p class="footer-hint">
        Share2Cal performs OCR and AI text extraction 100% locally on your device. Native calendar
        events are saved directly to your local device storage. If you choose Google Calendar, event
        parameters are opened in your web browser with Google.
      </p>
    </footer>
  </div>
</template>

<style scoped>
.settings-page {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  box-sizing: border-box;
}

/* Nav Bar */
.settings-nav-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.25rem 0.25rem 0.5rem;
  position: relative;
}

.btn-back {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 999px;
  padding: 0.45rem 0.85rem 0.45rem 0.6rem;
  color: var(--accent-primary);
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  box-shadow: var(--shadow-subtle);
  transition: all 0.15s ease;
  min-height: 38px;
}

.btn-back:hover {
  background: var(--bg-input);
  transform: translateX(-1px);
}

.nav-arrow-icon {
  width: 18px;
  height: 18px;
}

.header-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  text-align: center;
  pointer-events: none;
}

.settings-title {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.nav-spacer {
  width: 70px;
}

/* Feedback Banners */
.banner {
  padding: 0.75rem 1rem;
  border-radius: 14px;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.85rem;
  box-sizing: border-box;
}

.banner-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.banner-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: break-word;
}

.banner-success {
  background: rgba(52, 199, 89, 0.12);
  color: #34c759;
  border: 1px solid rgba(52, 199, 89, 0.25);
}

.banner-error {
  background: rgba(255, 59, 48, 0.12);
  color: #ff3b30;
  border: 1px solid rgba(255, 59, 48, 0.25);
}

.banner-close {
  background: none;
  border: none;
  color: currentColor;
  cursor: pointer;
  padding: 0.2rem;
  font-size: 0.9rem;
  opacity: 0.7;
}

.banner-close:hover {
  opacity: 1;
}

/* Surface Card Foundation */
.surface-card {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card, 20px);
  padding: 1.25rem;
  box-shadow: var(--shadow-card);
  box-sizing: border-box;
  width: 100%;
  max-width: 100%;
}

/* Section Header */
.section-top {
  margin-bottom: 1rem;
}

.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.section-heading-col {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  flex: 1;
  min-width: 0;
}

.heading-with-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.section-icon-bubble {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: rgba(0, 122, 255, 0.12);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.bubble-calendar {
  background: rgba(52, 199, 89, 0.12);
  color: #34c759;
}

.bubble-mode {
  background: linear-gradient(135deg, rgba(88, 86, 214, 0.15) 0%, rgba(0, 122, 255, 0.15) 100%);
  color: var(--accent-primary);
}

.section-icon-bubble svg {
  width: 20px;
  height: 20px;
}

.section-heading {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.active-mode-chip {
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.chip-simple {
  background: rgba(0, 122, 255, 0.12);
  color: var(--accent-primary);
  border: 1px solid rgba(0, 122, 255, 0.2);
}

.chip-enhanced {
  background: linear-gradient(135deg, rgba(88, 86, 214, 0.15) 0%, rgba(0, 122, 255, 0.15) 100%);
  color: #5856d6;
  border: 1px solid rgba(88, 86, 214, 0.25);
}

.section-subheading {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.35;
}

.setting-subsection {
  margin-top: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.subsection-label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: -0.01em;
}

/* Destination Segmented Toggle */
.toggle-track {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  border-radius: 14px;
  margin-bottom: 0.5rem;
  box-sizing: border-box;
}

.target-toggle-track {
  grid-template-columns: 1fr 1fr 1fr;
}

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.65rem 0.75rem;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.88rem;
  font-weight: 600;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  min-height: 42px;
}

.toggle-btn.is-active {
  background: var(--bg-card);
  color: var(--accent-primary);
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.08),
    0 1px 2px rgba(0, 0, 0, 0.04);
}

.toggle-icon {
  width: 17px;
  height: 17px;
  flex-shrink: 0;
}

.toggle-label {
  letter-spacing: -0.01em;
}

/* Calendar dropdown */
.calendar-picker-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.settings-select {
  width: 100%;
  padding: 0.75rem 0.9rem;
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 12px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.settings-select:focus {
  border-color: var(--accent-primary);
}

.permission-prompt-box {
  padding: 0.85rem 1rem;
  background: var(--bg-input);
  border: 1px dashed var(--border-input);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  align-items: flex-start;
}

.prompt-text {
  margin: 0;
  font-size: 0.82rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

.btn-grant-permission {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--accent-primary);
  color: #fff;
  border: none;
  border-radius: 8px;
  padding: 0.45rem 0.85rem;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.btn-grant-permission:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* MODE SELECTION CARDS (Requirement 1) */
.mode-selector-container {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin-top: 0.5rem;
}

.mode-cards-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
  width: 100%;
  box-sizing: border-box;
}

.mode-choice-card {
  background: var(--bg-input);
  border: 2px solid var(--border-card-subtle, rgba(0, 0, 0, 0.06));
  border-radius: 16px;
  padding: 1rem;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  box-sizing: border-box;
  outline: none;
  position: relative;
}

.mode-choice-card:hover {
  background: rgba(0, 122, 255, 0.03);
  border-color: rgba(0, 122, 255, 0.3);
  transform: translateY(-1px);
}

.mode-choice-card.is-selected {
  background: var(--bg-card);
  border-color: var(--accent-primary);
  box-shadow:
    0 4px 16px rgba(0, 122, 255, 0.12),
    0 1px 3px rgba(0, 122, 255, 0.08);
}

.choice-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.5rem;
}

.choice-icon-label {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
}

.choice-symbol {
  font-size: 1.35rem;
  line-height: 1;
}

.choice-title {
  font-size: 0.98rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.choice-tagline {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary);
  display: block;
  margin-top: 0.1rem;
}

.radio-indicator {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--border-card);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s ease;
  background: var(--bg-card);
}

.radio-indicator.is-checked {
  border-color: var(--accent-primary);
  background: var(--accent-primary);
}

.radio-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ffffff;
}

.choice-desc {
  font-size: 0.78rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.4;
  flex: 1;
}

.choice-tags-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  padding-top: 0.2rem;
}

.choice-pill {
  font-size: 0.68rem;
  font-weight: 600;
  padding: 0.2rem 0.45rem;
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
}

.mode-choice-card.is-selected .choice-pill {
  background: rgba(0, 122, 255, 0.08);
  color: var(--accent-primary);
  border-color: rgba(0, 122, 255, 0.15);
}

/* Active Mode Callout */
.active-mode-callout {
  margin-top: 1rem;
  padding: 0.9rem 1.1rem;
  border-radius: 14px;
  box-sizing: border-box;
}

.callout-simple {
  background: rgba(0, 122, 255, 0.06);
  border: 1px solid rgba(0, 122, 255, 0.2);
}

.callout-enhanced {
  background: linear-gradient(135deg, rgba(88, 86, 214, 0.08) 0%, rgba(0, 122, 255, 0.08) 100%);
  border: 1px solid rgba(88, 86, 214, 0.22);
}

.callout-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.3rem;
  flex-wrap: wrap;
}

.callout-icon {
  font-size: 1rem;
}

.callout-title {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--text-primary);
}

.ready-indicator-tag {
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
  margin-left: auto;
}

.callout-text {
  font-size: 0.82rem;
  color: var(--text-secondary);
  line-height: 1.45;
  margin: 0;
}

/* ENHANCED MODE ARCHITECTURE */
.enhanced-models-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  animation: fadeIn 0.2s ease-out;
}

/* Storage Card */
.storage-card {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.storage-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.storage-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.storage-title {
  font-size: 0.8rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
}

.storage-status-pill {
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
}

.pill-green {
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.pill-gray {
  background: rgba(142, 142, 147, 0.15);
  color: #8e8e93;
}

.pill-muted {
  background: var(--bg-input);
  color: var(--text-secondary);
}

.storage-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
  gap: 0.65rem;
  box-sizing: border-box;
}

.storage-item {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  background: var(--bg-input);
  padding: 0.6rem 0.75rem;
  border-radius: 10px;
  min-width: 0;
}

.storage-item-label {
  font-size: 0.7rem;
  color: var(--text-secondary);
  white-space: nowrap;
}

.storage-item-val {
  font-size: 0.85rem;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.text-brand {
  color: var(--accent-primary);
  font-weight: 600;
}

/* Storage Path & Actions (Requirement 3) */
.storage-path-container {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  box-sizing: border-box;
  width: 100%;
  flex-wrap: wrap;
}

.path-details {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  flex: 1 1 240px;
}

.path-title-row {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.path-heading {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.platform-hint {
  font-size: 0.68rem;
  color: var(--accent-primary);
}

.path-string {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.72rem;
  color: var(--text-primary);
  background: var(--bg-input);
  padding: 0.35rem 0.55rem;
  border-radius: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
  max-width: 100%;
  box-sizing: border-box;
  border: 1px solid var(--border-input);
}

.clickable-path {
  cursor: pointer;
  transition: all 0.15s ease;
}

.clickable-path:hover {
  background: rgba(0, 122, 255, 0.08);
  border-color: rgba(0, 122, 255, 0.3);
}

.path-actions-group {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-shrink: 0;
}

.btn-path-action {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.76rem;
  font-weight: 600;
  padding: 0.4rem 0.75rem;
  background: var(--bg-input);
  border: 1px solid var(--border-card);
  border-radius: 8px;
  color: var(--text-primary);
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
  min-height: 34px;
}

.btn-path-action:hover {
  background: rgba(0, 122, 255, 0.08);
  border-color: var(--accent-primary);
  color: var(--accent-primary);
}

.btn-open-folder {
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  border-color: rgba(0, 122, 255, 0.25);
}

.btn-open-folder:hover {
  background: var(--accent-primary);
  color: #ffffff;
}

.btn-icon-svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

/* Models Architecture Container */
.models-architecture-flow {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* HIGHLIGHTED DEFAULT MODEL SECTION (Requirement 2) */
.default-model-section {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.section-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0.25rem;
}

.section-label-group {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.section-label-tag {
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  color: var(--accent-primary);
}

.default-model-heading {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.badge-recommended-pill {
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.2rem 0.6rem;
  border-radius: 999px;
  background: linear-gradient(135deg, rgba(0, 122, 255, 0.15) 0%, rgba(88, 86, 214, 0.15) 100%);
  color: var(--accent-primary);
  border: 1px solid rgba(0, 122, 255, 0.25);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

/* Hero Default Model Card */
.hero-model-card {
  border: 2px solid rgba(0, 122, 255, 0.35);
  background: linear-gradient(180deg, var(--bg-card) 0%, rgba(0, 122, 255, 0.02) 100%);
  box-shadow:
    0 6px 24px rgba(0, 122, 255, 0.08),
    0 1px 3px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.35rem;
}

.hero-model-card.card-is-ready {
  border-color: rgba(52, 199, 89, 0.45);
  box-shadow:
    0 6px 24px rgba(52, 199, 89, 0.1),
    0 1px 3px rgba(0, 0, 0, 0.04);
}

.hero-top-block {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.85rem;
}

.hero-info-col {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  flex: 1;
  min-width: 0;
}

.hero-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.hero-icon-badge {
  font-size: 1.25rem;
  line-height: 1;
}

.hero-model-name {
  font-size: 1.15rem;
  font-weight: 800;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.015em;
}

.hero-model-desc {
  font-size: 0.84rem;
  color: var(--text-secondary);
  line-height: 1.45;
  margin: 0;
}

.btn-model-hero-primary {
  background: linear-gradient(135deg, #007aff 0%, #0062cc 100%);
  color: #ffffff;
  border: none;
  font-size: 0.92rem;
  font-weight: 700;
  padding: 0.8rem 1.2rem;
  border-radius: 14px;
  box-shadow: 0 4px 14px rgba(0, 122, 255, 0.3);
}

.btn-model-hero-primary:hover {
  background: linear-gradient(135deg, #006ae6 0%, #0056b3 100%);
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(0, 122, 255, 0.38);
}

/* COLLAPSIBLE ALTERNATIVE MODELS SECTION (Requirement 2) */
.alternative-models-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.accordion-card {
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--border-card);
}

.accordion-toggle-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.85rem;
  padding: 1.1rem 1.25rem;
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease;
}

.accordion-toggle-btn:hover {
  background: rgba(0, 0, 0, 0.02);
}

.accordion-left-content {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  flex: 1;
}

.chevron-icon {
  width: 20px;
  height: 20px;
  color: var(--text-secondary);
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  flex-shrink: 0;
}

.chevron-icon.is-rotated {
  transform: rotate(180deg);
}

.accordion-title-col {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}

.accordion-heading-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.accordion-heading {
  font-size: 0.98rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.accordion-count-badge {
  font-size: 0.7rem;
  font-weight: 600;
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-secondary);
}

.accordion-subtext {
  font-size: 0.78rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.35;
}

.accordion-right-content {
  flex-shrink: 0;
}

.accordion-status-pill {
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.25rem 0.6rem;
  border-radius: 999px;
}

.accordion-body {
  padding: 0 1.25rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  border-top: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  padding-top: 1rem;
}

.accordion-intro-note {
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.4;
  margin: 0;
}

.alternative-models-stack {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.alt-model-card {
  background: var(--bg-input);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.08));
  border-radius: 14px;
  padding: 1.1rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

/* Specs Grid */
.model-specs-grid {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.65rem 0.8rem;
  background: var(--bg-input);
  border-radius: 10px;
  box-sizing: border-box;
  width: 100%;
}

.hero-specs-grid {
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
}

.specs-two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
}

.spec-row {
  display: flex;
  align-items: baseline;
  gap: 0.45rem;
  font-size: 0.74rem;
  min-width: 0;
}

.spec-label {
  color: var(--text-secondary);
  white-space: nowrap;
  flex-shrink: 0;
}

.spec-value {
  color: var(--text-primary);
  min-width: 0;
}

.code-truncate {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.7rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
}

.path-accent {
  color: var(--accent-primary);
}

.quant-badge {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.15rem 0.4rem;
  background: var(--bg-input);
  border-radius: 6px;
  color: var(--text-secondary);
}

.model-top-block {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.model-info-col {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 0;
  flex: 1;
}

.model-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.model-name {
  font-size: 1.02rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.model-desc {
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.4;
  margin: 0;
  overflow-wrap: break-word;
}

.model-status-col {
  flex-shrink: 0;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.25rem 0.55rem;
  border-radius: 999px;
  white-space: nowrap;
}

.status-check-icon {
  width: 13px;
  height: 13px;
}

.status-ready {
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.status-downloading {
  background: rgba(0, 122, 255, 0.15);
  color: var(--accent-primary);
}

.status-missing {
  background: var(--bg-input);
  color: var(--text-secondary);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
}

.pulse-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent-primary);
  animation: pulse 1.2s infinite;
}

/* Active Progress Bar */
.active-progress-box {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.65rem 0.85rem;
  background: rgba(0, 122, 255, 0.06);
  border: 1px solid rgba(0, 122, 255, 0.2);
  border-radius: 10px;
  box-sizing: border-box;
  width: 100%;
}

.progress-info-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.74rem;
  gap: 0.5rem;
}

.progress-label {
  color: var(--accent-primary);
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.progress-pct {
  color: var(--accent-primary);
  font-weight: 700;
  flex-shrink: 0;
}

.progress-bar-track {
  height: 7px;
  background: rgba(0, 122, 255, 0.15);
  border-radius: 999px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: var(--accent-primary);
  border-radius: 999px;
  transition: width 0.2s ease;
}

.progress-stats-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.7rem;
  color: var(--text-secondary);
}

/* Action Buttons */
.model-actions-wrap {
  width: 100%;
}

.btn-model-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  padding: 0.7rem 1rem;
  border-radius: 12px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  min-height: 44px;
  width: 100%;
  box-sizing: border-box;
  text-align: center;
}

.btn-action-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.btn-model-primary {
  background: var(--accent-primary);
  color: #ffffff;
  border: none;
  box-shadow: 0 4px 12px rgba(0, 122, 255, 0.25);
}

.btn-model-primary:hover {
  background: var(--accent-primary-hover, #0066d6);
  transform: translateY(-1px);
}

.btn-model-cancel {
  background: rgba(255, 59, 48, 0.1);
  color: var(--accent-danger);
  border: 1px solid rgba(255, 59, 48, 0.25);
}

.btn-model-cancel:hover {
  background: rgba(255, 59, 48, 0.18);
}

.downloaded-btn-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.65rem;
  width: 100%;
}

.btn-model-secondary {
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-card);
}

.btn-model-secondary:hover {
  background: rgba(0, 0, 0, 0.06);
}

.btn-model-danger {
  background: rgba(255, 59, 48, 0.08);
  color: var(--accent-danger);
  border: 1px solid rgba(255, 59, 48, 0.2);
}

.btn-model-danger:hover {
  background: rgba(255, 59, 48, 0.15);
}

.loading-box {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 2.5rem;
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid rgba(0, 122, 255, 0.2);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* Footer */
.settings-footer {
  padding: 0.5rem 0.5rem 1.5rem;
  text-align: center;
}

.footer-hint {
  font-size: 0.72rem;
  color: var(--text-tertiary, #9ca3af);
  line-height: 1.45;
  margin: 0;
}

/* Animations */
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.4;
    transform: scale(0.85);
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Responsive adjustments */
@media (max-width: 500px) {
  .mode-cards-grid {
    grid-template-columns: 1fr;
  }

  .specs-two-col {
    grid-template-columns: 1fr;
  }

  .downloaded-btn-grid {
    grid-template-columns: 1fr;
  }

  .storage-path-container {
    flex-direction: column;
    align-items: stretch;
  }

  .path-actions-group {
    justify-content: flex-end;
  }
}
</style>
