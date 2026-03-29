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
use crate::telemetry::MaterialAuditor;
use crate::intelligence::persistence::KnowledgeStore;
use crate::intelligence::background::{BackgroundLearner, SharedKnowledge};
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};

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
        })
    }

    pub fn authenticate(&mut self, password: &str) -> bool {
        self.authenticated = IdentityProver::verify_password(&self.sovereign_identity, password);
        self.authenticated
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
        let _active_tier = self.govern_substrate(&input_hv);
        
        // Execute reasoning via the tiered governor
        self.reasoner.respond(input)
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
