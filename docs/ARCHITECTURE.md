# LFI System Architecture

## Overview

LFI is a Neuro-Symbolic agent built on three computational pillars that work in concert:

1. **HDC Core** — The reasoning engine (bitwise algebra on 10,000-dim hypervectors)
2. **PSL Supervisor** — The verification layer (hostile witness, zero-trust auditing)
3. **HDLM** — The language interface (AST generation and rendering)

These are connected via a modular `ComputeBackend` trait and communicate with external agents through a file-based IPC bus.

---

## Data Flow

```
                    External Input
                         |
                         v
              +---------------------+
              |    AuditTarget      |   <-- PSL wraps all inputs
              | (Vector/RawBytes/   |
              |  Scalar/Payload)    |
              +---------------------+
                         |
           +-------------+-------------+
           |                           |
           v                           v
  +------------------+       +------------------+
  |   HDC Core       |       |  PSL Supervisor  |
  |                  |       |                  |
  |  Encode input    |       |  Run all axioms  |
  |  as BipolarVector|       |  against target  |
  |  via transducers |       |                  |
  |  (future Phase)  |       |  Produce         |
  |                  |       |  TrustAssessment  |
  +------------------+       +------------------+
           |                           |
           |    Trust >= Verified?     |
           |<--------------------------+
           |        YES / NO
           |
           v (if YES)
  +------------------+
  |   HDLM           |
  |                  |
  |  Tier 1: Decode  |
  |  vector -> AST   |
  |  (ForensicGen)   |
  |                  |
  |  Tier 2: Render  |
  |  AST -> output   |
  |  (DecorativeExp) |
  +------------------+
           |
           v
      Output (Code / Prose / Action)
```

---

## Subsystem Details

### HDC Core (`src/hdc/`)

**Purpose:** Encode, manipulate, and query information using hyperdimensional computing.

**Key insight:** In 10,000 dimensions, random vectors are quasi-orthogonal with overwhelming probability. This means we can use simple bitwise operations (XOR, majority vote, cyclic shift) to create, combine, and query symbolic representations without floating-point neural network weights.

**Components:**
- `vector.rs` — `BipolarVector` struct. All 10,000 bits stored in a `BitVec<u8, Lsb0>` for memory efficiency. Encoding: bit 0 = bipolar -1, bit 1 = bipolar +1.
- `error.rs` — `HdcError` enum covering dimension mismatches, initialization failures, empty bundles, and compute dispatch errors.
- `compute.rs` — `ComputeBackend` trait with `LocalBackend` implementation. The trait is the extension point for remote GPU clusters; `LocalBackend` performs all operations locally via bitwise ops on the host CPU.

**Memory model:** A `BipolarVector` occupies approximately 1,250 bytes (10,000 bits / 8). This is fixed and deterministic — no heap allocations grow beyond this.

### PSL Supervisor (`src/psl/`)

**Purpose:** Enforce zero-hallucination by verifying every computation result against material axioms before it can influence downstream processing.

**Key insight:** Trust is never assumed. Every datum enters the system at `TrustLevel::Untrusted` and must be promoted through verified axiom gates before it can be used.

**Components:**
- `axiom.rs` — The `Axiom` trait defines the verification interface. `AuditTarget` wraps the data types that can be audited. `AxiomVerdict` carries pass/fail with a soft truth value in [0.0, 1.0]. Two structural axioms are built-in: `DimensionalityAxiom` (vector dim check) and `DataIntegrityAxiom` (hostile data size/emptiness check). **Domain-specific axioms are defined by Beta (Gemini), not Alpha.**
- `trust.rs` — The CARTA trust model. Five discrete levels with numeric scores, execution gates, and audit requirement checks.
- `supervisor.rs` — `PslSupervisor` holds registered axioms, runs them against targets, computes pass ratios and average truth values, and derives a `TrustAssessment`.
- `error.rs` — `PslError` enum for axiom violations, invalid targets, trust threshold breaches, and hostile data detection.

**Audit pipeline:** When `supervisor.audit(target)` is called:
1. Each registered axiom's `verify()` is called against the target
2. Axioms that return `InvalidAuditTarget` are skipped (wrong type match)
3. Axioms that return structural errors propagate immediately
4. Pass ratio and average truth value are computed from applicable verdicts
5. Trust level is derived from these metrics against the configured threshold
6. A `TrustAssessment` is returned summarizing the audit

### HDLM (`src/hdlm/`)

**Purpose:** Bridge between the vector space (HDC) and human-readable outputs (code, prose).

**Key insight:** Separate the logical truth (AST) from the aesthetic presentation (rendered output). Tier 1 produces the truth; Tier 2 decorates it. Tier 2 is provably read-only on the AST.

**Components:**
- `ast.rs` — Arena-allocated AST. Nodes stored in a `Vec<AstNode>`, referenced by `NodeId` (usize index). Each node has a `NodeKind` enum variant, ordered child list, and optional `BipolarVector` fingerprint. Supports DFS and BFS traversal.
- `tier1_forensic.rs` — `ForensicGenerator` trait with two entry points: `generate_from_tokens()` and `generate_from_vector()`. `ArithmeticGenerator` is a working demo that parses prefix-notation arithmetic into a verified AST.
- `tier2_decorative.rs` — `DecorativeExpander` trait taking `&Ast` (immutable reference — Rust's borrow checker enforces the read-only invariant at compile time). `InfixRenderer` and `SExprRenderer` are implemented.
- `error.rs` — `HdlmError` enum for malformed ASTs, generation failures, expansion failures, empty trees, and unmapped symbols.

---

## ComputeBackend Architecture

The `ComputeBackend` trait abstracts where computation happens:

```rust
pub trait ComputeBackend {
    fn bind(&self, a: &BipolarVector, b: &BipolarVector) -> Result<BipolarVector, HdcError>;
    fn bundle(&self, vectors: &[&BipolarVector]) -> Result<BipolarVector, HdcError>;
    fn permute(&self, v: &BipolarVector, shift: usize) -> Result<BipolarVector, HdcError>;
    fn similarity(&self, a: &BipolarVector, b: &BipolarVector) -> Result<f64, HdcError>;
}
```

**Current:** `LocalBackend` delegates to `BipolarVector`'s native methods (bitwise ops on the host CPU).

**Future:** A `RemoteGpuBackend` will serialize vectors, dispatch to remote FOSS GPU clusters, receive results, and — critically — pass them through the PSL Supervisor as `AuditTarget::RawBytes` with `TrustLevel::Untrusted` before accepting them. This is the CARTA / Assume Breach model in action.

---

## IPC Bus Architecture

```
lfi_daemon.sh (inotifywait)
       |
       | watches close_write events on:
       |
  +----+----+          +-----+-----+
  |         |          |           |
  v         v          v           v
lfi_bus.json       lfi_audit.json
(Alpha -> Beta)    (Beta -> Alpha)
       |                    |
       +-----> LFI.log <---+
               (append-only telemetry)
```

The daemon runs as a background process, logging all IPC events with UTC timestamps. This provides a forensic audit trail of every Alpha<->Beta interaction.

---

## Error Handling Strategy

Every subsystem has its own error enum:
- `HdcError` — HDC core failures
- `PslError` — PSL audit failures
- `HdlmError` — HDLM generation/expansion failures

All operations return `Result<T, Error>`. The `?` operator is used for propagation. **No `.unwrap()`, `.expect()`, or `panic!()` anywhere in the codebase.** In tests, we use `-> Result<(), ErrorType>` function signatures with `?` instead.

---

## Future Phases

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1 | HDC Core (BipolarVector, 3 operations, similarity) | COMPLETE |
| Phase 2 | PSL Supervisor + HDLM AST | COMPLETE |
| Phase 3 | HDC Item Memory / Codebook (vector<->symbol mapping) | PLANNED |
| Phase 4 | Unified Sensorium (audio/video/image transducers) | PLANNED |
| Phase 5 | axum Web API + WebSocket backend daemon | PLANNED |
| Phase 6 | HID Injection interface (/dev/hidg0) | PLANNED |
| Phase 7 | Frontend (Android App + Web Dashboard) | PLANNED |
