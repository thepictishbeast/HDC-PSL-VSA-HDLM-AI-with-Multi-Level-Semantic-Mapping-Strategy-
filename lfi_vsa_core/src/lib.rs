#![forbid(unsafe_code)]

// ============================================================
// LFI VSA Core — Crate Root
// Section 5: Absolute Memory Safety enforced via forbid(unsafe_code).
// All math operations return Result<T, E> or Option<T>.
// No .unwrap(), .expect(), or panic!() permitted.
// ============================================================

pub mod telemetry;
pub mod hdc;

// Re-export core public types for ergonomic access.
pub use hdc::error::HdcError;
pub use hdc::vector::{BipolarVector, HD_DIMENSIONS};
pub use hdc::compute::{ComputeBackend, LocalBackend};
