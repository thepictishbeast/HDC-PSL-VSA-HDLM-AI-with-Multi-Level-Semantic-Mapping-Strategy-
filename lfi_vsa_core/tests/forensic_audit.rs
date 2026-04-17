// ============================================================
// Forensic Integration Test — Hamming Weight Statistical Audit
// Verifies unbiased random initialization across N samples.
// ============================================================

use lfi_vsa_core::hdc::error::HdcError;
use lfi_vsa_core::hdc::vector::BipolarVector;

#[test]
fn forensic_hamming_weight_audit() -> Result<(), HdcError> {
    let sample_count = 100;
    let mut total_weight: usize = 0;

    for _ in 0..sample_count {
        let v = BipolarVector::new_random()?;
        total_weight += v.count_ones();
    }

    let avg_weight = total_weight as f64 / sample_count as f64;
    println!(
        "[DEBUGLOG][forensic_audit.rs] - Avg Hamming weight over {} samples: {:.1}",
        sample_count, avg_weight
    );

    // Expected: ~5000 (50% of 10,000). Tolerance: +/- 200 (~2%).
    // CLT standard error = sqrt(10000 * 0.25 / 100) = 5.0.
    // 200 / 5.0 = 40 sigma. Probability of false failure ~ 0.
    assert!(
        avg_weight > 4800.0 && avg_weight < 5200.0,
        "Avg Hamming weight out of tolerance: {:.1}", avg_weight
    );
    Ok(())
}
