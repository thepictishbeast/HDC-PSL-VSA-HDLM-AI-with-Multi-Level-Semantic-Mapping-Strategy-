# LFI Master Roadmap — Execution Order For Everything

**Purpose:** Orchestrate the design documents and sprint plans into a single executable sequence. Claude Code can read this doc and know what to build next, when, and why. The operator can read this doc and know what state LFI is in, what's coming, and what decisions need to be made.

**Dependency model:** Items listed in the same phase can run in parallel if engineering resources allow. Items in later phases depend on earlier phases completing. Some specific dependencies are noted where relevant.

## Phase 0 — Unblock and stabilize (0-4 weeks)

Nothing else proceeds until these are done.

- **0.1 — Repo naming cleanup** (1 day) — fix lowercase/capitalized collisions, delete empty placeholders, update workspace references.
- **0.2 — Framing alignment** (3 days) — strike any remaining LLM-training references (SFT, RLHF, DPO, ORPO, GRPO, Magpie, reward models, preference pairs, fine-tuning, chat wrappers). Replace with neurosymbolic equivalents.
- **0.3 — Test baseline verification** (1 week) — run the full 1,742-test suite. Document flaky tests. Fix or skip with justification.
- **0.4 — CI infrastructure** (1-2 weeks) — GitHub Actions running tests, lints, benchmarks, and timing-invariance checks on every PR. Artifact signing via Sigstore.

**Exit criteria:** clean repo structure, consistent framing, green test baseline, working CI.

## Phase 1 — Security critical (4-12 weeks)

All four must ship before LFI touches sensitive data in production.

- **1.1 — HDC bundling hardening** (3 weeks) — tier-weighted two-stage voted bundle with trimmed-mean and outlier rejection. HyperDefense redundant critical-dimension replication. BCH/Hamming ECC over prototype blocks. Replaces flat bundling in lfi_vsa_core. Unit test: 10% bit-flip attack drops naive HDC 90.9% → 10%, hardened bundle retains >80%.
- **1.2 — Constant-time HDC primitives** (2 weeks) — subtle crate, ct_argmax cleanup, zeroize::Zeroizing on intermediate sums, dudect-bencher timing-invariance tests.
- **1.3 — Encoder reversibility defense** (2-3 weeks) — HDLock secret-permutation stack. Per-deployment permutation keyed by ChaCha20Rng. Key in Confidentiality Kernel. ~10^10× more queries required for encoder inversion.
- **1.4 — Incremental fact ingestion** (2 weeks) — running-sum approximation for prototype updates. Single-fact ingestion without full rebuild. Periodic reconciliation during wake-sleep cycles.

**Runs parallel:** 1.1 and 1.2 if two engineers. 1.3 depends on 1.2. 1.4 independent.

## Phase 2 — Confidentiality Kernel (10-13 weeks)

Per `LFI-CONFIDENTIALITY-KERNEL-DESIGN.md`. Makes plaintext leaks structurally impossible.

- **2.1 — Type system foundation** (3 weeks) — lfi-conf-kernel crate, Sealed<T>, Sensitive trait, broker primitives, audit kernel, trybuild compile-fail tests.
- **2.2 — Memory protection** (2 weeks) — key-store detection (SEV-SNP/TDX/SGX/TrustZone/SEP/TPM/kernel-keyring/userspace), SecretAllocator with mlock + MADV_DONTDUMP + zeroize, prctl process hardening, startup preflight.
- **2.3 — Network egress control** (3 weeks) — SecureChannel with rustls TLS-1.3-only, CT verification, egress firewall via cgroups+nftables, LlmApiBroker with prompt-builder secret substitution.
- **2.4 — Egress scanner + chat scrubber** (2 weeks) — four-tier classifier (gitleaks patterns, entropy detector, GLiNER PII NER, vault-membership check). ChatScrubber daemon with shell-history + terminal-scrollback + LFI-log source adapters. RotationDispatcher (AWS IAM, GitHub PAT, GitLab token, npm token, SSH key, GPG subkey).
- **2.5 — Encrypted computation** (optional, 4-6 weeks) — TEE adapters, TFHE-rs FHE, Shamir + MPC framework.
- **2.6 — ZK operation proofs** (optional, 3 weeks) — Nova folding via nova-snark. Proof attachment to audit entries. Verifier service.

## Phase 3 — PlausiDen Secrets (10-13 weeks)

Per `PLAUSIDEN-SECRETS-DESIGN.md`. Depends on 2.1-2.2 minimum. Parallel with 2.3-2.4.

- **3.1 — Core types + SshBroker** (3 weeks)
- **3.2 — WebAuth, ApiKey, GpgSigning brokers** (3 weeks) — Anthropic, OpenAI, GitHub, AWS, GCP, Stripe, Twilio handlers; WebAuthn via webauthn-rs 0.5.2; TOTP via totp-rs
- **3.3 — Macaroon delegation + mesh integration** (4 weeks)
- **3.4 — PII subsystem + chat scrubber wiring** (3 weeks) — PiiBroker with legal-basis enforcement, retention, right-to-erasure, breach notification, scrubber-triggered vault rotation

## Phase 4 — Fact corpora ingestion (15-20 weeks, parallelizable)

Per `LFI-FACT-CORPORA.md`. Requires Phase 1.4.

- **4.1 — Formal verification backbone** (2 weeks) — Metamath set.mm, Mathlib4, NIST reference data, Sigstore Rekor. Tier 0.95.
- **4.2 — Causal and commonsense** (3 weeks) — CauseNet-Precision → Full, ATOMIC-2020, GLUCOSE, e-CARE, AnalogyKB.
- **4.3 — Security and code** (3 weeks) — MITRE ATT&CK + CAPEC + CWE, CVE v5, top 5000 crates.io, Primus-FineWeb cyber subset.
- **4.4 — Biomedical** (2 weeks, license dependent) — UniProt, ChEMBL, SemMedDB (UMLS license), DrugBank academic.
- **4.5 — Academic and legal** (3 weeks) — OpenAlex monthly, CaseLaw Access Project, US Code, Getty Vocabularies.
- **4.6 — Multilingual and calibration** (2 weeks) — FLORES-200, Aya Collection, Panlex, Open Multilingual Wordnet, SelfAware/HoneSet/CalibratedMath.
- **4.7 — Broad coverage and events** (ongoing) — Wikipedia structured, Web Data Commons, EventKG, selective arXiv.

**Exit criteria:** fact store 50.7M → 200-400M facts, all with tier/temporal-class/provenance-chain/PSL-validated.

## Phase 5 — Learning mechanisms (15-20 weeks)

Per `LFI-TRAINING-METHODOLOGY.md`. Depends on Phase 1 + Phase 4.

- **5.1 — Prototype consolidation hardened** (shipped in Phase 1.1)
- **5.2 — Axiom refinement** (4-6 weeks) — online rule-weight updates, pattern-mining proposal generator, safety checks, formal-verification feedback.
- **5.3 — Metacognitive calibration** (1-2 weeks) — per-domain outcome log, Platt calibrator, abstention-threshold learning.
- **5.4 — FSRS scheduler** (2-3 weeks) — fsrs-rs integration, per-fact-type review, tier-demotion on fail.
- **5.5 — Stitch library learning + wake-sleep** (3-4 weeks) — Stitch integration with reasoning traces, wake-sleep consolidation scheduler, template extraction to cognition operator library.

## Phase 6 — Cognition module enhancements (20-25 weeks)

- **6.1 — Natural gradient on AIF Fisher manifold** (3 weeks) — K-FAC or diagonal Fisher, replace vanilla SGD in active_inference.rs.
- **6.2 — Global Workspace attention bottleneck** (1-2 weeks) — Goyal et al. ICLR 2022 key-value GWT, k=4-8 slot competition, integration with cognition dispatch.
- **6.3 — Tensor-train precision tier** (6 weeks) — tensor-train-rs crate, TT-SVD/contraction/addition/rounding, HDC integration for precision-critical structures.
- **6.4 — Formal verification integration** (6-8 weeks) — Kimina Lean Server via Rust HTTP, PSL policy for Required/BestEffort/None dispatch, three-tier ProofCertificate enum, commit-reveal synergy.
- **6.5 — Reasoning provenance retrieval** (2 weeks) — DerivationStore trait, "why did LFI conclude X" query API.
- **6.6 — Reasoning chain metrics** (3-4 weeks) — ROSCOE, ReCEval, MR-Ben integration. Dashboard. CI quality gates.

## Phase 7 — Mesh federation (12-15 weeks)

Depends on Phases 2, 3, 5.

- **7.1 — CRDT fact federation** (3-4 weeks) — per-dimension PN-counter CRDTs, libp2p gossip, eventual-consistency verification.
- **7.2 — EigenTrust propagation** (3-4 weeks) — sparse CSR trust matrix, power iteration, Kademlia distribution, Jøsang Subjective Logic composition.
- **7.3 — Cross-device handoff** (2 weeks) — Global Workspace serialization, encrypted libp2p channel, continuity verification.
- **7.4 — Sheaf consistency checking** (4-6 weeks) — cellular sheaf over fact graph, Sheaf Laplacian, consistency radius, subgraph via sprs.

## Phase 8 — Mobile port (8-12 weeks)

Depends on Phases 1-6. Enables Phase 9.

- **8.1 — ARM64 cross-compilation** (1-2 weeks) — Nix-based reproducible build, CI cross-compilation, signed artifacts.
- **8.2 — NEON SIMD optimization** (3-4 weeks) — replace x86 SIMD with ARM NEON in all HDC hot paths, Pixel benchmark verification.
- **8.3 — Mobile memory management** (2-3 weeks) — fact-store pagination, memory-pressure-aware caching, hot-prototype residency.
- **8.4 — Mobile UI** (2-3 weeks) — Tauri-based mobile app. Settings, dashboard, query, audit log viewer.
- **8.5 — Battery optimization** (1-2 weeks) — verify 2-4 W steady-state, idle <100 mW.

**Exit criteria:** Pixel 7+, <3s latency, <5% battery/hour active.

## Phase 9 — PlausiDenOS integration (12-16 weeks)

Depends on Phase 8 + PlausiDenOS base.

- **9.1 — seL4 protection domain decomposition** (4-6 weeks) — each module a seL4 task, capability-restricted IPC, PSL Supervisor elevated capability.
- **9.2 — TrustZone Trusted Application** (3-4 weeks) — broker enclave as OP-TEE TA, key-store backed by TrustZone.
- **9.3 — Microkernel-level confidentiality** (3-4 weeks) — Confidentiality Kernel at seL4 capability layer.
- **9.4 — Mobile-first UX** (2 weeks) — keyboard, voice, gesture refinement.

## Phase 10 — Sacred.Vote integration (parallel track, 8-12 weeks)

- **10.1 — Multi-party operations** (4 weeks) — MeshBroker for you + Tim + DLD, threshold signatures, MPC for shared secret manipulation.
- **10.2 — zkTLS proof integration** (4-6 weeks) — Lean4 formal verification of DLD zkTLS claims, brokered API to Sacred.Vote TypeScript.
- **10.3 — Audit trail for election operations** (2 weeks) — specialized audit log with C2PA attestation, Rekor transparency, multi-signer quorum.

## Phase 11 — Public release preparation (6-10 weeks)

- **11.1 — Documentation sweep** (3 weeks)
- **11.2 — Benchmark suite** (2-3 weeks) — formal proof pass rate, provenance completeness, prototype purity, calibration ECE per domain, reasoning chain metrics, retrieval latency.
- **11.3 — Packaging and distribution** (3 weeks) — Debian, Android APK, F-Droid-style signed, Flatpak, installer scripts, Sigstore.
- **11.4 — External audit** (4 weeks, external contractor) — security, cryptographic review, HIPAA/SOC 2/GDPR compliance.

## Resource profile

- Total engineering: ~120-160 weeks of focused work
- Calendar with parallelization: ~18-30 months to complete everything
- **Critical path:** Phase 0 → 1 → 2 → 3 → 8 → 9 → 11 (~45-55 weeks)
- **V1.0 (12-18 mo):** Phases 0, 1, 2, 3, 4 (partial), 5, 6, 8
- **V1.5 (18-24 mo):** Phases 4 (complete), 7, 9, 10
- **V2.0 (24+ mo):** Phase 11, lower-priority Improvements

## Operator decision points

- Phase 2.5 / 2.6 — encrypted compute + ZK proofs: ship in V1.0 or defer?
- Phase 4.4 — biomedical: pursue UMLS license for SemMedDB or skip?
- Phase 6.4 — Lean4 or Metamath-only? (Metamath simpler)
- Phase 9 timing — aligned with PlausiDenOS readiness or independent?
- Phase 10 timing — Sacred.Vote V1 coupled to LFI V1 or separate tracks?

## Session baseline (20-30% complete)

**Existing:** 77,000 LOC Rust, 1,742 passing tests, 50.7M-fact store, HDC/VSA/PSL/HDLM operational, causal reasoning, metacognition, Active Inference, crypto epistemology.
**Missing:** confidentiality kernel, secrets layer, most fact corpora, learning-mechanism upgrades, mobile port, mesh federation, PlausiDenOS integration.

**Execute.**
