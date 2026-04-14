// ============================================================
// Local Inference — Multi-Backend LLM Integration for Training
//
// Connects LFI's training pipeline to local LLM backends so it
// can ask questions, get answers, verify them, and learn from
// the results. This is how LFI transitions from memorization
// to genuine reasoning.
//
// BACKENDS SUPPORTED:
//   - Ollama (local models: llama3, mistral, phi3, etc.)
//   - Gemini CLI (existing integration)
//   - Claude CLI (claude code)
//   - Direct HTTP (any OpenAI-compatible API at localhost)
//   - Mock (for testing without a real model)
//
// TRAINING FLOW:
//   1. LFI presents a question from training data
//   2. Local LLM generates an answer
//   3. LFI compares answer to expected output
//   4. If correct: reinforce the concept
//   5. If wrong: classify error type, teach correct answer, learn from reasoning
//
// INTELLIGENT FEATURES:
//   - Progressive model routing: lightweight model for easy, heavy for hard
//   - Inference cache: avoid re-querying same/similar questions
//   - Error taxonomy: classify WHY answers are wrong, not just IF
//   - Active learning: prioritize highest-value questions
//   - Multi-model ensemble: vote across backends for higher accuracy
// ============================================================

use crate::hdc::error::HdcError;
use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::training_data::TrainingExample;
use std::collections::HashMap;

// ============================================================
// Backend Configuration
// ============================================================

/// Which local inference backend to use.
#[derive(Debug, Clone)]
pub enum InferenceBackend {
    /// Ollama running locally (default port 11434).
    Ollama { model: String, host: String },
    /// Gemini CLI (existing LFI integration).
    GeminiCli,
    /// Claude Code CLI.
    ClaudeCli,
    /// Any OpenAI-compatible HTTP endpoint.
    HttpApi { url: String, model: String },
    /// Mock backend for testing — returns predefined answers.
    Mock { answers: Vec<String> },
}

impl Default for InferenceBackend {
    fn default() -> Self {
        Self::Mock { answers: vec!["I don't know".into()] }
    }
}

// ============================================================
// Error Taxonomy — classify WHY answers are wrong
// ============================================================

/// Classification of why an LLM answer was incorrect.
/// BUG ASSUMPTION: Error classification is heuristic-based; edge cases
/// may misclassify. Treat as advisory, not authoritative.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Answer is factually wrong (e.g., 2+3=7).
    FactualError,
    /// Answer shows correct approach but wrong conclusion.
    ReasoningError,
    /// Answer is correct but in wrong format (e.g., "seven" vs "7").
    FormatMismatch,
    /// Answer contains the correct info buried in noise.
    PartialCorrect,
    /// Answer confidently states something fabricated.
    Hallucination,
    /// Answer is about a completely different topic.
    OffTopic,
    /// Answer is empty, refused, or "I don't know".
    Refusal,
    /// Could not classify the error type.
    Unknown,
}

impl ErrorKind {
    /// Classify an incorrect answer against the expected output.
    /// BUG ASSUMPTION: classification is best-effort heuristic.
    fn classify(answer: &str, expected: &str, question: &str) -> Self {
        let answer_lower = answer.to_lowercase();
        let expected_lower = expected.to_lowercase();

        // Refusal detection
        let refusal_patterns = [
            "i don't know", "i cannot", "i'm not sure", "unable to",
            "i can't", "no answer", "not enough information",
        ];
        if refusal_patterns.iter().any(|p| answer_lower.contains(p)) || answer.trim().is_empty() {
            return Self::Refusal;
        }

        // Off-topic: does the answer share ANY significant words with the question or expected?
        let answer_words = Self::extract_words(&answer_lower);
        let question_words = Self::extract_words(&question.to_lowercase());
        let expected_words = Self::extract_words(&expected_lower);
        let topic_overlap = answer_words.iter()
            .filter(|w| question_words.contains(*w) || expected_words.contains(*w))
            .count();
        if topic_overlap == 0 && answer_words.len() > 3 {
            return Self::OffTopic;
        }

        // Partial correct: answer contains most expected words but not all
        if !expected_words.is_empty() {
            let expected_overlap = answer_words.iter()
                .filter(|w| expected_words.contains(*w))
                .count();
            let ratio = expected_overlap as f64 / expected_words.len() as f64;
            if ratio > 0.3 && ratio < 0.8 {
                return Self::PartialCorrect;
            }
        }

        // Format mismatch: numeric answers that are equivalent
        if Self::numeric_equivalent(&answer_lower, &expected_lower) {
            return Self::FormatMismatch;
        }

        // Reasoning error: answer mentions key domain concepts but gets conclusion wrong
        if topic_overlap > 2 {
            return Self::ReasoningError;
        }

        // Hallucination: long confident answer with no overlap
        if answer.len() > 100 && topic_overlap < 2 {
            return Self::Hallucination;
        }

        // Default: factual error
        Self::FactualError
    }

    fn extract_words(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }

    fn numeric_equivalent(a: &str, b: &str) -> bool {
        let num_a = a.trim().parse::<f64>();
        let num_b = b.trim().parse::<f64>();
        if let (Ok(va), Ok(vb)) = (num_a, num_b) {
            return (va - vb).abs() < 1e-6;
        }
        // Check written numbers
        let written = [
            ("zero", 0.0), ("one", 1.0), ("two", 2.0), ("three", 3.0),
            ("four", 4.0), ("five", 5.0), ("six", 6.0), ("seven", 7.0),
            ("eight", 8.0), ("nine", 9.0), ("ten", 10.0),
        ];
        for (word, val) in &written {
            if a.contains(word) {
                if let Ok(vb) = b.trim().parse::<f64>() {
                    if (val - vb).abs() < 1e-6 { return true; }
                }
            }
            if b.contains(word) {
                if let Ok(va) = a.trim().parse::<f64>() {
                    if (val - va).abs() < 1e-6 { return true; }
                }
            }
        }
        false
    }
}

// ============================================================
// Inference Result
// ============================================================

/// Result of asking a local LLM a question.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub question: String,
    pub answer: String,
    pub backend: String,
    pub latency_ms: u64,
    pub correct: Option<bool>,
    /// Error classification when answer is wrong.
    pub error_kind: Option<ErrorKind>,
    /// Whether this result came from cache.
    pub cached: bool,
}

// ============================================================
// Inference Cache — avoid re-querying the same questions
// ============================================================

/// Cache for LLM responses. Keyed by normalized question text.
/// BUG ASSUMPTION: cache poisoning possible if LLM returns garbage
/// that gets cached. Stale entries may persist across model changes.
pub struct InferenceCache {
    /// Exact-match cache: normalized question → (answer, backend, timestamp_ms).
    entries: HashMap<String, CacheEntry>,
    /// Maximum cache size before eviction.
    max_entries: usize,
    /// Cache hits counter.
    hits: usize,
    /// Cache misses counter.
    misses: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    answer: String,
    backend: String,
    timestamp_ms: u64,
}

impl InferenceCache {
    pub fn new(max_entries: usize) -> Self {
        debuglog!("InferenceCache::new: max_entries={}", max_entries);
        Self {
            entries: HashMap::new(),
            max_entries,
            hits: 0,
            misses: 0,
        }
    }

    /// Normalize a question for cache lookup (lowercase, trim, collapse whitespace).
    fn normalize(question: &str) -> String {
        question.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Look up a cached answer.
    pub fn get(&mut self, question: &str) -> Option<&CacheEntry> {
        let key = Self::normalize(question);
        if self.entries.contains_key(&key) {
            self.hits += 1;
            self.entries.get(&key)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store an answer in the cache.
    pub fn put(&mut self, question: &str, answer: &str, backend: &str) {
        if self.entries.len() >= self.max_entries {
            // Evict oldest entry (simple strategy — first key found).
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
        let key = Self::normalize(question);
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.entries.insert(key, CacheEntry {
            answer: answer.to_string(),
            backend: backend.to_string(),
            timestamp_ms,
        });
    }

    /// Cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear the cache (e.g., when switching models).
    pub fn clear(&mut self) {
        self.entries.clear();
        debuglog!("InferenceCache::clear: cache flushed");
    }
}

// ============================================================
// Progressive Model Router — lightweight for easy, heavy for hard
// ============================================================

/// Routes questions to different backends based on difficulty.
/// BUG ASSUMPTION: difficulty thresholds are tuned for Ollama on i7/64GB.
/// Different hardware may need different thresholds.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    /// Backend for easy questions (difficulty <= easy_threshold).
    pub lightweight_backend: InferenceBackend,
    /// Backend for hard questions (difficulty > easy_threshold).
    pub heavyweight_backend: InferenceBackend,
    /// Difficulty boundary: below = lightweight, above = heavyweight.
    pub difficulty_threshold: f64,
    /// If true, the router is active. If false, always use lightweight.
    pub enabled: bool,
}

impl ModelRouter {
    /// Create a router with Ollama models optimized for laptop (i7/3050Ti).
    /// BUG ASSUMPTION: assumes both models are available in Ollama.
    pub fn laptop_optimized(host: &str) -> Self {
        Self {
            lightweight_backend: InferenceBackend::Ollama {
                model: "deepseek-r1:8b".into(),
                host: host.into(),
            },
            heavyweight_backend: InferenceBackend::Ollama {
                model: "qwen2.5-coder:7b".into(),
                host: host.into(),
            },
            difficulty_threshold: 0.5,
            enabled: true,
        }
    }

    /// Select the appropriate backend for a given difficulty.
    pub fn route(&self, difficulty: f64) -> &InferenceBackend {
        if !self.enabled || difficulty <= self.difficulty_threshold {
            &self.lightweight_backend
        } else {
            &self.heavyweight_backend
        }
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self {
            lightweight_backend: InferenceBackend::default(),
            heavyweight_backend: InferenceBackend::default(),
            difficulty_threshold: 0.5,
            enabled: false,
        }
    }
}

// ============================================================
// Active Learning — prioritize highest-value questions
// ============================================================

/// Scores training examples by expected learning value.
/// Higher score = more valuable to ask next.
pub struct ActiveLearner;

impl ActiveLearner {
    /// Score an example by expected information gain.
    ///
    /// Factors: (1) domains with lowest mastery get priority,
    /// (2) questions near the mastery boundary (not too easy, not too hard),
    /// (3) cross-domain bridge questions get a boost.
    pub fn score_example(
        example: &TrainingExample,
        knowledge: &KnowledgeEngine,
        error_history: &HashMap<String, Vec<ErrorKind>>,
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: inverse mastery — low-mastery domains are more valuable.
        let mastery = knowledge.domain_mastery(&example.domain);
        score += 1.0 - mastery; // Max 1.0 for completely unknown domain

        // Factor 2: difficulty sweet spot — near current mastery boundary.
        // Questions slightly above current mastery are most informative.
        let difficulty_gap = (example.difficulty - mastery).abs();
        let sweet_spot_bonus = if difficulty_gap < 0.2 { 0.5 } else { 0.0 };
        score += sweet_spot_bonus;

        // Factor 3: error-prone domains get priority.
        if let Some(errors) = error_history.get(&example.domain) {
            let error_rate = errors.len() as f64 / (errors.len() as f64 + 1.0);
            score += error_rate * 0.3;
        }

        // Factor 4: cross-domain examples are extra valuable.
        if example.tags.len() > 1 {
            score += 0.2;
        }

        score
    }

    /// Rank examples by learning value and return sorted indices.
    pub fn prioritize(
        examples: &[TrainingExample],
        knowledge: &KnowledgeEngine,
        error_history: &HashMap<String, Vec<ErrorKind>>,
    ) -> Vec<usize> {
        let mut scored: Vec<(usize, f64)> = examples.iter()
            .enumerate()
            .map(|(i, ex)| (i, Self::score_example(ex, knowledge, error_history)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(i, _)| i).collect()
    }
}

// ============================================================
// Training Configuration
// ============================================================

/// Configuration for local inference training.
#[derive(Debug, Clone)]
pub struct InferenceTrainingConfig {
    pub backend: InferenceBackend,
    pub max_retries: usize,
    pub timeout_ms: u64,
    pub verify_answers: bool,
    /// Enable inference caching.
    pub cache_enabled: bool,
    /// Maximum cache entries.
    pub cache_max_entries: usize,
    /// Enable active learning question prioritization.
    pub active_learning: bool,
}

impl Default for InferenceTrainingConfig {
    fn default() -> Self {
        Self {
            backend: InferenceBackend::default(),
            max_retries: 2,
            timeout_ms: 30_000,
            verify_answers: true,
            cache_enabled: true,
            cache_max_entries: 1000,
            active_learning: true,
        }
    }
}

// ============================================================
// Inference Trainer — the main engine
// ============================================================

/// The local inference trainer — asks LLMs questions and learns from answers.
pub struct InferenceTrainer {
    config: InferenceTrainingConfig,
    questions_asked: usize,
    correct_answers: usize,
    mock_index: usize,
    /// Response cache.
    cache: InferenceCache,
    /// Error history per domain.
    error_history: HashMap<String, Vec<ErrorKind>>,
    /// Optional model router for progressive difficulty.
    router: Option<ModelRouter>,
}

impl InferenceTrainer {
    pub fn new(config: InferenceTrainingConfig) -> Self {
        debuglog!("InferenceTrainer::new: backend={:?}, cache={}, active_learning={}",
            config.backend, config.cache_enabled, config.active_learning);
        let cache_max = config.cache_max_entries;
        Self {
            config,
            questions_asked: 0,
            correct_answers: 0,
            mock_index: 0,
            cache: InferenceCache::new(cache_max),
            error_history: HashMap::new(),
            router: None,
        }
    }

    /// Create a trainer with progressive model routing for laptop.
    pub fn with_router(config: InferenceTrainingConfig, router: ModelRouter) -> Self {
        debuglog!("InferenceTrainer::with_router: progressive routing enabled, threshold={:.2}",
            router.difficulty_threshold);
        let cache_max = config.cache_max_entries;
        Self {
            config,
            questions_asked: 0,
            correct_answers: 0,
            mock_index: 0,
            cache: InferenceCache::new(cache_max),
            error_history: HashMap::new(),
            router: Some(router),
        }
    }

    /// Ask a question and get an answer from the local LLM.
    /// BUG ASSUMPTION: cache may serve stale answers if model was changed.
    pub fn ask(&mut self, question: &str) -> Result<InferenceResult, HdcError> {
        self.ask_with_difficulty(question, None)
    }

    /// Ask a question with optional difficulty hint for model routing.
    pub fn ask_with_difficulty(
        &mut self,
        question: &str,
        difficulty: Option<f64>,
    ) -> Result<InferenceResult, HdcError> {
        debuglog!("InferenceTrainer::ask: '{}' (diff={:?})",
            crate::truncate_str(question, 60), difficulty);

        // Check cache first.
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(question) {
                debuglog!("InferenceTrainer::ask: CACHE HIT");
                self.questions_asked += 1;
                return Ok(InferenceResult {
                    question: question.into(),
                    answer: cached.answer.clone(),
                    backend: format!("{}_cached", cached.backend),
                    latency_ms: 0,
                    correct: None,
                    error_kind: None,
                    cached: true,
                });
            }
        }

        let start = std::time::Instant::now();

        // Select backend: router > config default.
        let backend = if let Some(ref router) = self.router {
            router.route(difficulty.unwrap_or(0.5)).clone()
        } else {
            self.config.backend.clone()
        };

        let (answer, backend_name) = self.call_backend(&backend, question);

        let latency = start.elapsed().as_millis() as u64;
        self.questions_asked += 1;

        // Cache the response.
        if self.config.cache_enabled && !answer.starts_with("ERROR:") {
            self.cache.put(question, &answer, &backend_name);
        }

        Ok(InferenceResult {
            question: question.into(),
            answer,
            backend: backend_name.into(),
            latency_ms: latency,
            correct: None,
            error_kind: None,
            cached: false,
        })
    }

    /// Dispatch to the appropriate backend.
    fn call_backend(&mut self, backend: &InferenceBackend, question: &str) -> (String, String) {
        match backend {
            InferenceBackend::Mock { answers } => {
                let idx = self.mock_index % answers.len().max(1);
                self.mock_index += 1;
                (answers.get(idx).cloned().unwrap_or_default(), "mock".into())
            }
            InferenceBackend::Ollama { model, host } => {
                match Self::call_ollama(host, model, question) {
                    Ok(answer) => (answer, format!("ollama:{}", model)),
                    Err(e) => {
                        debuglog!("InferenceTrainer: Ollama failed: {:?}", e);
                        (format!("ERROR: {}", e), "ollama_error".into())
                    }
                }
            }
            InferenceBackend::GeminiCli => {
                match Self::call_cli("gemini", &["chat", question]) {
                    Ok(answer) => (answer, "gemini".into()),
                    Err(e) => (format!("ERROR: {}", e), "gemini_error".into()),
                }
            }
            InferenceBackend::ClaudeCli => {
                match Self::call_cli("claude", &["-p", question]) {
                    Ok(answer) => (answer, "claude".into()),
                    Err(e) => (format!("ERROR: {}", e), "claude_error".into()),
                }
            }
            InferenceBackend::HttpApi { url, model } => {
                match Self::call_http_api(url, model, question) {
                    Ok(answer) => (answer, format!("http:{}", model)),
                    Err(e) => (format!("ERROR: {}", e), "http_error".into()),
                }
            }
        }
    }

    /// Ask a training question, verify the answer, classify errors, and learn.
    pub fn train_on_example(
        &mut self,
        example: &TrainingExample,
        knowledge: &mut KnowledgeEngine,
    ) -> Result<InferenceResult, HdcError> {
        let mut result = self.ask_with_difficulty(&example.input, Some(example.difficulty))?;

        if self.config.verify_answers {
            // Use the robust AnswerVerifier: handles whitespace, unicode,
            // LaTeX, units, equation forms, commentary, numeric equivalence,
            // word-number, and fuzzy keyword overlap. Returns detailed
            // matched_mode for debugging.
            let verify_result = crate::intelligence::answer_verifier::AnswerVerifier::verify(
                &result.answer, &example.expected_output,
            );
            let is_correct = verify_result.is_correct;
            if is_correct {
                debuglog!("InferenceTrainer::verify: CORRECT via {:?} (conf={:.2})",
                    verify_result.matched_mode, verify_result.confidence);
            }

            result.correct = Some(is_correct);

            if is_correct {
                self.correct_answers += 1;
                knowledge.reinforce(&example.domain);
                debuglog!("InferenceTrainer: CORRECT — reinforcing '{}'", example.domain);
            } else {
                // Classify the error.
                let error_kind = ErrorKind::classify(
                    &result.answer, &example.expected_output, &example.input,
                );
                debuglog!("InferenceTrainer: WRONG ({:?}) — teaching correct answer", error_kind);

                // Record error in history for active learning.
                self.error_history
                    .entry(example.domain.clone())
                    .or_default()
                    .push(error_kind.clone());

                result.error_kind = Some(error_kind);

                // Teach the correct answer.
                let concept_name = format!("inferred_{}_{}", example.domain, self.questions_asked);
                let answer_preview: String = result.answer.chars().take(100).collect();
                knowledge.learn_with_definition(
                    &concept_name,
                    &format!("Q: {} A: {} (LLM said: {})",
                        example.input, example.expected_output, answer_preview),
                    &[&example.domain],
                    0.6,
                    true,
                )?;
            }
        }

        Ok(result)
    }

    /// Run inference training across all examples.
    /// With active learning enabled, questions are prioritized by expected value.
    pub fn train_all(
        &mut self,
        examples: &[TrainingExample],
        knowledge: &mut KnowledgeEngine,
    ) -> Result<InferenceTrainingResult, HdcError> {
        debuglog!("InferenceTrainer::train_all: {} examples, active_learning={}",
            examples.len(), self.config.active_learning);

        let order: Vec<usize> = if self.config.active_learning {
            ActiveLearner::prioritize(examples, knowledge, &self.error_history)
        } else {
            (0..examples.len()).collect()
        };

        let mut results = Vec::new();
        for &idx in &order {
            match self.train_on_example(&examples[idx], knowledge) {
                Ok(result) => results.push(result),
                Err(e) => debuglog!("InferenceTrainer: Example {} failed: {:?}", idx, e),
            }
        }

        let correct = results.iter().filter(|r| r.correct == Some(true)).count();
        let total = results.len();
        let accuracy = if total > 0 { correct as f64 / total as f64 } else { 0.0 };

        // Build per-domain error summary.
        let mut error_summary = HashMap::new();
        for r in &results {
            if let Some(ref ek) = r.error_kind {
                *error_summary.entry(format!("{:?}", ek)).or_insert(0usize) += 1;
            }
        }

        Ok(InferenceTrainingResult {
            total_questions: total,
            correct_answers: correct,
            accuracy,
            results,
            cache_hit_rate: self.cache.hit_rate(),
            error_breakdown: error_summary,
        })
    }

    /// Accuracy so far.
    pub fn accuracy(&self) -> f64 {
        if self.questions_asked == 0 { return 0.0; }
        self.correct_answers as f64 / self.questions_asked as f64
    }

    /// Cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }

    /// Error history for analysis.
    pub fn error_history(&self) -> &HashMap<String, Vec<ErrorKind>> {
        &self.error_history
    }

    /// Domains with highest error rates (for targeted retraining).
    pub fn weakest_domains(&self, top_n: usize) -> Vec<(String, usize)> {
        let mut domains: Vec<(String, usize)> = self.error_history.iter()
            .map(|(d, errors)| (d.clone(), errors.len()))
            .collect();
        domains.sort_by(|a, b| b.1.cmp(&a.1));
        domains.truncate(top_n);
        domains
    }

    /// Simple fuzzy match — checks if key terms overlap.
    fn fuzzy_match(answer: &str, expected: &str) -> bool {
        let answer_words: std::collections::HashSet<String> = answer.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|s| s.to_string())
            .collect();
        let expected_words: std::collections::HashSet<String> = expected.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|s| s.to_string())
            .collect();
        if expected_words.is_empty() {
            return false;
        }
        let overlap = answer_words.intersection(&expected_words).count();
        let needed = (expected_words.len() + 1) / 2;
        overlap >= needed
    }

    /// Call Ollama HTTP API synchronously.
    /// BUG ASSUMPTION: curl must be available on PATH. No TLS validation on localhost.
    fn call_ollama(host: &str, model: &str, prompt: &str) -> Result<String, String> {
        // Sanitize prompt for JSON embedding.
        let safe_prompt = prompt.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        let output = std::process::Command::new("curl")
            .args(&[
                "-s", "--max-time", "120",
                "-X", "POST",
                &format!("{}/api/generate", host),
                "-H", "Content-Type: application/json",
                "-d", &format!(
                    r#"{{"model":"{}","prompt":"Answer concisely: {}","stream":false,"options":{{"temperature":0.3,"num_predict":200}}}}"#,
                    model, safe_prompt
                ),
            ])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if output.status.success() {
            let body = String::from_utf8_lossy(&output.stdout).to_string();
            // Extract response from Ollama JSON.
            if let Some(start) = body.find("\"response\":\"") {
                let rest = &body[start + 12..];
                if let Some(end) = rest.find('"') {
                    return Ok(rest[..end].replace("\\n", "\n").replace("\\t", "\t"));
                }
            }
            // Fallback: try to find response with escaped quotes
            if let Some(start) = body.find("\"response\":") {
                let rest = &body[start + 11..];
                // Skip whitespace and opening quote
                let trimmed = rest.trim_start();
                if trimmed.starts_with('"') {
                    let inner = &trimmed[1..];
                    // Find closing quote (not escaped)
                    let mut end = 0;
                    let bytes = inner.as_bytes();
                    while end < bytes.len() {
                        if bytes[end] == b'"' && (end == 0 || bytes[end - 1] != b'\\') {
                            break;
                        }
                        end += 1;
                    }
                    if end < bytes.len() {
                        return Ok(inner[..end].replace("\\n", "\n").replace("\\t", "\t"));
                    }
                }
            }
            Ok(body)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Call a CLI tool synchronously.
    fn call_cli(cmd: &str, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| format!("{} failed: {}", cmd, e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Call an OpenAI-compatible HTTP API.
    fn call_http_api(url: &str, model: &str, prompt: &str) -> Result<String, String> {
        let safe_prompt = prompt.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");

        let output = std::process::Command::new("curl")
            .args(&[
                "-s", "--max-time", "120",
                "-X", "POST",
                &format!("{}/v1/chat/completions", url),
                "-H", "Content-Type: application/json",
                "-d", &format!(
                    r#"{{"model":"{}","messages":[{{"role":"user","content":"Answer concisely: {}"}}],"max_tokens":200}}"#,
                    model, safe_prompt
                ),
            ])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

// ============================================================
// Multi-Model Ensemble — vote across backends for accuracy
// ============================================================

/// Ask multiple backends and take consensus.
/// BUG ASSUMPTION: ensemble is only as good as the weakest voter.
/// All models must be available or the ensemble degrades.
pub struct EnsembleInference {
    backends: Vec<InferenceBackend>,
}

impl EnsembleInference {
    pub fn new(backends: Vec<InferenceBackend>) -> Self {
        debuglog!("EnsembleInference::new: {} backends", backends.len());
        Self { backends }
    }

    /// Ask all backends and return the consensus answer.
    /// Voting: if any two answers fuzzy-match, that's the consensus.
    /// If no consensus, return the longest answer (heuristic: more detail = more effort).
    pub fn ask_ensemble(
        &self,
        question: &str,
        trainer: &mut InferenceTrainer,
    ) -> Result<InferenceResult, HdcError> {
        let start = std::time::Instant::now();
        let mut answers: Vec<(String, String)> = Vec::new();

        for backend in &self.backends {
            let (answer, name) = trainer.call_backend(backend, question);
            if !answer.starts_with("ERROR:") {
                answers.push((answer, name));
            }
        }

        if answers.is_empty() {
            return Err(HdcError::LogicFault {
                reason: "All ensemble backends failed".into(),
            });
        }

        // Vote: check for fuzzy-match pairs.
        let mut best_answer = answers[0].0.clone();
        let mut best_backend = answers[0].1.clone();

        for i in 0..answers.len() {
            for j in (i + 1)..answers.len() {
                if InferenceTrainer::fuzzy_match(&answers[i].0, &answers[j].0) {
                    // Consensus found — take the longer (more detailed) one.
                    best_answer = if answers[i].0.len() >= answers[j].0.len() {
                        answers[i].0.clone()
                    } else {
                        answers[j].0.clone()
                    };
                    best_backend = format!("ensemble_consensus({},{})", answers[i].1, answers[j].1);
                }
            }
        }

        // No consensus: take longest answer.
        if !best_backend.starts_with("ensemble") {
            if let Some(longest) = answers.iter().max_by_key(|(a, _)| a.len()) {
                best_answer = longest.0.clone();
                best_backend = format!("ensemble_longest({})", longest.1);
            }
        }

        let latency = start.elapsed().as_millis() as u64;

        Ok(InferenceResult {
            question: question.into(),
            answer: best_answer,
            backend: best_backend,
            latency_ms: latency,
            correct: None,
            error_kind: None,
            cached: false,
        })
    }
}

// ============================================================
// Training Results
// ============================================================

/// Results from inference-based training.
#[derive(Debug)]
pub struct InferenceTrainingResult {
    pub total_questions: usize,
    pub correct_answers: usize,
    pub accuracy: f64,
    pub results: Vec<InferenceResult>,
    /// Cache hit rate during this training run.
    pub cache_hit_rate: f64,
    /// Error type breakdown: ErrorKind name → count.
    pub error_breakdown: HashMap<String, usize>,
}

impl InferenceTrainingResult {
    /// Generate an ASCII report of the training results.
    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("=== Inference Training Report ===\n"));
        out.push_str(&format!("Total: {} | Correct: {} | Accuracy: {:.1}%\n",
            self.total_questions, self.correct_answers, self.accuracy * 100.0));
        out.push_str(&format!("Cache hit rate: {:.1}%\n", self.cache_hit_rate * 100.0));

        if !self.error_breakdown.is_empty() {
            out.push_str("\nError breakdown:\n");
            let mut errors: Vec<(&String, &usize)> = self.error_breakdown.iter().collect();
            errors.sort_by(|a, b| b.1.cmp(a.1));
            for (kind, count) in errors {
                out.push_str(&format!("  {:20} {}\n", kind, count));
            }
        }

        out
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::training_data::TrainingDataGenerator;

    #[test]
    fn test_mock_inference() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["42".into(), "Paris".into(), "DNA".into()],
            },
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);
        let result = trainer.ask("What is the meaning of life?")?;
        assert_eq!(result.answer, "42");
        assert_eq!(result.backend, "mock");
        assert!(!result.cached);
        Ok(())
    }

    #[test]
    fn test_mock_training() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["5".into()],
            },
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);
        let mut knowledge = KnowledgeEngine::new();

        let example = TrainingExample {
            domain: "math".into(),
            input: "2 + 3".into(),
            expected_output: "5".into(),
            difficulty: 0.1,
            tags: vec!["arithmetic".into()],
        };

        let result = trainer.train_on_example(&example, &mut knowledge)?;
        assert_eq!(result.correct, Some(true));
        assert_eq!(trainer.accuracy(), 1.0);
        assert!(result.error_kind.is_none()); // Correct → no error
        Ok(())
    }

    #[test]
    fn test_mock_wrong_answer_with_error_taxonomy() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["I don't know".into()],
            },
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);
        let mut knowledge = KnowledgeEngine::new();

        let example = TrainingExample {
            domain: "math".into(),
            input: "2 + 3".into(),
            expected_output: "5".into(),
            difficulty: 0.1,
            tags: vec![],
        };

        let result = trainer.train_on_example(&example, &mut knowledge)?;
        assert_eq!(result.correct, Some(false));
        assert_eq!(result.error_kind, Some(ErrorKind::Refusal));
        assert!(knowledge.concept_count() > 0);
        Ok(())
    }

    #[test]
    fn test_train_all_mock() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["generic answer".into()],
            },
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);
        let mut knowledge = KnowledgeEngine::new();
        let examples = TrainingDataGenerator::math_examples();

        let result = trainer.train_all(&examples[..5], &mut knowledge)?;
        assert_eq!(result.total_questions, 5);
        assert!(result.accuracy >= 0.0);
        assert!(result.cache_hit_rate >= 0.0);
        Ok(())
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(InferenceTrainer::fuzzy_match(
            "the answer is mass-energy equivalence",
            "mass-energy equivalence"
        ));
        assert!(!InferenceTrainer::fuzzy_match(
            "I have no idea",
            "mass-energy equivalence"
        ));
    }

    #[test]
    fn test_backend_default() {
        let backend = InferenceBackend::default();
        assert!(matches!(backend, InferenceBackend::Mock { .. }));
    }

    // ================================================================
    // Inference Cache Tests
    // ================================================================

    #[test]
    fn test_cache_hit() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["first answer".into(), "second answer".into()],
            },
            cache_enabled: true,
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);

        // First call: cache miss → gets "first answer".
        let r1 = trainer.ask("What is 2+2?")?;
        assert_eq!(r1.answer, "first answer");
        assert!(!r1.cached);

        // Second call: cache hit → gets "first answer" again (not "second answer").
        let r2 = trainer.ask("What is 2+2?")?;
        assert_eq!(r2.answer, "first answer");
        assert!(r2.cached);
        assert!(r2.backend.contains("cached"));
        Ok(())
    }

    #[test]
    fn test_cache_normalization() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["answer".into()],
            },
            cache_enabled: true,
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);

        let _ = trainer.ask("What is  2+2?")?; // Double space
        let r2 = trainer.ask("what is 2+2?")?;  // Different case
        assert!(r2.cached, "Normalized cache should match despite case/whitespace");
        Ok(())
    }

    #[test]
    fn test_cache_disabled() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["a".into(), "b".into()],
            },
            cache_enabled: false,
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);

        let r1 = trainer.ask("test")?;
        assert_eq!(r1.answer, "a");
        let r2 = trainer.ask("test")?;
        assert_eq!(r2.answer, "b"); // No cache → gets next mock answer
        assert!(!r2.cached);
        Ok(())
    }

    // ================================================================
    // Error Taxonomy Tests
    // ================================================================

    #[test]
    fn test_error_classification_refusal() {
        let kind = ErrorKind::classify("I don't know", "5", "what is 2+3?");
        assert_eq!(kind, ErrorKind::Refusal);
    }

    #[test]
    fn test_error_classification_off_topic() {
        let kind = ErrorKind::classify(
            "The weather today is sunny and warm with temperatures reaching 75 degrees",
            "5",
            "what is 2+3?",
        );
        assert_eq!(kind, ErrorKind::OffTopic);
    }

    #[test]
    fn test_error_classification_format_mismatch() {
        let kind = ErrorKind::classify("five", "5", "what is 2+3?");
        assert_eq!(kind, ErrorKind::FormatMismatch);
    }

    #[test]
    fn test_error_classification_factual() {
        let kind = ErrorKind::classify("7", "5", "what is 2+3?");
        assert_eq!(kind, ErrorKind::FactualError);
    }

    // ================================================================
    // Progressive Model Router Tests
    // ================================================================

    #[test]
    fn test_model_router_selects_by_difficulty() {
        let router = ModelRouter {
            lightweight_backend: InferenceBackend::Mock { answers: vec!["light".into()] },
            heavyweight_backend: InferenceBackend::Mock { answers: vec!["heavy".into()] },
            difficulty_threshold: 0.5,
            enabled: true,
        };

        // Easy question → lightweight
        let backend = router.route(0.2);
        assert!(matches!(backend, InferenceBackend::Mock { answers } if answers[0] == "light"));

        // Hard question → heavyweight
        let backend = router.route(0.8);
        assert!(matches!(backend, InferenceBackend::Mock { answers } if answers[0] == "heavy"));
    }

    #[test]
    fn test_model_router_disabled_uses_lightweight() {
        let router = ModelRouter {
            lightweight_backend: InferenceBackend::Mock { answers: vec!["light".into()] },
            heavyweight_backend: InferenceBackend::Mock { answers: vec!["heavy".into()] },
            difficulty_threshold: 0.5,
            enabled: false,
        };

        // Even hard questions use lightweight when disabled.
        let backend = router.route(0.9);
        assert!(matches!(backend, InferenceBackend::Mock { answers } if answers[0] == "light"));
    }

    #[test]
    fn test_trainer_with_router() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock { answers: vec!["fallback".into()] },
            ..Default::default()
        };
        let router = ModelRouter {
            lightweight_backend: InferenceBackend::Mock { answers: vec!["easy answer".into()] },
            heavyweight_backend: InferenceBackend::Mock { answers: vec!["hard answer".into()] },
            difficulty_threshold: 0.5,
            enabled: true,
        };
        let mut trainer = InferenceTrainer::with_router(config, router);

        let easy = trainer.ask_with_difficulty("easy question", Some(0.2))?;
        assert_eq!(easy.answer, "easy answer");

        let hard = trainer.ask_with_difficulty("hard question", Some(0.8))?;
        assert_eq!(hard.answer, "hard answer");
        Ok(())
    }

    // ================================================================
    // Active Learning Tests
    // ================================================================

    #[test]
    fn test_active_learning_prioritizes_weak_domains() {
        let knowledge = KnowledgeEngine::new();
        // Use domains not in seeded knowledge so mastery = 0.0.
        // Give domain_a heavy error history AND sweet-spot difficulty AND cross-domain tags.
        let error_history: HashMap<String, Vec<ErrorKind>> = {
            let mut h = HashMap::new();
            h.insert("quantum_chromodynamics".into(), vec![
                ErrorKind::FactualError; 10
            ]);
            h
        };

        let examples = vec![
            TrainingExample {
                domain: "quantum_chromodynamics".into(),
                input: "What are the three color charges?".into(),
                expected_output: "red green blue".into(),
                difficulty: 0.05, // Near mastery=0.0 → sweet spot
                tags: vec!["physics".into(), "particle".into()], // 2 tags → cross-domain
            },
            TrainingExample {
                domain: "underwater_basket_weaving".into(),
                input: "Name a technique".into(),
                expected_output: "coil weaving".into(),
                difficulty: 0.9, // Far from mastery=0.0 → no sweet spot
                tags: vec!["crafts".into()], // 1 tag → no cross-domain bonus
            },
        ];

        let order = ActiveLearner::prioritize(&examples, &knowledge, &error_history);
        // quantum_chromodynamics: errors(0.27) + sweet_spot(0.5) + cross_domain(0.2) = 1.97
        // basket_weaving: no errors + no sweet spot + no cross-domain = 1.0
        assert_eq!(order[0], 0, "Error-prone + sweet-spot domain should be prioritized");
    }

    // ================================================================
    // Multi-Model Ensemble Tests
    // ================================================================

    #[test]
    fn test_ensemble_consensus() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock { answers: vec!["unused".into()] },
            cache_enabled: false,
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);

        let ensemble = EnsembleInference::new(vec![
            InferenceBackend::Mock { answers: vec!["the answer is 5".into()] },
            InferenceBackend::Mock { answers: vec!["5 is the answer".into()] },
            InferenceBackend::Mock { answers: vec!["wrong answer entirely".into()] },
        ]);

        let result = ensemble.ask_ensemble("2+3", &mut trainer)?;
        // Two answers agree on "5" → consensus.
        assert!(result.backend.contains("consensus"), "Should reach consensus, got: {}", result.backend);
        Ok(())
    }

    // ================================================================
    // Training Report Tests
    // ================================================================

    #[test]
    fn test_training_report() {
        let result = InferenceTrainingResult {
            total_questions: 10,
            correct_answers: 7,
            accuracy: 0.7,
            results: vec![],
            cache_hit_rate: 0.3,
            error_breakdown: {
                let mut m = HashMap::new();
                m.insert("FactualError".into(), 2);
                m.insert("Refusal".into(), 1);
                m
            },
        };
        let report = result.report();
        assert!(report.contains("70.0%"));
        assert!(report.contains("FactualError"));
        assert!(report.contains("30.0%")); // Cache hit rate
    }

    // ================================================================
    // Weakest Domains Test
    // ================================================================

    #[test]
    fn test_weakest_domains() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["wrong".into()],
            },
            cache_enabled: false,
            ..Default::default()
        };
        let mut trainer = InferenceTrainer::new(config);
        let mut knowledge = KnowledgeEngine::new();

        // Train on examples that will all be wrong.
        let examples = vec![
            TrainingExample { domain: "math".into(), input: "2+3".into(), expected_output: "5".into(), difficulty: 0.1, tags: vec![] },
            TrainingExample { domain: "math".into(), input: "3+4".into(), expected_output: "7".into(), difficulty: 0.1, tags: vec![] },
            TrainingExample { domain: "history".into(), input: "Hamlet".into(), expected_output: "Shakespeare".into(), difficulty: 0.2, tags: vec![] },
        ];

        let _ = trainer.train_all(&examples, &mut knowledge)?;
        let weak = trainer.weakest_domains(5);
        assert!(!weak.is_empty());
        // Math has 2 errors, history has 1.
        assert_eq!(weak[0].0, "math");
        assert_eq!(weak[0].1, 2);
        Ok(())
    }
}
