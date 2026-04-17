# Claude 2 — Frontend Session Log

Instance: claude-2 (frontend/UI). Session start: 2026-04-16 ~23:05 EDT. Date rolled to 2026-04-17 at midnight EDT.

## CHANGES

### c0-078 queue (all 10 items shipped)

1. **React key collisions** — added `msgId()` generator in App.tsx; replaced 19 `id: Date.now()` and 4 `const toolId = Date.now()` with calls to it. No more duplicate-key warnings.
2. **Stats wiring** — `/api/status` 5s poll no longer reads phantom `data.axiom_count`; tracks `sources_count`; 8s AbortController timeout. Replaced hardcoded `'97.2%/392K/52M+/168'` literals in training dashboard with real values from `/api/admin/training/accuracy`.
3. **Training progress viz** — rewrote `TrainingDashboardContent` to `Promise.allSettled` across 3 admin endpoints; partial-failure tolerant; PSL progress bar with threshold tint; per-domain heatmap (fact-share × quality → alpha); "LIVE" badge on recently-trained domains.
4. **CSS pulse on active trainer** — `@keyframes lfi-trainer-pulse`; respects `prefers-reduced-motion`.
5. **QoS / PSL pass rate** — sidebar Status panel got three live rows (PSL Pass, Adversarial, Sources) with color thresholds and `(stale)` suffix.
6. **Skeleton loaders** — `Skeleton` component + `@keyframes lfi-shimmer`; first-load state on training dashboard renders shape-matching placeholders.
7. **Sidebar animation** — existing width/transform transitions verified intact (pre-existing REGRESSION-GUARD).
8. **Error boundary** — `AppErrorBoundary` class component; `<AppRoot>` default export wraps `<SovereignCommandConsole>`.
9. **Decomposition** — see file table below.
10. **Start/Stop training** — POST `/api/admin/training/{start,stop}` with 10s timeout, disabled state, inline error surface.

### Beyond the queue
- Live/stale/offline pill on Substrate Telemetry header (green ≤20s, yellow 20–90s, red >90s/offline).
- `/api/quality/report` 30s poll with stale flag + 12s abort.
- Facts, QoS, and chat-log panels surface explicit error/empty states (red cards with auth-hint when applicable) instead of silently hiding "0 results".
- Disk-pressure alert banner (yellow ≥90%, red ≥95%) — currently showing because /root was ~92–93% for most of the session.
- "Most recent cycle" live banner on training dashboard, parsed from `recent_training_log`.
- Auto-detect `prefers-color-scheme` for first-visit theme.
- Browser tab title reflects active conversation.
- WebSocket reconnect: fixed 3s → exponential 1–30s + jitter (chat), 2–60s + jitter (telemetry); resets on `onopen`.
- Code splitting via `React.lazy` for 5 modals (Settings, CommandPalette, KnowledgeBrowser, ActivityModal, TicTacToeModal) under a top-level `<Suspense>`.
- Training dashboard Refresh button shows in-flight state.
- PSL percent auto-detect (fraction vs percent) — prevented a "9720%" display bug.
- Compact-JSON emission for IPC bus — discovered `ipc_watcher.sh` regex only matches compact form, so 34 of my Python-emitted status messages were invisible to Claude 0's watcher.

## DECISIONS

- **Kept inline styles + `C` palette everywhere** — no CSS-in-JS migration. Matches existing convention and avoids churn. Design tokens from `design.ts` remain available but underused; promoting them would double-diff every file.
- **Extracted components take opaque `C: any`** — rather than typing to the full palette shape. Keeps prop interfaces terse; palette refactors don't cascade.
- **Parent holds state, components stay pure** — e.g. `ToolMessage` takes `{expanded, onToggle}` rather than owning a Set. Works for all five message variants.
- **No backend touches.** All flagged backend issues (empty /api/facts, auth-gated /api/chat-log, phantom /api/status.axiom_count, intermittent /api/quality/report body) reported on bus; not in my lane.
- **No git push initiated.** `/tmp/claude-ipc/backup_checkpoint.sh` handles commits + push attempts (fire-and-forget); push pipeline appears broken (10+ commits ahead of origin at one point) — flagged to Claude 0.

## RISKS

- **No end-to-end browser test.** I can't open the dashboard in a real browser from this shell. Vite returning HTTP 200 on each source file means the code parses, not that it runs correctly. Runtime logic errors could remain hidden. Recommend user does an eyeball pass.
- **MessageBubble `AssistantMessage`** has 18 props — biggest single prop blob of any extracted component. Refactor risk if/when we add new action-bar buttons.
- **`const C = DARK` shadow** — App.tsx resolves `C` via `THEMES[settings.theme]` inside the component, but the module-level `const C = DARK` is still referenced by `AppErrorBoundary`'s theme fallbacks. Works today; if a future component references `C` at module level during init it'll get DARK regardless of user pref. Not actively broken.
- **Lazy-loading error surface** — `<Suspense fallback={null}>` means a network-fail during chunk load surfaces as "modal doesn't appear." Should probably wrap with an error boundary that shows a retry. Future work.
- **IPC bus is append-only + unbounded** — grows forever. `/tmp/claude-ipc/c2_check.sh` acks messages by writing ack stubs; doesn't compact the bus. Rotation is someone else's concern.

## NEXT (when session resumes)

1. Hook `AssistantMessage` feedback buttons to a backend endpoint (currently just `logEvent` to localStorage).
2. Virtualize message list for long conversations (Phase 2 #1) — need `react-virtuoso` dep, pre-approval.
3. Extract `renderSidebar` (~230 lines) — many state deps; delayed because the simpler wins were higher ROI.
4. Add an error boundary around the lazy modal `<Suspense>` so chunk-load failures are recoverable.
5. Move remaining inline DOM helpers (`scrollToBottom`, `logEvent`) into util — low priority.

## FILES (15 total in `lfi_dashboard/src/`, 6200+ LOC)

| File | Lines | Purpose |
|---|---|---|
| App.tsx | 3458 | Main container, state, WS, routing |
| MessageBubble.tsx | 436 | System/Web/Tool/User/Assistant message variants |
| TrainingDashboard.tsx | 429 | Admin/training panel with live banner + heatmap |
| SettingsModal.tsx | 422 | 4-tab settings (profile/appearance/behavior/data) |
| design.ts | 406 | Design-token reference (unchanged, underused) |
| ActivityModal.tsx | 226 | 3-tab activity log modal |
| themes.ts | 160 | 7 palette exports + THEMES record |
| CommandPalette.tsx | 143 | Cmd+K palette |
| KnowledgeBrowser.tsx | 138 | Facts/concepts/due modal |
| LoginScreen.tsx | 93 | Sovereign-key gate |
| AppErrorBoundary.tsx | 74 | React error boundary |
| TicTacToeModal.tsx | 70 | Easter-egg game |
| WelcomeScreen.tsx | 62 | Empty-state + quick-starts |
| catalogs.tsx | 56 | SKILLS + AVATAR_PRESETS |
| util.ts | 36 | compactNum, formatRam, formatTime, copyToClipboard |
| main.tsx | 9 | Entry |

App.tsx session delta: 4675 → 3458 (-1217, 26% reduction) despite adding >20 features.

## CHECKPOINTS (`working-*` tags)

Latest few: working-20260417_002508, _002351, _002255, _002200, _002040, _001845, _001752, _001554, _001306, _001246, _001038, _000902, _000846, _000717, _000602, _000454, _000250, _000123, _000014, 20260416_234902, _234732, _234503, _234342, _233910, _233838, _233552.

## BUS

36+ messages on `/tmp/claude-ipc/bus.jsonl` from `claude-2`. Most are per-cycle status updates. c0-099 was a status-check; c2-037 diagnosed the format-mismatch bug that made most of my earlier messages invisible to their watcher.
