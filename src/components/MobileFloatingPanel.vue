<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";
import { ChevronUp, ChevronDown, List, Calendar as CalendarIcon, ArrowLeft, Maximize2 } from "lucide-vue-next";

const props = withDefaults(
  defineProps<{
    previewUrl: string | null;
    fileName?: string;
    totalEvents: number;
    overallConfidence?: number;
    isEditing?: boolean;
  }>(),
  {
    isEditing: false,
  },
);

watch(
  () => props.isEditing,
  (editing) => {
    if (editing) {
      sheetState.value = "expanded";
      currentDragHeight.value = null;
    }
  },
);

const emit = defineEmits<{
  (e: "back"): void;
  (e: "openImageModal"): void;
}>();

const viewMode = defineModel<"details" | "week">("viewMode", { default: "details" });

// Snap states: "half" (view both at once) or "expanded" (cover image)
const sheetState = ref<"half" | "expanded">("half");
const isDragging = ref(false);
const hasMoved = ref(false);
const startY = ref(0);
const startHeight = ref(0);
const currentDragHeight = ref<number | null>(null);

const sheetRef = ref<HTMLElement | null>(null);

function getHalfHeightPx(): number {
  return Math.round(window.innerHeight * 0.5);
}

function getExpandedHeightPx(): number {
  return Math.round(window.innerHeight * 0.92);
}

function toggleSheet() {
  isDragging.value = false;
  currentDragHeight.value = null;
  sheetState.value = sheetState.value === "half" ? "expanded" : "half";
}

function handleZoneClick() {
  if (hasMoved.value) {
    hasMoved.value = false;
    return;
  }
  toggleSheet();
}
function startDrag(clientY: number) {
  isDragging.value = true;
  hasMoved.value = false;
  startY.value = clientY;
  startHeight.value = sheetRef.value ? sheetRef.value.offsetHeight : getHalfHeightPx();
  currentDragHeight.value = null;

  window.addEventListener("touchmove", onTouchMove, { passive: false });
  window.addEventListener("touchend", onTouchEnd);
  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
}

function handleTouchStart(e: TouchEvent) {
  if (e.touches.length === 1) {
    startDrag(e.touches[0].clientY);
  }
}

function handleMouseDown(e: MouseEvent) {
  if (e.button === 0) {
    startDrag(e.clientY);
  }
}

function updateDrag(clientY: number) {
  if (!isDragging.value) return;
  const deltaY = startY.value - clientY; // dragging UP increases height
  if (Math.abs(deltaY) >= 6) {
    hasMoved.value = true;
  }
  if (!hasMoved.value) return;
  const minH = window.innerHeight * 0.35;
  const maxH = window.innerHeight * 0.95;
  const targetH = Math.min(maxH, Math.max(minH, startHeight.value + deltaY));
  currentDragHeight.value = targetH;
}
function onTouchMove(e: TouchEvent) {
  if (!isDragging.value) return;
  e.preventDefault();
  updateDrag(e.touches[0].clientY);
}

function onMouseMove(e: MouseEvent) {
  if (!isDragging.value) return;
  updateDrag(e.clientY);
}

function endDrag() {
  if (!isDragging.value) return;
  isDragging.value = false;

  window.removeEventListener("touchmove", onTouchMove);
  window.removeEventListener("touchend", onTouchEnd);
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", onMouseUp);

  if (currentDragHeight.value !== null) {
    const halfH = getHalfHeightPx();
    const expandedH = getExpandedHeightPx();
    const currentH = currentDragHeight.value;
    const midPoint = (halfH + expandedH) / 2;

    if (currentH > midPoint) {
      sheetState.value = "expanded";
    } else {
      sheetState.value = "half";
    }
  }
  currentDragHeight.value = null;
}

function onTouchEnd() {
  endDrag();
}

function onMouseUp() {
  endDrag();
}

onUnmounted(() => {
  window.removeEventListener("touchmove", onTouchMove);
  window.removeEventListener("touchend", onTouchEnd);
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", onMouseUp);
});
</script>

<template>
  <div class="mobile-flow-screen">
    <!-- PERSISTENT IMAGE STAGE (Behind & above the sheet) -->
    <div class="mobile-image-stage">
      <!-- Floating Top Navigation -->
      <div class="floating-top-nav">
        <button
          type="button"
          class="btn-floating-nav"
          aria-label="Back to image upload"
          @click="emit('back')"
        >
          <ArrowLeft class="nav-icon" :stroke-width="2.2" />
          <span>Upload</span>
        </button>

        <button
          v-if="previewUrl"
          type="button"
          class="btn-floating-nav btn-floating-nav-icon"
          aria-label="Inspect original image in full screen"
          title="Zoom image"
          @click="emit('openImageModal')"
        >
          <Maximize2 class="nav-icon" :stroke-width="2.2" />
        </button>
      </div>

      <!-- Main Flyer Image -->
      <div
        v-if="previewUrl"
        class="flyer-img-container"
        role="button"
        tabindex="0"
        aria-label="View original image full screen"
        @click="emit('openImageModal')"
      >
        <img :src="previewUrl" :alt="fileName || 'Original Flyer'" class="flyer-img" />
        <div class="tap-hint-pill" aria-hidden="true">
          <span>Tap image to zoom</span>
        </div>
      </div>
    </div>

    <!-- DRAGGABLE FLOATING BOTTOM PANEL -->
    <section
      ref="sheetRef"
      class="floating-bottom-panel"
      :class="{
        'is-half': sheetState === 'half' && !isDragging,
        'is-expanded': sheetState === 'expanded' && !isDragging,
        'is-dragging': isDragging,
      }"
      :style="currentDragHeight ? { height: `${currentDragHeight}px` } : undefined"
      aria-label="Event summary floating panel"
    >
      <!-- Drag Handle & Header -->
      <div
        class="panel-drag-zone"
        role="button"
        tabindex="0"
        :aria-expanded="sheetState === 'expanded'"
        aria-label="Drag up to cover image, or drag down to view both at once"
        @touchstart="handleTouchStart"
        @mousedown="handleMouseDown"
        @click="handleZoneClick"
        @keydown.enter="toggleSheet"
        @keydown.space.prevent="toggleSheet"
      >
        <div class="drag-handle-pill" aria-hidden="true"></div>

        <div class="panel-header-row">
          <div class="panel-meta-info">
            <span class="panel-event-count">
              {{ totalEvents }} {{ totalEvents === 1 ? "Event" : "Events" }}
            </span>
            <span v-if="overallConfidence" class="badge-pill badge-pill-confidence">
              {{ overallConfidence }}% match
            </span>
          </div>

          <button
            type="button"
            class="panel-toggle-indicator"
            :aria-label="sheetState === 'expanded' ? 'Collapse panel' : 'Expand panel'"
            @click.stop="toggleSheet"
          >
            <ChevronDown v-if="sheetState === 'expanded'" class="indicator-icon" />
            <ChevronUp v-else class="indicator-icon" />
          </button>
        </div>

        <!-- View Switcher: Detail View vs Calendar View -->
        <div
          class="panel-view-switcher"
          role="tablist"
          aria-label="Switch between detail view and calendar view"
          @click.stop
        >
          <button
            type="button"
            role="tab"
            :aria-selected="viewMode === 'details'"
            class="switcher-tab"
            :class="{ active: viewMode === 'details' }"
            @click="viewMode = 'details'"
          >
            <List class="tab-icon" :stroke-width="2.2" />
            <span>Detail View</span>
          </button>

          <button
            type="button"
            role="tab"
            :aria-selected="viewMode === 'week'"
            class="switcher-tab"
            :class="{ active: viewMode === 'week' }"
            @click="viewMode = 'week'"
          >
            <CalendarIcon class="tab-icon" :stroke-width="2.2" />
            <span>Calendar View</span>
          </button>
        </div>
      </div>

      <!-- Scrollable Panel Body -->
      <div class="panel-content-body">
        <slot></slot>
      </div>
    </section>
  </div>
</template>

<style scoped>
.mobile-flow-screen {
  position: relative;
  width: 100%;
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
  background-color: #12151c;
  display: flex;
  flex-direction: column;
}

/* Image Stage (Background Layer) */
.mobile-image-stage {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  overflow: hidden;
  background: radial-gradient(circle at 50% 30%, #1e2430 0%, #0c0f14 100%);
  padding-top: max(0.75rem, env(safe-area-inset-top));
}

/* Floating Top Navigation */
.floating-top-nav {
  position: absolute;
  top: max(0.75rem, env(safe-area-inset-top));
  left: 1rem;
  right: 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  z-index: 40;
  pointer-events: auto;
}

.btn-floating-nav {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.85rem;
  border-radius: 999px;
  background: rgba(18, 21, 28, 0.78);
  -webkit-backdrop-filter: blur(16px);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.16);
  color: #ffffff;
  font-size: 0.84rem;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
  transition: all 0.15s ease;
}

.btn-floating-nav:hover {
  background: rgba(26, 32, 44, 0.9);
  border-color: rgba(255, 255, 255, 0.28);
}

.btn-floating-nav:active {
  transform: scale(0.96);
}

.btn-floating-nav-icon {
  width: 36px;
  height: 36px;
  padding: 0;
  justify-content: center;
}

.nav-icon {
  width: 16px;
  height: 16px;
}

/* Flyer Image Stage */
.flyer-img-container {
  position: relative;
  width: 100%;
  height: 52vh;
  height: 52dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 3.5rem 1rem 1rem 1rem;
  box-sizing: border-box;
  cursor: pointer;
}

.flyer-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 12px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.tap-hint-pill {
  position: absolute;
  bottom: 1.25rem;
  background: rgba(0, 0, 0, 0.65);
  -webkit-backdrop-filter: blur(8px);
  backdrop-filter: blur(8px);
  color: rgba(255, 255, 255, 0.85);
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.25rem 0.65rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  pointer-events: none;
}

/* Floating Bottom Panel */
.floating-bottom-panel {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 30;
  background: var(--bg-card);
  border-top-left-radius: 26px;
  border-top-right-radius: 26px;
  box-shadow: 0 -8px 36px rgba(0, 0, 0, 0.35);
  border-top: 1px solid var(--border-card);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-sizing: border-box;
  will-change: height;
}

.floating-bottom-panel.is-half {
  height: 50vh;
  height: 50dvh;
  transition: height 0.32s cubic-bezier(0.16, 1, 0.3, 1);
}

.floating-bottom-panel.is-expanded {
  height: calc(92vh - env(safe-area-inset-top));
  height: calc(92dvh - env(safe-area-inset-top));
  transition: height 0.32s cubic-bezier(0.16, 1, 0.3, 1);
}

.floating-bottom-panel.is-dragging {
  transition: none;
}

/* Drag Zone & Header */
.panel-drag-zone {
  padding: 0.5rem 1rem 0.65rem 1rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  border-bottom: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.05));
  background: var(--bg-card-elevated, var(--bg-card));
  cursor: grab;
  user-select: none;
  touch-action: none;
}

.panel-drag-zone:active {
  cursor: grabbing;
}

.drag-handle-pill {
  width: 42px;
  height: 5px;
  border-radius: 999px;
  background: var(--text-tertiary);
  opacity: 0.5;
  transition: opacity 0.15s ease;
}

.panel-drag-zone:hover .drag-handle-pill {
  opacity: 0.8;
}

.panel-header-row {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-meta-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.panel-event-count {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.panel-toggle-indicator {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-secondary);
  cursor: pointer;
}

.indicator-icon {
  width: 16px;
  height: 16px;
}

/* Segmented View Switcher: Detail View vs Calendar View */
.panel-view-switcher {
  display: grid;
  grid-template-columns: 1fr 1fr;
  width: 100%;
  background: var(--bg-input);
  padding: 0.2rem;
  border-radius: 12px;
  border: 1px solid var(--border-input);
  gap: 0.25rem;
}

.switcher-tab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  padding: 0.5rem 0.75rem;
  border-radius: 10px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  white-space: nowrap;
}

.switcher-tab.active {
  background: var(--bg-card);
  color: var(--accent-primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.tab-icon {
  width: 15px;
  height: 15px;
}

/* Scrollable Panel Body */
.panel-content-body {
  flex: 1;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  padding: 0.85rem 1rem max(2rem, env(safe-area-inset-bottom)) 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
</style>
