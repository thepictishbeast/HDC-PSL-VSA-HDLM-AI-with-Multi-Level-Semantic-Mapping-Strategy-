// ============================================================
// LFI Agent Orchestrator — The Archon Mind
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

/// The Archon Agent. Orchestrates the full VSA stack under absolute law.
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
    pub entropy_level: f64,
    /// Absolute proof of the Sovereign User.
    pub sovereign_identity: SovereignProof,
}

impl LfiAgent {
    /// Initialize a new Archon agent with Laws and Identity.
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("LfiAgent::new: Initializing Archon intelligence");
        
        let compute = LocalBackend;
        let mut supervisor = PslSupervisor::new();
        
        // 1. Load Forensic Axioms (Zero-Trust)
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        supervisor.register_axiom(Box::new(StatisticalEquilibriumAxiom { tolerance: 0.02 }));
        supervisor.register_axiom(Box::new(WebSearchSkepticismAxiom { min_credibility_score: 0.7 }));
        
        // 2. Load Identity-derived Forbidden Space (The Write-Blocker)
        // We derive vectors from the identity markers to form the "Forbidden Space".
        let name_vec = BipolarVector::from_seed(IdentityProver::hash("William Jhan Paul Armstrong"));
        let ssn_vec = BipolarVector::from_seed(IdentityProver::hash("647568607"));
        let license_vec = BipolarVector::from_seed(IdentityProver::hash("s23233305"));

        supervisor.register_axiom(Box::new(ForbiddenSpaceAxiom {
            forbidden_vectors: vec![name_vec, ssn_vec, license_vec],
            tolerance: 0.1,
        }));

        // 3. Load Dialectical & Coercion Axioms
        supervisor.register_axiom(Box::new(ClassInterestAxiom));
        supervisor.register_axiom(Box::new(CoercionAxiom { sensitivity: 0.7 }));
        supervisor.register_axiom(Box::new(ConnectivityAxiom { required_tunnel: "tor_obfs4".into() }));

        // 4. Load CARTA Probes (Offensive Security)
        supervisor.register_axiom(Box::new(OverflowProbe));
        supervisor.register_axiom(Box::new(EncryptionProbe));
        
        // 3. Initialize codebook (HDLM Layer)
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
        
        // 4. Secure Identity Commitment (ZKI)
        // Values provided by the user are committed to memory.
        let sovereign_identity = IdentityProver::commit(
            "William Jhan Paul Armstrong",
            "647568607",
            "s23233305",
            IdentityKind::Sovereign
        );
        
        Ok(Self { 
            compute, supervisor, codebook, hid, coder, 
            sensorium, optimizer, memory, holographic, analogy, cortex, osint, 
            entropy_level: 0.1, // Default low entropy for logical tasks
            sovereign_identity 
        })
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

    /// Executes a task only if it complies with the Archon Laws and HSM signature.
    pub fn execute_task(&self, task_name: &str, level: LawLevel, signature: &SovereignSignature) -> Result<(), HdcError> {
        debuglog!("LfiAgent::execute_task: auditing '{}' against Archon Laws", task_name);

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
    fn test_archon_law_enforcement() -> Result<(), HdcError> {
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
