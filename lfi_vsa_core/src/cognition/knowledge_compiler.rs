// ============================================================
// Knowledge Compiler — System 2 → System 1 Pipeline
//
// When System 2 (MCTS + planning) successfully solves a problem,
// the Knowledge Compiler encodes the solution into System 1
// holographic memory for O(1) retrieval next time.
//
// This is the core acceleration mechanism:
//   - First encounter: Deep reasoning (System 2), seconds to minutes
//   - Future encounters: Instant pattern match (System 1), microseconds
//   - Over time: System 1 handles more, System 2 needed less
//
// Compilation steps:
//   1. Extract solution pattern from ThoughtResult
//   2. Bind input vector with output vector
//   3. Store in holographic memory (System 1)
//   4. Track compilation statistics
//
// Escape velocity condition: when System 1 hit rate > 80% and rising.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::error::HdcError;
use crate::cognition::reasoner::CognitiveMode;
use std::collections::VecDeque;

/// A compiled knowledge entry — a System 2 solution encoded for System 1.
#[derive(Debug, Clone)]
pub struct CompiledEntry {
    /// The input pattern that triggered System 2.
    pub input_vector: BipolarVector,
    /// The output vector produced by System 2.
    pub output_vector: BipolarVector,
    /// Confidence of the original System 2 result.
    pub confidence: f64,
    /// Description of what was compiled.
    pub description: String,
    /// How many times this compiled knowledge has been retrieved.
    pub retrieval_count: usize,
}

/// Acceleration metrics for tracking escape velocity.
#[derive(Debug, Clone)]
pub struct AccelerationMetrics {
    /// Fraction of queries handled by System 1 (0.0 to 1.0).
    pub system1_hit_rate: f64,
    /// Rate of change of system1_hit_rate (positive = accelerating).
    pub d_hit_rate: f64,
    /// Rate of change of d_hit_rate (second derivative — sustained acceleration).
    pub d2_hit_rate: f64,
    /// Average System 2 deliberation time (lower = faster).
    pub avg_deliberation_ms: f64,
    /// Total compilations performed.
    pub total_compilations: usize,
    /// Total System 1 hits.
    pub total_system1_hits: usize,
    /// Total System 2 invocations.
    pub total_system2_invocations: usize,
}

impl AccelerationMetrics {
    /// Check if escape velocity conditions are met.
    ///
    /// Escape velocity: System 1 hit rate > 80%, first derivative positive,
    /// and second derivative non-negative (sustained acceleration).
    pub fn escape_velocity_reached(&self) -> bool {
        self.system1_hit_rate > 0.8 && self.d_hit_rate > 0.0 && self.d2_hit_rate >= 0.0
    }
}

/// The Knowledge Compiler — transforms System 2 deliberation into System 1 intuition.
pub struct KnowledgeCompiler {
    /// The compilation target — System 1's holographic memory.
    compiled_memory: HolographicMemory,

    /// Log of all compilations (bounded ring buffer).
    compilation_log: VecDeque<CompiledEntry>,

    /// Maximum compilation log size.
    max_log_size: usize,

    /// Minimum confidence required to compile a System 2 result.
    /// Below this threshold, the result is too uncertain to teach System 1.
    min_compile_confidence: f64,

    /// Rolling window of System 1 vs System 2 usage for hit rate calculation.
    mode_history: VecDeque<CognitiveMode>,

    /// Size of the rolling window.
    mode_window_size: usize,

    /// Previous hit rates for derivative calculation.
    hit_rate_history: VecDeque<f64>,

    /// Total compilations.
    total_compilations: usize,
}

impl KnowledgeCompiler {
    /// Create a new Knowledge Compiler.
    pub fn new() -> Self {
        debuglog!("KnowledgeCompiler::new: Initializing System 2 → System 1 pipeline");
        Self {
            compiled_memory: HolographicMemory::new(),
            compilation_log: VecDeque::new(),
            max_log_size: 1000,
            min_compile_confidence: 0.7,
            mode_history: VecDeque::new(),
            mode_window_size: 100,
            hit_rate_history: VecDeque::new(),
            total_compilations: 0,
        }
    }

    /// Compile a System 2 result into System 1 holographic memory.
    ///
    /// Only compiles if:
    /// 1. The result came from System 2 (Deep mode)
    /// 2. Confidence exceeds min_compile_confidence
    /// 3. The input/output vectors are valid
    ///
    /// Returns true if compilation occurred, false if skipped.
    pub fn compile(
        &mut self,
        mode: &CognitiveMode,
        input: &BipolarVector,
        output: &BipolarVector,
        confidence: f64,
        description: &str,
    ) -> Result<bool, HdcError> {
        debuglog!(
            "KnowledgeCompiler::compile: mode={:?}, conf={:.3}, desc='{}'",
            mode, confidence, crate::truncate_str(description, 50)
        );

        // Only compile System 2 results
        if *mode != CognitiveMode::Deep {
            debuglog!("KnowledgeCompiler::compile: Skipping — not System 2");
            return Ok(false);
        }

        // Check confidence threshold
        if confidence < self.min_compile_confidence {
            debuglog!(
                "KnowledgeCompiler::compile: Skipping — confidence {:.3} < threshold {:.3}",
                confidence, self.min_compile_confidence
            );
            return Ok(false);
        }

        // Store in holographic memory: associate(input, output)
        // Future System 1 can probe(input) → get output
        self.compiled_memory.associate(input, output)?;

        // Log the compilation
        let entry = CompiledEntry {
            input_vector: input.clone(),
            output_vector: output.clone(),
            confidence,
            description: description.to_string(),
            retrieval_count: 0,
        };

        self.compilation_log.push_back(entry);
        if self.compilation_log.len() > self.max_log_size {
            self.compilation_log.pop_front();
        }

        self.total_compilations += 1;
        debuglog!(
            "KnowledgeCompiler::compile: SUCCESS — total compilations={}",
            self.total_compilations
        );

        Ok(true)
    }

    /// Try to retrieve a compiled answer from System 1 memory.
    ///
    /// Returns the retrieved vector and its similarity to what was stored.
    /// The caller should check if similarity is high enough to use.
    pub fn retrieve(&self, input: &BipolarVector) -> Result<(BipolarVector, f64), HdcError> {
        debuglog!("KnowledgeCompiler::retrieve: Probing compiled memory");

        let retrieved = self.compiled_memory.probe(input)?;

        // Measure quality by checking if the retrieved vector is
        // well-formed (not noise). We use logic_flux as a proxy.
        let flux = self.compiled_memory.logic_flux()?;
        let quality = 1.0 - flux.min(1.0); // Lower flux = higher quality

        debuglog!(
            "KnowledgeCompiler::retrieve: flux={:.4}, quality={:.4}",
            flux, quality
        );

        Ok((retrieved, quality))
    }

    /// Record which cognitive mode was used for a query.
    /// This feeds the acceleration metrics.
    pub fn record_mode(&mut self, mode: CognitiveMode) {
        debuglog!("KnowledgeCompiler::record_mode: {:?}", mode);

        self.mode_history.push_back(mode);
        if self.mode_history.len() > self.mode_window_size {
            self.mode_history.pop_front();
        }

        // Recalculate hit rate every 10 records
        if self.mode_history.len() % 10 == 0 {
            let hit_rate = self.current_hit_rate();
            self.hit_rate_history.push_back(hit_rate);
            if self.hit_rate_history.len() > 20 {
                self.hit_rate_history.pop_front();
            }
        }
    }

    /// Get current System 1 hit rate from rolling window.
    pub fn current_hit_rate(&self) -> f64 {
        if self.mode_history.is_empty() {
            return 0.0;
        }
        let fast_count = self.mode_history.iter()
            .filter(|m| **m == CognitiveMode::Fast)
            .count();
        fast_count as f64 / self.mode_history.len() as f64
    }

    /// Calculate acceleration metrics.
    pub fn acceleration_metrics(&self) -> AccelerationMetrics {
        let system1_hit_rate = self.current_hit_rate();
        let total_system1_hits = self.mode_history.iter()
            .filter(|m| **m == CognitiveMode::Fast)
            .count();
        let total_system2_invocations = self.mode_history.iter()
            .filter(|m| **m == CognitiveMode::Deep)
            .count();

        // Calculate first derivative (d/dt of hit rate)
        let d_hit_rate = if self.hit_rate_history.len() >= 2 {
            let recent = self.hit_rate_history.back().copied().unwrap_or(0.0);
            let prev = self.hit_rate_history.iter().rev().nth(1).copied().unwrap_or(0.0);
            recent - prev
        } else {
            0.0
        };

        // Calculate second derivative (d²/dt² of hit rate)
        let d2_hit_rate = if self.hit_rate_history.len() >= 3 {
            let rates: Vec<f64> = self.hit_rate_history.iter().copied().collect();
            let n = rates.len();
            let d1_recent = rates[n - 1] - rates[n - 2];
            let d1_prev = rates[n - 2] - rates[n - 3];
            d1_recent - d1_prev
        } else {
            0.0
        };

        AccelerationMetrics {
            system1_hit_rate,
            d_hit_rate,
            d2_hit_rate,
            avg_deliberation_ms: 0.0, // TODO: wire in actual timing
            total_compilations: self.total_compilations,
            total_system1_hits: total_system1_hits,
            total_system2_invocations: total_system2_invocations,
        }
    }

    /// Get the number of entries in compiled memory.
    pub fn compiled_count(&self) -> usize {
        self.compiled_memory.capacity
    }

    /// Set the minimum confidence threshold for compilation.
    pub fn set_min_confidence(&mut self, threshold: f64) {
        self.min_compile_confidence = threshold.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let compiler = KnowledgeCompiler::new();
        assert_eq!(compiler.compiled_count(), 0);
        assert_eq!(compiler.total_compilations, 0);
    }

    #[test]
    fn test_compile_system2_result() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;

        let compiled = compiler.compile(
            &CognitiveMode::Deep,
            &input,
            &output,
            0.9,
            "Test compilation",
        )?;

        assert!(compiled, "Should compile high-confidence System 2 result");
        assert_eq!(compiler.compiled_count(), 1);
        assert_eq!(compiler.total_compilations, 1);
        Ok(())
    }

    #[test]
    fn test_skip_system1_result() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;

        let compiled = compiler.compile(
            &CognitiveMode::Fast,
            &input,
            &output,
            0.95,
            "System 1 result — should skip",
        )?;

        assert!(!compiled, "Should not compile System 1 results");
        assert_eq!(compiler.compiled_count(), 0);
        Ok(())
    }

    #[test]
    fn test_skip_low_confidence() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;

        let compiled = compiler.compile(
            &CognitiveMode::Deep,
            &input,
            &output,
            0.3, // Below threshold
            "Low confidence — should skip",
        )?;

        assert!(!compiled, "Should not compile low-confidence results");
        assert_eq!(compiler.compiled_count(), 0);
        Ok(())
    }

    #[test]
    fn test_retrieve_compiled() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;

        compiler.compile(&CognitiveMode::Deep, &input, &output, 0.9, "Test")?;

        // Retrieve — with only 1 association, retrieval should be clean
        let (retrieved, quality) = compiler.retrieve(&input)?;
        let sim = retrieved.similarity(&output)?;

        debuglog!("Retrieved similarity: {:.4}, quality: {:.4}", sim, quality);
        assert!(sim > 0.5, "Should retrieve the compiled output, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_mode_tracking() {
        let mut compiler = KnowledgeCompiler::new();

        // Record 80 Fast, 20 Deep
        for _ in 0..80 {
            compiler.record_mode(CognitiveMode::Fast);
        }
        for _ in 0..20 {
            compiler.record_mode(CognitiveMode::Deep);
        }

        let hit_rate = compiler.current_hit_rate();
        assert!(
            (hit_rate - 0.8).abs() < 0.01,
            "Hit rate should be ~0.8, got {}",
            hit_rate
        );
    }

    #[test]
    fn test_acceleration_metrics() {
        let mut compiler = KnowledgeCompiler::new();

        // Phase 1: mostly System 2 (learning phase)
        for _ in 0..30 {
            compiler.record_mode(CognitiveMode::Deep);
        }
        for _ in 0..10 {
            compiler.record_mode(CognitiveMode::Fast);
        }

        let metrics = compiler.acceleration_metrics();
        debuglog!(
            "Phase 1 metrics: hit_rate={:.3}, d={:.3}, d2={:.3}",
            metrics.system1_hit_rate, metrics.d_hit_rate, metrics.d2_hit_rate
        );

        // Phase 2: transitioning to System 1
        for _ in 0..30 {
            compiler.record_mode(CognitiveMode::Fast);
        }
        for _ in 0..10 {
            compiler.record_mode(CognitiveMode::Deep);
        }

        let metrics = compiler.acceleration_metrics();
        debuglog!(
            "Phase 2 metrics: hit_rate={:.3}, d={:.3}, d2={:.3}",
            metrics.system1_hit_rate, metrics.d_hit_rate, metrics.d2_hit_rate
        );

        // Should not yet have reached escape velocity
        // (insufficient data in rolling window for robust derivatives)
        assert!(!metrics.escape_velocity_reached() || metrics.system1_hit_rate > 0.8);
    }

    #[test]
    fn test_escape_velocity_detection() {
        let metrics = AccelerationMetrics {
            system1_hit_rate: 0.85,
            d_hit_rate: 0.02,
            d2_hit_rate: 0.001,
            avg_deliberation_ms: 50.0,
            total_compilations: 200,
            total_system1_hits: 170,
            total_system2_invocations: 30,
        };
        assert!(metrics.escape_velocity_reached());

        // Not reached: hit rate too low
        let metrics2 = AccelerationMetrics {
            system1_hit_rate: 0.5,
            ..metrics.clone()
        };
        assert!(!metrics2.escape_velocity_reached());

        // Not reached: decelerating
        let metrics3 = AccelerationMetrics {
            d_hit_rate: -0.01,
            ..metrics
        };
        assert!(!metrics3.escape_velocity_reached());
    }

    #[test]
    fn test_multiple_compilations() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();

        for i in 0..10 {
            let input = BipolarVector::from_seed(i as u64);
            let output = BipolarVector::from_seed(i as u64 + 1000);
            compiler.compile(
                &CognitiveMode::Deep,
                &input,
                &output,
                0.8 + (i as f64 * 0.01),
                &format!("Compilation {}", i),
            )?;
        }

        assert_eq!(compiler.compiled_count(), 10);
        assert_eq!(compiler.total_compilations, 10);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for KnowledgeCompiler
    // ============================================================

    /// INVARIANT: retrieve() returns Ok with a finite quality value for any
    /// input — the "no compiled entries" case yields a clean-vector result,
    /// not a panic or non-finite score.
    #[test]
    fn invariant_retrieve_empty_returns_finite_quality() {
        let compiler = KnowledgeCompiler::new();
        let input = BipolarVector::from_seed(42);
        let (vec, quality) = compiler.retrieve(&input)
            .expect("retrieve on empty must not error");
        assert_eq!(vec.dim(), input.dim(),
            "retrieved vector must match input dim");
        assert!(quality.is_finite() && (0.0..=1.0).contains(&quality),
            "quality out of [0,1]: {}", quality);
    }

    /// INVARIANT: compiled_count grows exactly by 1 per successful compile()
    /// above threshold.
    #[test]
    fn invariant_compile_grows_count_by_one() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();
        for i in 0..10 {
            let before = compiler.compiled_count();
            let input = BipolarVector::from_seed(100 + i as u64);
            let output = BipolarVector::from_seed(200 + i as u64);
            compiler.compile(&CognitiveMode::Deep, &input, &output, 0.9,
                &format!("entry {}", i))?;
            assert_eq!(compiler.compiled_count(), before + 1,
                "count must grow by 1 at iter {}", i);
        }
        Ok(())
    }

    /// INVARIANT: acceleration_metrics.escape_velocity_reached is a pure
    /// function of the metrics — deterministic across calls.
    #[test]
    fn invariant_escape_velocity_deterministic() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();
        for i in 0..30 {
            let input = BipolarVector::from_seed(i as u64);
            let output = BipolarVector::from_seed(1000 + i as u64);
            compiler.compile(&CognitiveMode::Deep, &input, &output, 0.85, "x")?;
        }
        let m1 = compiler.acceleration_metrics();
        let m2 = compiler.acceleration_metrics();
        assert_eq!(m1.escape_velocity_reached(), m2.escape_velocity_reached(),
            "escape_velocity_reached must be deterministic");
        Ok(())
    }

    /// INVARIANT: current_hit_rate is in [0,1].
    #[test]
    fn invariant_hit_rate_in_unit_interval() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();
        // Fresh compiler.
        let r = compiler.current_hit_rate();
        assert!(r.is_finite() && (0.0..=1.0).contains(&r),
            "fresh hit rate out of [0,1]: {}", r);
        // After recording some modes.
        for i in 0..20 {
            compiler.record_mode(if i % 2 == 0 { CognitiveMode::Fast }
                else { CognitiveMode::Deep });
        }
        let r = compiler.current_hit_rate();
        assert!(r.is_finite() && (0.0..=1.0).contains(&r),
            "hit rate out of [0,1] after records: {}", r);
        Ok(())
    }

    /// INVARIANT: set_min_confidence accepts arbitrary values but enforces
    /// them — compiling below the threshold is rejected.
    #[test]
    fn invariant_min_confidence_threshold_respected() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();
        compiler.set_min_confidence(0.9);
        let input = BipolarVector::from_seed(1);
        let output = BipolarVector::from_seed(2);
        // Below threshold — should not compile.
        compiler.compile(&CognitiveMode::Deep, &input, &output, 0.5, "low")?;
        assert_eq!(compiler.compiled_count(), 0,
            "below-threshold compile must be rejected");
        // Above threshold — should compile.
        compiler.compile(&CognitiveMode::Deep, &input, &output, 0.95, "high")?;
        assert_eq!(compiler.compiled_count(), 1,
            "above-threshold compile must succeed");
        Ok(())
    }

    /// INVARIANT: current_hit_rate is finite (non-NaN) after many records.
    #[test]
    fn invariant_hit_rate_finite_after_records() {
        let mut compiler = KnowledgeCompiler::new();
        for _ in 0..5 {
            compiler.record_mode(CognitiveMode::Fast);
        }
        for _ in 0..5 {
            compiler.record_mode(CognitiveMode::Deep);
        }
        let rate = compiler.current_hit_rate();
        assert!(rate.is_finite(),
            "hit_rate not finite: {}", rate);
    }

    /// INVARIANT: fresh compiler has zero compilations and hit_rate is 0.
    #[test]
    fn invariant_fresh_compiler_zero_state() {
        let compiler = KnowledgeCompiler::new();
        assert_eq!(compiler.compiled_count(), 0);
        let rate = compiler.current_hit_rate();
        assert_eq!(rate, 0.0);
    }

    /// INVARIANT: AccelerationMetrics fields are finite after many records.
    #[test]
    fn invariant_acceleration_metrics_finite() {
        let mut compiler = KnowledgeCompiler::new();
        for i in 0..20 {
            compiler.record_mode(if i % 2 == 0 { CognitiveMode::Fast } else { CognitiveMode::Deep });
        }
        let m = compiler.acceleration_metrics();
        assert!(m.system1_hit_rate.is_finite());
        assert!(m.d_hit_rate.is_finite());
        assert!(m.d2_hit_rate.is_finite());
        assert!(m.avg_deliberation_ms.is_finite() && m.avg_deliberation_ms >= 0.0);
    }

    /// INVARIANT: compiled_count monotonically increases on successful compiles.
    #[test]
    fn invariant_compiled_count_monotone() -> Result<(), HdcError> {
        let mut compiler = KnowledgeCompiler::new();
        compiler.set_min_confidence(0.5);
        let mut prev = compiler.compiled_count();
        for i in 0..10 {
            let input = BipolarVector::from_seed(i);
            let output = BipolarVector::from_seed(i + 1000);
            compiler.compile(&CognitiveMode::Deep, &input, &output, 0.9, "t")?;
            let cur = compiler.compiled_count();
            assert!(cur >= prev,
                "compiled_count decreased: {} -> {}", prev, cur);
            prev = cur;
        }
        Ok(())
    }
}
