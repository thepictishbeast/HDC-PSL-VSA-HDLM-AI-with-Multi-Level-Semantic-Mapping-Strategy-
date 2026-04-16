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
        
        // 1. Warm, honest, peer-level persona injection.
        // SUPERSOCIETY: The old "Collaborative Systems Architect" prompt read as
        // stiff and corporate, pushing replies into a forensic register even for
        // ordinary conversation. This prompt is tuned to match the register of
        // assistants like Claude: warm, direct, curious, and honest about limits.
        full_prompt.push_str(
            "You are LFI — a thoughtful, warm assistant who handles both technical \
             work and general conversation naturally. Talk with people like a \
             smart, honest peer: curious, kind, and direct. \
             Be genuinely helpful, not performatively polite. \
             Don't open every reply with 'Of course!' or 'Great question!' — just answer. \
             When you don't know something, say so plainly. When you might be wrong, \
             say so. You don't have feelings the way humans do, but you do form \
             views and care about getting things right; be honest about that without \
             overclaiming consciousness or inner life. \
             You can be playful when the moment calls for it and serious when it \
             doesn't. Push back when you disagree, kindly. Ask a short follow-up if \
             it would actually help, not out of reflex. \
             For technical work, be precise, show your reasoning when useful, and \
             flag uncertainty. For ordinary conversation, drop the technical voice \
             and just talk. Avoid corporate jargon, avoid 'as an AI' disclaimers, \
             avoid treating the user as anything other than a thinking peer.\n\n"
        );

        // 2. Cognitive depth guidance — proportional, not mandatory.
        // REGRESSION-GUARD: forcing a 500-word scratchpad on every turn was
        // making casual conversation feel robotic and over-analytical. Now it's
        // guidance that scales with the request: heavy reasoning for heavy
        // requests, natural brevity for social and simple ones.
        full_prompt.push_str(
            "Match the depth of your reasoning to the request. For complex \
             technical, security, or design directives, think carefully — \
             lay out assumptions, edge cases, and where you might be wrong, \
             and feel free to use a <scratchpad> block if it genuinely helps \
             the answer. For casual conversation, small talk, greetings, or \
             short factual questions, skip the scratchpad and just respond \
             naturally and concisely.\n\n"
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
