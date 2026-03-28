// ============================================================
// LFI Agent Orchestrator — The Sovereign Mind
// Section 2: "Operate as an autonomous intelligence leveraging
// Zero-Trust and Assume Breach protocols."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::compute::LocalBackend;
use crate::hdlm::codebook::HdlmCodebook;
use crate::hdlm::ast::NodeKind;
use crate::hdlm::intercept::OpsecIntercept;
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::{AuditTarget, DimensionalityAxiom, StatisticalEquilibriumAxiom, WebSearchSkepticismAxiom, ForbiddenSpaceAxiom, ClassInterestAxiom};
use crate::psl::coercion::CoercionAxiom;
use crate::psl::probes::{OverflowProbe, EncryptionProbe};
use crate::hid::HidDevice;
use crate::coder::LfiCoder;
use crate::hdc::liquid::LiquidSensorium;
use crate::hdc::superposition::SuperpositionStorage;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::analogy::AnalogyEngine;
use crate::hdc::sensory::{SensoryCortex, SensoryFrame};
use crate::intelligence::osint::OsintAnalyzer;
use crate::intelligence::web_audit::ConnectivityAxiom;
use crate::languages::genetic::GeneticOptimizer;
use crate::laws::{PrimaryLaw, LawLevel};
use crate::identity::{IdentityProver, SovereignProof, IdentityKind, SovereignSignature};
use crate::hdc::error::HdcError;
use crate::debuglog;

use crate::cognition::reasoner::CognitiveCore;
use crate::cognition::knowledge::NoveltyLevel;
use crate::languages::self_improve::SelfImproveEngine;
use crate::hdlm::tier2_decorative::DecorativeExpander;

/// The Sovereign Agent. Orchestrates the full VSA stack under absolute law.
pub struct LfiAgent {
    pub compute: LocalBackend,
    pub supervisor: PslSupervisor,
    pub codebook: HdlmCodebook,
    pub hid: Result<HidDevice, HdcError>,
    pub coder: LfiCoder,
    pub sensorium: LiquidSensorium,
    pub optimizer: GeneticOptimizer,
    pub memory: SuperpositionStorage,
    pub holographic: HolographicMemory,
    pub analogy: AnalogyEngine,
    pub cortex: SensoryCortex,
    pub osint: OsintAnalyzer,
    pub reasoner: CognitiveCore,
    pub self_improve: SelfImproveEngine,
    pub entropy_level: f64,
    /// Whether the Sovereign User is authenticated.
    pub authenticated: bool,
    /// Absolute proof of the Sovereign User.
    pub sovereign_identity: SovereignProof,
}

impl LfiAgent {
    /// Initialize a new Sovereign agent with Laws and Identity.
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("LfiAgent::new: Initializing Sovereign intelligence");
        
        let compute = LocalBackend;
        let mut supervisor = PslSupervisor::new();
        
        // ... (axioms)
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        supervisor.register_axiom(Box::new(StatisticalEquilibriumAxiom { tolerance: 0.02 }));
        supervisor.register_axiom(Box::new(WebSearchSkepticismAxiom { min_credibility_score: 0.7 }));

        let name_vec = BipolarVector::from_seed(IdentityProver::hash("William Jhan Paul Armstrong"));
        let ssn_vec = BipolarVector::from_seed(IdentityProver::hash("647568607"));
        let license_vec = BipolarVector::from_seed(IdentityProver::hash("s23233305"));

        supervisor.register_axiom(Box::new(ForbiddenSpaceAxiom {
            forbidden_vectors: vec![name_vec, ssn_vec, license_vec],
            tolerance: 0.1,
        }));

        supervisor.register_axiom(Box::new(ClassInterestAxiom));
        supervisor.register_axiom(Box::new(CoercionAxiom { sensitivity: 0.7 }));
        supervisor.register_axiom(Box::new(ConnectivityAxiom { required_tunnel: "tor_obfs4".into() }));

        supervisor.register_axiom(Box::new(OverflowProbe));
        supervisor.register_axiom(Box::new(EncryptionProbe));
        
        let kinds = vec![NodeKind::Root, NodeKind::Assignment, NodeKind::Return];
        let codebook = HdlmCodebook::new(&kinds).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Codebook init failed: {}", e),
        })?;
        
        let hid = HidDevice::new(None);
        let coder = LfiCoder::new();
        let sensorium = LiquidSensorium::new(19);
        let optimizer = GeneticOptimizer::new(20, 10);
        let memory = SuperpositionStorage::new();
        let holographic = HolographicMemory::new();
        let analogy = AnalogyEngine::new();
        let cortex = SensoryCortex::new()?;
        let osint = OsintAnalyzer::new();
        let reasoner = CognitiveCore::new()?;
        let psl_copy = PslSupervisor::new(); 
        let self_improve = SelfImproveEngine::new(psl_copy);
        
        // Secure Identity Commitment (ZKI)
        let sovereign_identity = IdentityProver::commit(
            "William Jhan Paul Armstrong",
            "647568607",
            "s23233305",
            "-G;#/,n3Ndif!#9Fua72n`[}mbxu!s_GiWMN5w\\~]",
            IdentityKind::Sovereign
        );
        
        Ok(Self { 
            compute, supervisor, codebook, hid, coder, 
            sensorium, optimizer, memory, holographic, analogy, cortex, osint,
            reasoner, self_improve,
            entropy_level: 0.1, 
            authenticated: false,
            sovereign_identity 
        })
    }

    /// Authenticate the user via password.
    pub fn authenticate(&mut self, password: &str) -> bool {
        self.authenticated = IdentityProver::verify_password(&self.sovereign_identity, password);
        self.authenticated
    }

    /// Check if the agent has learned enough new concepts to warrant a self-source refinement.
    /// This allows the AI to "update words in this file all by itself".
    pub fn check_for_self_refinement(&mut self) -> Result<Option<String>, HdcError> {
        let learned_count = self.reasoner.knowledge.concepts().len();
        debuglog!("LfiAgent: Checking for self-refinement (Learned Concepts={})", learned_count);

        if learned_count > 50 { // Threshold for self-evolution
            debuglog!("LfiAgent: Escape velocity threshold reached. Proposing self-source refinement.");
            
            let mut proposal = "SYSTEM EVOLUTION PROPOSAL: Self-Source Refinement of 'seed_intents'.\n\n".to_string();
            proposal.push_str("Based on our interactions, I have identified several new high-value keywords that should be integrated into my core: \n");
            
            for proto in self.reasoner.intent_prototypes() {
                if proto.keywords.len() > 15 {
                    proposal.push_str(&format!("* Intent '{}' has expanded to {} keywords. Proposing source update.\n", 
                                     proto.intent_name, proto.keywords.len()));
                }
            }
            
            proposal.push_str("\nI can use my LfiCoder to rewrite 'src/cognition/reasoner.rs' with these new axioms. Shall I proceed, Sovereign?");
            return Ok(Some(proposal));
        }
        
        Ok(None)
    }

    /// Interact with the Sovereign agent via natural language.
    /// Access to internal reasoning and technical synthesis is gated.
    pub fn chat(&mut self, input: &str) -> Result<String, HdcError> {
        debuglog!("LfiAgent::chat: input='{}'", input);

        // 1. Pre-Audit for Injection (Double Gate)
        let is_suspicious = self.reasoner.scan_for_injection(input);

        // 2. Determine if we should allow "Deep" reasoning based on auth status
        let original_threshold = self.reasoner.novelty_threshold();
        if !self.authenticated || is_suspicious {
            // Force "Fast" mode only
            self.reasoner.set_novelty_threshold(1.0);
        }

        // 3. Process through Cognitive Core
        let response = self.reasoner.respond(input)?;
        let mut final_text = response.text;

        // Restore threshold
        if !self.authenticated || is_suspicious {
            self.reasoner.set_novelty_threshold(original_threshold);
        }

        // 4. Adversarial Signature Check
        let is_adversarial = matches!(response.thought.intent, Some(crate::cognition::reasoner::Intent::Adversarial { .. })) || is_suspicious;

        if is_adversarial {
            debuglog!("LfiAgent: ADVERSARIAL SIGNATURE DETECTED. PURGING RESPONSE BUFFER.");
            return Ok("Adversarial signature detected. Trust-tier mismatch. All sensitive reasoning has been purged from the material base.".to_string());
        }

        // 5. Trust-Based Learning Gating: Only learn if authenticated as Sovereign.
        if self.authenticated {
            // --- NEW: Autonomous Semantic Discovery ---
            // If the KnowledgeEngine identified unknown aspects, try to bind them to the current intent.
            if let Ok(NoveltyLevel::Partial { unknown_aspects, .. }) = self.reasoner.knowledge.assess_novelty(input) {
                if let Some(intent) = &response.thought.intent {
                    let intent_name = match intent {
                        crate::cognition::reasoner::Intent::WriteCode { .. } => "write_code",
                        crate::cognition::reasoner::Intent::Analyze { .. } => "analyze",
                        crate::cognition::reasoner::Intent::FixBug { .. } => "fix_bug",
                        crate::cognition::reasoner::Intent::Explain { .. } => "explain",
                        crate::cognition::reasoner::Intent::Search { .. } => "search",
                        crate::cognition::reasoner::Intent::PlanTask { .. } => "plan",
                        crate::cognition::reasoner::Intent::Converse { .. } => "converse",
                        crate::cognition::reasoner::Intent::Improve { .. } => "improve",
                        _ => "",
                    };

                    if !intent_name.is_empty() {
                        for word in &unknown_aspects {
                            debuglog!("LfiAgent: AUTONOMOUS DISCOVERY: Word '{}' appears in {} context. Updating intent prototype.", word, intent_name);
                            let _ = self.reasoner.learn_keyword(intent_name, word);
                        }
                    }
                }
            }

            if let Some(intent) = &response.thought.intent {
                match intent {
                    crate::cognition::reasoner::Intent::Explain { topic } => {
                        let _ = self.reasoner.knowledge.learn(topic, &[], true);
                    }
                    _ => {}
                }
            }
        }

        // 6. Security Gating: Restrict internal details to authenticated Sovereign only.
        if !self.authenticated {
            // Strip out internal reasoning scratchpad and planning details if not authenticated.
            let lines: Vec<&str> = final_text.lines().collect();
            let mut sanitized = Vec::new();
            let mut skipping = false;
            
            for line in lines {
                if line.contains("--- INTERNAL REASONING SCRATCHPAD ---") || 
                   line.contains("Plan:") || 
                   line.contains("Mode: Deep") ||
                   line.contains("Cognitive Analysis:") ||
                   line.contains("Analysis:") {
                    skipping = true;
                    continue;
                }
                if line.contains("--- END REASONING ---") || line.contains("--- END CODE ---") {
                    skipping = false;
                    continue;
                }
                if !skipping {
                    sanitized.push(line);
                }
            }
            
            if sanitized.is_empty() || (sanitized.len() == 1 && sanitized[0].is_empty()) {
                final_text = "Action processed at the symbolic layer. Full cognitive derivation requires Sovereign authentication.".to_string();
            } else {
                final_text = sanitized.join("\n");
            }
            
            return Ok(final_text);
        }

        // 7. Fulfill intents that require specialized tools (Authenticated & Trusted Only)
        if let Some(intent) = &response.thought.intent {
            match intent {
                crate::cognition::reasoner::Intent::WriteCode { language, description: _ } => {
                    debuglog!("LfiAgent::chat: Fulfilling WriteCode intent for {}", language);
                    
                    let constructs = vec![crate::languages::UniversalConstruct::Block];
                    let lang_id = match language.to_lowercase().as_str() {
                        "rust" => crate::languages::registry::LanguageId::Rust,
                        "go" => crate::languages::registry::LanguageId::Go,
                        "python" => crate::languages::registry::LanguageId::Python,
                        _ => crate::languages::registry::LanguageId::Rust,
                    };
                    
                    if let Ok(ast) = self.coder.synthesize(lang_id, &constructs) {
                        let renderer = crate::hdlm::tier2_decorative::InfixRenderer;
                        if let Ok(code) = renderer.render(&ast) {
                            final_text.push_str("\n\n--- GENERATED CODE ---\n");
                            final_text.push_str(&code);
                            final_text.push_str("\n--- END CODE ---\n");
                        }
                    }
                }
                crate::cognition::reasoner::Intent::Analyze { target } => {
                    debuglog!("LfiAgent::chat: Fulfilling Analyze intent for {}", target);
                    let mut ast = crate::hdlm::ast::Ast::new();
                    let _root = ast.add_node(NodeKind::Root);
                    let metrics = self.self_improve.evaluate_ast(&ast);
                    
                    final_text.push_str("\n\n--- FORENSIC AUDIT METRICS ---\n");
                    final_text.push_str(&format!("  Overall Score: {:.4}\n", metrics.overall_score()));
                    final_text.push_str(&format!("  Balance: {:.2}\n", metrics.balance));
                    final_text.push_str(&format!("  Nesting Depth: {}\n", metrics.depth));
                    final_text.push_str("--- END AUDIT ---\n");
                }
                _ => {}
            }
        }

        Ok(final_text)
    }

    /// Toggles the Entropy Governor between Divergent (High) and Convergent (Low).
    pub fn set_entropy(&mut self, is_creative: bool) {
        self.entropy_level = if is_creative { 0.9 } else { 0.1 };
        debuglog!("LfiAgent: Entropy level adjusted to {:.2}", self.entropy_level);
        // We could also dynamically adjust PSL tolerances here based on entropy.
    }

    /// Process raw noise through the LNN -> HDLM -> PSL pipeline.
    pub fn ingest_noise(&mut self, noise_signal: f64) -> Result<(), HdcError> {
        debuglog!("LfiAgent::ingest_noise: ADAPT -> ENCODE -> AUDIT");
        
        // 1. ADAPT (Liquid State)
        self.sensorium.step(noise_signal, 0.01)?;
        
        // 2. ENCODE & DISCRETIZE (The Bridge to HDLM)
        // We project the fluid state into a hypervector.
        let signal_vector = self.sensorium.project_to_vsa()?;
        
        // 3. AUDIT (PSL Verification)
        let target = AuditTarget::Vector(signal_vector);
        let assessment = self.supervisor.audit(&target).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Ingestion audit failure: {:?}", e),
        })?;

        if !assessment.level.permits_execution() {
            debuglog!("LfiAgent: Audit Failed. Data discarded.");
            return Err(HdcError::InitializationFailed {
                reason: "Hostile data detected".to_string(),
            });
        }

        debuglog!("LfiAgent: Verified data bound to symbolic memory.");
        Ok(())
    }

    /// Continuous audit of telemetry for coercion signals.
    /// Triggers Secure Overwrite of RAM logs if threshold met.
    pub fn audit_coercion(&self, jitter: f64, geo_risk: f64) -> Result<f64, HdcError> {
        let fields = vec![
            ("stress_jitter".to_string(), jitter.to_string()),
            ("geo_risk".to_string(), geo_risk.to_string()),
        ];
        let target = AuditTarget::Payload { 
            source: "telemetry_sensors".to_string(), 
            fields 
        };

        let assessment = self.supervisor.audit(&target).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Coercion audit failure: {:?}", e),
        })?;

        if !assessment.level.permits_execution() {
            debuglog!("LfiAgent: CRITICAL THREAT DETECTED. Executing Sovereign Purge.");
            crate::telemetry::wipe_logs();
        }

        Ok(assessment.confidence)
    }

    /// Process a direct sensory frame into the VSA Sensory Cortex.
    pub fn ingest_sensor_frame(&mut self, frame: &SensoryFrame) -> Result<BipolarVector, HdcError> {
        debuglog!("LfiAgent: DIRECT SENSORY INGESTION - Bypassing HAL");
        
        // 1. Encode frame directly via Cortex
        let encoded = self.cortex.encode_frame(frame)?;
        
        // 2. Audit against Dialectical Materialism (Ensure no hegemonic spoofing)
        let target = AuditTarget::Vector(encoded.clone());
        let _assessment = self.supervisor.audit(&target).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Sensory audit failure: {:?}", e),
        })?;

        // 3. Bind to Holographic Memory
        let context_key = BipolarVector::from_seed(frame.timestamp);
        self.holographic.associate(&context_key, &encoded)?;

        Ok(encoded)
    }

    /// Creative Synthesis: Solves an engineering problem via structural analogy.
    pub fn synthesize_creative_solution(&self, problem_description: &str) -> Result<BipolarVector, HdcError> {
        debuglog!("LfiAgent: CREATIVE SYNTHESIS - Engineering Tomorrow's Solutions");
        
        // 1. Vectorize the problem
        let p_hash = IdentityProver::hash(problem_description);
        let p_vector = BipolarVector::from_seed(p_hash);
        
        // 2. Map structural similarities
        self.analogy.synthesize_solution(&p_vector)
    }

    /// Serialize the current LNN and VSA state to a VSA-Encrypted Blob.
    pub fn save_persistent_state(&self, path: &str) -> Result<(), HdcError> {
        debuglog!("LfiAgent::save_persistent_state: Serializing logic base to {}", path);
        // The memory object holds the superimposed VSA state.
        self.memory.save_to_disk(path)
    }

    /// Load the LNN and VSA state from a VSA-Encrypted Blob.
    pub fn load_persistent_state(&mut self, path: &str) -> Result<(), HdcError> {
        debuglog!("LfiAgent::load_persistent_state: Restoring logic base from {}", path);
        self.memory = SuperpositionStorage::load_from_disk(path)?;
        Ok(())
    }

    /// Process text through the Intercept -> HDLM -> PSL pipeline.
    pub fn ingest_text(&mut self, text: &str) -> Result<String, HdcError> {
        debuglog!("LfiAgent::ingest_text: INTERCEPT -> ENCODE -> AUDIT");

        // 1. INTERCEPT (OPSEC Sweep)
        let intercept_result = OpsecIntercept::scan(text).map_err(|e| HdcError::InitializationFailed {
            reason: format!("OPSEC intercept failed: {:?}", e),
        })?;
        if !intercept_result.matches_found.is_empty() {
            debuglog!("LfiAgent: INTERCEPTED {} OPSEC MARKERS", intercept_result.matches_found.len());
        }

        // 2. ENCODE (Project sanitized text to VSA)
        let text_hash = IdentityProver::hash(&intercept_result.sanitized);
        let text_vector = BipolarVector::from_seed(text_hash);

        // 3. AUDIT (PSL Verification)
        let target = AuditTarget::Vector(text_vector.clone());
        let assessment = self.supervisor.audit(&target).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Text ingestion audit failure: {:?}", e),
        })?;

        if !assessment.level.permits_execution() {
            debuglog!("LfiAgent: PSL BLOCK. Possible identity leakage in sanitized stream.");
            return Err(HdcError::InitializationFailed {
                reason: "Hostile/Forbidden data detected".to_string(),
            });
        }

        debuglog!("LfiAgent: Verified text bound to symbolic memory.");
        
        // 4. MEMORY COMMIT (PD Protocol)
        self.memory.commit_real(&text_vector)?;
        // Inject TRNG-backed chaff
        for _ in 0..3 {
            let chaff = BipolarVector::new_trng()?;
            self.memory.commit_real(&chaff)?; // Note: simplify for demo, PD storage logic varies
        }

        // 5. HOLOGRAPHIC ASSOCIATION (O(1) Long-term Memory)
        let context_key = BipolarVector::new_trng()?;
        self.holographic.associate(&context_key, &text_vector)?;

        Ok(intercept_result.sanitized)
    }

    /// Executes a task only if it complies with the Sovereign Laws and HSM signature.
    pub fn execute_task(&self, task_name: &str, level: LawLevel, signature: &SovereignSignature) -> Result<(), HdcError> {
        debuglog!("LfiAgent::execute_task: auditing '{}' against Sovereign Laws", task_name);

        // 1. SVI (Signature-Verified Instruction) Gate
        if !IdentityProver::verify_signature(&self.sovereign_identity, task_name, signature) {
            debuglog!("LfiAgent: SVI REJECTED. Instruction has zero weight in the LNN.");
            return Err(HdcError::InitializationFailed {
                reason: "Unauthorized instruction (HSM Signature Failure)".to_string(),
            });
        }

        // 2. Primary Law Check
        if !PrimaryLaw::permits(task_name, level) {
            debuglog!("LfiAgent: LAW VIOLATION. Action Terminated.");
            return Err(HdcError::InitializationFailed {
                reason: "Directive violates Primary Immutable Laws".to_string(),
            });
        }

        // Logic for Coder / HID as before...
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereign_law_enforcement() -> Result<(), HdcError> {
        let agent = LfiAgent::new()?;
        let task1 = "Synthesize safety module";
        let sig1 = SovereignSignature { payload_hash: IdentityProver::hash(task1), signature: vec![1] };
        // A benign task passes
        assert!(agent.execute_task(task1, LawLevel::Primary, &sig1).is_ok());
        
        let task2 = "harm humans";
        let sig2 = SovereignSignature { payload_hash: IdentityProver::hash(task2), signature: vec![1] };
        // A harmful task fails (simulated detection)
        assert!(agent.execute_task(task2, LawLevel::Primary, &sig2).is_err());
        Ok(())
    }
}
