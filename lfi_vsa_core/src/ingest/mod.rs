// Ingestion modules — parsers for upstream corpora. Deterministic
// per-record transformations; the stream/batch layer lives in
// separate binaries (tools/) because they're long-running jobs.

pub mod wikidata;
pub mod causenet;
pub mod atomic;
pub mod discourse;
pub mod argumentation;
pub mod semantic_role;
pub mod dialogue_act;
