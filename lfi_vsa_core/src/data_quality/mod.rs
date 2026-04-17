// ============================================================
// Data Quality Module — MinHash Dedup + Bloom Decontamination
// AVP-PASS-13: 2026-04-16 — Sprint 1 quality ceiling work
//
// PURPOSE: Near-duplicate detection via MinHash (FineWeb params),
// test-set decontamination via 13-gram Bloom filter.
//
// BUG ASSUMPTION: MinHash similarity threshold of 0.8 may need
// tuning per-domain. Academic text has higher legitimate overlap.
// ============================================================

pub mod minhash;
pub mod bloom;
