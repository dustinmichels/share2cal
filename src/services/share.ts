import { invoke } from "@tauri-apps/api/core";

export interface SharedImagePayload {
  file_name: string;
  file_path: string;
  mime_type: string;
  size_bytes: number;
  timestamp: number;
  source: string;
  bytes?: number[];
}

/**
 * Checks if there is any pending shared image waiting from the iOS Share Extension
 * or system share mechanism.
 */
export async function getPendingSharedImage(includeBytes = true): Promise<SharedImagePayload | null> {
  try {
    return await invoke<SharedImagePayload | null>("get_pending_shared_image", {
      includeBytes,
    });
  } catch (err) {
    console.warn("Failed to check pending shared image from native bridge:", err);
    return null;
  }
}

/**
 * Clears pending shared images in the shared container once processed.
 */
export async function clearPendingSharedImage(): Promise<void> {
  try {
    await invoke("clear_pending_shared_image");
  } catch (err) {
    console.warn("Failed to clear pending shared image:", err);
  }
}

/**
 * Stages an image for simulation, dev testing, or local cross-platform share flow.
 */
export async function stageSharedImage(
  bytes: Uint8Array | number[],
  fileName: string,
  mimeType?: string
): Promise<SharedImagePayload> {
  const payloadBytes = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  return await invoke<SharedImagePayload>("stage_shared_image", {
    bytes: payloadBytes,
    fileName,
    mimeType: mimeType || "image/png",
  });
}

/**
 * Converts a SharedImagePayload with bytes into a standard browser File object.
 */
export function payloadToFile(payload: SharedImagePayload): File | null {
  if (!payload.bytes || payload.bytes.length === 0) {
    return null;
  }

  const u8Array = new Uint8Array(payload.bytes);
  const blob = new Blob([u8Array], { type: payload.mime_type || "image/png" });
  return new File([blob], payload.file_name, {
    type: payload.mime_type || "image/png",
    lastModified: payload.timestamp ? payload.timestamp * 1000 : Date.now(),
  });
}
