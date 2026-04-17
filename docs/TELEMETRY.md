# Delta Telemetry Map

**Workflow Delta Rule:** Assume all code is fundamentally broken until the log proves success.

This document maps every `debuglog!` and `debuglog_val!` call in the codebase. It is updated after every phase commit.

---

## Macro API

### `debuglog!`

```rust
debuglog!("format string: {}", value);
```

Expands to:
```
[DEBUGLOG][src/hdc/vector.rs:42] - format string: 10000
```

Defined in `src/telemetry.rs`. Uses `file!()` and `line!()` for automatic source location.

### `debuglog_val!`

```rust
debuglog_val!("label", &my_variable);
```

Expands to:
```
[DEBUGLOG][src/psl/axiom.rs:88] - label = BipolarVector { ... }
```

Uses `{:#?}` (pretty-printed Debug) for the value.

---

## Production Stripping

All telemetry is structurally isolated inside the two macros. To strip for production:

**Option A — Comment out macro bodies:**
```rust
#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        // PRODUCTION: telemetry stripped
    };
}
```

**Option B — Feature gate:**
```rust
#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        #[cfg(feature = "telemetry")]
        println!("[DEBUGLOG][{}:{}] - {}", file!(), line!(), format_args!($($arg)*));
    };
}
```

**Option C — Build with release profile** (does not strip but optimizer may remove unused format strings).

---

## Telemetry Location Map

### `src/telemetry.rs`

| Line | Macro | Context |
|------|-------|---------|
| (definitions only) | `debuglog!`, `debuglog_val!` | Macro definitions, no calls here |

### `src/hdc/vector.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `new_random()` | 2 | Success: dimension count. Failure: actual vs expected dim |
| `from_bitvec()` | 1 | Input dimension count |
| `zeros()` | 1 | Dimension of created zero vector |
| `ones()` | 1 | Dimension of created ones vector |
| `bind()` | 1 | Result dimension after XOR |
| `bundle()` | 2-3 | Empty bundle error. Dimension mismatch at index. Merge count + result dim |
| `permute()` | 2 | Zero-shift identity shortcut. Effective shift + result dim |
| `similarity()` | 1 | Agreement count, disagreement count, cosine value |
| `hamming_distance()` | 1 | Distance value |
| `check_dim()` | 0 | (returns error, caller logs) |

### `src/hdc/compute.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `LocalBackend::bind()` | 1 | Dispatch notification |
| `LocalBackend::bundle()` | 1 | Dispatch notification + vector count |
| `LocalBackend::permute()` | 1 | Dispatch notification + shift value |
| `LocalBackend::similarity()` | 1 | Dispatch notification |

### `src/psl/axiom.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `AxiomVerdict::pass()` | 1 | Axiom ID + truth value |
| `AxiomVerdict::fail()` | 1 | Axiom ID + truth value |
| `DimensionalityAxiom::verify()` | 1 | Verification start |
| `DataIntegrityAxiom::verify()` | 1 | Max bytes limit |

### `src/psl/trust.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `TrustLevel::permits_execution()` | 1 | Current trust level |
| `TrustLevel::requires_audit()` | 1 | Current trust level |
| `TrustLevel::score()` | 1 | Level -> score mapping |
| `TrustAssessment::new()` | 1 | Level, confidence, checked/passed counts |
| `TrustAssessment::pass_ratio()` | 1 | Ratio value or "no axioms" |

### `src/psl/supervisor.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `PslSupervisor::new()` | 1 | Default threshold |
| `PslSupervisor::with_threshold()` | 1 | Custom threshold value |
| `PslSupervisor::register_axiom()` | 1 | Axiom ID being registered |
| `PslSupervisor::audit()` | 4+ | Empty axiom set. Per-axiom verdict (id, passed, tv). Skipped axioms. Summary (total, applicable, passed, skipped) |
| `PslSupervisor::compute_trust_level()` | 1 | Pass ratio, avg truth, threshold |

### `src/hdlm/ast.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `AstNode::new()` | 1 | Node ID + kind |
| `AstNode::with_fingerprint()` | 1 | Node ID + kind (with HV) |
| `Ast::new()` | 1 | Empty tree creation |
| `Ast::add_node()` | 1-2 | Node ID. Root auto-set notification |
| `Ast::add_node_with_hv()` | 1 | Node ID |
| `Ast::add_child()` | 1-2 | Parent->child link. Invalid ID error |
| `Ast::dfs()` | 1 | Visited node count |
| `Ast::bfs()` | 1 | Visited node count |
| `Ast::leaf_count()` | 1 | Count |

### `src/hdlm/tier1_forensic.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `ArithmeticGenerator::parse_prefix()` | 1 | Current token + position |
| `ArithmeticGenerator::generate_from_tokens()` | 1 | Node count + tokens consumed |
| `ArithmeticGenerator::generate_from_vector()` | 1 | NOT IMPLEMENTED notice |

### `src/hdlm/tier2_decorative.rs`

| Function | debuglog Calls | What They Log |
|----------|---------------|---------------|
| `InfixRenderer::render_node()` | 1 | Node ID + kind |
| `InfixRenderer::render()` | 1 | Final rendered string |
| `SExprRenderer::render_node()` | 1 | Node ID + kind |
| `SExprRenderer::render()` | 1 | Final rendered string |

---

## Total debuglog Count

| Subsystem | File | Call Count |
|-----------|------|-----------|
| HDC Core | `vector.rs` | 12 |
| HDC Core | `compute.rs` | 4 |
| PSL | `axiom.rs` | 4 |
| PSL | `trust.rs` | 5 |
| PSL | `supervisor.rs` | 7+ (variable per axiom count) |
| HDLM | `ast.rs` | 10 |
| HDLM | `tier1_forensic.rs` | 3 |
| HDLM | `tier2_decorative.rs` | 4 |
| **Total** | | **49+ calls** |

---

## Adding New Telemetry

When adding any new function or branch:

1. Add `debuglog!` at entry with key parameters
2. Add `debuglog!` at every error/early-return path
3. Add `debuglog!` at exit with result summary
4. For complex values, use `debuglog_val!` with a descriptive label
5. Update this document with the new locations
