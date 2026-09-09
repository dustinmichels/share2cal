<script setup lang="ts">
import { Sparkles, LoaderCircle } from "lucide-vue-next";

defineProps<{
  file: File;
  previewUrl: string | null;
  isProcessing?: boolean;
  isFromShareExtension?: boolean;
  hasEvent?: boolean;
}>();

const emit = defineEmits<{
  (e: "scan"): void;
  (e: "remove"): void;
  (e: "chooseAnother"): void;
  (e: "takePhoto"): void;
}>();

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div class="surface-card preview-card">
    <div class="preview-top-bar">
      <div class="preview-meta">
        <span class="preview-file-name" :title="file.name">{{ file.name }}</span>
        <div class="preview-chips">
          <span class="chip-size">{{ formatFileSize(file.size) }}</span>
          <span v-if="isFromShareExtension" class="chip-share">
            <svg
              class="chip-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
            >
              <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path>
              <polyline points="16 6 12 2 8 6"></polyline>
              <line x1="12" y1="2" x2="12" y2="15"></line>
            </svg>
            iOS Share
          </span>
        </div>
      </div>

      <button
        type="button"
        class="btn-pill-danger"
        :disabled="isProcessing"
        aria-label="Remove photo"
        @click="emit('remove')"
      >
        <svg
          class="btn-icon-xs"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="3 6 5 6 21 6"></polyline>
          <path
            d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
          ></path>
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
        class="btn-touch btn-touch-scan btn-touch-parse"
        :class="{ 'is-loading': isProcessing }"
        :disabled="isProcessing"
        @click="emit('scan')"
      >
        <template v-if="!isProcessing">
          <Sparkles class="btn-icon" :stroke-width="2.2" />
          <span>Parse</span>
        </template>
        <template v-else>
          <LoaderCircle class="btn-icon spinner-icon animate-spin" :stroke-width="2.2" />
          <span>Parsing...</span>
        </template>
      </button>
      <div class="quick-switch-bar">
        <button
          type="button"
          class="btn-subtle-link"
          :disabled="isProcessing"
          @click="emit('chooseAnother')"
        >
          Choose another photo
        </button>
        <span class="quick-dot">•</span>
        <button
          type="button"
          class="btn-subtle-link"
          :disabled="isProcessing"
          @click="emit('takePhoto')"
        >
          Take new photo
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
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

.animate-spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.btn-touch-parse {
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: -0.01em;
}
</style>
