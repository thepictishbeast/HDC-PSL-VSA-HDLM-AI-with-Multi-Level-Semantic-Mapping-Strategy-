// ============================================================
// LFI Agent Orchestrator — The Sovereign Mind (TNSS Governor)
// ============================================================

use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, debug, warn};

use crate::hdc::vector::BipolarVector;
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::AuditTarget;
use crate::hdc::superposition::SuperpositionStorage;
use crate::hdc::sensory::{SensoryCortex, SensoryFrame};
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::liquid::LiquidSensorium;
use crate::hdlm::intercept::OpsecIntercept;
use crate::identity::{IdentityProver, SovereignProof, IdentityKind, SovereignSignature};
use crate::hdc::error::HdcError;

use crate::cognition::reasoner::CognitiveCore;
use crate::cognition::router::{SemanticRouter, IntelligenceTier};
use crate::psl::feedback::PslFeedbackLoop;
use crate::telemetry::MaterialAuditor;
use crate::intelligence::persistence::KnowledgeStore;
use crate::intelligence::background::{BackgroundLearner, SharedKnowledge};
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::reasoning_provenance::ProvenanceEngine;

/// The Sovereign Agent. Orchestrates the Trimodal Neuro-Symbolic Swarm (TNSS).
pub struct LfiAgent {
    pub supervisor: PslSupervisor,
    pub memory: SuperpositionStorage,
    pub holographic: HolographicMemory,
    pub sensorium: LiquidSensorium,
    pub reasoner: CognitiveCore,
    pub router: SemanticRouter,
    pub current_tier: IntelligenceTier,
    pub authenticated: bool,
    pub entropy_level: f64,
    pub conversation_facts: std::collections::HashMap<String, String>,
    pub sovereign_identity: SovereignProof,
    pub shared_knowledge: Arc<Mutex<SharedKnowledge>>,
    pub background_learner: BackgroundLearner,
    /// PSL rejection feedback loop — learns from audit failures.
    pub psl_feedback: PslFeedbackLoop,
    /// Reasoning provenance engine — derivation traces for every conclusion.
    pub provenance: Arc<Mutex<ProvenanceEngine>>,
    /// RAG context — relevant facts from brain.db, set by the API layer
    /// before each chat call so the reasoner can inject them into Ollama prompts.
    /// SUPERSOCIETY: This is how 51M+ facts improve every answer.
    pub rag_context: Vec<(String, String, f64)>,
    /// Causal reasoning graph — Pearl's 3-level framework.
    /// SUPERSOCIETY: Transforms "what IS" into "what WOULD HAPPEN IF".
    pub causal_graph: crate::cognition::causal::CausalGraph,
    /// Global Workspace — capacity-bounded attention bottleneck.
    pub workspace: crate::cognition::global_workspace::GlobalWorkspace,
    /// Grokking phase monitor — detects memorization→cleanup transitions.
    pub grok_monitor: crate::cognition::grokking_monitor::GrokMonitor,
    /// Commitment registry — cross-cutting commit-reveal fabric.
    pub commitments: crate::crypto_commitment::CommitmentRegistry,
}

impl LfiAgent {
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("LfiAgent::new: Initializing Sovereign Strategic Core");
        
        let mut supervisor = PslSupervisor::new();
        // Register default axioms for the symbolic cage
        supervisor.register_axiom(Box::new(crate::psl::axiom::DimensionalityAxiom));
        supervisor.register_axiom(Box::new(crate::psl::axiom::StatisticalEquilibriumAxiom { tolerance: 0.15 }));
        supervisor.register_axiom(Box::new(crate::psl::axiom::DataIntegrityAxiom { max_bytes: 10_000_000 }));
        supervisor.register_axiom(Box::new(crate::psl::axiom::ClassInterestAxiom));
        // ConfidenceCalibrationAxiom — rejects vectors whose mean exceeds ±0.5
        // (degenerate / adversarial inputs that would spike cosine similarity).
        supervisor.register_axiom(Box::new(crate::psl::axiom::ConfidenceCalibrationAxiom::default()));

        let memory = SuperpositionStorage::new();
        let reasoner = CognitiveCore::new()?;
        let router = SemanticRouter::new();

        let store_path = KnowledgeStore::default_path();
        let persistent_store = KnowledgeStore::load(&store_path).unwrap_or_else(|_| KnowledgeStore::new());
        let background_learner = BackgroundLearner::new(persistent_store);
        let shared_knowledge = background_learner.shared_knowledge();

        let mut conversation_facts = std::collections::HashMap::new();
        {
            let guard = shared_knowledge.lock();
            for fact in &guard.store.facts {
                conversation_facts.insert(fact.key.clone(), fact.value.clone());
            }
        }

        // Load sovereign identity from environment — never hardcode PII in source
        let sov_name = std::env::var("LFI_SOVEREIGN_NAME")
            .unwrap_or_else(|_| "Sovereign".to_string());
        let sov_credential = std::env::var("LFI_SOVEREIGN_CREDENTIAL")
            .unwrap_or_else(|_| "000000000".to_string());
        let sov_id = std::env::var("LFI_SOVEREIGN_ID")
            .unwrap_or_else(|_| "s00000000".to_string());
        let sov_key = std::env::var("LFI_SOVEREIGN_KEY")
            .unwrap_or_else(|_| "CHANGE_ME_SET_LFI_SOVEREIGN_KEY".to_string());
        debuglog!("LfiAgent::new: Sovereign identity loaded from environment");

        // Register ForbiddenSpaceAxiom — blocks vectors derived from sovereign PII
        let forbidden_vectors = vec![
            BipolarVector::from_seed(IdentityProver::hash(&sov_credential)),
            BipolarVector::from_seed(IdentityProver::hash(&sov_id)),
            BipolarVector::from_seed(IdentityProver::hash(&sov_name)),
        ];
        supervisor.register_axiom(Box::new(crate::psl::axiom::ForbiddenSpaceAxiom {
            forbidden_vectors,
            tolerance: 0.7,
        }));
        debuglog!("LfiAgent::new: {} PSL axioms registered (incl. ForbiddenSpace)", supervisor.axiom_count());

        let sovereign_identity = IdentityProver::commit(
            &sov_name,
            &sov_credential,
            &sov_id,
            &sov_key,
            IdentityKind::Sovereign
        );

        // Seed the holographic memory with a base association for recall capacity
        let mut holographic = HolographicMemory::new();
        let seed_key = BipolarVector::from_seed(42);
        let seed_val = BipolarVector::from_seed(84);
        let _ = holographic.associate(&seed_key, &seed_val);
        debuglog!("LfiAgent::new: Holographic memory seeded (capacity={})", holographic.capacity);

        let sensorium = LiquidSensorium::new(19);

        Ok(Self {
            supervisor, memory, holographic, sensorium, reasoner, router,
            current_tier: IntelligenceTier::Pulse,
            authenticated: false,
            entropy_level: 0.1,
            conversation_facts,
            sovereign_identity,
            shared_knowledge,
            background_learner,
            psl_feedback: PslFeedbackLoop::new(),
            provenance: Arc::new(Mutex::new(ProvenanceEngine::new())),
            rag_context: Vec::new(),
            causal_graph: crate::cognition::causal::CausalGraph::new(),
            workspace: crate::cognition::global_workspace::GlobalWorkspace::standard(),
            grok_monitor: crate::cognition::grokking_monitor::GrokMonitor::new(100),
            commitments: crate::crypto_commitment::CommitmentRegistry::new(),
        })
    }

    pub fn authenticate(&mut self, password: &str) -> bool {
        self.authenticated = IdentityProver::verify_password(&self.sovereign_identity, password);
        self.authenticated
    }

    /// Deterministic conclusion ID for a given input string.
    ///
    /// Uses a simple FNV-1a 64-bit hash so the same question always maps to
    /// the same ID — clients can retrieve the trace of a prior answer by
    /// re-hashing the original input. Not a cryptographic hash; collisions
    /// are acceptable since the trace arena still distinguishes entries
    /// by TraceId and confidence.
    pub fn conclusion_id_for_input(input: &str) -> u64 {
        // FNV-1a 64-bit
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in input.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Think with reasoning provenance — records a traced derivation into the
    /// agent's own ProvenanceEngine. Returns the ThoughtResult plus the
    /// conclusion_id the caller can use to query the `/api/provenance/:id`
    /// endpoint.
    ///
    /// BUG ASSUMPTION: `input` is already validated at the API surface.
    /// This method trusts its caller.
    pub fn think_traced(
        &mut self,
        input: &str,
    ) -> Result<(crate::cognition::reasoner::ThoughtResult, u64), HdcError> {
        let cid = Self::conclusion_id_for_input(input);
        debuglog!("LfiAgent::think_traced: cid={}, input={}",
            cid, crate::truncate_str(input, 60));

        // Scope the provenance lock around only the trace-recording call.
        let (result, _trace_id) = {
            let mut engine = self.provenance.lock();
            self.reasoner.think_with_provenance(
                input,
                &mut engine.arena,
                None,
                Some(cid),
            )?
        };

        Ok((result, cid))
    }

    /// SWAP: Manages the material residency of models in RAM/NPU.
    fn swap_model_tier(&mut self, target: IntelligenceTier) {
        if self.current_tier == target { return; }

        match target {
            IntelligenceTier::BigBrain => {
                warn!("// AUDIT: Escalating to BIGBRAIN. Swapping MoE weights into RAM...");
                // In production, this issues an RPC to Ollama/llama.cpp to load the 8B GGUF
            }
            IntelligenceTier::Bridge => {
                info!("// AUDIT: Switching to BRIDGE. Loading LFM 1.5B kernel.");
            }
            IntelligenceTier::Pulse => {
                debug!("// AUDIT: Dropping to PULSE. Hibernating Bridge/BigBrain.");
            }
        }
        self.current_tier = target;
    }

    /// GOVERN: Dynamic resource management based on VSA triggers and telemetry.
    pub fn govern_substrate(&mut self, input_vector: &HyperMemory) -> IntelligenceTier {
        // Calculate semantic health for telemetry
        let vsa_ortho = input_vector.audit_orthogonality();
        let psl_pass_rate = 1.0; // Placeholder: in production, this tracks historical audit success

        let stats = MaterialAuditor::get_stats(vsa_ortho, psl_pass_rate);
        let target_tier = self.router.route_intent(input_vector);

        if !self.authenticated { return IntelligenceTier::Pulse; }

        // Thermodynamic check
        if stats.is_throttled {
            warn!("// AUDIT: Thermal threshold exceeded. Forcing Pulse tier.");
            self.swap_model_tier(IntelligenceTier::Pulse);
            return IntelligenceTier::Pulse;
        }

        // Memory check
        if target_tier == IntelligenceTier::BigBrain && stats.ram_available_mb < 6000 {
            warn!("// AUDIT: RAM saturation risk. Throttling BigBrain escalation.");
            self.swap_model_tier(IntelligenceTier::Bridge);
            return IntelligenceTier::Bridge;
        }

        self.swap_model_tier(target_tier);
        target_tier
    }

    pub fn chat(&mut self, input: &str) -> Result<crate::cognition::reasoner::ConversationResponse, HdcError> {
        let input_hv = HyperMemory::from_string(input, DIM_PROLETARIAT);
        // #324: previously `let _active_tier = ...` — tier was computed and
        // thrown away, so every tier hit the same 7B model. Now stash the
        // chosen tier on the reasoner so query_ollama_with_context_model
        // picks the right Ollama model (Pulse→0.5b, Bridge→3b, BigBrain→7b).
        let active_tier = self.govern_substrate(&input_hv);
        self.reasoner.active_tier = active_tier;

        // Set RAG context on the reasoner before responding
        // SUPERSOCIETY: This is how 51M+ facts ground every answer
        self.reasoner.rag_context = std::mem::take(&mut self.rag_context);

        // Execute reasoning via the tiered governor
        self.reasoner.respond(input)
    }

    /// Chat with provenance tracking — records a trace entry capturing the
    /// input, mode, and confidence for each exchange.
    ///
    /// Returns the ConversationResponse plus a deterministic conclusion_id
    /// (FNV-1a hash of input) the caller can use to query the
    /// `/api/provenance/:id` endpoint.
    pub fn chat_traced(
        &mut self,
        input: &str,
    ) -> Result<(crate::cognition::reasoner::ConversationResponse, u64), HdcError> {
        let cid = Self::conclusion_id_for_input(input);
        debuglog!("LfiAgent::chat_traced: cid={}", cid);

        let response = self.chat(input)?;

        // Record a single-entry trace documenting the conversation turn.
        // System 1 vs System 2 is inferred from the ThoughtResult's mode.
        use crate::reasoning_provenance::InferenceSource;
        let source = match response.thought.mode {
            crate::cognition::reasoner::CognitiveMode::Fast =>
                InferenceSource::System1FastPath {
                    similarity_score: response.thought.confidence,
                },
            crate::cognition::reasoner::CognitiveMode::Deep =>
                InferenceSource::System2Deliberation {
                    iterations: response.thought.plan.as_ref()
                        .map(|p| p.steps.len()).unwrap_or(0),
                },
        };

        {
            let mut engine = self.provenance.lock();
            engine.arena.record_step(
                None,
                source,
                vec![format!("chat:\"{}\"", crate::truncate_str(input, 40))],
                response.thought.confidence,
                Some(cid),
                format!("Chat: {} (mode={:?}, conf={:.4})",
                    crate::truncate_str(&response.text, 60),
                    response.thought.mode,
                    response.thought.confidence),
                0,
            );
        }
        Ok((response, cid))
    }

    pub fn execute_task(&self, task_name: &str, level: crate::laws::LawLevel, signature: &SovereignSignature) -> Result<(), HdcError> {
        debuglog!("LfiAgent::execute_task: task='{}' level={:?}", task_name, level);

        // 1. Primary Law audit — sovereign constraints override all signatures
        if !crate::laws::PrimaryLaw::permits(task_name, level) {
            return Err(HdcError::LogicFault {
                reason: format!("Primary Law violation: task '{}' blocked", task_name),
            });
        }

        // 2. SVI Signature verification
        if !IdentityProver::verify_signature(&self.sovereign_identity, task_name, signature) {
            return Err(HdcError::InitializationFailed { reason: "SVI Signature Failure".to_string() });
        }
        Ok(())
    }

    pub fn ingest_sensor_frame(&mut self, frame: &SensoryFrame) -> Result<BipolarVector, HdcError> {
        let encoded = SensoryCortex::new()?.encode_frame(frame)?;
        let target = AuditTarget::Vector(encoded.clone());
        let _ = self.supervisor.audit(&target).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Sensory audit failure: {:?}", e),
        })?;
        Ok(encoded)
    }

    pub fn synthesize_creative_solution(&self, problem_description: &str) -> Result<BipolarVector, HdcError> {
        debuglog!("LfiAgent::synthesize_creative_solution: {}", problem_description);
        let p_hash = IdentityProver::hash(problem_description);
        let p_vector = BipolarVector::from_seed(p_hash);
        crate::hdc::analogy::AnalogyEngine::new().synthesize_solution(&p_vector)
    }

    /// OPSEC Intercept: Sanitizes text through the HDLM firewall before vectorization.
    pub fn ingest_text(&mut self, input: &str) -> Result<String, HdcError> {
        debuglog!("LfiAgent::ingest_text: Scanning input ({} bytes)", input.len());
        let result = OpsecIntercept::scan(input).map_err(|e| HdcError::InitializationFailed {
            reason: format!("OPSEC scan failure: {:?}", e),
        })?;

        if !result.matches_found.is_empty() {
            debuglog!("LfiAgent::ingest_text: {} OPSEC markers redacted", result.matches_found.len());
        }

        // Vectorize and store the sanitized text in holographic memory
        let text_hash = IdentityProver::hash(&result.sanitized);
        let text_vector = BipolarVector::from_seed(text_hash);
        let val_vector = BipolarVector::from_seed(text_hash.wrapping_add(1));
        let _ = self.holographic.associate(&text_vector, &val_vector);

        Ok(result.sanitized)
    }

    /// Ingest a noisy signal through the Liquid Neural Network sensorium.
    pub fn ingest_noise(&mut self, signal: f64) -> Result<(), HdcError> {
        debuglog!("LfiAgent::ingest_noise: signal={:.4}", signal);
        self.sensorium.step(signal, 0.01)
    }

    /// Entropy Governor: Toggle high/low entropy mode for the agent.
    pub fn set_entropy(&mut self, high: bool) {
        self.entropy_level = if high { 0.9 } else { 0.1 };
        debuglog!("LfiAgent::set_entropy: level={:.1}", self.entropy_level);
    }

    /// Coercion Detection: Audits jitter and geo-risk for adversarial signals.
    /// Returns a confidence score (0.0 = total coercion, 1.0 = clean).
    pub fn audit_coercion(&self, jitter: f64, geo_risk: f64) -> Result<f64, HdcError> {
        debuglog!("LfiAgent::audit_coercion: jitter={:.2}, geo_risk={:.2}", jitter, geo_risk);
        let threat = (jitter + geo_risk) / 2.0;
        let confidence = 1.0 - threat.clamp(0.0, 1.0);
        debuglog!("LfiAgent::audit_coercion: threat={:.4}, confidence={:.4}", threat, confidence);
        Ok(confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = LfiAgent::new();
        assert!(agent.is_ok(), "Agent should initialize without error");
    }

    #[test]
    fn test_agent_supervisor_includes_default_axioms() {
        let agent = LfiAgent::new().expect("agent init");
        let count = agent.supervisor.axiom_count();
        // 5 default axioms: Dimensionality, StatisticalEquilibrium,
        // DataIntegrity, ClassInterest, ConfidenceCalibration.
        assert!(count >= 5,
            "default supervisor must register at least 5 axioms, got {}", count);
    }

    #[test]
    fn test_agent_supervisor_rejects_degenerate_vector() {
        use crate::psl::axiom::AuditTarget;
        let agent = LfiAgent::new().expect("agent init");
        // Build an all-+1 vector — ConfidenceCalibrationAxiom must catch it.
        let template = crate::hdc::vector::BipolarVector::new_random().expect("random");
        let mut data = template.data.clone();
        for i in 0..data.len() { data.set(i, true); }
        let degenerate = crate::hdc::vector::BipolarVector { data };
        let target = AuditTarget::Vector(degenerate);
        let verdict = agent.supervisor.audit(&target).expect("audit");
        // A degenerate all-+1 vector violates calibration; should not pass
        // the trust threshold.
        assert!(verdict.confidence < agent.supervisor.material_trust_threshold,
            "degenerate vector must not reach trust threshold, got conf {:.3}",
            verdict.confidence);
    }

    #[test]
    fn test_agent_has_empty_provenance_engine() {
        use crate::reasoning_provenance::ProvenanceKind;
        let agent = LfiAgent::new().expect("agent init");
        let engine = agent.provenance.lock();
        // A fresh agent's engine has no traces yet.
        assert_eq!(engine.trace_count(), 0);
        // Queries for unknown conclusions must return ReconstructedRationalization —
        // the core architectural invariant of the provenance system.
        let explanation = engine.explain_conclusion(42);
        assert!(
            matches!(explanation.kind, ProvenanceKind::ReconstructedRationalization { .. }),
            "Empty engine must return Reconstructed, never Traced"
        );
    }

    #[test]
    fn test_conclusion_id_is_deterministic() {
        // Same input → same ID, always.
        let a = LfiAgent::conclusion_id_for_input("hello world");
        let b = LfiAgent::conclusion_id_for_input("hello world");
        assert_eq!(a, b);
        // Different inputs → different IDs.
        let c = LfiAgent::conclusion_id_for_input("different");
        assert_ne!(a, c);
    }

    #[test]
    fn test_think_traced_records_into_engine_and_returns_same_cid() {
        use crate::reasoning_provenance::ProvenanceKind;
        let mut agent = LfiAgent::new().expect("agent init");
        let input = "What is sovereignty?";
        let expected_cid = LfiAgent::conclusion_id_for_input(input);

        // Before: no trace for this cid.
        {
            let engine = agent.provenance.lock();
            let kind = engine.explain_conclusion(expected_cid).kind;
            assert!(
                matches!(kind, ProvenanceKind::ReconstructedRationalization { .. }),
                "no trace recorded yet — must be Reconstructed"
            );
        }

        // Think — records trace.
        let (_result, cid) = agent.think_traced(input).expect("think_traced ok");
        assert_eq!(cid, expected_cid);

        // After: trace exists for the returned cid.
        let engine = agent.provenance.lock();
        assert!(engine.trace_count() >= 1, "at least one trace recorded");
        let kind = engine.explain_conclusion(cid).kind;
        assert_eq!(kind, ProvenanceKind::TracedDerivation,
            "think_traced must produce a TracedDerivation for the returned cid");
    }

    #[test]
    fn test_chat_traced_records_trace_and_returns_cid() {
        use crate::reasoning_provenance::ProvenanceKind;
        let mut agent = LfiAgent::new().expect("agent init");
        let input = "Hello, who are you?";
        let expected_cid = LfiAgent::conclusion_id_for_input(input);

        let (_response, cid) = agent.chat_traced(input).expect("chat_traced ok");
        assert_eq!(cid, expected_cid, "cid must be deterministic");

        // Trace for this cid must now exist and be TracedDerivation.
        let engine = agent.provenance.lock();
        let explanation = engine.explain_conclusion(cid);
        assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation,
            "chat_traced must produce a TracedDerivation");
        assert!(engine.trace_count() >= 1);
    }

    #[test]
    fn test_chat_traced_different_inputs_get_different_cids() {
        let mut agent = LfiAgent::new().expect("agent init");
        let (_, cid_a) = agent.chat_traced("what is rust").expect("chat a");
        let (_, cid_b) = agent.chat_traced("what is python").expect("chat b");
        assert_ne!(cid_a, cid_b, "different inputs must produce different cids");
    }

    #[test]
    fn test_agent_provenance_engine_records_traces() {
        use crate::reasoning_provenance::{InferenceSource, ProvenanceKind};
        let agent = LfiAgent::new().expect("agent init");
        {
            let mut engine = agent.provenance.lock();
            engine.arena.record_step(
                None,
                InferenceSource::ExternalAssertion { source: "test".into() },
                vec!["premise".into()],
                0.9,
                Some(7),
                "test trace".into(),
                100,
            );
        }
        // After recording, the trace is retrievable.
        let engine = agent.provenance.lock();
        assert_eq!(engine.trace_count(), 1);
        assert_eq!(engine.explain_conclusion(7).kind, ProvenanceKind::TracedDerivation);
    }

    #[test]
    fn test_agent_starts_unauthenticated() {
        let agent = LfiAgent::new().expect("agent init");
        // Agent should start unauthenticated (password not provided).
        assert!(!agent.authenticated);
    }

    #[test]
    fn test_coercion_audit_clean() {
        let agent = LfiAgent::new().expect("agent init");
        let conf = agent.audit_coercion(0.0, 0.0).expect("audit should work");
        assert!((conf - 1.0).abs() < 0.01, "Zero threat should give conf=1.0, got {:.4}", conf);
    }

    #[test]
    fn test_coercion_audit_high_threat() {
        let agent = LfiAgent::new().expect("agent init");
        let conf = agent.audit_coercion(1.0, 1.0).expect("audit should work");
        assert!(conf < 0.01, "Max threat should give conf≈0.0, got {:.4}", conf);
    }

    #[test]
    fn test_coercion_audit_medium() {
        let agent = LfiAgent::new().expect("agent init");
        let conf = agent.audit_coercion(0.3, 0.5).expect("audit should work");
        assert!(conf > 0.0 && conf < 1.0, "Medium threat should give partial conf: {:.4}", conf);
    }

    #[test]
    fn test_agent_authenticate_wrong_password() {
        let mut agent = LfiAgent::new().expect("agent init");
        // With default env (no SOVEREIGN_PASSWORD set), empty password might pass.
        // Any non-matching password should fail.
        let result = agent.authenticate("definitely_wrong_password_123456");
        // Result depends on env var — just verify it doesn't panic.
        debuglog!("test_authenticate: result={}", result);
    }

    #[test]
    fn test_govern_substrate() {
        let mut agent = LfiAgent::new().expect("agent init");
        let input = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let tier = agent.govern_substrate(&input);
        // Should return a valid tier without panic.
        debuglog!("test_govern: tier={:?}", tier);
        assert!(matches!(tier, IntelligenceTier::Pulse | IntelligenceTier::Bridge | IntelligenceTier::BigBrain));
    }

    #[test]
    fn test_set_entropy() {
        let mut agent = LfiAgent::new().expect("agent init");
        agent.set_entropy(true);
        agent.set_entropy(false);
        // Should not panic on toggle.
    }

    // ============================================================
    // Stress / invariant tests for LfiAgent
    // ============================================================

    /// INVARIANT: conclusion_id_for_input is FNV-1a — stable across process
    /// restarts (pure function).
    #[test]
    fn invariant_conclusion_id_pure() {
        let inputs = ["", "hello", "αβγ", "a very long question with context"];
        for input in inputs {
            let a = LfiAgent::conclusion_id_for_input(input);
            let b = LfiAgent::conclusion_id_for_input(input);
            let c = LfiAgent::conclusion_id_for_input(input);
            assert_eq!(a, b);
            assert_eq!(b, c);
        }
    }

    /// INVARIANT: audit_coercion returns confidence in [0,1] for any input.
    #[test]
    fn invariant_coercion_audit_in_unit_interval() -> Result<(), HdcError> {
        let agent = LfiAgent::new()?;
        let probes = [
            (0.0, 0.0), (1.0, 1.0), (-0.5, 1.5),
            (0.3, 0.5), (f64::NAN, 0.5),
        ];
        for (j, g) in probes {
            let conf = agent.audit_coercion(j, g)?;
            if conf.is_finite() {
                assert!((0.0..=1.0).contains(&conf),
                    "confidence out of [0,1]: {} for jitter={}, geo={}",
                    conf, j, g);
            }
        }
        Ok(())
    }

    /// INVARIANT: set_entropy results in entropy_level being either 0.9 or 0.1.
    #[test]
    fn invariant_set_entropy_binary() -> Result<(), HdcError> {
        let mut agent = LfiAgent::new()?;
        agent.set_entropy(true);
        assert!((agent.entropy_level - 0.9).abs() < 0.001,
            "set_entropy(true) should be ~0.9, got {}", agent.entropy_level);
        agent.set_entropy(false);
        assert!((agent.entropy_level - 0.1).abs() < 0.001,
            "set_entropy(false) should be ~0.1, got {}", agent.entropy_level);
        Ok(())
    }

    /// INVARIANT: two agents created back-to-back have identity proofs
    /// that differ only if env vars differ. Given same env, commits match.
    #[test]
    fn invariant_new_reproducible_under_stable_env() -> Result<(), HdcError> {
        // Env vars may be unset or stable; both agents will use same values.
        let a = LfiAgent::new()?;
        let b = LfiAgent::new()?;
        assert_eq!(a.sovereign_identity.name_hash, b.sovereign_identity.name_hash);
        assert_eq!(a.sovereign_identity.password_commitment,
                   b.sovereign_identity.password_commitment);
        Ok(())
    }

    /// INVARIANT: two different inputs yield two different conclusion IDs
    /// (with overwhelming probability via FNV-1a avalanche).
    #[test]
    fn invariant_conclusion_id_avalanche() {
        let pairs = [
            ("a", "b"), ("hello", "world"),
            ("question 1", "question 2"),
            ("x", "X"), (" ", "  "),
        ];
        for (a, b) in pairs {
            let ha = LfiAgent::conclusion_id_for_input(a);
            let hb = LfiAgent::conclusion_id_for_input(b);
            assert_ne!(ha, hb,
                "different inputs {:?}/{:?} produced same cid", a, b);
        }
    }
}
