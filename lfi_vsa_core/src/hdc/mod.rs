// ============================================================
// HDC (Hyperdimensional Computing) Core
// Section 1.II: "Maps concepts into a high-dimensional, discrete
// semantic space using 10,000-bit bipolar vectors."
// ============================================================

pub mod vector;
pub mod compute;
pub mod adaptive;
pub mod error;
pub mod liquid;
pub mod superposition;
pub mod holographic;
pub mod analogy;
pub mod sensory;
pub mod hadamard;
pub mod hdlm;
pub mod crdt;

pub use vector::{BipolarVector, HD_DIMENSIONS};
pub use hadamard::{HadamardGenerator, CorrelatedGenerator};
pub use compute::{ComputeBackend, LocalBackend};
pub use adaptive::{UiAttributes, UiElement};
pub use error::HdcError;
pub use liquid::{LiquidSensorium, LiquidNeuron};
pub mod constant_time;
pub mod encoder_protection;
pub mod tier_weighted_bundle;
