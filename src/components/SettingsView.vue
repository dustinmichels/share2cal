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

let unlistenProgress: (() => void) | null = null;

async function loadCalendarData() {
  try {
    const [status, cals] = await Promise.all([
      checkCalendarPermission(),
      getAvailableCalendars(),
    ]);
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
    target === "native" ? "Apple / Device Calendar" : target === "google" ? "Google Calendar" : ".ics Export"
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
    actionError.value = `Download failed: ${err instanceof Error ? err.message : String(err)}`;
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

const readyModelCount = computed(() => {
  return models.value.filter((m) => m.is_downloaded).length;
});

function setMode(mode: ParsingMode) {
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
        <div class="toggle-track target-toggle-track" role="tablist" aria-label="Default calendar destination">
          <button
            type="button"
            class="toggle-btn"
            :class="{ 'is-active': defaultCalendarTarget === 'native' }"
            role="tab"
            :aria-selected="defaultCalendarTarget === 'native'"
            @click="handleTargetChange('native')"
          >
            <svg class="toggle-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
            <svg class="toggle-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
            <svg class="toggle-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
            <option
              v-for="cal in availableCalendars"
              :key="cal.id"
              :value="cal.id"
            >
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
            v-if="calendarPermissionStatus === 'not_determined' || calendarPermissionStatus === 'write_only' || calendarPermissionStatus === 'unknown'"
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

    <!-- MAIN SETTING CARD: Text Parsing -->
    <section class="surface-card setting-section-card">
      <div class="section-top">
        <div class="section-title-wrap">
          <div class="section-icon-bubble">
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
          <div>
            <h2 class="section-heading">Text Parsing</h2>
            <p class="section-subheading">Select your extraction engine preference</p>
          </div>
        </div>
      </div>

      <!-- Segmented Toggle: Simple vs Enhanced -->
      <div class="toggle-track" role="tablist" aria-label="Text parsing mode">
        <button
          type="button"
          class="toggle-btn"
          :class="{ 'is-active': parsingMode === 'simple' }"
          role="tab"
          :aria-selected="parsingMode === 'simple'"
          @click="setMode('simple')"
        >
          <svg
            class="toggle-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
          </svg>
          <span class="toggle-label">Simple</span>
        </button>

        <button
          type="button"
          class="toggle-btn"
          :class="{ 'is-active': parsingMode === 'enhanced' }"
          role="tab"
          :aria-selected="parsingMode === 'enhanced'"
          @click="setMode('enhanced')"
        >
          <svg
            class="toggle-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z"></path>
            <path d="M6 10a6 6 0 0 0 12 0"></path>
            <line x1="12" y1="16" x2="12" y2="22"></line>
            <line x1="8" y1="22" x2="16" y2="22"></line>
          </svg>
          <span class="toggle-label">Enhanced</span>
        </button>
      </div>

      <!-- Dynamic Mode Description Callout -->
      <div
        class="mode-description-box"
        :class="parsingMode === 'enhanced' ? 'box-enhanced' : 'box-simple'"
      >
        <div class="desc-header-row">
          <span class="mode-badge">
            {{ parsingMode === "enhanced" ? "Enhanced Mode" : "Simple Mode" }}
          </span>
          <span v-if="parsingMode === 'enhanced' && readyModelCount > 0" class="ready-tag">
            ✓ Model Ready
          </span>
        </div>

        <p class="mode-desc-text">
          <template v-if="parsingMode === 'simple'">
            use basic rules for text parsing. quick and easy and lightweight, but less accurate.
          </template>
          <template v-else>
            use a tiny LLM, running locally on your phone, to parse the text
          </template>
        </p>

        <!-- Feature highlights list -->
        <div class="mode-features-list">
          <template v-if="parsingMode === 'simple'">
            <div class="feature-chip">
              <span class="feature-bullet">⚡</span>
              <span>Lightweight & instant</span>
            </div>
            <div class="feature-chip">
              <span class="feature-bullet">📦</span>
              <span>No model download needed</span>
            </div>
          </template>
          <template v-else>
            <div class="feature-chip">
              <span class="feature-bullet">🧠</span>
              <span>Contextual AI reasoning</span>
            </div>
            <div class="feature-chip">
              <span class="feature-bullet">🔒</span>
              <span>100% on-device & private</span>
            </div>
          </template>
        </div>
      </div>
    </section>

    <!-- ENHANCED MODE SECTION: Options for downloading models -->
    <div v-if="parsingMode === 'enhanced'" class="enhanced-models-flow">
      <!-- Storage Overview Card -->
      <section class="surface-card storage-card" v-if="storageInfo">
        <div class="storage-top">
          <div class="storage-title-row">
            <span class="storage-title">Storage & Engine</span>
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

        <!-- Storage Directory with Copy Button (Safe for Mobile Overflow) -->
        <div class="storage-path-container">
          <div class="path-details">
            <span class="path-heading">Local App Path:</span>
            <code class="path-string" :title="storageInfo.storage_dir">
              {{ storageInfo.storage_dir }}
            </code>
          </div>
          <button
            type="button"
            class="btn-copy-path"
            @click="copyStoragePath"
            aria-label="Copy path"
          >
            {{ copiedPath ? "✓ Copied" : "Copy" }}
          </button>
        </div>
      </section>

      <!-- Model Options Header & List -->
      <section class="models-options-section">
        <div class="models-header-row">
          <h3 class="models-header-title">On-Device Models</h3>
          <span class="models-header-sub">Hugging Face GGUF</span>
        </div>

        <div v-if="isLoading" class="loading-box">
          <div class="spinner"></div>
          <span>Checking model statuses...</span>
        </div>

        <div v-else class="models-stack">
          <div
            v-for="model in models"
            :key="model.id"
            class="surface-card model-option-card"
            :class="{
              'card-is-ready': model.is_downloaded,
              'card-is-downloading': model.is_downloading || activeDownloads[model.id],
            }"
          >
            <!-- Header with Title, Quantization, and Status -->
            <div class="model-top-block">
              <div class="model-info-col">
                <div class="model-title-wrap">
                  <h4 class="model-name">{{ model.name }}</h4>
                  <span class="quant-badge">{{ model.quantization }}</span>
                  <span v-if="model.is_default" class="badge-rec"> Recommended </span>
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

            <!-- Specs Grid (Mobile Optimized) -->
            <div class="model-specs-grid">
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

            <!-- Download Progress Bar (When Active) -->
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
                  {{ formatBytes(activeDownloads[model.id]?.received || model.downloaded_bytes) }}
                  / {{ formatBytes(model.size_bytes) }}
                </span>
                <span v-if="activeDownloads[model.id]?.speed">
                  {{ formatSpeed(activeDownloads[model.id]!.speed) }}
                </span>
              </div>
            </div>

            <!-- Action Buttons (Mobile Friendly Full Width) -->
            <div class="model-actions-wrap">
              <!-- Case 1: Downloading -> Cancel -->
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

              <!-- Case 2: Not Downloaded -> Download -->
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
                  <span>Download Model ({{ formatBytes(model.size_bytes) }})</span>
                </button>
              </template>

              <!-- Case 3: Downloaded -> Verify & Delete -->
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
                      verifyingModelId === model.id ? "Verifying..." : "Verify Checksum"
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
      </section>
    </div>

    <!-- Bottom Privacy & Safety Note -->
    <footer class="settings-footer">
      <p class="footer-hint">
        Share2Cal performs OCR and AI text extraction 100% locally on your device. Native calendar events are saved directly to your local device storage. If you choose Google Calendar, event parameters are opened in your web browser with Google.
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

.section-icon-bubble {
  width: 40px;
  height: 40px;
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

.setting-subsection {
  margin-top: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.subsection-label {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.target-toggle-track {
  grid-template-columns: 1fr 1fr 1fr;
  margin-bottom: 0.25rem;
}

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
.section-subheading {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin: 0.15rem 0 0;
}

/* Toggle Segmented Track */
.toggle-track {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  border-radius: 14px;
  margin-bottom: 1rem;
  box-sizing: border-box;
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
  font-size: 0.9rem;
  font-weight: 600;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  min-height: 44px;
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

/* Mode Description Box */
.mode-description-box {
  padding: 1rem 1.15rem;
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  transition: all 0.2s ease;
  box-sizing: border-box;
  width: 100%;
}

.box-simple {
  background: rgba(0, 122, 255, 0.05);
  border: 1px solid rgba(0, 122, 255, 0.15);
}

.box-enhanced {
  background: linear-gradient(135deg, rgba(88, 86, 214, 0.08) 0%, rgba(0, 122, 255, 0.08) 100%);
  border: 1px solid rgba(88, 86, 214, 0.2);
}

.desc-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.mode-badge {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--accent-primary);
}

.ready-tag {
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.mode-desc-text {
  font-size: 0.88rem;
  line-height: 1.45;
  color: var(--text-primary);
  margin: 0;
  overflow-wrap: break-word;
}

.mode-features-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  padding-top: 0.35rem;
}

.feature-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--bg-card);
  padding: 0.25rem 0.55rem;
  border-radius: 8px;
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.04));
}

.feature-bullet {
  font-size: 0.8rem;
}

/* Enhanced Flow */
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

.storage-path-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  padding-top: 0.65rem;
  border-top: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  box-sizing: border-box;
  width: 100%;
}

.path-details {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
  flex: 1;
}

.path-heading {
  font-size: 0.68rem;
  color: var(--text-secondary);
}

.path-string {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.72rem;
  color: var(--text-primary);
  background: var(--bg-input);
  padding: 0.2rem 0.45rem;
  border-radius: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
  max-width: 100%;
  box-sizing: border-box;
}

.btn-copy-path {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.35rem 0.65rem;
  background: var(--bg-input);
  border: 1px solid var(--border-card);
  border-radius: 8px;
  color: var(--accent-primary);
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.btn-copy-path:hover {
  background: rgba(0, 122, 255, 0.1);
}

/* Models Options Section */
.models-options-section {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.models-header-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 0 0.25rem;
}

.models-header-title {
  font-size: 0.85rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
  margin: 0;
}

.models-header-sub {
  font-size: 0.72rem;
  color: var(--text-tertiary, #9ca3af);
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

.models-stack {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

/* Model Option Card */
.model-option-card {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  transition: all 0.2s ease;
}

.card-is-ready {
  border-color: rgba(52, 199, 89, 0.35);
  box-shadow: 0 4px 18px rgba(52, 199, 89, 0.08);
}

.card-is-downloading {
  border-color: rgba(0, 122, 255, 0.35);
  box-shadow: 0 4px 18px rgba(0, 122, 255, 0.08);
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
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  letter-spacing: -0.01em;
}

.quant-badge {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.15rem 0.4rem;
  background: var(--bg-input);
  border-radius: 6px;
  color: var(--text-secondary);
}

.badge-rec {
  font-size: 0.65rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  background: rgba(0, 122, 255, 0.12);
  color: var(--accent-primary);
  border-radius: 6px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
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
}

.pulse-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent-primary);
  animation: pulse 1.2s infinite;
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

/* Actions */
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

@media (max-width: 420px) {
  .downloaded-btn-grid {
    grid-template-columns: 1fr;
  }
}
</style>
