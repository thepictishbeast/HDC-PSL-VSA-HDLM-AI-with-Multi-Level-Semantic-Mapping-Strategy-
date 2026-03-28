// ============================================================
// LFI Cognition — Dual-Mode Reasoning Architecture
//
// Two modes of thought:
//   FAST (System 1): O(1) VSA similarity lookup, pattern matching,
//     reflexive responses. Used for familiar problems.
//   DEEP (System 2): Multi-step planning, goal decomposition,
//     constraint propagation, iterative refinement. Used for
//     novel or complex problems.
//
// The CognitiveCore automatically selects the appropriate mode
// based on task novelty (similarity to known patterns) and
// complexity (estimated step count).
// ============================================================

pub mod planner;
pub mod reasoner;
pub mod knowledge;

pub use planner::{Plan, PlanStep, StepStatus, Planner};
pub use reasoner::{CognitiveMode, CognitiveCore, ThoughtResult};
pub use knowledge::{KnowledgeEngine, NoveltyLevel, ClarifyingQuestion, ResearchNeed, SignalAssessment};
