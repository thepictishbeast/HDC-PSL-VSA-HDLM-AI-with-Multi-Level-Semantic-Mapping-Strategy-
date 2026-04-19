# LFI Manager & Training Guide

In-app manual for running an LFI instance. Covers day-to-day operation, training workflows, and how to read every dashboard. Mobile-friendly: all tabs in the UI that this guide references exist on phone layouts.

## 1. What you're looking at

LFI is a **post-LLM** reasoning system. There is no transformer. There are no tokens. There is a fact graph, a hypervector substrate, a symbolic logic layer, and a language encoder — together they answer questions with **checkable provenance on every claim**.

The three things that make LFI different from Claude / GPT / Gemini:

- **Every assertion is a clickable `[fact:KEY]` chip** that opens its full derivation — no hallucination without a trail.
- **It runs on your hardware.** No cloud round-trip, no vendor retention.
- **You improve it directly.** Type a correction, the system updates a persistent axiom. Not "next training cycle" — *now*.

## 2. Quick tour of the UI

### Chat
The main conversation surface. Type a question, get a response. Every cited fact is a chip you can click to see where the claim came from.

### Classroom — the training control centre
Ten sub-tabs. In priority order for a new operator:

- **Profile** — who the system thinks it is. Sovereign name, voice preferences, persistent facts about *you* learned across conversations.
- **Ingestion Control** — start / stop / monitor corpus ingest runs. This is how you add data.
- **Drift** — six health metrics over time with sparklines. Red = act now. Export to JSON for a ticket or paste into a spreadsheet.
- **Ledger** — unresolved contradictions. One-click resolve or auto-resolve-by-trust. Click any fact key to see its ancestry.
- **Runs** — ingest job history. Filter + search.
- **Office Hours** — recent user feedback (👍 / 👎 / pencil corrections). Each correction becomes an axiom update; this tab is where you audit the stream.
- **Curriculum** — which domains the system has seen vs. which are thin. Sortable + filterable.
- **Gradebook** — per-domain mastery scores (FSRS).
- **Lessons** — scheduled FSRS fact reviews due now.
- **Reports** — exportable KPI snapshots.

### Library
Corpus marketplace. Every source that has contributed facts, ranked by a composite (trust × avg-quality × vetted × log-size). Adjust trust weights with the slider. Low-scoring sources get down-weighted in retrieval + cross-source reconciliation.

### Knowledge
Interactive fact browser + FSRS review queue. Four rating buttons per due card (Again / Hard / Good / Easy = 1–4).

### Admin
Operator-only. Tokens (capability-scoped, rotatable), Integrity (audit chain), Proof (Lean4 verdicts), Diag (runtime event ring-buffer, no DevTools needed), Tokens, Events.

## 3. Training workflows

### 3.1 Teach LFI a fact

**In Chat.** Say "Remember that X is Y." The auto-learn pipeline catches `my name is …`, `I like …`, `call me …`, etc. and writes a profile fact.

**Via the API.** `POST /api/knowledge/learn { concept, related[] }` (requires auth). Related items bind the new concept to existing graph nodes.

**Bulk (advanced).** Drop a JSON-Lines file in a supported corpus format (CauseNet, ATOMIC, Wikidata, discourse, semantic-role, dialogue-act — all have parsers in `lfi_vsa_core/src/ingest/`) and trigger a batch via `POST /api/ingest/start`.

### 3.2 Correct a wrong answer

Click the 👎 on the offending message. A text field appears — type the correct version. Two things happen:

1. The corrected value lands in `user_feedback` as audit.
2. The signal is captured by `ExperienceLearner` and folded into future retrieval (an axiom weight shifts, or a contradiction row opens for human triage).

This is the ongoing-improvement loop. **You speak, it updates. No retraining, no redeploy.**

### 3.3 Schedule reviews (FSRS)

Open Knowledge. Due cards appear with four rating buttons. Rate them: Again (1) through Easy (4). The FSRS scheduler updates stability + difficulty per card and picks the next review date. Same algorithm Anki uses.

### 3.4 Verify a claim (Lean4 / Kimina)

If you have a Kimina server running (optional), click Verify on any fact chip. The system ships a Lean4 proof obligation; the server returns proved / rejected / unreachable. Proved facts get a green check + proof hash; rejected ones demote.

### 3.5 Clean up contradictions

Classroom → Ledger. Each row shows two competing values for the same key. Three actions:

- **Keep A / Keep B / Dismiss** on a single row.
- **Auto-resolve** at the top: resolves every row where source-trust differs by ≥ 0.20 in favour of the higher-trust source.

## 4. The six Drift metrics (and what to do when they go red)

| Metric | What it means | Red action |
|---|---|---|
| Fresh facts | % of sampled rows updated ≤ 7 days ago | Kick an ingest in Runs tab |
| Stale facts | % > 365 days old | Schedule a re-verify pass |
| HDC cache | % of facts with a precomputed hypervector | POST /api/hdc/cache/encode {limit:1000} |
| Contradictions | # pending review | Ledger → Auto-resolve |
| Neg feedback 24h | down-votes / total | Office Hours — read what went wrong |
| FSRS lapse | lapses / cards | Review the failing cards in Knowledge |

Each card is clickable — goes to the matching Classroom tab so you can act.

## 5. Deep control surface

### 5.1 Context window size (RAM cap)

Settings modal → Workspace slider. Default 512 MB = ~275,000 slots. Minimum 1 MB = effectively no workspace. Maximum 16 GB (hard cap for safety). Every resize is chained into the audit log.

### 5.2 Cognitive tier (Pulse / Bridge / BigBrain)

Authenticated API only. Not an LLM switch — it's a depth-of-reasoning dial:

- **Pulse** — fast prototype match, no planning.
- **Bridge** — planner runs, multi-step reasoning attempted.
- **BigBrain** — full cognitive pipeline: planner + abduction + critique.

### 5.3 Per-source trust

Library → Trust slider per row. 0 = adversarial (ignored). 1 = fully trusted (wins every contradiction). 0.5 = default for unknown sources.

### 5.4 Capability tokens

Admin → Tokens. Issue scoped credentials for:

- `ingest` — bulk corpus loading
- `admin_read` — read-only dashboards
- `chain_append` — add security-audit entries
- `auth` / `research` / `hdc_encode` — explicit API access

Hashes are SHA-256 stored; you see the raw value ONCE at issue. Rotate frequently.

## 6. External bridges

### 6.1 Feeding Gemini-CLI output into LFI

Gemini (or any LLM) can be used as a *data producer*, not a runtime. The workflow:

```
gemini-cli prompt "Generate 100 (subject, predicate, object) tuples about
chemistry suitable for a forensic AI knowledge base. JSON lines, each with
{subj, pred, obj, tier:'scientific', provenance:{source:'gemini_cli',
extracted_at:'<iso>'}}." \
  > /tmp/gemini_chem_tuples.jsonl

curl -X POST http://127.0.0.1:3000/api/ingest/start \
  -H 'Content-Type: application/json' \
  -d '{"run_id":"gemini_chem_001","corpus":"gemini_cli","tuples_requested":100}'

# Stream the file through the tuple extractor
while read line; do
  curl -s -X POST http://127.0.0.1:3000/api/tuples/extract \
    -H 'Content-Type: application/json' \
    -d '{"limit":1}'
done < /tmp/gemini_chem_tuples.jsonl

curl -X POST http://127.0.0.1:3000/api/ingest/finish \
  -d '{"run_id":"gemini_chem_001","status":"completed"}'
```

Set Library trust for `gemini_cli` to 0.4–0.6 (it's an adversarial generator from LFI's doctrine) until you've run validation.

### 6.2 Two-instance debate (LFI ↔ Gemini)

Run two chat loops: one against LFI, one against gemini-cli. Feed LFI's response (with its fact chips) as the next prompt to gemini; feed gemini's counter-argument back to LFI. LFI's refusal-with-reason path will flag claims it can't ground. Captures the difference between vibes and verifiable knowledge.

## 7. Doctrine (the non-negotiable rules)

These are enforced by CI (`tests/doctrine_audit.rs`):

1. **No LLM imports.** Nothing under `ollama::`, `openai::`, `anthropic::`, `llama_cpp::`, `hf_hub::`, `tokenizers::`. Violating commit fails the build.
2. **No hardcoded response pools in source.** ≥ 5 sentence-like strings in a `const &[&str]` fails audit. Response language must be sampled from learned patterns.
3. **No `.unwrap()` / `.expect()` in library code** without `// SAFETY:` or `// test-only`. Ratcheted against a ceiling that comes down over time.
4. **Secret comparisons are constant-time.** `password == X` without `subtle::ConstantTimeEq` fails.

Plus data audits (`/api/audit/datasets`, 6 checks):

1. Edge orphans (sample)
2. Source mono-culture (flag at > 95 %)
3. FTS5 freshness probe
4. Contradiction backlog (flag at > 10k)
5. Source-trust coverage (active sources with no trust row)
6. Schema parity (every column the Rust code reads is present)

Both run on every commit in CI and are visible in Admin → Diag.

## 8. When something breaks

### "Backend isn't streaming"
The 45 s timeout banner. Backend *is* answering — it just doesn't send chunks yet (streaming chat is task #384). Wait; the final response will land. If it really is stuck, `Stop` and retry.

### "Rate limit exceeded"
Per-capability: auth 5/60 s, research 10/300 s, hdc_encode 30/60 s. Counters reset on a rolling window.

### Contradictions rising fast
A new corpus is disagreeing with established facts. Library → lower its trust, then Ledger → Auto-resolve.

### First response after restart is slow
Warmup runs at startup but can take a few seconds on a 92 GB brain.db. Subsequent responses should be ≤ 100 ms. Watch the server log for `STARTUP: warmup done in …`.

## 9. Quick reference — HTTP API

| Endpoint | Method | What |
|---|---|---|
| `/api/chat` (WS `/ws/chat`) | WS | Conversation |
| `/api/health/extended` | GET | One-call dashboard bundle |
| `/api/drift/snapshot` | GET | 11 health metrics |
| `/api/ingest/list` | GET | Run history |
| `/api/ingest/start` | POST | Kick a run |
| `/api/contradictions/recent` | GET | Ledger |
| `/api/library/quality` | GET | Per-source dimensions |
| `/api/corpus/marketplace` | GET | Composite-ranked sources |
| `/api/sources/trust` | GET/PUT | Trust weights |
| `/api/fsrs/due` | GET | Review queue |
| `/api/fsrs/review` | POST | Submit a grade |
| `/api/proof/verify` | POST | Lean4 check |
| `/api/audit/chain/verify` | GET | Integrity banner |
| `/api/audit/datasets` | GET | 6-check dataset audit |
| `/api/explain` | POST | Dry-run a query |
| `/api/settings/workspace` | GET/PUT | RAM cap |
| `/api/parse/english` | POST | Tokenise + POS tag |
| `/api/hdlm/render` | POST | Concept similarity sketch |

## 10. Solo operator playbook — days without Claude

You can run + improve LFI entirely without outside help. Follow this loop:

### 10.1 Daily rhythm

1. **Morning (5 min).** Open Classroom → Drift. Any red tile = act before new ingest:
   - Contradictions red → Ledger → **Auto-resolve** (requires ≥ 0.20 trust gap between sources). Anything left, do Keep A / Keep B / Dismiss per row.
   - HDC cache red → `curl -X POST http://127.0.0.1:3000/api/hdc/cache/encode -d '{"limit":1000}'` and let it catch up.
   - FSRS lapse red → open Knowledge, rate the failing cards (Again / Hard / Good / Easy).
2. **During the day (as you work).** Chat. Every canned-sounding reply? Hit 👎 and type a better answer. That correction is the training signal — the axiom weights reshape for next time.
3. **Evening (10 min).** Classroom → Office Hours. Read the day's feedback queue. Anything needing triage, do it now. Close any resolved items.

### 10.2 Add a new corpus you trust

1. Library → scroll to the bottom. If the source isn't listed, it hasn't been ingested yet.
2. Drop the JSONL into `/home/user/LFI-data/` (parsers in `lfi_vsa_core/src/ingest/` know the CauseNet / ATOMIC / Wikidata / discourse / dialogue formats).
3. `curl -X POST http://127.0.0.1:3000/api/ingest/start -H 'content-type: application/json' -d '{"run_id":"manual_001","corpus":"my_corpus","tuples_requested":10000}'`
4. Tail /var/log/lfi/server.log.* for `INGEST:` lines. When finished, the Library tab shows the new source with a default trust of 0.5.
5. Set trust: Library → slider. 0 for adversarial, 0.7+ for anything you'd actually quote, 1.0 only for sources you verified in person.

### 10.3 Delete a wrong fact

1. Chat → click the `[fact:KEY]` chip that's wrong.
2. Popover → **Dismiss as wrong**. The fact is soft-deleted + the source's trust is nudged down.
3. If the whole source is bad: Library → set trust to 0 (all its contributions are down-weighted in retrieval).

### 10.4 Back up the brain

```bash
# Cold backup (server stopped)
pkill -TERM -f 'release/server'; sleep 5
cp /home/user/.local/share/plausiden/brain.db /home/user/LFI-data/brain.db.$(date +%Y%m%d).bak
nohup /home/user/cargo-target/release/server > /tmp/lfi_server.log 2>&1 & disown
```

Or live: `sqlite3 /home/user/.local/share/plausiden/brain.db ".backup /home/user/LFI-data/brain.db.live.bak"`.

### 10.5 Server won't start

Typically:
- **Port 3000 busy** → `ss -tln | grep :3000` → find PID → kill it → retry.
- **brain.db locked** → `lsof /home/user/.local/share/plausiden/brain.db`. Kill any stray ingester.
- **Panic on startup** → Last 40 lines of `/var/log/lfi/server.log.$(date +%F)` usually tells you which corpus / row is malformed.

### 10.6 Quick smoke test (30 s)

```bash
curl http://127.0.0.1:3000/api/health
curl http://127.0.0.1:3000/api/status
curl -X POST http://127.0.0.1:3000/api/settings -H 'content-type: application/json' -d '{"key":"probe","value":"ok"}'
```

All three should return `ok` or a populated JSON body in < 50 ms.

### 10.7 Chat feels stuck

1. Check `ss -tln | grep 3000` — server listening?
2. Hit `curl http://127.0.0.1:3000/api/health`. If 200 → problem is WebSocket, not REST. Reload the tab; the UI auto-reconnects.
3. Look at `/var/log/lfi/server.log.$(date +%F)` tail — `CHAT-TRACE[xxxx]` lines show each stage of a turn. Wherever the +ms offsets stop growing is where it's stuck.
4. If backend is warming brain.db after a restart: wait for `STARTUP: warmup done` line. First hit after that is fast again.

## 11. Knowing what to ask

LFI is substrate-first. It's good at:
- **Definitions** — "what is X", "define X", "tell me about X"
- **Causal chains** — "why does X cause Y", "how does X work"
- **Disambiguation** — "X vs Y"
- **Walk-the-graph** — "what's related to X", "what depends on X"

It refuses (with reason) on:
- Open-ended casual chit-chat (no fact grounds it)
- Topics no ingested source covers (rephrase as knowledge query instead)
- Questions that hinge on live data LFI can't see (weather, news, markets)

That refusal is a feature, not a bug — per doctrine it won't fabricate. If you want it to answer something it currently refuses, ingest a corpus that grounds the topic (§ 10.2).

## 12. Roadmap

Detail in `/docs/LFI_BEATS_LLMS_ROADMAP.md` (60 tasks across 9 tiers). Current session frontier in `/docs/LFI_TASK_QUEUE_2026-04-19.md`.

Shipped in the latest push (ff4a5f4 + predecessors):
- lock-free stats cache: chat under UI-poll load 11 s → 40 ms
- de-hardcoded conversational pool: Pulse routes through substrate
- /api/avp/status, /api/settings generic, WS keepalive
- #398 workspace slider (Settings → Behavior → Workspace capacity)
- #359 calibrated refusal when no fact grounds the claim

Remaining headline items:
- #399 Learn uncertainty expressions from the dialogue corpus
- #384 Streaming chat (gets rid of the 45 s "not streaming" banner permanently)
- #382 ARM64 mobile build on a real device
- #390 LFI mesh federation — multiple instances sharing facts via signed CRDT gossip

---

Questions? Hit the 👎 on anything in this guide and type what's unclear — the correction feeds back into LFI's own knowledge of how to explain itself.
