import { invoke } from "@tauri-apps/api/core";

export interface BoundingBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OcrLine {
  text: string;
  confidence: number;
  bounding_box?: BoundingBox;
}

export interface OcrResult {
  text: string;
  lines: OcrLine[];
}

export async function extractTextFromImage(path: string): Promise<OcrResult> {
  return await invoke<OcrResult>("extract_text_from_image", { path });
}

export async function extractTextFromBytes(bytes: Uint8Array | number[]): Promise<OcrResult> {
  const payload = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  return await invoke<OcrResult>("extract_text_from_image_bytes", { bytes: payload });
}
