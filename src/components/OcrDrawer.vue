<script setup lang="ts">
import { ref, computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isWebLink, type OcrResult } from "../services/ocr";
const props = defineProps<{
  ocrResult: OcrResult;
}>();

const emit = defineEmits<{
  (e: "reparse"): void;
}>();

const showOcrSection = ref(false);
const showLineDetails = ref(false);
const copiedOcr = ref(false);
const copiedQrIdx = ref<number | null>(null);

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

async function copyQrToClipboard(url: string, idx: number) {
  try {
    await navigator.clipboard.writeText(url);
    copiedQrIdx.value = idx;
    setTimeout(() => {
      if (copiedQrIdx.value === idx) {
        copiedQrIdx.value = null;
      }
    }, 2000);
  } catch (err) {
    console.error("Failed to copy QR link:", err);
  }
}

const wordCount = computed(() => {
  if (!props.ocrResult?.text) return 0;
  return props.ocrResult.text.trim().split(/\s+/).filter(Boolean).length;
});

const averageConfidence = computed(() => {
  if (!props.ocrResult?.lines || props.ocrResult.lines.length === 0) return 0;
  const total = props.ocrResult.lines.reduce((sum, line) => sum + line.confidence, 0);
  return Math.round((total / props.ocrResult.lines.length) * 100);
});

async function copyOcrToClipboard() {
  if (!props.ocrResult?.text) return;
  try {
    await navigator.clipboard.writeText(props.ocrResult.text);
    copiedOcr.value = true;
    setTimeout(() => {
      copiedOcr.value = false;
    }, 2000);
  } catch (err) {
    console.error("Failed to copy OCR text:", err);
  }
}
</script>

<template>
  <div class="surface-card ocr-accordion">
    <button type="button" class="accordion-trigger" @click="showOcrSection = !showOcrSection">
      <div class="accordion-title-wrap">
        <svg
          class="accordion-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="4 7 4 4 20 4 20 7"></polyline>
          <line x1="9" y1="20" x2="15" y2="20"></line>
          <line x1="12" y1="4" x2="12" y2="20"></line>
        </svg>
        <span class="accordion-title">Extracted OCR Text</span>
        <span class="chip-count"
          >{{ ocrResult.lines.length }} lines • {{ wordCount }} words<template
            v-if="ocrResult.qr_codes && ocrResult.qr_codes.length > 0"
          >
            • {{ ocrResult.qr_codes.length }} QR</template
          >
          • {{ averageConfidence }}% conf</span
        >
      </div>

      <svg
        class="accordion-chevron"
        :class="{ 'is-open': showOcrSection }"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <polyline points="6 9 12 15 18 9"></polyline>
      </svg>
    </button>

    <div v-if="showOcrSection" class="accordion-content">
      <div class="ocr-toolbar">
        <button type="button" class="btn-subtle-tool" @click="emit('reparse')">
          <svg
            class="btn-icon-xs"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="23 4 23 10 17 10"></polyline>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
          </svg>
          <span>Re-parse</span>
        </button>

        <button
          type="button"
          class="btn-subtle-tool"
          :class="{ 'is-copied': copiedOcr }"
          :disabled="!ocrResult.text.trim()"
          @click="copyOcrToClipboard"
        >
          <template v-if="copiedOcr">
            <svg
              class="btn-icon-xs"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
            <span>Copied Text</span>
          </template>
          <template v-else>
            <svg
              class="btn-icon-xs"
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
            <span>Copy OCR</span>
          </template>
        </button>
      </div>

      <!-- Detected QR Codes Section -->
      <div v-if="ocrResult.qr_codes && ocrResult.qr_codes.length > 0" class="qr-codes-container">
        <div class="qr-section-header">
          <svg
            class="qr-header-icon"
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
          <span class="qr-section-title"
            >Detected QR Codes & Links ({{ ocrResult.qr_codes.length }})</span
          >
        </div>

        <div class="qr-chips-list">
          <div v-for="(code, idx) in ocrResult.qr_codes" :key="idx" class="qr-chip-card">
            <div class="qr-chip-info">
              <svg
                class="qr-link-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
              </svg>
              <a
                v-if="isWebLink(code)"
                :href="code"
                target="_blank"
                rel="noopener noreferrer"
                class="qr-url-link"
                :title="code"
                @click="handleOpenUrl($event, code)"
              >
                {{ code }}
              </a>
              <span v-else class="qr-url-link is-text" :title="code">
                {{ code }}
              </span>
            </div>

            <div class="qr-chip-actions">
              <button
                type="button"
                class="btn-chip-action"
                :class="{ 'is-copied': copiedQrIdx === idx }"
                :title="copiedQrIdx === idx ? 'Copied!' : 'Copy link to clipboard'"
                @click="copyQrToClipboard(code, idx)"
              >
                <template v-if="copiedQrIdx === idx">
                  <svg
                    class="action-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                  <span>Copied</span>
                </template>
                <template v-else>
                  <svg
                    class="action-icon"
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
                  <span>Copy</span>
                </template>
              </button>
            </div>
          </div>
        </div>
      </div>

      <textarea
        readonly
        class="ocr-raw-display"
        :value="ocrResult.text"
        rows="5"
        placeholder="No text detected."
      ></textarea>

      <!-- Line Details Toggle -->
      <div v-if="ocrResult.lines.length > 0" class="line-details-block">
        <button type="button" class="btn-line-toggle" @click="showLineDetails = !showLineDetails">
          <span>{{ showLineDetails ? "Hide" : "Show" }} line-by-line confidence</span>
          <svg
            class="btn-icon-xs chevron-sm"
            :class="{ 'is-rotated': showLineDetails }"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </button>

        <div v-if="showLineDetails" class="line-breakdown-list">
          <div v-for="(line, idx) in ocrResult.lines" :key="idx" class="line-item">
            <span class="line-idx">{{ idx + 1 }}</span>
            <span class="line-content">{{ line.text }}</span>
            <span
              class="line-score"
              :class="{
                'score-high': line.confidence >= 0.8,
                'score-med': line.confidence >= 0.5 && line.confidence < 0.8,
                'score-low': line.confidence < 0.5,
              }"
            >
              {{ Math.round(line.confidence * 100) }}%
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.qr-codes-container {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.65rem 0.75rem;
  border-radius: var(--radius-sm, 8px);
  background: var(--bg-surface-elevated, rgba(59, 130, 246, 0.05));
  border: 1px solid var(--border-card-subtle, rgba(59, 130, 246, 0.15));
}

.qr-section-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.qr-header-icon {
  width: 14px;
  height: 14px;
  color: var(--accent-primary, #3b82f6);
}

.qr-section-title {
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--accent-primary, #3b82f6);
}

.qr-chips-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.qr-chip-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.35rem 0.5rem;
  background: var(--bg-input, rgba(0, 0, 0, 0.04));
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.08));
  border-radius: 6px;
  font-size: 0.8rem;
}

.qr-chip-info {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  min-width: 0;
  flex: 1;
}

.qr-link-icon {
  width: 13px;
  height: 13px;
  flex-shrink: 0;
  color: var(--accent-primary, #3b82f6);
}

.qr-url-link {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--accent-primary, #3b82f6);
  text-decoration: none;
  font-weight: 500;
}

.qr-url-link:hover {
  text-decoration: underline;
}

.qr-url-link.is-text {
  color: var(--text-primary, #111827);
  cursor: text;
}

.qr-url-link.is-text:hover {
  text-decoration: none;
}

.qr-chip-actions {
  flex-shrink: 0;
}

.btn-chip-action {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  background: var(--bg-surface, #ffffff);
  border: 1px solid var(--border-card-subtle, rgba(0, 0, 0, 0.12));
  border-radius: 4px;
  padding: 0.15rem 0.45rem;
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary, #4b5563);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-chip-action:hover {
  background: var(--bg-surface-elevated, #f3f4f6);
  color: var(--text-primary, #111827);
}

.btn-chip-action.is-copied {
  color: var(--accent-success, #10b981);
  border-color: var(--accent-success, #10b981);
}

.action-icon {
  width: 11px;
  height: 11px;
}
.ocr-accordion {
  padding: 0.9rem 1.1rem;
  gap: 0.75rem;
}

.accordion-trigger {
  width: 100%;
  background: none;
  border: none;
  padding: 0.25rem 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  color: var(--text-primary);
  text-align: left;
}

.accordion-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.accordion-icon {
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
}

.accordion-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--text-primary);
}

.chip-count {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
}

.accordion-chevron {
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.accordion-chevron.is-open {
  transform: rotate(180deg);
}

.accordion-content {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border-card-subtle);
}

.ocr-toolbar {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.btn-subtle-tool {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-secondary);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.35rem 0.65rem;
  border-radius: 8px;
  transition: all 0.15s ease;
}

.btn-subtle-tool:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}

.btn-subtle-tool.is-copied {
  background: rgba(52, 199, 89, 0.12);
  border-color: rgba(52, 199, 89, 0.4);
  color: #15803d;
}

.btn-icon-xs {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.ocr-raw-display {
  width: 100%;
  box-sizing: border-box;
  padding: 0.75rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.82rem;
  line-height: 1.5;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 10px;
  resize: vertical;
}

.ocr-raw-display:focus {
  outline: none;
  border-color: var(--accent-primary);
}

.line-details-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.btn-line-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: none;
  border: none;
  color: var(--accent-primary);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0.2rem 0;
  width: fit-content;
}

.chevron-sm {
  transition: transform 0.2s ease;
}

.chevron-sm.is-rotated {
  transform: rotate(180deg);
}

.line-breakdown-list {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--border-input);
  border-radius: 8px;
  background: var(--bg-input);
}

.line-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.35rem 0.6rem;
  border-bottom: 1px solid var(--border-card-subtle);
  font-size: 0.8rem;
}

.line-item:last-child {
  border-bottom: none;
}

.line-idx {
  color: var(--text-tertiary);
  font-family: monospace;
  font-size: 0.75rem;
  min-width: 18px;
}

.line-content {
  flex: 1;
  color: var(--text-primary);
  word-break: break-word;
}

.line-score {
  font-family: monospace;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.1rem 0.3rem;
  border-radius: 4px;
}

.score-high {
  background: rgba(52, 199, 89, 0.15);
  color: #15803d;
}

.score-med {
  background: rgba(234, 179, 8, 0.15);
  color: #a16207;
}

.score-low {
  background: rgba(239, 68, 68, 0.15);
  color: #b91c1c;
}

@media (prefers-color-scheme: dark) {
  .score-high {
    background: rgba(48, 209, 88, 0.2);
    color: #4ade80;
  }
  .score-med {
    background: rgba(250, 204, 21, 0.2);
    color: #fde047;
  }
  .score-low {
    background: rgba(248, 113, 113, 0.2);
    color: #fca5a5;
  }
}
</style>
