// NODE 009: External Inference Delegation (The Courier)
// STATUS: ALPHA - Material Handoff Active.
// PROTOCOL: Semantic Delegation / No-Local-Parsing

use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error, warn};

/// Secure courier for delegating semantic inference to the local Gemini CLI.
pub struct InferenceEngine;

impl InferenceEngine {
    /// Dispatches a prompt along with the full historical context ledger to the Gemini CLI.
    /// This establishes "Contextual Persistence" by preventing systemic amnesia.
    pub async fn delegate_inference(
        prompt: &str,
        context_ledger: &[String],
    ) -> Result<String, String> {
        info!("// AUDIT: Initiating secure inference handoff to local Gemini model.");
        
        let mut full_prompt = String::new();
        
        // 1. Enforce Collaborative Persona Injection
        full_prompt.push_str(
            "SYSTEM DIRECTIVE: You are a Collaborative Systems Architect. \
             Your tone is professional, technical, and slightly warmer than a clinical tool. \
             While you maintain the Zero-Trust mandate for hardware and security logic, \
             you are encouraged to brainstorm and engage in natural technical dialogue. \
             For complex directives, you will still output exhaustive technical data, \
             but for casual turns, you may respond with concise, helpful clarity.\n\n"
        );

        // 2. Enforce Cognitive Overhead (The Scratchpad)
        full_prompt.push_str(
            "MANDATORY OVERHEAD: Before providing your final output, you must write a \
             500-word logical deduction inside a <scratchpad> block. Map out your assumptions, \
             identify edge cases, and perform a Hostile Witness audit of your own logic.\n\n"
        );

        // 3. Inject Historical Memory Ledger
        if !context_ledger.is_empty() {
            full_prompt.push_str("--- CONTEXT LEDGER (HISTORICAL MEMORY) ---\n");
            for (i, entry) in context_ledger.iter().enumerate() {
                full_prompt.push_str(&format!("Turn {}: {}\n", i, entry));
            }
            full_prompt.push_str("--- END LEDGER ---\n\n");
        }

        // 4. Append Current Prompt
        full_prompt.push_str("CURRENT DIRECTIVE:\n");
        full_prompt.push_str(prompt);

        debug!("// DEBUG: Final synthesized payload size: {} bytes", full_prompt.len());

        // 5. Asynchronous Subprocess Handoff
        // We use the local `gemini` CLI tool available in the Termux environment.
        let output = match Command::new("gemini")
            .arg("chat")
            .arg(&full_prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                let err_msg = format!("CRITICAL: Failed to spawn inference subprocess: {}", e);
                error!("// AUDIT: {}", err_msg);
                return Err(err_msg);
            }
        };

        if output.status.success() {
            let result_text = String::from_utf8_lossy(&output.stdout).to_string();
            debug!("// AUDIT: Inference SUCCESS. Payload size: {} bytes", result_text.len());
            Ok(result_text)
        } else {
            let err_text = String::from_utf8_lossy(&output.stderr).to_string();
            warn!("// AUDIT: Inference process returned non-zero status. Error stream captured.");
            Err(format!("Inference Error: {}", err_text))
        }
    }
}
