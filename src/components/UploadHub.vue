<script setup lang="ts">
import { ref, computed } from "vue";

const props = defineProps<{
  isDragging?: boolean;
}>();

const emit = defineEmits<{
  (e: "selectFile", file: File): void;
  (e: "chooseFile"): void;
  (e: "takePhoto"): void;
}>();

const localDragging = ref(false);
const activeDragging = computed(() => props.isDragging || localDragging.value);

function handleDragOver(event: DragEvent) {
  event.preventDefault();
  localDragging.value = true;
}

function handleDragLeave(event: DragEvent) {
  event.preventDefault();
  localDragging.value = false;
}

function handleDrop(event: DragEvent) {
  event.preventDefault();
  localDragging.value = false;

  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    emit("selectFile", event.dataTransfer.files[0]);
  }
}
</script>

<template>
  <section
    class="upload-hub"
    :class="{ 'is-dragging': activeDragging }"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <div class="upload-hero">
      <div class="upload-icon-bubble">
        <svg
          class="upload-bubble-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect x="3" y="3" width="18" height="18" rx="3" ry="3"></rect>
          <circle cx="8.5" cy="8.5" r="1.5"></circle>
          <polyline points="21 15 16 10 5 21"></polyline>
        </svg>
      </div>
      <h2 class="upload-title">Add Flyer or Screenshot</h2>
      <p class="upload-subtitle">
        Drag and drop a flyer or screenshot here, or choose a file to begin.
      </p>
    </div>

    <div class="upload-actions">
      <button type="button" class="btn-touch btn-touch-primary" @click="emit('chooseFile')">
        <svg
          class="btn-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
          <polyline points="17 8 12 3 7 8"></polyline>
          <line x1="12" y1="3" x2="12" y2="15"></line>
        </svg>
        <span>Choose from Library</span>
      </button>

      <button type="button" class="btn-touch btn-touch-secondary" @click="emit('takePhoto')">
        <svg
          class="btn-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path
            d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
          ></path>
          <circle cx="12" cy="13" r="4"></circle>
        </svg>
        <span>Take Photo</span>
      </button>
    </div>

    <div class="upload-footer">
      <p class="format-note">Supports PNG, JPG, HEIF, WebP • Drag & drop or paste via ⌘V</p>
    </div>
  </section>
</template>

<style scoped>
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
</style>
