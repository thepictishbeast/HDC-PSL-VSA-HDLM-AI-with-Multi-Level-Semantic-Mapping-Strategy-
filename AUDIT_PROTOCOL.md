# PlausiDen AI — Audit Protocol

## Audit Rotation Schedule

Run audits in round-robin order, 3 passes each. Never run the same audit
back-to-back — always interleave with a different audit type between passes.

### Rotation Order (repeat 3x):
```
Pass 1: Security → Supersociety → Code Quality → Data Quality → Docs → Logs → Dependencies → Residual
Pass 2: Supersociety → Code Quality → Security → Logs → Data Quality → Residual → Dependencies → Docs
Pass 3: Code Quality → Security → Data Quality → Docs → Supersociety → Dependencies → Residual → Logs
```

## Audit Types

### 1. SECURITY (AVP-2 Tier 3)
- [ ] All POST endpoints have input size limits
- [ ] All destructive endpoints require authentication
- [ ] CORS restricted to localhost origins
- [ ] No SQL injection (parameterized queries only)
- [ ] No command injection (no format!() with user input in shell commands)
- [ ] Error responses scrubbed of internal details
- [ ] Secrets never in source code or logs
- [ ] Constant-time comparison for all auth tokens
- [ ] Rate limiting on expensive endpoints
- [ ] WebSocket message size capped

### 2. SUPERSOCIETY COMPLIANCE
- [ ] PSA filter: no telemetry upload, no external service deps for core function
- [ ] All data encrypted at rest capability (AES-256-GCM path exists)
- [ ] Provenance: every derivation is TracedDerivation or ReconstructedRationalization
- [ ] PSL pass rate 95-98% on adversarial corpus (not 100%)
- [ ] HDC vectors use CRDT consensus (not naive bundling) for mesh
- [ ] CaMeL barrier on incoming messages
- [ ] Commit-reveal on all epistemic claims
- [ ] No single point of failure in mesh design

### 3. CODE QUALITY
- [ ] cargo clippy -W clippy::all passes
- [ ] cargo fmt --check passes
- [ ] No .unwrap() in library code without SAFETY comment
- [ ] Every public function has doc comment
- [ ] Every module has Purpose/Design/Invariants/Failure header
- [ ] Test-to-public-function ratio >= 2
- [ ] No dead code (warnings = 0 target)
- [ ] All error types use thiserror

### 4. DATA QUALITY
- [ ] Dedup rate < 1% on live facts
- [ ] No facts with value < 10 chars
- [ ] Quality scores assigned to all facts
- [ ] Domain classification on all facts
- [ ] Adversarial corpus >= 1M examples
- [ ] FTS5 index in sync with facts table
- [ ] Staging table validated before promotion

### 5. DOCUMENTATION
- [ ] IMPROVEMENTS.md current with today's work
- [ ] SESSION-LOG.md written
- [ ] README.md accurate (test count, fact count, feature list)
- [ ] API reference matches actual endpoints
- [ ] All /root/LFI/docs/ files current
- [ ] CLAUDE.md instructions accurate

### 6. LOGGING & OBSERVABILITY
- [ ] tracing::info on all public function entry points
- [ ] tracing::warn on all error recovery paths
- [ ] tracing::error on all unrecoverable failures
- [ ] Structured fields (not string interpolation) in log messages
- [ ] Log output to /var/log/lfi/ with identifiable module names
- [ ] /api/admin/logs endpoint exists and works
- [ ] LFI can read its own logs for self-diagnosis

### 7. DEPENDENCIES
- [ ] cargo audit — zero known vulnerabilities
- [ ] No outdated critical dependencies
- [ ] No unused dependencies in Cargo.toml
- [ ] Rust edition 2021
- [ ] All feature flags intentional

### 8. RESIDUAL CLEANUP
- [ ] No TODO/FIXME/HACK without ticket reference
- [ ] No commented-out code blocks > 5 lines
- [ ] No stale config files
- [ ] No orphaned test files
- [ ] No debug print statements in production paths
- [ ] .gitignore covers all generated artifacts
- [ ] No secrets in git history

## Scoring

Each audit item: PASS / FAIL / N/A
Per audit type: count PASS / total applicable
Target: >= 90% PASS rate on every audit type after 3 passes
