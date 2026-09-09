<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Image as ImageIcon, Maximize2, ZoomIn, ZoomOut, RotateCcw, X, Eye } from "lucide-vue-next";

const props = defineProps<{
  file?: File | null;
  previewUrl: string | null;
}>();

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

function onPan(e: MouseEvent | TouchEvent) {
  if (!isDragging.value || zoomLevel.value <= 1) return;
  const clientX = "touches" in e ? e.touches[0].clientX : e.clientX;
  const clientY = "touches" in e ? e.touches[0].clientY : e.clientY;
  panOffset.value = {
    x: clientX - dragStart.value.x,
    y: clientY - dragStart.value.y,
  };
}

function endPan() {
  isDragging.value = false;
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
              @click="closeModal"
            >
              <X class="tool-icon" :stroke-width="2.2" />
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
            @touchstart.passive="startPan"
            @touchmove.passive="onPan"
            @touchend="endPan"
            @dblclick="toggleDoubleTapZoom"
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
            <span class="lightbox-instruction">
              <kbd class="key-cap">Double-click</kbd> to toggle zoom •
              <kbd class="key-cap">Drag</kbd> to pan • <kbd class="key-cap">Esc</kbd> to close
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
  padding: 0.85rem 1.25rem;
  background: rgba(22, 27, 38, 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  z-index: 10;
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
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.1);
  color: #f3f4f6;
  cursor: pointer;
  transition: all 0.15s ease;
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
  padding: 0.65rem 1rem;
  background: rgba(15, 20, 30, 0.7);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
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
@media (max-width: 600px) {
  .lightbox-toolbar {
    padding: 0.65rem 0.85rem;
    gap: 0.5rem;
  }

  .lightbox-filename {
    max-width: 120px;
  }

  .lightbox-sub {
    display: none;
  }

  .lightbox-instruction {
    font-size: 0.72rem;
  }
}
</style>
