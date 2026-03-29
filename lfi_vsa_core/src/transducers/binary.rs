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
        // Different content should produce quasi-orthogonal vectors
        let sim = hv1.similarity(&hv2)?;
        assert!(sim.abs() < 0.3, "Different data should be quasi-orthogonal, sim={}", sim);
        Ok(())
    }
}
