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
pub mod metacognitive;
pub mod knowledge_compiler;
pub mod spaced_repetition;
pub mod causal;
pub mod calibration;
pub mod global_workspace;
pub mod natural_gradient;

pub use planner::{Plan, PlanStep, StepStatus, Planner};
pub use reasoner::{CognitiveMode, CognitiveCore, ThoughtResult};
pub use knowledge::{KnowledgeEngine, NoveltyLevel, ClarifyingQuestion, ResearchNeed, SignalAssessment};
pub use mcts::{MctsEngine, MctsAction};
pub use router::{SemanticRouter, IntelligenceTier};
pub use world_model::WorldModel;
pub use active_inference::ActiveInferenceCore;
pub use metacognitive::{MetaCognitiveProfiler, CognitiveDomain, PerformanceRecord, ImprovementTarget};
pub use knowledge_compiler::{KnowledgeCompiler, AccelerationMetrics, CompiledEntry};
pub use spaced_repetition::{SpacedRepetitionScheduler, ReviewCard};
pub mod grokking_monitor;
pub mod fsrs_scheduler;
pub mod knowledge_graph;

pub mod emotion_detector;
pub mod conversation_summarizer;

pub use knowledge_graph::{KnowledgeGraph, EdgeType, FactEdge, Subgraph, GraphStats};
pub use emotion_detector::{detect_emotion, Emotion, EmotionAnalysis};
// Stitch wake-sleep library learning integration point
// Stitch cloned at /home/user/Development/PlausiDen/stitch/
// Integration: feed provenance traces → Stitch anti-unification → extract abstractions
// Promote abstractions into knowledge compiler as compiled entries
