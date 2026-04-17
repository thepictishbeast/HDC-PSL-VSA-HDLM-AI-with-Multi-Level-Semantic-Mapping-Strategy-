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
pub mod adversarial_data;
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
pub mod deepfake_detection;
pub mod supply_chain;
pub mod secret_scanner;
pub mod data_poisoning;
pub mod model_extraction;
pub mod network_anomaly;
pub mod prompt_firewall;
pub mod honey_tokens;
pub mod audit_log;
pub mod config;
pub mod metrics;
pub mod policy_engine;
pub mod rate_limiter;
pub mod webhook;
pub mod candle_inference;
pub mod experience_learning;
pub mod notification;

pub use osint::{OsintAnalyzer, OsintSignal};
pub use web_audit::ConnectivityAxiom;
pub use background::BackgroundLearner;
pub use persistence::KnowledgeStore;
pub use web_search::{WebSearchEngine, SearchResponse, SearchResult};
pub use serial_streamer::SerialStreamer;
pub mod camel_barrier;
pub mod knowledge_benchmark;
pub mod storage_tiering;
