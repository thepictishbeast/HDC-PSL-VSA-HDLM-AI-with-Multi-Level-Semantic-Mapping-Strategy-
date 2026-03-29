# Localized Forensic Intelligence (LFI)

**Version:** 5.6 (The Ground Zero Protocol + Remote Sovereign Sync)
**Lead Engineer:** Paul (PlausiDen Technologies)
**Architecture:** Neuro-Symbolic Hyperdimensional Computing Agent

---

## Mission Statement

Construct a deterministic, autonomous, multimodal Neuro-Symbolic agent that obsoletes probabilistic LLM architectures. **Material Reality > Probabilistic Prediction.**

LFI replaces the high-parameter probabilistic weights of legacy LLMs with verifiable, deterministic computation built on Vector Symbolic Architectures (VSA) and Hyperdimensional Computing (HDC).

---

## Core Architecture

LFI is composed of four tightly integrated subsystems:

```
+-------------------------------------------------------------------+
|                        LFI VSA CORE                               |
|                                                                   |
|  +-------------------+    +-------------------+                   |
|  |   HDC Core        |    |   PSL Supervisor  |                   |
|  |   (Logic/Compute) |<-->|   (The Auditor)   |                   |
|  |                   |    |                   |                   |
|  | BipolarVector     |    | Axiom trait       |                   |
|  | 10,000-dim {-1,+1}|    | TrustLevel (CARTA)|                   |
|  | Bind (XOR)        |    | AuditTarget       |                   |
|  | Bundle (Sum+Clip) |    | PslSupervisor     |                   |
|  | Permute (Shift)   |    | AxiomVerdict      |                   |
|  | ComputeBackend    |    |                   |                   |
|  +-------------------+    +-------------------+                   |
|           |                        |                              |
|           v                        v                              |
|  +---------------------------------------------------+            |
|  |              HDLM (Language/Generation)            |           |
|  |                                                    |           |
|  |  Tier 1 (Forensic):  Token -> AST (verified)       |           |
|  |  Tier 2 (Decorative): AST -> Code/Prose (render)   |           |
|  |                                                    |           |
|  |  AST arena with optional HV fingerprints           |           |
|  |  InfixRenderer | SExprRenderer                     |           |
|  +---------------------------------------------------+            |
+-------------------------------------------------------------------+
         |                    |                    |
    LocalBackend         Remote GPU           IPC Bus
    (ARM SIMD)           (CARTA/ZT)       (lfi_daemon.sh)
```

### I. HDC Core (Logic & Compute)

Employs **10,000-dimensional bipolar hypervectors** (`V in {-1, 1}^10000`). All reasoning relies on three bitwise algebra operations:

| Operation | Implementation | Algebraic Properties |
|-----------|---------------|---------------------|
| **Binding** | XOR | Commutative, associative, self-inverse |
| **Bundling** | Sum + Clip | Commutative, majority-vote superposition |
| **Permutation** | Cyclic left shift | Invertible, weight-preserving |

Execution defaults to local ARMv9.2-A (SIMD/NEON on Tensor G5 Laguna) but uses a modular `ComputeBackend` trait for remote GPU dispatch.

### II. PSL Supervisor (The Auditor)

A Probabilistic Soft Logic layer acting as a **"Hostile Witness."** Verifies all VSA outputs, external GPU returns, and file ingestions against material axioms (physics, logic, security) to enforce a **Zero-Hallucination** environment.

- **CARTA Trust Model:** Untrusted -> Suspicious -> Provisional -> Verified -> Sovereign
- **Axiom trait:** Pluggable verification rules (Beta defines logic, Alpha builds infrastructure)
- **AuditTarget variants:** Vector, RawBytes, Scalar, Payload

### III. HDLM (Language & Generation)

Multi-Level Semantic Mapping with strict tier separation:

- **Tier 1 (Forensic):** Generates mathematically perfect Abstract Syntax Trees (ASTs). The AST IS the truth.
- **Tier 2 (Decorative):** Expands the AST into aesthetic code or human prose. **Read-only on the AST.** Cannot alter logic.

Each AST node can carry an optional `BipolarVector` fingerprint for bidirectional HDC<->HDLM mapping.

### IV. Unified Hyperdimensional Sensorium (Planned)

FOSS transducers to project audio, video, images, and arbitrary file binaries into the unified 10,000-bit VSA space. Media generation via a local Hybrid Synthesis Engine.

---

## Hardware Target

| Component | Specification |
|-----------|--------------|
| **Root Hardware** | Google Pixel 10 Pro XL (Tensor G5 "Laguna" SoC) |
| **Architecture** | ARMv9.2-A, SIMD/NEON |
| **Elastic Compute** | Remote FOSS GPU Clusters (via CARTA / Assume Breach) |
| **Operating Environment** | Aarch64 / Debian-based Linux (Proot Layer) |

---

## The Four Segregated Workflows

Strict segregation of duties to eliminate cross-contamination:

| Workflow | Role | Agent | Responsibility |
|----------|------|-------|----------------|
| **Alpha** | The Architect | Claude Code | Structural engineering, Rust/C++/ASM, Web API, transducers. **Prohibited from inventing logic rules.** |
| **Beta** | The Auditor | Gemini 3.1 Pro | Verification, security, PSL axiom definition, dialectical skepticism. |
| **Gamma** | The Chronicler | State Persistence | Manages STATE.md, CLAUDE.md, GEMINI.md, git protocol. |
| **Delta** | The Watchdog | Telemetry | Strict logging mandates. Assume all code is broken until logs prove success. |

### IPC Protocol

Alpha and Beta communicate via file ledger:
- `lfi_bus.json` — Alpha writes output payloads here
- `lfi_audit.json` — Beta writes audit resolutions here
- `lfi_daemon.sh` — Monitors both files via `inotifywait`, logs to `LFI.log`

---

## Security Posture

- **`#![forbid(unsafe_code)]`** at crate root — absolute memory safety
- **All operations return `Result<T, E>` or `Option<T>`** — no implicit failure
- **`.unwrap()`, `.expect()`, `.panic!()` are strictly forbidden** — even in non-test code
- **Zero-Trust / Assume Breach** — all external data enters at `TrustLevel::Untrusted`
- **CARTA model** — Continuous Adaptive Risk and Trust Assessment on every datum

See [docs/SECURITY.md](docs/SECURITY.md) for the full security posture document.

---

## Building & Testing

### Prerequisites

- Rust toolchain (edition 2021+)
- `inotify-tools` (for `lfi_daemon.sh`)

### Build

```bash
cd lfi_vsa_core
cargo build
```

### Test

```bash
cd lfi_vsa_core
cargo test
```

Current test suite: **89 tests across 3 subsystems, 0 failures.**

| Subsystem | Test Count | Coverage |
|-----------|-----------|----------|
| HDC Core | 48 | Algebraic proofs, statistical distribution, cross-operation recovery |
| PSL Supervisor | 13 | Audit pipeline, trust levels, hostile data, threshold tuning |
| HDLM | 28 | AST construction/traversal, Tier 1 parsing, Tier 2 rendering, mutation invariant |

See [docs/TESTING.md](docs/TESTING.md) for the full test strategy.

---

## Delta Telemetry

**Assume all code is fundamentally broken until the log proves success.**

Every function, every branch, every edge case emits a `[DEBUGLOG]` line:

```
[DEBUGLOG][src/hdc/vector.rs:42] - new_random: dim=10000
[DEBUGLOG][src/psl/supervisor.rs:87] - audit: axiom=Axiom:Dimensionality_Constraint, passed=true, tv=1.0000
```

All telemetry is structurally isolated via the `debuglog!` and `debuglog_val!` macros for seamless production stripping. See [docs/TELEMETRY.md](docs/TELEMETRY.md) for the complete telemetry map.

---

## Project Structure

```
lfi_project/                          # Sovereign root directory
├── .git/                             # Version control (Atomic Git Protocol)
├── .github/
│   └── dependabot.yml                # Dependency update automation
├── .gitignore                        # Excludes target/ build artifacts
├── README.md                         # This file
├── CLAUDE.md                         # Workflow Alpha state document
├── STATE.md                          # Gamma Handoff state for agent continuity
├── LFI.log                           # Delta telemetry daemon log
├── lfi_bus.json                      # IPC: Alpha -> Beta payload ledger
├── lfi_audit.json                    # IPC: Beta -> Alpha audit resolution
├── lfi_daemon.sh                     # IPC watchdog daemon (inotifywait)
├── docs/
│   ├── ARCHITECTURE.md               # System architecture deep-dive
│   ├── PROJECT_STRUCTURE.md          # Per-file descriptions
│   ├── HDC_OPERATIONS.md             # Mathematical foundations
│   ├── PSL_SUPERVISOR.md             # PSL framework documentation
│   ├── HDLM_AST.md                  # HDLM multi-level semantic mapping
│   ├── TELEMETRY.md                  # Debuglog location map
│   ├── TESTING.md                    # Test strategy and coverage
│   └── SECURITY.md                   # Security posture and invariants
└── lfi_vsa_core/                     # Rust library crate
    ├── Cargo.toml                    # Dependencies: bitvec, rand, serde, serde_json
    ├── Cargo.lock                    # Pinned dependency versions
    ├── .gitignore                    # Excludes target/
    ├── src/
    │   ├── lib.rs                    # Crate root: #![forbid(unsafe_code)], module wiring
    │   ├── telemetry.rs              # debuglog! and debuglog_val! macros
    │   ├── hdc/                      # Phase 1: Hyperdimensional Computing Core
    │   │   ├── mod.rs                # Module exports
    │   │   ├── error.rs              # HdcError enum
    │   │   ├── vector.rs             # BipolarVector + all HDC algebra + 48 tests
    │   │   └── compute.rs            # ComputeBackend trait + LocalBackend + 4 tests
    │   ├── psl/                      # Phase 2A: Probabilistic Soft Logic Supervisor
    │   │   ├── mod.rs                # Module exports
    │   │   ├── error.rs              # PslError enum
    │   │   ├── axiom.rs              # Axiom trait + AuditTarget + built-in structural axioms
    │   │   ├── trust.rs              # TrustLevel (CARTA) + TrustAssessment
    │   │   └── supervisor.rs         # PslSupervisor engine + 13 tests
    │   └── hdlm/                     # Phase 2B: Hyperdimensional Language Model
    │       ├── mod.rs                # Module exports
    │       ├── error.rs              # HdlmError enum
    │       ├── ast.rs                # AST arena + NodeKind + DFS/BFS + 11 tests
    │       ├── tier1_forensic.rs     # ForensicGenerator trait + ArithmeticGenerator + 9 tests
    │       └── tier2_decorative.rs   # DecorativeExpander trait + renderers + 8 tests
    └── tests/
        └── forensic_audit.rs         # Integration test: Hamming weight statistical audit

```

See [docs/PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md) for detailed per-file descriptions.

---

## Git History

| Commit | Phase | Description |
|--------|-------|-------------|
| `827ee4e` | Phase 1 | INIT: VSA Core Baseline (HDC algebra, 48 tests) |
| `15e4678` | Audit | Beta cleared Phase 1 (5 axioms verified) |
| `d12feb3` | Phase 2 | PSL Supervisor + HDLM AST (89 tests) |
| `448319a` | Merge | Merged origin/main dependabot config |

---

## License

Proprietary - PlausiDen Technologies. All rights reserved.
