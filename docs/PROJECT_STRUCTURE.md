# Project Structure

Exhaustive per-file documentation of the LFI sovereign directory.

---

## Root Directory (`lfi_project/`)

| File | Purpose | Modified By |
|------|---------|-------------|
| `README.md` | Comprehensive project documentation, architecture overview, build instructions | Alpha |
| `CLAUDE.md` | Workflow Alpha state document — what Alpha knows, what it has done, what it should do next | Gamma |
| `STATE.md` | Gamma Handoff state — AST status, compile status, telemetry check, IPC status, next instruction | Gamma |
| `LFI.log` | Append-only telemetry log written by `lfi_daemon.sh` and other Delta watchdog processes | Delta |
| `lfi_bus.json` | IPC ledger: Alpha writes payload objects here after completing a phase or step | Alpha |
| `lfi_audit.json` | IPC ledger: Beta writes audit resolutions here after verifying Alpha's output | Beta |
| `lfi_daemon.sh` | Bash daemon using `inotifywait` to monitor `lfi_bus.json` and `lfi_audit.json`. Logs all events to `LFI.log` with UTC timestamps. Must be `chmod +x`. | Alpha |
| `.gitignore` | Excludes `**/target/` (Rust build artifacts) from version control | Alpha |

## Documentation (`docs/`)

| File | Content |
|------|---------|
| `ARCHITECTURE.md` | System architecture: HDC Core, PSL Supervisor, HDLM, data flow diagrams, ComputeBackend design, IPC bus |
| `PROJECT_STRUCTURE.md` | This file. Per-file descriptions of the entire repository |
| `HDC_OPERATIONS.md` | Mathematical foundations: bipolar encoding, XOR binding, sum+clip bundling, cyclic shift permutation, similarity metrics with proofs |
| `PSL_SUPERVISOR.md` | PSL framework: Axiom trait, audit pipeline, CARTA trust model, hostile data handling |
| `HDLM_AST.md` | HDLM design: AST arena allocation, NodeKind variants, Tier 1/2 separation, ForensicGenerator and DecorativeExpander traits |
| `TELEMETRY.md` | Complete map of every `debuglog!` / `debuglog_val!` call in the codebase, organized by file. Production stripping guide |
| `TESTING.md` | Test strategy, coverage breakdown by subsystem, how to run, what each test proves |
| `SECURITY.md` | Security posture: `forbid(unsafe_code)`, no unwrap, Result everywhere, CARTA model, hostile data treatment |
| `PD_PROTOCOL.md` | Plausible Deniability Protocol: VSA Superposition, Chaff Injection, ZKP Retrieval |
| `OPSEC_INTERCEPT.md` | Autonomous OPSEC Protocol: HDLM Intercept, PSL Write-Blocker, Identity Sovereignty |

## GitHub Configuration (`.github/`)

| File | Purpose |
|------|---------|
| `dependabot.yml` | Automated dependency update configuration for Cargo packages |

## Rust Crate (`lfi_vsa_core/`)

### Root

| File | Purpose |
|------|---------|
| `Cargo.toml` | Crate manifest. Dependencies: `bitvec 1.0.1` (bit vector storage), `rand 0.8.5` (random vector init), `serde 1` + `serde_json 1` (IPC serialization) |
| `Cargo.lock` | Pinned dependency versions for reproducible builds |
| `.gitignore` | Excludes `/target/` |

### Source (`src/`)

| File | Lines | Purpose | Tests |
|------|-------|---------|-------|
| `lib.rs` | ~20 | **Crate root.** Sets `#![forbid(unsafe_code)]`. Declares all module paths (`hdc`, `psl`, `hdlm`, `telemetry`). Re-exports core public types for ergonomic `use lfi_vsa_core::BipolarVector` style imports. | 0 (wiring only) |
| `telemetry.rs` | ~30 | **Delta telemetry macros.** `debuglog!($fmt, $args)` emits `[DEBUGLOG][file:line] - message`. `debuglog_val!($label, $val)` emits debug representation. Both are structurally isolated for production stripping. | 0 (macro definitions) |

### HDC Core (`src/hdc/`) — Phase 1

| File | Lines | Purpose | Tests |
|------|-------|---------|-------|
| `mod.rs` | 3 | Module exports: `error`, `vector`, `compute` | 0 |
| `error.rs` | ~50 | `HdcError` enum: `DimensionMismatch`, `InitializationFailed`, `EmptyBundle`, `ComputeDispatchError`. Implements `Display` and `Error`. | 0 |
| `vector.rs` | ~480 | **The HDC engine.** `BipolarVector` struct backed by `BitVec<u8, Lsb0>`. Constructors: `new_random()`, `from_bitvec()`, `zeros()`, `ones()`. Operations: `bind()` (XOR), `bundle()` (Sum+Clip), `permute()` (cyclic shift). Metrics: `similarity()` (cosine), `hamming_distance()`. Internal `check_dim()` helper. | 43 |
| `compute.rs` | ~80 | `ComputeBackend` trait defining `bind`, `bundle`, `permute`, `similarity`. `LocalBackend` struct implementing the trait by delegating to `BipolarVector` methods. | 4 |

### PSL Supervisor (`src/psl/`) — Phase 2A

| File | Lines | Purpose | Tests |
|------|-------|---------|-------|
| `mod.rs` | 4 | Module exports: `error`, `axiom`, `trust`, `supervisor` | 0 |
| `error.rs` | ~55 | `PslError` enum: `AxiomViolation`, `InvalidAuditTarget`, `TrustThresholdBreached`, `HostileDataDetected`, `EmptyAxiomSet`. | 0 |
| `axiom.rs` | ~140 | `Axiom` trait (`id()`, `description()`, `verify()`). `AuditTarget` enum (Vector, RawBytes, Scalar, Payload). `AxiomVerdict` struct. Built-in: `DimensionalityAxiom`, `DataIntegrityAxiom`. | 0 (tested via supervisor) |
| `trust.rs` | ~90 | `TrustLevel` enum (5 levels, Ord-derived). `TrustAssessment` struct with confidence, pass ratio, rationale. Methods: `permits_execution()`, `requires_audit()`, `score()`. | 0 (tested via supervisor) |
| `supervisor.rs` | ~160 | `PslSupervisor` engine. `register_axiom()`, `audit()`, `compute_trust_level()`. Skips inapplicable axioms, computes pass ratio and avg truth. | 13 |

### HDLM (`src/hdlm/`) — Phase 2B

| File | Lines | Purpose | Tests |
|------|-------|---------|-------|
| `mod.rs` | 4 | Module exports: `error`, `ast`, `tier1_forensic`, `tier2_decorative` | 0 |
| `error.rs` | ~50 | `HdlmError` enum: `MalformedAst`, `Tier1GenerationFailed`, `Tier2ExpansionFailed`, `EmptyAst`, `UnmappedSymbol`. | 0 |
| `ast.rs` | ~220 | `Ast` arena: `Vec<AstNode>` with `NodeId` references. `AstNode` struct: id, kind, children, optional hv_fingerprint. `NodeKind` enum: 13 variants covering program structure, expressions, statements, and NL. DFS/BFS traversal. | 11 |
| `tier1_forensic.rs` | ~120 | `ForensicGenerator` trait. `ArithmeticGenerator` parses prefix-notation arithmetic tokens into verified ASTs. `parse_prefix()` recursive descent. | 9 |
| `tier2_decorative.rs` | ~140 | `DecorativeExpander` trait (takes `&Ast` — immutable). `InfixRenderer` produces `(1 + 2)`. `SExprRenderer` produces `(+ 1 2)`. | 8 |

### Integration Tests (`tests/`)

| File | Purpose |
|------|---------|
| `forensic_audit.rs` | Statistical audit: generates 100 random `BipolarVector`s, verifies average Hamming weight is within 2% of 5000 (the expected mean for unbiased random bits). Uses `-> Result<(), HdcError>` — no `.unwrap()`. |
