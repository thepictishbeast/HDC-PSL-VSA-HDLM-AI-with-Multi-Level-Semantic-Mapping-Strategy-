// ============================================================
// LFI Knowledge Engine — Intelligence Acquisition & Novelty
//
// Handles the AI's ability to:
// 1. ASSESS novelty: Is this problem familiar or completely new?
// 2. ASK questions: When uncertain, generate clarifying questions.
// 3. RESEARCH: Identify what information is needed and where to find it.
// 4. FILTER noise: Distinguish high-value signal from irrelevant data.
// 5. LEARN: Absorb new patterns and integrate them into memory.
// 6. SELF-REFLECT: Know its own capabilities and limitations.
//
// The engine uses VSA similarity to measure how "novel" a problem
// is relative to its existing knowledge base, and generates
// structured research plans for truly unknown territory.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::error::HdcError;
use crate::debuglog;

/// How novel is this input relative to what the engine knows?
#[derive(Debug, Clone, PartialEq)]
pub enum NoveltyLevel {
    /// Very familiar — can respond immediately (System 1).
    Familiar { similarity: f64 },
    /// Partially known — some aspects are new.
    Partial { known_fraction: f64, unknown_aspects: Vec<String> },
    /// Completely novel — needs research and decomposition.
    Novel { description: String },
}

/// A clarifying question the AI should ask before proceeding.
#[derive(Debug, Clone)]
pub struct ClarifyingQuestion {
    /// The question text.
    pub question: String,
    /// Why this question matters for the task.
    pub reason: String,
    /// Priority (0.0 = nice-to-know, 1.0 = critical).
    pub priority: f64,
    /// Category of information needed.
    pub category: QuestionCategory,
}

/// Categories of clarifying questions.
#[derive(Debug, Clone, PartialEq)]
pub enum QuestionCategory {
    /// Need to understand the goal better.
    GoalClarification,
    /// Need technical constraints or requirements.
    TechnicalConstraints,
    /// Need to know about the target environment.
    Environment,
    /// Need to understand existing code or architecture.
    ExistingContext,
    /// Need to understand user preferences.
    Preferences,
    /// Need to know about available resources.
    Resources,
}

/// A research need — something the AI needs to learn.
#[derive(Debug, Clone)]
pub struct ResearchNeed {
    /// What topic needs research.
    pub topic: String,
    /// Where to look for this information.
    pub sources: Vec<ResearchSource>,
    /// How critical this is (0.0 = optional, 1.0 = blocking).
    pub criticality: f64,
    /// Estimated knowledge gap (0.0 = almost know it, 1.0 = completely unknown).
    pub gap_size: f64,
}

/// Where to research a topic.
#[derive(Debug, Clone, PartialEq)]
pub enum ResearchSource {
    /// Search the web for documentation, tutorials, examples.
    WebSearch { query: String },
    /// Read existing codebase files.
    CodebaseSearch { pattern: String },
    /// Analyze existing examples in memory.
    MemoryRecall,
    /// Ask the user for clarification.
    AskUser,
    /// Examine API documentation.
    ApiDocs { endpoint: String },
    /// Study language-specific references.
    LanguageReference { language: String },
}

/// Signal quality assessment for noise filtering.
#[derive(Debug, Clone)]
pub struct SignalAssessment {
    /// Overall information value (0.0 = pure noise, 1.0 = critical signal).
    pub value: f64,
    /// Relevance to the current task (0.0 = irrelevant, 1.0 = directly relevant).
    pub relevance: f64,
    /// Credibility of the source (0.0 = untrustworthy, 1.0 = authoritative).
    pub credibility: f64,
    /// Whether this is actionable information.
    pub actionable: bool,
    /// Explanation of the assessment.
    pub explanation: String,
}

/// Self-knowledge about the AI's own capabilities.
#[derive(Debug, Clone)]
pub struct SelfKnowledge {
    /// Languages the AI is expert in (used in its own development).
    pub expert_languages: Vec<String>,
    /// Languages the AI knows well.
    pub proficient_languages: Vec<String>,
    /// Languages the AI has basic knowledge of.
    pub basic_languages: Vec<String>,
    /// Core capabilities.
    pub capabilities: Vec<String>,
    /// Known limitations.
    pub limitations: Vec<String>,
    /// What the AI is currently learning.
    pub learning_targets: Vec<String>,
}

/// A learned concept stored in knowledge memory.
#[derive(Debug, Clone)]
pub struct LearnedConcept {
    /// Name of the concept.
    pub name: String,
    /// VSA vector representation.
    pub vector: BipolarVector,
    /// How well the AI understands this (0.0 = barely, 1.0 = mastery).
    pub mastery: f64,
    /// How many times this concept has been encountered.
    pub encounter_count: usize,
    /// Trust score of the source (1.0 = Sovereign, 0.0 = Untrusted).
    pub trust_score: f64,
    /// Related concept names.
    pub related_concepts: Vec<String>,
}

/// The Knowledge Engine: manages intelligence acquisition and learning.
pub struct KnowledgeEngine {
    /// Long-term knowledge store (VSA holographic memory).
    knowledge_memory: HolographicMemory,
    /// Learned concepts indexed by name.
    concepts: Vec<LearnedConcept>,
    /// Self-knowledge about capabilities.
    self_knowledge: SelfKnowledge,
    /// Noise filter threshold (signals below this are discarded).
    noise_threshold: f64,
    /// Maximum number of clarifying questions to generate.
    max_questions: usize,
}

impl KnowledgeEngine {
    /// Initialize the knowledge engine with baseline self-knowledge.
    pub fn new() -> Self {
        debuglog!("KnowledgeEngine::new: Initializing intelligence acquisition engine");
        let mut engine = Self {
            knowledge_memory: HolographicMemory::new(),
            concepts: Vec::new(),
            self_knowledge: Self::baseline_self_knowledge(),
            noise_threshold: 0.2,
            max_questions: 5,
        };
        engine.seed_core_concepts();
        engine
    }

    /// Baseline self-knowledge: what the AI knows about itself.
    fn baseline_self_knowledge() -> SelfKnowledge {
        debuglog!("KnowledgeEngine::baseline_self_knowledge: loading self-model");
        SelfKnowledge {
            expert_languages: vec![
                "Rust".to_string(),
            ],
            proficient_languages: vec![
                "Go".to_string(),
                "TypeScript".to_string(),
                "Python".to_string(),
            ],
            basic_languages: vec![
                "Java".to_string(),
                "Kotlin".to_string(),
                "Swift".to_string(),
                "C".to_string(),
                "Cpp".to_string(),
                "Assembly".to_string(),
            ],
            capabilities: vec![
                "Hyperdimensional computing (VSA operations)".to_string(),
                "Code synthesis and generation".to_string(),
                "AST analysis and optimization".to_string(),
                "Intent detection and NLU".to_string(),
                "Goal decomposition and planning".to_string(),
                "Security auditing (PSL axioms)".to_string(),
                "Self-improvement (recursive optimization)".to_string(),
                "Holographic memory (O(1) associative recall)".to_string(),
                "Multi-agent coordination (HMAS)".to_string(),
                "OSINT analysis".to_string(),
                "Dual-mode reasoning (System 1 fast / System 2 deep)".to_string(),
            ],
            limitations: vec![
                "Cannot access external networks without WebIngestor agent".to_string(),
                "VSA similarity is approximate, not exact".to_string(),
                "Limited by available compute and memory resources".to_string(),
                "Cannot learn from data it has not been exposed to".to_string(),
                "Holographic memory has capacity limits (superposition interference)".to_string(),
                "Must respect PSL axioms — cannot bypass safety constraints".to_string(),
            ],
            learning_targets: Vec::new(),
        }
    }

    /// Seed core programming concepts into knowledge memory.
    fn seed_core_concepts(&mut self) {
        debuglog!("KnowledgeEngine::seed_core_concepts: loading foundational knowledge");

        let concepts = vec![
            // Rust mastery concepts (expert level)
            ("ownership_borrowing", 0.95, vec!["lifetimes", "references", "move_semantics"]),
            ("pattern_matching", 0.95, vec!["match_arms", "destructuring", "enums"]),
            ("error_handling_result", 0.95, vec!["result_type", "option_type", "question_mark_operator"]),
            ("trait_system", 0.95, vec!["generics", "trait_bounds", "impl_blocks"]),
            ("async_await_rust", 0.90, vec!["futures", "tokio", "async_runtime"]),
            ("unsafe_rust", 0.90, vec!["raw_pointers", "ffi", "memory_safety"]),
            ("macro_system", 0.85, vec!["declarative_macros", "procedural_macros", "derive"]),
            ("lifetime_annotations", 0.90, vec!["elision", "static_lifetime", "higher_rank"]),
            ("concurrency_rust", 0.90, vec!["threads", "channels", "arc_mutex"]),
            ("module_system", 0.95, vec!["crate_structure", "pub_visibility", "use_imports"]),

            // General programming concepts
            ("recursion", 0.85, vec!["base_case", "recursive_case", "stack_overflow"]),
            ("data_structures", 0.85, vec!["arrays", "trees", "hash_maps", "graphs"]),
            ("algorithms", 0.80, vec!["sorting", "searching", "dynamic_programming"]),
            ("design_patterns", 0.80, vec!["factory", "observer", "strategy", "builder"]),
            ("testing", 0.90, vec!["unit_tests", "integration_tests", "property_tests"]),
            ("security", 0.85, vec!["input_validation", "authentication", "encryption"]),
            ("networking", 0.75, vec!["http", "tcp", "websockets", "rest_api"]),
            ("databases", 0.70, vec!["sql", "orm", "transactions", "indexes"]),

            // VSA/HDC concepts (self-knowledge about own architecture)
            ("hypervectors", 0.95, vec!["bipolar_vectors", "dimensionality", "quasi_orthogonal"]),
            ("vsa_operations", 0.95, vec!["bind_xor", "bundle_sum", "permute_shift"]),
            ("holographic_memory", 0.95, vec!["associative_recall", "superposition", "capacity"]),
            ("similarity_search", 0.90, vec!["cosine_similarity", "hamming_distance", "threshold"]),

            // Conversational & General Knowledge concepts (baseline)
            ("greetings", 0.95, vec!["hello", "hi", "hey", "howdy", "greetings"]),
            ("small_talk", 0.90, vec!["how", "are", "you", "doing", "today", "status"]),
            ("basic_arithmetic", 0.95, vec!["math", "calculation", "arithmetic", "sum", "plus", "minus", "2+2"]),
            ("general_geography", 0.85, vec!["geography", "france", "capital", "paris", "country", "city"]),
            ("politeness", 0.95, vec!["please", "thanks", "thank", "you", "welcome"]),
            ("common_words", 0.95, vec!["what", "is", "the", "are", "can", "tell", "show", "me"]),

            // Hard/Advanced concepts for testing
            ("vsa_binding_mathematics", 0.85, vec!["xor_commutativity", "self_inverse", "circular_convolution", "fractional_binding"]),
            ("distributed_consensus", 0.80, vec!["raft_algorithm", "paxos", "byzantine_fault_tolerance", "split_brain"]),
            ("quantum_cryptography", 0.70, vec!["qkd", "bb84", "entanglement", "no_cloning_theorem"]),
            ("sovereign_identity_architecture", 0.90, vec!["zero_knowledge_proofs", "did", "verifiable_credentials", "sovereign_identity"]),

            // Diverse Interdisciplinary Concepts
            ("ai_safety_ethics", 0.80, vec!["reward_shaping", "goal_alignment", "instrumental_convergence", "recursive_self_improvement"]),
            ("quantum_information_theory", 0.75, vec!["decoherence", "superposition_interference", "von_neumann_entropy", "qubit_mapping"]),
            ("biological_cybernetics", 0.85, vec!["epigenetic_memory", "homeostasis", "neural_plasticity", "metabolic_logic"]),
            ("forensic_epistemology", 0.90, vec!["ground_truth", "adversarial_reasoning", "probabilistic_proof", "axiom_verification"]),
        ];

        for (name, mastery, related) in concepts {
            let vector = BipolarVector::from_seed(
                crate::identity::IdentityProver::hash(name)
            );
            self.concepts.push(LearnedConcept {
                name: name.to_string(),
                vector,
                mastery,
                encounter_count: 1,
                trust_score: 1.0, // Seeded concepts are absolute truth
                related_concepts: related.into_iter().map(|s| s.to_string()).collect(),
            });
        }

        debuglog!("KnowledgeEngine::seed_core_concepts: {} concepts loaded", self.concepts.len());
    }

    /// Assess how novel a problem is relative to existing knowledge.
    ///
    /// Uses word-level matching: each word in the input is compared against
    /// concept names and their related concepts. Words that match known
    /// concepts are "understood"; words that don't are "unknown aspects."
    pub fn assess_novelty(&self, input: &str) -> Result<NoveltyLevel, HdcError> {
        debuglog!("KnowledgeEngine::assess_novelty: analyzing '{}'",
                 &input[..input.len().min(60)]);

        // Filter out common English stop words that are never technical concepts.
        // These add noise to the novelty assessment.
        let stop_words: &[&str] = &[
            "the", "and", "for", "are", "but", "not", "you", "all", "can", "had",
            "her", "was", "one", "our", "out", "has", "his", "how", "its", "let",
            "may", "new", "now", "old", "see", "way", "who", "did", "get", "got",
            "him", "hit", "say", "she", "too", "use", "what", "why", "with",
            "from", "have", "this", "that", "they", "will", "been", "each",
            "make", "like", "just", "over", "such", "take", "than", "them",
            "very", "when", "come", "could", "into", "some", "time", "about",
            "know", "would", "your", "more", "does", "dont", "seem",
            "explain", "describe", "tell", "show", "give", "help",
            "need", "want", "think", "works", "work", "using", "used",
            "understand", "anything", "something", "saying", "should", "there",
            "write", "read", "look", "find", "keep", "going", "done",
        ];

        let words: Vec<String> = input.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty() && w.len() > 2 && !stop_words.contains(w))
            .map(|s| s.to_string())
            .collect();

        if words.is_empty() {
            // All words are stop words → treat as familiar (generic English input)
            debuglog!("KnowledgeEngine::assess_novelty: all stop words → FAMILIAR");
            return Ok(NoveltyLevel::Familiar { similarity: 0.5 });
        }

        let mut matching_concepts = Vec::new();
        let mut unknown_aspects = Vec::new();
        let mut best_mastery = 0.0_f64;

        // For each word, check if it appears in any concept name or related concepts
        for word in &words {
            let mut found_match = false;

            for concept in &self.concepts {
                // Check concept name (split on underscores for matching)
                let concept_parts: Vec<&str> = concept.name.split('_').collect();
                if concept_parts.iter().any(|p| *p == word.as_str()) {
                    found_match = true;
                    if !matching_concepts.contains(&concept.name) {
                        matching_concepts.push(concept.name.clone());
                    }
                    if concept.mastery > best_mastery {
                        best_mastery = concept.mastery;
                    }
                    break;
                }

                // Check related concepts
                for related in &concept.related_concepts {
                    let related_parts: Vec<&str> = related.split('_').collect();
                    if related_parts.iter().any(|p| *p == word.as_str()) {
                        found_match = true;
                        if !matching_concepts.contains(&concept.name) {
                            matching_concepts.push(concept.name.clone());
                        }
                        if concept.mastery > best_mastery {
                            best_mastery = concept.mastery;
                        }
                        break;
                    }
                }

                if found_match {
                    break;
                }
            }

            if !found_match {
                unknown_aspects.push(word.clone());
            }
        }

        let known_fraction = if words.is_empty() {
            0.0
        } else {
            1.0 - (unknown_aspects.len() as f64 / words.len() as f64)
        };

        let result = if known_fraction > 0.7 && !matching_concepts.is_empty() {
            debuglog!("KnowledgeEngine::assess_novelty: FAMILIAR (known={:.0}%, mastery={:.2})",
                     known_fraction * 100.0, best_mastery);
            NoveltyLevel::Familiar { similarity: best_mastery }
        } else if !matching_concepts.is_empty() {
            debuglog!("KnowledgeEngine::assess_novelty: PARTIAL (known={:.0}%, unknowns={})",
                     known_fraction * 100.0, unknown_aspects.len());
            NoveltyLevel::Partial { known_fraction, unknown_aspects }
        } else {
            debuglog!("KnowledgeEngine::assess_novelty: NOVEL");
            NoveltyLevel::Novel { description: input.to_string() }
        };

        Ok(result)
    }

    /// Generate clarifying questions when the AI is uncertain.
    pub fn generate_questions(&self, input: &str, novelty: &NoveltyLevel) -> Vec<ClarifyingQuestion> {
        debuglog!("KnowledgeEngine::generate_questions: for '{}'",
                 &input[..input.len().min(60)]);

        let mut questions = Vec::new();
        let text_lower = input.to_lowercase();

        match novelty {
            NoveltyLevel::Novel { .. } => {
                // For completely novel problems, ask fundamental questions
                questions.push(ClarifyingQuestion {
                    question: "What is the expected output or result?".to_string(),
                    reason: "Need to understand the goal before attempting a novel problem".to_string(),
                    priority: 1.0,
                    category: QuestionCategory::GoalClarification,
                });
                questions.push(ClarifyingQuestion {
                    question: "Are there any constraints or requirements I should know about?".to_string(),
                    reason: "Novel problems need boundary conditions to scope the solution".to_string(),
                    priority: 0.9,
                    category: QuestionCategory::TechnicalConstraints,
                });
                questions.push(ClarifyingQuestion {
                    question: "Is there existing code or documentation I should reference?".to_string(),
                    reason: "Even novel problems may have relevant prior art".to_string(),
                    priority: 0.7,
                    category: QuestionCategory::ExistingContext,
                });
            }
            NoveltyLevel::Partial { unknown_aspects, .. } => {
                // For partially known problems, ask about the unknowns
                for aspect in unknown_aspects.iter().take(self.max_questions) {
                    questions.push(ClarifyingQuestion {
                        question: format!("Can you clarify what you mean by '{}'?", aspect),
                        reason: format!("'{}' is not in my current knowledge base", aspect),
                        priority: 0.8,
                        category: QuestionCategory::GoalClarification,
                    });
                }
            }
            NoveltyLevel::Familiar { .. } => {
                debuglog!("KnowledgeEngine::generate_questions: familiar problem, minimal questions");
                // Familiar problems may still need clarification on specifics
                if text_lower.contains("code") || text_lower.contains("implement") || text_lower.contains("write") {
                    questions.push(ClarifyingQuestion {
                        question: "Which programming language should I use?".to_string(),
                        reason: "Need to select the appropriate language for code generation".to_string(),
                        priority: 0.5,
                        category: QuestionCategory::Preferences,
                    });
                }
            }
        }

        // Environment questions for any task involving deployment or execution
        if text_lower.contains("deploy") || text_lower.contains("run") || text_lower.contains("execute") {
            questions.push(ClarifyingQuestion {
                question: "What is the target execution environment (server, mobile, embedded, browser)?".to_string(),
                reason: "Need to optimize for the target platform's constraints".to_string(),
                priority: 0.6,
                category: QuestionCategory::Environment,
            });
        }

        // Resource questions for optimization tasks
        if text_lower.contains("optimize") || text_lower.contains("performance") || text_lower.contains("fast") {
            questions.push(ClarifyingQuestion {
                question: "What are the memory and CPU constraints?".to_string(),
                reason: "Optimization strategy depends on available resources".to_string(),
                priority: 0.7,
                category: QuestionCategory::Resources,
            });
        }

        // Truncate to max
        questions.truncate(self.max_questions);
        debuglog!("KnowledgeEngine::generate_questions: generated {} questions", questions.len());
        questions
    }

    /// Identify what research is needed for a given problem.
    pub fn identify_research_needs(&self, input: &str, novelty: &NoveltyLevel) -> Vec<ResearchNeed> {
        debuglog!("KnowledgeEngine::identify_research_needs: for '{}'",
                 &input[..input.len().min(60)]);

        let mut needs = Vec::new();
        let text_lower = input.to_lowercase();

        match novelty {
            NoveltyLevel::Novel { description } => {
                // For novel problems, broad research is needed
                needs.push(ResearchNeed {
                    topic: description.clone(),
                    sources: vec![
                        ResearchSource::WebSearch {
                            query: format!("how to {}", description),
                        },
                        ResearchSource::AskUser,
                    ],
                    criticality: 1.0,
                    gap_size: 0.9,
                });
            }
            NoveltyLevel::Partial { unknown_aspects, .. } => {
                // Research only the unknown parts
                for aspect in unknown_aspects {
                    let sources = vec![
                        ResearchSource::WebSearch {
                            query: format!("{} programming", aspect),
                        },
                        ResearchSource::CodebaseSearch {
                            pattern: format!("**/*{}*", aspect),
                        },
                        ResearchSource::MemoryRecall,
                    ];
                    needs.push(ResearchNeed {
                        topic: aspect.clone(),
                        sources,
                        criticality: 0.7,
                        gap_size: 0.6,
                    });
                }
            }
            NoveltyLevel::Familiar { .. } => {
                debuglog!("KnowledgeEngine::identify_research_needs: familiar, no research needed");
            }
        }

        // Language-specific research
        let language_keywords = vec![
            ("rust", "Rust"), ("python", "Python"), ("go", "Go"),
            ("java", "Java"), ("kotlin", "Kotlin"), ("swift", "Swift"),
            ("typescript", "TypeScript"), ("javascript", "JavaScript"),
        ];

        for (keyword, lang) in language_keywords {
            if text_lower.contains(keyword) {
                let is_expert = self.self_knowledge.expert_languages.iter()
                    .any(|l| l.to_lowercase() == keyword);
                if !is_expert {
                    needs.push(ResearchNeed {
                        topic: format!("{} language specifics", lang),
                        sources: vec![
                            ResearchSource::LanguageReference {
                                language: lang.to_string(),
                            },
                            ResearchSource::MemoryRecall,
                        ],
                        criticality: 0.5,
                        gap_size: if self.self_knowledge.proficient_languages.iter()
                            .any(|l| l.to_lowercase() == keyword) { 0.3 } else { 0.6 },
                    });
                }
            }
        }

        debuglog!("KnowledgeEngine::identify_research_needs: {} needs identified", needs.len());
        needs
    }

    /// Assess the signal quality of incoming information.
    /// Filters noise from valuable intelligence.
    pub fn assess_signal(&self, content: &str, context: &str) -> Result<SignalAssessment, HdcError> {
        debuglog!("KnowledgeEngine::assess_signal: evaluating content quality");

        let content_vector = BipolarVector::from_seed(
            crate::identity::IdentityProver::hash(content)
        );
        let context_vector = BipolarVector::from_seed(
            crate::identity::IdentityProver::hash(context)
        );

        // Relevance: how similar is this content to the current context?
        let relevance = content_vector.similarity(&context_vector)?.max(0.0);

        // Information density: longer, more varied content tends to have more value
        let words: Vec<&str> = content.split_whitespace().collect();
        let unique_words: std::collections::HashSet<&str> = words.iter().copied().collect();
        let density = if words.is_empty() {
            0.0
        } else {
            unique_words.len() as f64 / words.len() as f64
        };

        // Credibility: check if content matches known patterns
        let mut knowledge_match = 0.0_f64;
        for concept in &self.concepts {
            let sim = content_vector.similarity(&concept.vector)?;
            if sim > knowledge_match {
                knowledge_match = sim;
            }
        }

        let credibility = (knowledge_match * 0.5 + density * 0.5).clamp(0.0, 1.0);
        let value = (relevance * 0.5 + credibility * 0.3 + density * 0.2).clamp(0.0, 1.0);
        let actionable = value > self.noise_threshold && relevance > 0.1;

        let explanation = if value > 0.7 {
            "High-value signal: relevant, credible, and information-dense".to_string()
        } else if value > self.noise_threshold {
            "Moderate signal: some useful information present".to_string()
        } else {
            "Low-value noise: not relevant or credible enough to act on".to_string()
        };

        debuglog!("KnowledgeEngine::assess_signal: value={:.4}, relevance={:.4}, credibility={:.4}",
                 value, relevance, credibility);

        Ok(SignalAssessment {
            value,
            relevance,
            credibility,
            actionable,
            explanation,
        })
    }

    /// Learn a new concept from experience.
    /// GATED: Requires Sovereign authentication to achieve high trust and persistence.
    pub fn learn(&mut self, name: &str, related: &[&str], is_sovereign: bool) -> Result<(), HdcError> {
        if !is_sovereign {
            debuglog!("KnowledgeEngine::learn: UNTRUSTED teaching attempt for '{}' REJECTED.", name);
            return Ok(()); // Silently ignore influence from untrusted sources
        }

        debuglog!("KnowledgeEngine::learn: acquiring concept '{}' (Sovereign Verified)", name);

        let vector = BipolarVector::from_seed(
            crate::identity::IdentityProver::hash(name)
        );

        // Check if we already know this concept
        for concept in &mut self.concepts {
            if concept.name == name {
                concept.encounter_count += 1;
                // Mastery increases with exposure
                concept.mastery = (concept.mastery + 0.05).min(1.0);
                concept.trust_score = 1.0; // Reinforced by Sovereign
                
                debuglog!("KnowledgeEngine::learn: reinforced '{}' (mastery={:.2})",
                         name, concept.mastery);

                self.knowledge_memory.associate(&vector, &vector)?;
                return Ok(());
            }
        }

        // New concept (Sovereign only)
        let concept = LearnedConcept {
            name: name.to_string(),
            vector: vector.clone(),
            mastery: 0.3,
            encounter_count: 1,
            trust_score: 1.0,
            related_concepts: related.iter().map(|s| s.to_string()).collect(),
        };
        self.concepts.push(concept);
        self.knowledge_memory.associate(&vector, &vector)?;

        debuglog!("KnowledgeEngine::learn: NEW Sovereign-verified concept '{}' acquired", name);
        Ok(())
    }

    /// Learn a new programming language by studying its patterns.
    pub fn learn_language(&mut self, language: &str, category: &str, is_sovereign: bool) -> Result<(), HdcError> {
        if !is_sovereign {
            debuglog!("KnowledgeEngine::learn_language: UNTRUSTED update attempt for '{}' REJECTED.", language);
            return Ok(());
        }

        debuglog!("KnowledgeEngine::learn_language: Sovereign node studying '{}'", language);

        // Add to appropriate proficiency category
        let lang_str = language.to_string();
        match category {
            "expert" => {
                if !self.self_knowledge.expert_languages.contains(&lang_str) {
                    self.self_knowledge.expert_languages.push(lang_str.clone());
                }
            }
            "proficient" => {
                if !self.self_knowledge.proficient_languages.contains(&lang_str) {
                    self.self_knowledge.proficient_languages.push(lang_str.clone());
                }
            }
            _ => {
                if !self.self_knowledge.basic_languages.contains(&lang_str) {
                    self.self_knowledge.basic_languages.push(lang_str.clone());
                }
            }
        }

        // Learn core language concepts
        let concepts = vec![
            format!("{}_syntax", language.to_lowercase()),
            format!("{}_idioms", language.to_lowercase()),
            format!("{}_stdlib", language.to_lowercase()),
            format!("{}_toolchain", language.to_lowercase()),
        ];

        for concept_name in &concepts {
            self.learn(concept_name, &[language], is_sovereign)?;
        }

        // Track as learning target
        if !self.self_knowledge.learning_targets.contains(&lang_str) {
            self.self_knowledge.learning_targets.push(lang_str);
        }

        debuglog!("KnowledgeEngine::learn_language: '{}' added at '{}' level", language, category);
        Ok(())
    }

    /// Get current self-knowledge.
    pub fn self_knowledge(&self) -> &SelfKnowledge {
        &self.self_knowledge
    }

    /// Get all known concepts.
    pub fn concepts(&self) -> &[LearnedConcept] {
        &self.concepts
    }

    /// Get concept mastery level by name.
    pub fn mastery_of(&self, concept_name: &str) -> f64 {
        debuglog!("KnowledgeEngine::mastery_of: '{}'", concept_name);
        self.concepts.iter()
            .find(|c| c.name == concept_name)
            .map(|c| c.mastery)
            .unwrap_or(0.0)
    }

    /// Check if the AI knows a programming language.
    pub fn knows_language(&self, language: &str) -> bool {
        debuglog!("KnowledgeEngine::knows_language: '{}'", language);
        let lang = language.to_string();
        self.self_knowledge.expert_languages.contains(&lang)
            || self.self_knowledge.proficient_languages.contains(&lang)
            || self.self_knowledge.basic_languages.contains(&lang)
    }

    /// Get the proficiency level for a language.
    pub fn language_proficiency(&self, language: &str) -> &str {
        debuglog!("KnowledgeEngine::language_proficiency: '{}'", language);
        let lang = language.to_string();
        if self.self_knowledge.expert_languages.contains(&lang) {
            "expert"
        } else if self.self_knowledge.proficient_languages.contains(&lang) {
            "proficient"
        } else if self.self_knowledge.basic_languages.contains(&lang) {
            "basic"
        } else {
            "unknown"
        }
    }

    /// Total number of known concepts.
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_engine_initialization() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        assert!(engine.concept_count() > 10, "Should have seeded concepts");
        assert!(engine.knows_language("Rust"));
        assert_eq!(engine.language_proficiency("Rust"), "expert");
        assert_eq!(engine.language_proficiency("Python"), "proficient");
        assert_eq!(engine.language_proficiency("Java"), "basic");
        assert_eq!(engine.language_proficiency("Brainfuck"), "unknown");
        Ok(())
    }

    #[test]
    fn test_novelty_assessment() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        let novelty = engine.assess_novelty("implement ownership borrowing in rust")?;
        // Should recognize some concepts
        assert!(!matches!(novelty, NoveltyLevel::Novel { .. }),
               "Rust ownership should not be completely novel");
        Ok(())
    }

    #[test]
    fn test_generate_questions_novel() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        let novelty = NoveltyLevel::Novel {
            description: "build a quantum computing simulator".to_string(),
        };
        let questions = engine.generate_questions("build a quantum computing simulator", &novelty);
        assert!(!questions.is_empty(), "Should generate questions for novel problems");
        assert!(questions[0].priority > 0.5, "First question should be high priority");
        Ok(())
    }

    #[test]
    fn test_generate_questions_familiar() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        let novelty = NoveltyLevel::Familiar { similarity: 0.9 };
        let questions = engine.generate_questions("sort an array", &novelty);
        // Familiar problems should generate few/no questions
        assert!(questions.len() <= 2);
        Ok(())
    }

    #[test]
    fn test_research_needs_novel() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        let novelty = NoveltyLevel::Novel {
            description: "implement a blockchain consensus protocol".to_string(),
        };
        let needs = engine.identify_research_needs("implement a blockchain consensus protocol", &novelty);
        assert!(!needs.is_empty(), "Novel problems should generate research needs");
        assert!(needs[0].criticality > 0.5);
        assert!(needs[0].gap_size > 0.5);
        Ok(())
    }

    #[test]
    fn test_signal_assessment() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();

        // Relevant signal
        let relevant = engine.assess_signal(
            "the ownership system in rust prevents data races at compile time",
            "rust memory safety"
        )?;
        assert!(relevant.value > 0.0, "Relevant content should have positive value");

        // Noise
        let noise = engine.assess_signal("", "rust programming")?;
        assert!(!noise.actionable, "Empty content should not be actionable");

        Ok(())
    }

    #[test]
    fn test_learn_new_concept() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        let initial_count = engine.concept_count();

        engine.learn("quantum_entanglement", &["physics", "quantum_computing"], true)?;
        assert_eq!(engine.concept_count(), initial_count + 1);
        assert!(engine.mastery_of("quantum_entanglement") > 0.0);

        // Reinforcement
        engine.learn("quantum_entanglement", &[], true)?;
        let after_mastery = engine.mastery_of("quantum_entanglement");
        assert!(after_mastery > 0.3, "Mastery should increase with reinforcement");

        Ok(())
    }

    #[test]
    fn test_learn_new_language() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        assert!(!engine.knows_language("Zig"));

        engine.learn_language("Zig", "basic", true)?;
        assert!(engine.knows_language("Zig"));
        assert_eq!(engine.language_proficiency("Zig"), "basic");

        // Promote to proficient
        engine.learn_language("Zig", "proficient", true)?;
        assert_eq!(engine.language_proficiency("Zig"), "proficient");

        Ok(())
    }

    #[test]
    fn test_rust_mastery() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        // Rust-specific concepts should have high mastery
        assert!(engine.mastery_of("ownership_borrowing") > 0.9);
        assert!(engine.mastery_of("pattern_matching") > 0.9);
        assert!(engine.mastery_of("error_handling_result") > 0.9);
        assert!(engine.mastery_of("trait_system") > 0.9);
        Ok(())
    }

    #[test]
    fn test_self_knowledge() -> Result<(), HdcError> {
        let engine = KnowledgeEngine::new();
        let sk = engine.self_knowledge();
        assert!(sk.expert_languages.contains(&"Rust".to_string()));
        assert!(!sk.capabilities.is_empty());
        assert!(!sk.limitations.is_empty());
        Ok(())
    }
}
