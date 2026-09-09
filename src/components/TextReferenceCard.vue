<script setup lang="ts">
import { ref, computed } from "vue";
import { FileText, Maximize2, Copy, Check, X, ExternalLink } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isWebLink } from "../services/ocr";

const props = defineProps<{
  text: string;
  qrCodes?: string[];
}>();

const emit = defineEmits<{
  (e: "edit"): void;
}>();

const isModalOpen = ref(false);
const copied = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;

const copiedQrIdx = ref<number | null>(null);
let copiedQrTimer: ReturnType<typeof setTimeout> | null = null;

const lineCount = computed(() => {
  if (!props.text) return 0;
  return props.text.split("\n").length;
});

const wordCount = computed(() => {
  if (!props.text) return 0;
  return props.text.trim().split(/\s+/).filter(Boolean).length;
});

const charCount = computed(() => props.text?.length || 0);

async function handleCopyText() {
  if (!props.text) return;
  try {
    if (navigator?.clipboard?.writeText) {
      await navigator.clipboard.writeText(props.text);
      copied.value = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copied.value = false;
      }, 2000);
    }
  } catch (err) {
    console.warn("Failed to copy text:", err);
  }
}

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
    console.error("Failed to copy link:", err);
  }
}

function openModal() {
  isModalOpen.value = true;
  document.body.style.overflow = "hidden";
}

function closeModal() {
  isModalOpen.value = false;
  document.body.style.overflow = "";
}

defineExpose({
  openModal,
  closeModal,
});
</script>

<template>
  <div class="text-reference-card-root">
    <!-- 1. COMPACT MOBILE STRIP (< 900px) -->
    <div
      class="compact-mobile-strip"
      role="button"
      tabindex="0"
      aria-label="View source text"
      @click="openModal"
      @keydown.enter="openModal"
      @keydown.space.prevent="openModal"
    >
      <div class="compact-thumb-wrap">
        <FileText class="compact-file-icon" :stroke-width="2.2" />
      </div>

      <div class="compact-info">
        <div class="compact-header-row">
          <span class="compact-title">Source Text</span>
          <span class="compact-badge">{{ wordCount }} words</span>
          <span v-if="qrCodes && qrCodes.length > 0" class="compact-qr-tag"
            >{{ qrCodes.length }} Link{{ qrCodes.length === 1 ? "" : "s" }}</span
          >
        </div>
        <span class="compact-snippet">{{ text.slice(0, 60).trim() }}...</span>
      </div>

      <button
        type="button"
        class="compact-expand-btn"
        aria-label="Expand text modal"
        @click.stop="openModal"
      >
        <Maximize2 class="expand-icon" :stroke-width="2.2" />
        <span class="expand-text">View</span>
      </button>
    </div>

    <!-- 2. DESKTOP STICKY SIDE PANEL (>= 900px) -->
    <div class="surface-card text-ref-desktop">
      <div class="desktop-header">
        <div class="desktop-header-title">
          <div class="header-icon-badge">
            <FileText class="header-icon" :stroke-width="2.2" />
          </div>
          <div class="desktop-title-text">
            <h3 class="desktop-heading">Source Text</h3>
            <span class="desktop-subheading">
              {{ wordCount }} words • {{ charCount }} chars • {{ lineCount }} lines
            </span>
          </div>
        </div>

        <div class="desktop-actions">
          <button
            type="button"
            class="desktop-action-btn"
            :title="copied ? 'Copied to clipboard' : 'Copy source text'"
            aria-label="Copy source text"
            @click="handleCopyText"
          >
            <Check v-if="copied" class="btn-icon-sm copy-success" :stroke-width="2.4" />
            <Copy v-else class="btn-icon-sm" :stroke-width="2.2" />
            <span>{{ copied ? "Copied" : "Copy" }}</span>
          </button>

          <button
            type="button"
            class="desktop-action-btn"
            title="Open full text view"
            aria-label="Open full text view"
            @click="openModal"
          >
            <Maximize2 class="btn-icon-sm" :stroke-width="2.2" />
            <span>Expand</span>
          </button>
        </div>
      </div>

      <!-- Text Container Area -->
      <div class="desktop-text-stage">
        <pre class="desktop-text-content">{{ text }}</pre>
      </div>

      <!-- Detected Links/URLs Section -->
      <div v-if="qrCodes && qrCodes.length > 0" class="desktop-qr-section">
        <div class="desktop-qr-header">
          <ExternalLink class="qr-icon-xs" :stroke-width="2.2" />
          <span class="qr-section-title">Detected Links ({{ qrCodes.length }})</span>
        </div>
        <div class="desktop-qr-list">
          <div
            v-for="(url, idx) in qrCodes"
            :key="idx"
            class="desktop-qr-chip"
            :class="{ 'is-web': isWebLink(url) }"
          >
            <a
              v-if="isWebLink(url)"
              :href="url"
              class="desktop-qr-link"
              target="_blank"
              rel="noopener noreferrer"
              @click="handleOpenUrl($event, url)"
            >
              <ExternalLink class="chip-action-icon" :stroke-width="2.2" />
              <span class="desktop-qr-text">{{ url }}</span>
            </a>
            <span v-else class="desktop-qr-text non-link">{{ url }}</span>

            <button
              type="button"
              class="desktop-qr-copy-btn"
              :class="{ 'is-copied': copiedQrIdx === idx }"
              title="Copy link to clipboard"
              aria-label="Copy link to clipboard"
              @click="handleCopyUrl($event, url, idx)"
            >
              <Check v-if="copiedQrIdx === idx" class="btn-icon-xs" :stroke-width="2.4" />
              <Copy v-else class="btn-icon-xs" :stroke-width="2.2" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. FULL-SCREEN LIGHTBOX MODAL -->
    <Teleport to="body">
      <div
        v-if="isModalOpen"
        class="lightbox-overlay"
        role="dialog"
        aria-modal="true"
        aria-label="Full Source Text"
        @keydown.esc="closeModal"
      >
        <div class="lightbox-backdrop" @click="closeModal"></div>
        <div class="lightbox-modal">
          <div class="lightbox-header">
            <div class="lightbox-header-title">
              <FileText class="lightbox-title-icon" :stroke-width="2.2" />
              <div>
                <h3 class="lightbox-title">Source Event Text</h3>
                <span class="lightbox-subtitle">
                  {{ wordCount }} words • {{ charCount }} characters • {{ lineCount }} lines
                </span>
              </div>
            </div>

            <div class="lightbox-header-actions">
              <button
                type="button"
                class="lightbox-btn"
                :title="copied ? 'Copied' : 'Copy all text'"
                @click="handleCopyText"
              >
                <Check v-if="copied" class="btn-icon-sm copy-success" :stroke-width="2.4" />
                <Copy v-else class="btn-icon-sm" :stroke-width="2.2" />
                <span>{{ copied ? "Copied" : "Copy text" }}</span>
              </button>

              <button
                type="button"
                class="lightbox-close-btn"
                aria-label="Close modal"
                @click="closeModal"
              >
                <X class="btn-icon-sm" :stroke-width="2.2" />
              </button>
            </div>
          </div>

          <div class="lightbox-body">
            <pre class="lightbox-text-content">{{ text }}</pre>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.text-reference-card-root {
  width: 100%;
}

/* COMPACT MOBILE VIEW (< 900px) */
.compact-mobile-strip {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card);
  padding: 0.75rem 1rem;
  cursor: pointer;
  box-shadow: var(--shadow-card);
  transition: all 0.2s ease;
}

.compact-mobile-strip:hover {
  border-color: var(--accent-primary);
}

.compact-thumb-wrap {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.compact-file-icon {
  width: 22px;
  height: 22px;
}

.compact-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  flex: 1;
  min-width: 0;
}

.compact-header-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.compact-title {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--text-primary);
}

.compact-badge {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
}

.compact-qr-tag {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--accent-primary);
  background: rgba(0, 122, 255, 0.1);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
}

.compact-snippet {
  font-size: 0.78rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.compact-expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 8px;
  padding: 0.4rem 0.65rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
  flex-shrink: 0;
}

/* DESKTOP PANEL (>= 900px) */
.text-ref-desktop {
  display: none;
}

@media (min-width: 900px) {
  .compact-mobile-strip {
    display: none;
  }

  .text-ref-desktop {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    position: sticky;
    top: 1.25rem;
  }
}

.desktop-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.desktop-header-title {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}

.header-icon-badge {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.header-icon {
  width: 20px;
  height: 20px;
}

.desktop-title-text {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}

.desktop-heading {
  font-size: 0.98rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.desktop-subheading {
  font-size: 0.78rem;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.desktop-actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-shrink: 0;
}

.desktop-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 8px;
  padding: 0.45rem 0.65rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.desktop-action-btn:hover {
  background: var(--border-input);
  border-color: var(--accent-primary);
}

.btn-icon-sm {
  width: 15px;
  height: 15px;
}

.copy-success {
  color: #34c759;
}

/* Stage */
.desktop-text-stage {
  background: var(--bg-main, #0f141c);
  border: 1px solid var(--border-input);
  border-radius: 10px;
  padding: 1rem;
  max-height: 420px;
  overflow-y: auto;
  overflow-x: hidden;
}

.desktop-text-content {
  margin: 0;
  font-family: inherit;
  font-size: 0.88rem;
  line-height: 1.55;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

/* Links / QR Section */
.desktop-qr-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-card);
}

.desktop-qr-header {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--text-secondary);
}

.qr-icon-xs {
  width: 14px;
  height: 14px;
  color: var(--accent-primary);
}

.desktop-qr-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.desktop-qr-chip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 8px;
  padding: 0.35rem 0.6rem;
  font-size: 0.78rem;
}

.desktop-qr-link {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--accent-primary);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.desktop-qr-link:hover {
  text-decoration: underline;
}

.chip-action-icon {
  width: 13px;
  height: 13px;
  flex-shrink: 0;
}

.desktop-qr-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.desktop-qr-text.non-link {
  color: var(--text-secondary);
}

.desktop-qr-copy-btn {
  background: none;
  border: none;
  color: var(--text-tertiary);
  cursor: pointer;
  padding: 0.2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: color 0.15s ease;
}

.desktop-qr-copy-btn:hover {
  color: var(--text-primary);
}

.desktop-qr-copy-btn.is-copied {
  color: #34c759;
}

.btn-icon-xs {
  width: 13px;
  height: 13px;
}

/* LIGHTBOX MODAL */
.lightbox-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.lightbox-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
}

.lightbox-modal {
  position: relative;
  z-index: 1;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card);
  width: 100%;
  max-width: 720px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-card);
}

.lightbox-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--border-card);
}

.lightbox-header-title {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.lightbox-title-icon {
  width: 24px;
  height: 24px;
  color: var(--accent-primary);
}

.lightbox-title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
}

.lightbox-subtitle {
  font-size: 0.8rem;
  color: var(--text-tertiary);
}

.lightbox-header-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.lightbox-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 8px;
  padding: 0.45rem 0.8rem;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
}

.lightbox-close-btn {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.lightbox-close-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent-primary);
}

.lightbox-body {
  padding: 1.5rem;
  overflow-y: auto;
  flex: 1;
}

.lightbox-text-content {
  margin: 0;
  font-family: inherit;
  font-size: 0.94rem;
  line-height: 1.6;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
