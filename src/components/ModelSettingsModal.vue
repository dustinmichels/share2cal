<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
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
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "modelsUpdated"): void;
}>();

const models = ref<ModelStatus[]>([]);
const storageInfo = ref<ModelsStorageInfo | null>(null);
const isLoading = ref(true);
const actionError = ref<string | null>(null);
const actionSuccess = ref<string | null>(null);
const verifyingModelId = ref<string | null>(null);
const deletingModelId = ref<string | null>(null);
const activeDownloads = ref<Record<string, { progress: number; speed: number; received: number; total: number; status: string }>>({});
const copiedPath = ref(false);

let unlistenProgress: (() => void) | null = null;

async function loadData() {
  isLoading.value = true;
  actionError.value = null;
  try {
    const [statuses, info] = await Promise.all([
      getModelStatuses(),
      getModelsStorageInfo(),
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

    // Update model status in list
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
      // Fallback
    }
  }
}

const readyModelCount = computed(() => {
  return models.value.filter((m) => m.is_downloaded).length;
});
</script>

<template>
  <div v-if="isOpen" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-panel" role="dialog" aria-modal="true" aria-labelledby="modal-title">
      <!-- Modal Header -->
      <div class="modal-header">
        <div class="modal-title-group">
          <div class="modal-icon-bubble">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z"></path>
              <path d="M6 10a6 6 0 0 0 12 0"></path>
              <line x1="12" y1="16" x2="12" y2="22"></line>
              <line x1="8" y1="22" x2="16" y2="22"></line>
            </svg>
          </div>
          <div>
            <h2 id="modal-title" class="modal-title">On-Device AI Models</h2>
            <p class="modal-subtitle">Download local GGUF models from Hugging Face for 100% offline event extraction</p>
          </div>
        </div>
        <button type="button" class="btn-close" @click="emit('close')" aria-label="Close Settings">✕</button>
      </div>

      <!-- Feedback Banners -->
      <div v-if="actionSuccess" class="banner banner-success">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="banner-icon">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
        <span>{{ actionSuccess }}</span>
        <button type="button" class="banner-close" @click="actionSuccess = null">✕</button>
      </div>

      <div v-if="actionError" class="banner banner-error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="banner-icon">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
        <span>{{ actionError }}</span>
        <button type="button" class="banner-close" @click="actionError = null">✕</button>
      </div>

      <!-- Storage Overview Card -->
      <div class="storage-overview-card" v-if="storageInfo">
        <div class="storage-header">
          <span class="storage-label">Local Storage Status</span>
          <span class="storage-badge" :class="readyModelCount > 0 ? 'badge-active' : 'badge-empty'">
            {{ readyModelCount > 0 ? `${readyModelCount} Model Ready` : 'No Model Downloaded' }}
          </span>
        </div>

        <div class="storage-stats-grid">
          <div class="stat-item">
            <span class="stat-label">Model Storage Used</span>
            <span class="stat-val font-semibold">{{ formatBytes(storageInfo.total_models_size_bytes) }}</span>
          </div>
          <div class="stat-item" v-if="storageInfo.free_disk_space_bytes">
            <span class="stat-label">Free Space on Device</span>
            <span class="stat-val">{{ formatBytes(storageInfo.free_disk_space_bytes) }}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Engine Backend</span>
            <span class="stat-val text-brand">llama.cpp (Metal GPU)</span>
          </div>
        </div>

        <!-- Directory Path Display -->
        <div class="storage-path-box">
          <div class="path-info">
            <span class="path-label">Storage Location:</span>
            <code class="path-code" :title="storageInfo.storage_dir">{{ storageInfo.storage_dir }}</code>
          </div>
          <button type="button" class="btn-copy-path" @click="copyStoragePath">
            {{ copiedPath ? '✓ Copied' : 'Copy Path' }}
          </button>
        </div>
      </div>

      <!-- Models List Section -->
      <div class="models-section">
        <h3 class="section-heading">Available Models</h3>

        <div v-if="isLoading" class="loading-state">
          <div class="spinner"></div>
          <span>Checking on-device model files...</span>
        </div>

        <div v-else class="models-list">
          <div
            v-for="model in models"
            :key="model.id"
            class="model-card"
            :class="{ 'card-active': model.is_downloaded, 'card-downloading': model.is_downloading }"
          >
            <!-- Card Header -->
            <div class="card-header">
              <div class="model-name-group">
                <div class="name-title-row">
                  <h4 class="model-title">{{ model.name }}</h4>
                  <span class="quant-chip">{{ model.quantization }}</span>
                  <span v-if="model.is_default" class="badge-recommended">Recommended</span>
                </div>
                <p class="model-desc">{{ model.description }}</p>
              </div>

              <!-- Status Badge -->
              <div class="status-indicator">
                <span
                  v-if="model.is_downloaded"
                  class="status-chip chip-ready"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="status-svg">
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                  Ready on Device
                </span>
                <span
                  v-else-if="model.is_downloading"
                  class="status-chip chip-downloading"
                >
                  <span class="pulse-dot"></span>
                  Downloading
                </span>
                <span
                  v-else
                  class="status-chip chip-missing"
                >
                  Not Downloaded
                </span>
              </div>
            </div>

            <!-- Model Details Meta Grid -->
            <div class="model-meta-grid">
              <div class="meta-row">
                <span class="meta-key">Download Size:</span>
                <span class="meta-val">{{ formatBytes(model.size_bytes) }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-key">RAM Budget:</span>
                <span class="meta-val">{{ model.recommended_ram }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-key">Hugging Face:</span>
                <span class="meta-val code-val" :title="model.filename">{{ model.filename }}</span>
              </div>
              <div class="meta-row" v-if="model.file_path">
                <span class="meta-key">File on Device:</span>
                <span class="meta-val code-val text-path" :title="model.file_path">{{ model.file_path }}</span>
              </div>
            </div>

            <!-- Active Download Progress Bar -->
            <div
              v-if="activeDownloads[model.id] || model.is_downloading"
              class="download-progress-container"
            >
              <div class="progress-meta-row">
                <span class="progress-status-label">
                  {{ activeDownloads[model.id]?.status === 'verifying' ? 'Verifying SHA-256 Checksum...' : 'Downloading weights from Hugging Face...' }}
                </span>
                <span class="progress-percent">
                  {{ Math.round(activeDownloads[model.id]?.progress || 0) }}%
                </span>
              </div>

              <div class="progress-track">
                <div
                  class="progress-bar-fill"
                  :style="{ width: `${Math.min(Math.max(activeDownloads[model.id]?.progress || 0, 0), 100)}%` }"
                ></div>
              </div>

              <div class="progress-metrics-row">
                <span>
                  {{ formatBytes(activeDownloads[model.id]?.received || model.downloaded_bytes) }} / {{ formatBytes(model.size_bytes) }}
                </span>
                <span v-if="activeDownloads[model.id]?.speed">
                  {{ formatSpeed(activeDownloads[model.id]!.speed) }}
                </span>
              </div>
            </div>

            <!-- Action Controls -->
            <div class="card-actions">
              <!-- Case 1: Downloading -> Cancel button -->
              <template v-if="model.is_downloading || activeDownloads[model.id]">
                <button
                  type="button"
                  class="btn-action btn-cancel"
                  @click="handleCancel(model.id)"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="action-icon">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                  Cancel Download
                </button>
              </template>

              <!-- Case 2: Not Downloaded -> Download button -->
              <template v-else-if="!model.is_downloaded">
                <button
                  type="button"
                  class="btn-action btn-download"
                  @click="handleDownload(model.id)"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="action-icon">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="7 10 12 15 17 10"></polyline>
                    <line x1="12" y1="15" x2="12" y2="3"></line>
                  </svg>
                  Download Model ({{ formatBytes(model.size_bytes) }})
                </button>
              </template>

              <!-- Case 3: Downloaded -> Verify and Delete buttons -->
              <template v-else>
                <div class="action-group-ready">
                  <button
                    type="button"
                    class="btn-action btn-secondary"
                    :disabled="verifyingModelId === model.id"
                    @click="handleVerify(model.id)"
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="action-icon">
                      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
                    </svg>
                    {{ verifyingModelId === model.id ? 'Verifying Hash...' : 'Verify Integrity' }}
                  </button>

                  <button
                    type="button"
                    class="btn-action btn-danger"
                    :disabled="deletingModelId === model.id"
                    @click="handleDelete(model.id)"
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="action-icon">
                      <polyline points="3 6 5 6 21 6"></polyline>
                      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                    {{ deletingModelId === model.id ? 'Deleting...' : 'Delete from Device' }}
                  </button>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="modal-footer">
        <p class="footer-note">
          Weights are downloaded once and cached permanently in the local app container. No API keys or cloud services required.
        </p>
        <button type="button" class="btn-done" @click="emit('close')">
          Done
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  animation: fadeIn 0.2s ease-out;
}

.modal-panel {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card, 20px);
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.3);
  width: 100%;
  max-width: 680px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: scaleUp 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

.modal-header {
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.06));
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.modal-title-group {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.modal-icon-bubble {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: rgba(0, 122, 255, 0.12);
  color: #007aff;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.modal-icon-bubble svg {
  width: 22px;
  height: 22px;
}

.modal-title {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.modal-subtitle {
  font-size: 0.8rem;
  color: var(--text-muted);
  margin: 0.15rem 0 0;
}

.btn-close {
  background: var(--bg-input);
  border: 1px solid var(--border-card-subtle);
  border-radius: 50%;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.9rem;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-close:hover {
  color: var(--text-primary);
  background: rgba(0, 0, 0, 0.08);
}

.banner {
  margin: 1rem 1.5rem 0;
  padding: 0.75rem 1rem;
  border-radius: 12px;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.85rem;
}

.banner-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
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
  margin-left: auto;
  background: none;
  border: none;
  color: currentColor;
  cursor: pointer;
  opacity: 0.7;
}

.banner-close:hover {
  opacity: 1;
}

.storage-overview-card {
  margin: 1rem 1.5rem 0;
  padding: 1rem 1.25rem;
  background: var(--bg-input);
  border: 1px solid var(--border-card);
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.storage-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.storage-label {
  font-size: 0.82rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}

.storage-badge {
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.6rem;
  border-radius: 999px;
}

.badge-active {
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.badge-empty {
  background: rgba(142, 142, 147, 0.15);
  color: #8e8e93;
}

.storage-stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.75rem;
}

.stat-item {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.stat-label {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.stat-val {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--text-primary);
}

.text-brand {
  color: #007aff;
  font-weight: 600;
}

.storage-path-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding-top: 0.6rem;
  border-top: 1px solid var(--border-card-subtle);
}

.path-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
  flex: 1;
}

.path-label {
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
}

.path-code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
  color: var(--text-primary);
  background: rgba(0, 0, 0, 0.05);
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-copy-path {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.3rem 0.6rem;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 8px;
  color: #007aff;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.btn-copy-path:hover {
  background: rgba(0, 122, 255, 0.08);
}

.models-section {
  padding: 1rem 1.5rem;
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.section-heading {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin: 0;
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 2.5rem;
  color: var(--text-muted);
  font-size: 0.85rem;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2.5px solid rgba(0, 122, 255, 0.2);
  border-top-color: #007aff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.models-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.model-card {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 16px;
  padding: 1.15rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  transition: all 0.2s ease;
}

.card-active {
  border-color: rgba(52, 199, 89, 0.4);
  box-shadow: 0 4px 16px rgba(52, 199, 89, 0.08);
}

.card-downloading {
  border-color: rgba(0, 122, 255, 0.4);
  box-shadow: 0 4px 16px rgba(0, 122, 255, 0.08);
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.model-name-group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.name-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.model-title {
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.quant-chip {
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 6px;
  color: var(--text-muted);
}

.badge-recommended {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  background: rgba(0, 122, 255, 0.12);
  color: #007aff;
  border-radius: 6px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.model-desc {
  font-size: 0.8rem;
  color: var(--text-muted);
  line-height: 1.4;
  margin: 0;
}

.status-indicator {
  flex-shrink: 0;
}

.status-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.3rem 0.65rem;
  border-radius: 999px;
}

.status-svg {
  width: 14px;
  height: 14px;
}

.chip-ready {
  background: rgba(52, 199, 89, 0.15);
  color: #34c759;
}

.chip-downloading {
  background: rgba(0, 122, 255, 0.15);
  color: #007aff;
}

.chip-missing {
  background: rgba(142, 142, 147, 0.12);
  color: var(--text-muted);
}

.pulse-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #007aff;
  animation: pulse 1.2s infinite;
}

.model-meta-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.5rem 1rem;
  padding: 0.75rem 0.9rem;
  background: var(--bg-input);
  border-radius: 10px;
}

.meta-row {
  display: flex;
  align-items: baseline;
  gap: 0.4rem;
  font-size: 0.75rem;
}

.meta-key {
  color: var(--text-muted);
  white-space: nowrap;
}

.meta-val {
  color: var(--text-primary);
  font-weight: 500;
}

.code-val {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, monospace;
  font-size: 0.72rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.text-path {
  color: #007aff;
}

.download-progress-container {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.6rem 0.85rem;
  background: rgba(0, 122, 255, 0.06);
  border: 1px solid rgba(0, 122, 255, 0.2);
  border-radius: 10px;
}

.progress-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.75rem;
}

.progress-status-label {
  color: #007aff;
  font-weight: 600;
}

.progress-percent {
  color: #007aff;
  font-weight: 700;
}

.progress-track {
  height: 8px;
  background: rgba(0, 122, 255, 0.15);
  border-radius: 999px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: #007aff;
  border-radius: 999px;
  transition: width 0.2s ease;
}

.progress-metrics-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.7rem;
  color: var(--text-muted);
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.action-group-ready {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
}

.btn-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  padding: 0.65rem 1.15rem;
  font-size: 0.85rem;
  font-weight: 600;
  border-radius: 12px;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}

.action-icon {
  width: 16px;
  height: 16px;
}

.btn-download {
  background: #007aff;
  color: #ffffff;
  width: 100%;
  box-shadow: 0 4px 12px rgba(0, 122, 255, 0.25);
}

.btn-download:hover {
  background: #0066d6;
}

.btn-cancel {
  background: rgba(255, 59, 48, 0.12);
  color: #ff3b30;
  border: 1px solid rgba(255, 59, 48, 0.3);
  width: 100%;
}

.btn-cancel:hover {
  background: rgba(255, 59, 48, 0.2);
}

.btn-secondary {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-card);
  color: var(--text-primary);
}

.btn-secondary:hover:not(:disabled) {
  background: rgba(0, 0, 0, 0.06);
}

.btn-danger {
  flex: 1;
  background: rgba(255, 59, 48, 0.1);
  border: 1px solid rgba(255, 59, 48, 0.25);
  color: #ff3b30;
}

.btn-danger:hover:not(:disabled) {
  background: rgba(255, 59, 48, 0.2);
}

.btn-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.modal-footer {
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--border-card-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  background: var(--bg-input);
}

.footer-note {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin: 0;
  max-width: 440px;
  line-height: 1.35;
}

.btn-done {
  padding: 0.6rem 1.25rem;
  background: #007aff;
  color: #ffffff;
  font-size: 0.88rem;
  font-weight: 600;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-done:hover {
  background: #0066d6;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes scaleUp {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

@media (max-width: 600px) {
  .storage-stats-grid {
    grid-template-columns: 1fr;
  }
  .model-meta-grid {
    grid-template-columns: 1fr;
  }
  .action-group-ready {
    flex-direction: column;
  }
  .modal-footer {
    flex-direction: column;
    align-items: stretch;
  }
  .btn-done {
    width: 100%;
  }
}
</style>
