# Brain v2 — Scalable Knowledge Architecture

## The Problem

brain.db is a single 74GB SQLite file with 57M+ facts. This worked for v1 but:
- 17.7GB Kitsune dataset alone is 25% of the current DB size
- Higgs boson is 2.5GB more
- 55 new datasets = another 10-20GB
- Target is 100M+ facts heading toward 1B
- Single-file SQLite has write contention with 3+ concurrent writers
- No columnar storage for analytics (domain distribution, quality trends)
- No vector index for semantic search (FTS5 is keyword-only)
- WAL file grows unbounded under write load (hit 24GB already)

## Brain v2 Design

```
┌──────────────────────────────────────────────────────┐
│                    BRAIN API LAYER                     │
│  Unified interface — callers don't know the backend   │
│  BrainDb::search(), store(), query(), analyze()       │
└──────────┬───────────┬───────────┬───────────────────┘
           │           │           │
    ┌──────┴──┐  ┌─────┴────┐  ┌──┴──────────┐
    │ SQLite  │  │  Vector   │  │  Analytics  │
    │ (facts) │  │  Index    │  │  (DuckDB)   │
    │ 57M+    │  │ (usearch) │  │  columnar   │
    └─────────┘  └──────────┘  └─────────────┘
```

### Layer 1: SQLite (keep, optimize)
- Remains the primary fact store (ACID, reliable, proven)
- Optimize: partition by domain into attached databases
- Fix WAL: periodic checkpoint, bounded WAL size
- Sharding: facts_core.db (hot, <10M rows), facts_archive.db (cold, 50M+)

### Layer 2: Vector Index (new — usearch or hnswlib)
- Semantic similarity search via HDC/embedding vectors
- "Find facts SIMILAR to this question" not just keyword match
- usearch crate: 1M vectors searchable in <1ms
- Index the BipolarVector or a reduced 256-dim embedding per fact
- Enables: "What do you know about X?" even when X isn't in the keywords

### Layer 3: Analytics Engine (new — DuckDB)
- Columnar storage for aggregate queries
- "Show me quality distribution across all domains" in <100ms
- "How many facts were added per day this week?" instant
- Powers the Classroom and Auditorium dashboards
- DuckDB runs in-process (no separate server), reads Parquet files

### Layer 4: Domain-Specific Stores (new)
Large datasets get their own optimized storage:
```
/home/user/.local/share/plausiden/
  brain.db              ← core facts (optimized, <20GB)
  brain_vectors.usearch ← vector similarity index
  analytics.duckdb      ← columnar analytics
  domains/
    kitsune/            ← 17.7GB network attack data (Parquet)
    higgs/              ← 2.5GB particle physics (Parquet)
    mathqa/             ← math reasoning (already in brain.db)
    sonarqube/          ← code quality (Parquet for large tables)
    cve/                ← CVE data (already in brain.db)
```

### Hot/Warm/Cold Tiering (from storage_tiering.rs)
Already designed, now actually enforced:
- **Hot** (brain.db): Frequently accessed, FTS5 indexed, <20M facts
- **Warm** (domain Parquet): Domain-specific, loaded on demand
- **Cold** (compressed archive): Rarely accessed, re-derivable

## Migration Path

1. **Now**: Keep brain.db as-is, add DuckDB for analytics queries
2. **Next**: Add usearch vector index alongside FTS5
3. **Later**: Partition large datasets into domain Parquet files
4. **Eventually**: Hot/warm/cold automatic tiering

## What This Enables

- Kitsune 17.7GB: stored as Parquet in domains/kitsune/, queryable via DuckDB
- Higgs 2.5GB: same, domains/higgs/
- 100M+ facts without single-file bottleneck
- Semantic search ("what's similar to X") not just keyword
- Analytics dashboards load in <100ms instead of timing out on 57M rows
- Concurrent writes from 3+ Claude instances without DB lock contention

## Crate Dependencies
- `rusqlite` (existing) — core fact store
- `duckdb` (new) — columnar analytics, Parquet read/write
- `usearch` (new) — vector similarity index
- `parquet` via `arrow` (new) — large dataset storage
