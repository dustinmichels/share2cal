<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Image as ImageIcon, Maximize2, ZoomIn, ZoomOut, RotateCcw, X, Eye } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isWebLink } from "../services/ocr";

const props = defineProps<{
  file?: File | null;
  previewUrl: string | null;
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
const isModalOpen = ref(false);
const zoomLevel = ref(1);
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const panOffset = ref({ x: 0, y: 0 });

const fileName = computed(() => props.file?.name || "Original Flyer");
const fileSizeFormatted = computed(() => {
  if (!props.file?.size) return "";
  const bytes = props.file.size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
});

const zoomPercent = computed(() => `${Math.round(zoomLevel.value * 100)}%`);

function openModal() {
  if (!props.previewUrl) return;
  isModalOpen.value = true;
  zoomLevel.value = 1;
  panOffset.value = { x: 0, y: 0 };
  document.body.style.overflow = "hidden";
}

function closeModal() {
  isModalOpen.value = false;
  zoomLevel.value = 1;
  panOffset.value = { x: 0, y: 0 };
  document.body.style.overflow = "";
}

function handleZoomIn() {
  if (zoomLevel.value < 3) {
    zoomLevel.value = Math.min(3, +(zoomLevel.value + 0.5).toFixed(1));
  }
}

function handleZoomOut() {
  if (zoomLevel.value > 0.5) {
    zoomLevel.value = Math.max(0.5, +(zoomLevel.value - 0.5).toFixed(1));
    if (zoomLevel.value <= 1) {
      panOffset.value = { x: 0, y: 0 };
    }
  }
}

function handleResetZoom() {
  zoomLevel.value = 1;
  panOffset.value = { x: 0, y: 0 };
}

function toggleDoubleTapZoom() {
  if (zoomLevel.value === 1) {
    zoomLevel.value = 2;
  } else {
    zoomLevel.value = 1;
    panOffset.value = { x: 0, y: 0 };
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!isModalOpen.value) return;
  if (e.key === "Escape") {
    closeModal();
  } else if (e.key === "+" || e.key === "=") {
    handleZoomIn();
  } else if (e.key === "-" || e.key === "_") {
    handleZoomOut();
  } else if (e.key === "0") {
    handleResetZoom();
  }
}

const touchStartY = ref(0);
const touchStartX = ref(0);
let lastTapTime = 0;

function handleStageClick(e: MouseEvent) {
  const target = e.target as HTMLElement | null;
  if (!target) return;
  // If clicking directly on the image, don't close
  if (target.tagName === "IMG") {
    return;
  }
  closeModal();
}

function startPan(e: MouseEvent | TouchEvent) {
  if (zoomLevel.value <= 1) return;
  isDragging.value = true;
  const clientX = "touches" in e ? e.touches[0].clientX : e.clientX;
  const clientY = "touches" in e ? e.touches[0].clientY : e.clientY;
  dragStart.value = {
    x: clientX - panOffset.value.x,
    y: clientY - panOffset.value.y,
  };
}

function handleTouchStart(e: TouchEvent) {
  if (e.touches.length === 1) {
    touchStartX.value = e.touches[0].clientX;
    touchStartY.value = e.touches[0].clientY;
  }
  startPan(e);
}

function onPan(e: MouseEvent | TouchEvent) {
  if (!isDragging.value || zoomLevel.value <= 1) return;
  const clientX = "touches" in e ? e.touches[0].clientX : e.clientX;
  const clientY = "touches" in e ? e.touches[0].clientY : e.clientY;
  panOffset.value = {
    x: clientX - dragStart.value.x,
    y: clientY - dragStart.value.y,
  };
}

function handleTouchMove(e: TouchEvent) {
  if (e.touches.length === 1 && zoomLevel.value <= 1) {
    const deltaY = e.touches[0].clientY - touchStartY.value;
    const deltaX = Math.abs(e.touches[0].clientX - touchStartX.value);
    // Swiping down on mobile to dismiss modal
    if (deltaY > 75 && deltaY > deltaX * 1.5) {
      closeModal();
      return;
    }
  }
  onPan(e);
}

function endPan() {
  isDragging.value = false;
}

function handleTouchEnd() {
  const now = Date.now();
  if (now - lastTapTime < 300) {
    toggleDoubleTapZoom();
    lastTapTime = 0;
  } else {
    lastTapTime = now;
  }
  endPan();
}

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
  document.body.style.overflow = "";
});

defineExpose({
  openModal,
  closeModal,
});
</script>

<template>
  <div v-if="previewUrl" class="image-reference-wrapper">
    <!-- 1. MOBILE COMPACT REFERENCE CARD (< 900px) -->
    <div
      class="surface-card image-ref-compact"
      role="button"
      tabindex="0"
      aria-label="View original image full screen"
      @click="openModal"
      @keydown.enter="openModal"
      @keydown.space.prevent="openModal"
    >
      <div class="compact-thumb-wrap">
        <img :src="previewUrl" :alt="fileName" class="compact-thumb-img" />
        <div class="compact-thumb-overlay" aria-hidden="true">
          <Eye class="compact-overlay-icon" :stroke-width="2.5" />
        </div>
      </div>

      <div class="compact-info">
        <div class="compact-header-row">
          <span class="compact-title">Reference Image</span>
          <span v-if="fileSizeFormatted" class="compact-badge">{{ fileSizeFormatted }}</span>
          <span v-if="qrCodes && qrCodes.length > 0" class="compact-qr-tag"
            >{{ qrCodes.length }} QR</span
          >
        </div>
        <span class="compact-filename" :title="fileName">{{ fileName }}</span>
      </div>

      <button
        type="button"
        class="compact-expand-btn"
        aria-label="Expand image modal"
        @click.stop="openModal"
      >
        <Maximize2 class="expand-icon" :stroke-width="2.2" />
        <span class="expand-text">View</span>
      </button>
    </div>

    <!-- 2. DESKTOP STICKY SIDE PANEL (>= 900px) -->
    <div class="surface-card image-ref-desktop">
      <div class="desktop-header">
        <div class="desktop-header-title">
          <div class="header-icon-badge">
            <ImageIcon class="header-icon" :stroke-width="2.2" />
          </div>
          <div class="desktop-title-text">
            <h3 class="desktop-heading">Reference Image</h3>
            <span class="desktop-subheading" :title="fileName">
              {{ fileName }}
              <template v-if="fileSizeFormatted"> • {{ fileSizeFormatted }}</template>
            </span>
          </div>
        </div>

        <button
          type="button"
          class="desktop-expand-btn"
          title="Open in full-screen lightbox"
          aria-label="Open in full-screen lightbox"
          @click="openModal"
        >
          <Maximize2 class="btn-icon-sm" :stroke-width="2.2" />
          <span>Expand</span>
        </button>
      </div>

      <div
        class="desktop-stage"
        role="button"
        tabindex="0"
        aria-label="Click to enlarge flyer image"
        @click="openModal"
        @keydown.enter="openModal"
        @keydown.space.prevent="openModal"
      >
        <img :src="previewUrl" :alt="fileName" class="desktop-stage-img" />
        <div class="desktop-stage-hover-hint">
          <div class="hover-hint-chip">
            <ZoomIn class="hint-icon" :stroke-width="2.2" />
            <span>Click to inspect & zoom</span>
          </div>
        </div>
      </div>

      <div v-if="qrCodes && qrCodes.length > 0" class="desktop-qr-section">
        <div class="desktop-qr-header">
          <svg
            class="qr-icon-xs"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <rect x="3" y="3" width="7" height="7"></rect>
            <rect x="14" y="3" width="7" height="7"></rect>
            <rect x="14" y="14" width="7" height="7"></rect>
            <rect x="3" y="14" width="7" height="7"></rect>
          </svg>
          <span>Detected QR Links ({{ qrCodes.length }})</span>
        </div>
        <div class="desktop-qr-list">
          <div v-for="(code, idx) in qrCodes" :key="idx" class="desktop-qr-pill">
            <a
              v-if="isWebLink(code)"
              :href="code"
              target="_blank"
              rel="noopener noreferrer"
              class="desktop-qr-link"
              :title="`Open ${code}`"
              @click.stop="handleOpenUrl($event, code)"
            >
              <span class="desktop-qr-text">{{ code }}</span>
              <svg
                class="qr-ext-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                <polyline points="15 3 21 3 21 9"></polyline>
                <line x1="10" y1="14" x2="21" y2="3"></line>
              </svg>
            </a>
            <span v-else class="desktop-qr-link is-text" :title="code">
              <span class="desktop-qr-text">{{ code }}</span>
            </span>
            <button
              type="button"
              class="desktop-qr-copy-btn"
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
      <div class="desktop-footer">
        <span class="desktop-hint-text">
          Compare extracted dates, times, and venue details against the original flyer.
        </span>
      </div>
    </div>

    <!-- 3. FULL LIGHTBOX MODAL (Teleported to Body) -->
    <Teleport to="body">
      <Transition name="fade-lightbox">
        <div
          v-if="isModalOpen"
          class="lightbox-backdrop"
          role="dialog"
          aria-modal="true"
          :aria-label="`Inspection view for ${fileName}`"
          @click.self="closeModal"
        >
          <!-- Top Controls Bar -->
          <div class="lightbox-toolbar" @click.stop>
            <div class="lightbox-meta">
              <div class="lightbox-meta-badge">
                <ImageIcon class="meta-icon" :stroke-width="2.2" />
              </div>
              <div class="lightbox-meta-text">
                <span class="lightbox-filename" :title="fileName">{{ fileName }}</span>
                <span class="lightbox-sub">Original Reference Image</span>
              </div>
            </div>

            <!-- Center Zoom Controls -->
            <div class="lightbox-zoom-controls">
              <button
                type="button"
                class="lightbox-tool-btn"
                title="Zoom Out (-)"
                aria-label="Zoom Out"
                :disabled="zoomLevel <= 0.5"
                @click="handleZoomOut"
              >
                <ZoomOut class="tool-icon" :stroke-width="2.2" />
              </button>

              <button
                type="button"
                class="lightbox-zoom-badge"
                title="Toggle fit / 200% zoom"
                aria-label="Toggle zoom level"
                @click="toggleDoubleTapZoom"
              >
                {{ zoomPercent }}
              </button>

              <button
                type="button"
                class="lightbox-tool-btn"
                title="Zoom In (+)"
                aria-label="Zoom In"
                :disabled="zoomLevel >= 3"
                @click="handleZoomIn"
              >
                <ZoomIn class="tool-icon" :stroke-width="2.2" />
              </button>

              <button
                type="button"
                class="lightbox-tool-btn"
                title="Reset Zoom (0)"
                aria-label="Reset Zoom"
                @click="handleResetZoom"
              >
                <RotateCcw class="tool-icon" :stroke-width="2.2" />
              </button>
            </div>

            <!-- Close Button -->
            <button
              type="button"
              class="lightbox-close-btn"
              title="Close image view (Esc)"
              aria-label="Close image view"
              @click.stop="closeModal"
            >
              <X class="tool-icon" :stroke-width="2.4" />
            </button>
          </div>

          <!-- Canvas / Image Stage -->
          <div
            class="lightbox-stage"
            :class="{ 'is-pannable': zoomLevel > 1, 'is-dragging': isDragging }"
            @mousedown="startPan"
            @mousemove="onPan"
            @mouseup="endPan"
            @mouseleave="endPan"
            @touchstart.passive="handleTouchStart"
            @touchmove.passive="handleTouchMove"
            @touchend="handleTouchEnd"
            @dblclick="toggleDoubleTapZoom"
            @click="handleStageClick"
          >
            <div
              class="lightbox-img-transform-wrap"
              :style="{
                transform: `translate(${panOffset.x}px, ${panOffset.y}px) scale(${zoomLevel})`,
              }"
            >
              <img :src="previewUrl" :alt="fileName" class="lightbox-full-img" draggable="false" />
            </div>
          </div>

          <!-- Bottom Floating Instructions Bar -->
          <div class="lightbox-bottom-bar" @click.stop>
            <span class="lightbox-instruction lightbox-instruction-desktop">
              <kbd class="key-cap">Double-click</kbd> to toggle zoom •
              <kbd class="key-cap">Drag</kbd> to pan • <kbd class="key-cap">Esc</kbd> to close
            </span>
            <span class="lightbox-instruction lightbox-instruction-mobile">
              Tap background or swipe down to close
            </span>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.image-reference-wrapper {
  width: 100%;
}

/* ==========================================================
   1. MOBILE COMPACT CARD (< 900px)
   ========================================================== */
.image-ref-compact {
  display: flex !important;
  flex-direction: row !important;
  align-items: center;
  justify-content: space-between;
  gap: 0.85rem;
  padding: 0.65rem 0.85rem;
  cursor: pointer;
  user-select: none;
  border-radius: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  box-shadow: var(--shadow-subtle);
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  -webkit-tap-highlight-color: transparent;
}

.image-ref-compact:hover {
  border-color: rgba(0, 122, 255, 0.3);
  background: var(--bg-card-elevated, var(--bg-card));
  transform: translateY(-1px);
  box-shadow: var(--shadow-card);
}

.image-ref-compact:active {
  transform: scale(0.99);
}

.compact-thumb-wrap {
  position: relative;
  width: 50px;
  height: 50px;
  border-radius: 10px;
  overflow: hidden;
  background: #000;
  border: 1px solid var(--border-card);
  flex-shrink: 0;
}

.compact-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.compact-thumb-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.image-ref-compact:hover .compact-thumb-overlay {
  opacity: 1;
}

.compact-overlay-icon {
  width: 18px;
  height: 18px;
  color: #ffffff;
}

.compact-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  text-align: left;
  gap: 0.15rem;
  flex: 1;
  min-width: 0;
}
.compact-header-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.compact-title {
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.compact-badge {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-tertiary);
}

.compact-filename {
  font-size: 0.82rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.compact-expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.4rem 0.75rem;
  border-radius: 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--accent-primary);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.compact-expand-btn:hover {
  background: rgba(0, 122, 255, 0.1);
  border-color: rgba(0, 122, 255, 0.3);
}

.expand-icon {
  width: 14px;
  height: 14px;
}

/* ==========================================================
   2. DESKTOP STICKY SIDE PANEL (>= 900px)
   ========================================================== */
.image-ref-desktop {
  display: none;
  flex-direction: column;
  gap: 1rem;
  padding: 1.25rem;
  border-radius: var(--radius-card);
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  box-shadow: var(--shadow-card);
  position: sticky;
  top: 1.25rem;
  max-height: calc(100vh - 2.5rem);
  overflow: hidden;
}

.desktop-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-shrink: 0;
}

.desktop-header-title {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.header-icon-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  flex-shrink: 0;
}

.header-icon {
  width: 18px;
  height: 18px;
}

.desktop-title-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.desktop-heading {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  line-height: 1.2;
}

.desktop-subheading {
  font-size: 0.78rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 0.1rem;
}

.desktop-expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.8rem;
  border-radius: 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--accent-primary);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.desktop-expand-btn:hover {
  background: rgba(0, 122, 255, 0.12);
  border-color: rgba(0, 122, 255, 0.35);
  transform: translateY(-1px);
}

.desktop-stage {
  position: relative;
  flex: 1;
  min-height: 340px;
  max-height: calc(100vh - 14rem);
  background: #0d1117;
  border: 1px solid var(--border-card);
  border-radius: 14px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  user-select: none;
}

.desktop-stage-img {
  width: 100%;
  height: 100%;
  max-height: calc(100vh - 14rem);
  object-fit: contain;
  display: block;
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.desktop-stage:hover .desktop-stage-img {
  transform: scale(1.02);
}

.desktop-stage-hover-hint {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s ease;
  backdrop-filter: blur(2px);
}

.desktop-stage:hover .desktop-stage-hover-hint {
  opacity: 1;
}

.hover-hint-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1rem;
  background: rgba(255, 255, 255, 0.92);
  color: #111827;
  border-radius: 30px;
  font-size: 0.84rem;
  font-weight: 700;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  transform: translateY(4px);
  transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.desktop-stage:hover .hover-hint-chip {
  transform: translateY(0);
}

.hint-icon {
  width: 16px;
  height: 16px;
  color: #007aff;
}

.desktop-footer {
  flex-shrink: 0;
  padding-top: 0.2rem;
}

.desktop-hint-text {
  font-size: 0.78rem;
  color: var(--text-tertiary);
  line-height: 1.35;
  display: block;
  text-align: center;
}

.compact-qr-tag {
  font-size: 0.68rem;
  font-weight: 700;
  color: var(--accent-primary, #3b82f6);
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.25);
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
}

.desktop-qr-section {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.5rem 0.65rem;
  background: var(--bg-surface-elevated, rgba(59, 130, 246, 0.05));
  border: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.15));
  border-radius: var(--radius-sm, 8px);
}

.desktop-qr-header {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--accent-primary, #3b82f6);
}

.qr-icon-xs {
  width: 12px;
  height: 12px;
}

.desktop-qr-list {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.desktop-qr-pill {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-surface, #ffffff);
  border: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.2));
  border-radius: 6px;
  font-size: 0.76rem;
  font-weight: 500;
  transition: all 0.15s ease;
  overflow: hidden;
}

.desktop-qr-link {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.25rem 0.45rem;
  color: var(--accent-primary, #3b82f6);
  text-decoration: none;
  min-width: 0;
  flex: 1;
}

.desktop-qr-link:hover {
  background: rgba(59, 130, 246, 0.08);
  text-decoration: underline;
}

.desktop-qr-link.is-text {
  color: var(--text-primary, #111827);
  cursor: text;
}

.desktop-qr-link.is-text:hover {
  background: transparent;
  text-decoration: none;
}

.desktop-qr-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0.25rem 0.45rem;
  background: transparent;
  border: none;
  border-left: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.18));
  color: var(--text-secondary, #6b7280);
  font-size: 0.72rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.desktop-qr-copy-btn:hover {
  background: rgba(59, 130, 246, 0.1);
  color: var(--accent-primary, #3b82f6);
}

.desktop-qr-copy-btn.is-copied {
  color: #10b981;
  background: rgba(16, 185, 129, 0.12);
}

.desktop-qr-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}

.qr-copy-icon {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
}

.qr-copy-label {
  line-height: 1;
}

.qr-ext-icon {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
  opacity: 0.8;
}

/* Responsive breakpoint for Mobile vs Desktop */
@media (min-width: 900px) {
  .image-ref-compact {
    display: none;
  }
  .image-ref-desktop {
    display: flex;
  }
}

/* ==========================================================
   3. FULL LIGHTBOX MODAL
   ========================================================== */
.lightbox-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(10, 14, 22, 0.88);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  display: flex;
  flex-direction: column;
  user-select: none;
  animation: fadeInModal 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.lightbox-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: max(0.85rem, env(safe-area-inset-top)) max(1.25rem, env(safe-area-inset-right)) 0.85rem
    max(1.25rem, env(safe-area-inset-left));
  background: rgba(22, 27, 38, 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  z-index: 20;
  flex-shrink: 0;
}

.lightbox-meta {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.lightbox-meta-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(0, 122, 255, 0.2);
  color: #388bfd;
  flex-shrink: 0;
}

.meta-icon {
  width: 18px;
  height: 18px;
}

.lightbox-meta-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.lightbox-filename {
  font-size: 0.88rem;
  font-weight: 700;
  color: #f3f4f6;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 240px;
}

.lightbox-sub {
  font-size: 0.72rem;
  color: #9ca3af;
}

.lightbox-zoom-controls {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  background: rgba(255, 255, 255, 0.08);
  padding: 0.25rem 0.4rem;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.lightbox-tool-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: #f3f4f6;
  cursor: pointer;
  transition: all 0.15s ease;
}

.lightbox-tool-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.15);
  color: #ffffff;
}

.lightbox-tool-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.tool-icon {
  width: 18px;
  height: 18px;
}

.lightbox-zoom-badge {
  background: transparent;
  border: none;
  color: #388bfd;
  font-weight: 700;
  font-size: 0.82rem;
  padding: 0.25rem 0.5rem;
  cursor: pointer;
  min-width: 48px;
  text-align: center;
  border-radius: 6px;
  transition: background 0.15s ease;
}

.lightbox-zoom-badge:hover {
  background: rgba(56, 139, 253, 0.15);
}

.lightbox-close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  min-width: 38px;
  min-height: 38px;
  flex-shrink: 0;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.12);
  color: #f3f4f6;
  cursor: pointer;
  transition: all 0.15s ease;
  z-index: 30;
}
.lightbox-close-btn:hover {
  background: rgba(255, 59, 48, 0.25);
  border-color: rgba(255, 59, 48, 0.4);
  color: #ff453a;
  transform: scale(1.05);
}

.lightbox-stage {
  flex: 1;
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  touch-action: none;
  cursor: default;
}

.lightbox-stage.is-pannable {
  cursor: grab;
}

.lightbox-stage.is-dragging {
  cursor: grabbing;
}

.lightbox-img-transform-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  max-width: 90vw;
  max-height: 80vh;
  transform-origin: center center;
  transition: transform 0.1s ease-out;
  will-change: transform;
}

.lightbox-full-img {
  max-width: 90vw;
  max-height: 78vh;
  object-fit: contain;
  border-radius: 8px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  user-select: none;
  -webkit-user-drag: none;
}

.lightbox-bottom-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.65rem 1rem max(0.65rem, env(safe-area-inset-bottom)) 1rem;
  background: rgba(15, 20, 30, 0.7);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
}

.lightbox-instruction-mobile {
  display: none;
}

.lightbox-instruction-desktop {
  display: inline;
}

.lightbox-instruction {
  font-size: 0.78rem;
  color: #9ca3af;
  text-align: center;
}
.key-cap {
  display: inline-block;
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.12);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  color: #f3f4f6;
  font-family: inherit;
  font-size: 0.72rem;
  font-weight: 600;
}

/* Animations */
@keyframes fadeInModal {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.fade-lightbox-enter-active,
.fade-lightbox-leave-active {
  transition: opacity 0.2s ease;
}

.fade-lightbox-enter-from,
.fade-lightbox-leave-to {
  opacity: 0;
}

/* Mobile toolbar adjustments */
@media (max-width: 640px) {
  .lightbox-toolbar {
    padding: max(0.65rem, env(safe-area-inset-top)) max(0.85rem, env(safe-area-inset-right)) 0.65rem
      max(0.85rem, env(safe-area-inset-left));
    gap: 0.5rem;
  }

  .lightbox-meta-badge {
    display: none;
  }

  .lightbox-filename {
    max-width: 110px;
    font-size: 0.82rem;
  }

  .lightbox-sub {
    display: none;
  }

  .lightbox-zoom-controls {
    gap: 0.2rem;
    padding: 0.2rem 0.35rem;
  }

  .lightbox-tool-btn {
    width: 30px;
    height: 30px;
  }

  .lightbox-tool-btn:last-child {
    display: none;
  }

  .lightbox-close-btn {
    width: 40px;
    height: 40px;
    min-width: 40px;
    min-height: 40px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.18);
    border: 1px solid rgba(255, 255, 255, 0.3);
  }

  /* Mobile image sizing & containment */
  .lightbox-stage {
    padding: 1rem 1.25rem;
  }

  .lightbox-img-transform-wrap {
    max-width: min(88vw, 380px);
    max-height: calc(72dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
  }

  .lightbox-full-img {
    max-width: min(88vw, 380px);
    max-height: calc(72dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.65);
  }

  .lightbox-instruction-desktop {
    display: none;
  }

  .lightbox-instruction-mobile {
    display: inline;
    font-size: 0.76rem;
    color: #9ca3af;
  }
}
</style>
