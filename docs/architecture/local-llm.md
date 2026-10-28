# On-device LLM

Enhanced mode re-reads the OCR text with a small quantised language model running through llama.cpp. The model is optional: it is downloaded on demand, it has a hard time limit, and any failure falls back to the [rule-based parser](/architecture/parsing).

## Models

Models are listed in `src-tauri/model-manifest.json`, which is compiled into the binary with `include_str!`. All three are Q4_K_M GGUF files downloaded directly from Hugging Face.

| Model                               | Size   | Notes                                    |
| ----------------------------------- | ------ | ---------------------------------------- |
| **SmolLM2 360M Instruct** (default) | 271 MB | Balanced speed and accuracy              |
| SmolLM2 135M Instruct               | 105 MB | For older devices or tight storage       |
| Qwen2.5 0.5B Instruct               | 491 MB | Most accurate, best with complex layouts |

Each entry carries its download URL, exact byte size and SHA-256. To add a model, append an entry with those fields, set `default_model_id` if it should be the default, and rebuild. The prompt uses ChatML tags (`<|im_start|>`, `<|im_end|>`), which both families understand.

## Downloads

`download_model` runs `start_model_download` in `model.rs`:

1. **Check space.** `statvfs` on the storage directory must report at least 1.5× the model size free, or the command fails with `InsufficientDiskSpace`.
2. **Resume if possible.** Data streams into `<filename>.part`. If a partial file exists, the request sends `Range: bytes=<size>-` and appends.
3. **Report progress.** `model_download_progress` events carry `received_bytes`, `total_bytes`, `percentage`, `speed_bytes_per_sec` and a `status` of `downloading`, `verifying`, `completed`, `error` or `cancelled`.
4. **Verify.** The finished `.part` is hashed with SHA-256. A mismatch deletes it and fails with `ChecksumMismatch`; a match renames it to the final filename.

`cancel_model_download` sets the model's `AtomicBool` in `DownloadState`. The streaming loop checks it before every chunk and returns, leaving the `.part` file so the next attempt resumes.

Models are stored in `<app data dir>/models/`, or in `$SHARE2CAL_MODELS_DIR` when set. On macOS that is `~/Library/Application Support/com.dustinmichels.share2cal/models/`. `open_models_directory` reveals it in Finder or Explorer on desktop and returns an error on mobile.

### When downloads start

- **Automatically** on launch, if the parsing mode is Enhanced, the default model is missing and there is enough space (`checkDiskSpaceAndAutoDownloadDefaultModel` in `model.ts`).
- **From Settings,** where each model can be downloaded, cancelled, verified or deleted.
- **From Try again** on the summary view, if the default model is missing.

## Loading

`InferenceEngineManager` keeps at most one model in memory:

- `get_or_load_model` returns the cached `Arc<LlamaModel>` when the id and path match, otherwise loads the file on a blocking thread and replaces the cache.
- On macOS and iOS, `n_gpu_layers` is set to 99, so every layer runs on Metal.
- `unload_inference_model` drops the cache; `is_inference_model_loaded` reports whether a given id is resident.

The first enhanced parse after launch pays the load cost. Loading happens before the inference timer starts, so a slow load does not count against the timeout.

## The prompt

`generate_extraction_prompt` in `parser/schema.rs` builds a ChatML prompt with the current reference time and weekday, so the model can resolve "this Friday":

```text
<|im_start|>system
You are a calendar assistant. Extract all events from the OCR text into a JSON
object with an "events" array.
Each event must have fields: "title" (string), "start_time" (ISO-8601 or
YYYY-MM-DD or null), "end_time" (…), "is_all_day" (boolean), "location" (string
or null), "description" (string or null), "recurrence_rule" (string or null,
e.g. FREQ=WEEKLY;BYDAY=MO,WE), "url" (string or null, …). Output ONLY raw JSON.
Reference Time: 2026-10-27T10:57:00-04:00 (Tuesday)<|im_end|>
<|im_start|>user
…OCR text…

Links / QR Codes:
https://…
<|im_end|>
<|im_start|>assistant
{"events": [
```

Two details matter:

- **The answer is pre-filled.** The prompt ends with `{"events": [`, which makes continuing the JSON array the most likely output. Nothing forces it: with greedy sampling and no grammar, the model can still emit non-JSON, which is why the repair steps below and the rule-based fallback exist.
- **The text is trimmed to fit.** The context window is 2,048 tokens with 512 reserved for output, leaving `MAX_PROMPT_TOKENS` = 1,536. OCR text beyond that budget is cut, but the `Links / QR Codes:` block is always kept.

To see the exact prompt for a piece of text, call `generate_event_prompt` (`generateEventPrompt` in `event.ts`).

## Generation

`run_inference_sync` in `inference.rs`:

| Setting    | Value                                                        |
| ---------- | ------------------------------------------------------------ |
| Context    | 2,048 tokens (`DEFAULT_CONTEXT_WINDOW`)                      |
| Batch      | 512                                                          |
| Threads    | Available cores, clamped to 1–4                              |
| Sampler    | Greedy                                                       |
| Output cap | `max_tokens` (1,536) bounded by the room left in the context |
| Deadline   | 5 s by default (`timeout_secs` overrides)                    |

Prompt tokens are decoded in batches of 512, then tokens are generated one at a time. After each token the accumulated text, with the pre-filled `{"events": [` restored, is tried as JSON; generation stops as soon as it parses. It also stops at an end-of-generation token or when the context is full.

The deadline is enforced twice: inside the loop, which returns `InferenceTimeout` when time is up, and by `tokio::time::timeout` around the blocking task in `run_inference_async`.

::: info GBNF grammar
`parser/schema.rs` also defines a GBNF grammar for the eight-field event shape, exposed through `get_event_gbnf_grammar`. A unit test confirms it compiles into a grammar sampler, but production generation uses the greedy sampler with the pre-filled prefix; the grammar is not in the sampling chain.
:::

## Reading the output

Small models produce almost-JSON. `parse_llm_json_payload` tries progressively looser readings and stops at the first that yields events:

1. Parse as `{"events": [...]}`, a bare array, or a single event object.
2. Wrap or close the text in the likely missing brackets and retry.
3. Cut back to the last `}` and close the array.
4. Pull fields out with regular expressions (`extract_events_from_loose_llm_json`) for output with unescaped quotes.

Every event read this way gets `source: "llm"` and `confidence: 0.95`. If nothing parses, the rule-based events are returned with `source: "deterministic_fallback"`.

## Tuning checklist

| To change                | Edit                                                                             |
| ------------------------ | -------------------------------------------------------------------------------- |
| Timeout                  | `DEFAULT_INFERENCE_TIMEOUT_SECS`, or pass `timeoutSecs` to `parseEventsFromText` |
| Context or output budget | `DEFAULT_CONTEXT_WINDOW`, `MAX_OUTPUT_TOKENS` in `inference.rs`                  |
| Prompt wording           | `generate_extraction_prompt` in `parser/schema.rs`                               |
| Default model            | `default_model_id` in `src-tauri/model-manifest.json`                            |
| Thread cap               | `get_optimal_thread_count`                                                       |

`tests/samples_validation.rs` has an ignored test that runs the real model against every sample. Point `SHARE2CAL_MODELS_DIR` at a folder containing `SmolLM2-360M-Instruct-Q4_K_M.gguf` and run it with `cargo test -- --ignored`.
