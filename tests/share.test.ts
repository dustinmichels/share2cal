import { describe, it, expect } from "bun:test";
import { payloadToFile, type SharedImagePayload } from "../src/services/share";

describe("Share Service", () => {
  it("converts a SharedImagePayload to a browser File object", () => {
    const sampleBytes = [137, 80, 78, 71, 13, 10, 26, 10]; // PNG header
    const payload: SharedImagePayload = {
      file_name: "test_flyer.png",
      file_path: "/tmp/group.com.dustinmichels.share2cal/shared_images/test_flyer.png",
      mime_type: "image/png",
      size_bytes: sampleBytes.length,
      timestamp: 1693999999,
      source: "ios_share_extension",
      bytes: sampleBytes,
    };

    const file = payloadToFile(payload);
    expect(file).not.toBeNull();
    expect(file!.name).toBe("test_flyer.png");
    expect(file!.type).toBe("image/png");
    expect(file!.size).toBe(sampleBytes.length);
  });

  it("returns null if payload has no bytes", () => {
    const payload: SharedImagePayload = {
      file_name: "test_flyer.png",
      file_path: "/tmp/group.com.dustinmichels.share2cal/shared_images/test_flyer.png",
      mime_type: "image/png",
      size_bytes: 0,
      timestamp: 1693999999,
      source: "ios_share_extension",
    };

    const file = payloadToFile(payload);
    expect(file).toBeNull();
  });

  it("handles different mime types and timestamps properly in payloadToFile", () => {
    const sampleBytes = [255, 216, 255]; // JPEG header
    const payload: SharedImagePayload = {
      file_name: "poster.jpeg",
      file_path: "/Users/test/Downloads/poster.jpeg",
      mime_type: "image/jpeg",
      size_bytes: sampleBytes.length,
      timestamp: 1700000000,
      source: "drag_and_drop",
      bytes: sampleBytes,
    };

    const file = payloadToFile(payload);
    expect(file).not.toBeNull();
    expect(file!.name).toBe("poster.jpeg");
    expect(file!.type).toBe("image/jpeg");
    expect(file!.lastModified).toBe(1700000000 * 1000);
  });
});
