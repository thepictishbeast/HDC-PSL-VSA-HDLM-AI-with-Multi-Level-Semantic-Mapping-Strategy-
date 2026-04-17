# PlausiDen Secrets — Capability-Based Secret Management for LFI

## The Problem

When secrets enter LFI's context (chat, logs, memory), three failures occur:
1. Secret enters unbounded scope (chat history, API logs, scrollback)
2. Secret used without provenance (no who/what/why/when)
3. Secret has no policy (no expiry, no use-count, no scope restriction)

## Solution: `plausiden-secrets` crate

Secrets become **capability tokens** — opaque handles the AI reasons over without seeing plaintext.

### Core Types

- `SecretRef` — opaque handle with capability class, policy, provenance chain
- `Capability` — typed by purpose (SshLogin, WebAuth, ApiKey, GpgSign, DiskEncryption, Pii)
- `SecretPolicy` — max_uses, valid_from/until, consent level, plaintext TTL, audit requirements
- `ConsentLevel` — None, PassiveNotify, ActiveConfirm, BiometricGate, QuorumGate

### Redemption Protocol

LFI hands `SecretRef` to a Broker with the operation. Broker validates capability + policy + consent, decrypts into mlock'd memory, performs operation, zeroes plaintext, writes audit record. LFI never sees plaintext.

### PII as Capability Class

Same protocol but with GDPR-aligned defaults: legal basis, retention period, permitted purposes, breach notification. Right-to-erasure = revoke all PiiRefs.

### LFI Integration Points

- HDC: SecretRefs encode as hypervectors with role-binding
- Provenance: every operation produces TracedDerivation with Blake3 commit
- PSL: axiom enforcing capability + policy + consent for all secret operations
- Mesh: macaroon-based capability delegation between nodes

### Build Order

Sprint 1 (2w): Core types + Broker + Vaultwarden backend + SSH handler + audit log
Sprint 2 (2w): WebAuthn + TOTP consent gates + operation handlers
Sprint 3 (3w): HDC encoding + PSL axioms + provenance + macaroon delegation + UI
