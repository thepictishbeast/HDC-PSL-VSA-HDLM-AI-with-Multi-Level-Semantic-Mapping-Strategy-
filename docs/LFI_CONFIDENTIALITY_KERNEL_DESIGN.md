# LFI Confidentiality Kernel — Structural Secret Protection

## Architecture

A mandatory confidentiality kernel that every LFI operation passes through.
Secrets cannot exist as plaintext outside protected execution — enforced by
Rust's type system at compile time, not runtime checks.

```
Application Layer (LFI reasoning) — only sees Sealed<T> handles
═══════════════════════════════════════════════════════════════
CONFIDENTIALITY KERNEL (mandatory, unbypassable)
  • Type system enforces sealing
  • Memory: encrypted-at-rest in RAM
  • Compute: TEE / FHE / MPC
  • Egress: TLS-only, scrubbed
  • Audit: Merkle-chained per op
═══════════════════════════════════════════════════════════════
Storage / Network / Compute
```

## Eight Subsystems

### 1. Type System — Sealed<T>
- `Sensitive` trait marks all secret data
- `Sealed<T>` wrapper: inner T unreachable except via `use_within` closure
- Does NOT implement Debug, Display, Serialize, Clone
- Compiler refuses to log, print, or serialize secrets

### 2. Encrypted RAM
- XChaCha20-Poly1305 encrypted at rest in memory
- Keys in: ConfidentialVM (SEV-SNP) > SGX > TrustZone > TPM > mlock'd keyring
- Plaintext only during use_within, in MADV_DONTDUMP + mlock'd pages
- Zeroized on drop

### 3. Encrypted Computation
- Standard: Sealed in RAM, plaintext in use_within only
- TeeRequired: operation runs inside TEE
- HomomorphicOnly: TFHE computation, plaintext never reconstructed
- SecretShared: MPC with N-of-M threshold

### 4. Zero-Knowledge Proofs
- Proof of authentication without revealing password
- Proof of correct computation on sealed data (Nova folding)
- Proof of policy compliance (capability + consent + time window)

### 5. Network Egress
- Typed SecureChannel API — no send_plaintext method exists
- TLS 1.3 only, certificate pinning, CT verification
- API calls: secrets go as HTTP headers via broker, never in prompts
- nftables enforces only egress kernel makes outbound connections

### 6. Egress Scanner
- Pattern matchers (AWS keys, GitHub tokens, private keys, JWT, CC#, SSN)
- Entropy detector for unknown-format secrets
- NER for PII (GLiNER via ONNX)
- Vault-membership check (hash comparison against known sealed secrets)

### 7. Chat History Scrubber
- Watches: conversation logs, bash_history, zsh_history, terminal scrollback
- Detects secrets via same scanner as egress
- Replaces with [REDACTED:capability_id]
- Auto-rotates burned secrets via vault API
- Surfaces manual rotation tasks where API unavailable

### 8. Audit Kernel
- Merkle-chained append-only log
- Every entry: sequence, timestamp, operation, capability_id, policy_hash, actor, outcome
- NO plaintext in audit entries ever
- TPM-attested periodic checkpoints
- Optional: publish to transparency log (Sigstore Rekor)

## Build Order

Sprint A (3w): Type system — Sealed<T>, Sensitive, broker, audit kernel
Sprint B (2w): Memory protection — mlock, keyring, hardware detection, zeroize
Sprint C (3w): Network egress — rustls, SecureChannel, scanner, prompt-builder
Sprint D (2w): Chat scrubbing — conversation adapters, scrub daemon, rotation
Sprint E (4-6w, optional): Encrypted compute — TEE, TFHE, MPC
Sprint F (3w, optional): ZK provenance — Nova folding, proof generation

## Strategic Value

This combination doesn't exist elsewhere as an integrated stack.
Makes LFI deployable in healthcare, defense, finance, government —
environments where current AI systems are categorically excluded.
