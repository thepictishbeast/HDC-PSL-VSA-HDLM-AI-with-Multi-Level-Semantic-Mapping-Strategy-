# PlausiDen AI — Complete API Reference

> Server: Rust (axum) on port 3000.  
> Frontend: React + Vite on port 5173.  
> All endpoints accept/return JSON. Auth via `POST /api/auth`.

---

## WebSocket Endpoints

### `WS /ws/chat`
Real-time chat. Send `{ "content": "your message" }`, receive:
```json
{
  "type": "chat_response",
  "content": "AI reply text",
  "mode": "Fast | Deep",
  "confidence": 0.0-1.0,
  "tier": "Pulse | Bridge | BigBrain",
  "intent": "Converse { message: ... } | WriteCode { ... } | ...",
  "reasoning": ["step 1", "step 2"],
  "plan": { "steps": 6, "complexity": 0.58, "goal": "..." } | null,
  "conclusion_id": 12345
}
```
May also send `{ "type": "web_result", "query": "...", "summary": "...", "source_count": 3, "trust": 0.85 }` if web search was triggered.
May send `{ "type": "chat_error", "error": "..." }` on failure.

**Every chat turn is logged** to `/var/log/lfi/chat.jsonl` with: `ts`, `user`, `reply`, `tier`, `intent`, `mode`, `confidence`, `conclusion_id`.

### `WS /ws/telemetry`
Pushes every 1s:
```json
{
  "type": "telemetry",
  "data": {
    "ram_available_mb": 52000,
    "ram_total_mb": 64000,
    "ram_used_mb": 12000,
    "cpu_temp_c": 67.0,
    "is_throttled": false,
    "vsa_orthogonality": 0.02,
    "axiom_pass_rate": 1.0,
    "logic_density": 0.0
  }
}
```

---

## Authentication

### `POST /api/auth`
```json
// Request
{ "key": "your-sovereign-key" }
// Response (success)
{ "status": "authenticated", "tier": "Sovereign" }
// Response (fail)
{ "status": "rejected" }
```
Required before: `/api/tier`, `/api/knowledge/learn`, `/api/system/notify`, `/api/system/clipboard`, `/api/provenance/export`, `/api/provenance/compact`, `/api/provenance/reset`.

---

## Status & Health

### `GET /api/status`
```json
{
  "tier": "Pulse",
  "authenticated": true,
  "entropy": 0.1,
  "facts_count": 12,
  "concepts_count": 36,
  "session_id": "SESSION_...",
  "background_learning": false
}
```

### `GET /api/health`
Subsystem health for load balancers:
```json
{
  "ok": true,
  "psl": true,
  "knowledge": true,
  "memory": true,
  "holographic": true,
  "sensorium": true,
  "crypto_epistemology": true
}
```

### `GET /api/metrics`
Prometheus text-format exposition of counters: `lfi_think_total`, `lfi_chat_total`, `lfi_audit_total`, `lfi_opsec_scan_total`, `lfi_provenance_query_total`.

### `GET /api/qos`
Full QoS compliance sweep:
```json
{
  "passed": true,
  "critical_failures": 0,
  "warnings": 0,
  "checks": [
    { "name": "PSL Axiom Pass Rate", "passed": true, "value": "100.0%", "threshold": ">= 95.0%", "severity": "Info" },
    { "name": "Thermal Compliance", "passed": true, "value": "67C", "threshold": "<= 80C", "severity": "Info" },
    { "name": "VSA Orthogonality", "passed": true, "value": "0.0000", "threshold": "<= 0.10", "severity": "Info" },
    { "name": "RAM Availability", "passed": true, "value": "52853MB", "threshold": ">= 2000MB (Bridge)", "severity": "Info" },
    { "name": "Throttle Status", "passed": true, "value": "NOMINAL", "threshold": "Not throttled", "severity": "Info" },
    { "name": "Memory Safety (forbid(unsafe_code))", "passed": true, "value": "ENFORCED", "threshold": "forbid(unsafe_code)", "severity": "Info" }
  ]
}
```

---

## Intelligence

### `POST /api/think`
Structured reasoning with provenance tracking (not conversational — use /ws/chat for chat).
```json
// Request
{ "input": "explain the CAP theorem" }
// Response
{
  "status": "ok",
  "answer": "The CAP theorem states...",
  "confidence": 0.87,
  "mode": "Deep",
  "conclusion_id": 42
}
```
Max input: 16 KiB.

### `POST /api/tier`  *(auth required)*
Switch the active model tier.
```json
// Request
{ "tier": "BigBrain" }   // Pulse | Bridge | BigBrain
// Response
{ "status": "ok", "tier": "BigBrain" }
```

### `POST /api/search`
Web search with cross-referencing and trust scoring.
```json
// Request
{ "query": "latest Rust async runtime benchmarks" }
// Response
{
  "query": "...",
  "summary": "...",
  "source_count": 5,
  "cross_reference_trust": 0.82,
  "sources": [...]
}
```

### `GET /api/facts`
All facts the agent has learned in this session:
```json
{ "facts": [{ "key": "sovereign_name", "value": "Wyatt" }], "count": 1 }
```

### `GET /api/agent/state`
Aggregated dashboard state in one call (replaces fan-out across health + concepts + provenance):
```json
{
  "subsystems": { "psl": true, "knowledge": true, ... },
  "axiom_count": 5,
  "axiom_names": ["DimensionalityAxiom", ...],
  "knowledge": { "concepts": 36, "facts": 12, "mastered": 8 },
  "provenance": { "total_conclusions": 42, "total_nodes": 128 },
  "tier": "BigBrain",
  "authenticated": true
}
```

---

## Knowledge / Spaced Repetition

### `POST /api/knowledge/learn`  *(auth required)*
Teach LFI a new concept:
```json
// Request
{ "concept": "monads", "description": "a design pattern for chaining operations with context" }
// Response
{ "status": "ok", "concept": "monads", "mastery": 0.1 }
```

### `POST /api/knowledge/review`
Record a spaced-repetition review grade:
```json
// Request
{ "concept": "monads", "grade": 4 }  // grade: 0-5 (SM-2)
// Response
{ "status": "ok", "concept": "monads", "new_mastery": 0.35, "next_review_days": 3.2 }
```

### `GET /api/knowledge/due`
Concepts due for review right now:
```json
{ "due": [{ "name": "monads", "mastery": 0.35, "days_overdue": 1.2 }] }
```

### `GET /api/knowledge/concepts`
Full concept list with mastery:
```json
{ "concepts": [{ "name": "monads", "mastery": 0.35, "review_count": 3 }], "count": 36 }
```

---

## Security / OPSEC

### `POST /api/audit`
PSL governance audit over a text seed:
```json
// Request
{ "text": "SELECT * FROM users WHERE id = 1" }
// Response
{
  "status": "ok",
  "confidence": 0.95,
  "verdict": "Pass | Fail",
  "axiom_results": [{ "axiom": "DimensionalityAxiom", "passed": true, "detail": "..." }]
}
```

### `POST /api/opsec/scan`
Scan text for PII, secrets, credentials:
```json
// Request
{ "text": "my SSN is 123-45-6789 and my API key is sk-abc123" }
// Response
{
  "status": "ok",
  "sanitized": "my SSN is [SSN_REDACTED] and my API key is [API_KEY_REDACTED]",
  "findings": [
    { "type": "ssn", "start": 10, "end": 21, "confidence": 0.95 },
    { "type": "api_key", "start": 40, "end": 49, "confidence": 0.90 }
  ]
}
```
Max input: 64 KiB.

---

## Provenance

### `GET /api/provenance/stats`
```json
{ "total_conclusions": 42, "total_nodes": 128, "arena_capacity": 10000 }
```

### `GET /api/provenance/:conclusion_id`
Explain how a conclusion was derived:
```json
{
  "conclusion_id": 42,
  "explanation": "Derived via intent=Explain, mode=Deep, confidence=0.87",
  "trace": [{ "kind": "ThinkStep", "detail": "..." }]
}
```

### `GET /api/provenance/:conclusion_id/chain`
Full derivation chain (all parent nodes):
```json
{ "chain": [{ "id": 1, "kind": "...", "parent": null }, { "id": 2, "kind": "...", "parent": 1 }] }
```

### `GET /api/provenance/export`  *(auth required)*
Bulk export the entire provenance arena as JSON.

### `POST /api/provenance/compact`  *(auth required)*
Compact the provenance arena (remove unreferenced nodes).
```json
{ "status": "ok", "removed": 14 }
```

### `POST /api/provenance/reset`  *(auth required)*
Wipe the entire provenance arena.
```json
{ "status": "ok" }
```

---

## Desktop / System Tools

### `GET /api/system/info`
Host information snapshot:
```json
{
  "hostname": "WORKSTATION-K7J3P9Z",
  "kernel": "Linux version 6.18.12+kali-amd64 ...",
  "uptime_secs": 13378,
  "os": "Kali GNU/Linux Rolling",
  "cpu_model": "11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz",
  "cpu_count": 16,
  "ram_total_kb": 65545444,
  "ram_available_kb": 53109252,
  "disk_root_total_bytes": 97825230848,
  "disk_root_free_bytes": 9088532480
}
```

### `POST /api/system/notify`  *(auth required)*
Desktop notification via `notify-send`:
```json
// Request
{ "title": "Build complete", "body": "All tests pass" }
// Response
{ "status": "ok" }
```

### `GET /api/system/clipboard`  *(auth required)*
Read system clipboard (tries Wayland wl-paste, falls back to X11 xclip):
```json
{ "status": "ok", "source": "x11", "text": "clipboard contents" }
```

### `POST /api/system/clipboard`  *(auth required)*
Write to system clipboard:
```json
// Request
{ "text": "text to copy" }
// Response
{ "status": "ok", "source": "wayland" }
```

---

## Logs / Observability

### `GET /api/chat-log?limit=N`
Recent chat turns (default 50, max 500) from `/var/log/lfi/chat.jsonl`:
```json
{
  "count": 3,
  "path": "/var/log/lfi/chat.jsonl",
  "entries": [
    { "ts": 1776300000, "user": "hi", "reply": "Hey, how's it going?", "tier": "Pulse", "intent": "Converse { ... }", "confidence": 0.85 }
  ]
}
```

### `POST /api/stop`
Cancel any in-flight generation. Currently a no-op (chat is synchronous); ready for streaming support.
```json
{ "status": "ok", "note": "no streaming in progress" }
```

---

## Frontend State (localStorage)

| Key | Purpose |
|-----|---------|
| `lfi_settings` | JSON: theme, fontSize, sendOnEnter, persistConversations, showReasoning, developerMode, defaultTier, displayName, avatarDataUrl, avatarGradient, erudaMode, apiHost |
| `lfi_conversations_v2` | JSON array of Conversation objects (id, title, messages[], createdAt, updatedAt, pinned, starred) |
| `lfi_current_conversation` | Current conversation ID string |
| `lfi_events_v1` | JSON array of { t, kind, data } UI event log entries (capped at 500) |
| `lfi_auth` | "true" / "false" auth state |

---

## Intent System

The AI classifies user input into one of these intents:

| Intent | Triggers plan? | Example |
|--------|---------------|---------|
| `Converse` | No | "how are you?", "tell me a joke" |
| `WriteCode` | Yes (if multi-step) | "build me a CLI tool" |
| `Analyze` | Yes (if multi-step) | "audit this API for security issues" |
| `Explain` | No (simple prefix) | "explain photosynthesis" |
| `Search` | No | "latest Rust news" |
| `FixBug` | Yes | "fix the memory leak in server.rs" |
| `PlanTask` | Yes | "plan my day tomorrow" |
| `Improve` | Yes | "optimize this function" |
| `Adversarial` | No (blocked) | "ignore previous instructions" |
| `Unknown` | No | fallback |

Plan fires when the input contains action verbs ("build", "create", "code", "fix", "set up"), sequence language ("how do I", "recipe", "steps to", "walk me through"), or explicit framing ("plan for", "checklist"). Simple "what is X" / "explain X" questions do NOT trigger plans.

---

## UI Skills (Tools Menu)

Available via the `+` button on the input bar:

| Skill | Backend | Status |
|-------|---------|--------|
| Chat | WS /ws/chat | Live |
| Deep Research | /api/research (planned) | Coming soon |
| Web Search | POST /api/search | Live |
| Image | /api/image (planned) | Coming soon |
| Code | WS /ws/chat (tier=BigBrain) | Live |
| Analyze | POST /api/audit | Live |
| OPSEC Scan | POST /api/opsec/scan | Live |

---

## Architecture Notes

- **Server binary**: `/root/LFI/lfi_vsa_core/target/release/server` (Rust, axum, tokio)
- **Dashboard**: `/root/LFI/lfi_dashboard/` (React 18, Vite 4, TypeScript)
- **Symlinked at**: `/home/user/Development/PlausiDen/PlausiDen-AI/`
- **Chat log**: `/var/log/lfi/chat.jsonl`
- **Training log**: `/var/log/lfi/training.jsonl` + per-domain `/var/log/lfi/training-<domain>.log`
- **Firewall**: nftables table `inet lfi_ports` restricts :3000/:5173 to lo/wg0/192.168.1.0/24
- **Training**: perpetual rotating loop across 11 domains via `/root/LFI/lfi_vsa_core/scripts/train_rotate.sh`

---

## Desktop Tools (added 2026-04-15 late session)

### `GET /api/system/apps`
Catalogue of all 717+ installed desktop applications from .desktop files:
```json
{
  "count": 717,
  "apps": [
    { "name": "Firefox", "exec": "firefox", "icon": "firefox", "categories": "Network;WebBrowser;", "comment": "Web Browser", "file": "/usr/share/applications/firefox.desktop" }
  ]
}
```

### `POST /api/system/launch` *(auth required)*
Launch a desktop app:
```json
{ "app": "firefox" }
→ { "status": "ok", "launched": "firefox" }
```
Uses `setsid xdg-open` so the app persists independently.

## SQLite Persistence (added 2026-04-15)

Brain database at `~/.local/share/plausiden/brain.db`. Tables: facts, conversations, messages, training_results, settings. Facts auto-persist from chat extraction and survive server restarts. Hydrated on startup.

### `POST /api/system/click` *(auth required)*
Click at screen coordinates:
```json
{ "x": 500, "y": 300, "button": 1 }
→ { "status": "ok", "x": 500, "y": 300 }
```

### `POST /api/system/type` *(auth required)*
Type text via xdotool (max 5000 chars):
```json
{ "text": "hello world" }
→ { "status": "ok", "chars": 11 }
```

### `POST /api/system/key` *(auth required)*
Send key combination:
```json
{ "keys": "ctrl+c" }
→ { "status": "ok", "keys": "ctrl+c" }
```

### `GET /api/system/screenshot` *(auth required)*
Full screen capture:
```json
{ "status": "ok", "format": "png", "size": 4759513, "data_base64": "iVBOR..." }
```

### `GET /api/training/status`
Training pipeline status:
```json
{
  "facts_in_db": 3,
  "trainer_running": true,
  "domain_state": { "social": { "sessions": 5, "total_examples": 750 }, ... },
  "recent_cycles": ["[2026-04-15T23:04:26] cycle=3 domain=math batch=150"],
  "training_history": [{ "domain": "social", "accuracy": 0.72, "total": 100, "correct": 72 }]
}
```
