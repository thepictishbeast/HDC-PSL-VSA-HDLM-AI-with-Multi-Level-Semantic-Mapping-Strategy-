# Patent Application #2 — Asymptotic Confidence with Multi-Tier Source Weighting

**Title:** System and Method for Bounded AI Confidence with Source-Weighted Asymptotic Scoring

**Filed by:** Paul [Last Name], PlausiDen Technologies
**Filing type:** US Provisional Patent Application
**Status:** DRAFT — ready for attorney review

---

## Abstract

A system and method for artificial intelligence claim confidence that asymptotically approaches but never reaches 1.0, combined with a tier-based ceiling system tied to source categorization. The confidence function is defined as `1 - exp(-evidence_weight)`, mathematically bounded below 1.0 even with unbounded evidence, and further clamped by tier-specific maxima (FormalProof: 0.9999 > PeerReviewed: 0.99 > Standards: 0.99 > Expert: 0.70 > Journalism: 0.60 > Community: 0.50 > Anonymous: 0.20 > Adversarial: 0.05). The system integrates claim corroboration across multiple sources, contradiction detection that reduces confidence bilaterally, and time-based decay of stale claims. The invention addresses the critical flaw in contemporary AI systems of expressing inappropriate high confidence on unverifiable or speculative claims.

---

## Field of the Invention

This invention relates to artificial intelligence systems, specifically to the confidence scoring of AI-generated or AI-ingested claims, and the prevention of inappropriate expression of certainty on claims whose underlying evidence does not warrant such certainty.

---

## Background

### The Overconfidence Problem

Contemporary large language models routinely produce natural-language claims with apparent certainty regardless of the actual epistemic status of the claim. An LLM may state "Paris is the capital of France" (verifiable fact) with the same linguistic confidence as "Bitcoin will be $X in a year" (unknowable prediction). Post-hoc calibration attempts (logit analysis, verbalized confidence) are inconsistent and gameable.

### Why Existing Approaches Fail

1. **Logit-based confidence**: reports how confident the model is *linguistically*, not how well-supported the claim is. High logit confidence does not imply correctness.

2. **Temperature scaling / Platt scaling**: calibration techniques that adjust overall confidence but do not distinguish claim types.

3. **Bayesian neural networks**: express uncertainty but are computationally expensive and don't incorporate source authority.

4. **Human feedback tuning (RLHF)**: teaches models to hedge verbally but does not enforce confidence bounds.

5. **Retrieval-augmented generation (RAG)**: incorporates source data but does not weight sources by category or enforce asymptotic limits.

### Gap Filled by Present Invention

No prior art system combines:
- Mathematically bounded confidence (strict asymptote below 1.0)
- Tier-based source authority weighting with explicit ceilings
- Multi-source corroboration with asymptotic accumulation
- Contradiction-triggered confidence reduction
- Time-based decay for staleness

---

## Summary of the Invention

The invention comprises a computing system and method that processes claims through an epistemic filter with the following components:

### Component A: Source Registry
A registry of sources, each classified into one of 8 categories:

| Category | Base Trust | Confidence Ceiling |
|---|---|---|
| FormalProof (Coq, Lean, machine-verified) | 0.95 | 0.9999 (Proof tier) |
| PeerReviewed (journals, conferences) | 0.85 | 0.99 (Consensus tier) |
| Standards (NIST, IETF, ISO, W3C) | 0.85 | 0.99 (Consensus tier) |
| Expert (named subject-matter authorities) | 0.70 | 0.65 (Plausible tier) |
| Journalism (reputable news organizations) | 0.60 | 0.65 (Plausible tier) |
| Community (crates.io, GitHub, Wikipedia) | 0.50 | 0.35 (Unverified tier) |
| Anonymous (unknown authors) | 0.20 | 0.35 (Unverified tier) |
| Adversarial (known unreliable) | 0.05 | 0.15 (Suspect tier) |

### Component B: Asymptotic Confidence Function
The raw confidence for a claim with accumulated evidence weight `w` is:

```
confidence(w) = min(1 - exp(-w), 0.9999)
```

Properties:
- `confidence(0) = 0`
- `confidence(∞) → 1` but never equals 1
- Hard-clamped at 0.9999 to prevent floating-point rounding from reaching 1.0
- Monotonically increasing in `w`
- Diminishing returns: each additional corroboration adds less

### Component C: Knowledge Tier System
The confidence is further clamped by the tier of the claim's most authoritative source:

```
tier_for_evidence(sources):
    if any source is FormalProof: return Proof
    if ≥3 PeerReviewed AND ≥1 Standards: return Consensus
    if ≥2 reputable sources: return Corroborated
    if ≥1 reputable source: return Plausible
    else: return Unverified

final_confidence = min(
    asymptotic_confidence(sum of trust scores),
    tier.ceiling()
)
```

### Component D: Contradiction Detector
When contradictory claims are detected (e.g., "X is true" vs "X is false"), both claims have confidence reduced by a factor (30%):

```
record_contradiction(claim_a, claim_b):
    claim_a.confidence *= 0.7
    claim_b.confidence *= 0.7
    if contradictions[claim] > 1: tier → Suspect
```

### Component E: Time-Based Decay
Old claims have confidence reduced proportional to age:

```
apply_decay(decay_rate_per_day):
    for claim in claims:
        age_days = (now - claim.last_updated) / 86400
        decay = min(decay_rate * age_days, 0.5)
        claim.confidence *= (1 - decay)
        if confidence < tier.floor(): demote_tier()
```

### Component F: Rejection Gate
Adversarial-tagged sources are rejected unless multi-source corroboration exists:

```
ingest_claim(claim, source):
    if source.category == Adversarial:
        return rejected(reason="Adversarial source requires corroboration")
    ...
```

---

## Detailed Description

### Mathematical Foundation

The asymptotic function `f(w) = 1 - exp(-w)` has the following desirable properties:

1. **Bounded above**: `lim_{w→∞} f(w) = 1` but `f(w) < 1` for all finite `w`.
2. **Derivative**: `f'(w) = exp(-w)`, so confidence grows most when evidence is scarce (first corroborations matter most).
3. **Information-theoretic interpretation**: related to Shannon entropy for binary outcomes.
4. **Calibration**: gives natural meaning to evidence weight — each unit of `w` represents a trust-weighted corroboration.

The clamp at 0.9999 ensures that even under floating-point rounding (where `exp(-100) ≈ 0`), the result does not equal 1.0.

### Tier Ceilings and the "Proof" Boundary

The Proof tier has a ceiling of 0.9999, *not* 1.0, even though formal mathematical proofs are typically regarded as certain. This reflects the invention's epistemic doctrine: no claim in software can be 100% certain because:
- The prover itself may have bugs
- The specification may not capture the intended property
- The hardware may fail
- The input may be malicious

This is a deliberate design choice distinguishing the invention from systems that treat "verified" as 100% certain.

### Source Category Boundaries

Category boundaries are calibrated to reflect empirical reliability data (see experiments in LFI's benchmark harness):
- FormalProof >> PeerReviewed: a machine-checked proof is rarely wrong in its logic (wrong spec ≠ wrong proof); peer review catches most but not all errors
- PeerReviewed > Standards: peer review scrutinizes novel contributions; standards document working consensus
- Expert > Journalism: SME knowledge is deeper; journalism is broader
- Community > Anonymous: community moderation reduces noise
- Anonymous > Adversarial: unknown ≠ known-bad

### Example Evidence Flow

**Scenario**: A user ingests the claim "TLS 1.3 is quantum-resistant."

| Step | Source | Trust | Evidence | Confidence | Tier |
|---|---|---|---|---|---|
| 1 | Wikipedia | 0.5 | 0.5 | 0.39 | Unverified |
| 2 | NIST NVD | 0.85 | 1.35 | 0.74 | Plausible |
| 3 | IETF RFC | 0.85 | 2.20 | 0.89 | Plausible |

Wait — the user made a factual error. TLS 1.3 is NOT quantum-resistant. A contradicting claim arrives:

| Step | Source | Effect |
|---|---|---|
| 4 | NIST post-quantum PDF | Contradicts: TLS 1.3 is vulnerable to Shor's algorithm |
| | | Confidence of original claim × 0.7 = 0.623 |
| 5 | Academic paper | Further contradicts |
| | | Confidence × 0.7 = 0.436, TIER → Suspect |

The system autonomously demotes the claim based on contradictory evidence rather than maintaining the initial confidence.

---

## Claims

**1.** A computer-implemented method for scoring artificial intelligence claim confidence, comprising:
- (a) receiving a claim and an identified source;
- (b) looking up the source category from a registry having at least eight categories: FormalProof, PeerReviewed, Standards, Expert, Journalism, Community, Anonymous, and Adversarial;
- (c) computing evidence weight as a sum of the trust values of all sources supporting the claim;
- (d) computing raw confidence as `1 - exp(-evidence_weight)`, clamped above by 0.9999;
- (e) determining the claim's tier from the distribution of supporting source categories;
- (f) clamping the confidence to the tier's maximum confidence;
- (g) storing the resulting confidence such that it never equals 1.0.

**2.** The method of claim 1, wherein the Proof tier has maximum confidence 0.9999, the Consensus tier has maximum 0.99, and the Suspect tier has maximum 0.15.

**3.** The method of claim 1, further comprising detecting contradictions between claims and reducing both contradicting claims' confidence by a multiplicative factor.

**4.** The method of claim 3, wherein the multiplicative factor is 0.7.

**5.** The method of claim 1, further comprising decaying confidence over time at a configurable rate, with tier demotion when confidence drops below the tier's minimum.

**6.** The method of claim 1, wherein claims from sources categorized as Adversarial are rejected unless corroborated by at least one source of a higher tier.

**7.** The method of claim 1, wherein the tier assignment rule comprises:
- (i) Proof tier if at least one source is FormalProof;
- (ii) Consensus tier if at least 3 sources are PeerReviewed and at least 1 is Standards;
- (iii) Corroborated tier if at least 2 reputable (Expert, PeerReviewed, Standards) sources agree;
- (iv) Plausible tier if at least 1 reputable source supports;
- (v) Unverified tier otherwise.

**8.** A non-transitory computer-readable medium comprising instructions that, when executed by a processor, cause the processor to perform the method of claim 1.

**9.** A system comprising:
- a memory storing a registry of source categorizations;
- a memory storing a collection of claims, each claim associated with a confidence value strictly less than 1.0, a tier, and a set of supporting sources;
- a processor configured to: receive new claims, ingest them with tier-clamped asymptotic confidence, detect contradictions, apply decay, and enforce the bounded-confidence invariant.

**10.** The system of claim 9, further configured to integrate with an inference engine such that AI-generated answers reference the epistemic filter to weight their output confidence.

---

## Distinguishing Features vs. Prior Art

| Prior Art | Our Difference |
|---|---|
| Logit-based confidence | We separate claim confidence from linguistic confidence. |
| Temperature scaling | We bound confidence below 1.0 as invariant, not as calibration. |
| Bayesian NN | We operate at claim level, not weight level; integrate sources. |
| RAG | We don't just retrieve — we score sources in a tier hierarchy. |
| RLHF hedging | We enforce limits mathematically, not behaviorally. |
| PageRank / TrustRank | We apply to claim confidence in AI inference, not web ranking. |

---

## Commercial Applications

- **Medical AI**: never claim 100% diagnosis certainty; reflect source evidence quality.
- **Finance/Trading**: confidence bounds on predictions protect against overconfident bets.
- **News verification**: rank claims by source tier for truthiness scoring.
- **Scientific literature AI**: respect the hierarchy of evidence (meta-analyses > RCTs > observational).
- **Legal discovery**: rank evidence by admissibility / reliability tiers.
- **AI content moderation**: adversarial source rejection prevents coordinated misinformation.
- **Regulated AI (EU AI Act)**: bounded confidence is a regulatory requirement for high-risk AI.

---

## Diagrams (to be finalized by patent attorney)

- `01-asymptote-curve.svg`: `1 - exp(-w)` function showing approach to 1.0
- `02-tier-hierarchy.svg`: 8-category source tree with ceilings
- `03-evidence-flow.svg`: claim lifecycle through the filter

---

## References to Embodying Source Code

- `src/intelligence/epistemic_filter.rs` (entire file): complete implementation
- Key functions:
  - `asymptotic_confidence()`: lines 310-317
  - `tier_for_evidence()`: lines 370-400
  - `ingest_claim()`: lines 215-290
  - `record_contradiction()`: lines 293-310
  - `apply_decay()`: lines 335-350

Test suite demonstrating enforcement:
- `test_asymptotic_never_reaches_one`: verifies mathematical bound
- `test_tier_max_confidence_caps`: verifies tier ceilings
- `test_adversarial_source_rejected`: verifies rejection gate
- `test_corroboration_promotes_tier`: verifies multi-source promotion
- `test_contradiction_detection`: verifies bilateral confidence reduction
- `test_confidence_decay_over_time`: verifies temporal decay

---

## Prosecution Notes

**Strongest claims for novelty:** 1 (the full combined method), 9 (system), 10 (AI inference integration).

**Anticipated objections:**
1. "Just a calibration technique." → Response: calibration adjusts; we enforce an asymptotic bound. No calibration technique guarantees < 1.0 as an invariant.
2. "Tier systems exist in other domains." → Response: the specific 8-category hierarchy with these exact ceilings, integrated with asymptotic confidence and contradiction detection, is not in prior art.
3. "Obvious to combine X and Y." → Response: the specific combination is not present in any single prior art system; the non-obviousness lies in combining all 5 components (asymptotic + tier + contradiction + decay + rejection gate).

**Narrower fallback claims:** 2 (specific ceiling values) and 7 (specific tier rules) add further detail if the broad claims are rejected.

---

*Draft completed: 2026-04-14*
*Ready for: patent attorney review, USPTO filing as provisional*
