# LFI Training Methodology — How A Neurosymbolic System Actually Learns

**Framing discipline:** LFI does not train in the LLM sense. There is no gradient descent on a next-token objective, no loss function over billions of floats, no dataset of input-output pairs for supervised fine-tuning, no preference pair collection for alignment. Instead, LFI learns through five (six) mechanisms that compose: fact ingestion, prototype consolidation, axiom refinement, metacognitive calibration, spaced repetition, and compositional skill acquisition.

Every section here is what Claude Code should actually implement. Anything that sounds like LLM training (SFT, RLHF, DPO, ORPO, GRPO, PPO, reward modeling, preference pairs) is outside scope and should not be built.

## Mechanism 1 — Fact Ingestion

Primary learning operation.

```
for each fact (subject, predicate, object, tier_hint, provenance, temporal_class):
  1. PSL-validate against axiom library (reject hard violations)
  2. Assign quality tier from source authority × extraction confidence × PSL soft-truth
  3. Encode fact_hv = bind(R_subj, E_s) + bind(R_pred, E_p) + bind(R_obj, E_o)
  4. Store tuple in SQLite with indexes on domain/quality/temporal/hash
  5. Trigger prototype update for each concept involved
  6. Emit TracedDerivation audit entry with source-byte-range hash
```

**Properties:** atomic + reversible (tier-demote or retract without retraining). Compositional (new facts participate in analogy/causal via shared algebra immediately). Provenance-preserving (unreliable source → tier-downgrade all facts from it in one op).

**Implementation:** lfi-storage crate handles SQLite. lfi_vsa_core/src/hdc handles encoding. Gap: batch-only assumption; mobile needs **incremental single-fact ingestion** without full prototype rebuilds.

## Mechanism 2 — Prototype Consolidation

Tier-weighted two-stage voted bundle (replaces flat bundling):

```
P = sign(Σ_t w_t × sign(Σ_{i ∈ Tier_t} H_i))
```

Inner sign() stops tier-7 volume from drowning tier-1 authority. Outer weighted vote preserves authority gradient. Geometric weighting → single tier-1 fact ≈ 64× single tier-7 fact.

**Robustness:**
- α=15% trimmed-mean per dimension (discards top/bottom 15% before sign — neutralizes coordinated poisoning)
- 3σ outlier rejection via leave-one-out cosine against random-pair distribution N(0, 1/D) at D=10,000
- Tier-tag provenance binding: `H_i' = H_i + R_tier ⊗ L_tier(t) + R_source ⊗ H_source` → unbind R_tier at query to recover dominant tier

**Update triggers:** new fact with concept as subj/obj, tier-change or retraction, scheduled consolidation pass. Mobile uses running-sum approximation with periodic full-bundle reconciliation.

**Properties:** no gradient descent — bitwise majority vote, deterministic, constant-time per fact. Adversarially hardened (defeats HyperAttack bit-flip). Inspectable (prototype decomposes to contributing facts + weights).

## Mechanism 3 — Axiom Refinement

PSL rules aren't static. Weights update online.

**Weight update via ADMM feedback:** rules used in subsequently-verified reasoning chains get `weight += η × verification_confidence` (η ~ 0.01-0.05). Verification source: formal verification > user feedback > cross-source > Active Inference free-energy reduction.

**Axiom discovery via consistency mining:** pattern-mine fact store for `(subject_type, predicate, object_type)` patterns that recur and rarely violate existing axioms. Propose new rule, initial weight proportional to observation frequency. Enter probationary tier. Reasoning chains using it provide weight-adjustment feedback.

*Example:* after biomedical corpus, `(Drug X, treats, Disease Y) → (Drug X, indicated_for, Disease Y)` almost always holds. Propose `treats(X,Y) → indicated_for(X,Y)` probationary.

**Properties:** online (not batch). Traceable (weight-change log). Reversible (bad proposals → weight → 0). Domain-specific (medical kernel probe accumulates biomedical regularities, legal kernel probe accumulates legal, etc.).

**Effort:** 4-6 weeks. Most novel mechanism. Needs feedback channels from formal verification/user/AIF outcomes, ADMM with per-rule weight updates, proposal generator, safety check preventing destabilization of proven claims.

## Mechanism 4 — Metacognitive Calibration

Per-query outcome tracking:
- query hypervector, domain, which modules ran, facts retrieved, axioms applied, final confidence, outcome (correct/incorrect/abstained)

Profiler maintains per-domain per-question-type prototype hypervectors + observed accuracy. New query → nearest prototype → historical accuracy → predicted confidence.

**Platt scaling:** fit logistic regression mapping raw similarity scores to calibrated probabilities via outcome history. Re-fit periodically.

**Abstention threshold** τ optimized per-domain:
```
argmax_τ   (abstain_if_wrong_benefit) - α × abstain_if_right_cost - β × answer_if_wrong_cost
```
with α >> β (wrong confident answer is more harmful than missed correct answer, especially medical/legal/financial).

**Implementation:** metacognitive.rs substrate exists. Add per-domain outcome log + Platt fitting. Low-effort, 1-2 weeks.

## Mechanism 5 — Spaced Repetition (FSRS over facts)

FSRS v6 (trained on 350M Anki reviews): per-card Difficulty/Stability/Retrievability jointly, 17-21 params. Forgetting curve `R(t, S) = (1 + F×t/S)^C` with R=0.9 at t=S.

Applied to LFI:
- **Card** = fact tuple (or cluster)
- **Review** = consistency check against current fact store + newly-available authoritative sources
- **Grade** = confirmed→easy, contradicted→again, consistent+newcontext→good
- **Schedule** via FSRS stability/retrievability

Per fact type:
- **Temporal facts** — auto-decay per temporal-class half-life; re-verify from authoritative source at each review
- **Scientific** — re-check latest literature; contradictions flag tier re-assessment
- **Mathematical/logical** — no decay, max-stability scheduling
- **Personal** — review against current state

**Properties:** bounded review load (mobile ~50-200 facts/day). Automatic staleness detection. Reversible.

**Effort:** 2-3 weeks. `fsrs-rs` crate production-ready.

## Mechanism 6 — Compositional Skill Acquisition

When a reasoning pattern recurs, compile it into a reusable inference procedure.

**Stitch library learning** (Bowers et al. POPL 2023, github.com/mlb2251/stitch, Rust-native): anti-unification across reasoning traces. Given many derivations, extracts reusable λ-abstractions via MDL compression.

For LFI: feed reasoning_provenance traces into Stitch. It extracts templates like:
```
λ(X, Y, Z). if is_a(X, Y) ∧ has_property(Y, Z) ⊢ has_property(X, Z)
```
Each template becomes a new composable operation in cognition. No retraining — just structure added to the operator library.

**Wake-sleep consolidation:** idle-time offline consolidation:
1. Review high-salience reasoning traces from the day
2. Run Stitch for library learning
3. Update prototypes with facts from high-confidence traces not yet tier-promoted
4. Retire low-tier facts contradicted or unreferenced
5. FSRS schedules tomorrow's reviews

Analogous to CLS theory: hippocampal traces → neocortical structure during sleep.

**Properties:** data-efficient (templates from dozens of traces generalize to thousands of queries). Interpretable (explicit λ-abstractions with documented pre/post). Local (on-device, no telemetry).

**Effort:** 3-4 weeks. Stitch is mature.

## NOT in scope (wrong category)

Do not build:

- Supervised Fine-Tuning (SFT) — no model to fine-tune
- Instruction tuning — no model to instruct
- RLHF / DPO / ORPO / KTO / SimPO / GRPO / PPO — no parameter vector to optimize
- Reward modeling — no policy to reward
- Magpie / self-instruct / WizardLM-style synthesis — no instruction-response dataset needed
- Chain-of-thought prompting — no prompts; cognition layer reasons natively
- In-context learning — no context window; working memory is semantic Global Workspace
- Few-shot exemplars — analogical reasoning is native, not emergent from exemplar exposure
- PEFT (LoRA, DoRA, O-LoRA) — no parameters to tune
- Catastrophic forgetting mitigation — ingestion is atomic + reversible
- Distillation — no teacher-student parameter transfer
- Quantization — facts already bipolar; nothing to quantize below 1-bit

**If any of these appear in downstream plans, the plan has drifted back into LLM framing and needs rewriting in LFI-native terms.**

## Sprint plan

- **Sprint A** (2 weeks): Mechanism 1 hardening — incremental single-fact ingestion, remove batch-only assumptions in lfi-storage.
- **Sprint B** (3 weeks): Mechanism 2 — tier-weighted two-stage voted bundle with trimmed-mean + outlier rejection. Replace existing bundling. Add HyperAttack defense.
- **Sprint C** (4-6 weeks): Mechanism 3 — axiom refinement, online weight updates, pattern-mining proposal generator, safety-check. Most novel work.
- **Sprint D** (1-2 weeks): Mechanism 4 — per-domain outcome logging + Platt calibrator.
- **Sprint E** (2-3 weeks): Mechanism 5 — FSRS over facts via fsrs-rs.
- **Sprint F** (3-4 weeks): Mechanism 6 — Stitch + wake-sleep consolidation, wired to reasoning_provenance.rs.

**Total:** 15-20 weeks for all six at production quality. Parallelizable where dependencies allow.

After these sprints, LFI has complete learning apparatus that is qualitatively different from — and in many respects superior to — LLM training. Learning is online, inspectable, per-fact traceable, adversarially hardened, zero cloud dependency.
