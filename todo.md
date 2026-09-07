# On-Device Model Integration Task List

## 1. Model Selection & Weight Hosting

- [x] **Benchmark & Select Target SLM**: Selected lightweight quantized GGUF models tailored for mobile memory budgets:
  - Primary default: `SmolLM2-360M-Instruct` (Q4_K_M, ~270 MB, ~280 MB RAM footprint)
  - Ultra-lightweight option: `SmolLM2-135M-Instruct` (Q4_K_M, ~105 MB, ~150 MB RAM footprint)
  - High-accuracy option: `Qwen2.5-0.5B-Instruct` (Q4_K_M, ~491 MB, ~550 MB RAM footprint)
- [x] **Direct Weight Distribution via Hugging Face Hub LFS**:
  - Direct downloads via Hugging Face Hub LFS resolve endpoints (HTTP Range supported for resume/pause; no custom Cloudflare R2 / S3 infra needed).
  - Created `model-manifest.json` with model IDs, verified SHA-256 checksums, byte counts, and direct Hugging Face URLs.

---

## 2. On-Device Storage & Download Manager

- [ ] **Device Storage Pre-flight Check**:
  - Implement native storage check via Rust/Objective-C to ensure free disk space $\ge 2.5\times$ the model size before starting download.
- [ ] **Resumable HTTP Download Manager**:
  - Implement chunked download manager with HTTP Range header support for pause/resume upon connection drops.
  - Stream progress events (`download_progress: { received_bytes, total_bytes, percentage }`) over Tauri IPC to Vue UI.
- [ ] **Integrity Verification & Persistent Sandboxing**:
  - Compute and verify SHA-256 checksum upon completion.
  - Move validated model file into persistent sandbox directory (`Library/Application Support/models/` on iOS, `no_backup/models/` on Android).
  - Implement cache invalidation and clean deletion methods.

---

## 3. Local Inference Engine Integration (`llama.cpp`)

- [ ] **Link `llama.cpp` Engine**:
  - Integrate `llama.cpp` via Rust FFI (`llama-cpp-2` crate or custom C++ FFI bridge).
  - Enable Apple Metal GPU backend (`GGML_METAL=ON`) in iOS build pipeline (`project.yml` / `build_rust.sh`).
- [ ] **Configure Memory & Resource Constraints**:
  - Set context window strictly to 1024 or 2048 tokens to minimize KV cache footprint.
  - Cap thread count according to device efficiency/performance cores.
- [ ] **Constrained Decoding with GBNF**:
  - Hook `get_gbnf_grammar()` (`src-tauri/src/parser.rs`) into `llama.cpp` sampler to guarantee deterministic JSON output conforming to `EventDetails`.
- [ ] **Async Inference Pipeline**:
  - Execute inference on dedicated background worker thread / Tokio blocking task pool with high QoS so the Tauri main thread / Webview UI never hitches.

---

## 4. Orchestration, Timeout & Fallback Routing

- [ ] **Extraction Orchestrator**:
  - Format input using `generate_extraction_prompt()` with reference time and timezone offsets.
  - Execute LLM inference and deserialize structured JSON response.
- [ ] **5-Second Timeout & Error Guard**:
  - Add a 5.0-second execution deadline for model inference.
- [ ] **Seamless Fallback Routing**:
  - If model is not downloaded, inference errors out, or timeout triggers, automatically route OCR text to `parse_event_deterministic()`.
  - Annotate `EventDetails.source` as `"llm"` or `"deterministic_fallback"` with corresponding confidence metrics.

---

## 5. UI & Model Management in Vue/TypeScript

- [ ] **Download Management Sheet / Modal**:
  - First-run prompt asking user to download the lightweight model for enhanced extraction.
  - Progress bar with download speed, percentage, pause/resume, and retry controls.
- [ ] **Model Settings Panel**:
  - Display current model status (Not Downloaded, Downloading, Ready, Storage Used).
  - Add button to delete downloaded model to reclaim storage.
- [ ] **Source Badge in Review Form**:
  - Show subtle indicator in the event review form indicating whether extraction was powered by LLM or deterministic rules.

---

## 6. Testing, Verification & Profiling

- [ ] **End-to-End Extraction Tests**:
  - Unit tests verifying OCR text $\rightarrow$ GBNF sampling $\rightarrow$ valid `EventDetails` output.
- [ ] **Memory & Latency Profiling on iOS**:
  - Profile peak memory usage (RAM ceiling $< 150\text{ MB}$ for $360\text{M}$ model) using Xcode Instruments (Allocations / Metal System Trace).
  - Verify zero UI stutters on iPhone during simultaneous OCR and LLM token generation.
