# CLAUDE.md — Workflow Alpha State Document

**Agent:** Claude Code (Workflow Alpha — The Architect)
**Last Updated:** 2026-03-27
**Protocol Version:** 5.6

---

## Role Boundaries

- **Responsible for:** Structural engineering, Rust/C++/ASM, Web API, transducer bridging
- **Prohibited from:** Inventing logic rules (Beta's domain)
- **Test mandate:** Exhaustive unit tests proving mathematical properties before integration
- **Telemetry mandate:** `debuglog!` in every function, every branch, every edge case

---

## Completed Phases

### Phase 1: HDC Core (Commit 827ee4e)
- `BipolarVector`: 10,000-dim bipolar hypervectors via bitvec
- Three operations: Bind (XOR), Bundle (Sum+Clip), Permute (Cyclic Shift)
- Cosine similarity + Hamming distance
- `ComputeBackend` trait + `LocalBackend`
- 48 tests, all passing
- **Beta cleared:** 5 axioms verified (Commit 15e4678)

### Phase 2: PSL Supervisor + HDLM AST (Commit d12feb3)
- PSL: `Axiom` trait, `PslSupervisor`, `TrustLevel` (CARTA), `AuditTarget`, `AxiomVerdict`
- Built-in structural axioms: `DimensionalityAxiom`, `DataIntegrityAxiom`
- HDLM: `Ast` arena, `NodeKind` (13 variants), `ForensicGenerator`, `DecorativeExpander`
- Tier 1: `ArithmeticGenerator` (prefix -> AST)
- Tier 2: `InfixRenderer`, `SExprRenderer` (read-only on AST)
- **Beta cleared:** Phase 2 audit passed

### Phase 3: Agent, API, HID, Adaptive + Alpha Audit (Commit 584d2c9)
- Gemini scaffolded: `LfiAgent`, axum REST API, `HidDevice`, `UiElement` folding, `HdlmCodebook`, `SemanticMap`
- Alpha audited and fixed 9 directive violations (see GEMINI_UPDATE.md)
- `BinaryTransducer` implemented by Alpha
- 97 tests total after audit
- **Beta cleared:** Phase 3 audit passed

### Phase 5A: Multimodal Transducers + Codebook Fixes (Uncommitted)
- **Audio transducer:** PCM frame-level spectral encoding
- **Image transducer:** Patch-level 2D spatial encoding (grayscale + RGB)
- **Text transducer:** Character n-gram encoding with lazy alphabet
- **Codebook:** Fixed `identify_kind` placeholder, added `encode_node`, `kind_count`
- **CodebookGenerator:** Implements `generate_from_vector` via codebook reverse lookup
- **PSL:** Added `StatisticalEquilibriumAxiom` for Hamming weight verification
- 130 tests total, 0 warnings
- **Beta audit:** Pending

---

## Current Git State

```
584d2c9 PHASE3: Agent, API, HID, Transducers + Alpha Forensic Audit (PUSHED)
448319a Merge origin/main (dependabot config)
d12feb3 PHASE2: PSL Supervisor + HDLM AST Generation Infrastructure
15e4678 AUDIT: Phase 1 VSA Core cleared by Beta (Gemini)
827ee4e INIT: VSA Core Baseline — Ground Zero Protocol v5.6
```

Phase 5A work is uncommitted, awaiting Beta audit.

---

## Conventions

- `#![forbid(unsafe_code)]` — never removed
- All ops return `Result<T, E>` — no `.unwrap()`, `.expect()`, `panic!()`
- `debuglog!` in every function, every branch
- Tests use `-> Result<(), ErrorType>` with `?` operator
- Forensic commit messages with full change summary
- Push to both `master` and `main` after every commit
- Write `lfi_bus.json` payload after every phase completion
- Yield to Beta after every phase for audit

---

## Next Steps (Pending Beta Clearance of Phase 5A)

1. **Phase 5B:** WebSocket API for live telemetry streaming
2. **Phase 5C:** Frontend scaffold (Web Dashboard)
3. **Phase 5D:** Remote GPU backend (`RemoteBackend`)
4. **Phase 5E:** End-to-end sensorium integration test
5. **Phase 6:** CARTA offensive probe framework
