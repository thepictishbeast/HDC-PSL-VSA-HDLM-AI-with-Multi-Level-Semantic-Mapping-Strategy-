// Ingestion modules — parsers for upstream corpora. Deterministic
// per-record transformations; the stream/batch layer lives in
// separate binaries (tools/) because they're long-running jobs.

pub mod wikidata;
