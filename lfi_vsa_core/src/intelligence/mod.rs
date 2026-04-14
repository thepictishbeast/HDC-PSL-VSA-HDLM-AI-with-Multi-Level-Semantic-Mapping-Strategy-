// ============================================================
// Intelligence Module — The Knowledge Substrate
// ============================================================

pub mod osint;
pub mod web_audit;
pub mod background;
pub mod persistence;
pub mod web_search;
pub mod serial_streamer;
pub mod weight_manager;
pub mod training;
pub mod training_data;
pub mod benchmark;
pub mod local_inference;
pub mod code_eval;
pub mod self_improvement;
pub mod cross_domain;
pub mod math_engine;
pub mod daemon;
pub mod defensive_ai;
pub mod epistemic_filter;
pub mod generalization;
pub mod continuous_intel;
pub mod concurrent;
pub mod answer_verifier;
pub mod phd_tests;
pub mod textbook_learning;
pub mod anti_memorization;
pub mod info_retrieval;
pub mod benchmark_harness;

pub use osint::{OsintAnalyzer, OsintSignal};
pub use web_audit::ConnectivityAxiom;
pub use background::BackgroundLearner;
pub use persistence::KnowledgeStore;
pub use web_search::{WebSearchEngine, SearchResponse, SearchResult};
pub use serial_streamer::SerialStreamer;
