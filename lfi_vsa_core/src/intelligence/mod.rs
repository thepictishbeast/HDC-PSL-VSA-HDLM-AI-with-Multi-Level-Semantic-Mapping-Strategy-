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

pub use osint::{OsintAnalyzer, OsintSignal};
pub use web_audit::ConnectivityAxiom;
pub use background::BackgroundLearner;
pub use persistence::KnowledgeStore;
pub use web_search::{WebSearchEngine, SearchResponse, SearchResult};
pub use serial_streamer::SerialStreamer;
