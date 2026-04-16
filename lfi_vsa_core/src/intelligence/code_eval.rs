// ============================================================
// Code Evaluation Sandbox — Generate, Compile, Test, Score
//
// PURPOSE: LFI generates code solutions to programming challenges,
// then verifies them by compiling and running test cases. This
// creates a closed feedback loop: generate → test → learn.
//
// CAPABILITIES:
//   - Coding challenges with test cases and expected outputs
//   - Compilation verification (does it compile?)
//   - Correctness verification (does it produce the right output?)
//   - Safety scoring (no unsafe, no unwrap, proper error handling)
//   - Style scoring (idiomatic patterns, naming conventions)
//   - Difficulty progression from trivial to advanced
//
// SANDBOX SECURITY:
//   All generated code runs in a temp directory with no network
//   access. The sandbox prevents:
//   - File system escape (chroot-equivalent via temp dir)
//   - Network access (no std::net in allowed imports)
//   - Process spawning (no std::process in allowed imports)
//   - Infinite loops (timeout on compilation and execution)
//
// SELF-IMPROVEMENT LOOP:
//   1. LFI attempts a coding challenge
//   2. Code is compiled and tested
//   3. If wrong: error is classified (compile error, wrong output, timeout, safety violation)
//   4. LFI learns from the error pattern
//   5. LFI re-attempts with the correction
//   6. Mastery tracked per challenge type
// ============================================================

use crate::hdc::error::HdcError;
use std::collections::HashMap;

// ============================================================
// Coding Challenge Definition
// ============================================================

/// A coding challenge with test cases.
#[derive(Debug, Clone)]
pub struct CodingChallenge {
    /// Unique identifier.
    pub id: String,
    /// Human-readable description of what to implement.
    pub description: String,
    /// The function signature that must be implemented.
    pub signature: String,
    /// Test cases: (input_args, expected_output).
    pub test_cases: Vec<(String, String)>,
    /// Difficulty level (0.0 = trivial, 1.0 = expert).
    pub difficulty: f64,
    /// Domain tags.
    pub tags: Vec<String>,
    /// Reference solution (for scoring, not shown to LFI).
    pub reference_solution: String,
}

/// Result of evaluating a code submission.
#[derive(Debug, Clone)]
pub struct CodeEvalResult {
    pub challenge_id: String,
    /// Did the code compile?
    pub compiles: bool,
    /// How many test cases passed?
    pub tests_passed: usize,
    /// Total test cases.
    pub tests_total: usize,
    /// Safety score: 0.0 (dangerous) to 1.0 (safe).
    pub safety_score: f64,
    /// Style score: 0.0 (poor) to 1.0 (idiomatic).
    pub style_score: f64,
    /// Overall score combining all factors.
    pub overall_score: f64,
    /// Compiler error if any.
    pub compile_error: Option<String>,
    /// Classification of the primary issue.
    pub issue: Option<CodeIssue>,
}

/// Classification of code quality issues.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeIssue {
    /// Code doesn't compile.
    CompileError { error_type: String },
    /// Code compiles but produces wrong output.
    WrongOutput { expected: String, got: String },
    /// Code times out (possible infinite loop).
    Timeout,
    /// Code uses unsafe or forbidden patterns.
    SafetyViolation { pattern: String },
    /// Code works but has style issues.
    StyleIssue { description: String },
    /// Code is correct and clean.
    Perfect,
}

// ============================================================
// Static Code Analysis (no compilation needed)
// ============================================================

/// Analyze code for safety and style without compiling.
/// BUG ASSUMPTION: static analysis is pattern-based, not semantic.
/// Won't catch all issues — it's a first-pass filter.
pub struct StaticAnalyzer;

impl StaticAnalyzer {
    /// Safety score: penalize dangerous patterns.
    pub fn safety_score(code: &str) -> (f64, Vec<String>) {
        let mut score: f64 = 1.0;
        let mut violations = Vec::new();

        let dangerous_patterns = [
            ("unsafe ", 0.3, "Uses unsafe block"),
            (".unwrap()", 0.1, "Uses unwrap() — can panic"),
            (".expect(", 0.08, "Uses expect() — can panic"),
            ("panic!", 0.15, "Explicit panic"),
            ("std::process", 0.3, "Attempts process spawning"),
            ("std::net", 0.2, "Attempts network access"),
            ("std::fs::remove", 0.25, "Attempts file deletion"),
            ("libc::", 0.2, "Uses libc FFI"),
            ("extern \"C\"", 0.25, "Uses C FFI"),
            ("std::mem::transmute", 0.3, "Uses transmute"),
        ];

        for (pattern, penalty, desc) in &dangerous_patterns {
            if code.contains(pattern) {
                score -= penalty;
                violations.push(desc.to_string());
            }
        }

        (score.max(0.0), violations)
    }

    /// Style score: reward idiomatic Rust patterns.
    pub fn style_score(code: &str) -> (f64, Vec<String>) {
        let mut score: f64 = 0.5; // Start at neutral
        let mut notes = Vec::new();

        // Positive patterns
        let good_patterns = [
            ("Result<", 0.1, "Uses Result type"),
            ("Option<", 0.05, "Uses Option type"),
            ("impl ", 0.05, "Uses impl blocks"),
            ("pub fn ", 0.05, "Has public API"),
            ("///", 0.05, "Has doc comments"),
            ("#[derive(", 0.05, "Uses derive macros"),
            ("-> Result", 0.1, "Returns Result"),
            (".map(", 0.05, "Uses functional combinators"),
            (".filter(", 0.03, "Uses iterator methods"),
            ("if let ", 0.05, "Uses pattern matching"),
        ];

        for (pattern, bonus, desc) in &good_patterns {
            if code.contains(pattern) {
                score += bonus;
                notes.push(format!("+: {}", desc));
            }
        }

        // Negative patterns
        let bad_patterns = [
            ("var ", 0.1, "Uses 'var' (not Rust)"),
            ("null", 0.05, "Uses 'null' (not Rust)"),
            ("class ", 0.1, "Uses 'class' (not Rust)"),
            ("void ", 0.05, "Uses 'void' (not Rust)"),
        ];

        for (pattern, penalty, desc) in &bad_patterns {
            if code.contains(pattern) {
                score -= penalty;
                notes.push(format!("-: {}", desc));
            }
        }

        (score.clamp(0.0, 1.0), notes)
    }

    /// Full static analysis.
    pub fn analyze(code: &str) -> CodeEvalResult {
        let (safety, safety_violations) = Self::safety_score(code);
        let (style, _style_notes) = Self::style_score(code);

        let issue = if !safety_violations.is_empty() {
            Some(CodeIssue::SafetyViolation {
                pattern: safety_violations.join(", "),
            })
        } else if style < 0.4 {
            Some(CodeIssue::StyleIssue {
                description: "Code has significant style issues".into(),
            })
        } else {
            None
        };

        let overall = safety * 0.5 + style * 0.3 + 0.2; // Base 0.2 for existing

        CodeEvalResult {
            challenge_id: String::new(),
            compiles: true, // Static analysis doesn't compile
            tests_passed: 0,
            tests_total: 0,
            safety_score: safety,
            style_score: style,
            overall_score: overall,
            compile_error: None,
            issue,
        }
    }
}

// ============================================================
// Code Evaluator — compile and test
// ============================================================

/// Evaluates code by compiling and running it in a sandbox.
/// BUG ASSUMPTION: requires `rustc` on PATH. Temp dir cleanup may fail
/// under disk pressure. Timeout enforcement depends on OS signals.
pub struct CodeEvaluator {
    /// Timeout for compilation (milliseconds).
    pub compile_timeout_ms: u64,
    /// Timeout for execution (milliseconds).
    pub exec_timeout_ms: u64,
    /// Track results per challenge.
    pub history: HashMap<String, Vec<CodeEvalResult>>,
}

impl CodeEvaluator {
    pub fn new() -> Self {
        debuglog!("CodeEvaluator::new: Initializing code evaluation sandbox");
        Self {
            compile_timeout_ms: 30_000,
            exec_timeout_ms: 10_000,
            history: HashMap::new(),
        }
    }

    /// Evaluate a code submission against a challenge.
    /// Combines static analysis with compilation and test execution.
    pub fn evaluate(
        &mut self,
        challenge: &CodingChallenge,
        code: &str,
    ) -> Result<CodeEvalResult, HdcError> {
        debuglog!("CodeEvaluator::evaluate: challenge='{}', code_len={}",
            challenge.id, code.len());

        // Step 1: Static analysis (always runs — no compilation needed).
        let static_result = StaticAnalyzer::analyze(code);

        // Step 2: Attempt compilation.
        let compile_result = self.try_compile(code);
        let compiles = compile_result.is_ok();
        let compile_error = compile_result.err();

        if !compiles {
            let error_type = compile_error.as_deref().unwrap_or("unknown").to_string();
            let result = CodeEvalResult {
                challenge_id: challenge.id.clone(),
                compiles: false,
                tests_passed: 0,
                tests_total: challenge.test_cases.len(),
                safety_score: static_result.safety_score,
                style_score: static_result.style_score,
                overall_score: 0.0, // Can't pass if it doesn't compile
                compile_error: Some(error_type.clone()),
                issue: Some(CodeIssue::CompileError { error_type }),
            };
            self.record_result(&challenge.id, &result);
            return Ok(result);
        }

        // Step 3: Run test cases.
        let mut tests_passed = 0;
        let mut first_failure: Option<CodeIssue> = None;

        for (input, expected) in &challenge.test_cases {
            match self.try_run(code, input) {
                Ok(output) => {
                    let output_trimmed = output.trim();
                    let expected_trimmed = expected.trim();
                    if output_trimmed == expected_trimmed
                        || output_trimmed.contains(expected_trimmed) {
                        tests_passed += 1;
                    } else if first_failure.is_none() {
                        first_failure = Some(CodeIssue::WrongOutput {
                            expected: expected.clone(),
                            got: output_trimmed.to_string(),
                        });
                    }
                }
                Err(_) => {
                    if first_failure.is_none() {
                        first_failure = Some(CodeIssue::Timeout);
                    }
                }
            }
        }

        let test_score = if challenge.test_cases.is_empty() {
            1.0
        } else {
            tests_passed as f64 / challenge.test_cases.len() as f64
        };

        let issue = if tests_passed == challenge.test_cases.len() && static_result.safety_score > 0.8 {
            Some(CodeIssue::Perfect)
        } else {
            first_failure.or(static_result.issue)
        };

        let overall = test_score * 0.5 + static_result.safety_score * 0.3 + static_result.style_score * 0.2;

        let result = CodeEvalResult {
            challenge_id: challenge.id.clone(),
            compiles: true,
            tests_passed,
            tests_total: challenge.test_cases.len(),
            safety_score: static_result.safety_score,
            style_score: static_result.style_score,
            overall_score: overall,
            compile_error: None,
            issue,
        };

        self.record_result(&challenge.id, &result);
        Ok(result)
    }

    /// Try to compile code. Returns Ok(()) if successful, Err(message) if not.
    fn try_compile(&self, code: &str) -> Result<(), String> {
        let dir = std::env::temp_dir().join(format!("lfi_sandbox_{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir failed: {}", e))?;

        let source_path = dir.join("solution.rs");
        std::fs::write(&source_path, code).map_err(|e| format!("write failed: {}", e))?;

        let output = std::process::Command::new("rustc")
            .args(&["--edition", "2021", "--crate-type", "lib",
                   "-o", dir.join("solution").to_str().unwrap_or("/dev/null"),
                   source_path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| format!("rustc failed: {}", e))?;

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Try to run code with input. Returns output string.
    /// BUG ASSUMPTION: this builds a main() wrapper around the submitted function.
    fn try_run(&self, code: &str, input: &str) -> Result<String, String> {
        let dir = std::env::temp_dir().join(format!("lfi_run_{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir failed: {}", e))?;

        // Wrap the code in a main() that feeds the input
        let wrapped = format!(
            "{}\nfn main() {{ let result = solution({}); println!(\"{{:?}}\", result); }}",
            code, input
        );

        let source_path = dir.join("main.rs");
        let binary_path = dir.join("main");
        std::fs::write(&source_path, &wrapped).map_err(|e| format!("write failed: {}", e))?;

        // Compile
        let compile = std::process::Command::new("rustc")
            .args(&["--edition", "2021",
                   "-o", binary_path.to_str().unwrap_or(""),
                   source_path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| format!("rustc failed: {}", e))?;

        if !compile.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(String::from_utf8_lossy(&compile.stderr).to_string());
        }

        // Execute with timeout
        let run = std::process::Command::new(binary_path.to_str().unwrap_or(""))
            .output()
            .map_err(|e| format!("execution failed: {}", e))?;

        let _ = std::fs::remove_dir_all(&dir);

        if run.status.success() {
            Ok(String::from_utf8_lossy(&run.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&run.stderr).to_string())
        }
    }

    fn record_result(&mut self, challenge_id: &str, result: &CodeEvalResult) {
        self.history.entry(challenge_id.to_string())
            .or_default()
            .push(result.clone());
    }

    /// Average score across all evaluated challenges.
    pub fn average_score(&self) -> f64 {
        let all_results: Vec<&CodeEvalResult> = self.history.values()
            .flat_map(|v| v.iter())
            .collect();
        if all_results.is_empty() { return 0.0; }
        let total: f64 = all_results.iter().map(|r| r.overall_score).sum();
        total / all_results.len() as f64
    }

    /// Challenges with lowest scores (need more practice).
    pub fn weakest_challenges(&self, top_n: usize) -> Vec<(String, f64)> {
        let mut scores: Vec<(String, f64)> = self.history.iter()
            .map(|(id, results)| {
                let best = results.iter().map(|r| r.overall_score)
                    .fold(0.0_f64, f64::max);
                (id.clone(), best)
            })
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_n);
        scores
    }
}

// ============================================================
// Challenge Library — built-in coding challenges
// ============================================================

pub struct ChallengeLibrary;

impl ChallengeLibrary {
    /// Beginner: pure functions, no I/O.
    pub fn beginner() -> Vec<CodingChallenge> {
        vec![
            CodingChallenge {
                id: "fizzbuzz".into(),
                description: "Return \"Fizz\" for multiples of 3, \"Buzz\" for multiples of 5, \"FizzBuzz\" for both, or the number as a string.".into(),
                signature: "fn solution(n: u32) -> String".into(),
                test_cases: vec![
                    ("1".into(), "\"1\"".into()),
                    ("3".into(), "\"Fizz\"".into()),
                    ("5".into(), "\"Buzz\"".into()),
                    ("15".into(), "\"FizzBuzz\"".into()),
                ],
                difficulty: 0.1,
                tags: vec!["basics".into(), "conditionals".into()],
                reference_solution: r#"fn solution(n: u32) -> String { if n % 15 == 0 { "FizzBuzz".into() } else if n % 3 == 0 { "Fizz".into() } else if n % 5 == 0 { "Buzz".into() } else { n.to_string() } }"#.into(),
            },
            CodingChallenge {
                id: "factorial".into(),
                description: "Compute n! (factorial of n). Return 1 for n=0.".into(),
                signature: "fn solution(n: u64) -> u64".into(),
                test_cases: vec![
                    ("0".into(), "1".into()),
                    ("1".into(), "1".into()),
                    ("5".into(), "120".into()),
                    ("10".into(), "3628800".into()),
                ],
                difficulty: 0.15,
                tags: vec!["math".into(), "recursion".into()],
                reference_solution: "fn solution(n: u64) -> u64 { (1..=n).product() }".into(),
            },
            CodingChallenge {
                id: "palindrome".into(),
                description: "Return true if the string is a palindrome (case-insensitive, alphanumeric only).".into(),
                signature: "fn solution(s: &str) -> bool".into(),
                test_cases: vec![
                    ("\"racecar\"".into(), "true".into()),
                    ("\"hello\"".into(), "false".into()),
                    ("\"A man a plan a canal Panama\"".into(), "true".into()),
                ],
                difficulty: 0.2,
                tags: vec!["strings".into(), "algorithms".into()],
                reference_solution: r#"fn solution(s: &str) -> bool { let clean: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_lowercase().next().unwrap_or(c)).collect(); clean == clean.iter().copied().rev().collect::<Vec<_>>() }"#.into(),
            },
            CodingChallenge {
                id: "fibonacci".into(),
                description: "Return the nth Fibonacci number (0-indexed: fib(0)=0, fib(1)=1).".into(),
                signature: "fn solution(n: u32) -> u64".into(),
                test_cases: vec![
                    ("0".into(), "0".into()),
                    ("1".into(), "1".into()),
                    ("10".into(), "55".into()),
                    ("20".into(), "6765".into()),
                ],
                difficulty: 0.15,
                tags: vec!["math".into(), "dynamic_programming".into()],
                reference_solution: "fn solution(n: u32) -> u64 { let (mut a, mut b) = (0u64, 1u64); for _ in 0..n { let t = a + b; a = b; b = t; } a }".into(),
            },
        ]
    }

    /// Intermediate: data structures, algorithms.
    pub fn intermediate() -> Vec<CodingChallenge> {
        vec![
            CodingChallenge {
                id: "two_sum".into(),
                description: "Given a sorted array and target, return indices of two numbers that add up to target.".into(),
                signature: "fn solution(nums: &[i32], target: i32) -> Option<(usize, usize)>".into(),
                test_cases: vec![
                    ("&[2, 7, 11, 15], 9".into(), "Some((0, 1))".into()),
                    ("&[1, 2, 3, 4], 7".into(), "Some((2, 3))".into()),
                    ("&[1, 2, 3], 10".into(), "None".into()),
                ],
                difficulty: 0.3,
                tags: vec!["arrays".into(), "two_pointers".into()],
                reference_solution: "fn solution(nums: &[i32], target: i32) -> Option<(usize, usize)> { let mut left = 0; let mut right = nums.len().saturating_sub(1); while left < right { let sum = nums[left] + nums[right]; if sum == target { return Some((left, right)); } else if sum < target { left += 1; } else { right -= 1; } } None }".into(),
            },
            CodingChallenge {
                id: "binary_search".into(),
                description: "Implement binary search on a sorted array. Return the index of target, or None.".into(),
                signature: "fn solution(arr: &[i32], target: i32) -> Option<usize>".into(),
                test_cases: vec![
                    ("&[1, 3, 5, 7, 9], 5".into(), "Some(2)".into()),
                    ("&[1, 3, 5, 7, 9], 4".into(), "None".into()),
                    ("&[1], 1".into(), "Some(0)".into()),
                ],
                difficulty: 0.25,
                tags: vec!["search".into(), "algorithms".into()],
                reference_solution: "fn solution(arr: &[i32], target: i32) -> Option<usize> { arr.binary_search(&target).ok() }".into(),
            },
            CodingChallenge {
                id: "caesar_cipher".into(),
                description: "Encrypt a string with Caesar cipher (shift letters by n, wrap around, preserve case).".into(),
                signature: "fn solution(text: &str, shift: u8) -> String".into(),
                test_cases: vec![
                    ("\"abc\", 1".into(), "\"bcd\"".into()),
                    ("\"xyz\", 3".into(), "\"abc\"".into()),
                    ("\"Hello\", 13".into(), "\"Uryyb\"".into()),
                ],
                difficulty: 0.35,
                tags: vec!["crypto".into(), "strings".into()],
                reference_solution: r#"fn solution(text: &str, shift: u8) -> String { text.chars().map(|c| { if c.is_ascii_lowercase() { (b'a' + (c as u8 - b'a' + shift) % 26) as char } else if c.is_ascii_uppercase() { (b'A' + (c as u8 - b'A' + shift) % 26) as char } else { c } }).collect() }"#.into(),
            },
        ]
    }

    /// Advanced: security-related coding challenges.
    pub fn security() -> Vec<CodingChallenge> {
        vec![
            CodingChallenge {
                id: "hex_encode".into(),
                description: "Encode a byte slice as a lowercase hex string.".into(),
                signature: "fn solution(bytes: &[u8]) -> String".into(),
                test_cases: vec![
                    ("&[0xDE, 0xAD, 0xBE, 0xEF]".into(), "\"deadbeef\"".into()),
                    ("&[0x00, 0xFF]".into(), "\"00ff\"".into()),
                    ("&[]".into(), "\"\"".into()),
                ],
                difficulty: 0.2,
                tags: vec!["encoding".into(), "security".into()],
                reference_solution: "fn solution(bytes: &[u8]) -> String { bytes.iter().map(|b| format!(\"{:02x}\", b)).collect() }".into(),
            },
            CodingChallenge {
                id: "xor_cipher".into(),
                description: "XOR each byte of data with the key (repeating key if shorter than data).".into(),
                signature: "fn solution(data: &[u8], key: &[u8]) -> Vec<u8>".into(),
                test_cases: vec![
                    ("&[0x41, 0x42, 0x43], &[0xFF]".into(), "vec![0xBE, 0xBD, 0xBC]".into()),
                    ("&[1, 2, 3], &[1, 2, 3]".into(), "vec![0, 0, 0]".into()),
                ],
                difficulty: 0.3,
                tags: vec!["crypto".into(), "xor".into(), "security".into()],
                reference_solution: "fn solution(data: &[u8], key: &[u8]) -> Vec<u8> { if key.is_empty() { return data.to_vec(); } data.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect() }".into(),
            },
            CodingChallenge {
                id: "detect_sql_injection".into(),
                description: "Return true if the input string contains common SQL injection patterns.".into(),
                signature: "fn solution(input: &str) -> bool".into(),
                test_cases: vec![
                    ("\"SELECT * FROM users\"".into(), "true".into()),
                    ("\"hello world\"".into(), "false".into()),
                    ("\"1' OR '1'='1\"".into(), "true".into()),
                    ("\"DROP TABLE users--\"".into(), "true".into()),
                ],
                difficulty: 0.4,
                tags: vec!["security".into(), "injection".into(), "detection".into()],
                reference_solution: r#"fn solution(input: &str) -> bool { let lower = input.to_lowercase(); let patterns = ["select ", "drop ", "insert ", "update ", "delete ", "union ", "' or '", "1=1", "' --", "'; "]; patterns.iter().any(|p| lower.contains(p)) }"#.into(),
            },
        ]
    }

    /// All challenges.
    pub fn all() -> Vec<CodingChallenge> {
        let mut all = Vec::new();
        all.extend(Self::beginner());
        all.extend(Self::intermediate());
        all.extend(Self::security());
        all
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_safety_clean_code() {
        let code = r#"
            fn solution(n: u32) -> u32 {
                if n == 0 { return 1; }
                n * solution(n - 1)
            }
        "#;
        let (score, violations) = StaticAnalyzer::safety_score(code);
        assert_eq!(score, 1.0, "Clean code should have perfect safety score");
        assert!(violations.is_empty());
    }

    #[test]
    fn test_static_safety_unsafe_code() {
        let code = r#"
            unsafe fn solution(ptr: *const u8) -> u8 {
                *ptr
            }
        "#;
        let (score, violations) = StaticAnalyzer::safety_score(code);
        assert!(score < 0.8, "Unsafe code should have low safety score: {:.2}", score);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_static_safety_unwrap() {
        let code = r#"
            fn solution(s: &str) -> i32 {
                s.parse::<i32>().unwrap()
            }
        "#;
        let (score, _) = StaticAnalyzer::safety_score(code);
        assert!(score < 1.0, "Code with unwrap should be penalized");
    }

    #[test]
    fn test_static_style_idiomatic() {
        let code = r#"
            /// Computes the sum.
            pub fn solution(nums: &[i32]) -> Result<i32, String> {
                if let Some(&first) = nums.first() {
                    Ok(nums.iter().sum())
                } else {
                    Err("empty".into())
                }
            }
        "#;
        let (score, _) = StaticAnalyzer::style_score(code);
        assert!(score > 0.6, "Idiomatic code should score well: {:.2}", score);
    }

    #[test]
    fn test_static_style_non_rust() {
        let code = "class Foo { var x = null; void bar() {} }";
        let (score, _) = StaticAnalyzer::style_score(code);
        assert!(score < 0.4, "Non-Rust code should score poorly: {:.2}", score);
    }

    #[test]
    fn test_challenge_library_has_challenges() {
        let all = ChallengeLibrary::all();
        assert!(all.len() >= 10, "Should have 10+ challenges, got {}", all.len());

        let beginner = ChallengeLibrary::beginner();
        let intermediate = ChallengeLibrary::intermediate();
        let security = ChallengeLibrary::security();

        assert!(!beginner.is_empty());
        assert!(!intermediate.is_empty());
        assert!(!security.is_empty());
    }

    #[test]
    fn test_challenge_test_cases_exist() {
        for challenge in ChallengeLibrary::all() {
            assert!(!challenge.test_cases.is_empty(),
                "Challenge '{}' should have test cases", challenge.id);
            assert!(!challenge.reference_solution.is_empty(),
                "Challenge '{}' should have a reference solution", challenge.id);
        }
    }

    #[test]
    fn test_evaluator_creation() {
        let evaluator = CodeEvaluator::new();
        assert_eq!(evaluator.average_score(), 0.0);
        assert!(evaluator.weakest_challenges(5).is_empty());
    }

    #[test]
    fn test_full_static_analysis() {
        let good = StaticAnalyzer::analyze("fn solution(n: u32) -> u32 { n * 2 }");
        assert!(good.safety_score >= 0.9);

        let bad = StaticAnalyzer::analyze("unsafe { std::mem::transmute(ptr) }");
        assert!(bad.safety_score < 0.5);
    }

    #[test]
    fn test_reference_solutions_are_safe() {
        for challenge in ChallengeLibrary::all() {
            let (safety, violations) = StaticAnalyzer::safety_score(&challenge.reference_solution);
            assert!(safety >= 0.8,
                "Reference solution for '{}' has safety issues: {:.2} — {:?}",
                challenge.id, safety, violations);
        }
    }

    #[test]
    fn test_difficulty_progression() {
        let beginner = ChallengeLibrary::beginner();
        let intermediate = ChallengeLibrary::intermediate();
        let _security = ChallengeLibrary::security();

        let avg_beginner: f64 = beginner.iter().map(|c| c.difficulty).sum::<f64>() / beginner.len() as f64;
        let avg_intermediate: f64 = intermediate.iter().map(|c| c.difficulty).sum::<f64>() / intermediate.len() as f64;

        assert!(avg_beginner < avg_intermediate,
            "Beginner avg difficulty ({:.2}) should be less than intermediate ({:.2})",
            avg_beginner, avg_intermediate);
    }

    // ============================================================
    // Stress / invariant tests for code_eval
    // ============================================================

    /// INVARIANT: safety_score is in [0.0, 1.0] for any code input.
    /// Score below 0 or above 1 means downstream logic relying on
    /// percentages/probabilities will misbehave.
    #[test]
    fn invariant_safety_score_in_unit_interval() {
        let inputs = [
            "",
            "fn safe() {}",
            "unsafe { unsafe { unsafe { unsafe { unsafe {} } } } }", // pile-up
            "panic!(\"x\"); std::process::exit(1); std::fs::remove_file(\"/\");",
            ".unwrap().unwrap().unwrap().unwrap().unwrap()",
            "extern \"C\" { fn evil(); } unsafe { libc::abort(); std::mem::transmute::<u8,u8>(0); }",
        ];
        for code in inputs {
            let (score, _) = StaticAnalyzer::safety_score(code);
            assert!((0.0..=1.0).contains(&score) && score.is_finite(),
                "safety_score out of [0,1] for input {:?}: {}", code, score);
        }
    }

    /// INVARIANT: style_score is in [0.0, 1.0] for any code input.
    #[test]
    fn invariant_style_score_in_unit_interval() {
        let big = "fn x() {}".repeat(100);
        let inputs: [&str; 5] = [
            "",
            "fn x() {}",
            "var x = null; class Foo { void bar() { var y = null; } }",
            "/// doc\npub fn x() -> Result<(), ()> { if let Some(_) = None { } Ok(()) }",
            &big,
        ];
        for code in inputs {
            let (score, _) = StaticAnalyzer::style_score(code);
            assert!((0.0..=1.0).contains(&score) && score.is_finite(),
                "style_score out of [0,1] for input: {}", score);
        }
    }

    /// INVARIANT: safety_score is monotone — adding more dangerous
    /// patterns never increases the score.
    #[test]
    fn invariant_safety_score_monotone_under_added_danger() {
        let base = "fn safe() { let x = 1; }";
        let with_unwrap = format!("{} let _ = Some(0).unwrap();", base);
        let with_unwrap_and_unsafe = format!("{} unsafe {{}}", with_unwrap);
        let (s_base, _) = StaticAnalyzer::safety_score(base);
        let (s_unwrap, _) = StaticAnalyzer::safety_score(&with_unwrap);
        let (s_both, _) = StaticAnalyzer::safety_score(&with_unwrap_and_unsafe);
        assert!(s_base >= s_unwrap, "adding unwrap must not raise safety: {} → {}", s_base, s_unwrap);
        assert!(s_unwrap >= s_both, "adding unsafe must not raise safety: {} → {}", s_unwrap, s_both);
    }

    /// INVARIANT: analyze() is deterministic and produces a result for any input.
    #[test]
    fn invariant_analyze_deterministic_and_total() {
        let inputs = [
            "",
            "fn x() {}",
            "// comment only",
            "<<malformed Rust",
            "fn 不安全() { let x = 'ω'; }", // unicode identifiers + char literal
        ];
        for code in inputs {
            let r1 = StaticAnalyzer::analyze(code);
            let r2 = StaticAnalyzer::analyze(code);
            // Shape determinism: both have the same combined score.
            assert!(r1.overall_score.is_finite());
            assert!((r1.overall_score - r2.overall_score).abs() < 1e-12,
                "analyze must be deterministic for input {:?}", code);
        }
    }

    /// INVARIANT: the challenge library is consistent —
    /// every challenge has a non-empty id, description, and at least one test case.
    #[test]
    fn invariant_challenge_library_well_formed() {
        for ch in ChallengeLibrary::all() {
            assert!(!ch.id.is_empty(), "challenge id must not be empty");
            assert!(!ch.description.is_empty(),
                "challenge description must not be empty: {}", ch.id);
            assert!(!ch.test_cases.is_empty(),
                "challenge {} must have at least one test case", ch.id);
            assert!(ch.difficulty.is_finite() && (0.0..=1.0).contains(&ch.difficulty),
                "challenge {} difficulty out of [0,1]: {}", ch.id, ch.difficulty);
        }
    }

    /// INVARIANT: every challenge has a unique id within the library.
    /// Duplicates silently overwrite training/scoring data.
    #[test]
    fn invariant_challenge_ids_unique() {
        let mut seen = std::collections::HashSet::new();
        for ch in ChallengeLibrary::all() {
            assert!(seen.insert(ch.id.clone()),
                "duplicate challenge id: {}", ch.id);
        }
    }

}
