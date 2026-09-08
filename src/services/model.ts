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
const DEFAULT_FALLBACK_MODELS: ModelStatus[] = [
  {
    id: "smollm2-360m-instruct-q4_k_m",
    name: "SmolLM2 360M Instruct",
    filename: "SmolLM2-360M-Instruct-Q4_K_M.gguf",
    size_bytes: 270590880,
    downloaded_bytes: 0,
    is_downloaded: false,
    is_downloading: false,
    file_path: null,
    storage_dir: "/data/user/0/com.share2cal.app/files/models",
    sha256: "2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2",
    is_verified: false,
    error: null,
    quantization: "Q4_K_M",
    description:
      "Recommended. Balanced 360M model offering fast inference, low RAM consumption, and reliable structured event extraction.",
    recommended_ram: "< 300 MB",
    is_default: true,
  },
  {
    id: "smollm2-135m-instruct-q4_k_m",
    name: "SmolLM2 135M Instruct",
    filename: "SmolLM2-135M-Instruct-Q4_K_M.gguf",
    size_bytes: 105454432,
    downloaded_bytes: 0,
    is_downloaded: false,
    is_downloading: false,
    file_path: null,
    storage_dir: "/data/user/0/com.share2cal.app/files/models",
    sha256: "2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d",
    is_verified: false,
    error: null,
    quantization: "Q4_K_M",
    description:
      "Ultra-lightweight 135M model with minimal storage footprint. Recommended for older devices or tight storage.",
    recommended_ram: "< 150 MB",
    is_default: false,
  },
  {
    id: "qwen2.5-0.5b-instruct-q4_k_m",
    name: "Qwen2.5 0.5B Instruct",
    filename: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
    size_bytes: 491400032,
    downloaded_bytes: 0,
    is_downloaded: false,
    is_downloading: false,
    file_path: null,
    storage_dir: "/data/user/0/com.share2cal.app/files/models",
    sha256: "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db",
    is_verified: false,
    error: null,
    quantization: "Q4_K_M",
    description:
      "High-accuracy 0.5B model with strong multilingual comprehension and complex flyer layout parsing.",
    recommended_ram: "< 550 MB",
    is_default: false,
  },
];

const browserMockModels = [...DEFAULT_FALLBACK_MODELS];

export async function getModelStatuses(): Promise<ModelStatus[]> {
  try {
    const res = await invoke<ModelStatus[]>("get_model_statuses");
    if (res && res.length > 0) return res;
    return browserMockModels;
  } catch (err) {
    console.warn("Using fallback manifest models:", err);
    return browserMockModels;
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
 * Unloads the currently active LLM inference model from memory / Metal GPU.
 */
export async function unloadInferenceModel(): Promise<void> {
  return await invoke("unload_inference_model");
}

/**
 * Checks whether a specific inference model is currently loaded in memory.
 */
export async function isInferenceModelLoaded(modelId: string): Promise<boolean> {
  return await invoke<boolean>("is_inference_model_loaded", { modelId });
}

/**
 * Gets storage information (total downloaded models, space used, free space).
 */
export async function getModelsStorageInfo(): Promise<ModelsStorageInfo | null> {
  try {
    return await invoke<ModelsStorageInfo>("get_models_storage_info");
  } catch (err) {
    console.warn("Using fallback models storage info:", err);
    return {
      storage_dir: "/var/mobile/Containers/Data/Application/Share2Cal/models",
      total_models_downloaded: browserMockModels.filter((m) => m.is_downloaded).length,
      total_models_size_bytes: browserMockModels
        .filter((m) => m.is_downloaded)
        .reduce((sum, m) => sum + m.size_bytes, 0),
      free_disk_space_bytes: 18450000000,
    };
  }
}

/**
 * Opens the models storage directory in the native file manager (macOS Finder, Windows Explorer, Linux).
 */
export async function openModelsDirectory(): Promise<void> {
  return await invoke("open_models_directory");
}

/**
 * Detects if the current running environment is a desktop platform (macOS, Windows, Linux) vs mobile.
 */
export function isDesktopDevice(): boolean {
  if (typeof navigator === "undefined") return true;
  return !/iPhone|iPad|iPod|Android/i.test(navigator.userAgent || "");
}

/**
 * Subscribes to model download progress events emitted by the native Rust backend.
 */
export async function onModelDownloadProgress(
  callback: (payload: DownloadProgressPayload) => void,
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
