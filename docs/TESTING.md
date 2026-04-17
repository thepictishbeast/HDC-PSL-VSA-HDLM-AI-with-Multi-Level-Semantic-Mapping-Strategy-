# Testing Strategy

## Philosophy

**Test-Driven Mandate (Section 4):** Alpha must write exhaustive unit tests and mathematically prove the code operates under edge cases before integrating it into the core.

**Assume Broken:** Every function is assumed to be incorrect until a test proves otherwise. Tests are not just verification — they are mathematical proofs of correctness.

---

## How to Run

```bash
cd lfi_vsa_core

# Run all tests
cargo test

# Run with output (see debuglog! output)
cargo test -- --nocapture

# Run a specific test
cargo test test_bind_commutativity

# Run tests for a specific module
cargo test hdc::vector

# Run only integration tests
cargo test --test forensic_audit
```

---

## Test Suite Summary

**Total: 89 tests, 0 failures**

| Subsystem | Module | Test Count | What They Prove |
|-----------|--------|-----------|-----------------|
| HDC Core | `vector.rs` | 43 | Algebraic properties of all 3 operations + similarity metrics |
| HDC Core | `compute.rs` | 4 | Backend dispatch parity with direct calls |
| PSL | `supervisor.rs` | 13 | Audit pipeline, trust levels, hostile data, thresholds |
| HDLM | `ast.rs` | 11 | AST construction, traversal, node management |
| HDLM | `tier1_forensic.rs` | 9 | Token parsing, error cases, all operators |
| HDLM | `tier2_decorative.rs` | 8 | Rendering, mutation invariant, empty tree handling |
| Integration | `forensic_audit.rs` | 1 | Statistical Hamming weight distribution |

---

## Test Categories

### I. Initialization Tests (8 tests)

Prove that vectors are correctly constructed:
- `test_init_random_exact_dimension` — Dimension is exactly 10,000
- `test_init_random_nontrivial_distribution` — Has both 0s and 1s (not degenerate)
- `test_init_random_uniqueness` — Two random vectors differ (collision impossible)
- `test_init_zeros` / `test_init_ones` — Deterministic constructors
- `test_from_bitvec_valid` / `_wrong_dimension` / `_empty` — Input validation

### II. Binding Tests (8 tests)

Prove XOR algebraic properties:
- Commutativity, associativity, identity element, self-inverse
- Recovery (extract B from A and A XOR B)
- Quasi-orthogonality to inputs
- Dimension preservation

### III. Bundling Tests (8 tests)

Prove majority-vote superposition:
- Single-vector identity, commutativity, dimension preservation
- Identical-vector idempotency
- Similarity preservation to all inputs
- Majority vote dominance (3xA vs 2xB)
- Deterministic tie-breaking
- Empty input error

### IV. Permutation Tests (9 tests)

Prove cyclic group properties:
- Zero and full-rotation identity
- Invertibility and composition
- Quasi-orthogonality after shift
- Hamming weight preservation
- Nontriviality (shift changes the vector)

### V. Similarity Tests (7 tests)

Prove metric properties:
- Self-similarity = 1.0
- Complement similarity = -1.0
- Symmetry
- Random vectors near 0.0
- Hamming distance: self=0, complements=DIM, symmetry

### VI. Cross-Operation Tests (3 tests)

Prove composite HDC patterns work:
- Bind+Bundle key-value recovery (associative memory)
- Permute sequence encoding (positional orthogonality)
- Bind+Permute combined encoding

### VII. PSL Supervisor Tests (13 tests)

Prove audit pipeline correctness:
- Empty supervisor returns error
- Registration increments count
- Valid vector passes dimensionality check
- Inapplicable axioms are skipped (not counted as failures)
- Valid raw bytes pass integrity check
- Oversized raw bytes are rejected
- Empty raw bytes are rejected
- Multi-axiom audit correctly filters applicable axioms
- Custom threshold changes behavior
- Trust level ordering (Untrusted < ... < Sovereign)
- Execution gate (only Verified/Sovereign permit execution)
- Trust level scores are correct
- Zero-axiom pass ratio is 0.0

### VIII. AST Tests (11 tests)

Prove tree operations:
- Empty tree state
- Root node auto-detection
- Child attachment
- Self-reference rejection
- Invalid ID rejection
- DFS traversal order
- BFS traversal order
- Empty tree traversal error
- Leaf counting
- Node with HV fingerprint
- Leaf/parent distinction

### IX. Tier 1 Tests (9 tests)

Prove forensic generation:
- Single literal parsing
- Simple binary operation
- Nested expressions
- DFS of generated AST
- Empty input error
- Invalid literal error
- Unconsumed tokens error
- Truncated expression error
- All four arithmetic operators

### X. Tier 2 Tests (8 tests)

Prove decorative expansion:
- Infix: literal, simple, nested
- S-expr: simple, nested
- **Mutation invariant** (node count unchanged after rendering)
- Empty AST error
- Both renderers produce consistent output

### XI. Integration Test (1 test)

- `forensic_hamming_weight_audit` — 100 random vectors, average Hamming weight within 2% of 5000. Statistical proof of unbiased initialization.

---

## Test Conventions

1. **No `.unwrap()` or `.expect()`** — Tests use `-> Result<(), ErrorType>` with `?`
2. **Descriptive names** — `test_bind_commutativity`, not `test_bind_1`
3. **One property per test** — Each test proves exactly one mathematical property
4. **Edge cases** — Empty inputs, wrong dimensions, self-references, ties
5. **Statistical tolerances** — Random tests use conservative thresholds (40+ sigma)
6. **Regression safety** — Phase 2 tests don't break Phase 1 tests
