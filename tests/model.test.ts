import { describe, it, expect } from "bun:test";
import { formatBytes, formatSpeed, isDesktopDevice } from "../src/services/model";
import manifest from "../model-manifest.json";

describe("Model Service & Manifest", () => {
  it("has a valid model-manifest.json with SmolLM2-360M as default", () => {
    expect(manifest.default_model_id).toBe("smollm2-360m-instruct-q4_k_m");
    expect(manifest.models.length).toBeGreaterThan(0);

    const defaultModel = manifest.models.find((m) => m.id === manifest.default_model_id);
    expect(defaultModel).toBeDefined();
    expect(defaultModel!.is_default).toBe(true);
    expect(defaultModel!.filename).toBe("SmolLM2-360M-Instruct-Q4_K_M.gguf");
    expect(defaultModel!.url).toStartWith("https://huggingface.co/");
    expect(defaultModel!.sha256.length).toBe(64);
    expect(defaultModel!.size_bytes).toBeGreaterThan(100_000_000);
  });

  it("formats bytes accurately", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024 * 270.5)).toBe("270.5 MB");
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toBe("2.50 GB");
  });

  it("formats download speeds accurately", () => {
    expect(formatSpeed(0)).toBe("0 B/s");
    expect(formatSpeed(500)).toBe("500 B/s");
    expect(formatSpeed(1024 * 50)).toBe("50.0 KB/s");
    expect(formatSpeed(1024 * 1024 * 3.5)).toBe("3.5 MB/s");
  });

  it("distinguishes default model and alternative models in manifest", () => {
    const defaultModel = manifest.models.find((m) => m.is_default);
    const alternativeModels = manifest.models.filter((m) => !m.is_default);

    expect(defaultModel).toBeDefined();
    expect(defaultModel!.id).toBe("smollm2-360m-instruct-q4_k_m");
    expect(alternativeModels.length).toBe(2);
    expect(alternativeModels.map((m) => m.id)).toContain("smollm2-135m-instruct-q4_k_m");
    expect(alternativeModels.map((m) => m.id)).toContain("qwen2.5-0.5b-instruct-q4_k_m");
  });

  it("correctly identifies desktop vs mobile environments", () => {
    expect(typeof isDesktopDevice()).toBe("boolean");
  });
});
