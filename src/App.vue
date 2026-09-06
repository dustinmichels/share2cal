<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { extractTextFromBytes, type OcrResult } from "./services/ocr";

const selectedFile = ref<File | null>(null);
const previewUrl = ref<string | null>(null);
const isDragging = ref(false);
const isProcessing = ref(false);
const errorMessage = ref<string | null>(null);
const ocrResult = ref<OcrResult | null>(null);
const copied = ref(false);
const showLineDetails = ref(false);

const fileInputRef = ref<HTMLInputElement | null>(null);
const cameraInputRef = ref<HTMLInputElement | null>(null);

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const wordCount = computed(() => {
  if (!ocrResult.value?.text) return 0;
  return ocrResult.value.text.trim().split(/\s+/).filter(Boolean).length;
});

const averageConfidence = computed(() => {
  if (!ocrResult.value?.lines || ocrResult.value.lines.length === 0) return 0;
  const total = ocrResult.value.lines.reduce((sum, line) => sum + line.confidence, 0);
  return Math.round((total / ocrResult.value.lines.length) * 100);
});

function setImageFile(file: File) {
  if (!file.type.startsWith("image/") && !file.name.match(/\.(heic|heif|png|jpe?g|webp|bmp|gif)$/i)) {
    errorMessage.value = "Please select a valid image file (PNG, JPEG, HEIF, WebP, etc.).";
    return;
  }

  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }

  selectedFile.value = file;
  previewUrl.value = URL.createObjectURL(file);
  errorMessage.value = null;
  ocrResult.value = null;
}

function handleFileInput(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    setImageFile(target.files[0]);
    target.value = "";
  }
}

function handleCameraInput(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    setImageFile(target.files[0]);
    target.value = "";
  }
}

function triggerFileUpload() {
  fileInputRef.value?.click();
}

function triggerCameraCapture() {
  cameraInputRef.value?.click();
}

function handleDragOver(event: DragEvent) {
  event.preventDefault();
  isDragging.value = true;
}

function handleDragLeave(event: DragEvent) {
  event.preventDefault();
  isDragging.value = false;
}

function handleDrop(event: DragEvent) {
  event.preventDefault();
  isDragging.value = false;

  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    setImageFile(event.dataTransfer.files[0]);
  }
}

function handlePaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items;
  if (!items) return;

  for (let i = 0; i < items.length; i++) {
    if (items[i].type.startsWith("image/")) {
      const file = items[i].getAsFile();
      if (file) {
        setImageFile(file);
        break;
      }
    }
  }
}

async function handleGo() {
  if (!selectedFile.value) return;

  errorMessage.value = null;
  isProcessing.value = true;

  try {
    const arrayBuffer = await selectedFile.value.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);
    const res = await extractTextFromBytes(bytes);
    ocrResult.value = res;
    if (!res.text.trim()) {
      errorMessage.value = "No text was detected in this image. Try another photo with clearer text.";
    }
  } catch (err: any) {
    errorMessage.value = err?.toString() || "Failed to process OCR on the selected image.";
  } finally {
    isProcessing.value = false;
  }
}

function handleReset() {
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
  selectedFile.value = null;
  previewUrl.value = null;
  ocrResult.value = null;
  errorMessage.value = null;
  showLineDetails.value = false;
}

async function copyToClipboard() {
  if (!ocrResult.value?.text) return;
  try {
    await navigator.clipboard.writeText(ocrResult.value.text);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy text:", err);
  }
}

onMounted(() => {
  window.addEventListener("paste", handlePaste);
});

onUnmounted(() => {
  window.removeEventListener("paste", handlePaste);
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value);
  }
});
</script>

<template>
  <main class="app-container">
    <!-- Header -->
    <header class="app-header">
      <div class="logo-badge">
        <svg class="logo-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
          <line x1="16" y1="2" x2="16" y2="6"></line>
          <line x1="8" y1="2" x2="8" y2="6"></line>
          <line x1="3" y1="10" x2="21" y2="10"></line>
          <path d="M8 14h.01"></path>
          <path d="M12 14h.01"></path>
          <path d="M16 14h.01"></path>
          <path d="M8 18h.01"></path>
          <path d="M12 18h.01"></path>
        </svg>
      </div>
      <h1 class="app-title">Share2Cal</h1>
      <p class="app-tagline">Upload an image or take a photo to extract text and details</p>
    </header>

    <!-- Hidden file inputs -->
    <input
      ref="fileInputRef"
      type="file"
      accept="image/*,.heic,.heif"
      class="hidden-input"
      @change="handleFileInput"
    />
    <input
      ref="cameraInputRef"
      type="file"
      accept="image/*"
      capture="environment"
      class="hidden-input"
      @change="handleCameraInput"
    />

    <!-- Upload / Capture Area (when no image selected) -->
    <section
      v-if="!selectedFile"
      class="upload-zone"
      :class="{ 'dragging': isDragging }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
    >
      <div class="upload-zone-content">
        <div class="upload-icon-circle">
          <svg class="upload-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="17 8 12 3 7 8"></polyline>
            <line x1="12" y1="3" x2="12" y2="15"></line>
          </svg>
        </div>

        <h2 class="upload-heading">Select an image to get started</h2>
        <p class="upload-subtext">Drop your flyer, screenshot, or invitation here</p>

        <div class="action-buttons-group">
          <button type="button" class="btn btn-primary" @click="triggerFileUpload">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              <circle cx="8.5" cy="8.5" r="1.5"></circle>
              <polyline points="21 15 16 10 5 21"></polyline>
            </svg>
            <span>Upload Image</span>
          </button>

          <button type="button" class="btn btn-secondary" @click="triggerCameraCapture">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"></path>
              <circle cx="12" cy="13" r="4"></circle>
            </svg>
            <span>Take Picture</span>
          </button>
        </div>

        <p class="paste-hint">Supports PNG, JPG, HEIF • You can also paste from clipboard (⌘V)</p>
      </div>
    </section>

    <!-- Image Selected State -->
    <section v-else class="selected-state-section">
      <!-- Preview Card -->
      <div class="card preview-card">
        <div class="preview-header">
          <div class="file-meta">
            <span class="file-name" :title="selectedFile.name">{{ selectedFile.name }}</span>
            <span class="file-size">{{ formatFileSize(selectedFile.size) }}</span>
          </div>
          <button type="button" class="btn-text-danger" :disabled="isProcessing" @click="handleReset">
            <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"></polyline>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
            <span>Remove</span>
          </button>
        </div>

        <div class="preview-container">
          <img v-if="previewUrl" :src="previewUrl" alt="Selected image preview" class="preview-image" />
        </div>

        <!-- Go Button Bar -->
        <div class="action-bar">
          <button
            type="button"
            class="btn btn-go"
            :disabled="isProcessing"
            @click="handleGo"
          >
            <template v-if="!isProcessing">
              <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
              <span>Go</span>
            </template>
            <template v-else>
              <div class="spinner"></div>
              <span>Processing OCR...</span>
            </template>
          </button>

          <div class="alt-actions">
            <button type="button" class="btn-link" :disabled="isProcessing" @click="triggerFileUpload">
              Choose another
            </button>
            <span class="dot-separator">•</span>
            <button type="button" class="btn-link" :disabled="isProcessing" @click="triggerCameraCapture">
              Take new photo
            </button>
          </div>
        </div>
      </div>

      <!-- Error State -->
      <div v-if="errorMessage" class="alert-box alert-error">
        <svg class="alert-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
        <div class="alert-content">
          <span class="alert-title">Processing Error</span>
          <p class="alert-message">{{ errorMessage }}</p>
        </div>
      </div>

      <!-- OCR Results -->
      <div v-if="ocrResult" class="card result-card">
        <div class="result-header">
          <div class="result-title-group">
            <h2 class="result-title">OCR Result</h2>
            <div class="badges-group">
              <span class="badge badge-primary">{{ ocrResult.lines.length }} lines</span>
              <span class="badge badge-secondary">{{ wordCount }} words</span>
              <span v-if="averageConfidence > 0" class="badge badge-accent">{{ averageConfidence }}% conf</span>
            </div>
          </div>

          <button
            type="button"
            class="btn btn-copy"
            :class="{ 'copied': copied }"
            :disabled="!ocrResult.text.trim()"
            @click="copyToClipboard"
          >
            <template v-if="copied">
              <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
              <span>Copied!</span>
            </template>
            <template v-else>
              <svg class="btn-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>Copy Text</span>
            </template>
          </button>
        </div>

        <div class="result-body">
          <textarea
            readonly
            class="ocr-textarea"
            :value="ocrResult.text"
            rows="10"
            placeholder="No text detected."
          ></textarea>
        </div>

        <!-- Line by Line breakdown -->
        <div v-if="ocrResult.lines.length > 0" class="line-breakdown-section">
          <button
            type="button"
            class="line-details-toggle"
            @click="showLineDetails = !showLineDetails"
          >
            <svg
              class="toggle-chevron"
              :class="{ 'rotated': showLineDetails }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
            <span>{{ showLineDetails ? 'Hide' : 'Show' }} line-by-line confidence details</span>
          </button>

          <div v-if="showLineDetails" class="lines-container">
            <ul class="lines-list">
              <li v-for="(line, idx) in ocrResult.lines" :key="idx" class="line-row">
                <span class="line-number">{{ idx + 1 }}</span>
                <span class="line-text">{{ line.text }}</span>
                <span
                  class="line-confidence"
                  :class="{
                    'conf-high': line.confidence >= 0.8,
                    'conf-med': line.confidence >= 0.5 && line.confidence < 0.8,
                    'conf-low': line.confidence < 0.5
                  }"
                >
                  {{ Math.round(line.confidence * 100) }}%
                </span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  </main>
</template>

<style scoped>
.app-container {
  max-width: 640px;
  margin: 0 auto;
  padding: 2rem 1.25rem;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  color: #111827;
  box-sizing: border-box;
}

/* Header */
.app-header {
  text-align: center;
  margin-bottom: 2rem;
}

.logo-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  background: linear-gradient(135deg, #007aff 0%, #5856d6 100%);
  border-radius: 14px;
  color: white;
  margin-bottom: 0.75rem;
  box-shadow: 0 4px 12px rgba(0, 122, 255, 0.25);
}

.logo-icon {
  width: 28px;
  height: 28px;
}

.app-title {
  font-size: 2rem;
  font-weight: 800;
  letter-spacing: -0.025em;
  margin: 0 0 0.4rem 0;
  color: #111827;
}

.app-tagline {
  font-size: 0.95rem;
  color: #6b7280;
  margin: 0;
}

/* Hidden inputs */
.hidden-input {
  display: none;
}

/* Card generic */
.card {
  background: #ffffff;
  border-radius: 16px;
  padding: 1.25rem;
  box-shadow: 0 4px 20px -2px rgba(0, 0, 0, 0.06), 0 2px 6px -1px rgba(0, 0, 0, 0.04);
  border: 1px solid rgba(0, 0, 0, 0.05);
  margin-bottom: 1.25rem;
}

/* Upload Zone */
.upload-zone {
  border: 2px dashed #cbd5e1;
  border-radius: 16px;
  background: #f8fafc;
  padding: 2.5rem 1.5rem;
  text-align: center;
  transition: all 0.2s ease-in-out;
}

.upload-zone.dragging {
  border-color: #007aff;
  background: #eff6ff;
  transform: scale(1.01);
}

.upload-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.upload-icon-circle {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: #e0f2fe;
  color: #007aff;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
}

.upload-icon {
  width: 30px;
  height: 30px;
}

.upload-heading {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0 0 0.35rem 0;
  color: #1e293b;
}

.upload-subtext {
  font-size: 0.9rem;
  color: #64748b;
  margin: 0 0 1.5rem 0;
}

.action-buttons-group {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
  justify-content: center;
  margin-bottom: 1.25rem;
}

.paste-hint {
  font-size: 0.8rem;
  color: #94a3b8;
  margin: 0;
}

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.7rem 1.25rem;
  border-radius: 10px;
  font-size: 0.95rem;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease-in-out;
}

.btn:active {
  transform: scale(0.98);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.btn-primary {
  background: #007aff;
  color: #ffffff;
  box-shadow: 0 2px 8px rgba(0, 122, 255, 0.25);
}

.btn-primary:hover:not(:disabled) {
  background: #0066d6;
}

.btn-secondary {
  background: #f1f5f9;
  color: #1e293b;
  border: 1px solid #e2e8f0;
}

.btn-secondary:hover:not(:disabled) {
  background: #e2e8f0;
}

.btn-go {
  width: 100%;
  padding: 0.9rem 1.5rem;
  font-size: 1.1rem;
  font-weight: 700;
  background: linear-gradient(135deg, #007aff 0%, #0056b3 100%);
  color: white;
  border-radius: 12px;
  box-shadow: 0 4px 14px rgba(0, 122, 255, 0.35);
  letter-spacing: 0.02em;
}

.btn-go:hover:not(:disabled) {
  background: linear-gradient(135deg, #006ee6 0%, #004c9e 100%);
  box-shadow: 0 6px 18px rgba(0, 122, 255, 0.4);
}

.btn-icon {
  width: 20px;
  height: 20px;
}

.btn-icon-sm {
  width: 16px;
  height: 16px;
}

.btn-text-danger {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  background: transparent;
  border: none;
  color: #ef4444;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
}

.btn-text-danger:hover:not(:disabled) {
  background: #fee2e2;
}

.btn-link {
  background: transparent;
  border: none;
  color: #007aff;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.25rem 0.4rem;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.btn-link:hover:not(:disabled) {
  color: #0056b3;
}

.btn-copy {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: #f1f5f9;
  border: 1px solid #e2e8f0;
  color: #1e293b;
  font-size: 0.85rem;
  font-weight: 600;
  padding: 0.4rem 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-copy:hover:not(:disabled) {
  background: #e2e8f0;
}

.btn-copy.copied {
  background: #ecfdf5;
  border-color: #a7f3d0;
  color: #059669;
}

/* Preview Card */
.preview-card {
  padding: 1.25rem;
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.85rem;
}

.file-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  overflow: hidden;
}

.file-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: #334155;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 260px;
}

.file-size {
  font-size: 0.8rem;
  color: #94a3b8;
  background: #f1f5f9;
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
}

.preview-container {
  display: flex;
  justify-content: center;
  align-items: center;
  background: #0f172a;
  border-radius: 10px;
  overflow: hidden;
  max-height: 380px;
  margin-bottom: 1.25rem;
}

.preview-image {
  max-width: 100%;
  max-height: 380px;
  object-fit: contain;
  display: block;
}

.action-bar {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
}

.alt-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
  color: #94a3b8;
}

.dot-separator {
  color: #cbd5e1;
}

/* Spinner */
.spinner {
  width: 20px;
  height: 20px;
  border: 3px solid rgba(255, 255, 255, 0.3);
  border-radius: 50%;
  border-top-color: #ffffff;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* Alert Box */
.alert-box {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1rem;
  border-radius: 12px;
  margin-bottom: 1.25rem;
}

.alert-error {
  background: #fef2f2;
  border: 1px solid #fecaca;
  color: #991b1b;
}

.alert-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  margin-top: 0.1rem;
}

.alert-content {
  flex: 1;
}

.alert-title {
  display: block;
  font-weight: 700;
  font-size: 0.9rem;
  margin-bottom: 0.2rem;
}

.alert-message {
  font-size: 0.85rem;
  margin: 0;
  line-height: 1.4;
}

/* Result Card */
.result-card {
  padding: 1.25rem;
}

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.result-title-group {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.result-title {
  font-size: 1.2rem;
  font-weight: 700;
  margin: 0;
  color: #1e293b;
}

.badges-group {
  display: flex;
  gap: 0.4rem;
}

.badge {
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
}

.badge-primary {
  background: #eff6ff;
  color: #1d4ed8;
}

.badge-secondary {
  background: #f1f5f9;
  color: #475569;
}

.badge-accent {
  background: #ecfdf5;
  color: #047857;
}

.ocr-textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 0.85rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.88rem;
  line-height: 1.5;
  color: #1e293b;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  resize: vertical;
}

.ocr-textarea:focus {
  outline: none;
  border-color: #007aff;
  background: #ffffff;
}

/* Line breakdown */
.line-breakdown-section {
  margin-top: 1rem;
  border-top: 1px solid #f1f5f9;
  padding-top: 0.75rem;
}

.line-details-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: transparent;
  border: none;
  color: #007aff;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.3rem 0;
}

.toggle-chevron {
  width: 16px;
  height: 16px;
  transition: transform 0.2s ease;
}

.toggle-chevron.rotated {
  transform: rotate(180deg);
}

.lines-container {
  margin-top: 0.75rem;
  max-height: 220px;
  overflow-y: auto;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #f8fafc;
}

.lines-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.line-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.4rem 0.65rem;
  border-bottom: 1px solid #f1f5f9;
  font-size: 0.82rem;
}

.line-row:last-child {
  border-bottom: none;
}

.line-number {
  color: #94a3b8;
  font-family: monospace;
  font-size: 0.75rem;
  min-width: 20px;
}

.line-text {
  flex: 1;
  color: #334155;
  word-break: break-word;
}

.line-confidence {
  font-family: monospace;
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
}

.conf-high {
  background: #dcfce7;
  color: #15803d;
}

.conf-med {
  background: #fef9c3;
  color: #854d0e;
}

.conf-low {
  background: #fee2e2;
  color: #b91c1c;
}

/* Dark mode */
@media (prefers-color-scheme: dark) {
  .app-container {
    color: #f3f4f6;
  }

  .app-title {
    color: #f9fafb;
  }

  .app-tagline {
    color: #9ca3af;
  }

  .card {
    background: #1e293b;
    border-color: #334155;
    box-shadow: 0 4px 20px -2px rgba(0, 0, 0, 0.4);
  }

  .upload-zone {
    background: #0f172a;
    border-color: #334155;
  }

  .upload-zone.dragging {
    background: #1e293b;
    border-color: #38bdf8;
  }

  .upload-heading {
    color: #f1f5f9;
  }

  .upload-subtext {
    color: #94a3b8;
  }

  .upload-icon-circle {
    background: #1e3a8a;
    color: #60a5fa;
  }

  .btn-secondary {
    background: #334155;
    color: #f1f5f9;
    border-color: #475569;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #475569;
  }

  .btn-copy {
    background: #334155;
    border-color: #475569;
    color: #f1f5f9;
  }

  .btn-copy:hover:not(:disabled) {
    background: #475569;
  }

  .btn-copy.copied {
    background: #064e3b;
    border-color: #047857;
    color: #6ee7b7;
  }

  .file-name {
    color: #f1f5f9;
  }

  .file-size {
    background: #334155;
    color: #94a3b8;
  }

  .btn-text-danger:hover:not(:disabled) {
    background: #450a0a;
  }

  .result-title {
    color: #f1f5f9;
  }

  .badge-primary {
    background: #1e3a8a;
    color: #93c5fd;
  }

  .badge-secondary {
    background: #334155;
    color: #cbd5e1;
  }

  .badge-accent {
    background: #064e3b;
    color: #6ee7b7;
  }

  .ocr-textarea {
    background: #0f172a;
    border-color: #334155;
    color: #f1f5f9;
  }

  .ocr-textarea:focus {
    background: #0b1120;
    border-color: #38bdf8;
  }

  .line-breakdown-section {
    border-top-color: #334155;
  }

  .lines-container {
    background: #0f172a;
    border-color: #334155;
  }

  .line-row {
    border-bottom-color: #1e293b;
  }

  .line-text {
    color: #e2e8f0;
  }

  .conf-high {
    background: #064e3b;
    color: #6ee7b7;
  }

  .conf-med {
    background: #713f12;
    color: #fde047;
  }

  .conf-low {
    background: #7f1d1d;
    color: #fca5a5;
  }

  .alert-error {
    background: #450a0a;
    border-color: #7f1d1d;
    color: #fca5a5;
  }
}
</style>
