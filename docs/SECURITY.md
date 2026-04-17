# Security Posture

## Guiding Principles

1. **Zero Trust** — No data is trusted by default, regardless of source
2. **Assume Breach** — All external systems (GPU clusters, APIs, files) are treated as potentially compromised
3. **Memory Safety** — Absolute, enforced by the Rust compiler
4. **Fail Loud** — Errors propagate, never silenced. Telemetry captures everything

---

## Rust Memory Safety

### `#![forbid(unsafe_code)]`

The crate root (`lib.rs`) contains:

```rust
#![forbid(unsafe_code)]
```

This is the strictest possible setting. It:
- **Prevents** any `unsafe` block in the entire crate
- **Cannot be overridden** by `#[allow(unsafe_code)]` on individual items (unlike `#[deny]`)
- **Applies transitively** to all modules within the crate

Note: Dependencies (bitvec, rand, etc.) may use `unsafe` internally. This is expected and acceptable — `forbid(unsafe_code)` applies only to our own code.

### No Implicit Failure

**Every** operation that can fail returns `Result<T, E>` or `Option<T>`:

```rust
// YES — explicit error handling
pub fn bind(&self, other: &BipolarVector) -> Result<BipolarVector, HdcError> { ... }

// NO — these are forbidden
pub fn bind(&self, other: &BipolarVector) -> BipolarVector { ... }  // could panic
```

### Forbidden Patterns

The following are **strictly forbidden** in all non-test code:

| Pattern | Why It's Forbidden | Alternative |
|---------|-------------------|-------------|
| `.unwrap()` | Panics on None/Err | Use `?` or `match` |
| `.expect("msg")` | Panics on None/Err with message | Use `?` or `match` |
| `panic!()` | Terminates the process | Return `Err(...)` |
| `unreachable!()` | Panics if reached | Return `Err(...)` with detail |
| `todo!()` | Panics with "not yet implemented" | Return `Err(...)` explaining what's missing |
| `assert!()` in non-test code | Panics on failure | Return `Err(...)` |
| `unsafe { }` | Blocked by `forbid(unsafe_code)` | Find a safe alternative |

In **test code**, we use `-> Result<(), ErrorType>` with `?` operator instead of `.unwrap()`.

---

## CARTA Trust Model

See [PSL_SUPERVISOR.md](PSL_SUPERVISOR.md) for the full trust level documentation.

Key security properties:
- External data enters at `TrustLevel::Untrusted`
- Must pass all applicable PSL axioms to reach `Verified`
- Only `Verified` or `Sovereign` data can influence computation
- Trust assessment includes confidence score and audit trail

---

## Hostile Data Handling

### Remote GPU Returns

When computation is dispatched to remote GPU clusters:

1. Results arrive as `AuditTarget::RawBytes` with source label
2. PSL Supervisor runs `DataIntegrityAxiom` (non-empty, within size bounds)
3. Future axioms will check: cryptographic signatures, statistical anomalies, dimension validity
4. Only after reaching `TrustLevel::Verified` can results be decoded into `BipolarVector`s

### File Ingestion

Files entering the system are wrapped as `AuditTarget::RawBytes` and audited before processing. The `DataIntegrityAxiom` catches empty files and oversized payloads.

### API Responses

External API data arrives as `AuditTarget::Payload` with key-value fields. Future axioms will validate schema conformance and content integrity.

---

## Error Isolation

Each subsystem has its own error type:
- `HdcError` — Cannot leak PSL or HDLM concerns
- `PslError` — Cannot leak HDC or HDLM concerns
- `HdlmError` — Cannot leak HDC or PSL concerns

This prevents error type confusion and ensures each subsystem handles only its own failure modes.

---

## Supply Chain Security

### Dependencies (Cargo.toml)

| Crate | Version | Purpose | Audit Status |
|-------|---------|---------|-------------|
| `bitvec` | 1.0.1 | Bit vector storage for hypervectors | Widely used, no known vulns |
| `rand` | 0.8.5 | Random vector initialization | Rust ecosystem standard |
| `serde` | 1.x | Serialization framework | Rust ecosystem standard |
| `serde_json` | 1.x | JSON serialization for IPC bus | Rust ecosystem standard |

### Dependabot

`.github/dependabot.yml` is configured to automatically check for dependency updates.

---

## Telemetry as Security Control

Delta telemetry (`debuglog!`) serves a security function:
- Every computation is logged with file:line location
- Trust level changes are logged
- Axiom verdicts (pass/fail/truth_value) are logged
- Hostile data detection events are logged

The `LFI.log` file provides a forensic audit trail. In production, telemetry should be redirected to a secure, append-only log store rather than stripped entirely.

---

## Threat Model

| Threat | Mitigation |
|--------|-----------|
| Buffer overflow | Rust memory safety, no unsafe code |
| Integer overflow | Rust default panic-on-overflow in debug, wrapping in release |
| Hostile GPU data | PSL audit with DataIntegrityAxiom, TrustLevel gating |
| Dimension mismatch | `check_dim()` in every vector operation |
| Empty input | Explicit error returns (EmptyBundle, EmptyAst, etc.) |
| Uninitialized memory | Rust prevents this at compile time |
| Race conditions | Single-threaded design, file-based IPC with inotifywait |
| Dependency vulns | Dependabot monitoring, minimal dependency surface |
