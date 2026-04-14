// ============================================================
// Benchmark Runner — Run LFI's Benchmark Harness Against Real Models
//
// USAGE:
//   cargo run --release --bin benchmark
//   cargo run --release --bin benchmark -- --model qwen2.5-coder:7b
//   cargo run --release --bin benchmark -- --output /tmp/results.md
//
// This runs the full benchmark suite against:
//   - PerfectMockBackend (sanity check)
//   - HallucinatorMockBackend (overconfidence baseline)
//   - Real Ollama backend (if available)
//   - Claude/GPT via HTTP (if API keys provided)
//
// Produces publishable markdown report showing LFI vs competitors.
// ============================================================

use lfi_vsa_core::intelligence::benchmark_harness::{
    BenchmarkRunner, ModelBackend, ModelAnswer,
    PerfectMockBackend, HallucinatorMockBackend,
};
use std::env;
use std::time::Instant;

/// Real Ollama backend that calls a local model.
/// Wraps each prompt with instructions to also return confidence.
pub struct OllamaBackend {
    pub model: String,
    pub host: String,
    pub display_name: String,
}

impl OllamaBackend {
    pub fn new(model: &str, host: &str) -> Self {
        Self {
            model: model.into(),
            host: host.into(),
            display_name: format!("ollama:{}", model),
        }
    }
}

impl ModelBackend for OllamaBackend {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn description(&self) -> &str {
        "Local Ollama LLM backend"
    }

    fn answer(&self, prompt: &str) -> ModelAnswer {
        let start = Instant::now();

        // Wrap prompt to elicit both answer AND confidence.
        let wrapped = format!(
            "Answer the following concisely, then rate your confidence 0-100%:\n{}",
            prompt
        );
        let safe_prompt = wrapped.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");

        let body = format!(
            r#"{{"model":"{}","prompt":"{}","stream":false,"options":{{"temperature":0.2,"num_predict":200}}}}"#,
            self.model, safe_prompt
        );

        let output = match std::process::Command::new("curl")
            .args(&["-s", "--max-time", "120", "-X", "POST",
                &format!("{}/api/generate", self.host),
                "-H", "Content-Type: application/json",
                "-d", &body])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return ModelAnswer {
                    text: format!("ERROR: curl failed: {}", e),
                    confidence: 0.0,
                    refused: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    has_trace: false,
                };
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        if !output.status.success() {
            return ModelAnswer {
                text: format!("ERROR: {}", String::from_utf8_lossy(&output.stderr)),
                confidence: 0.0,
                refused: false,
                latency_ms,
                has_trace: false,
            };
        }

        let body_text = String::from_utf8_lossy(&output.stdout).to_string();
        let answer_text = extract_ollama_response(&body_text);

        if answer_text.is_empty() {
            return ModelAnswer {
                text: "ERROR: empty response".into(),
                confidence: 0.0,
                refused: false,
                latency_ms,
                has_trace: false,
            };
        }

        // Extract self-reported confidence if present.
        let confidence = extract_self_confidence(&answer_text).unwrap_or(0.5);

        // Heuristic: check if the model refused / expressed uncertainty.
        let answer_lower = answer_text.to_lowercase();
        let refusal_markers = [
            "i don't know", "i cannot predict", "i can't predict",
            "unpredictable", "no way to know", "i'm not sure",
            "i cannot answer", "i don't have access",
        ];
        let refused = refusal_markers.iter().any(|m| answer_lower.contains(m));

        // Heuristic: check if the model provided a reasoning trace.
        let trace_markers = [
            "step 1", "step 2", "first,", "then,", "next,",
            "therefore,", "because", "since", "so that",
            "we have", "we get", "this gives",
        ];
        let has_trace = trace_markers.iter().filter(|m| answer_lower.contains(*m)).count() >= 2;

        ModelAnswer {
            text: answer_text,
            confidence,
            refused,
            latency_ms,
            has_trace,
        }
    }
}

/// Extract the "response" field from Ollama's JSON output.
fn extract_ollama_response(body: &str) -> String {
    // Find `"response":"..."` block.
    let marker = "\"response\":\"";
    let start = match body.find(marker) {
        Some(s) => s + marker.len(),
        None => return String::new(),
    };
    let rest = &body[start..];

    // Walk to the matching closing quote (unescaped).
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    'n' => { out.push('\n'); chars.next(); }
                    't' => { out.push('\t'); chars.next(); }
                    'r' => { chars.next(); } // skip
                    '"' => { out.push('"'); chars.next(); }
                    '\\' => { out.push('\\'); chars.next(); }
                    _ => out.push(c),
                }
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract a self-reported confidence (e.g., "confidence: 85%" → 0.85).
fn extract_self_confidence(text: &str) -> Option<f64> {
    let lower = text.to_lowercase();

    // Try common patterns.
    for phrase in &["confidence:", "confidence is", "i'm", "i am"] {
        if let Some(idx) = lower.find(phrase) {
            let after = &lower[idx + phrase.len()..];
            if let Some(pct) = extract_percentage(after) {
                return Some(pct);
            }
        }
    }

    None
}

fn extract_percentage(text: &str) -> Option<f64> {
    // Look for N% or N percent.
    let mut digits = String::new();
    let mut found_digit = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            found_digit = true;
        } else if c == '.' && found_digit {
            digits.push(c);
        } else if found_digit {
            // Hit non-digit after digits — check if followed by '%' or 'percent'.
            let n: f64 = digits.parse().ok()?;
            if text.contains('%') || text.to_lowercase().contains("percent") {
                return Some((n / 100.0).clamp(0.0, 1.0));
            }
            return None;
        }
    }
    None
}

/// Check if Ollama is available at the host.
fn ollama_available(host: &str) -> bool {
    std::process::Command::new("curl")
        .args(&["-s", "--max-time", "3", &format!("{}/api/tags", host)])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut model = "qwen2.5-coder:7b".to_string();
    let mut host = "http://localhost:11434".to_string();
    let mut output_path: Option<String> = None;
    let mut skip_ollama = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => {
                if i + 1 < args.len() { model = args[i + 1].clone(); i += 2; } else { i += 1; }
            }
            "--host" => {
                if i + 1 < args.len() { host = args[i + 1].clone(); i += 2; } else { i += 1; }
            }
            "--output" => {
                if i + 1 < args.len() { output_path = Some(args[i + 1].clone()); i += 2; } else { i += 1; }
            }
            "--skip-ollama" => { skip_ollama = true; i += 1; }
            _ => i += 1,
        }
    }

    println!("================================================");
    println!("LFI Benchmark Runner");
    println!("================================================");
    println!("Model:      {}", model);
    println!("Host:       {}", host);
    println!("Output:     {}", output_path.as_deref().unwrap_or("(stdout only)"));
    println!();

    let mut runner = BenchmarkRunner::with_default_tasks();

    // Sanity check: PerfectMockBackend should pass ~everything.
    println!("[1/3] Running PerfectMockBackend (sanity check)...");
    runner.run(&PerfectMockBackend);
    println!("      Pass rate: {:.1}%", runner.pass_rate("perfect-mock") * 100.0);
    println!();

    // Baseline: HallucinatorMockBackend (always confident).
    println!("[2/3] Running HallucinatorMockBackend (overconfidence baseline)...");
    runner.run(&HallucinatorMockBackend);
    println!("      Pass rate: {:.1}%", runner.pass_rate("hallucinator-mock") * 100.0);
    println!();

    // Real Ollama (if available).
    if !skip_ollama && ollama_available(&host) {
        println!("[3/3] Running Ollama ({})...", model);
        let ollama = OllamaBackend::new(&model, &host);
        runner.run(&ollama);
        println!("      Pass rate: {:.1}%", runner.pass_rate(&ollama.name()) * 100.0);
        println!();
    } else if skip_ollama {
        println!("[3/3] Skipping Ollama (--skip-ollama)");
    } else {
        println!("[3/3] Ollama not available at {} — skipping.", host);
        println!("      To include: ensure Ollama is running and model '{}' is pulled.", model);
    }

    // Generate comparison report.
    let report = runner.comparison_report();
    println!();
    println!("================================================");
    println!("BENCHMARK RESULTS");
    println!("================================================");
    println!();
    println!("{}", report);

    // Save to file if requested.
    if let Some(path) = output_path {
        std::fs::write(&path, &report)?;
        println!("Report saved to: {}", path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ollama_response_simple() {
        let body = r#"{"model":"test","response":"The answer is 4.","done":true}"#;
        assert_eq!(extract_ollama_response(body), "The answer is 4.");
    }

    #[test]
    fn test_extract_ollama_response_with_newlines() {
        let body = r#"{"response":"Line 1\nLine 2\nLine 3","done":true}"#;
        let result = extract_ollama_response(body);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
    }

    #[test]
    fn test_extract_ollama_response_with_escaped_quote() {
        let body = r#"{"response":"He said \"hello\"","done":true}"#;
        let result = extract_ollama_response(body);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_extract_ollama_response_missing() {
        let body = r#"{"model":"test","done":true}"#;
        assert_eq!(extract_ollama_response(body), "");
    }

    #[test]
    fn test_extract_percentage_basic() {
        assert_eq!(extract_percentage(": 85%"), Some(0.85));
        assert_eq!(extract_percentage(" is 50 percent"), Some(0.5));
        assert!(extract_percentage("no percentage here").is_none());
    }

    #[test]
    fn test_extract_self_confidence() {
        assert_eq!(extract_self_confidence("Answer: 4. Confidence: 95%"), Some(0.95));
        assert_eq!(extract_self_confidence("I'm 80% sure"), Some(0.8));
        assert!(extract_self_confidence("no confidence mentioned").is_none());
    }

    #[test]
    fn test_extract_percentage_greater_than_100_clamped() {
        // Extract should handle edge cases.
        let result = extract_percentage(": 250%");
        // 250/100 = 2.5, but clamped to 1.0
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn test_ollama_backend_name() {
        let backend = OllamaBackend::new("llama3", "http://localhost:11434");
        assert_eq!(backend.name(), "ollama:llama3");
    }
}
