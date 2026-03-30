// ============================================================
// Cognition Module — Reasoning, Planning, and Active Inference
// ============================================================

pub mod planner;
pub mod reasoner;
pub mod knowledge;
pub mod mcts;
pub mod router;
pub mod world_model;
pub mod active_inference;

pub use planner::{Plan, PlanStep, StepStatus, Planner};
pub use reasoner::{CognitiveMode, CognitiveCore, ThoughtResult};
pub use knowledge::{KnowledgeEngine, NoveltyLevel, ClarifyingQuestion, ResearchNeed, SignalAssessment};
pub use mcts::{MctsEngine, MctsAction};
pub use router::{SemanticRouter, IntelligenceTier};
pub use world_model::WorldModel;
pub use active_inference::ActiveInferenceCore;
