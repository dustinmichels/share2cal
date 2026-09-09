import { isTauri, invoke } from "@tauri-apps/api/core";

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
  qr_codes?: string[];
}

export async function extractTextFromImage(path: string): Promise<OcrResult> {
  try {
    return await invoke<OcrResult>("extract_text_from_image", { path });
  } catch (err) {
    if (!isTauri()) {
      return {
        text: "Weekly Pottery Class\nTuesdays & Thursdays 6:00 PM - 8:00 PM\nCommunity Arts Center",
        lines: [
          { text: "Weekly Pottery Class", confidence: 0.95 },
          { text: "Tuesdays & Thursdays 6:00 PM - 8:00 PM", confidence: 0.92 },
          { text: "Community Arts Center", confidence: 0.9 },
        ],
      };
    }
    throw err;
  }
}

export async function extractTextFromBytes(bytes: Uint8Array | number[]): Promise<OcrResult> {
  const payload = bytes instanceof Uint8Array ? Array.from(bytes) : bytes;
  try {
    return await invoke<OcrResult>("extract_text_from_image_bytes", { bytes: payload });
  } catch (err) {
    if (!isTauri()) {
      return {
        text: "Weekly Pottery Class\nTuesdays & Thursdays 6:00 PM - 8:00 PM\nCommunity Arts Center",
        lines: [
          { text: "Weekly Pottery Class", confidence: 0.95 },
          { text: "Tuesdays & Thursdays 6:00 PM - 8:00 PM", confidence: 0.92 },
          { text: "Community Arts Center", confidence: 0.9 },
        ],
      };
    }
    throw err;
  }
}
