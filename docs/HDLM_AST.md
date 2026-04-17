# HDLM — Hyperdimensional Language Model

## Overview

HDLM bridges the gap between the vector space (HDC) and human-readable outputs. It uses a strict two-tier architecture:

- **Tier 1 (Forensic):** Produces mathematically verified Abstract Syntax Trees
- **Tier 2 (Decorative):** Renders ASTs into aesthetic output formats

**The AST is the single source of truth.** Tier 2 cannot modify it.

---

## AST Design

### Arena Allocation

The AST uses arena allocation for memory efficiency and safe reference handling:

```rust
pub struct Ast {
    nodes: Vec<AstNode>,     // Arena: all nodes stored here
    root_id: Option<NodeId>, // Index of the root node
}
```

`NodeId` is a `usize` index into the `nodes` vector. This avoids:
- Lifetimes and borrow checker complexity from `Rc<RefCell<>>` or `Box<>`
- Unsafe pointer manipulation
- Reference cycles

### AstNode

```rust
pub struct AstNode {
    pub id: NodeId,                       // Position in arena
    pub kind: NodeKind,                   // Semantic type
    pub children: Vec<NodeId>,            // Ordered child references
    pub hv_fingerprint: Option<BipolarVector>,  // HDC<->HDLM bridge
}
```

The optional `hv_fingerprint` is the key to bidirectional mapping between the symbolic (AST) and vector (HDC) representations. When an AST is generated from a hypervector query, each node carries the vector that produced it.

### NodeKind Variants

13 variants covering three domains:

**Program Structure:**
| Variant | Fields | Purpose |
|---------|--------|---------|
| `Root` | none | Top-level tree root |
| `Block` | `name: String` | Named code block / function / section |

**Expressions:**
| Variant | Fields | Purpose |
|---------|--------|---------|
| `Literal` | `value: String` | Integer, float, string, bool literal |
| `Identifier` | `name: String` | Variable or symbol reference |
| `BinaryOp` | `operator: String` | Binary operation (+, -, *, /, etc.) |
| `UnaryOp` | `operator: String` | Unary operation (-, !, etc.) |
| `Call` | `function: String` | Function/method call |

**Statements:**
| Variant | Fields | Purpose |
|---------|--------|---------|
| `Assignment` | none | Variable binding / assignment |
| `Return` | none | Return statement |
| `Conditional` | none | If/else branch |
| `Loop` | none | Loop construct |

**Natural Language:**
| Variant | Fields | Purpose |
|---------|--------|---------|
| `Sentence` | none | A sentence/clause in generated prose |
| `Phrase` | `text: String` | A discrete phrase or term |

---

## Traversal

Two traversal methods, both returning `Result<Vec<NodeId>, HdlmError>`:

### DFS (Depth-First Search)

```
       Root(0)
      /       \
   Block(1)  Block(2)
     |
   Lit(3)

DFS order: [0, 1, 3, 2]
```

Uses a stack. Children are pushed in reverse order so the leftmost child is visited first.

### BFS (Breadth-First Search)

```
BFS order: [0, 1, 2, 3]
```

Uses a queue (VecDeque). Level-order traversal.

---

## Tier 1 — Forensic Generation

### ForensicGenerator Trait

```rust
pub trait ForensicGenerator {
    fn generate_from_tokens(&self, tokens: &[&str]) -> Result<Ast, HdlmError>;
    fn generate_from_vector(&self, hv: &BipolarVector) -> Result<Ast, HdlmError>;
}
```

Two entry points:
1. **Token-based:** Takes symbolic tokens and parses into a verified AST
2. **Vector-based:** Takes a hypervector and decodes into an AST (requires trained codebook — Phase 3)

### ArithmeticGenerator

Working demonstration of the Tier 1 pipeline. Parses **prefix-notation** arithmetic:

```
Input tokens: ["*", "+", "1", "2", "-", "5", "3"]
Output AST:   Root -> BinaryOp(*) -> [BinaryOp(+) -> [Lit(1), Lit(2)],
                                       BinaryOp(-) -> [Lit(5), Lit(3)]]
```

Implementation: recursive descent parser via `parse_prefix()`.

**Error cases handled:**
- Empty token stream
- Invalid literals (non-numeric tokens in operand position)
- Truncated expressions (operator without enough operands)
- Unconsumed tokens (extra tokens after a complete expression)

---

## Tier 2 — Decorative Expansion

### DecorativeExpander Trait

```rust
pub trait DecorativeExpander {
    fn render(&self, ast: &Ast) -> Result<String, HdlmError>;
}
```

**Critical invariant:** Takes `&Ast` (immutable reference). Rust's borrow checker guarantees at compile time that no Tier 2 implementation can mutate the AST. This is tested explicitly in `test_tier2_does_not_mutate_ast`.

### InfixRenderer

Renders to standard mathematical notation with full parenthesization:

```
AST: Root -> *(+(1, 2), -(5, 3))
Output: "((1 + 2) * (5 - 3))"
```

### SExprRenderer

Renders to Lisp-style S-expressions:

```
AST: Root -> *(+(1, 2), -(5, 3))
Output: "(* (+ 1 2) (- 5 3))"
```

---

## HDC<->HDLM Bridge

The `hv_fingerprint` field on `AstNode` enables bidirectional mapping:

**Encoding (AST -> Vector):**
Each symbol in a codebook has a random hypervector. An AST node's fingerprint is computed by binding and bundling its kind, children, and position vectors.

**Decoding (Vector -> AST):**
Query the codebook with the vector, find the nearest symbol, and reconstruct the tree.

**Status:** The codebook / item memory is planned for Phase 3. The `generate_from_vector()` method currently returns `Tier1GenerationFailed` with a clear message about this dependency.

---

## Error Cases

| Error | When | Recovery |
|-------|------|----------|
| `MalformedAst` | Invalid node IDs, self-referencing children | Fix AST construction logic |
| `Tier1GenerationFailed` | Bad tokens, truncated input, missing codebook | Check input, retry with valid tokens |
| `Tier2ExpansionFailed` | Unsupported NodeKind, wrong child count | Check AST structure matches renderer expectations |
| `EmptyAst` | DFS/BFS/render on tree with no root | Ensure root node is added before traversal |
| `UnmappedSymbol` | Vector decoding finds no codebook match | Expand codebook, retrain |
