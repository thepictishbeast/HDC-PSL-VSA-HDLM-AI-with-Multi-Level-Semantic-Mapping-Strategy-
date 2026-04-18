# LFI Improvement Areas and Fixes — Gap Analysis Against Current Codebase

**Baseline:** 77,000 LOC Rust, 1,742 passing tests, 50.7M facts, operational HDC/VSA/PSL/HDLM + causal + metacognition + Active Inference + crypto_epistemology.

Each item: gap → consequence → fix → effort.

## Critical — fix before further scaling

### C1. Repository naming collision (1 day)
Lowercase/capitalized variants (plausiden-auth/PlausiDen-Auth, plausiden-vault/PlausiDen-Vault) confuse path resolution. Pick one canonical casing (lowercase-hyphenated to match Rust crate naming). Delete empties. Update Cargo.toml + CLAUDE.md refs. Pre-commit hook rejecting case-collisions.

### C2. HDC bundling vulnerable to poisoning (3 weeks)
Flat bundling: tier-7 web claim = tier-1 theorem. HyperAttack drops 90.9% → 10% with 10% bit-flip. Fix: tier-weighted two-stage voted bundle + α=15% trimmed-mean + 3σ outlier rejection + HyperDefense critical-dim replication + BCH/Hamming ECC.

### C3. No incremental fact ingestion (2 weeks)
Batch-only assumption. Full prototype recompute per fact = mobile impossible. Fix: running-sum approximation for prototype updates + periodic wake-sleep reconciliation.

### C4. No constant-time HDC (2 weeks)
bind/bundle/cosine branch on values; cleanup argmax leaks via timing. Tahoori 2025 demonstrates FPGA side-channel recovery. Fix: subtle::ConstantTimeEq, ct_argmax with uniform memory access, zeroize::Zeroizing<Vec<i8>> on intermediates, dudect-bencher timing-invariance tests.

## High priority

### H5. Encoder reversibility (2-3 weeks)
PRID/HDLock reverse fixed-random encoders in ~10^4 queries. Fix: HDLock secret-permutation stack keyed by ChaCha20Rng, per-deployment rotation, key in Confidentiality Kernel key store. Raises attacker cost ~10^10× at 21% runtime overhead.

### H6. No retrieval-time access control (3-4 weeks)
Facts returned plaintext regardless of PII/access-control. Fix: per-fact access-control label (public/operator/capability-required/PII-subject-X). Retrieval requires capability chain. PII through PiiBroker. Ties into PlausiDen Secrets Sprint 1.

### H7. Provenance exists but not queryable (2 weeks)
TracedDerivation records exist; no "why did LFI conclude X" API. Fix: DerivationStore trait with get_derivation(conclusion_id) + search_by_premise/rule. Index by conclusion hypervector hash. Interaction-layer "show me why" expansion.

### H8. No tensor-train precision tier (3-4 + 2 weeks)
HRR crosstalk accumulates in deeply-nested structures. Fix: build tensor-train-rs (no prod-quality Rust impl exists). TT-SVD, contraction, addition, rounding. Promote at policy-specified points (Required-class proofs). At D=10k, k=4, χ=128: 20M floats representing 10^16-dim tensor.

### H9. No natural gradient on AIF Fisher manifold (3 weeks)
active_inference.rs uses vanilla SGD on VFE; VFE lives on curved Riemannian manifold with Fisher metric. Vanilla SGD drifts. Fix: K-FAC or diagonal-Fisher. θ ← θ - η · G⁻¹ · ∇F. ~200-500 LOC nalgebra for diagonal, 800-1500 for K-FAC block-diagonal.

### H10. No Global Workspace attention bottleneck (1-2 weeks)
Working memory = unbounded bundle with interference. Fix: Goyal ICLR 2022 key-value GWT, k=4-8 slots, softmax-selected broadcast. ~200 LOC.

### H11. Reasoning chain metrics not collected (3-4 weeks)
ROSCOE/ReCEval/MR-Ben not integrated. No regression detection. Fix: evaluation harness + per-domain calibration curves + CI quality gate.

## Medium priority

### M12. No CRDT-compatible fact federation (3-4 weeks)
Naive HDC bundling not associative over multisets. Fix: per-dimension PN-counter CRDT; state S ∈ ℤ^d, join = max per-replica, readout = sign(S). Proper state-based CRDT. δ-gossip ~10 KB/message (2 bits/dim packed). Build on crdts::PNCounter + libp2p gossip.

### M13. No formal verification integration (6-8 weeks)
No machine-checked proofs. Fix: Kimina Lean Server integration via Rust HTTP. PSL policy Required|BestEffort|None. Three-tier ProofCertificate: Term, Tactic, Reference(CID), Unproven. Commit-reveal synergy.

### M14. No sheaf consistency checking (4-6 weeks)
Silent contradictions accumulate. Fix: cellular sheaf over fact graph. Stalks F(v), F(e), restriction maps. Global sections = consistent knowledge. H¹ = incoherence signature. Sheaf Laplacian L_F = δᵀδ sparse. Compute on ~10⁵-node subgraphs in minutes via sprs + nalgebra-sparse.

### M15. No EigenTrust propagation (3-4 weeks)
Trust is per-source static. Fix: sparse CSR trust matrix, power iteration t^(k+1) = (1-α)·C^T·t^(k) + α·p. Converges ~100 iterations to ε=1e-6 on 10⁶ nodes. libp2p Kademlia for mesh. Compose with Jøsang Subjective Logic.

### M16. Working memory too small (2-3 weeks)
Fix: hierarchical tiers (immediate ~8, recent ~64, session ~512) with decaying salience. Hysteresis to prevent thrashing. Explicit pin capability. Memory-pressure eviction for mobile.

### M17. No streaming fact updates (3 weeks)
Temporal facts (news, sensors) lag. Fix: streaming ingestion API (gRPC), per-source rate limits + quality gates, libp2p gossip for mesh propagation, auto tier-decay at half-life.

### M18. HDLM codebook monolingual-focused (4-6 weeks)
English WordNet-derived. FLORES-200/Aya can't properly feed lemma-to-concept. Fix: Open Multilingual Wordnet ingestion per-language, Panlex for low-resource, stanza morphology analyzers (70+ langs), unified semantic at synset level.

### M19. No automated adversarial corpus (4 weeks)
1,010 facts insufficient. Fix: synthetic via PSL axiom-guided mutation, contradiction-harvesting, attacker-simulation framework (HyperAttack bit-flip patterns, deauth-equivalents, poisoning). Target 50K-100K across 14 domains.

### M20. Sensory transducers incomplete (6-8 weeks)
Image=simple patches, audio=basic spectrogram, binary=no structural parser (PE/ELF/Mach-O/PDF/Office). Fix: hierarchical image (patch→region→scene), PCA-learned audio codebook, binary-format structural parsers, video = temporal permutation of image hvs.

## Lower priority

### L21. Mobile memory opt (2-3 weeks) — Apache Arrow columnar, delta-encoded timestamps, bit-packed prototypes
### L22. LFI-native benchmark suite (3-4 weeks) — miniF2F/ProofNet/PutnamBench, provenance completeness, prototype purity, calibration ECE per domain, chain metrics, mobile retrieval latency
### L23. Observability UI (4-6 weeks) — Tauri dashboard: working memory, prototype viz, axiom weights history, reasoning trace explorer, fact store + provenance, calibration curves, session replay
### L24. Code generation operator library (4-6 weeks) — 300-500 Rust construct operators with PSL atoms, from Rust grammar parse
### L25. Cross-platform build (2-3 weeks) — Nix reproducible build for all targets + signed artifacts + CI
### L26. Documentation coverage (continuous, 2-3 week initial pass) — module-level doc comments, auto-generated architecture docs, operator manual, dev guide
### L27. Performance profiling (2-3 weeks) — flamegraph-rs + criterion benchmarks, top-20 hot paths optimized
### L28. Packaging and distribution (3-4 weeks) — .deb, Android APK, F-Droid-signed, Flatpak, installer scripts

## Anti-patterns (ongoing discipline)

- **A1. LLM-framing drift** — design docs state "not an LLM" ground rule. PRs introducing transformer/attention/tokenizer require architectural review. Regular audits catch drift early.
- **A2. Scaffolding accumulation** — `// SCAFFOLDING:` marker required. CI rejects scaffolding → production. Monthly tools/ audit to sunset scaffolding >30 days old.
- **A3. Over-restrictive assistant heuristics** — operator's stated constraints authoritative. Push back once on safety-critical ambiguity, accept, proceed. No "but you might want to consider…" softening.
- **A4. Network-destructive commands without preflight** — wlan0 UP + managed + default route + NetworkManager running check. Banned: airmon-ng, `iw dev wlan0 set type monitor`, `systemctl stop NetworkManager`. Post-op verification with auto-abort.
- **A5. Large speculative refactors** — prefer incremental PRs each compilable + testable.
- **A6. Feature without tests** — proptest/unit tests mandatory. 1,742-test baseline must grow with codebase, not shrink.
- **A7. Bypassing Confidentiality Kernel** — all secret-handling routes through Sealed<T> + use_within closure. Bypass fails CI.

## Architectural commitments

- Rust-native always. Python/shell scaffolding marked + removed before ship.
- Sovereign operation — no cloud required for any core function.
- Provenance for everything — every fact/reasoning step/axiom weight change/prototype update carries provenance.
- Constant-time where sensitive.
- Mobile-first UX — every feature works on mobile; desktop-only needs justification.

## Release roadmap

- **V0.9** (next 3 months): C1-C4 + H5-H11 done. Desktop operational. Functional mobile port starts.
- **V1.0** (6-9 months): All critical + high priority. Functional mobile deployment. Mesh MVP. Public technical preview.
- **V1.5** (12-18 months): Medium priority fixes. Polished mobile. Sacred.Vote integration. First enterprise deployments.
- **V2.0** (18-24 months): Lower priority + polish. Mature ecosystem.

## Priority sequence if resources limited

1. C1 — repo naming (1d)
2. C2 — HDC bundling (3w)
3. C3 — incremental ingestion (2w)
4. C4 — constant-time HDC (2w)
5. H5 — encoder reversibility (2-3w)
6. H6 — retrieval access control (3-4w)
7. H7 — provenance retrieval API (2w)
8. Everything else by priority tier

**Critical + high priority block:** ~20-25 weeks to V1.0.
