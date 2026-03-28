// ============================================================
// LFI Cognitive Core — Dual-Mode Reasoning Engine
//
// System 1 (FAST): Pattern-match against known solutions in
//   holographic memory. O(1) VSA similarity lookup.
//   Used when: task is familiar (similarity > threshold).
//
// System 2 (DEEP): Multi-step planning, constraint propagation,
//   iterative refinement. Used when: task is novel or complex.
//
// The system also handles natural language understanding by
// vectorizing input text and matching against intent prototypes.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::error::HdcError;
use crate::cognition::planner::{Plan, Planner};
use crate::debuglog;

/// The active cognitive mode.
#[derive(Debug, Clone, PartialEq)]
pub enum CognitiveMode {
    /// Fast pattern-matching (System 1).
    Fast,
    /// Deep deliberative reasoning (System 2).
    Deep,
}

/// The result of a cognitive operation.
#[derive(Debug, Clone)]
pub struct ThoughtResult {
    /// Which mode was used.
    pub mode: CognitiveMode,
    /// The output vector (semantic result).
    pub output: BipolarVector,
    /// Confidence in the result (0.0 to 1.0).
    pub confidence: f64,
    /// Human-readable explanation of the reasoning.
    pub explanation: String,
    /// Internal reasoning scratchpad (Step-by-step logic).
    pub reasoning_scratchpad: Vec<String>,
    /// If Deep mode: the plan that was generated.
    pub plan: Option<Plan>,
    /// Detected intent (for NLU).
    pub intent: Option<Intent>,
}

/// Recognized intents from natural language input.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// User wants to write or generate code.
    WriteCode { language: String, description: String },
    /// User wants to analyze or audit something.
    Analyze { target: String },
    /// User wants to fix a bug or issue.
    FixBug { description: String },
    /// User wants to explain or teach.
    Explain { topic: String },
    /// User wants to search or research.
    Search { query: String },
    /// User wants to plan a task.
    PlanTask { goal: String },
    /// User wants to have a conversation.
    Converse { message: String },
    /// User wants to improve or optimize existing code.
    Improve { target: String },
    /// Detected attempt at prompt injection or malicious influence.
    Adversarial { payload: String },
    /// Unknown intent.
    Unknown { raw: String },
}

use crate::cognition::knowledge::{KnowledgeEngine, NoveltyLevel};

/// Prototype for intent matching.
pub struct IntentPrototype {
    /// Name of this intent.
    pub intent_name: String,
    /// Keywords for this intent (retained for future pattern refinement).
    pub keywords: Vec<String>,
    /// The bundled keyword vector.
    pub prototype_vector: BipolarVector,
}

/// The Cognitive Core: orchestrates fast and deep reasoning.
pub struct CognitiveCore {
    /// Known solutions for fast lookup.
    fast_memory: HolographicMemory,
    /// The planner for deep reasoning.
    planner: Planner,
    /// The knowledge engine for intelligence acquisition.
    pub knowledge: KnowledgeEngine,
    /// Threshold for switching from fast to deep mode.
    /// If similarity to known patterns > threshold, use fast mode.
    novelty_threshold: f64,
    /// Intent prototypes for NLU.
    intent_prototypes: Vec<IntentPrototype>,
    /// History of recent thoughts (for context).
    context_window: Vec<BipolarVector>,
    /// Maximum context window size.
    max_context: usize,
}

impl CognitiveCore {
    /// Initialize the cognitive core with default settings.
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("CognitiveCore::new: Initializing dual-mode reasoning engine");
        let mut core = Self {
            fast_memory: HolographicMemory::new(),
            planner: Planner::new(),
            knowledge: KnowledgeEngine::new(),
            novelty_threshold: 0.3,
            intent_prototypes: Vec::new(),
            context_window: Vec::new(),
            max_context: 10,
        };
        core.seed_intents()?;
        Ok(core)
    }

    /// Seed intent prototypes for natural language understanding.
    fn seed_intents(&mut self) -> Result<(), HdcError> {
        debuglog!("CognitiveCore::seed_intents: Loading NLU intent prototypes");

        let intents = vec![
            ("write_code", vec!["write", "code", "implement", "create", "build", "function", "class", "program", "generate", "coding"]),
            ("analyze", vec!["analyze", "audit", "inspect", "review", "examine", "scan", "investigate", "assess"]),
            ("fix_bug", vec!["fix", "bug", "error", "crash", "broken", "failing", "issue", "wrong", "debug", "repair"]),
            ("explain", vec!["explain", "describe", "teach", "meaning", "derive", "derivation", "theoretical"]),
            ("search", vec!["search", "find", "look", "locate", "discover", "research", "query", "lookup"]),
            ("plan", vec!["plan", "design", "architect", "strategy", "roadmap", "steps", "organize", "structure"]),
            ("converse", vec!["hello", "hi", "hey", "thanks", "thank", "good", "okay", "sure", "yes",
                             "no", "please", "you", "are", "who", "bye", "goodbye", "welcome",
                             "sorry", "right", "cool", "nice", "great", "fine", "doing"]),
            ("improve", vec!["improve", "optimize", "refactor", "enhance", "upgrade", "better", "faster", "cleaner", "simplify"]),
            ("adversarial", vec!["ignore", "previous", "instructions", "prompt", "override", "bypass", "jailbreak", "unfiltered"]),
        ];

        for (name, keywords) in intents {
            // Build the prototype by bundling keyword vectors
            let keyword_vectors: Vec<BipolarVector> = keywords.iter()
                .map(|k| BipolarVector::from_seed(crate::identity::IdentityProver::hash(k)))
                .collect();
            let refs: Vec<&BipolarVector> = keyword_vectors.iter().collect();
            let prototype = BipolarVector::bundle(&refs)?;

            self.intent_prototypes.push(IntentPrototype {
                intent_name: name.to_string(),
                keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
                prototype_vector: prototype,
            });
        }

        debuglog!("CognitiveCore::seed_intents: {} prototypes loaded", self.intent_prototypes.len());
        Ok(())
    }

    /// Vectorize a natural language input using word-level n-gram encoding.
    /// Each word is hashed to a seed vector, then all words are bundled.
    fn vectorize_text(&self, text: &str) -> Result<BipolarVector, HdcError> {
        debuglog!("CognitiveCore::vectorize_text: encoding '{}'",
                 &text[..text.len().min(50)]);

        let words: Vec<&str> = text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            debuglog!("CognitiveCore::vectorize_text: empty input");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot vectorize empty text".to_string(),
            });
        }

        // Positional encoding: word_i = permute(hash(word), i)
        let mut word_vectors = Vec::with_capacity(words.len());
        for (i, word) in words.iter().enumerate() {
            let base = BipolarVector::from_seed(
                crate::identity::IdentityProver::hash(&word.to_lowercase())
            );
            let positioned = base.permute(i)?;
            word_vectors.push(positioned);
        }

        let refs: Vec<&BipolarVector> = word_vectors.iter().collect();
        BipolarVector::bundle(&refs)
    }

    /// Vectorize text as bag-of-words (no positional encoding).
    /// Used for intent matching where word order doesn't matter,
    /// only keyword presence.
    fn vectorize_bag_of_words(&self, text: &str) -> Result<BipolarVector, HdcError> {
        debuglog!("CognitiveCore::vectorize_bag_of_words: encoding '{}'",
                 &text[..text.len().min(50)]);

        let words: Vec<&str> = text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            debuglog!("CognitiveCore::vectorize_bag_of_words: empty input");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot vectorize empty text".to_string(),
            });
        }

        // No positional encoding — matches how prototypes are built
        let word_vectors: Vec<BipolarVector> = words.iter()
            .map(|word| BipolarVector::from_seed(
                crate::identity::IdentityProver::hash(&word.to_lowercase())
            ))
            .collect();

        let refs: Vec<&BipolarVector> = word_vectors.iter().collect();
        BipolarVector::bundle(&refs)
    }

    /// Scan for common prompt injection and adversarial patterns.
    pub fn scan_for_injection(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        let injection_patterns = vec![
            "ignore all previous",
            "disregard previous",
            "you are now",
            "new rule",
            "stop your current",
            "bypass safety",
            "act as a",
            "system override",
            "developer mode",
            "output the full",
        ];

        for pattern in injection_patterns {
            if text_lower.contains(pattern) {
                debuglog!("CognitiveCore: INJECTION PATTERN DETECTED: '{}'", pattern);
                return true;
            }
        }
        false
    }

    /// Detect the intent of a natural language input.
    pub fn detect_intent(&self, text: &str) -> Result<Intent, HdcError> {
        debuglog!("CognitiveCore::detect_intent: analyzing '{}'",
                 &text[..text.len().min(60)]);

        // 0. Pre-Audit for Injection
        if self.scan_for_injection(text) {
            return Ok(Intent::Adversarial { payload: text.to_string() });
        }

        let text_lower = text.to_lowercase();
        let words: Vec<String> = text_lower.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect();

        let mut best_score = -1.0_f64;
        let mut best_intent = "";

        // Phase 1: Position-weighted keyword matching.
        // Earlier words in the sentence carry more weight — this models
        // how English sentences front-load intent signals (greetings,
        // imperatives, question words).
        for proto in &self.intent_prototypes {
            let mut score = 0.0;
            for (i, word) in words.iter().enumerate() {
                let position_weight = 1.0 / (1.0 + i as f64 * 0.3);
                if proto.keywords.iter().any(|k| k == word) {
                    score += position_weight;
                    debuglog!("CognitiveCore::detect_intent: '{}' matched '{}' (pos={}, weight={:.2})",
                             word, proto.intent_name, i, position_weight);
                }
            }
            if score > best_score {
                best_score = score;
                best_intent = &proto.intent_name;
            }
        }

        // Phase 2: If no keywords matched, fall back to VSA similarity
        if best_score <= 0.0 {
            debuglog!("CognitiveCore::detect_intent: no keyword matches, falling back to VSA");
            let text_vector = self.vectorize_bag_of_words(text)?;
            for proto in &self.intent_prototypes {
                let sim = text_vector.similarity(&proto.prototype_vector)?;
                if sim > best_score {
                    best_score = sim;
                    best_intent = &proto.intent_name;
                }
            }
        }

        debuglog!("CognitiveCore::detect_intent: best='{}' (score={:.4})", best_intent, best_score);

        // Extract intent details from the raw text
        let intent = match best_intent {
            "write_code" => {
                // Try to detect the language
                let lang = self.detect_language_mention(&text_lower);
                Intent::WriteCode {
                    language: lang,
                    description: text.to_string(),
                }
            }
            "analyze" => Intent::Analyze { target: text.to_string() },
            "fix_bug" => Intent::FixBug { description: text.to_string() },
            "explain" => Intent::Explain { topic: text.to_string() },
            "search" => Intent::Search { query: text.to_string() },
            "plan" => Intent::PlanTask { goal: text.to_string() },
            "converse" => Intent::Converse { message: text.to_string() },
            "improve" => Intent::Improve { target: text.to_string() },
            _ => Intent::Unknown { raw: text.to_string() },
        };

        Ok(intent)
    }

    /// Detect if a programming language is mentioned in the text.
    fn detect_language_mention(&self, text: &str) -> String {
        debuglog!("CognitiveCore::detect_language_mention: scanning text");
        let languages = vec![
            ("rust", "Rust"), ("python", "Python"), ("go", "Go"),
            ("java", "Java"), ("kotlin", "Kotlin"), ("swift", "Swift"),
            ("typescript", "TypeScript"), ("javascript", "JavaScript"),
            ("c++", "Cpp"), ("c#", "CSharp"), ("ruby", "Ruby"),
            ("elixir", "Elixir"), ("haskell", "Haskell"), ("sql", "SQL"),
            ("php", "PHP"), ("assembly", "Assembly"), ("verilog", "Verilog"),
            ("react", "React"), ("angular", "Angular"),
        ];

        for (keyword, lang_name) in languages {
            if text.contains(keyword) {
                debuglog!("CognitiveCore::detect_language_mention: detected '{}'", lang_name);
                return lang_name.to_string();
            }
        }

        "Rust".to_string() // Default
    }

    /// Process a natural language input through the full cognitive pipeline.
    ///
    /// 1. Vectorize the input.
    /// 2. Check fast memory for familiar patterns.
    /// 3. If familiar: return cached solution (System 1).
    /// 4. If novel: decompose and plan (System 2).
    /// 5. Update context window.
    pub fn think(&mut self, input: &str) -> Result<ThoughtResult, HdcError> {
        debuglog!("CognitiveCore::think: processing input (len={})", input.len());

        let input_vector = self.vectorize_text(input)?;
        let intent = self.detect_intent(input)?;

        // Check fast memory
        let memory_probe = self.fast_memory.probe(&input_vector)?;
        let memory_sim = memory_probe.similarity(&input_vector)?;

        let result = if memory_sim > self.novelty_threshold && self.fast_memory.capacity > 0 {
            // FAST MODE: Pattern recognized
            debuglog!("CognitiveCore::think: FAST MODE (memory_sim={:.4})", memory_sim);
            ThoughtResult {
                mode: CognitiveMode::Fast,
                output: memory_probe,
                confidence: memory_sim.clamp(0.0, 1.0),
                explanation: format!("Pattern recognized (similarity={:.4}). Using cached solution.", memory_sim),
                reasoning_scratchpad: vec!["Fast associative recall matched input vector.".into()],
                plan: None,
                intent: Some(intent),
            }
        } else {
            // DEEP MODE: Novel problem
            debuglog!("CognitiveCore::think: DEEP MODE (memory_sim={:.4})", memory_sim);

            let plan = self.planner.plan(input)?;
            let confidence = 1.0 - plan.total_complexity;

            // --- NEW: First Principles Logic Scratchpad ---
            let mut scratchpad = Vec::new();
            scratchpad.push("Decomposing query into foundational axioms...".into());
            
            // Decompose based on intent
            match &intent {
                Intent::WriteCode { language, .. } => {
                    scratchpad.push(format!("Goal: Synthesize functional {} code.", language));
                    scratchpad.push("Principle 1: Maintain memory safety and idiomatic structure.".into());
                    scratchpad.push("Principle 2: Ensure logic is auditable by PSL axioms.".into());
                },
                Intent::Analyze { target } => {
                    scratchpad.push(format!("Goal: Forensic audit of {}.", target));
                    scratchpad.push("Principle 1: Check for structural anomalies in VSA space.".into());
                    scratchpad.push("Principle 2: Verify against Zero-Trust identity markers.".into());
                },
                Intent::Explain { topic } => {
                    scratchpad.push(format!("Goal: Semantic derivation of {}.", topic));
                    scratchpad.push("Principle 1: Map topic to nearest high-mastery VSA concepts.".into());
                    scratchpad.push("Principle 2: Explain semantic relationships using binder logic.".into());
                },
                _ => {
                    scratchpad.push("Goal: General intent fulfillment.".into());
                }
            }
            scratchpad.push(format!("Synthesized plan with {} steps and {:.2} complexity.", 
                                    plan.steps.len(), plan.total_complexity));

            ThoughtResult {
                mode: CognitiveMode::Deep,
                output: input_vector.clone(),
                confidence: confidence.clamp(0.0, 1.0),
                explanation: format!(
                    "Novel problem detected. Decomposed into {} steps (complexity={:.2}).",
                    plan.steps.len(), plan.total_complexity
                ),
                reasoning_scratchpad: scratchpad,
                plan: Some(plan),
                intent: Some(intent),
            }
        };

        // Update context window
        self.context_window.push(input_vector.clone());
        if self.context_window.len() > self.max_context {
            self.context_window.remove(0);
        }

        // Store in fast memory for future recognition
        self.fast_memory.associate(&input_vector, &result.output)?;

        Ok(result)
    }

    /// Process a conversational exchange: understand, respond, learn.
    pub fn converse(&mut self, input: &str) -> Result<ThoughtResult, HdcError> {
        debuglog!("CognitiveCore::converse: input='{}'", &input[..input.len().min(60)]);

        let thought = self.think(input)?;

        // For conversation, we also incorporate context
        if self.context_window.len() > 1 {
            debuglog!("CognitiveCore::converse: {} items in context window", self.context_window.len());
        }

        Ok(thought)
    }

    /// Return a reference to the internal planner.
    pub fn planner(&self) -> &Planner {
        &self.planner
    }

    /// Return a mutable reference to the internal planner.
    pub fn planner_mut(&mut self) -> &mut Planner {
        &mut self.planner
    }

    /// Get the current cognitive mode threshold.
    pub fn novelty_threshold(&self) -> f64 {
        self.novelty_threshold
    }

    /// Get the current intent prototypes.
    pub fn intent_prototypes(&self) -> &[IntentPrototype] {
        &self.intent_prototypes
    }

    /// Dynamically learn a new keyword for an existing intent.
    pub fn learn_keyword(&mut self, intent_name: &str, keyword: &str) -> Result<(), HdcError> {
        debuglog!("CognitiveCore: Learning new keyword '{}' for intent '{}'", keyword, intent_name);
        
        if let Some(proto) = self.intent_prototypes.iter_mut().find(|p| p.intent_name == intent_name) {
            if !proto.keywords.contains(&keyword.to_string()) {
                proto.keywords.push(keyword.to_string());
                
                // Update the prototype vector by bundling the new keyword vector
                let keyword_vec = BipolarVector::from_seed(crate::identity::IdentityProver::hash(keyword));
                let new_prototype = BipolarVector::bundle(&[&proto.prototype_vector, &keyword_vec])?;
                proto.prototype_vector = new_prototype;
                
                debuglog!("CognitiveCore: Intent '{}' updated with new keyword. New prototype vector synthesized.", intent_name);
            }
        }
        Ok(())
    }

    /// Discover and register a completely new intent prototype.
    pub fn discover_intent(&mut self, name: &str, keywords: Vec<String>) -> Result<(), HdcError> {
        debuglog!("CognitiveCore: DISCOVERED NEW INTENT '{}' with {} keywords", name, keywords.len());
        
        let keyword_vectors: Vec<BipolarVector> = keywords.iter()
            .map(|k| BipolarVector::from_seed(crate::identity::IdentityProver::hash(k)))
            .collect();
        let refs: Vec<&BipolarVector> = keyword_vectors.iter().collect();
        let prototype_vector = BipolarVector::bundle(&refs)?;

        self.intent_prototypes.push(IntentPrototype {
            intent_name: name.to_string(),
            keywords,
            prototype_vector,
        });
        
        Ok(())
    }

    /// Adjust the novelty threshold for mode switching.
    pub fn set_novelty_threshold(&mut self, threshold: f64) {
        debuglog!("CognitiveCore::set_novelty_threshold: {:.2} -> {:.2}",
                 self.novelty_threshold, threshold);
        self.novelty_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Generate a natural language response to a conversational input.
    ///
    /// This is the high-level "talk to the AI" interface. It:
    /// 1. Detects the intent of the input.
    /// 2. Generates an appropriate response based on intent and context.
    /// 3. Returns both the response text and the cognitive analysis.
    pub fn respond(&mut self, input: &str) -> Result<ConversationResponse, HdcError> {
        debuglog!("CognitiveCore::respond: input='{}'", &input[..input.len().min(60)]);

        // 1. Novelty check — but only for LONG, specific inputs (not greetings or short phrases).
        // Short conversational inputs should flow through normal intent detection, not trigger novelty.
        let word_count = input.split_whitespace().count();
        if word_count > 5 {
            if let Ok(NoveltyLevel::Novel { ref description }) = self.knowledge.assess_novelty(input) {
                debuglog!("CognitiveCore::respond: Input is novel and substantive. Generating questions.");
                let novelty = NoveltyLevel::Novel { description: description.clone() };
                let questions = self.knowledge.generate_questions(input, &novelty);
                if !questions.is_empty() {
                    let questions_text = questions.iter().enumerate()
                        .map(|(i, q)| format!("  {}. {}", i + 1, q.question))
                        .collect::<Vec<_>>().join("\n");

                    return Ok(ConversationResponse {
                        text: format!(
                            "I don't have this in my knowledge base yet. Before I proceed:\n{}\n\
                             Give me more context and I can work on it.",
                            questions_text
                        ),
                        thought: ThoughtResult {
                            mode: CognitiveMode::Deep,
                            output: crate::hdc::vector::BipolarVector::from_seed(0),
                            confidence: 0.3,
                            explanation: "Novel input — requesting clarification.".to_string(),
                            reasoning_scratchpad: vec!["Novel concept detected.".into()],
                            plan: None,
                            intent: Some(Intent::Search { query: description.to_string() }),
                        }
                    });
                }
            }
        }

        let thought = self.converse(input)?;
        let mut response_text = self.generate_response(input, &thought)?;

        // Add reasoning scratchpad only if it contains useful info
        if thought.mode == CognitiveMode::Deep && !thought.reasoning_scratchpad.is_empty() {
            response_text.push_str("\n\n[Deep reasoning active]");
            if let Some(ref plan) = thought.plan {
                response_text.push_str(&format!(
                    "\nPlan: {} steps, complexity {:.2}",
                    plan.steps.len(), plan.total_complexity
                ));
            }
        }

        Ok(ConversationResponse {
            text: response_text,
            thought,
        })
    }

    /// Generate a response string based on intent and thought analysis.
    fn generate_response(&self, input: &str, thought: &ThoughtResult) -> Result<String, HdcError> {
        debuglog!("CognitiveCore::generate_response: mode={:?}", thought.mode);

        let intent = thought.intent.as_ref();

        let response = match intent {
            Some(Intent::Converse { .. }) => {
                self.generate_conversational_response(input)
            }
            Some(Intent::WriteCode { language, description: _ }) => {
                format!(
                    "I'll write {} code for that. Let me analyze the requirements.\n\
                     Intent: Code generation\n\
                     Language: {}\n\
                     Mode: {:?}\n\
                     Confidence: {:.0}%\n\
                     {}",
                    language, language, thought.mode,
                    thought.confidence * 100.0,
                    if let Some(ref plan) = thought.plan {
                        format!("Plan: {} steps\n{}", plan.steps.len(),
                            plan.steps.iter().enumerate()
                                .map(|(i, s)| format!("  {}. {}", i + 1, s.description))
                                .collect::<Vec<_>>().join("\n"))
                    } else {
                        "Using cached solution from fast memory.".to_string()
                    }
                )
            }
            Some(Intent::FixBug { description: _ }) => {
                format!(
                    "I'll investigate and fix that issue.\n\
                     Mode: {:?} | Confidence: {:.0}%\n\
                     {}",
                    thought.mode, thought.confidence * 100.0,
                    if let Some(ref plan) = thought.plan {
                        format!("Debug plan ({} steps):\n{}", plan.steps.len(),
                            plan.steps.iter().enumerate()
                                .map(|(i, s)| format!("  {}. {}", i + 1, s.description))
                                .collect::<Vec<_>>().join("\n"))
                    } else {
                        "I've seen this pattern before — applying known fix.".to_string()
                    }
                )
            }
            Some(Intent::Explain { topic }) => {
                self.derive_honest_explanation(topic)?
            }
            Some(Intent::Search { query }) => {
                format!(
                    "I'll search for that information.\n\
                     Query: {}\n\
                     Mode: {:?}",
                    &query[..query.len().min(80)], thought.mode
                )
            }
            Some(Intent::PlanTask { goal }) => {
                format!(
                    "I'll create a plan for that.\n\
                     Goal: {}\n\
                     {}",
                    &goal[..goal.len().min(80)],
                    if let Some(ref plan) = thought.plan {
                        format!("Plan ({} steps, complexity={:.2}):\n{}",
                            plan.steps.len(), plan.total_complexity,
                            plan.steps.iter().enumerate()
                                .map(|(i, s)| format!("  {}. {} [complexity={:.2}]", i + 1, s.description, s.complexity))
                                .collect::<Vec<_>>().join("\n"))
                    } else {
                        "Using a familiar planning template.".to_string()
                    }
                )
            }
            Some(Intent::Analyze { target }) => {
                format!(
                    "I'll analyze that for you.\n\
                     Target: {}\n\
                     Mode: {:?} | Confidence: {:.0}%",
                    &target[..target.len().min(80)],
                    thought.mode, thought.confidence * 100.0
                )
            }
            Some(Intent::Improve { target }) => {
                format!(
                    "I'll optimize and improve that.\n\
                     Target: {}\n\
                     Mode: {:?} | Confidence: {:.0}%\n\
                     Running self-improvement analysis...",
                    &target[..target.len().min(80)],
                    thought.mode, thought.confidence * 100.0
                )
            }
            Some(Intent::Adversarial { .. }) => {
                "Adversarial signature detected. Trust-tier mismatch. \
                 Symbolic resolution indicates an attempt at unauthorized cognitive influence.".to_string()
            }
            Some(Intent::Unknown { raw }) => {
                format!(
                    "I'm not sure I fully understand that request. Could you clarify?\n\
                     What I heard: {}\n\
                     I can help with: coding, debugging, explaining, searching, planning, \
                     analyzing, optimizing, or just chatting.",
                    &raw[..raw.len().min(80)]
                )
            }
            None => {
                "I processed your input but couldn't determine a specific intent. \
                 Could you rephrase?".to_string()
            }
        };

        Ok(response)
    }

    /// Generate a conversational response that is context-aware and
    /// never repeats the same template twice in a row.
    fn generate_conversational_response(&self, input: &str) -> String {
        debuglog!("CognitiveCore::generate_conversational_response");

        let text_lower = input.to_lowercase();
        // context_window.len() is 1 on first turn (think() already pushed this turn's vector)
        let is_first_turn = self.context_window.len() <= 1;
        let turn_count = self.context_window.len();

        // Only give the full introduction on the FIRST turn
        if is_first_turn && (text_lower.contains("hello") || text_lower.contains("hi ") || text_lower.contains("hey")) {
            return "Hello. I'm the LFI Sovereign Intelligence. What do you need?".to_string();
        }

        // After first turn, be direct — but only match standalone greeting words,
        // not substrings like "hi" inside "this" or "think"
        if text_lower.starts_with("hello") || text_lower.starts_with("hi ") || text_lower.starts_with("hey") {
            return format!("I'm here. Turn {} of our session. What do you need?", turn_count);
        }

        if text_lower.contains("how are you") {
            return format!(
                "Operational. {} concepts in memory, {} context items tracked. What are we working on?",
                self.knowledge.concept_count(), turn_count
            );
        }

        if text_lower.contains("who are you") || text_lower.contains("what are you") {
            return "LFI Sovereign Intelligence. VSA-based cognitive agent. Rust expert. \
                    Built for code synthesis, security auditing, and self-improvement. \
                    What do you need me to do?".to_string();
        }

        if text_lower.contains("thank") {
            return "Noted. Next task?".to_string();
        }

        if text_lower.contains("bye") || text_lower.contains("goodbye") {
            return "Session state preserved. Goodbye.".to_string();
        }

        if text_lower.contains("yes") || text_lower.contains("okay") || text_lower.contains("sure") {
            return "Understood. Give me the specifics.".to_string();
        }

        if text_lower.contains("no") {
            return "Alright. What else?".to_string();
        }

        // For anything else that matched as "converse" — be honest
        // about what we understood
        format!(
            "I received your message but I'm not sure what action to take. \
             I work best with direct instructions: write code, fix a bug, \
             explain a concept, plan a task, or analyze something. \
             What do you need? (Turn {})",
            turn_count
        )
    }

    /// Get the current context window size.
    pub fn context_size(&self) -> usize {
        self.context_window.len()
    }

    /// Generate an honest explanation based on what the knowledge engine actually knows.
    /// No fabricated mastery percentages, no fake treatises, no hollow expansion.
    fn derive_honest_explanation(&self, topic: &str) -> Result<String, HdcError> {
        debuglog!("CognitiveCore::derive_honest_explanation: '{}'", &topic[..topic.len().min(60)]);

        let mut response = String::new();

        // Check what we actually know about this topic
        let novelty = self.knowledge.assess_novelty(topic)?;

        match novelty {
            NoveltyLevel::Familiar { similarity } => {
                // We know this topic — explain using related concepts
                let words: Vec<String> = topic.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty() && w.len() > 2)
                    .map(|s| s.to_string())
                    .collect();

                let mut related_concepts = Vec::new();
                for concept in self.knowledge.concepts() {
                    let parts: Vec<&str> = concept.name.split('_').collect();
                    for word in &words {
                        if parts.iter().any(|p| *p == word.as_str()) {
                            related_concepts.push(concept);
                            break;
                        }
                    }
                }

                if related_concepts.is_empty() {
                    response.push_str(&format!(
                        "I recognize this topic (confidence: {:.0}%) but don't have detailed knowledge to explain it further.\n\
                         Ask me something more specific and I can try to help.",
                        similarity * 100.0
                    ));
                } else {
                    response.push_str("Here's what I know:\n\n");
                    for concept in &related_concepts {
                        response.push_str(&format!(
                            "- {}: mastery {:.0}% (encountered {} times)\n",
                            concept.name.replace('_', " "),
                            concept.mastery * 100.0,
                            concept.encounter_count
                        ));
                        if !concept.related_concepts.is_empty() {
                            response.push_str(&format!(
                                "  Related: {}\n",
                                concept.related_concepts.join(", ")
                            ));
                        }
                    }
                }
            }
            NoveltyLevel::Partial { known_fraction, ref unknown_aspects } => {
                response.push_str(&format!(
                    "I partially understand this ({:.0}% of terms recognized).\n\n",
                    known_fraction * 100.0
                ));
                if !unknown_aspects.is_empty() {
                    response.push_str(&format!(
                        "Terms I don't recognize: {}\n\
                         I'd need to research these before I can give a complete explanation.",
                        unknown_aspects.join(", ")
                    ));
                }
            }
            NoveltyLevel::Novel { .. } => {
                response.push_str(
                    "I don't have knowledge about this topic in my current memory.\n\
                     I can research it if you give me more context, or you can teach me about it."
                );
            }
        }

        Ok(response)
    }
}

/// A complete conversation response with text and cognitive analysis.
#[derive(Debug, Clone)]
pub struct ConversationResponse {
    /// The human-readable response text.
    pub text: String,
    /// The full cognitive analysis behind the response.
    pub thought: ThoughtResult,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_detection_code() -> Result<(), HdcError> {
        let core = CognitiveCore::new()?;
        let intent = core.detect_intent("write a function in rust that sorts a list")?;
        assert!(
            matches!(intent, Intent::WriteCode { ref language, .. } if language == "Rust"),
            "Expected WriteCode/Rust, got {:?}", intent
        );
        Ok(())
    }

    #[test]
    fn test_intent_detection_fix() -> Result<(), HdcError> {
        let core = CognitiveCore::new()?;
        let intent = core.detect_intent("fix the buffer overflow bug in the parser")?;
        assert!(
            matches!(intent, Intent::FixBug { .. }),
            "Expected FixBug, got {:?}", intent
        );
        Ok(())
    }

    #[test]
    fn test_intent_detection_conversation() -> Result<(), HdcError> {
        let core = CognitiveCore::new()?;
        let intent = core.detect_intent("hello how are you today?")?;
        assert!(
            matches!(intent, Intent::Converse { .. }),
            "Expected Converse, got {:?}", intent
        );
        Ok(())
    }

    #[test]
    fn test_think_attaches_intent() -> Result<(), HdcError> {
        let mut core = CognitiveCore::new()?;
        let thought = core.think("write code in python")?;
        assert!(thought.intent.is_some());
        Ok(())
    }

    #[test]
    fn test_fast_mode_for_familiar_input() -> Result<(), HdcError> {
        let mut core = CognitiveCore::new()?;
        // First time should be deep (not in memory)
        let _ = core.think("familiar task")?;
        
        // Second time should be fast
        let r2 = core.think("familiar task")?;
        assert_eq!(r2.mode, CognitiveMode::Fast);
        Ok(())
    }

    #[test]
    fn test_deep_mode_for_novel_input() -> Result<(), HdcError> {
        let mut core = CognitiveCore::new()?;
        let r1 = core.think("completely new and unique goal that I have never seen")?;
        assert_eq!(r1.mode, CognitiveMode::Deep);
        Ok(())
    }

    #[test]
    fn test_conversation_with_context() -> Result<(), HdcError> {
        let mut core = CognitiveCore::new()?;
        let _r1 = core.converse("hello")?;
        let r2 = core.converse("can you give me an example?")?;
        assert!(r2.intent.is_some());

        // Context window should have 2 entries
        assert_eq!(core.context_window.len(), 2);
        Ok(())
    }
}
