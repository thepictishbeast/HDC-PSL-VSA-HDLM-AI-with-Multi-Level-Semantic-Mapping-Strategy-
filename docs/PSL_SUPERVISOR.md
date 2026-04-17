# PSL Supervisor — The Hostile Witness

## Overview

The Probabilistic Soft Logic (PSL) Supervisor is LFI's verification layer. It treats every computation result as potentially hostile and requires mathematical proof of correctness before allowing integration into downstream processing.

**Philosophy:** Zero Trust. Assume Breach. Every datum is guilty until proven innocent.

---

## CARTA Trust Model

CARTA (Continuous Adaptive Risk and Trust Assessment) defines five discrete trust levels:

```
Untrusted (0.0) -> Suspicious (0.25) -> Provisional (0.50) -> Verified (0.75) -> Sovereign (1.0)
```

| Level | Score | Permits Execution? | Requires Audit? | Typical Source |
|-------|-------|-------------------|-----------------|----------------|
| `Untrusted` | 0.0 | NO | YES | External GPU return, network data |
| `Suspicious` | 0.25 | NO | YES | Partially verified, some axioms failed |
| `Provisional` | 0.50 | NO | YES | Most axioms passed, pending deep verify |
| `Verified` | 0.75 | YES | NO | All applicable axioms passed |
| `Sovereign` | 1.0 | YES | NO | Local compute chain, provably correct |

**Only `Verified` and `Sovereign` data can be used for downstream computation.**

---

## The Axiom Trait

All verification rules implement the `Axiom` trait:

```rust
pub trait Axiom {
    fn id(&self) -> &str;                    // e.g., "Axiom:Dimensionality_Constraint"
    fn description(&self) -> &str;           // Human-readable
    fn verify(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError>;
}
```

### AuditTarget

What can be audited:

| Variant | Fields | Use Case |
|---------|--------|----------|
| `Vector(BipolarVector)` | The vector itself | HDC output verification |
| `RawBytes { source, data }` | Source label + byte payload | GPU returns, file ingestion |
| `Scalar { label, value }` | Named f64 | Numeric computation results |
| `Payload { source, fields }` | Source + key-value pairs | API responses |

### AxiomVerdict

Each axiom check produces a verdict:

| Field | Type | Meaning |
|-------|------|---------|
| `axiom_id` | String | Which axiom was checked |
| `passed` | bool | Hard pass/fail |
| `truth_value` | f64 [0.0, 1.0] | Soft PSL truth (1.0 = full satisfaction) |
| `detail` | String | Human-readable explanation |

---

## Built-in Structural Axioms

These verify framework invariants (safe for Alpha to define):

### DimensionalityAxiom

- **Checks:** Vector target has exactly 10,000 dimensions
- **Passes:** dim == HD_DIMENSIONS
- **Fails:** dim != HD_DIMENSIONS
- **Applies to:** `AuditTarget::Vector` only

### DataIntegrityAxiom

- **Checks:** Raw bytes from external source are non-empty and within size limit
- **Passes:** 0 < data.len() <= max_bytes
- **Fails:** Empty payload OR exceeds max_bytes
- **Applies to:** `AuditTarget::RawBytes` only
- **Configurable:** `max_bytes` parameter

---

## The Audit Pipeline

When `PslSupervisor::audit(target)` is called:

```
1. Check axiom set is non-empty (else Err(EmptyAxiomSet))
        |
2. For each registered axiom:
   |
   +-- verify(target) returns Ok(verdict) --> collect verdict
   |
   +-- verify(target) returns Err(InvalidAuditTarget) --> SKIP (wrong type)
   |
   +-- verify(target) returns Err(other) --> PROPAGATE immediately
        |
3. If no axioms were applicable (all skipped):
   |
   +-- Return TrustAssessment { level: Suspicious }
        |
4. Compute metrics:
   |   pass_ratio = passed_count / checked_count
   |   avg_truth  = sum(truth_values) / checked_count
        |
5. Derive TrustLevel:
   |
   +-- pass_ratio >= threshold AND avg_truth >= 0.75 --> Verified
   +-- pass_ratio >= 0.75 AND avg_truth >= 0.50 ------> Provisional
   +-- pass_ratio >= 0.25 -----------------------------> Suspicious
   +-- else --------------------------------------------> Untrusted
        |
6. Return TrustAssessment { level, confidence, rationale, checked, passed }
```

---

## Hostile GPU Data Flow

When a `RemoteGpuBackend` (future Phase) receives computation results:

```
Remote GPU --> RawBytes --> PSL Supervisor --> TrustAssessment
                                                    |
                              Verified? --YES--> Decode to BipolarVector
                                    |
                                   NO --> Reject, log to LFI.log, alert
```

The `DataIntegrityAxiom` catches:
- Empty payloads (GPU returned nothing)
- Oversized payloads (potential buffer overflow / data injection)

Future domain axioms (defined by Beta) will catch:
- Invalid vector dimensions after decoding
- Statistical anomalies in returned vectors
- Cryptographic signature verification failures

---

## Extending with Custom Axioms

Beta (Gemini) defines domain-specific axioms by implementing the `Axiom` trait:

```rust
pub struct MyCustomAxiom { /* config */ }

impl Axiom for MyCustomAxiom {
    fn id(&self) -> &str { "Axiom:My_Custom_Check" }
    fn description(&self) -> &str { "Verifies my custom property" }
    fn verify(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        // Verification logic here
    }
}
```

Then register with the supervisor:
```rust
supervisor.register_axiom(Box::new(MyCustomAxiom { /* config */ }));
```
