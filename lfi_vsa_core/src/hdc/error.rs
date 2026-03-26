// ============================================================
// HDC Core Error Types — Forensic Error Taxonomy
// All operations return Result<T, HdcError>. No panics.
// ============================================================

use std::fmt;

/// Exhaustive error variants for the HDC Core subsystem.
#[derive(Debug, PartialEq, Clone)]
pub enum HdcError {
    /// Vector dimension does not match HD_DIMENSIONS (10,000).
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    /// Random initialization produced invalid output.
    InitializationFailed {
        reason: String,
    },
    /// Bundle operation received an empty vector set.
    EmptyBundle,
    /// Compute backend dispatch failure (remote GPU / local fallback).
    ComputeDispatchError {
        reason: String,
    },
}

impl fmt::Display for HdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "HDC DimensionMismatch: expected {} dimensions, got {}",
                    expected, actual
                )
            }
            Self::InitializationFailed { reason } => {
                write!(f, "HDC InitializationFailed: {}", reason)
            }
            Self::EmptyBundle => {
                write!(f, "HDC EmptyBundle: cannot bundle zero vectors")
            }
            Self::ComputeDispatchError { reason } => {
                write!(f, "HDC ComputeDispatchError: {}", reason)
            }
        }
    }
}

impl std::error::Error for HdcError {}
