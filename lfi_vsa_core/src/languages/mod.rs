// ============================================================
// LFI Language Intelligence — Universal Polyglot Code Engine
// ============================================================

pub mod constructs;
pub mod registry;
pub mod self_improve;
pub mod genetic;

pub use constructs::{UniversalConstruct, Paradigm, PlatformTarget};
pub use registry::{LanguageId, LanguageRegistry, LanguageMetadata};
pub use self_improve::{SelfImproveEngine, OptimizationMetrics};
pub use genetic::{GeneticOptimizer, Chromosome};
