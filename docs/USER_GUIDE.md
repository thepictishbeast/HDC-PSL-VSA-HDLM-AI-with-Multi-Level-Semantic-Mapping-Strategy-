# PlausiDen / LFI — User Training Guide

This is the hands-on guide for using and training LFI without anyone's help. Every feature is one click away once you know where to look.

## The 30-second picture

- **Chat** is the main surface. Type a question, hit Enter.
- **LFI only answers from its substrate.** If it doesn't know, it refuses honestly — "No HDC match in knowledge base for X — I won't fabricate" — instead of making something up.
- **You teach it.** Refusals are an invitation. Every refusal shows a yellow "Teach LFI" card with a one-click button that opens a form.
- **You can also teach it proactively.** Cmd/Ctrl+K → "Teach LFI a fact", or type `/teach` in the chat input.
- **It reviews what it learned** using FSRS spaced repetition. The Knowledge Browser surfaces due cards so you can rate them Again / Hard / Good / Easy.

Everything below is a deeper dive into each of these.

## Reading the UI

### Connection chip (sidebar, under your name)
- **Green "Online"** — WebSocket connected, frames arriving. Everything works.
- **Yellow "Stale"** — WS says open but no frames for 15s+. Usually LTE/WireGuard idle-kill; reconnect is about to fire.
- **Red "Offline"** — disconnected. Auto-reconnect with exponential backoff is running; your unsent messages are queued to localStorage and will replay when the socket comes back.
- **Yellow "· Ns" badge** next to chip — backend stats cache is older than 60s. Usually benign; means the 60s refresh loop is running slow.

### Assistant message treatments
- **Standard message** — LFI answered. Body is prose composed from substrate facts. Look for `[fact:KEY]` inline chips — click them to open a popover with ancestry, versions, contradictions, and translation entries.
- **Citation pills** — `(source: X, similarity N%)` chips mean the reply pulled from that source. Color tiers: green ≥80%, yellow ≥50%, red <50%.
- **Per-clause (N% certain) pills** — honesty markers from HDLM hedging. Green = strong grounding, yellow = medium, red = weak.
- **Yellow left border + REFUSAL pill** — LFI refused. The tooltip on the pill shows the reason (e.g. `no substrate match for "volcanoes"`). The dashed-yellow **Teach LFI** card below is your one-click teach-the-answer flow.
- **Nf · Ks footer** — number of facts and sources that message used. Click it (when the feature is live) to see exactly which ones.

### The input bar
- **Ctrl/Cmd+Enter** sends from any textarea in the app.
- **Shift+Enter** inserts a newline.
- **Shift+↑ / Shift+↓** walks your recent prompt history.
- **/** opens the slash-command menu. Useful commands: `/teach`, `/new`, `/clear`, `/theme`, `/settings`, `/logs`, `/export`.
- **Paste a URL** into an empty input → title chip appears above the textarea so you see what you're about to send.
- **Paste an image** → thumbnail strip appears above the input (backend upload not yet wired — metadata is logged).

### Cmd/Ctrl+K — command palette
The fastest way to reach any feature. Every UI entry point is in the palette. Top actions:
- **Teach LFI a fact** — first entry under Actions.
- **New chat / Clear chat / Duplicate conversation**
- **Go to Chat / Classroom / Fleet / Library / Auditorium**
- **Open Admin → Tokens / Proof / Diag / Docs**
- **Copy last reply's citations / Export diagnostic logs**
- **Resolve phrase → concept / Parse English → tuples / Verify fact by key**
- **Toggle dev mode / theme / sidebar**

## Teaching LFI — the three paths

Teaching LFI means adding something to the substrate that it will recall later. All three paths land in the same place — `/api/feedback` with `rating=correct` — which the ingestion pipeline picks up.

### 1. The Teach Modal (proactive)

**Open via:** Cmd/Ctrl+K → "Teach LFI a fact" · OR · `/teach` in the chat input.

Write a plain-English fact. The substrate extracts tuples automatically. Examples that work:

- `Water boils at 100°C at sea level.`
- `My dog's name is Maya.`
- `The Eiffel Tower was completed in 1889 and stands 330 metres tall.`
- `A volcano is a mountain with a vent for molten rock.`

Hit **Teach** (or Cmd/Ctrl+Enter) to submit. You'll see a toast: *Sent to LFI for ingestion*. The fact shows up in the Classroom feedback queue and gets folded into the substrate.

### 2. The inline refusal CTA (reactive)

When LFI refuses ("No HDC match in knowledge base for X"), a dashed-yellow card appears below the reply with a **Teach LFI** button. Click it → the correct-this modal opens pre-filled with your original question. Type the answer, hit Send.

This is the most productive training loop: ask → get refused → teach → ask again → now it knows.

### 3. The correct-this flow (fix a wrong answer)

Hover any assistant message (desktop) or long-press it (mobile). In the action bar:
- **👍 Thumbs up** — this answer was good. No follow-up; logs a positive.
- **👎 Thumbs down** — this answer was wrong. Opens a modal to tag a category + optional comment.
- **✎ Correct this** — opens a modal showing your original question + LFI's reply + a textarea for what it should have said. Submit ships a `rating=correct` to `/api/feedback`.

Use this when LFI said something confidently wrong. The correction enters the Classroom queue so the substrate learns from the discrepancy.

## Reviewing due cards (FSRS)

FSRS (Free Spaced Repetition Scheduler) is how LFI re-encounters what you taught it. Cards "come due" when the scheduler estimates retention will drop below target.

**Open via:** sidebar → Knowledge icon · OR · Cmd/Ctrl+K → "Open Knowledge browser".

The Knowledge Browser has three sections:
- **Due for review** — cards the scheduler surfaced. Each row shows the concept, mastery percentage, and how many days overdue. Rate with **Again** (failed) / **Hard** (slow) / **Good** (normal) / **Easy** (trivial). Pagination: 100 at a time with a "Show more" button.
- **Facts** — all facts currently in the substrate. Filterable.
- **Concepts** — higher-level clusters the substrate built from facts. Mastery percentages.

Reviewing is how LFI consolidates. A card you rate Again will reappear sooner; Easy pushes it further out.

## What LFI knows and doesn't know

Three views surface substrate state:

### Classroom (top-level view)
Twelve sub-tabs covering training state:
- **Student Profile** — grades, strengths, weaknesses.
- **Ingestion Control** — start/stop corpus runs.
- **Curriculum** — training datasets and sizes.
- **Gradebook** — pass/fail over time.
- **Lesson Plans** — active sessions.
- **Test Center** — benchmarks and quizzes.
- **Report Cards** — weekly progress.
- **Office Hours** — your own feedback queue (thumbs / corrections / teach events).
- **Library** — fact browser with per-source trust sliders.
- **Ledger** — contradictions the substrate flagged (two sources disagree ≥0.7 confidence each). Resolve with **Keep A**, **Keep B**, or **Dismiss**.
- **Drift** — 9 metric cards: fresh/stale, HDC cache coverage, contradictions pending, negative feedback rate, FSRS lapse rate, SPO-tuples, proof coverage, workspace fill. Trend arrows with polarity-aware coloring.
- **Ingest Runs** — history of ingestion runs with filters.

Cmd/Ctrl+K has **"Go to Ledger / Drift / Ingest Runs"** for direct jumps.

### Library (top-level view)
- **Sources** — the full inventory of sources the substrate has absorbed. Click a row to expand per-source quality dimensions.
- **Trust sliders** — adjust per-source trust (0–1), persisted via `/api/sources/trust`.
- **Marketplace top-10** — external sources ranked by quality.
- **Auto-resolve contradictions** button — applies the majority-trust rule to every pending contradiction at once.

### Admin → Docs tab
You're reading it.

## Settings you can tune

Open via Cmd/Ctrl+K → "Open settings" · OR · sidebar gear icon · OR · `/settings`.

Ten preferences persist to backend via `/api/settings`:
- **Theme** — dark / light / midnight / forest / sunset / contrast / rose
- **Font size** — small / medium / large / xlarge
- **Send on Enter** — off = Cmd+Enter only sends; on = Enter sends.
- **Show reasoning** — display internal reasoning steps on each answer.
- **Eruda mode** — mobile devtools: auto / on / off.
- **Developer mode** — shows telemetry, workstation ID, plan panel.
- **Default tier** — Pulse (fast) / Bridge / BigBrain. LFI has no transformer tiers today; this is vestigial.
- **Compact mode** — TUI-density for power users.
- **Auto theme** — follow OS prefers-color-scheme.
- **Notify on reply** — OS notification when LFI finishes while tab is hidden.

Workspace capacity (in Settings → Data) sets the substrate working-memory cap: 64 / 128 / 256 / 512 / 1024 / 2048 MB. Eviction-pressure warning fires above 75% fill.

## When something breaks

### Module load failed
Happens if the tab was open across a frontend redeploy — the chunk hash rotated. The app auto-retries 2× with backoff, then auto-reloads once. If you see the card anyway, click **Reload page** or **Try again**.

### Chat seems frozen
Look at the connection chip.
- **Red** — WS disconnected, reconnecting. Your unsent message is queued.
- **Yellow "Stale"** — no frames for 15s+. Backend may be processing; wait ~30s. If it stays yellow, a backend keepalive should kick in (every 25s).
- **Green but long wait** — open Admin → Diag tab. Each turn emits `turn-trace` entries: `send → first_frame → response → rendered`. The phase that's slow tells you where to look. Click **Copy diag log** to share the full buffer.

### A specific panel is broken
Every modal has a local error boundary. If it catches, you'll see a contained card inside the modal with the error + a Copy-diag-log button. The rest of the app stays usable — you can close the modal and keep going.

### Admin tab stuck on Loading
The tab auto-retries twice on failure with 1s then 3s backoff. If it still errors after that, the banner reads "HTTP 500 — backend returned an error. Is the route registered?" Check the server. Manual **Refresh** button in the Admin header re-triggers at any time.

### Back button does weird things
It shouldn't anymore — every modal and view is wired to the history API. Back closes the topmost modal, then traverses chat ↔ classroom ↔ fleet ↔ library ↔ auditorium.

## Exporting your data

- **Chat export** — `/export` slash command downloads all conversations as JSON.
- **Diagnostic log** — Cmd/Ctrl+K → "Export diagnostic logs" copies a JSON blob (last 500 entries) to your clipboard.
- **Substrate state** — Classroom has Copy-JSON buttons on Drift / Ledger / Runs / KB / Library / Activity.
- **All prefs** — `GET /api/settings` returns everything `ui_pref`-tagged on your profile.

## Keyboard-only operation

Every interactive element is reachable without a mouse:
- **Tab / Shift-Tab** — cycle focusable elements.
- **Arrow keys** — navigate tablists (Admin, Classroom), slash menu, palette results, due-card list.
- **Home / End** — first / last in a tablist.
- **Escape** — close topmost modal. Stack order: Shortcuts > Palette > Settings > KB > Activity > Admin > Training > Game > feedback modals > Terminal.
- **Cmd/Ctrl+K** — command palette (works from anywhere).
- **Cmd/Ctrl+N** — new chat.
- **Cmd/Ctrl+B** — toggle sidebar.
- **Cmd/Ctrl+1..5** — jump to views (Chat / Classroom / Admin / Fleet / Library).
- **Cmd/Ctrl+Shift+D** — toggle theme.
- **Shift+↑ / Shift+↓** — prompt history recall.

Focus traps: every modal traps Tab inside itself and restores focus to the launcher on close.

## Getting unstuck fast

1. **Cmd/Ctrl+K → "Export diagnostic logs"** — snapshot everything the UI captured.
2. **Admin → Diag tab** — live tail with level filter (debug/info/warn/error) + free-text search.
3. **Admin → Logs tab** — server-side chat log + UI event log.
4. **Admin → Proof tab** — verification status of facts (proved / rejected / unreachable / unknown).
5. **Admin → Dashboard → Integrity banner** — green if audit-chain verified, red if tampered. Click to expand recent entries.

If everything else fails, **reload the page** — all persistent state (conversations, settings, diag logs last 200) survives in localStorage.

## Quick reference — the five most-used actions

| Action                     | How                                                 |
|----------------------------|-----------------------------------------------------|
| Teach LFI a fact           | Cmd/Ctrl+K → "Teach LFI a fact", or `/teach`        |
| Ask a question             | Type in the chat bar, Enter to send                 |
| Correct a wrong answer     | Hover/long-press the message → ✎ Correct this       |
| Rate a due card (FSRS)     | Knowledge Browser → Again / Hard / Good / Easy      |
| Export session diagnostics | Cmd/Ctrl+K → "Export diagnostic logs"               |
