<script setup lang="ts">
import type { ParsingMode } from "../services/settings";

defineProps<{
  parsingMode: ParsingMode;
  hasLocalModel?: boolean;
  defaultModelName?: string | null;
}>();

const emit = defineEmits<{
  (e: "openSettings"): void;
}>();
</script>

<template>
  <div class="settings-nav-section">
    <button
      type="button"
      class="btn-settings-card"
      aria-label="Open Settings"
      @click="emit('openSettings')"
    >
      <div class="settings-card-left">
        <div class="settings-card-icon-wrap">
          <svg
            class="settings-card-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="12" cy="12" r="3"></circle>
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            ></path>
          </svg>
        </div>
        <div class="settings-card-body">
          <div class="settings-card-title-line">
            <span class="settings-card-title">Settings</span>
            <span
              class="settings-badge"
              :class="parsingMode === 'enhanced' ? 'badge-enhanced' : 'badge-simple'"
            >
              {{
                parsingMode === "enhanced"
                  ? hasLocalModel
                    ? "Enhanced (Ready)"
                    : "Enhanced AI"
                  : "Simple"
              }}
            </span>
          </div>
          <p class="settings-card-subtitle">
            {{
              parsingMode === "enhanced"
                ? hasLocalModel
                  ? `${defaultModelName || "Local AI"} ready on device`
                  : "Local AI model setup & download"
                : "Using lightweight basic rules"
            }}
          </p>
        </div>
      </div>

      <div class="settings-card-right">
        <svg
          class="settings-chevron"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
      </div>
    </button>
  </div>
</template>

<style scoped>
.settings-nav-section {
  width: 100%;
  margin-top: 0.25rem;
}

.btn-settings-card {
  width: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 0.85rem;
  padding: 1rem 1.25rem;
  cursor: pointer;
  text-align: left;
  border: 1px solid var(--border-card);
  border-radius: var(--radius-card, 20px);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  box-sizing: border-box;
}

.btn-settings-card:hover {
  border-color: rgba(0, 122, 255, 0.3);
  background: var(--bg-card-elevated, var(--bg-card));
  transform: translateY(-1px);
  box-shadow: var(--shadow-card);
}

.settings-card-left {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  flex: 1;
}

.settings-card-icon-wrap {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.settings-card-icon {
  width: 22px;
  height: 22px;
}

.settings-card-body {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
}

.settings-card-title-line {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.settings-card-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.settings-badge {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  border-radius: 999px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.badge-enhanced {
  background: rgba(88, 86, 214, 0.12);
  color: #5856d6;
}

.badge-simple {
  background: rgba(0, 122, 255, 0.1);
  color: var(--accent-primary);
}

.settings-card-subtitle {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.settings-card-right {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--text-tertiary, #9ca3af);
  padding-left: 0.25rem;
}

.settings-chevron {
  width: 20px;
  height: 20px;
  transition: transform 0.15s ease;
}

.btn-settings-card:hover .settings-chevron {
  transform: translateX(2px);
  color: var(--accent-primary);
}
</style>
