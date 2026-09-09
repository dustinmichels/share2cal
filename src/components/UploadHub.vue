<script setup lang="ts">
import { ref, computed } from "vue";
import { Sparkles, ClipboardPaste, X } from "lucide-vue-next";

const props = defineProps<{
  isDragging?: boolean;
}>();

const emit = defineEmits<{
  (e: "selectFile", file: File): void;
  (e: "chooseFile"): void;
  (e: "takePhoto"): void;
  (e: "pasteText", text: string): void;
}>();

const localDragging = ref(false);
const activeDragging = computed(() => props.isDragging || localDragging.value);

const showTextModal = ref(false);
const manualText = ref("");

async function handlePasteTextClick() {
  try {
    if (navigator?.clipboard?.readText) {
      const text = await navigator.clipboard.readText();
      if (text && text.trim()) {
        emit("pasteText", text.trim());
        return;
      }
    }
  } catch {
    // Clipboard read access denied or unavailable; fall through to modal
  }
  showTextModal.value = true;
}

function submitManualText() {
  const trimmed = manualText.value.trim();
  if (!trimmed) return;
  emit("pasteText", trimmed);
  manualText.value = "";
  showTextModal.value = false;
}

function handleDragOver(event: DragEvent) {
  event.preventDefault();
  localDragging.value = true;
}

function handleDragLeave(event: DragEvent) {
  event.preventDefault();
  localDragging.value = false;
}

async function handleDrop(event: DragEvent) {
  event.preventDefault();
  localDragging.value = false;

  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    const file = event.dataTransfer.files[0];
    if (file.type === "text/plain" || file.name.endsWith(".txt")) {
      try {
        const text = await file.text();
        if (text && text.trim()) {
          emit("pasteText", text.trim());
          return;
        }
      } catch {
        // Fallback to selectFile
      }
    }
    emit("selectFile", file);
    return;
  }

  const text = event.dataTransfer?.getData("text");
  if (text && text.trim()) {
    emit("pasteText", text.trim());
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
      <h2 class="upload-title">Add Flyer, Screenshot or Text</h2>
      <p class="upload-subtitle">
        Drag & drop a flyer, choose a file, or paste event text to begin.
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
      <button type="button" class="btn-touch btn-touch-secondary btn-paste-text" @click="handlePasteTextClick">
        <ClipboardPaste class="btn-icon" :stroke-width="2.2" />
        <span>Paste Text</span>
      </button>
    </div>
    <div class="upload-parse-wrap">
      <button
        type="button"
        class="btn-touch btn-touch-parse is-disabled"
        disabled
        aria-disabled="true"
        title="Upload a picture first to activate Parse"
      >
        <Sparkles class="btn-icon" :stroke-width="2.2" />
        <span>Parse</span>
      </button>
      <p class="parse-hint">Upload a picture to activate parse</p>
    </div>
    <div class="upload-footer">
      <p class="format-note">Supports PNG, JPG, HEIF, WebP, or paste text • Drag & drop or paste via ⌘V</p>
    </div>
  </section>

    <!-- Modal Dialog for Pasting / Typing Text -->
    <Teleport to="body">
      <div
        v-if="showTextModal"
        class="paste-text-overlay"
        role="dialog"
        aria-modal="true"
        aria-label="Paste Event Text"
        @keydown.esc="showTextModal = false"
      >
        <div class="paste-text-backdrop" @click="showTextModal = false"></div>
        <div class="paste-text-dialog">
          <div class="paste-text-header">
            <div class="paste-text-header-title">
              <ClipboardPaste class="dialog-icon" :stroke-width="2.2" />
              <div>
                <h3 class="dialog-title">Paste Event Text</h3>
                <span class="dialog-subtitle">Paste any invitation, email, flyer details, or notes</span>
              </div>
            </div>
            <button
              type="button"
              class="dialog-close-btn"
              aria-label="Close"
              @click="showTextModal = false"
            >
              <X class="btn-icon-sm" :stroke-width="2.2" />
            </button>
          </div>
          <div class="paste-text-body">
            <textarea
              v-model="manualText"
              class="paste-textarea"
              placeholder="Paste or type event text here...&#10;e.g. Pottery Class&#10;Tuesdays & Thursdays 6:00 PM - 8:00 PM&#10;Community Arts Center&#10;https://arts.example.org"
              rows="6"
              autofocus
              @keydown.meta.enter="submitManualText"
              @keydown.ctrl.enter="submitManualText"
            ></textarea>
          </div>
          <div class="paste-text-footer">
            <button
              type="button"
              class="btn-touch btn-touch-secondary btn-cancel"
              @click="showTextModal = false"
            >
              Cancel
            </button>
            <button
              type="button"
              class="btn-touch btn-touch-primary btn-submit"
              :disabled="!manualText.trim()"
              @click="submitManualText"
            >
              <Sparkles class="btn-icon" :stroke-width="2.2" />
              <span>Parse Event</span>
            </button>
          </div>
        </div>
      </div>
    </Teleport>
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

.upload-parse-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.45rem;
  width: 100%;
  margin-top: 0.25rem;
}

.btn-touch-parse {
  background: linear-gradient(135deg, var(--accent-primary) 0%, #0056b3 100%);
  color: #ffffff;
  box-shadow: var(--shadow-primary-btn);
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: -0.01em;
}

.btn-touch-parse.is-disabled,
.btn-touch-parse:disabled {
  background: var(--bg-input);
  color: var(--text-tertiary);
  border: 1px solid var(--border-input);
  box-shadow: none;
  cursor: not-allowed;
  opacity: 0.6;
}

.parse-hint {
  font-size: 0.8rem;
  color: var(--text-tertiary);
  margin: 0;
}

/* Paste Text Dialog */
.paste-text-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.25rem;
}

.paste-text-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(6px);
}

.paste-text-dialog {
  position: relative;
  z-index: 1;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card);
  width: 100%;
  max-width: 520px;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-card);
  overflow: hidden;
}

.paste-text-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.25rem 0.75rem 1.25rem;
}

.paste-text-header-title {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.dialog-icon {
  width: 24px;
  height: 24px;
  color: var(--accent-primary);
  flex-shrink: 0;
}

.dialog-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--text-primary);
}

.dialog-subtitle {
  font-size: 0.78rem;
  color: var(--text-tertiary);
}

.dialog-close-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.dialog-close-btn:hover {
  color: var(--text-primary);
}

.paste-text-body {
  padding: 0.75rem 1.25rem;
}

.paste-textarea {
  width: 100%;
  background: var(--bg-main, #0f141c);
  border: 1.5px solid var(--border-input);
  border-radius: 10px;
  padding: 0.85rem;
  font-family: inherit;
  font-size: 0.9rem;
  line-height: 1.5;
  color: var(--text-primary);
  resize: vertical;
  box-sizing: border-box;
  outline: none;
  transition: border-color 0.15s ease;
}

.paste-textarea:focus {
  border-color: var(--accent-primary);
}

.paste-text-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0.75rem 1.25rem 1.25rem 1.25rem;
}

.btn-cancel {
  width: auto;
  min-height: 42px;
  padding: 0.5rem 1rem;
  font-size: 0.88rem;
}

.btn-submit {
  width: auto;
  min-height: 42px;
  padding: 0.5rem 1.25rem;
  font-size: 0.9rem;
}
</style>
