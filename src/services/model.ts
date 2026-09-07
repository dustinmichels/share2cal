import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ModelManifestEntry {
  id: string;
  name: string;
  version: string;
  filename: string;
  repo: string;
  url: string;
  size_bytes: number;
  sha256: string;
  quantization: string;
  description: string;
  recommended_ram: string;
  is_default: boolean;
}

export interface ModelManifest {
  version: string;
  updated_at: string;
  default_model_id: string;
  models: ModelManifestEntry[];
}

export interface ModelStatus {
  id: string;
  name: string;
  filename: string;
  size_bytes: number;
  downloaded_bytes: number;
  is_downloaded: boolean;
  is_downloading: boolean;
  file_path: string | null;
  storage_dir: string;
  sha256: string;
  is_verified: boolean;
  error: string | null;
  quantization: string;
  description: string;
  recommended_ram: string;
  is_default: boolean;
}

export interface DownloadProgressPayload {
  model_id: string;
  received_bytes: number;
  total_bytes: number;
  percentage: number;
  speed_bytes_per_sec: number;
  status: "downloading" | "verifying" | "completed" | "error" | "cancelled";
  error: string | null;
}

export interface ModelsStorageInfo {
  storage_dir: string;
  total_models_downloaded: number;
  total_models_size_bytes: number;
  free_disk_space_bytes: number | null;
}

/**
 * Retrieves the embedded model manifest detailing available models and Hugging Face sources.
 */
export async function getModelManifest(): Promise<ModelManifest> {
  return await invoke<ModelManifest>("get_model_manifest");
}

/**
 * Retrieves the current status, path, and download state for all models.
 */
export async function getModelStatuses(): Promise<ModelStatus[]> {
  try {
    return await invoke<ModelStatus[]>("get_model_statuses");
  } catch (err) {
    console.warn("Failed to retrieve model statuses:", err);
    return [];
  }
}

/**
 * Retrieves the status for a specific model by ID.
 */
export async function getModelStatus(modelId: string): Promise<ModelStatus | null> {
  try {
    return await invoke<ModelStatus>("get_model_status", { modelId });
  } catch (err) {
    console.warn(`Failed to retrieve model status for ${modelId}:`, err);
    return null;
  }
}

/**
 * Starts downloading a model from Hugging Face into the persistent sandbox directory.
 */
export async function downloadModel(modelId: string): Promise<void> {
  return await invoke("download_model", { modelId });
}

/**
 * Cancels an active model download.
 */
export async function cancelModelDownload(modelId: string): Promise<void> {
  return await invoke("cancel_model_download", { modelId });
}

/**
 * Deletes a downloaded model file and any temporary partial files to reclaim storage.
 */
export async function deleteModel(modelId: string): Promise<void> {
  return await invoke("delete_model", { modelId });
}

/**
 * Verifies the integrity of a downloaded model against its expected SHA-256 hash.
 */
export async function verifyModelHash(modelId: string): Promise<boolean> {
  return await invoke<boolean>("verify_model_hash", { modelId });
}

/**
 * Gets storage information (total downloaded models, space used, free space).
 */
export async function getModelsStorageInfo(): Promise<ModelsStorageInfo | null> {
  try {
    return await invoke<ModelsStorageInfo>("get_models_storage_info");
  } catch (err) {
    console.warn("Failed to get models storage info:", err);
    return null;
  }
}

/**
 * Subscribes to model download progress events emitted by the native Rust backend.
 */
export async function onModelDownloadProgress(
  callback: (payload: DownloadProgressPayload) => void
): Promise<UnlistenFn> {
  return await listen<DownloadProgressPayload>("model_download_progress", (event) => {
    callback(event.payload);
  });
}

/**
 * Format bytes into human-readable string (B, KB, MB, GB).
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Format download transfer speed into human-readable string (e.g. 3.2 MB/s).
 */
export function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "0 B/s";
  if (bytesPerSec < 1024) return `${Math.round(bytesPerSec)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}
