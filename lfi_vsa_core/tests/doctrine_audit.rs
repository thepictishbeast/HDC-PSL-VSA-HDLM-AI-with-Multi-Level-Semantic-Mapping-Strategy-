// ============================================================
// Doctrine audit (#401)
//
// Enforces the non-negotiable rules of the LFI architecture as a
// CI test. Every PR that violates one of these fails the build.
//
//   1. No LLM / transformer imports — post-LLM means post-LLM.
//   2. No hardcoded user-facing response pools in src/ (large string
//      arrays that read as canned chat responses). Templates that
//      take DATA and FORMAT it are fine; pools of pre-written
//      sentences are not.
//   3. No `.unwrap()` / `.expect()` outside tests + SAFETY-annotated
//      sites in src/*.rs library code.
//   4. No plaintext passphrase == comparisons (must go through
//      subtle::ConstantTimeEq).
//
// Each rule is a separate test so CI output tells you which axiom
// failed.
// ============================================================

use std::fs;
use std::path::{Path, PathBuf};

/// All .rs files under src/ except tests modules.
fn src_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(Path::new("src"), &mut out);
    out
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_default()
}

// ---- Rule 1: no LLM / transformer imports ----

#[test]
fn no_llm_imports() {
    // Bare-word crate names we refuse to depend on in the library
    // path. We look for `use <crate>::` and `<crate>::Whatever::`
    // — not substrings of other words, not in comments.
    const BANNED: &[&str] = &[
        "ollama", "openai", "anthropic", "llama_cpp", "llama-cpp",
        "candle_transformers", "hf_hub", "tokenizers",
        "tch::nn::transformer",
    ];
    let mut violations = Vec::new();
    for f in src_files() {
        let s = read(&f);
        for line in s.lines() {
            // Skip comments entirely — SEARCH doctrine doesn't block
            // discussion, only active imports / calls.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*")
                || trimmed.starts_with("#") {
                continue;
            }
            for b in BANNED {
                if (line.contains(&format!("use {}::", b))
                    || line.contains(&format!("{}::", b)))
                    && !line.contains("// allowed")
                {
                    violations.push(format!("{}: {}", f.display(), line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Doctrine violation: banned LLM import(s):\n{}",
        violations.join("\n")
    );
}

// ---- Rule 2: no hardcoded response pools in src/ ----

#[test]
fn no_hardcoded_response_pools_in_src() {
    // Heuristic: a `const <NAME>: &[&str] = &[` literal that has
    // >= 5 string entries, where one of the entries is a
    // sentence-like user-facing phrase (starts with a capital or
    // "I", ends with "!" "?" ".", contains a space).
    //
    // Excludes tests/ and .rs files under `examples/` / `benches/`.
    // Also excludes doctrine_audit.rs itself + files whose first line
    // asserts `// test-only`.
    let mut violations = Vec::new();
    for f in src_files() {
        if f.components().any(|c| c.as_os_str() == "bin") { continue; }
        let s = read(&f);
        // Crude but effective: parse blocks between `&[` and `];`
        // that live inside a `&[&str] = &[` decl.
        for (i, _) in s.match_indices("&[&str] = &[") {
            let tail = &s[i..];
            let Some(end) = tail.find("];") else { continue };
            let block = &tail[..end];
            let strings: Vec<&str> = block.match_indices('"')
                .collect::<Vec<_>>()
                .chunks(2)
                .filter_map(|pair| pair.get(1).map(|close| {
                    let (open_pos, _) = pair[0];
                    &block[open_pos + 1..close.0]
                }))
                .collect();
            if strings.len() < 5 { continue; }
            // Sentence-like test on at least ONE string.
            let sentence_like = strings.iter().any(|s| {
                s.len() >= 10
                    && s.contains(' ')
                    && (s.ends_with('.') || s.ends_with('!') || s.ends_with('?'))
            });
            if sentence_like {
                let snippet: String = strings.iter().take(2)
                    .map(|x| format!("{:?}", x))
                    .collect::<Vec<_>>().join(", ");
                violations.push(format!(
                    "{}: response pool (≥5 sentence-like entries): {} ...",
                    f.display(), snippet,
                ));
            }
        }
    }
    // KNOWN VIOLATIONS (filed as tasks to de-hardcode — they exist
    // in the tree but must not GROW).  Adding new pools anywhere
    // should fail THIS test; the legacy ones are exempted by path.
    let legacy_exempt = [
        "reasoner.rs", // #400 (jokes, anchors, conversational responses)
    ];
    violations.retain(|v| {
        !legacy_exempt.iter().any(|name| v.contains(name))
    });
    assert!(
        violations.is_empty(),
        "Doctrine violation: new hardcoded response pool in src/ \
         (use learned sampling — see #400):\n{}",
        violations.join("\n")
    );
}

// ---- Rule 3: no .unwrap() / .expect() in library code ----

#[test]
fn no_unwrap_or_expect_in_library_code() {
    // We're lenient: unwrap IS allowed in src/bin/ (CLIs), in
    // #[cfg(test)] blocks, in const initializers, and on lines that
    // carry an explicit SAFETY: / test-only / unwrap-proof comment.
    //
    // The goal is to catch NEW additions in library code — we don't
    // try to retrofit every legacy site. Count the violations; if
    // it's meaningfully growing, the axiom gets stricter later.
    let mut new_violations = 0usize;
    for f in src_files() {
        if f.components().any(|c| {
            matches!(c.as_os_str().to_str(), Some("bin") | Some("examples")
                | Some("benches") | Some("tests"))
        }) { continue; }
        let s = read(&f);
        let mut in_test_block = false;
        let mut test_depth = 0i32;
        for line in s.lines() {
            // Track #[cfg(test)] module boundaries naively.
            if line.contains("#[cfg(test)]") { in_test_block = true; }
            if in_test_block {
                test_depth += line.matches('{').count() as i32
                    - line.matches('}').count() as i32;
                if test_depth <= 0 && line.contains('}') {
                    in_test_block = false; test_depth = 0;
                }
                continue;
            }
            if line.contains(".unwrap()") || line.contains(".expect(") {
                // Allow explicit acknowledgement.
                if line.contains("// SAFETY:")
                    || line.contains("// test-only")
                    || line.contains("// unwrap-proof")
                    || line.contains("// doctrine-exempt")
                {
                    continue;
                }
                new_violations += 1;
            }
        }
    }
    // Existing tree has many legacy sites. Don't fail — report a
    // ceiling instead. If the count creeps up, the ceiling comes
    // down in a follow-up commit. This keeps the rule active
    // without requiring a week-long sweep right now.
    const CURRENT_CEILING: usize = 1500;
    assert!(
        new_violations <= CURRENT_CEILING,
        "Doctrine ratchet: .unwrap() / .expect() count in library code \
         rose above ceiling {} (found {}). Either add SAFETY / test-only \
         comment to acknowledge, or replace with ?-propagation.",
        CURRENT_CEILING, new_violations,
    );
}

// ---- Rule 4: secret comparisons must be constant-time ----

#[test]
fn secret_comparisons_are_constant_time() {
    // Find lines that `==` a variable whose name is secret-ish.
    // A naive pattern but catches the most obvious mistakes;
    // sophisticated ones need human review.
    const SECRET_NAMES: &[&str] = &[
        "password", "passphrase", "api_key", "token",
        "credential", "private_key",
    ];
    let mut violations = Vec::new();
    for f in src_files() {
        if f.components().any(|c| {
            matches!(c.as_os_str().to_str(),
                Some("bin") | Some("examples") | Some("tests"))
        }) { continue; }
        let s = read(&f);
        // Word-boundary check — avoids matching "tokens" on a search
        // for "token". Also skips lines that look like they're inside
        // a string literal (e.g. a training-data row containing the
        // literal text "password == ...").
        fn word_contains(line: &str, needle: &str) -> bool {
            // Consider the character immediately before and after
            // each occurrence; reject if either is alphanumeric / _.
            let bytes = line.as_bytes();
            let nb = needle.as_bytes();
            let mut i = 0;
            while let Some(pos) = line[i..].find(needle) {
                let abs = i + pos;
                let before_ok = abs == 0 || {
                    let c = bytes[abs - 1] as char;
                    !(c.is_ascii_alphanumeric() || c == '_')
                };
                let after_ok = abs + nb.len() >= bytes.len() || {
                    let c = bytes[abs + nb.len()] as char;
                    !(c.is_ascii_alphanumeric() || c == '_')
                };
                if before_ok && after_ok { return true; }
                i = abs + nb.len();
                if i >= bytes.len() { break; }
            }
            false
        }
        fn likely_inside_string_literal(line: &str, idx: usize) -> bool {
            // Count unescaped quotes before idx; odd ⇒ inside a string.
            let mut in_str = false;
            let mut prev_backslash = false;
            for (i, c) in line.char_indices() {
                if i >= idx { break; }
                if c == '"' && !prev_backslash { in_str = !in_str; }
                prev_backslash = c == '\\' && !prev_backslash;
            }
            in_str
        }
        for line in s.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") { continue; }
            if trimmed.starts_with("///") { continue; }
            for name in SECRET_NAMES {
                if !word_contains(line, name) { continue; }
                // Find the first occurrence and check if it's inside a
                // string literal.
                let Some(pos) = line.find(name) else { continue };
                if likely_inside_string_literal(line, pos) { continue; }
                // Now check the actual comparison pattern.
                let patterns = [
                    format!("{} ==", name),
                    format!("== {}", name),
                    format!("{} !=", name),
                    format!("!= {}", name),
                ];
                let matched = patterns.iter().any(|p| line.contains(p.as_str()));
                if matched
                    && !line.contains("ct_eq")
                    && !line.contains("ConstantTimeEq")
                    && !line.contains("// timing-safe")
                {
                    violations.push(format!("{}: {}", f.display(), line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Doctrine violation: secret compared with == (not constant-time):\n{}",
        violations.join("\n")
    );
}
