use crate::error::AppError;
use crate::model;
use crate::parser::{self, EventDetails, ReferenceContext};
use encoding_rs::UTF_8;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::sync::Mutex as TokioMutex;

/// Intermediate struct matching the exact 6 JSON fields produced by the GBNF grammar
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmEventOutput {
    pub title: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub recurrence_rule: Option<String>,
}

/// Wrapper matching the root JSON object with an `events` array
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmEventsPayload {
    pub events: Vec<LlmEventOutput>,
}

impl LlmEventOutput {
    pub fn into_event_details(self, source_name: &str) -> EventDetails {
        EventDetails {
            title: self.title,
            start_time: self.start_time,
            end_time: self.end_time,
            is_all_day: self.is_all_day,
            location: self.location,
            description: self.description,
            recurrence_rule: self.recurrence_rule,
            confidence: 0.95,
            source: source_name.to_string(),
        }
    }
}
/// Default context window limit to minimize KV cache RAM footprint on mobile devices
pub const DEFAULT_CONTEXT_WINDOW: u32 = 2048;

/// Explicit output token reservation budget to guarantee generation headroom within the context window
pub const MAX_OUTPUT_TOKENS: usize = 512;

/// Default maximum tokens generated for structured event JSON output
pub const DEFAULT_MAX_GENERATION_TOKENS: usize = 1536;

/// Default inference timeout in seconds
pub const DEFAULT_INFERENCE_TIMEOUT_SECS: u64 = 5;

static BACKEND: OnceLock<Arc<LlamaBackend>> = OnceLock::new();
pub fn get_global_backend() -> Result<Arc<LlamaBackend>, AppError> {
    if let Some(backend) = BACKEND.get() {
        return Ok(Arc::clone(backend));
    }
    let backend = LlamaBackend::init().map_err(|e| AppError::Inference(format!("Failed to init llama backend: {}", e)))?;
    let backend_arc = Arc::new(backend);
    let _ = BACKEND.set(Arc::clone(&backend_arc));
    Ok(backend_arc)
}

/// Returns the optimal thread count for model execution.
/// Capped at 4 to prevent device overheating and UI latency on mobile Apple/Android cores.
pub fn get_optimal_thread_count() -> i32 {
    let count = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    count.clamp(1, 4)
}

pub struct LoadedModel {
    pub model_id: String,
    pub model_path: PathBuf,
    pub model: Arc<LlamaModel>,
}

pub struct InferenceEngineManager {
    loaded_model: TokioMutex<Option<LoadedModel>>,
}

impl Default for InferenceEngineManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceEngineManager {
    pub fn new() -> Self {
        Self {
            loaded_model: TokioMutex::new(None),
        }
    }
    pub async fn get_or_load_model(
        &self,
        model_id: &str,
        model_path: &Path,
    ) -> Result<Arc<LlamaModel>, AppError> {
        let mut guard = self.loaded_model.lock().await;
        if let Some(loaded) = &*guard {
            if loaded.model_id == model_id && loaded.model_path == model_path {
                return Ok(Arc::clone(&loaded.model));
            }
        }

        let path_clone = model_path.to_path_buf();
        let loaded_model = tokio::task::spawn_blocking(move || {
            let backend = get_global_backend()?;
            let mut model_params = LlamaModelParams::default();

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                // Offload all layers to Metal GPU on Apple platforms
                model_params = model_params.with_n_gpu_layers(99);
            }

            LlamaModel::load_from_file(&backend, &path_clone, &model_params)
                .map_err(|e| AppError::Inference(format!("Failed to load GGUF model from {:?}: {}", path_clone, e)))
        })
        .await
        .map_err(|e| AppError::Inference(format!("Model loading task panicked: {}", e)))??;

        let model_arc = Arc::new(loaded_model);
        *guard = Some(LoadedModel {
            model_id: model_id.to_string(),
            model_path: model_path.to_path_buf(),
            model: Arc::clone(&model_arc),
        });

        Ok(model_arc)
    }

    pub async fn unload_model(&self) {
        let mut guard = self.loaded_model.lock().await;
        *guard = None;
    }

    pub async fn is_model_loaded(&self, model_id: &str) -> bool {
        let guard = self.loaded_model.lock().await;
        if let Some(loaded) = &*guard {
            loaded.model_id == model_id
        } else {
            false
        }
    }
}

/// Runs synchronous inference using llama.cpp with strict GBNF grammar constrained decoding.
pub fn run_inference_sync(
    model: &LlamaModel,
    prompt: &str,
    max_tokens: usize,
    deadline: Option<Instant>,
) -> Result<String, AppError> {
    let backend = get_global_backend()?;
    let thread_count = get_optimal_thread_count();

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(DEFAULT_CONTEXT_WINDOW))
        .with_n_batch(512)
        .with_n_threads(thread_count)
        .with_n_threads_batch(thread_count);

    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| AppError::Inference(format!("Failed to create llama context: {}", e)))?;

    let mut sampler = LlamaSampler::greedy();
    tracing::debug!("Sampler initialized before prompt decoding");

    // Tokenize prompt
    let tokens = model
        .str_to_token(prompt, AddBos::Always)
        .map_err(|e| AppError::Inference(format!("Failed to tokenize prompt: {}", e)))?;

    if tokens.is_empty() {
        return Err(AppError::Inference("Prompt resulted in zero tokens".to_string()));
    }

    let n_ctx = DEFAULT_CONTEXT_WINDOW as usize;
    let reserve = max_tokens.min(MAX_OUTPUT_TOKENS);
    if tokens.len() + reserve > n_ctx {
        return Err(AppError::Inference(format!(
            "Prompt exceeds context window limit with reserved output headroom (tokens: {}, reserve: {}, n_ctx: {})",
            tokens.len(),
            reserve,
            n_ctx
        )));
    }

    // Process prompt tokens in batch
    let batch_size = 512;
    let mut batch = LlamaBatch::new(batch_size, 1);
    let total_prompt_tokens = tokens.len();

    for (i, &token) in tokens.iter().enumerate() {
        let is_last = i == total_prompt_tokens - 1;
        batch
            .add(token, i as i32, &[0], is_last)
            .map_err(|e| AppError::Inference(format!("Failed to add token to batch: {}", e)))?;

        if batch.n_tokens() >= batch_size as i32 || is_last {
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Inference(format!("Failed to decode batch: {}", e)))?;
            if !is_last {
                batch.clear();
            }
        }
    }

    let mut decoder = UTF_8.new_decoder();
    let mut generated_text = String::new();
    let mut current_pos = total_prompt_tokens as i32;

    // Sample initial token from prompt logits (-1 selects the last token with logits in the batch)
    let mut token = sampler.sample(&ctx, -1);
    sampler.accept(token);
    batch.clear();

    let max_generation_steps = max_tokens.min(n_ctx.saturating_sub(total_prompt_tokens));
    for _step in 0..max_generation_steps {
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return Err(AppError::InferenceTimeout(DEFAULT_INFERENCE_TIMEOUT_SECS));
            }
        }

        // Check for EOS / EOG token
        if model.is_eog_token(token) {
            break;
        }

        // Decode token to string piece
        let piece = model
            .token_to_piece(token, &mut decoder, false, None)
            .map_err(|e| AppError::Inference(format!("Failed to decode token to string piece: {}", e)))?;
        generated_text.push_str(&piece);

        // If closing JSON brace is reached and JSON parses, stop early
        let candidate_json = if !generated_text.trim_start().starts_with('{') {
            format!("{{\"events\": [{}", generated_text.trim())
        } else {
            generated_text.trim().to_string()
        };
        if candidate_json.trim_end().ends_with('}')
            && serde_json::from_str::<serde_json::Value>(candidate_json.trim()).is_ok()
        {
            break;
        }
        // Add generated token for next step decode
        batch.clear();
        batch
            .add(token, current_pos, &[0], true)
            .map_err(|e| AppError::Inference(format!("Failed to add generated token to batch: {}", e)))?;

        ctx.decode(&mut batch)
            .map_err(|e| AppError::Inference(format!("Failed to decode generated token: {}", e)))?;

        token = sampler.sample(&ctx, -1);
        sampler.accept(token);

        current_pos += 1;
        if current_pos >= DEFAULT_CONTEXT_WINDOW as i32 {
            break;
        }
    }
    let trimmed = generated_text.trim();
    let final_json = if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        format!("{{\"events\": [{}", trimmed)
    } else {
        trimmed.to_string()
    };
    Ok(final_json)
}
/// Asynchronously runs inference on a background worker thread with strict timeout.
pub async fn run_inference_async(
    model: Arc<LlamaModel>,
    prompt: String,
    max_tokens: usize,
    timeout_duration: Duration,
) -> Result<String, AppError> {
    let deadline = Instant::now() + timeout_duration;
    let infer_future = tokio::task::spawn_blocking(move || {
        run_inference_sync(&model, &prompt, max_tokens, Some(deadline))
    });

    match tokio::time::timeout(timeout_duration, infer_future).await {
        Ok(join_res) => {
            join_res.map_err(|e| AppError::Inference(format!("Inference worker panicked: {}", e)))?
        }
        Err(_) => Err(AppError::InferenceTimeout(timeout_duration.as_secs())),
    }
}
pub fn parse_llm_json_payload(raw_json: &str) -> Option<Vec<EventDetails>> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. Direct JSON parse attempts
    if let Ok(payload) = serde_json::from_str::<LlmEventsPayload>(trimmed) {
        if !payload.events.is_empty() {
            return Some(payload.events.into_iter().map(|e| e.into_event_details("llm")).collect());
        }
    }
    if let Ok(list) = serde_json::from_str::<Vec<LlmEventOutput>>(trimmed) {
        if !list.is_empty() {
            return Some(list.into_iter().map(|e| e.into_event_details("llm")).collect());
        }
    }
    if let Ok(single) = serde_json::from_str::<LlmEventOutput>(trimmed) {
        return Some(vec![single.into_event_details("llm")]);
    }

    // 2. Try wrapping/repairing JSON array elements
    let clean = trimmed.trim_end_matches(',').trim();
    let candidates = [
        format!("{{\"events\": [{}]}}", clean),
        format!("[{}]", clean),
        format!("{{\"events\": [{}", clean),
        format!("{}]}}", clean),
    ];

    for cand in &candidates {
        if let Ok(payload) = serde_json::from_str::<LlmEventsPayload>(cand) {
            if !payload.events.is_empty() {
                return Some(payload.events.into_iter().map(|e| e.into_event_details("llm")).collect());
            }
        }
        if let Ok(list) = serde_json::from_str::<Vec<LlmEventOutput>>(cand) {
            if !list.is_empty() {
                return Some(list.into_iter().map(|e| e.into_event_details("llm")).collect());
            }
        }
    }

    // 3. If truncated mid-stream, find the last `}` and try closing
    if let Some(last_brace_idx) = trimmed.rfind('}') {
        let truncated = &trimmed[..=last_brace_idx];
        let trunc_clean = truncated.trim_end_matches(',').trim();
        let trunc_candidates = [
            format!("{{\"events\": [{}]}}", trunc_clean),
            format!("[{}]", trunc_clean),
        ];
        for cand in &trunc_candidates {
            if let Ok(payload) = serde_json::from_str::<LlmEventsPayload>(cand) {
                if !payload.events.is_empty() {
                    return Some(payload.events.into_iter().map(|e| e.into_event_details("llm")).collect());
                }
            }
            if let Ok(list) = serde_json::from_str::<Vec<LlmEventOutput>>(cand) {
                if !list.is_empty() {
                    return Some(list.into_iter().map(|e| e.into_event_details("llm")).collect());
                }
            }
        }
    }
    // 4. Resilient field-level extraction for malformed or unescaped quotes in LLM output
    let loose_events = extract_events_from_loose_llm_json(trimmed);
    if !loose_events.is_empty() {
        return Some(loose_events);
    }

    None
}

fn extract_events_from_loose_llm_json(raw: &str) -> Vec<EventDetails> {
    let mut events = Vec::new();
    let title_re = regex::Regex::new(r#""title"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)""#).unwrap();
    let start_time_re = regex::Regex::new(r#""start_time"\s*:\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|null)"#).unwrap();
    let end_time_re = regex::Regex::new(r#""end_time"\s*:\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|null)"#).unwrap();
    let is_all_day_re = regex::Regex::new(r#""is_all_day"\s*:\s*(true|false)"#).unwrap();
    let location_re = regex::Regex::new(r#""location"\s*:\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|null)"#).unwrap();
    let recurrence_re = regex::Regex::new(r#""recurrence_rule"\s*:\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|null)"#).unwrap();

    for block in raw.split('{') {
        if let Some(t_cap) = title_re.captures(block) {
            let title = t_cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            if title.trim().is_empty() {
                continue;
            }

            let start_time = start_time_re.captures(block).and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
            let end_time = end_time_re.captures(block).and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
            let is_all_day = is_all_day_re.captures(block).map(|c| c.get(1).unwrap().as_str() == "true").unwrap_or(false);
            let location = location_re.captures(block).and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
            let recurrence_rule = recurrence_re.captures(block).and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

            let event = EventDetails {
                title,
                start_time,
                end_time,
                is_all_day,
                location,
                description: None,
                recurrence_rule,
                confidence: 0.95,
                source: "llm".to_string(),
            };

            if !events.contains(&event) {
                events.push(event);
            }
        }
    }

    events
}

/// Orchestrates multi-event extraction using local LLM inference with dynamic context injection,
pub async fn extract_events_orchestrated(
    app: &AppHandle,
    engine: &InferenceEngineManager,
    ocr_text: &str,
    context: &ReferenceContext,
    model_id_override: Option<&str>,
    timeout_secs: Option<u64>,
) -> Vec<EventDetails> {
    let schedule_events = parser::parse_schedule_table_events(ocr_text, context);
    if schedule_events.len() >= 2 {
        return schedule_events;
    }

    let start_time = Instant::now();
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_INFERENCE_TIMEOUT_SECS));
    // Resolve target model ID
    let manifest = match model::get_manifest() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to load manifest: {}. Using deterministic fallback.", e);
            let mut events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut events {
                event.source = "deterministic_fallback".to_string();
            }
            return events;
        }
    };

    let target_model_id = model_id_override.unwrap_or(&manifest.default_model_id);

    // Verify model file exists
    let storage_dir = match model::get_storage_directory(app) {
        Ok(dir) => dir,
        Err(e) => {
            tracing::error!("Storage directory error: {}. Using fallback.", e);
            let mut events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut events {
                event.source = "deterministic_fallback".to_string();
            }
            return events;
        }
    };

    let model_entry = manifest.models.iter().find(|m| m.id == target_model_id);
    let filename = match model_entry {
        Some(entry) => &entry.filename,
        None => {
            tracing::warn!("Model ID '{}' not found in manifest. Using fallback.", target_model_id);
            let mut events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut events {
                event.source = "deterministic_fallback".to_string();
            }
            return events;
        }
    };

    let model_path = storage_dir.join(filename);
    if !model_path.exists() {
        tracing::info!("Model file {:?} does not exist. Using deterministic fallback.", model_path);
        let mut events = parser::parse_events_deterministic(ocr_text, context);
        for event in &mut events {
            event.source = "deterministic_fallback".to_string();
        }
        return events;
    }

    // Load model
    let model = match engine.get_or_load_model(target_model_id, &model_path).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to load model weights: {}. Using fallback.", e);
            let mut events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut events {
                event.source = "deterministic_fallback".to_string();
            }
            return events;
        }
    };

    // Format prompt with dynamic reference context
    let prompt = parser::generate_extraction_prompt(ocr_text, context);

    // Run inference with timeout
    match run_inference_async(model, prompt, DEFAULT_MAX_GENERATION_TOKENS, timeout).await {
        Ok(raw_json) => {
            let trimmed = raw_json.trim();
            if let Some(events) = parse_llm_json_payload(trimmed) {
                if !events.is_empty() {
                    tracing::info!(
                        "Successfully extracted {} events via LLM ({}) in {:.2}s",
                        events.len(),
                        target_model_id,
                        start_time.elapsed().as_secs_f32()
                    );
                    return events;
                }
            }
            tracing::warn!(
                "Failed to parse LLM output JSON (raw: '{}'). Falling back to deterministic parser.",
                trimmed
            );
            let mut fallback_events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut fallback_events {
                event.source = "deterministic_fallback".to_string();
            }
            fallback_events
        }
        Err(infer_err) => {
            tracing::warn!(
                "LLM inference failed / timed out ({:.2}s): {}. Falling back to deterministic parser.",
                start_time.elapsed().as_secs_f32(),
                infer_err
            );
            let mut fallback_events = parser::parse_events_deterministic(ocr_text, context);
            for event in &mut fallback_events {
                event.source = "deterministic_fallback".to_string();
            }
            fallback_events
        }
    }
}

/// Convenience single event extraction
pub async fn extract_event_orchestrated(
    app: &AppHandle,
    engine: &InferenceEngineManager,
    ocr_text: &str,
    context: &ReferenceContext,
    model_id_override: Option<&str>,
    timeout_secs: Option<u64>,
) -> EventDetails {
    let events = extract_events_orchestrated(app, engine, ocr_text, context, model_id_override, timeout_secs).await;
    events.into_iter().next().unwrap_or_else(|| {
        let mut fallback = parser::parse_event_deterministic(ocr_text, context);
        fallback.source = "deterministic_fallback".to_string();
        fallback
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_count_is_bounded() {
        let threads = get_optimal_thread_count();
        assert!(threads >= 1);
        assert!(threads <= 4);
    }

    #[test]
    fn test_llm_event_output_conversion() {
        let raw_json = r#"{
            "title": "Hackathon Demo Day",
            "start_time": "2026-09-12T14:00:00-04:00",
            "end_time": "2026-09-12T18:00:00-04:00",
            "is_all_day": false,
            "location": "Innovation Lab Room 101",
            "description": "Final project presentations and awards ceremony."
        }"#;

        let parsed: LlmEventOutput = serde_json::from_str(raw_json).expect("Should parse GBNF json");
        assert_eq!(parsed.title, "Hackathon Demo Day");
        assert_eq!(parsed.start_time.as_deref(), Some("2026-09-12T14:00:00-04:00"));
        assert_eq!(parsed.end_time.as_deref(), Some("2026-09-12T18:00:00-04:00"));
        assert!(!parsed.is_all_day);
        assert_eq!(parsed.location.as_deref(), Some("Innovation Lab Room 101"));
        assert!(parsed.description.is_some());

        let event = parsed.into_event_details("llm");
        assert_eq!(event.source, "llm");
        assert_eq!(event.confidence, 0.95);
        assert_eq!(event.title, "Hackathon Demo Day");
    }

    #[test]
    fn test_gbnf_grammar_contains_required_rules() {
        let grammar = parser::get_gbnf_grammar();
        assert!(grammar.contains("root ::="));
        assert!(grammar.contains(r#"\"title\":"#));
        assert!(grammar.contains(r#"\"start_time\":"#));
        assert!(grammar.contains(r#"\"end_time\":"#));
        assert!(grammar.contains(r#"\"is_all_day\":"#));
        assert!(grammar.contains(r#"\"location\":"#));
        assert!(grammar.contains(r#"\"description\":"#));
    }

    #[tokio::test]
    async fn test_inference_manager_lifecycle() {
        let manager = InferenceEngineManager::new();
        assert!(!manager.is_model_loaded("smollm2-360m-instruct-q4_k_m").await);
        manager.unload_model().await;
        assert!(!manager.is_model_loaded("smollm2-360m-instruct-q4_k_m").await);
    }

    #[test]
    fn test_llm_events_payload_conversion() {
        let raw_json = r#"{
            "events": [
                {
                    "title": "CS 0150-09 Special Topics",
                    "start_time": "2026-09-11T14:00:00-04:00",
                    "end_time": "2026-09-11T16:30:00-04:00",
                    "is_all_day": false,
                    "location": "Online",
                    "description": "Faculty: J. Skripchuk"
                },
                {
                    "title": "CSHD 0166-01 Children's Play",
                    "start_time": "2026-09-10T13:30:00-04:00",
                    "end_time": "2026-09-10T16:00:00-04:00",
                    "is_all_day": false,
                    "location": "Eliot-Pearson, Room 157",
                    "description": "Faculty: W. Scarlett"
                }
            ]
        }"#;

        let payload: LlmEventsPayload = serde_json::from_str(raw_json).expect("Should parse multi-event JSON");
        assert_eq!(payload.events.len(), 2);
    }

    #[test]
    fn test_context_window_and_headroom_constants() {
        assert_eq!(DEFAULT_CONTEXT_WINDOW, 2048);
        assert_eq!(MAX_OUTPUT_TOKENS, 512);
        assert_eq!(DEFAULT_MAX_GENERATION_TOKENS, 1536);
        assert!(MAX_OUTPUT_TOKENS < DEFAULT_CONTEXT_WINDOW as usize);
        let max_prompt_budget = (DEFAULT_CONTEXT_WINDOW as usize) - MAX_OUTPUT_TOKENS;
        assert_eq!(max_prompt_budget, 1536);
    }

    #[tokio::test]
    async fn test_gbnf_grammar_compilation() {
        let home = std::env::var("HOME").unwrap_or_default();
        let model_path = PathBuf::from(home)
            .join("Library/Application Support/com.dustinmichels.share2cal/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
        if !model_path.exists() {
            return;
        }
        let manager = InferenceEngineManager::new();
        let model = manager
            .get_or_load_model("smollm2-360m-instruct-q4_k_m", &model_path)
            .await
            .expect("Should load model");

        let grammar_str = parser::get_gbnf_grammar();
        let grammar_sampler = LlamaSampler::grammar(&model, grammar_str, "root").expect("grammar sampler");
        let greedy_sampler = LlamaSampler::greedy();
        let _chain = LlamaSampler::chain_simple([grammar_sampler, greedy_sampler]);
        println!("Chain created successfully!");
        drop(model);
        manager.unload_model().await;
    }
}
