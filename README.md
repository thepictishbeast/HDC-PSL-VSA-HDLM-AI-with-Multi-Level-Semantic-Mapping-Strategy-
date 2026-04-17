# LFI — Localized Forensic Intelligence

**A sovereign, self-improving neurosymbolic AI defense system.**

Built as the AI engine of [PlausiDen Technologies](https://github.com/thepictishbeast) — designed to defend sovereign users against offensive AI, mass surveillance, and automated data collection.

---

## What LFI Is

LFI is a general-purpose AI framework that combines **Hyperdimensional Computing (VSA)** with **symbolic reasoning** to produce verifiable, traceable, defensive intelligence. Unlike traditional LLMs, LFI:

- **Never claims 100% certainty** — confidence is asymptotic (max 99.99%)
- **Traces every reasoning step** — cryptographically-verifiable derivation chains
- **Refuses post-hoc rationalization** — distinguishes real recall from confabulation
- **Self-improves autonomously** — meta-learning loop adapts without human intervention
- **Runs on your hardware** — no cloud dependency, no data leakage
- **Combats offensive AI** — built-in detectors for prompt injection, AI-generated phishing, surveillance
- **Learns like a human** — textbook-based study with active recall, never sees answers during learning

---

## Live Capabilities

| Capability | Status | Notes |
|---|---|---|
| Real LLM training via Ollama | LIVE | qwen2.5-coder:7b, deepseek-r1:8b |
| Math reasoning + self-verification | LIVE | Step-by-step derivation, inverse-op check |
| Code evaluation sandbox | LIVE | Static analysis + compile + test |
| Self-improvement loop | LIVE | OODA cycles with plateau detection |
| Cross-domain analogical reasoning | LIVE | 14 structural analogies |
| Epistemic filter | LIVE | 6-tier confidence, source-weighted |
| Defensive AI threat detection | LIVE | LLM text, injection, phishing, bots |
| Continuous daemon mode | LIVE | Phase-rotating autonomous ops |
| Concurrent train+improve | LIVE | 3 parallel threads |
| Textbook learning | LIVE | Human-style active recall |
| PhD-level test framework | LIVE | 7 test categories, 23 cases |
| Answer verifier (semantic) | LIVE | Unicode, LaTeX, units, commentary |
| Training data | LIVE | 457+ examples × 49 domains |

---

## Architecture

```
+---------------------------------------------------------+
|                       LFI VSA Core                      |
|                                                         |
|  +-----------+   +-----------+   +---------------+      |
|  | HDC       |   | PSL       |   | Provenance    |      |
|  | Engine    |---| Auditor   |---| Engine        |      |
|  | 10k-bit   |   | 10 axioms |   | Traced vs     |      |
|  | bipolar   |   | CARTA     |   | Reconstructed |      |
|  +-----------+   +-----------+   +---------------+      |
|       |               |                |                |
|       +-------+-------+----------------+                |
|               |                                         |
|  +------------+-------------------------------+         |
|  |           Intelligence Layer              |         |
|  |                                            |         |
|  |  * Self-Improvement Engine                 |         |
|  |  * Cross-Domain Reasoning                  |         |
|  |  * Epistemic Filter (skeptical intake)     |         |
|  |  * Defensive AI (threat detection)         |         |
|  |  * Generalization Tester                   |         |
|  |  * Answer Verifier (semantic)              |         |
|  |  * Textbook Learner (active recall)        |         |
|  |  * PhD Test Framework                      |         |
|  |  * Math Engine (verified derivation)       |         |
|  |  * Code Evaluator (sandbox)                |         |
|  |  * Local Inference (Ollama/CLI/HTTP)       |         |
|  |  * Concurrent Runner (parallel threads)    |         |
|  |  * Continuous Intelligence Gatherer        |         |
|  |  * Daemon (7-phase rotation)               |         |
|  +--------------------------------------------+         |
+---------------------------------------------------------+
```

---

## Test Coverage

**1058 lib tests + 13 integration tests, 0 failures** across 80+ modules.

| Layer | Tests |
|---|---|
| HDC Core (vector, holographic, compute, liquid) | 80+ |
| PSL Governance (10 axioms, supervisor, coercion) | 45+ |
| Cognition (reasoner, MCTS, planner, knowledge, spaced-repetition) | 95+ |
| Intelligence (training, code eval, self-improve, verifier, textbook, phd) | 180+ |
| HDLM (AST, codebook, intercept, renderers) | 35+ |
| Crypto Epistemology (commitments, provenance) | 15+ |
| Reasoning Provenance (arena, serde, introspection) | 19 |
| Adversarial training corpus | 60+ |
| Integration tests (adversarial, stress, pipeline, agent→provenance) | 13 |

Run the suite yourself:

```bash
cd lfi_vsa_core
cargo test
```

---

## Quick Start

### Prerequisites

- Rust 1.75+
- Ollama (optional, for real LLM training)
- Linux (Debian/Ubuntu/Kali tested)

### Install

```bash
git clone https://github.com/thepictishbeast/PlausiDen-AI.git
cd PlausiDen-AI/lfi_vsa_core
cargo test --release
```

All 759 tests should pass.

### Run real LLM training

```bash
# Install and start Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2.5-coder:7b
ollama serve &

# Run training
cargo run --release --bin ollama_train -- --examples 50
```

See [OWNERS_GUIDE.md](OWNERS_GUIDE.md) for the full walkthrough in plain English.

---

## Core Principles

1. **Material reality > probabilistic prediction.** Every output is verifiable, not guessed.
2. **Epistemic honesty.** LFI distinguishes traced derivations from post-hoc rationalizations.
3. **Asymptotic confidence.** No claim reaches 100% certainty. Even formal proofs cap at 99.99%.
4. **Skeptical intake.** Unknown sources get low initial confidence. Corroboration required.
5. **Sovereign operation.** Runs entirely on your hardware. No cloud, no telemetry.
6. **Defense in depth.** Multi-layer threat detection. Assume attacker is AI-powered.
7. **Human-style learning.** Active recall from references, never sees answers during learning.

---

## Security Posture

- `#![forbid(unsafe_code)]` at crate root
- All public APIs return `Result<T, E>` or `Option<T>` — no implicit panics
- UTF-8 safe string handling throughout (34 byte-slicing panics eliminated)
- Memory-leak-free (no `Box::leak()` in production paths)
- CARTA trust model: Untrusted → Suspicious → Provisional → Verified → Sovereign
- Every axiom evaluation produces a signed provenance trace

---

## HTTP API

The axum-based server exposes a REST + WebSocket API. All JSON. Handlers are in
`lfi_vsa_core/src/api.rs`.

| Route | Method | Auth | Purpose |
|---|---|---|---|
| `/api/health` | GET | no | Subsystem readiness snapshot |
| `/api/metrics` | GET | no | Prometheus text-format counters |
| `/api/agent/state` | GET | no | Single-call dashboard summary (psl + knowledge + provenance + agent) |
| `/api/status` | GET | no | Substrate status + telemetry |
| `/api/facts` | GET | no | Persistent knowledge facts |
| `/api/qos` | GET | no | PSL axiom pass-rate probe |
| `/api/auth` | POST | — | Sovereign key verification |
| `/api/tier` | POST | yes | Manual tier switch (Pulse / Bridge / BigBrain) |
| `/api/search` | POST | no | Web search with cross-reference |
| `/api/think` | POST | no | Think **with provenance**, returns `conclusion_id` |
| `/api/audit` | POST | no | PSL governance audit over a seed-derived vector |
| `/api/opsec/scan` | POST | no | Scan text for PII / sensitive markers, return sanitized + matches |
| `/api/knowledge/review` | POST | no | Graded review (quality 0–5) — updates mastery + SM-2 schedule |
| `/api/knowledge/due` | GET | no | Concepts currently due for spaced-repetition review |
| `/api/knowledge/concepts` | GET | no | List every known concept with mastery + relations |
| `/api/knowledge/learn` | POST | **yes** | Teach a new concept (sovereign-only; auth-gated) |
| `/api/provenance/stats` | GET | no | Trace counter + traced/reconstructed note |
| `/api/provenance/:id` | GET | no | Explanation tagged Traced vs Reconstructed |
| `/api/provenance/:id/chain` | GET | no | Full `Vec<TraceEntry>` for a conclusion |
| `/api/provenance/export` | GET | **yes** | Whole arena as JSON (audit download) |
| `/api/provenance/compact` | POST | **yes** | Reclaim dead entries (invalidates TraceIds) |
| `/api/provenance/reset` | POST | **yes** | Wipe the arena (destructive) |
| `/ws/telemetry` | WS | no | Real-time substrate telemetry stream |
| `/ws/chat` | WS | no | Chat with provenance; each response includes `conclusion_id` |

The full provenance loop:

```
POST /api/think  {"input": "what is sovereignty"}
  → { "answer": "…", "confidence": 0.87, "conclusion_id": 12345678 }

GET /api/provenance/12345678
  → { "kind": { "kind": "TracedDerivation" },
      "explanation": "[Step 0 | System1FastPath … ] …",
      "confidence_chain": [0.87],
      "depth": 0 }
```

---

## Documentation

- [OWNERS_GUIDE.md](OWNERS_GUIDE.md) — plain-English setup and usage walkthrough
- [IMPROVEMENTS.md](IMPROVEMENTS.md) — active development roadmap
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — system architecture deep-dive
- [docs/HDC_OPERATIONS.md](docs/HDC_OPERATIONS.md) — VSA mathematical foundations
- [docs/PSL_SUPERVISOR.md](docs/PSL_SUPERVISOR.md) — axiom governance framework
- [docs/SECURITY.md](docs/SECURITY.md) — threat model and mitigations

---

## Hardware Targets

| Device | Status |
|---|---|
| Kali Linux / Debian workstation (i7/64GB/GPU) | Primary dev |
| Pixel 10 Pro XL (Tensor G5) | Planned (NDK build) |
| Cloud VPS (always-on training) | Supported |

---

## Mission

LFI is the core defensive component of [PlausiDen](https://github.com/thepictishbeast), a sovereign technology stack that gives individual users the same defensive capabilities that state actors and corporations already have.

**Every citizen deserves a sovereign AI defender that answers only to them.**

---

## License

Proprietary — PlausiDen Technologies. All rights reserved.
Contact the maintainer for licensing discussions.

---

**Status:** Active development. Training pipeline LIVE. 1058 lib + 13 integration tests passing.
**Last updated:** 2026-04-14
