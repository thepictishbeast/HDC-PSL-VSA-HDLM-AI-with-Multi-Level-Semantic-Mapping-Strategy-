// ============================================================
// LFI Sovereign Cognitive Agent Terminal — Chat Binary
//
// ROUTING PROTOCOL:
// 1. ALL input passes through the local CognitiveCore FIRST.
//    This ensures VSA vectorization, intent detection, knowledge
//    acquisition, and MCTS reasoning happen locally.
// 2. If the local reasoner determines it needs external help
//    (BigBrain tier escalation), it delegates to Gemini CLI.
// 3. Context ledger persists across the full session.
// 4. System commands (/status, /save, /learn, /model, /train,
//    /search, /teach) provide direct agent control.
// ============================================================

use std::io::{self, Write};
use lfi_vsa_core::agent::LfiAgent;
use lfi_vsa_core::inference_engine::InferenceEngine;
use lfi_vsa_core::cognition::router::IntelligenceTier;
use lfi_vsa_core::intelligence::web_search::WebSearchEngine;
use lfi_vsa_core::intelligence::persistence::KnowledgeStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with DEBUG level for maximum forensic visibility
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    println!("============================================================");
    println!(" LFI Sovereign Intelligence v0.8.0");
    println!(" Trimodal Neuro-Symbolic Swarm — Cognitive Agent Terminal");
    println!("------------------------------------------------------------");
    println!(" Commands: /status /save /learn on|off /model <tier>");
    println!("           /search <query> /teach <key> = <value>");
    println!("           /train /facts /help exit|quit");
    println!("============================================================\n");

    let mut agent = LfiAgent::new()?;

    // --- SECURE LOGIN CHALLENGE ---
    print!("Sovereign Identity Verification Required.\nEnter Password> ");
    let _ = io::stdout().flush();
    let mut password = String::new();
    if io::stdin().read_line(&mut password).is_err() {
        println!("Authentication Fault.");
        return Ok(());
    }
    let password = password.trim();

    if agent.authenticate(password) {
        println!("LFI> [IDENTITY VERIFIED] Full cognitive access granted.");
        println!("LFI> Intelligence tier routing: Pulse -> Bridge -> BigBrain (auto)");
        println!("LFI> Background learning daemon available. Type /learn on to activate.\n");
    } else {
        println!("LFI> [RESTRICTED MODE] Local symbolic reasoning only.\n");
    }

    // --- CONTEXTUAL PERSISTENCE (MEMORY LEDGER) ---
    let mut context_ledger: Vec<String> = Vec::new();
    let search_engine = WebSearchEngine::new();

    // Load persistent facts into the conversation
    {
        let guard = agent.shared_knowledge.lock();
        let fact_count = guard.store.facts.len();
        let concept_count = guard.store.concepts.len();
        if fact_count > 0 || concept_count > 0 {
            println!("LFI> [PERSISTENT MEMORY] Loaded {} facts, {} concepts from previous sessions.", fact_count, concept_count);
        }
    }

    loop {
        print!("\nUser> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error reading input.");
            break;
        }

        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            // Save knowledge before exit
            let store_path = KnowledgeStore::default_path();
            let mut guard = agent.shared_knowledge.lock();
            if let Err(e) = guard.store.save(&store_path) {
                eprintln!("LFI> [WARNING] Failed to save knowledge: {:?}", e);
            } else {
                println!("LFI> [SAVED] Knowledge persisted to disk.");
            }
            println!("LFI> Goodbye. Sovereign session terminated.");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // ============================================================
        // SYSTEM COMMAND DISPATCH
        // ============================================================

        if input.starts_with('/') {
            handle_system_command(input, &mut agent, &search_engine).await;
            continue;
        }

        // ============================================================
        // COGNITIVE PIPELINE — LOCAL INTELLIGENCE FIRST
        // ============================================================

        // 1. Auto-learn from conversational teaching patterns
        auto_learn_from_input(input, &mut agent);

        // 2. Check persistent knowledge for direct fact recall
        if let Some(fact_response) = try_fact_recall(input, &agent) {
            println!("LFI> {}", fact_response);
            context_ledger.push(format!("User: {}", input));
            context_ledger.push(format!("LFI: {}", fact_response));
            continue;
        }

        // 3. Route through local CognitiveCore (intent detection + reasoning)
        match agent.chat(input) {
            Ok(response) => {
                let thought = &response.thought;
                let tier = agent.current_tier;

                // Display the local reasoning result
                println!("LFI> [{}|{:?}|conf:{:.0}%] {}",
                    match tier {
                        IntelligenceTier::Pulse => "PULSE",
                        IntelligenceTier::Bridge => "BRIDGE",
                        IntelligenceTier::BigBrain => "BIGBRAIN",
                    },
                    thought.mode,
                    thought.confidence * 100.0,
                    response.text
                );

                // 4. If BigBrain tier AND authenticated, enhance with Gemini delegation
                if tier == IntelligenceTier::BigBrain && agent.authenticated {
                    println!("\nLFI> [ESCALATING TO BIGBRAIN — delegating to external inference...]");
                    match InferenceEngine::delegate_inference(input, &context_ledger).await {
                        Ok(external_response) => {
                            println!("\n{}\n", external_response);
                            context_ledger.push(format!("User: {}", input));
                            context_ledger.push(format!("LFI_Local: {}", response.text));
                            context_ledger.push(format!("LFI_BigBrain: {}", external_response));
                        }
                        Err(e) => {
                            println!("LFI> [BigBrain unavailable: {}] Using local response.", e);
                            context_ledger.push(format!("User: {}", input));
                            context_ledger.push(format!("LFI: {}", response.text));
                        }
                    }
                } else {
                    // 5. For non-escalated intents, check if web search could help
                    if response.text.contains("I don't have this") || response.text.contains("not sure I fully understand") {
                        println!("\nLFI> [SEARCHING WEB for additional context...]");
                        match search_engine.search(input) {
                            Ok(search_response) if !search_response.results.is_empty() => {
                                println!("LFI> [WEB] Found {} results (trust: {:.0}%)",
                                    search_response.results.len(),
                                    search_response.cross_reference_trust * 100.0
                                );
                                if !search_response.best_summary.is_empty() {
                                    let summary = &search_response.best_summary[..search_response.best_summary.len().min(500)];
                                    println!("LFI> {}", summary);

                                    // Ingest into persistent knowledge
                                    let mut guard = agent.shared_knowledge.lock();
                                    let topic = input.to_lowercase();
                                    guard.store.upsert_fact(&topic.replace(' ', "_"), summary);
                                    guard.store.mark_searched(&topic);
                                }
                            }
                            _ => {
                                println!("LFI> [WEB] No results found.");
                            }
                        }
                    }

                    context_ledger.push(format!("User: {}", input));
                    context_ledger.push(format!("LFI: {}", response.text));
                }

                // 6. Ingest any background learnings
                let learnings = agent.background_learner.drain_recent_learnings();
                if !learnings.is_empty() {
                    println!("\nLFI> [BACKGROUND LEARNING] {} new concepts acquired:", learnings.len());
                    for learning in &learnings {
                        println!("     - {} (trust: {:.0}%, sources: {})",
                            learning.topic, learning.trust * 100.0, learning.source_count);
                    }
                }
            }
            Err(e) => {
                println!("LFI> [Cognitive Fault] {:?}", e);
            }
        }
    }
    Ok(())
}

/// Handle system commands that start with /
async fn handle_system_command(input: &str, agent: &mut LfiAgent, search_engine: &WebSearchEngine) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0];

    match cmd {
        "/help" => {
            println!("LFI> Commands:");
            println!("  /status         — Show substrate telemetry and cognitive state");
            println!("  /model <tier>   — Lock intelligence tier: pulse, bridge, bigbrain");
            println!("  /learn on|off   — Toggle background learning daemon");
            println!("  /search <query> — Direct web search with cross-referencing");
            println!("  /teach K = V    — Teach a persistent fact (e.g., /teach capital_of_france = Paris)");
            println!("  /facts          — List all known persistent facts");
            println!("  /save           — Force-save knowledge to disk");
            println!("  /train          — Run local VSA training on available datasets");
            println!("  exit | quit     — Terminate session (auto-saves)");
        }
        "/status" => {
            let guard = agent.shared_knowledge.lock();
            println!("LFI> [STATUS]");
            println!("  Tier: {:?}", agent.current_tier);
            println!("  Authenticated: {}", agent.authenticated);
            println!("  Entropy: {:.1}", agent.entropy_level);
            println!("  Facts: {}", guard.store.facts.len());
            println!("  Concepts: {}", guard.store.concepts.len());
            println!("  Session: {}", guard.store.current_session_id);
            println!("  Background Learning: {}", if agent.background_learner.is_running() { "ACTIVE" } else { "INACTIVE" });
        }
        "/model" => {
            if parts.len() > 1 {
                match parts[1].to_lowercase().as_str() {
                    "pulse" => { agent.current_tier = IntelligenceTier::Pulse; println!("LFI> [OVERRIDE] Pulse tier locked."); }
                    "bridge" => { agent.current_tier = IntelligenceTier::Bridge; println!("LFI> [OVERRIDE] Bridge tier locked."); }
                    "bigbrain" => { agent.current_tier = IntelligenceTier::BigBrain; println!("LFI> [OVERRIDE] BigBrain tier locked."); }
                    _ => println!("LFI> Unknown tier. Options: pulse, bridge, bigbrain"),
                }
            } else {
                println!("LFI> Current Tier: {:?}", agent.current_tier);
            }
        }
        "/learn" => {
            if parts.len() > 1 {
                match parts[1] {
                    "on" => {
                        if let Err(e) = agent.background_learner.start() {
                            println!("LFI> [ERROR] Failed to start background learning: {:?}", e);
                        } else {
                            println!("LFI> [DAEMON] Background learning ACTIVATED. Continuous web research enabled.");
                        }
                    }
                    "off" => {
                        if let Err(e) = agent.background_learner.stop() {
                            println!("LFI> [ERROR] Failed to stop background learning: {:?}", e);
                        } else {
                            println!("LFI> [DAEMON] Background learning DEACTIVATED. Knowledge saved.");
                        }
                    }
                    _ => println!("LFI> Usage: /learn on | /learn off"),
                }
            } else {
                println!("LFI> Background learning is {}.",
                    if agent.background_learner.is_running() { "ACTIVE" } else { "INACTIVE" });
            }
        }
        "/search" => {
            if parts.len() > 1 {
                let query = parts[1..].join(" ");
                println!("LFI> [SEARCHING] '{}'...", query);
                match search_engine.search(&query) {
                    Ok(response) => {
                        println!("LFI> {} results from {} sources (trust: {:.0}%)",
                            response.results.len(), response.source_count,
                            response.cross_reference_trust * 100.0);
                        if !response.best_summary.is_empty() {
                            println!("LFI> {}", &response.best_summary[..response.best_summary.len().min(800)]);
                        }
                        for (i, r) in response.results.iter().take(3).enumerate() {
                            println!("  [{}] {} — {}", i + 1, r.title, &r.snippet[..r.snippet.len().min(120)]);
                        }
                        // Persist the result
                        let mut guard = agent.shared_knowledge.lock();
                        let topic = query.to_lowercase().replace(' ', "_");
                        if !response.best_summary.is_empty() {
                            guard.store.upsert_fact(&topic, &response.best_summary[..response.best_summary.len().min(500)]);
                        }
                        guard.store.mark_searched(&query);
                    }
                    Err(e) => println!("LFI> [SEARCH FAILED] {:?}", e),
                }
            } else {
                println!("LFI> Usage: /search <query>");
            }
        }
        "/teach" => {
            let rest = input.strip_prefix("/teach").unwrap_or("").trim();
            if let Some((key, value)) = rest.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                agent.conversation_facts.insert(key.to_string(), value.to_string());
                let mut guard = agent.shared_knowledge.lock();
                guard.store.upsert_fact(key, value);
                println!("LFI> [LEARNED] '{}' = '{}' — persisted to knowledge store.", key, value);
            } else {
                println!("LFI> Usage: /teach key = value");
            }
        }
        "/facts" => {
            let guard = agent.shared_knowledge.lock();
            if guard.store.facts.is_empty() {
                println!("LFI> No persistent facts stored.");
            } else {
                println!("LFI> [PERSISTENT FACTS] ({} total)", guard.store.facts.len());
                for fact in &guard.store.facts {
                    println!("  {} = {}", fact.key, &fact.value[..fact.value.len().min(100)]);
                }
            }
        }
        "/save" => {
            let store_path = KnowledgeStore::default_path();
            let mut guard = agent.shared_knowledge.lock();
            match guard.store.save(&store_path) {
                Ok(()) => println!("LFI> [SAVED] Knowledge persisted to {:?}", store_path),
                Err(e) => println!("LFI> [SAVE FAILED] {:?}", e),
            }
        }
        "/train" => {
            println!("LFI> [FORGE INITIATED] Scanning for training datasets...");
            let datasets = [
                "data_ingestion/toucan_intent_subset.json",
                "data_ingestion/swe_bench_code_subset.json",
                "data_ingestion/gsm8k_math_subset.json",
            ];
            for ds in &datasets {
                let path = format!("/root/lfi_project/{}", ds);
                if std::path::Path::new(&path).exists() {
                    println!("LFI> [FOUND] {}", ds);
                } else {
                    println!("LFI> [MISSING] {} — skipping", ds);
                }
            }
            println!("LFI> Training pipeline ready. Use the `train` binary for full execution.");
        }
        _ => {
            println!("LFI> Unknown command '{}'. Type /help for available commands.", cmd);
        }
    }
}

/// Auto-learn from conversational teaching patterns
fn auto_learn_from_input(input: &str, agent: &mut LfiAgent) {
    let lower = input.to_lowercase();

    // Pattern: "my name is X" / "I am X" / "call me X"
    if lower.starts_with("my name is ") {
        let name = input[11..].trim();
        if !name.is_empty() {
            agent.conversation_facts.insert("sovereign_name".to_string(), name.to_string());
            let mut guard = agent.shared_knowledge.lock();
            guard.store.upsert_fact("sovereign_name", name);
        }
    } else if lower.starts_with("i am ") && !lower.contains('?') {
        let identity = input[5..].trim();
        if !identity.is_empty() && identity.split_whitespace().count() <= 4 {
            agent.conversation_facts.insert("sovereign_identity".to_string(), identity.to_string());
            let mut guard = agent.shared_knowledge.lock();
            guard.store.upsert_fact("sovereign_identity", identity);
        }
    }

    // Pattern: "X is Y" (factual teaching)
    if !lower.starts_with("what") && !lower.starts_with("who") &&
       !lower.starts_with("how") && !lower.starts_with("why") &&
       !lower.contains('?') {
        if let Some((key, value)) = input.split_once(" is ") {
            let key = key.trim();
            let value = value.trim();
            if key.split_whitespace().count() <= 3 && value.len() > 2 && value.len() < 200 {
                agent.conversation_facts.insert(key.to_lowercase(), value.to_string());
                let mut guard = agent.shared_knowledge.lock();
                guard.store.upsert_fact(&key.to_lowercase(), value);
            }
        }
    }
}

/// Try to recall a fact from persistent or session memory
fn try_fact_recall(input: &str, agent: &LfiAgent) -> Option<String> {
    let lower = input.to_lowercase();

    // "what is my name" / "what's my name" pattern
    if lower.contains("my name") && (lower.contains("what") || lower.contains("remember")) {
        if let Some(name) = agent.conversation_facts.get("sovereign_name") {
            return Some(format!("Your name is {}.", name));
        }
        let guard = agent.shared_knowledge.lock();
        if let Some(name) = guard.store.get_fact("sovereign_name") {
            return Some(format!("Your name is {}.", name));
        }
    }

    // "what do you know about X" / "tell me about X" pattern
    if lower.starts_with("what do you know about ") {
        let topic = lower.strip_prefix("what do you know about ").unwrap_or("").trim();
        let key = topic.replace(' ', "_");

        // Check session facts
        if let Some(value) = agent.conversation_facts.get(&key) {
            return Some(format!("{}: {}", topic, value));
        }

        // Check persistent store
        let guard = agent.shared_knowledge.lock();
        if let Some(value) = guard.store.get_fact(&key) {
            return Some(format!("[Persistent Memory] {}: {}", topic, value));
        }
    }

    None
}
