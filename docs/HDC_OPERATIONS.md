# HDC Operations — Mathematical Foundations

## Bipolar Encoding

LFI uses **bipolar hypervectors** in the space `{-1, +1}^10000`.

For storage efficiency, we encode these as bits:
- **Bit 0** = bipolar value **-1**
- **Bit 1** = bipolar value **+1**

This encoding is stored in a `BitVec<u8, Lsb0>` from the `bitvec` crate, giving us 10,000 bits = **1,250 bytes** per vector.

### Why 10,000 Dimensions?

In high-dimensional spaces, random vectors concentrate near orthogonal. For dimension `D`, the expected cosine similarity between two random bipolar vectors is 0, with standard deviation `1/sqrt(D)`.

At D=10,000: `sigma = 1/sqrt(10000) = 0.01`

This means:
- Two random vectors have `|similarity| < 0.03` with 99.7% probability (3-sigma)
- We can store ~1000 items in an associative memory before interference degrades recall
- The "capacity" of the space scales with `D / log(D)`

---

## The Three Fundamental Operations

### 1. Binding (XOR)

**What it does:** Creates a new vector representing the *association* between two vectors. Think of it as pairing a key with a value.

**Implementation:**
```rust
let result = a.data.clone() ^ &b.data;  // Bitwise XOR
```

**In bipolar terms:** XOR on bits implements element-wise multiplication in bipolar space:
```
Bit a=0, Bit b=0  ->  XOR=0  ->  (-1)*(-1) = +1 ... but bit 0 = -1 ???
```

**Important note on the encoding:** The XOR result has a specific algebraic meaning in the binary encoding that differs from naive bipolar multiplication. The key property that matters is:

- `A XOR A = all-zeros` (the identity element, since `X XOR all-zeros = X`)
- `A XOR B XOR A = B` (self-inverse: applying A twice cancels out)
- The result is quasi-orthogonal to both inputs

These properties are what make binding useful for associative memory, regardless of the encoding interpretation.

**Algebraic properties (all verified by tests):**

| Property | Statement | Test |
|----------|-----------|------|
| Commutativity | `A XOR B = B XOR A` | `test_bind_commutativity` |
| Associativity | `(A XOR B) XOR C = A XOR (B XOR C)` | `test_bind_associativity` |
| Identity | `zeros XOR A = A` | `test_bind_with_identity` |
| Self-inverse | `A XOR A = zeros` | `test_bind_self_produces_identity` |
| Cancellation | `A XOR A XOR B = B` | `test_bind_self_inverse` |
| Recovery | `A XOR (A XOR B) = B` | `test_bind_recovery` |
| Quasi-orthogonality | `|sim(A XOR B, A)| < 0.1` | `test_bind_quasi_orthogonal_to_inputs` |
| Dimension preservation | `dim(A XOR B) = 10000` | `test_bind_dimension_preserved` |

### 2. Bundling (Sum + Clip)

**What it does:** Creates a new vector representing the *superposition* of multiple vectors. Think of it as putting multiple items into a set.

**Implementation:**
```rust
// Phase 1: Accumulate bipolar sums in i32 scratch space
let mut sums = vec![0i32; HD_DIMENSIONS];
for v in vectors {
    for (i, bit) in v.data.iter().enumerate() {
        sums[i] += if *bit { 1 } else { -1 };
    }
}

// Phase 2: Clip to bipolar (majority vote)
for s in &sums {
    result.push(*s > 0);  // strictly positive -> +1 (bit=1)
}
```

**Tie-breaking:** When the sum is exactly 0 (equal votes for +1 and -1), the result is **-1 (bit=0)**. This is a deterministic convention.

**Why i32 scratch space?** We need to accumulate sums that can exceed the range of i8 when bundling many vectors. i32 supports bundling up to ~2 billion vectors.

**Algebraic properties:**

| Property | Statement | Test |
|----------|-----------|------|
| Commutativity | `bundle(A,B) = bundle(B,A)` | `test_bundle_commutativity` |
| Identity | `bundle(A) = A` | `test_bundle_single_is_identity` |
| Idempotency | `bundle(A,A,...,A) = A` | `test_bundle_identical_returns_original` |
| Similarity | `sim(bundle(A,B,C), A) > 0.15` | `test_bundle_similarity_to_all_inputs` |
| Majority | `3xA+2xB: sim(result,A) > sim(result,B)` | `test_bundle_majority_vote_dominance` |
| Tie-break | `bundle(ones, zeros) = zeros` | `test_bundle_tie_breaking` |
| Non-empty | `bundle([]) = Err(EmptyBundle)` | `test_bundle_empty_returns_error` |
| Dimension preservation | `dim(bundle(A,B)) = 10000` | `test_bundle_dimension_preserved` |

### 3. Permutation (Cyclic Left Shift)

**What it does:** Creates a new vector by rotating all elements. Used for encoding *position* or *sequence order*.

**Implementation:**
```rust
// new[i] = old[(i + shift) % DIM]
for i in 0..HD_DIMENSIONS {
    let src = (i + effective_shift) % HD_DIMENSIONS;
    new_data.push(self.data[src]);
}
```

**Why it works for position encoding:** Shifting a random vector by even 1 position makes it quasi-orthogonal to the original. This means `permute(A, 0)`, `permute(A, 1)`, `permute(A, 2)`, ... are all approximately orthogonal, giving us a natural way to encode position.

**Algebraic properties (cyclic group Z_10000):**

| Property | Statement | Test |
|----------|-----------|------|
| Identity | `permute(A, 0) = A` | `test_permute_zero_is_identity` |
| Full rotation | `permute(A, DIM) = A` | `test_permute_full_rotation_is_identity` |
| Double rotation | `permute(A, 2*DIM) = A` | `test_permute_double_rotation_is_identity` |
| Invertibility | `permute(permute(A,k), DIM-k) = A` | `test_permute_invertible` |
| Composition | `permute(permute(A,a), b) = permute(A, a+b)` | `test_permute_composition` |
| Quasi-orthogonality | `|sim(permute(A,1), A)| < 0.1` | `test_permute_quasi_orthogonal` |
| Weight preservation | `count_ones(permute(A,k)) = count_ones(A)` | `test_permute_preserves_hamming_weight` |
| Nontriviality | `permute(A,1) != A` (for random A) | `test_permute_nontrivial_changes_vector` |
| Dimension preservation | `dim(permute(A,k)) = 10000` | `test_permute_dimension_preserved` |

---

## Similarity Metrics

### Cosine Similarity

For bipolar vectors, cosine similarity simplifies to:

```
cos(A, B) = (2 * agreements - DIM) / DIM
```

Where `agreements` = number of positions where `A[i] == B[i]`.

| Value | Meaning |
|-------|---------|
| +1.0 | Identical vectors |
| 0.0 | Orthogonal (uncorrelated, expected for random pairs) |
| -1.0 | Anti-correlated (bitwise complement) |

**Implementation:** XOR gives the disagreement mask. Count ones in XOR result = disagreements. Agreements = DIM - disagreements.

### Hamming Distance

The number of positions where bits differ. Related to cosine similarity:

```
hamming = DIM * (1 - cos) / 2
```

---

## Cross-Operation Patterns

### Associative Memory (Bind + Bundle)

The classic HDC key-value store:

```
memory = bundle(bind(K1, V1), bind(K2, V2), ...)
```

To query for the value associated with K1:
```
result = bind(K1, memory)
// result is similar to V1, orthogonal to V2
```

**Tested in:** `test_bind_bundle_kv_recovery`

### Sequence Encoding (Permute + Bind + Bundle)

Encode a sequence `[A, B, C]` preserving order:

```
seq = bundle(
    permute(A, 0),  // A at position 0
    permute(B, 1),  // B at position 1
    permute(C, 2),  // C at position 2
)
```

**Tested in:** `test_permute_sequence_encoding`, `test_bind_permute_combined_encoding`
