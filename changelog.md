# Changelog

## [Unreleased] - 2026-09-07

### Added

- **Hugging Face Model Direct Distribution (`model-manifest.json`)**: Configured direct LFS resolve URLs, exact file sizes, and SHA-256 hashes for `SmolLM2-360M-Instruct` (default, Q4_K_M, ~270 MB), `SmolLM2-135M-Instruct` (Q4_K_M, ~105 MB), and `Qwen2.5-0.5B-Instruct` (Q4_K_M, ~491 MB).
- **On-Device Model Manager (`src-tauri/src/model.rs`)**:
  - Sandboxed persistent storage resolution in app data directory (`models/`).
  - Resumable streaming HTTP downloader with HTTP `Range` header support and cancellation flags.
  - Pre-flight storage checks (`statvfs`) ensuring $\ge 1.5\times$ free disk space before download.
  - Streaming progress events (`model_download_progress`) with received bytes, total bytes, percentage, transfer speed, and status.
  - Post-download SHA-256 verification and atomic file relocation from `.part` staging.
  - Model deletion and integrity verification methods.
- **Tauri IPC Commands (`src-tauri/src/lib.rs`)**: Registered `get_model_manifest`, `get_model_statuses`, `get_model_status`, `download_model`, `cancel_model_download`, `delete_model`, `verify_model_hash`, and `get_models_storage_info`.
- **Model Client Service (`src/services/model.ts`)**: Typed TypeScript IPC wrapper and event listener for model operations and progress streams.
- **Settings View (`src/components/SettingsView.vue`)**:
  - Storage overview displaying space used by models, free disk space, and exact local storage path with copy action.
  - Per-model status cards indicating `Ready on Device`, `Downloading`, or `Not Downloaded`.
  - Active download progress bar with live transfer rate and cancel button.
  - On-demand `Verify Integrity` and `Delete from Device` actions.
- **Header & Empty State Integrations (`src/App.vue`)**: Added header model readiness status pill and empty-state on-device model card linking to settings modal.

### Changed

- **`todo.md`**: Updated Section 1 architecture to use direct Hugging Face endpoints instead of custom Cloudflare R2 / S3 infrastructure.
