# LFI in-flight task queue — frozen 2026-04-19

Saved so the next session can rehydrate without relying on the in-memory task tool state. 41 tasks pending at handoff. Ordered by priority tier from `docs/LFI_BEATS_LLMS_ROADMAP.md` with one-line starting hints.

## Tier 1 — fluent language grounded in the substrate

Pick next:

- **#359 Refusal with reason** — when confidence < 0.5 or no source ≥ 0.7 trust covers the claim, respond "I don't know because …" naming the blocking condition (missing axiom / no source / unresolved contradiction). Partner to calibrated hedging.
- **#360 Multi-sentence discourse composition** — connect clauses with #334 discourse relations (Contrast, Cause, Elaboration). Emit Contrast between disagreeing fact tiers; Cause between an IsA and a Causes; Elaboration between base and sub-property.
- **#361 Style-transfer axioms (concise / formal / technical)** — PSL style axiom that a user correction like "be more concise" updates. All future responses honour it.
- **#362 Paraphrase robustness** — generate N paraphrases, keep the one with highest mean PSL pass rate.
- **#363 Source-agreement surfacing** — "Wikidata says X, ConceptNet says Y, I'm going with X because trust 0.85 > 0.70".

## Tier 2 — reasoning the LLMs fake

- **#364 Multi-hop HDC analogy (A:B :: C:?)** — unbind(B, A) ⊗ C, resonator-factorised. Ships `/api/analogy`.
- **#365 Counterfactual reasoning — "what if X were false"** — flip one fact, recompute dependent axiom pass rates, surface what changes.
- **#367 Abductive inference** — given observations, return minimum axiom set that explains them, ranked by simplicity.
- **#368 Multi-step planner with backtracking** — goal → precondition chain → action sequence; backtracks on axiom failure.
- **#369 Self-consistency vote** — N chains, pick modal answer; refuse on no majority.
- **#370 Critique-then-revise loop** — second pass tries to falsify each sentence; surviving ones ship.
- **#371 Non-monotonic default reasoning** — "birds fly" + explicit exception facts (penguins, ostriches).

## Tier 3 — multimodal, locally

- **#372 Image → HDC via existing transducer, wired into chat** — transducer shipped; wire into chat handler for "what's in this picture".
- **#373 Document ingest (PDF / DOCX / HTML)** — chunked, tuple-extracted, per-page provenance.
- **#374 Table-understanding ingestor** — column headers as predicates, rows as (header, value) tuples.
- **#375 Code-as-AST ingest (Python / Rust / JS front-ends)** — feed hdlm::Ast with per-language parsers.

## Tier 4 — grounded learning

- **#376 Conversation-to-tuple auto-ingestion** — every chat turn flows through the #329 tuple extractor.
- **#377 User-correction into axiom weights** — 👎 + correction updates axiom weight, not just a feedback row.
- **#378 Active question generation** — detect low-confidence retrieval, ask the user to fill the gap.
- **#399 Learn hedges from dialogue corpus** — mine `dialogue_tuples_v1` for uncertainty expressions (probably / likely / maybe / could be / usually / typically), tag by confidence bin, sample at render.
- **#400 Replace all hardcoded response templates** — strip the jokes / greetings / anchor pools in `reasoner.rs`. Replace with sampling from the dialogue corpus. Unblocks the legacy_exempt in doctrine audit.

## Tier 5 — safety that LLMs don't have

- **#379 Per-user encryption key for contributed facts** — user key encrypts their facts at rest; delete key → crypto-shred.
- **#380 Privacy dashboard** — per-answer "LFI used facts X, Y, Z" in plain language.
- **#381 Red-team property harness in CI** — adversarial input properties; every commit must survive.
- **#389 Cryptographically signed conversation export** — Ed25519 signature; tamper-evident shareable conversations.
- **#393 Homomorphic query option** — CKKS / BFV; server never sees plaintext query.
- **#394 Federated learning** — phones train locally, share gradient-only updates, differentially private.

## Tier 6 — performance

- **#382 ARM64 mobile build end-to-end** — scaffold shipped; deploy + verify on a real Android.
- **#383 NEON SIMD for HDC bind/bundle/similarity** — 4-8× ARM speedup, scalar fallback identity.
- **#384 Streaming chat response** — concept-by-concept WS deltas so it feels faster than Claude/GPT.

## Tier 7 — UX

- **#385 Voice conversation — STT → LFI → TTS** — Whisper.cpp + Piper, offline.
- **#386 Time-travel debugging — replay past query** — snapshot fact base per turn; replay at any point.
- **#387 Fork conversation — "what if fact X weren't there?"** — exclude a fact, see alt answer.
- **#388 Multi-LFI debate — two instances argue, third judges** — user watches the exchange.
- **#398 UI RAM-cap control** — slider in Settings modal mounted by Chat/Classroom/Admin. Backend endpoint shipped.

## Tier 8 — ecosystem

- **#390 LFI mesh federation — CRDT delta gossip over HTTPS** — signed-update HTTPS service; wire the existing HdcCrdt + CrdtDelta.
- **#391 Benchmark suite vs GPT-5 / Claude / Gemini** — provenance, multi-turn coherence, refusal-on-unknown, consistency.
- **#396 Paper: Post-LLM Cognitive Architecture** — writeup establishing the category.

## Tier 9 — finishing close-but-not-done work

- **#366 Temporal valid_from / valid_to on facts** — extend the existing temporal_class column with explicit datetime bounds + query filter.
- **#392 Wikidata streaming ingester** — parser shipped (#200); build `tools/lfi-wikidata-ingest` binary that drains 100 GB bz2.
- **#395 Trained English dep+constituency parser in #343 seam** — swap heuristic with a real model at the pos_tag + build_tree boundary.

## Rehydration recipe

When you resume:

1. `git log --oneline main | head -20` → see what shipped
2. Open `docs/LFI_SESSION_HANDOFF_2026-04-19.md` for the standing-orders context
3. Open this file for the queue
4. Pick by priority:
   - If backend-only + fast: #359 / #399 / #400
   - If backend + needs Claude 2: #363 / #372 / #380
   - If big + alone: #400 (de-hardcoding is the blocker for doctrine full-compliance)
5. `cargo test --release --test doctrine_audit` — must pass before any commit
6. `cargo test --release` — sanity check
7. Server restart: `pkill -9 -f target/release/server; nohup /home/user/cargo-target/release/server > /tmp/lfi_server.log 2>&1 &`

## Cross-session invariants

- Post-LLM. No transformer, no Ollama, no attention. HDC + PSL + HDLM only.
- `[fact:KEY]` chips on every assertion — not optional.
- No hardcoded response pools. Learn + sample from corpus.
- Numeric certainty 0.00–100.00% where possible.
- Every secret comparison constant-time.
- Every external data sink scrubs secrets first.
- `tests/doctrine_audit.rs` must pass.
- User runs Kali in rescue mode — don't touch GRUB / initramfs / wlan0.
