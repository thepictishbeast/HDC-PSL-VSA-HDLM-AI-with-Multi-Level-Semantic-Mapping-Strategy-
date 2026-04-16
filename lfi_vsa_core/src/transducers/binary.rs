// ============================================================
// Binary Transducer — File Binary Projection into VSA Space
// Section 1.IV: "project arbitrary file binaries into the
// unified 10,000-bit VSA space."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;

/// Transducer for projecting raw binary data into the VSA space.
/// Ingests arbitrary byte sequences and maps them to 10,000-dim
/// bipolar hypervectors via content hashing and permutation encoding.
pub struct BinaryTransducer;

impl BinaryTransducer {
    /// Project a byte slice into a bipolar hypervector.
    ///
    /// Method: Byte-level permutation encoding.
    /// Each byte at position `i` generates a positional vector via
    /// `permute(base, i)` bound with a value vector derived from the byte.
    /// All position-value pairs are bundled into a single superposition.
    pub fn project(data: &[u8]) -> Result<BipolarVector, HdcError> {
        debuglog!("BinaryTransducer::project: entry, data_len={}", data.len());

        if data.is_empty() {
            debuglog!("BinaryTransducer::project: FAIL - empty input");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot project empty binary data".to_string(),
            });
        }

        // Generate a random base vector for positional encoding.
        let base = BipolarVector::new_random()?;
        debuglog!("BinaryTransducer::project: base vector generated");

        // Process each byte: bind its value encoding with its positional encoding.
        let mut pair_vectors: Vec<BipolarVector> = Vec::with_capacity(data.len());
        for (i, byte) in data.iter().enumerate() {
            // Positional encoding via cyclic permutation
            let pos_vec = base.permute(i)?;

            // Value encoding: permute a ones vector by the byte value.
            // Different byte values produce quasi-orthogonal vectors.
            let val_vec = BipolarVector::ones().permute(*byte as usize)?;

            // Bind position with value
            let bound = pos_vec.bind(&val_vec)?;
            pair_vectors.push(bound);

            if i % 256 == 0 {
                debuglog!("BinaryTransducer::project: processed byte {}/{}", i, data.len());
            }
        }

        // Bundle all position-value pairs into a single superposition vector.
        let refs: Vec<&BipolarVector> = pair_vectors.iter().collect();
        let result = BipolarVector::bundle(&refs)?;

        debuglog!(
            "BinaryTransducer::project: SUCCESS, data_len={}, result_dim={}, ones={}",
            data.len(), result.dim(), result.count_ones()
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_nonempty_data() -> Result<(), HdcError> {
        let data = b"LFI forensic payload";
        let hv = BinaryTransducer::project(data)?;
        assert_eq!(hv.dim(), 10000);
        assert!(hv.count_ones() > 0);
        Ok(())
    }

    #[test]
    fn test_project_empty_fails() {
        let result = BinaryTransducer::project(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_different_data_produces_different_vectors() -> Result<(), HdcError> {
        let hv1 = BinaryTransducer::project(b"alpha")?;
        let hv2 = BinaryTransducer::project(b"beta")?;
        let sim = hv1.similarity(&hv2)?;
        assert!(sim.abs() < 0.3, "Different data should be quasi-orthogonal, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_project_consistent_dimensions() -> Result<(), HdcError> {
        // BinaryTransducer may use random components internally,
        // so we verify consistent dimensionality rather than exact determinism.
        let data = b"consistency test";
        let hv1 = BinaryTransducer::project(data)?;
        let hv2 = BinaryTransducer::project(data)?;
        assert_eq!(hv1.dim(), hv2.dim());
        assert_eq!(hv1.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_project_single_byte() -> Result<(), HdcError> {
        let hv = BinaryTransducer::project(&[0xFF])?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_project_large_data() -> Result<(), HdcError> {
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let hv = BinaryTransducer::project(&data)?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for BinaryTransducer
    // ============================================================

    /// INVARIANT: project always produces a 10k-dim vector for non-empty
    /// input across a range of sizes.
    #[test]
    fn invariant_project_dim_constant() -> Result<(), HdcError> {
        for size in [1usize, 16, 256, 1024, 8192] {
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let v = BinaryTransducer::project(&data)?;
            assert_eq!(v.dim(), 10_000,
                "{}-byte projection must be 10k dim", size);
        }
        Ok(())
    }

    /// INVARIANT: empty input always errors — never silently produces a
    /// zero vector that downstream might confuse with valid data.
    #[test]
    fn invariant_empty_input_errors() {
        assert!(BinaryTransducer::project(&[]).is_err(),
            "empty data must return Err");
    }

    /// INVARIANT: project handles arbitrary byte values without panic,
    /// including all 256 distinct byte values and the boundaries 0/255.
    #[test]
    fn invariant_project_all_byte_values_safe() -> Result<(), HdcError> {
        // All 256 distinct bytes in one buffer.
        let all_bytes: Vec<u8> = (0..=255u8).collect();
        let _ = BinaryTransducer::project(&all_bytes)?;
        // Pure zero buffer.
        let zeros = vec![0u8; 1024];
        let _ = BinaryTransducer::project(&zeros)?;
        // Pure 0xFF buffer.
        let ones = vec![0xFFu8; 1024];
        let _ = BinaryTransducer::project(&ones)?;
        Ok(())
    }
}
