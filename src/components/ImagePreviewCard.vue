<script setup lang="ts">
import { ref } from "vue";
import { Sparkles, LoaderCircle } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isWebLink } from "../services/ocr";
defineProps<{
  file: File;
  previewUrl: string | null;
  isProcessing?: boolean;
  isFromShareExtension?: boolean;
  hasEvent?: boolean;
  qrCodes?: string[];
}>();

async function handleOpenUrl(e: MouseEvent, url: string) {
  e.preventDefault();
  e.stopPropagation();
  try {
    await openUrl(url);
  } catch {
    if (typeof window !== "undefined") {
      window.open(url, "_blank");
    }
  }
}

const copiedQrIdx = ref<number | null>(null);
let copiedQrTimer: ReturnType<typeof setTimeout> | null = null;

async function handleCopyUrl(e: MouseEvent, url: string, index: number) {
  e.preventDefault();
  e.stopPropagation();
  try {
    if (navigator?.clipboard?.writeText) {
      await navigator.clipboard.writeText(url);
    }
    copiedQrIdx.value = index;
    if (copiedQrTimer) clearTimeout(copiedQrTimer);
    copiedQrTimer = setTimeout(() => {
      copiedQrIdx.value = null;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy QR code URL to clipboard:", err);
  }
}
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

    <!-- Detected QR Links -->
    <div v-if="qrCodes && qrCodes.length > 0" class="qr-preview-chips-row">
      <div class="qr-preview-label">
        <svg
          class="qr-preview-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect x="3" y="3" width="7" height="7"></rect>
          <rect x="14" y="3" width="7" height="7"></rect>
          <rect x="14" y="14" width="7" height="7"></rect>
          <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
        <span>Detected Link{{ qrCodes.length > 1 ? "s" : "" }}:</span>
      </div>
      <div class="qr-chips-wrap">
        <div v-for="(code, idx) in qrCodes" :key="idx" class="qr-link-pill">
          <a
            v-if="isWebLink(code)"
            :href="code"
            target="_blank"
            rel="noopener noreferrer"
            class="qr-link-pill-link"
            :title="`Open ${code}`"
            @click.stop="handleOpenUrl($event, code)"
          >
            <span class="qr-link-pill-text">{{ code }}</span>
            <svg
              class="qr-link-ext-icon"
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
          </a>
          <span v-else class="qr-link-pill-link is-text" :title="code">
            <span class="qr-link-pill-text">{{ code }}</span>
          </span>
          <button
            type="button"
            class="qr-link-copy-btn"
            :class="{ 'is-copied': copiedQrIdx === idx }"
            :title="copiedQrIdx === idx ? 'Copied!' : 'Copy link to clipboard'"
            @click.stop="handleCopyUrl($event, code, idx)"
          >
            <svg
              v-if="copiedQrIdx === idx"
              class="qr-copy-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
            <svg
              v-else
              class="qr-copy-icon"
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
            <span class="qr-copy-label">{{ copiedQrIdx === idx ? "Copied" : "Copy" }}</span>
          </button>
        </div>
      </div>
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
.qr-preview-chips-row {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.6rem 0.8rem;
  background: var(--bg-surface-elevated, rgba(59, 130, 246, 0.05));
  border: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.15));
  border-radius: var(--radius-sm, 8px);
  margin: 0.6rem 0;
}

.qr-preview-label {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--accent-primary, #3b82f6);
}

.qr-preview-icon {
  width: 13px;
  height: 13px;
}

.qr-chips-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.qr-link-pill {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  background: var(--bg-surface, #ffffff);
  border: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.25));
  border-radius: 9999px;
  font-size: 0.78rem;
  font-weight: 500;
  color: var(--accent-primary, #3b82f6);
  transition: all 0.15s ease;
  overflow: hidden;
}

.qr-link-pill-link {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.2rem 0.45rem 0.2rem 0.55rem;
  color: var(--accent-primary, #3b82f6);
  text-decoration: none;
  min-width: 0;
  transition: background 0.15s ease;
}

.qr-link-pill-link:hover {
  background: rgba(59, 130, 246, 0.08);
  text-decoration: underline;
}

.qr-link-pill-link.is-text {
  color: var(--text-primary, #111827);
  cursor: text;
}

.qr-link-pill-link.is-text:hover {
  background: transparent;
  text-decoration: none;
}

.qr-link-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.2rem 0.45rem;
  background: transparent;
  border: none;
  border-left: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.2));
  color: var(--text-secondary, #6b7280);
  font-size: 0.72rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.qr-link-copy-btn:hover {
  background: rgba(59, 130, 246, 0.1);
  color: var(--accent-primary, #3b82f6);
}

.qr-link-copy-btn.is-copied {
  color: #10b981;
  background: rgba(16, 185, 129, 0.12);
}

.qr-copy-icon {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
}

.qr-copy-label {
  line-height: 1;
}

.qr-link-pill-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 240px;
}

.qr-link-ext-icon {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
  opacity: 0.8;
}

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
