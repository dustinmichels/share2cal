# Plan

## Phase 1: Model Selection & Prompt Engineering

Build a local test harness using 50 real-world screenshots (flyers, chat messages, ticket receipts, invitations).Implement strict GBNF grammar or JSON schema decoding to guarantee valid JSON keys: title, start_time, end_time, location, description.Test dynamic context injection:PlaintextCurrent Reference Time: 2026-09-06T10:57:00-04:00 (Sunday, EDT)

Task: Extract the event from the text below into ISO-8601 timestamps relative to the reference time.

## Phase 2: Local OCR & Inference Engine Setup

iOS: Implement VNRecognizeTextRequest with .accurate recognition level. Link llama.cpp using Swift Package Manager with Metal GPU support enabled.Android: Configure ML Kit Text Recognition via Play Services. Integrate llama.cpp using CMake and Android NDK targeting OpenCL/Vulkan backends.Chain OCR output directly into the inference thread. Run inference on background threads with high QoS to avoid blocking the main UI.

## Phase 3: Model Download & Storage Architecture

Host the quantized GGUF weights on an S3 bucket fronted by Cloudflare R2 / CloudFront.

Implement a robust download manager supporting HTTP range requests (pause/resume on network loss).Verify SHA-256 checksums post-download before moving the model from temporary cache to the app's persistent sandbox (Application Support on iOS, no_backup on Android).Add a device memory check before starting: abort and prompt if free storage is $< 2.5\times$ the model size.

## Phase 4: Share Sheet & Calendar Integration

Register an iOS Share Extension and Android Intent Filter (ACTION_SEND with image/\*) so users can share screenshots directly from their photo gallery or messaging apps without opening the app first.

Implement calendar authorization requests:iOS: Request EKEntityType.event full access permissions via Info.plist.Android: Request READ_CALENDAR and WRITE_CALENDAR permissions.Build an editable review modal showing parsed parameters (editable fields + preview button) before committing the event to the system calendar.

## Phase 5: Profiling & Fallbacks

Memory Constraints: Set the LLM context window strictly to 1024 or 2048 tokens to minimize the KV cache footprint.Deterministic Fallback: If the LLM generation fails or times out (>5 seconds), fall back to regex/heuristic date parsing (e.g., Apple's NSDataDetector or Android's text classifier) to populate fields gracefully rather than crashing.
