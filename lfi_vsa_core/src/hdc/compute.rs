// ============================================================
// ComputeBackend Trait — Modular Dispatch Layer
// Default: Local ARM SIMD (bitwise). Extensible to remote GPU.
// Section 1.I: "modular ComputeBackend trait to dispatch
// massive matrix operations to remote GPU grids."
// ============================================================

use crate::hdc::error::HdcError;
use crate::hdc::vector::BipolarVector;
use crate::debuglog;

/// Trait defining the compute dispatch interface for HDC operations.
/// Implementors handle the actual arithmetic — locally or remotely.
pub trait ComputeBackend {
    /// Binding: element-wise bipolar multiplication (XOR in binary).
    fn bind(&self, a: &BipolarVector, b: &BipolarVector) -> Result<BipolarVector, HdcError>;

    /// Bundling: majority-vote superposition of N vectors (Sum+Clip).
    fn bundle(&self, vectors: &[&BipolarVector]) -> Result<BipolarVector, HdcError>;

    /// Permutation: cyclic left shift by `shift` positions.
    fn permute(&self, v: &BipolarVector, shift: usize) -> Result<BipolarVector, HdcError>;

    /// Cosine similarity in bipolar space. Returns [-1.0, 1.0].
    fn similarity(&self, a: &BipolarVector, b: &BipolarVector) -> Result<f64, HdcError>;
}

/// Local compute backend — bitwise operations on host CPU.
/// Targets ARMv9.2-A SIMD/NEON on the Tensor G5 Laguna SoC.
pub struct LocalBackend;

impl ComputeBackend for LocalBackend {
    fn bind(&self, a: &BipolarVector, b: &BipolarVector) -> Result<BipolarVector, HdcError> {
        debuglog!("LocalBackend::bind dispatched");
        a.bind(b)
    }

    fn bundle(&self, vectors: &[&BipolarVector]) -> Result<BipolarVector, HdcError> {
        debuglog!("LocalBackend::bundle dispatched, n={}", vectors.len());
        BipolarVector::bundle(vectors)
    }

    fn permute(&self, v: &BipolarVector, shift: usize) -> Result<BipolarVector, HdcError> {
        debuglog!("LocalBackend::permute dispatched, shift={}", shift);
        v.permute(shift)
    }

    fn similarity(&self, a: &BipolarVector, b: &BipolarVector) -> Result<f64, HdcError> {
        debuglog!("LocalBackend::similarity dispatched");
        a.similarity(b)
    }
}

// ============================================================
// ComputeBackend dispatch tests — verify local backend parity
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_backend_bind_dispatches() -> Result<(), HdcError> {
        let backend = LocalBackend;
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let direct = a.bind(&b)?;
        let via_backend = backend.bind(&a, &b)?;
        assert_eq!(direct, via_backend, "Backend dispatch must match direct call");
        Ok(())
    }

    #[test]
    fn test_local_backend_bundle_dispatches() -> Result<(), HdcError> {
        let backend = LocalBackend;
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let direct = BipolarVector::bundle(&[&a, &b])?;
        let via_backend = backend.bundle(&[&a, &b])?;
        assert_eq!(direct, via_backend, "Backend dispatch must match direct call");
        Ok(())
    }

    #[test]
    fn test_local_backend_permute_dispatches() -> Result<(), HdcError> {
        let backend = LocalBackend;
        let a = BipolarVector::new_random()?;
        let direct = a.permute(7)?;
        let via_backend = backend.permute(&a, 7)?;
        assert_eq!(direct, via_backend, "Backend dispatch must match direct call");
        Ok(())
    }

    #[test]
    fn test_local_backend_similarity_dispatches() -> Result<(), HdcError> {
        let backend = LocalBackend;
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let direct = a.similarity(&b)?;
        let via_backend = backend.similarity(&a, &b)?;
        assert!((direct - via_backend).abs() < f64::EPSILON);
        Ok(())
    }
}
