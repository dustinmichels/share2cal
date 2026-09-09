import { describe, it, expect, beforeEach } from "bun:test";
import {
  formatBytes,
  formatSpeed,
  isDesktopDevice,
  isMobileDevice,
  checkDiskSpaceAndAutoDownloadDefaultModel,
} from "../src/services/model";
import {
  setStoredParsingMode,
  getStoredParsingMode,
  clearStoredSettings,
} from "../src/services/settings";
import manifest from "../model-manifest.json";

describe("Model Service & Manifest", () => {
  beforeEach(() => {
    clearStoredSettings();
  });
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
    expect(typeof isMobileDevice()).toBe("boolean");
    expect(isMobileDevice()).toBe(!isDesktopDevice());

    const originalUserAgent = navigator.userAgent;
    try {
      // Test mobile user agents
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15",
        configurable: true,
      });
      expect(isMobileDevice()).toBe(true);
      expect(isDesktopDevice()).toBe(false);

      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36",
        configurable: true,
      });
      expect(isMobileDevice()).toBe(true);
      expect(isDesktopDevice()).toBe(false);

      // Test desktop user agent
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        configurable: true,
      });
      expect(isMobileDevice()).toBe(false);
      expect(isDesktopDevice()).toBe(true);
    } finally {
      Object.defineProperty(navigator, "userAgent", {
        value: originalUserAgent,
        configurable: true,
      });
    }
  });

  it("ensures default mode is enhanced and auto-download targets default model", async () => {
    // 2. Default mode is enhanced when not set
    expect(getStoredParsingMode()).toBe("enhanced");

    // 1. Install default tiny LLM right away (auto-download)
    const res = await checkDiskSpaceAndAutoDownloadDefaultModel();
    expect(res.modelId).toBe("smollm2-360m-instruct-q4_k_m");
    // 3. Mode remains enhanced even if space is insufficient or error occurs
    expect(res.mode).toBe("enhanced");
    expect(getStoredParsingMode()).toBe("enhanced");
  });

  it("returns disabled reason when stored mode is 'simple'", async () => {
    setStoredParsingMode("simple");
    const res = await checkDiskSpaceAndAutoDownloadDefaultModel();
    expect(res.triggered).toBe(false);
    expect(res.mode).toBe("simple");
    expect(res.reason).toBe("disabled");
  });

  it("checks disk space and attempts download when stored mode is 'enhanced'", async () => {
    setStoredParsingMode("enhanced");
    const res = await checkDiskSpaceAndAutoDownloadDefaultModel();
    expect(res.modelId).toBe("smollm2-360m-instruct-q4_k_m");
    expect([
      "already_ready",
      "already_downloading",
      "download_started",
      "insufficient_space",
      "error",
    ]).toContain(res.reason!);
    expect(res.mode).toBe("enhanced");
    expect(getStoredParsingMode()).toBe("enhanced");
  });
});
