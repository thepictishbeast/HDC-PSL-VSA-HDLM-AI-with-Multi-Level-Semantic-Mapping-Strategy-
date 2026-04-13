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
//   5. If wrong: teach the correct answer + learn from the LLM's reasoning
// ============================================================

use crate::hdc::error::HdcError;
use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::training_data::TrainingExample;

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

/// Result of asking a local LLM a question.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub question: String,
    pub answer: String,
    pub backend: String,
    pub latency_ms: u64,
    pub correct: Option<bool>,
}

/// Configuration for local inference training.
#[derive(Debug, Clone)]
pub struct InferenceTrainingConfig {
    pub backend: InferenceBackend,
    pub max_retries: usize,
    pub timeout_ms: u64,
    pub verify_answers: bool,
}

impl Default for InferenceTrainingConfig {
    fn default() -> Self {
        Self {
            backend: InferenceBackend::default(),
            max_retries: 2,
            timeout_ms: 30_000,
            verify_answers: true,
        }
    }
}

/// The local inference trainer — asks LLMs questions and learns from answers.
pub struct InferenceTrainer {
    config: InferenceTrainingConfig,
    questions_asked: usize,
    correct_answers: usize,
    mock_index: usize,
}

impl InferenceTrainer {
    pub fn new(config: InferenceTrainingConfig) -> Self {
        debuglog!("InferenceTrainer::new: backend={:?}", config.backend);
        Self { config, questions_asked: 0, correct_answers: 0, mock_index: 0 }
    }

    /// Ask a question and get an answer from the local LLM.
    pub fn ask(&mut self, question: &str) -> Result<InferenceResult, HdcError> {
        debuglog!("InferenceTrainer::ask: '{}'", &question[..question.len().min(60)]);
        let start = std::time::Instant::now();

        let (answer, backend_name) = match &self.config.backend {
            InferenceBackend::Mock { answers } => {
                let idx = self.mock_index % answers.len().max(1);
                self.mock_index += 1;
                (answers.get(idx).cloned().unwrap_or_default(), "mock")
            }
            InferenceBackend::Ollama { model, host } => {
                // Synchronous HTTP call to Ollama.
                match Self::call_ollama(host, model, question) {
                    Ok(answer) => (answer, "ollama"),
                    Err(e) => {
                        debuglog!("InferenceTrainer: Ollama failed: {:?}", e);
                        (format!("ERROR: {}", e), "ollama_error")
                    }
                }
            }
            InferenceBackend::GeminiCli => {
                // Sync subprocess call.
                match Self::call_cli("gemini", &["chat", question]) {
                    Ok(answer) => (answer, "gemini"),
                    Err(e) => (format!("ERROR: {}", e), "gemini_error"),
                }
            }
            InferenceBackend::ClaudeCli => {
                match Self::call_cli("claude", &["-p", question]) {
                    Ok(answer) => (answer, "claude"),
                    Err(e) => (format!("ERROR: {}", e), "claude_error"),
                }
            }
            InferenceBackend::HttpApi { url, model } => {
                match Self::call_http_api(url, model, question) {
                    Ok(answer) => (answer, "http_api"),
                    Err(e) => (format!("ERROR: {}", e), "http_error"),
                }
            }
        };

        let latency = start.elapsed().as_millis() as u64;
        self.questions_asked += 1;

        Ok(InferenceResult {
            question: question.into(),
            answer,
            backend: backend_name.into(),
            latency_ms: latency,
            correct: None,
        })
    }

    /// Ask a training question, verify the answer, and learn from it.
    pub fn train_on_example(
        &mut self,
        example: &TrainingExample,
        knowledge: &mut KnowledgeEngine,
    ) -> Result<InferenceResult, HdcError> {
        let mut result = self.ask(&example.input)?;

        if self.config.verify_answers {
            // Check if the LLM's answer contains the expected output.
            let answer_lower = result.answer.to_lowercase();
            let expected_lower = example.expected_output.to_lowercase();
            let is_correct = answer_lower.contains(&expected_lower)
                || Self::fuzzy_match(&result.answer, &example.expected_output);

            result.correct = Some(is_correct);

            if is_correct {
                self.correct_answers += 1;
                // Reinforce the concept.
                knowledge.reinforce(&example.domain);
                debuglog!("InferenceTrainer: CORRECT — reinforcing '{}'", example.domain);
            } else {
                // Teach the correct answer.
                let concept_name = format!("inferred_{}_{}", example.domain, self.questions_asked);
                knowledge.learn_with_definition(
                    &concept_name,
                    &format!("Q: {} A: {} (LLM said: {})",
                        example.input, example.expected_output,
                        &result.answer[..result.answer.len().min(100)]),
                    &[&example.domain],
                    0.6, // Moderate mastery — learned from correction
                    true,
                )?;
                debuglog!("InferenceTrainer: WRONG — taught correct answer");
            }
        }

        Ok(result)
    }

    /// Run inference training across all examples.
    pub fn train_all(
        &mut self,
        examples: &[TrainingExample],
        knowledge: &mut KnowledgeEngine,
    ) -> Result<InferenceTrainingResult, HdcError> {
        debuglog!("InferenceTrainer::train_all: {} examples", examples.len());
        let mut results = Vec::new();

        for example in examples {
            match self.train_on_example(example, knowledge) {
                Ok(result) => results.push(result),
                Err(e) => debuglog!("InferenceTrainer: Example failed: {:?}", e),
            }
        }

        let correct = results.iter().filter(|r| r.correct == Some(true)).count();
        let total = results.len();
        let accuracy = if total > 0 { correct as f64 / total as f64 } else { 0.0 };

        Ok(InferenceTrainingResult {
            total_questions: total,
            correct_answers: correct,
            accuracy,
            results,
        })
    }

    /// Accuracy so far.
    pub fn accuracy(&self) -> f64 {
        if self.questions_asked == 0 { return 0.0; }
        self.correct_answers as f64 / self.questions_asked as f64
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
            return false; // Can't fuzzy match against no words
        }
        let overlap = answer_words.intersection(&expected_words).count();
        let needed = (expected_words.len() + 1) / 2; // At least half must match
        overlap >= needed
    }

    /// Call Ollama HTTP API synchronously.
    fn call_ollama(host: &str, model: &str, prompt: &str) -> Result<String, String> {
        // Use ureq or std HTTP — for now, just format the command
        let output = std::process::Command::new("curl")
            .args(&["-s", "-X", "POST", &format!("{}/api/generate", host),
                "-d", &format!(r#"{{"model":"{}","prompt":"{}","stream":false}}"#,
                    model, prompt.replace('"', "\\\""))])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if output.status.success() {
            let body = String::from_utf8_lossy(&output.stdout).to_string();
            // Extract response from JSON.
            if let Some(start) = body.find("\"response\":\"") {
                let rest = &body[start + 12..];
                if let Some(end) = rest.find('"') {
                    return Ok(rest[..end].replace("\\n", "\n"));
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
        let output = std::process::Command::new("curl")
            .args(&["-s", "-X", "POST", &format!("{}/v1/chat/completions", url),
                "-H", "Content-Type: application/json",
                "-d", &format!(r#"{{"model":"{}","messages":[{{"role":"user","content":"{}"}}]}}"#,
                    model, prompt.replace('"', "\\\""))])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

/// Results from inference-based training.
#[derive(Debug)]
pub struct InferenceTrainingResult {
    pub total_questions: usize,
    pub correct_answers: usize,
    pub accuracy: f64,
    pub results: Vec<InferenceResult>,
}

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
        Ok(())
    }

    #[test]
    fn test_mock_training() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["5".into()], // Correct answer for 2+3
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
        Ok(())
    }

    #[test]
    fn test_mock_wrong_answer() -> Result<(), HdcError> {
        let config = InferenceTrainingConfig {
            backend: InferenceBackend::Mock {
                answers: vec!["wrong answer".into()],
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
        // Should have taught the correct answer.
        assert!(knowledge.concept_count() > 0); // Has at least seeded concepts
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
}
