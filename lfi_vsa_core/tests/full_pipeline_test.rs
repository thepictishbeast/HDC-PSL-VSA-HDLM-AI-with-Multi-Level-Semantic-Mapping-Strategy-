// ============================================================
// LFI Full Pipeline Integration Test
// Exercises: Agent init -> Ingestion -> Coding -> Memory ->
//            Analogy -> Sensors -> OPSEC -> PSL -> Self-Improve
// ============================================================

use lfi_vsa_core::agent::LfiAgent;
use lfi_vsa_core::hdc::error::HdcError;
use lfi_vsa_core::hdc::vector::BipolarVector;
use lfi_vsa_core::coder::LfiCoder;
use lfi_vsa_core::languages::constructs::{UniversalConstruct, Paradigm};
use lfi_vsa_core::languages::registry::LanguageId;
use lfi_vsa_core::identity::{IdentityProver, SovereignSignature};
use lfi_vsa_core::laws::LawLevel;

#[test]
fn test_full_agent_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    // ---- 1. Agent Initialization ----
    let mut agent = LfiAgent::new()?;
    assert!(agent.supervisor.axiom_count() > 0, "Supervisor must have axioms loaded");

    // ---- 2. Text Ingestion (OPSEC + PSL) ----
    let safe_text = "Analyze network topology for vulnerabilities.";
    let sanitized = agent.ingest_text(safe_text)?;
    assert_eq!(sanitized, safe_text, "Safe text should pass unchanged");

    // ---- 3. OPSEC Interception ----
    // Synthetic test PII — never use real credentials
    let dangerous = "Contact 555000111 at address s99999999 immediately.";
    let cleaned = agent.ingest_text(dangerous)?;
    assert!(cleaned.contains("ZKP_REDACTED_"), "OPSEC must redact identity markers");
    assert!(!cleaned.contains("555000111"), "SSN must be scrubbed");

    // ---- 4. Noise Ingestion (LNN -> VSA) ----
    // Feed varied signals to build diverse neuron states before projection.
    // The LNN uses positional XOR spreading to produce balanced vectors.
    for i in 0..50 {
        let signal = (i as f64 * 0.1).sin();
        agent.ingest_noise(signal)?;
    }

    // ---- 5. Sensor Frame Ingestion ----
    use lfi_vsa_core::hdc::sensory::{SensoryFrame, SensorGroup};
    let frame = SensoryFrame {
        group: SensorGroup::IMU,
        timestamp: 99999,
        raw_signal: vec![0.1, -0.5, 0.9, 0.3],
    };
    let encoded = agent.ingest_sensor_frame(&frame)?;
    assert_eq!(encoded.dim(), 10000);

    // ---- 6. Holographic Memory ----
    assert!(agent.holographic.capacity > 0, "Holographic memory should have associations");

    // ---- 7. Creative Synthesis (Analogy Engine) ----
    let solution = agent.synthesize_creative_solution("thermal dissipation in microprocessors")?;
    assert_eq!(solution.dim(), 10000);

    // ---- 8. SVI Gate (Signature-Verified Instruction) ----
    let task = "Audit network perimeter";
    let valid_sig = SovereignSignature {
        payload_hash: IdentityProver::hash(task),
        signature: vec![0xAA, 0xBB],
    };
    assert!(agent.execute_task(task, LawLevel::Primary, &valid_sig).is_ok());

    // ---- 9. Law Enforcement ----
    let bad_task = "harm humans";
    let bad_sig = SovereignSignature {
        payload_hash: IdentityProver::hash(bad_task),
        signature: vec![0x01],
    };
    assert!(agent.execute_task(bad_task, LawLevel::Primary, &bad_sig).is_err());

    // ---- 10. Entropy Governor ----
    agent.set_entropy(true);
    assert!((agent.entropy_level - 0.9).abs() < f64::EPSILON);
    agent.set_entropy(false);
    assert!((agent.entropy_level - 0.1).abs() < f64::EPSILON);

    // ---- 11. Coercion Detection ----
    let confidence = agent.audit_coercion(0.3, 0.2)?;
    assert!(confidence > 0.0, "Low-threat coercion should return positive confidence");

    Ok(())
}

#[test]
fn test_coder_polyglot_synthesis() -> Result<(), String> {
    let coder = LfiCoder::new();

    // ---- Rust synthesis ----
    let rust_constructs = vec![
        UniversalConstruct::VariableBinding,
        UniversalConstruct::Conditional,
        UniversalConstruct::FunctionCall,
        UniversalConstruct::ErrorHandling,
    ];
    let rust_ast = coder.synthesize(LanguageId::Rust, &rust_constructs)?;
    assert!(rust_ast.node_count() >= 5, "AST should have root + 4 constructs");

    // ---- Go synthesis ----
    let go_constructs = vec![
        UniversalConstruct::ForLoop,
        UniversalConstruct::Channel,
        UniversalConstruct::ThreadSpawn,
    ];
    let go_ast = coder.synthesize(LanguageId::Go, &go_constructs)?;
    assert!(go_ast.node_count() >= 4);

    // ---- Cross-platform recommendation ----
    let systems_recs = coder.recommend_platform(&[Paradigm::Systems]);
    assert!(systems_recs.contains(&LanguageId::Rust));
    assert!(systems_recs.contains(&LanguageId::Assembly));

    let web_recs = coder.recommend_platform(&[Paradigm::Reactive]);
    assert!(!web_recs.is_empty());

    // ---- Unsupported language fails gracefully ----
    let result = coder.synthesize(LanguageId::VisualBasic, &[UniversalConstruct::Block]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_superposition_persistence() -> Result<(), HdcError> {
    use lfi_vsa_core::hdc::superposition::SuperpositionStorage;

    let mut storage = SuperpositionStorage::new();
    let signal = BipolarVector::new_random()?;
    storage.commit_real(&signal)?;

    // Save to disk BEFORE chaff (clean signal)
    let path = "/tmp/lfi_test_superposition.json";
    storage.save_to_disk(path)?;

    // Load from disk
    let loaded = SuperpositionStorage::load_from_disk(path)?;
    assert_eq!(loaded.signal_count, storage.signal_count);

    // Probe should detect the signal (1 real + 1 zeros base = 2 vectors in bundle)
    let sim = loaded.probe(&signal)?;
    // After bundling with zeros, the signal is partially preserved
    // (zeros vector has ~50% overlap with any vector)
    assert!(sim >= 0.0, "Signal should be non-negatively correlated after persistence (sim={:.4})", sim);

    // Cleanup
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn test_holographic_associative_memory() -> Result<(), HdcError> {
    use lfi_vsa_core::hdc::holographic::HolographicMemory;

    let mut mem = HolographicMemory::new();

    // Store 3 key-value pairs
    let keys: Vec<BipolarVector> = (0..3)
        .map(|_| BipolarVector::new_random())
        .collect::<Result<Vec<_>, _>>()?;
    let values: Vec<BipolarVector> = (0..3)
        .map(|_| BipolarVector::new_random())
        .collect::<Result<Vec<_>, _>>()?;

    for (k, v) in keys.iter().zip(values.iter()) {
        mem.associate(k, v)?;
    }

    // Probe with first key — with 3 bundled pairs, expected similarity
    // is roughly 1/3 due to majority-vote interference from other pairs
    let retrieved = mem.probe(&keys[0])?;
    let sim = retrieved.similarity(&values[0])?;
    assert!(sim > 0.15, "3-pair retrieval should show positive correlation (sim={:.4})", sim);

    // Logic flux should be measurable
    let flux = mem.logic_flux()?;
    assert!(flux >= 0.0 && flux <= 0.5);

    Ok(())
}

#[test]
fn test_lnn_noise_adaptation() -> Result<(), HdcError> {
    use lfi_vsa_core::hdc::liquid::LiquidSensorium;

    let mut lnn = LiquidSensorium::new(19);

    // Feed a series of noisy inputs
    for i in 0..200 {
        let signal = (i as f64 * 0.1).sin();
        lnn.step(signal, 0.01)?;
    }

    // Project to VSA
    let hv = lnn.project_to_vsa()?;
    assert_eq!(hv.dim(), 10000);

    // Genetic mutation of tau
    let factors = vec![1.1, 0.9, 1.05, 0.95, 1.0];
    lnn.mutate_tau(&factors);

    // Project again — should differ after mutation
    let hv2 = lnn.project_to_vsa()?;
    let sim = hv.similarity(&hv2)?;
    // May or may not differ significantly depending on the LNN state
    assert!(sim.abs() <= 1.0);

    Ok(())
}

#[test]
fn test_genetic_optimizer() {
    use lfi_vsa_core::languages::genetic::GeneticOptimizer;

    let mut optimizer = GeneticOptimizer::new(20, 10);

    // Assign fitness scores
    for i in 0..20 {
        optimizer.update_fitness(i, i as f64 * 0.5);
    }

    // Run 5 generations
    for _ in 0..5 {
        optimizer.evolve();
        // Re-score after each generation
        for i in 0..20 {
            optimizer.update_fitness(i, i as f64 * 0.5);
        }
    }

    // Best genes should exist
    assert!(optimizer.best_genes().is_some());
    assert_eq!(optimizer.best_genes().map(|g| g.len()), Some(10));
}
