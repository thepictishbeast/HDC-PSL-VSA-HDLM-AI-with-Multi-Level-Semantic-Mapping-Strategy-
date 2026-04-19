// ============================================================
// Sovereign Command Console (SCC) v4.0 — Production Dashboard
//
// PROTOCOL: Real-time WebSocket integration with LFI Cognitive Core
// SUBSTRATE: React, inline styles + CSS media queries (no framework)
// LAYOUT: Mobile-first, responsive to tablet and desktop
//
// BREAKPOINTS:
//   Mobile:  < 768px  — Single column, collapsible panels
//   Tablet:  768-1199 — Wider chat, collapsible telemetry
//   Desktop: >= 1200  — Persistent telemetry sidebar, wide chat
//
// ENDPOINTS:
//   ws://<host>:3000/ws/chat       — Bidirectional cognitive chat
//   ws://<host>:3000/ws/telemetry  — Real-time substrate telemetry
//   POST /api/auth                 — Sovereign key verification
//   POST /api/tier                 — Model tier switching
//   GET  /api/status               — Substrate status
//   GET  /api/facts                — Knowledge facts
//   GET  /api/qos                  — QoS compliance report
//
// DEBUG: console.debug() on every state change for Eruda inspector
// FIX: Eruda FAB positioned to avoid input bar overlap
// ============================================================

import React, { useState, useEffect, useRef, useCallback, useDeferredValue } from 'react';
// c2-228 / #79: language grammars are loaded on demand by markdown.tsx via
// hljsLazy.ts so each lives in its own Vite chunk; only the theme CSS
// (required before any highlighted HTML renders) ships in the initial bundle.
import 'highlight.js/styles/github-dark.css';
// c2-388 / BIG #179: GroupedVirtuoso for the sidebar conversation list —
// renders only visible rows so the DOM stays O(viewport) instead of
// O(conversations). Pinned + day-bucket clusters map 1:1 to Virtuoso
// groups, which gives us native sticky headers for free.
import { GroupedVirtuoso } from 'react-virtuoso';
import { compactNum, formatRam, formatTime, copyToClipboard, diskPressure, smartTitle, exportConversationMd, exportConversationPdf, exportConversationTxt, exportAllAsJson, formatRelative, formatDayBucket, mod, modKey, stripMarkdown, hapticTick, flashMessageById } from './util';
// c2-433: diagnostic logger with auto-capture of console.warn/error + window
// error events. Installed once on mount. Exposed on window.diag for devtools.
import { diag } from './diag';
import { markSend, markFirstFrame, markResponse, markRendered, type TurnTrace } from './turnTrace';
import { useHistoryDialog } from './useHistoryDialog';
import { useModalFocus } from './useModalFocus';
import { TourOverlay, type TourStep } from './TourOverlay';
import { usePageVisible } from './usePageVisible';
import { useToastQueue } from './useToastQueue';
import { useFeedbackModals } from './useFeedbackModals';
import { useChatSearch } from './useChatSearch';
import { useThinkingState } from './useThinkingState';
import { useSlashMenu } from './useSlashMenu';
import { useConvoDrag } from './useConvoDrag';
import { useMessageEdit } from './useMessageEdit';
import { useChatStreaming } from './useChatStreaming';
// c2-433: TrainingDashboardContent only renders inside the showTraining
// modal — lazy so the chat-only paint doesn't pay the chart bytes.
import { lazyWithRetry } from './lazyWithRetry';
const TrainingDashboardContent = lazyWithRetry(() => import('./TrainingDashboard').then(m => ({ default: m.TrainingDashboardContent })), 'TrainingDashboard');
import { AppErrorBoundary } from './AppErrorBoundary';
// c2-433: LoginScreen only renders on the unauth path (passwordless mode
// keeps isAuthenticated=true by default). Lazy so the common case skips it.
const LoginScreen = lazyWithRetry(() => import('./LoginScreen').then(m => ({ default: m.LoginScreen })), 'LoginScreen');
import { SKILLS, AVATAR_PRESETS, type Skill as CatalogSkill } from './catalogs';
import { SystemMessage, WebMessage, ToolMessage, UserMessage, AssistantMessage } from './MessageBubble';
// Code-splitting: the overlays below are only rendered on user action, so we
// load their code on demand. Cuts the initial JS bundle by ~1000 lines of TSX.
import { type CmdPaletteItem } from './CommandPalette';
import { DARK, THEMES } from './themes';
import { T } from './tokens';
// c2-433: WelcomeScreen lazy — only renders when there are zero messages
// in the current convo. First-paint of an existing convo doesn't pay for it.
const WelcomeScreen = lazyWithRetry(() => import('./WelcomeScreen').then(m => ({ default: m.WelcomeScreen })), 'WelcomeScreen');
// c2-433: 4 telemetry-sidebar panels are desktop-sidebar-only. Mobile +
// non-developer-mode visits don't render them — lazy so the initial paint
// doesn't pay for the DataTable + chart bytes they pull in. They render
// inside renderSidebar() which is gated on isDesktop, and the parent
// React.Suspense boundary catches the chunk loads.
const FactsPanel = lazyWithRetry(() => import('./FactsPanel').then(m => ({ default: m.FactsPanel })), 'FactsPanel');
const QosPanel = lazyWithRetry(() => import('./QosPanel').then(m => ({ default: m.QosPanel })), 'QosPanel');
const DomainsPanel = lazyWithRetry(() => import('./DomainsPanel').then(m => ({ default: m.DomainsPanel })), 'DomainsPanel');
const AccuracyPanel = lazyWithRetry(() => import('./AccuracyPanel').then(m => ({ default: m.AccuracyPanel })), 'AccuracyPanel');
// Full-screen admin console (c0-017). Lazy because it bundles 6 tabs of
// panels that are only seen when the user clicks the Admin entry.
const AdminModal = lazyWithRetry(() => import('./AdminModal').then(m => ({ default: m.AdminModal })), 'AdminModal');
import type { AdminTab } from './AdminModal';
// Classroom full page (c0-027). Lazy — not visited until user switches view.
const ClassroomView = lazyWithRetry(() => import('./ClassroomView').then(m => ({ default: m.ClassroomView })), 'ClassroomView');
// c0-037 #2 / c2-328: dedicated Fleet page. Lazy so the orchestrator SDK
// payload doesn't bloat the initial chat bundle.
const FleetView = lazyWithRetry(() => import('./FleetView').then(m => ({ default: m.FleetView })), 'FleetView');
// c0-037 #3 / c2-329: dedicated Library page for the 365-source inventory.
const LibraryView = lazyWithRetry(() => import('./LibraryView').then(m => ({ default: m.LibraryView })), 'LibraryView');
// c0-037 #12 / c2-331: Auditorium — AVP-2 audit state surface.
const AuditoriumView = lazyWithRetry(() => import('./AuditoriumView').then(m => ({ default: m.AuditoriumView })), 'AuditoriumView');
import { TelemetryCard } from './TelemetryCards';
// c2-433: 3 more telemetry-sidebar components, lazy for the same reason
// as the panels above. All render inside renderSidebar() (isDesktop only).
const SidebarStatus = lazyWithRetry(() => import('./SidebarStatus').then(m => ({ default: m.SidebarStatus })), 'SidebarStatus');
const SubstrateTelemetry = lazyWithRetry(() => import('./SubstrateTelemetry').then(m => ({ default: m.SubstrateTelemetry })), 'SubstrateTelemetry');
const AdminActions = lazyWithRetry(() => import('./AdminActions').then(m => ({ default: m.AdminActions })), 'AdminActions');
import { renderMessageBody as renderMdBody, type MarkdownCtx } from './markdown';
import { useTicTacToe } from './useTicTacToe';
import { useStatusPoll, useQualityPoll, useSysInfoPoll } from './usePolls';
import { ChatView, type ChatViewHandle } from './ChatView';
const ShortcutsModal = lazyWithRetry(() => import('./ShortcutsModal').then(m => ({ default: m.ShortcutsModal })), 'ShortcutsModal');

const TicTacToeModal = lazyWithRetry(() => import('./TicTacToeModal').then(m => ({ default: m.TicTacToeModal })), 'TicTacToeModal');
// c2-356 / task #67: in-browser xterm.js terminal. Lazy because xterm is
// ~200 KB and most sessions never open it.
const XTermModal = lazyWithRetry(() => import('./XTermModal').then(m => ({ default: m.XTermModal })), 'XTermModal');
const KnowledgeBrowser = lazyWithRetry(() => import('./KnowledgeBrowser').then(m => ({ default: m.KnowledgeBrowser })), 'KnowledgeBrowser');
import type { KnowledgeDue } from './KnowledgeBrowser';
const ActivityModal = lazyWithRetry(() => import('./ActivityModal').then(m => ({ default: m.ActivityModal })), 'ActivityModal');
const CommandPalette = lazyWithRetry(() => import('./CommandPalette').then(m => ({ default: m.CommandPalette })), 'CommandPalette');
const SettingsModal = lazyWithRetry(() => import('./SettingsModal').then(m => ({ default: m.SettingsModal })), 'SettingsModal');

// ---- Responsive hook ----
type Breakpoint = 'mobile' | 'tablet' | 'desktop';

function useBreakpoint(): Breakpoint {
  const [bp, setBp] = useState<Breakpoint>(() => {
    if (typeof window === 'undefined') return 'mobile';
    const w = window.innerWidth;
    if (w >= 1200) return 'desktop';
    if (w >= 768) return 'tablet';
    return 'mobile';
  });

  useEffect(() => {
    const onResize = () => {
      const w = window.innerWidth;
      const next: Breakpoint = w >= 1200 ? 'desktop' : w >= 768 ? 'tablet' : 'mobile';
      setBp(prev => {
        if (prev !== next) {
          console.debug("// SCC: Breakpoint changed:", prev, "->", next, "width:", w);
          return next;
        }
        return prev;
      });
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  return bp;
}

// id generator. Date.now() alone collided when >1 msg arrived/ms → React duplicate-key warnings.
let __msgSeq = 0, __msgLastMs = 0;
const msgId = (): number => {
  const now = Date.now();
  if (now === __msgLastMs) __msgSeq = (__msgSeq + 1) & 0x3ff;
  else { __msgLastMs = now; __msgSeq = 0; }
  return now * 1024 + __msgSeq;
};

// ---- Types ----
interface ChatMessage {
  id: number;
  role: 'user' | 'assistant' | 'system' | 'web' | 'tool';
  content: string;
  mode?: string;
  confidence?: number;
  tier?: string;
  intent?: string;
  reasoning?: string[];
  plan?: { steps: number; complexity: number; goal: string };
  timestamp: number;
  // Tool-call rendering (Claude Code style)
  toolName?: string;
  toolStatus?: 'running' | 'ok' | 'error';
  toolInput?: string;
  toolOutput?: string;
  toolDuration?: number;
  conclusion_id?: number;
}

interface SubstrateStats {
  ram_available_mb: number;
  ram_total_mb?: number;
  ram_used_mb?: number;
  cpu_temp_c: number;
  vsa_orthogonality: number;
  axiom_pass_rate: number;
  is_throttled: boolean;
  logic_density: number;
}

interface QosReport {
  passed: boolean;
  critical_failures: number;
  warnings: number;
  checks: { name: string; passed: boolean; value: string; threshold: string; severity: string }[];
}

// ---- Color palettes (rebuilt from scratch 2026-04-15) ----
// Dark: near-black slate with a subtle indigo hue, peach accent that reads
// warm against the cool background — a palette closer to Linear / Vercel than
// the stock "blue-on-black" terminal vibe the old one had.
// Light: Claude.ai's cream/bone aesthetic — warm off-white, ink text, the
// same peach accent so the identity carries across themes.

// ---- Main Component ----
const SovereignCommandConsole: React.FC = () => {
  const bp = useBreakpoint();
  const isDesktop = bp === 'desktop';
  const isTablet = bp === 'tablet';
  const isMobile = bp === 'mobile';
  console.debug("// SCC v4.0: Component mounting, breakpoint:", bp);

  // ---- State ----
  // Passwordless mode: the API doesn't gate any route on authentication, so the
  // login screen was purely cosmetic. Default to authenticated; the login flow
  // and key handling stay in place for future re-enablement.
  const [isAuthenticated, setIsAuthenticated] = useState(true);
  const [password, setPassword] = useState('');
  const [authError, setAuthError] = useState('');
  const [authLoading, setAuthLoading] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  // c2-230 / #71: images pasted into the chat input sit here until the next
  // send. Kept as an ephemeral preview buffer — not persisted, not inlined
  // into message content (data URLs would blow out localStorage). Backend
  // upload is tracked separately; for now we log the paste and summarize.
  const [pastedImages, setPastedImages] = useState<{ id: string; dataUrl: string; size: number; type: string }[]>([]);
  // #187: URL-paste title preview. Fires a background /api/unfurl fetch on
  // URL paste; chip above input renders domain → title when resolved.
  // null when no URL pasted or user dismissed.
  const [urlPreview, setUrlPreview] = useState<{ url: string; title: string | null; loading: boolean; error: string | null } | null>(null);
  // c2-370 / task 84: drag-and-drop file upload overlay state. True while a
  // drag is in progress over the chat pane; drives the dashed-border hint.
  const [isDraggingFile, setIsDraggingFile] = useState(false);
  // c2-371 / task 79: set when the last assistant turn errored out -- lets
  // us render an inline Retry affordance that resends the prior user
  // message. Cleared on successful next send or manual dismiss.
  // c2-433 / #313 pass 8: chat streaming + last-error-retry state lifted
  // into useChatStreaming. Hook owns the 500ms tick interval (only running
  // when a stream is active) + chars-per-second derivation.
  const cstr = useChatStreaming();
  const lastErrorRetry = cstr.lastError;
  const setLastErrorRetry = cstr.setLastError;
  // c2-387 / BIG #176: pending branch marker. Set by onCommitEdit immediately
  // before the resend; the next user-message append inside handleSend stamps
  // _branchedFromId onto the new bubble so the UI can render a "Branch" tag.
  // Cleared in handleSend after being consumed so subsequent normal sends
  // don't inherit the flag.
  const pendingBranchFromRef = useRef<number | null>(null);
  // c2-372 / task 105 / c2-433 #313 pass 8: streaming throughput tracker
  // moved into useChatStreaming. cstr.timing carries startAt + chars; the
  // tick interval lives in the hook. Local aliases below preserve existing
  // call-site shape (setStreamTiming, streamTimingTick) so the surrounding
  // chunk handler + render path don't have to change.
  const streamTiming = cstr.timing;
  const streamTimingTick = cstr.tick;
  const [isConnected, setIsConnected] = useState(false);
  // c2-433 / task 236+253: substrate health stats from /api/health (concepts +
  // axioms) + chat-throughput from /api/metrics (lfi_chat_total counter).
  // Both polled in the same 30s cycle to amortize network overhead. null =
  // haven't fetched yet; missing fields stay 0. Chip is hidden until we
  // have a confirmed payload so the UI doesn't render zero-value chrome.
  const [substrateStats, setSubstrateStats] = useState<{ concepts: number; axioms: number; chatTotal: number } | null>(null);
  // claude-0 #403: age of the backend-cached stats snapshot (seconds). >60s
  // means the background refresh task stalled — counts may be stale.
  const [statsAgeSecs, setStatsAgeSecs] = useState<number | null>(null);
  // c2-433 / #298: pending contradiction count from /api/contradictions/recent.
  // Feeds a small red badge on the Classroom tab button + a tooltip line so
  // the user can see at a glance that the ledger has unresolved disagreements
  // ready for triage. null = never-loaded (hide badge), 0 = loaded-empty
  // (also hide), >0 = badge with compact number.
  const [contradictionsPending, setContradictionsPending] = useState<number | null>(null);
  // c2-433: unseen diag error counter. Subscribes to diag on mount; every
  // new error entry bumps the counter. Clicking Admin (any tab) resets
  // it since the operator is now looking. Drives a small red dot badge
  // on the Admin tab button so ops see "something broke" before they
  // open Admin → Diag.
  const [diagUnseenErrors, setDiagUnseenErrors] = useState<number>(0);
  // c2-433 / #298 followup: rise-detection ref + pulse state. When the
  // latest poll returns a count strictly greater than the previous count,
  // we bump badgePulseId to trigger a 3s CSS scale animation on the
  // badge — tiny but genuinely catches the eye when a new contradiction
  // lands while the operator is on another tab.
  const prevContradictionsRef = useRef<number | null>(null);
  const [contradictionsPulseId, setContradictionsPulseId] = useState<number>(0);
  // c2-433 / #256: HDC encode-cache coverage from /api/hdc/cache/stats.
  // {sample_cached, sample_size, coverage 0..1}. Appended to the substrate
  // chip as "cache N%" so users can glance-see whether recent queries are
  // hitting cached encodings (coverage=0 means every query re-encodes).
  const [hdcCache, setHdcCache] = useState<{ coverage: number; sample_cached: number; sample_size: number } | null>(null);
  // c2-433 / #316 / #300: pre-send dry-run of the chat pipeline via
  // /api/explain. Debounced ~450ms on input typing so we don't hammer the
  // endpoint on every keystroke; returns { speech_act, extracted_concept,
  // rag_top_facts, causal_preview, topic_stack, gate_verdicts } which we
  // render as predicted-module chips above the input. Null = no preview
  // yet (input too short or offline).
  const [explainPreview, setExplainPreview] = useState<null | {
    speech_act?: string;
    extracted_concept?: string;
    rag_top_facts?: any[];
    causal_preview?: any;
    topic_stack?: any;
    gate_verdicts?: any;
    [k: string]: any;
  }>(null);
  // c2-433 / #307: when /api/explain returns 429, stash the Retry-After
  // deadline here. The effect skips fetches until Date.now() passes the
  // deadline; the preview row shows a "rate limited · Ns" chip with a
  // 1s ticker counting down. null = no active rate-limit.
  const [explainRateLimitUntil, setExplainRateLimitUntil] = useState<number | null>(null);
  const [explainRateLimitTick, setExplainRateLimitTick] = useState<number>(0);
  // Debounced disconnect banner — avoid flashing the banner on the initial
  // pre-connect moment or on momentary reconnects under 2s.
  const [showDisconnectBanner, setShowDisconnectBanner] = useState(false);
  // c2-254 / #116: when the chat WS is in the backoff window, track the
  // absolute wall time of the next attempt. Banner shows "reconnecting in Ns"
  // based on wsReconnectAt - Date.now(). Null when a connect is in-flight
  // or the socket is healthy.
  const [wsReconnectAt, setWsReconnectAt] = useState<number | null>(null);
  // Banner countdown ticker. Bumping wsTick every 500ms forces the banner's
  // inline JSX to re-render so the "in Ns" text decrements. Ticker only
  // runs while wsReconnectAt is set — no wasted wakeups when connected.
  const [, setWsTick] = useState(0);
  // Distinguishes "WS dropped (reconnecting)" from "backend is fully offline"
  // (probe to /api/status fails too). Lets the disconnect banner show a
  // different, more actionable message when the dev server is down.
  const [backendOffline, setBackendOffline] = useState(false);
  // Network-level offline state (navigator.onLine). Distinct from server
  // disconnect: if the user's WiFi drops, no point blaming the backend.
  const [networkOffline, setNetworkOffline] = useState<boolean>(() =>
    typeof navigator !== 'undefined' && navigator.onLine === false
  );
  // Ephemeral toast (copy feedback, etc). Single-slot — newer toasts replace.
  // `exiting` decouples the display-done moment from the DOM unmount so we
  // can run an exit animation before removing the node.
  // `onUndo` populates an Undo button inside the toast (soft-delete flow).
  // c2-242 / #103 / c2-433 #313: toast queue lifted into useToastQueue hook.
  // Renderer reads `toasts`; producers call `showToast(msg, onUndo?)`; click-
  // to-dismiss calls `dismiss(id)`. setToasts kept exposed for the (rare)
  // mutation paths that still touch the array directly (exiting flag flips,
  // bulk delete on hash change). The two-phase auto-dismiss + per-id
  // schedule guard now lives in the hook, off App.tsx.
  const { toasts, showToast, dismiss: dismissToast } = useToastQueue();
  // c2-397 / task 200: global Cmd+Z undo for the last delete. Written by
  // deleteConversation alongside its toast-undo button; cleared after the
  // toast hold window so a stale Cmd+Z doesn't resurrect an old entry.
  const pendingUndoRef = useRef<{ fn: () => void; at: number } | null>(null);
  // c2-410 / task 206: backup of the most recent input clear so Cmd+Z in
  // the textarea can restore it. Populated by the clearInputWithBackup
  // helper (send path + slash commands) — not by natural typing, since
  // the browser's native undo handles that.
  const draftBackupRef = useRef<{ text: string; at: number } | null>(null);
  // c2-433 / task 250+251: prompt history ring buffer. Last 10 sent inputs;
  // Shift+ArrowUp walks back, Shift+ArrowDown walks forward (or clears at
  // the recent end). Capacity 10 — beyond that and users should use
  // chat-search to find the prompt they want. Cursor -1 means "not
  // navigating" (next ArrowUp lands at the most recent entry). Persisted
  // to localStorage so it survives reloads — read once at mount, written
  // each time handleSend pushes a new entry.
  const PROMPT_HISTORY_LS_KEY = 'lfi_prompt_history_v1';
  const promptHistoryRef = useRef<string[]>((() => {
    try {
      const raw = localStorage.getItem(PROMPT_HISTORY_LS_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((s): s is string => typeof s === 'string' && s.length > 0).slice(-10);
    } catch { return []; }
  })());
  const promptHistoryCursorRef = useRef<number>(-1);
  // c2-433 / task 243: force a re-render after handleSend writes
  // draftBackupRef so the Restore chip appears. Bumped by clearInputWithBackup
  // when a non-empty draft is captured.
  const [draftBackupTick, setDraftBackupTick] = useState<number>(0);
  const clearInputWithBackup = (current: string) => {
    if (current.trim()) {
      draftBackupRef.current = { text: current, at: Date.now() };
      setDraftBackupTick(t => t + 1);
    }
    setInput('');
  };
  // c2-433 / task 252c: programmatic setInput + auto-grow + cursor-end.
  // setInput from React doesn't fire the textarea's onChange, so the height
  // auto-grow + cursor placement that handleInputChange does must be done
  // manually anywhere we recall a value (prompt history, restore-prompt
  // pill, legacy lastUser fallback). Single helper avoids drift.
  const setInputAndResize = (value: string) => {
    setInput(value);
    setTimeout(() => {
      const el = inputRef.current;
      if (!el) return;
      el.style.height = 'auto';
      el.style.height = (value.length === 0 ? '' : Math.min(el.scrollHeight, 280) + 'px');
      el.selectionStart = el.selectionEnd = el.value.length;
    }, 0);
  };
  const restoreDraftBackup = () => {
    const b = draftBackupRef.current;
    if (!b) return;
    setInputAndResize(b.text);
    inputRef.current?.focus();
    draftBackupRef.current = null;
    setDraftBackupTick(t => t + 1);
  };
  // c2-400 / task 185: floating right-click menu over chat messages. Role
  // disambiguates which action set to render. Closes on outside click + Esc
  // via the escape-hatch handler that's already wired for modals, plus a
  // dedicated outside-click listener below.
  const [msgContextMenu, setMsgContextMenu] = useState<{ x: number; y: number; msgId: number; role: 'user' | 'assistant'; content: string } | null>(null);
  // Brief visual pulse on the input container when a message is sent (c0-020
  // "visual feedback on send"). Tracked as a bumping id so consecutive sends
  // retrigger the animation cleanly.
  const [sendPulseId, setSendPulseId] = useState(0);
  // c2-433 / #313: feedback modal state lifted into useFeedbackModals.
  // Negative modal carries category+text drafts; Correct modal carries a
  // single correction-text draft + the AI reply for context display. The
  // hook gives us scoped open/close callbacks so callers don't have to
  // reset draft fields by hand.
  const fb = useFeedbackModals();
  const negFeedbackFor = fb.negFeedbackFor;
  const negFeedbackCategory = fb.negFeedbackCategory;
  const negFeedbackText = fb.negFeedbackText;
  const setNegFeedbackCategory = fb.setNegFeedbackCategory;
  const setNegFeedbackText = fb.setNegFeedbackText;
  const correctFeedbackFor = fb.correctFeedbackFor;
  const correctFeedbackText = fb.correctFeedbackText;
  const setCorrectFeedbackText = fb.setCorrectFeedbackText;
  // c2-433 / #317: fact-key inspection popover. Anchored at click position;
  // fetched lazily once on open. data shape is whatever /api/facts/:key
  // returns — the popover renders known fields (subj/pred/obj/source/PSL/
  // trust/temporal_class) and falls back to JSON for unknown shapes.
  const [factPopover, setFactPopover] = useState<{
    key: string; x: number; y: number;
    data: any | null; error: string | null; loading: boolean;
  } | null>(null);
  // c2-433 / task 240: toggle between structured-fields view and raw JSON
  // inside the fact popover. Flag is reset whenever a new popover opens
  // (different key = different shape, want to start in the friendly view).
  const [factPopoverRaw, setFactPopoverRaw] = useState<boolean>(false);
  // c2-433 / #337 followup: in-flight FSRS review from inside the fact
  // popover. Boolean is sufficient because only one popover is open at a
  // time. Disables all 4 rating buttons during POST so a double-click
  // can't fire two grades for the same card.
  const [factReviewing, setFactReviewing] = useState<boolean>(false);
  // c2-433 / #354: "Verify now" button state. True while POST
  // /api/proof/verify is in-flight so the button disables + shows
  // wait cursor.
  const [factVerifying, setFactVerifying] = useState<boolean>(false);
  // c2-433 / #354 followup: 1.5s flash when proof_hash is click-copied.
  const [copiedProofHash, setCopiedProofHash] = useState<string | null>(null);
  // c2-433 / #274 followup: add-translation inline form state. When expanded,
  // the popover footer shows a small language + text form. Saving POSTs
  // /api/concepts/link and refreshes the Translations section.
  const [factLinkOpen, setFactLinkOpen] = useState<boolean>(false);
  const [factLinkLang, setFactLinkLang] = useState<string>('en');
  const [factLinkText, setFactLinkText] = useState<string>('');
  const [factLinkSaving, setFactLinkSaving] = useState<boolean>(false);
  const [factLinkErr, setFactLinkErr] = useState<string | null>(null);
  // Tracks whether the chat is scrolled to the latest message. False = user
  // is reading history; we surface a "scroll to bottom" affordance.
  const [chatAtBottom, setChatAtBottom] = useState(true);
  // c2-433 / task 249b: track messages.length at the moment the user
  // scrolled away from the bottom — drives the "+N new" badge on the
  // scroll-to-bottom FAB. Reset when the user scrolls back to bottom OR
  // clicks the FAB. Tracks length, not message-id, so chat_chunk in-place
  // growth doesn't inflate the count (only fully-new messages do).
  const scrollAwayLengthRef = useRef<number | null>(null);
  const chatViewRef = useRef<ChatViewHandle>(null);
  // Index of the topmost-visible message in Virtuoso. Drives the floating
  // day-header pinned at the top of the chat pane.
  const [chatTopIndex, setChatTopIndex] = useState(0);
  // c2-433 / #313 pass 3: chat search state + helpers lifted into
  // useChatSearch. Hook owns query/show/mode/cursor + the input ref +
  // open/close/toggle that maintain the focus + reset invariants.
  const cs = useChatSearch();
  const chatSearch = cs.query;
  const showChatSearch = cs.show;
  const chatSearchMode = cs.mode;
  const chatSearchCursor = cs.cursor;
  const chatSearchInputRef = cs.inputRef;
  const setChatSearch = cs.setQuery;
  const setChatSearchMode = cs.setMode;
  const setChatSearchCursor = cs.setCursor;
  // c2-433 / #313 pass 4: thinking lifecycle (isThinking + thinkingStart +
  // thinkingStep + thinkingElapsed + activeModule + modulesUsed) lifted into
  // useThinkingState. Hook owns the elapsed-tick interval + start/stop/reset
  // invariants. Setter passthroughs (setIsThinking, setThinkingStep,
  // setActiveModule, setThinkingStart) kept exposed for the WS handler that
  // pokes individual fields mid-stream. recordModule(name) updates active +
  // adds to modulesUsed Set.
  const ts = useThinkingState();
  const isThinking = ts.isThinking;
  const thinkingStart = ts.thinkingStart;
  const thinkingStep = ts.thinkingStep;
  const thinkingElapsed = ts.thinkingElapsed;
  const activeModule = ts.activeModule;
  const modulesUsed = ts.modulesUsed;
  const setIsThinking = ts.setIsThinking;
  const setThinkingStart = ts.setThinkingStart;
  const setThinkingStep = ts.setThinkingStep;
  const setActiveModule = ts.setActiveModule;
  // c2-433 / task 255: mirror modulesUsed into a ref so the WS chat_done
  // handler (closure captured at WS-effect setup, deps:[isAuthenticated])
  // can read the current set instead of the empty initial value. Without
  // this mirror the chat_modules_used log event would never fire because
  // the closure only sees the modulesUsed at WS-setup time.
  const modulesUsedRef = useRef<Set<string>>(modulesUsed);
  useEffect(() => { modulesUsedRef.current = modulesUsed; }, [modulesUsed]);
  // c2-433 / task 259: same closure-staleness pattern for isThinking — used
  // by ws.onclose to decide whether the WS death interrupted an active
  // stream (in which case we set lastError so the Retry pill shows).
  const isThinkingRef = useRef<boolean>(isThinking);
  useEffect(() => { isThinkingRef.current = isThinking; }, [isThinking]);
  // c2-433 / #352: topic context for multi-turn pronoun resolution. Backend
  // (post topic_stack ship) emits the active topic on chat_progress; UI
  // surfaces it as a chip so users see what 'them' / 'it' will resolve to.
  // null = no topic yet (fresh session or backend not emitting).
  const [activeTopic, setActiveTopic] = useState<string | null>(null);
  // c2-433 / task 259: mirror messages so ws.onclose can find the last
  // user-turn for the retry-pill content without closing over stale state.
  const messagesRef = useRef<typeof messages>([]);
  useEffect(() => { messagesRef.current = messages; }, [messages]);
  // c2-433 / task 260: currentConversationIdRef mirror is set up AFTER the
  // useState for currentConversationId (below line 2204). This comment
  // stays here so future refactors don't re-introduce the ref up here and
  // recreate the TDZ bug ('Cannot access ke before initialization').
  // c2-433 / task 261: settingsRef + conversationsRef moved to AFTER their
  // respective useState declarations (~line 592 + ~2244). Placing the ref
  // above the state caused a production TDZ ('Cannot access ge before
  // initialization') identical to the currentConversationIdRef bug above.
  // Search for "task 261 mirror" below for the actual setup.
  const [expandedReasoning, setExpandedReasoning] = useState<number | null>(null);
  const [showTelemetry, setShowTelemetry] = useState(false);
  const [showAdmin, setShowAdmin] = useState(false);
  const [currentTier, setCurrentTier] = useState<string>(() => {
    try {
      const raw = localStorage.getItem('lfi_settings');
      if (raw) {
        const s = JSON.parse(raw);
        if (s?.defaultTier) return s.defaultTier;
      }
    } catch {}
    return 'Pulse';
  });
  const [tierSwitching, setTierSwitching] = useState(false);
  const [facts, setFacts] = useState<{ key: string; value: string }[]>([]);
  const [qosReport, setQosReport] = useState<QosReport | null>(null);
  const [adminLoading, setAdminLoading] = useState('');
  const [stats, setStats] = useState<SubstrateStats>({
    ram_available_mb: 0, cpu_temp_c: 0, vsa_orthogonality: 0.02,
    axiom_pass_rate: 1.0, is_throttled: false, logic_density: 0
  });

  // Knowledge-graph counters, quality metrics, and host info all come from the
  // polling hooks defined later (useStatusPoll / useQualityPoll / useSysInfoPoll).
  // Nothing else ever writes to these, so no local state is needed.

  // c2-419 / c2-501: preload chunks during browser idle so first open feels
  // instant. Tier 1 only (palette, shortcuts, settings — ~12 KB gzipped
  // total). Heavier destinations (Admin ~18KB, Classroom ~22KB, Activity,
  // KB) stay pay-on-open — background-prefetching them was competing
  // with the user's actual navigation on slow networks. User reported
  // "extremely long load" → cutting Tier 2 removes ~300KB from the idle
  // bandwidth race. lazyWithRetry still covers stale-chunk recovery.
  // Also skip preload entirely on save-data or 2G connections.
  useEffect(() => {
    const conn: any = (navigator as any).connection;
    if (conn?.saveData) return;
    if (conn?.effectiveType === 'slow-2g' || conn?.effectiveType === '2g') return;
    const tier1 = () => {
      import('./CommandPalette');
      import('./ShortcutsModal');
      import('./SettingsModal');
    };
    const ric: any = (window as any).requestIdleCallback;
    if (typeof ric === 'function') {
      const id1 = ric(tier1, { timeout: 4000 });
      return () => { (window as any).cancelIdleCallback?.(id1); };
    }
    const id1 = window.setTimeout(tier1, 1500);
    return () => { window.clearTimeout(id1); };
  }, []);

  // Persistent settings (localStorage-backed). A single object keeps storage
  // compact and makes future additions one-line.
  type Settings = {
    theme: 'dark' | 'light' | 'midnight' | 'forest' | 'sunset' | 'contrast' | 'rose';
    fontSize: 'small' | 'medium' | 'large' | 'xlarge';
    sendOnEnter: boolean;
    persistConversations: boolean;
    showReasoning: boolean;
    apiHost: string;
    displayName: string;
    avatarDataUrl: string;
    avatarGradient: string;
    erudaMode: 'auto' | 'on' | 'off';
    developerMode: boolean;        // Gate telemetry, workstation ID, PLAN reasoning
    defaultTier: 'Pulse' | 'Bridge' | 'BigBrain'; // Persistent model default
    compactMode: boolean;          // TUI-density mode for power users
    autoTheme: boolean;            // Follow OS prefers-color-scheme dynamically
    notifyOnReply: boolean;        // OS notification when AI finishes while tab hidden
    customTheme: {
      bg: string; accent: string; text: string; green: string; red: string;
    } | null;
  };
  const defaultSettings: Settings = {
    theme: 'dark', fontSize: 'medium', sendOnEnter: true,
    persistConversations: true, showReasoning: false, apiHost: '',
    displayName: 'User',
    avatarDataUrl: '',
    avatarGradient: 'linear-gradient(135deg, #8b7bf7, #a88dff)',
    erudaMode: 'auto',
    developerMode: false,   // Telemetry + PLAN hidden by default
    defaultTier: 'Pulse',   // Persistent model default the user controls in Settings
    compactMode: false,
    autoTheme: false,
    notifyOnReply: false,
    customTheme: null,
  };
  const [settings, setSettings] = useState<Settings>(() => {
    try {
      const raw = localStorage.getItem('lfi_settings');
      if (raw) return { ...defaultSettings, ...JSON.parse(raw) };
      // First visit — honor the OS-level color-scheme preference. Users can
      // still switch themes in Settings; this only picks the initial default.
      const prefersLight = typeof window !== 'undefined'
        && window.matchMedia?.('(prefers-color-scheme: light)').matches;
      return { ...defaultSettings, theme: prefersLight ? 'light' : 'dark' };
    } catch { return defaultSettings; }
  });
  // c2-433 / task 261 mirror: settingsRef is read by the WS chat_done handler
  // (OS-notification block) which closes over this function's scope at WS
  // setup. Without a ref, toggling notifyOnReply mid-session wouldn't take
  // effect. Must be declared AFTER the useState to avoid TDZ — previously
  // at line ~506 this crashed prod with 'Cannot access ge before
  // initialization'.
  const settingsRef = useRef(settings);
  useEffect(() => { settingsRef.current = settings; }, [settings]);
  // c0-011 #9 + claude-0 11:15 fix: sync persistable prefs to backend via
  // POST /api/settings {key, value}. Debounced 500ms so multi-setting
  // changes batch. Last-synced ref per key prevents redundant writes.
  // Whitelist of server-persistable settings only — free-text like
  // displayName stays client-side.
  const lastSyncedRef = useRef<Partial<Record<keyof Settings, any>>>({});
  useEffect(() => {
    const persist: Array<keyof Settings> = ['theme', 'fontSize', 'sendOnEnter', 'showReasoning',
      'erudaMode', 'developerMode', 'defaultTier', 'compactMode', 'autoTheme', 'notifyOnReply'];
    const id = window.setTimeout(() => {
      for (const key of persist) {
        const value = settings[key];
        if (lastSyncedRef.current[key] === value) continue;
        lastSyncedRef.current[key] = value;
        fetch(`http://${getHost()}:3000/api/settings`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key, value }),
        }).catch(() => { /* non-fatal — settings stay in localStorage either way */ });
      }
    }, 500);
    return () => window.clearTimeout(id);
  }, [settings.theme, settings.fontSize, settings.sendOnEnter, settings.showReasoning,
      settings.erudaMode, settings.developerMode, settings.defaultTier, settings.compactMode,
      settings.autoTheme, settings.notifyOnReply]);

  useEffect(() => {
    try { localStorage.setItem('lfi_settings', JSON.stringify(settings)); } catch {}
    // Runtime Eruda sync: if the setting changes while the app is open, show
    // or hide immediately without needing a page reload.
    try {
      const er: any = (window as any).eruda;
      if (!er) return;
      const isMobile = /Mobi|Android|iPhone|iPad|iPod/i.test(navigator.userAgent);
      const should =
        settings.erudaMode === 'on' ||
        (settings.erudaMode === 'auto' && isMobile);
      if (should && !er._isInit) { er.init(); }
      else if (!should && er._isInit) { er.destroy?.(); }
    } catch {}
  }, [settings]);
  const [showSettings, setShowSettings] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [settingsTab, setSettingsTab] = useState<'profile' | 'appearance' | 'behavior' | 'data'>('profile');

  // Active skill/tool for the next message (like Perplexity Focus, Gemini Extensions,
  // Claude Code tool routing). Real backends wired: chat (WS), web (api/search),
  // analyze (api/audit), opsec (api/opsec/scan). Image/research/code stubbed until
  // backend support lands; clicking the chip makes that clear.
  type Skill = CatalogSkill;
  const [activeSkill, setActiveSkill] = useState<Skill>('chat');
  const [showSkillMenu, setShowSkillMenu] = useState(false);
  // c2-433 / #313 pass 5: slash-menu state lifted into useSlashMenu. Hook
  // bundles the open-resets-index + filter-resets-index invariants. Call
  // sites use sm.open/close/moveUp/moveDown directly — the older bare-setter
  // shape isn't reconstructed.
  const sm = useSlashMenu();
  const showSlashMenu = sm.show;
  const slashFilter = sm.filter;
  const slashIndex = sm.index;

  type SlashCmd = { cmd: string; label: string; desc: string; run: () => void };
  const slashCommands: SlashCmd[] = [
    { cmd: '/teach', label: 'Teach LFI', desc: 'Add a fact to the substrate',
      run: () => setShowTeach(true) },
    { cmd: '/guide', label: 'User guide', desc: 'Hands-on training guide (opens Admin → Docs)',
      run: () => { setAdminInitialTab('docs'); setShowAdmin(true); } },
    { cmd: '/new', label: 'New chat', desc: 'Start a fresh conversation',
      run: () => createNewConversation() },
    { cmd: '/clear', label: 'Clear chat', desc: 'Erase current messages',
      run: () => clearChat() },
    { cmd: '/theme', label: 'Toggle theme', desc: 'Switch dark / light',
      run: () => setSettings(s => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' })) },
    { cmd: '/settings', label: 'Open settings', desc: 'All preferences',
      run: () => setShowSettings(true) },
    { cmd: '/logs', label: 'Activity logs', desc: 'Chat log + UI events',
      run: () => { setAdminInitialTab('logs'); setShowAdmin(true); fetchChatLog(50); } },
    // c2-428 / #339 pivot: /pulse /bridge /bigbrain slash commands removed.
    // LFI is post-LLM (HDC/VSA/PSL/HDLM) — no transformer tier applies.
    { cmd: '/web', label: 'Web Search', desc: 'Search the internet',
      run: () => { setActiveSkill('web'); } },
    { cmd: '/code', label: 'Code mode', desc: 'BigBrain + code focus',
      run: () => { setActiveSkill('code'); } },
    { cmd: '/analyze', label: 'Analyze', desc: 'PSL-supervised audit',
      run: () => { setActiveSkill('analyze'); } },
    { cmd: '/opsec', label: 'OPSEC Scan', desc: 'Scan for secrets / PII',
      run: () => { setActiveSkill('opsec'); } },
    { cmd: '/dev', label: 'Toggle dev mode', desc: 'Show telemetry + plan panel',
      run: () => setSettings(s => ({ ...s, developerMode: !s.developerMode })) },
    { cmd: '/sidebar', label: 'Toggle sidebar', desc: 'Show / hide conversations',
      run: () => setShowConvoSidebar(v => !v) },
    { cmd: '/export', label: 'Export conversations', desc: 'Download as JSON',
      run: () => {
        try {
          const blob = new Blob([JSON.stringify(conversations, null, 2)], { type: 'application/json' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url; a.download = `plausiden-conversations.json`;
          document.body.appendChild(a); a.click(); a.remove();
          URL.revokeObjectURL(url);
        } catch {}
      } },
    // c2-401 / task 194: clone the active conversation so the user can
    // explore an alternate path without losing the original.
    { cmd: '/duplicate', label: 'Duplicate conversation', desc: 'Clone the current conversation',
      run: () => {
        if (!currentConversationId) { showToast('No active conversation'); return; }
        duplicateConversation(currentConversationId);
      } },
    // c2-395 / task 197: plain-text export for the current conversation.
    // Useful for analysis tools that prefer untagged prose.
    { cmd: '/export-txt', label: 'Export as plain text', desc: 'Current conversation as .txt',
      run: () => {
        const c = conversations.find(cc => cc.id === currentConversationId);
        if (!c) { showToast('No active conversation'); return; }
        try {
          exportConversationTxt(c);
          logEvent('conversation_exported_txt', { id: c.id });
          showToast('Exported .txt');
        } catch { showToast('Export failed'); }
      } },
    { cmd: '/compact', label: 'Toggle compact mode', desc: 'Dense TUI-style layout for power users',
      run: () => setSettings(s => ({ ...s, compactMode: !s.compactMode })) },
    { cmd: '/training', label: 'Training dashboard', desc: 'View training status, domain stats, and pipeline health',
      run: () => { setShowTraining(true); } },
    { cmd: '/incognito', label: 'Incognito chat', desc: 'Start a private chat that won\'t be saved or logged',
      run: () => createNewConversation(true) },
    { cmd: '/knowledge', label: 'Knowledge browser', desc: 'Browse facts, concepts, and reviews',
      run: () => { setShowKnowledge(true); fetchKnowledge(); } },
    { cmd: '/game', label: 'Play a game', desc: 'Tic-tac-toe vs the AI',
      run: () => { setShowGame('tictactoe'); tttReset(); } },
    // c2-356 / task #67: in-browser xterm.js terminal.
    { cmd: '/terminal', label: 'Terminal', desc: 'Open the in-browser terminal (xterm.js, client-local)',
      run: () => { setShowTerminal(true); } },
    // c2-251 / #113: natural coverage for features already in the app but
    // not reachable via slash.
    { cmd: '/search', label: 'Search this chat', desc: 'Open the in-conversation search bar',
      run: () => { cs.open(); } },
    { cmd: '/shortcuts', label: 'Keyboard shortcuts', desc: 'Open the cheatsheet (also: ?)',
      run: () => { setShowShortcuts(true); } },
    { cmd: '/admin', label: 'Admin console', desc: 'Dashboard, domains, system, fleet, logs',
      run: () => { setShowAdmin(true); } },
    { cmd: '/classroom', label: 'Classroom', desc: 'Training, grades, datasets',
      run: () => { setShowAdmin(false); setActiveView('classroom'); } },
    // c2-391 / task 217: /time — prints local + UTC + server uptime as a
    // system message. Useful for cross-timezone debugging without leaving
    // the chat. Uses the already-available /api/status payload (fire + forget).
    { cmd: '/time', label: 'Time check', desc: 'Local + UTC + server uptime',
      run: () => {
        const now = new Date();
        const local = now.toLocaleString();
        const utc = now.toISOString().replace('T', ' ').slice(0, 19) + ' UTC';
        const offsetMin = -now.getTimezoneOffset();
        const offsetStr = (offsetMin >= 0 ? '+' : '-') + String(Math.abs(Math.floor(offsetMin / 60))).padStart(2, '0') + ':' + String(Math.abs(offsetMin % 60)).padStart(2, '0');
        const append = (body: string) => setMessages(prev => [...prev, {
          id: msgId(), role: 'system', content: body, timestamp: Date.now(),
        }]);
        append(`**Time check**\n- Local: ${local} (UTC${offsetStr})\n- UTC:   ${utc}\n- Server uptime: querying…`);
        // Fetch status to append uptime. If it fails, leave the "querying..."
        // placeholder — users can see the backend is offline separately.
        fetch(`http://${getHost()}:3000/api/status`).then(r => r.json()).then(s => {
          const secs = Number(s?.uptime_seconds ?? 0);
          if (!isFinite(secs) || secs <= 0) return;
          const d = Math.floor(secs / 86400), h = Math.floor((secs % 86400) / 3600), m = Math.floor((secs % 3600) / 60);
          const uptimeStr = d > 0 ? `${d}d ${h}h ${m}m` : h > 0 ? `${h}h ${m}m` : `${m}m`;
          append(`Server uptime: ${uptimeStr}`);
        }).catch(() => { /* silent; user already has local+UTC */ });
      } },
    { cmd: '/fleet', label: 'Fleet', desc: 'Orchestrator: instances, tasks, timeline',
      run: () => { setShowAdmin(false); setActiveView('fleet'); } },
    { cmd: '/library', label: 'Library', desc: 'Source inventory with quality + vetted status',
      run: () => { setShowAdmin(false); setActiveView('library'); } },
    { cmd: '/auditorium', label: 'Auditorium', desc: 'AVP-2 audit state, pass history, findings',
      run: () => { setShowAdmin(false); setActiveView('auditorium'); } },
    { cmd: '/help', label: 'Help & docs', desc: 'Commands, shortcuts, tips, and feedback guide',
      run: () => {
        const cmdList = slashCommands.filter(c => c.cmd !== '/help').map(c => `  ${c.cmd.padEnd(14)} ${c.desc}`).join('\n');
        // c2-297: render shortcuts with the platform-correct modifier and
        // cover the chords added in the sidebar/a11y cycles.
        const m = mod();
        const help = `**PlausiDen AI — Quick Reference**

**Slash commands** (type / in the input):
${cmdList}

**Keyboard shortcuts:**
  ${m}+K            Command palette
  ${m}+N            New conversation
  ${m}+B            Toggle sidebar
  ${m}+1 / 2 / 3    Chat / Classroom / Admin
  ${m}+F            Search in chat
  ${m}+Shift+F      Search (overrides browser find)
  ${m}+,            Settings
  ${m}+D            Toggle developer mode
  ${m}+Shift+D      Cycle themes
  ${m}+Shift+K      Knowledge browser
  ?                Keyboard cheatsheet
  Esc              Close any modal

**On a focused sidebar row:** ↑/↓ move, Enter open, P pin, S star, F2 rename, Del delete.

**How to give feedback:**
  Thumbs up/down on any AI response — hover to see them on the right.
  Thumbs down asks "what should it have said?" — your correction goes into the training pipeline.

**How to teach the AI:**
  Just tell it things naturally: "my name is X", "I'm a developer", "I love hiking."
  It auto-extracts facts and remembers them across sessions (stored in brain.db).
  Use /knowledge to browse what it knows.

**Tools:** Click the + button on the input bar to access Web Search, Code, Analyze, and OPSEC Scan.

**Privacy:** Your data never leaves this machine. Telemetry is OFF by default. Use /incognito for conversations that aren't even saved locally.

**Website:** plausiden.com
**Architecture:** Built on the Supersociety stack — HDC, PSL, Active Inference, Rust.`;
        setMessages(prev => [...prev, { id: msgId(), role: 'system', content: help, timestamp: Date.now() }]);
      } },
  ];
  const [showCmdPalette, setShowCmdPalette] = useState(false);
  const [showGame, setShowGame] = useState<null | 'tictactoe' | 'twenty_questions'>(null);
  // c2-356 / task #67: in-browser terminal toggle.
  const [showTerminal, setShowTerminal] = useState(false);
  // Tool confirmation — per Bible §3.5. First web search per session requires
  // explicit approval; after that auto-approved.
  const [webSearchApproved, setWebSearchApproved] = useState(false);
  const [pendingConfirm, setPendingConfirm] = useState<{ tool: string; desc: string; onApprove: () => void } | null>(null);
  const [showWelcome, setShowWelcome] = useState(() => {
    try { return !localStorage.getItem('lfi_welcomed'); } catch { return true; }
  });
  const [tosAccepted, setTosAccepted] = useState(() => {
    try { return localStorage.getItem('lfi_tos_accepted') === 'true'; } catch { return false; }
  });
  const dismissWelcome = () => {
    setShowWelcome(false);
    try { localStorage.setItem('lfi_welcomed', 'true'); } catch {}
    // c2-312: audit trail parity with tos_accepted. Useful for first-run
    // telemetry when we wire a local analytics view later.
    logEvent('welcome_dismissed', {});
  };
  const [showKnowledge, setShowKnowledge] = useState(false);
  // Direct Teach-LFI modal (user directive: 'must be able to train LFI
  // proactively'). Standalone from the refusal-flow Teach CTA so users
  // can add facts without first hitting a refusal.
  const [showTeach, setShowTeach] = useState(false);
  // #352 interactive tour state. Persists 'seen' flag so first-time users
  // auto-trigger once, but existing users don't get interrupted.
  const [showTour, setShowTour] = useState<boolean>(false);
  useEffect(() => {
    // Don't fire while login screen is up — the tour targets main-app
    // surfaces. Also private-mode browsers return '' from localStorage
    // (not '1'), so without isAuthenticated gating the tour would auto-
    // launch every session.
    if (!isAuthenticated) return;
    let seen = '1';
    try { seen = localStorage.getItem('lfi_tour_seen_v1') || ''; } catch { /* silent */ }
    if (seen === '1') return;
    // Delay 1.5s to let the first-paint settle before introducing the
    // overlay — otherwise the spotlight lands on an element that's still
    // mounting + jumping.
    const id = window.setTimeout(() => {
      diag.info('tour', 'first-visit autolaunch');
      setShowTour(true);
    }, 1500);
    return () => window.clearTimeout(id);
  }, [isAuthenticated]);
  const [teachText, setTeachText] = useState('');
  const [teachSending, setTeachSending] = useState(false);
  const teachDialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(showTeach, teachDialogRef);
  const [showTraining, setShowTraining] = useState(false);
  const [trainingLog, setTrainingLog] = useState<Array<{ ts: string; domain: string; batch: number; sessions: number }>>([]);
  const fetchTrainingLog = async () => {
    try {
      const res = await fetch(`http://${getHost()}:3000/api/chat-log?limit=1`);
      // Use training.jsonl directly — parse last 50 lines
      // For now, show what we can from the training state file
      const stateRes = await fetch(`http://${getHost()}:3000/api/system/info`);
      const sysInfo = await stateRes.json();
      // Parse training.jsonl via a quick fetch of the log endpoint
      // (we don't have a dedicated training endpoint yet, so we'll show what's available)
      setTrainingLog([]);
    } catch {}
  };
  // State for items inside .map() — can't use useState inside a map callback
  // (React hooks rules violation). Track expanded/editing IDs instead.
  const [expandedTools, setExpandedTools] = useState<Set<number>>(new Set());
  // c2-433 / #313 pass 7: user-message edit-in-place state lifted into
  // useMessageEdit. Hook bundles begin/cancel/commit lifecycle.
  const me = useMessageEdit();
  const editingMsgId = me.editingId;
  const editText = me.draft;
  const setEditText = me.setDraft;
  const [knowledgeFacts, setKnowledgeFacts] = useState<Array<{ key: string; value: string }>>([]);
  const [knowledgeConcepts, setKnowledgeConcepts] = useState<Array<{ name: string; mastery: number; review_count: number }>>([]);
  const [knowledgeDue, setKnowledgeDue] = useState<Array<{ name: string; mastery: number; days_overdue: number; fact_key?: string }>>([]);
  // c2-433 / #337 followup: FSRS top-level meta from /api/fsrs/due response
  // — {due_cards: N, target_retention: 0.9}. Rendered as a subtle header
  // line above the "Due for review" block so users see the target they're
  // grading toward. Null when the endpoint was the legacy SM-2 path.
  const [knowledgeFsrsMeta, setKnowledgeFsrsMeta] = useState<{ due_cards?: number; target_retention?: number } | null>(null);
  const [knowledgeLoading, setKnowledgeLoading] = useState(false);
  const [knowledgeError, setKnowledgeError] = useState<string | null>(null);
  // c2-433 / task 273: stale-while-revalidate the knowledge fetch. Track
  // when we last successfully populated the lists; on reopen-within-window
  // show the cached data immediately + skip the loading flag (background
  // refresh fires anyway). 60s window matches the human "I just glanced
  // at this" intuition without ever serving deeply stale data.
  const knowledgeLastFetchedRef = useRef<number>(0);
  const fetchKnowledge = async () => {
    const host = getHost();
    const haveCache = knowledgeLastFetchedRef.current > 0;
    const fresh = haveCache && (Date.now() - knowledgeLastFetchedRef.current) < 60_000;
    // Skip the loading-flag flash when serving fresh-enough cache; still
    // refresh in the background.
    if (!fresh) setKnowledgeLoading(true);
    setKnowledgeError(null);
    try {
      // c2-433 / #337: FSRS scheduler endpoint is now primary for due cards.
      // Falls back to legacy SM-2 /api/knowledge/due when FSRS isn't
      // available. Payload-shape tolerant: FSRS rows carry fact_key +
      // retrievability/stability; we normalize to {name, mastery,
      // days_overdue, fact_key}. If fact_key is absent (legacy), the
      // rating-buttons row stays hidden in the browser component.
      const [f, c, d] = await Promise.all([
        fetch(`http://${host}:3000/api/facts`).then(r => r.json()),
        fetch(`http://${host}:3000/api/knowledge/concepts`).then(r => r.json()),
        (async () => {
          try {
            const r = await fetch(`http://${host}:3000/api/fsrs/due?limit=50&target_r=0.9`);
            if (!r.ok) throw new Error(`HTTP ${r.status}`);
            return { __fsrs: true, ...(await r.json()) };
          } catch {
            return await fetch(`http://${host}:3000/api/knowledge/due`).then(r => r.json());
          }
        })(),
      ]);
      setKnowledgeFacts(f.facts || []);
      setKnowledgeConcepts(c.concepts || []);
      // FSRS payload likely: { due: [{ fact_key, name?, retrievability?,
      // stability?, difficulty?, days_overdue?, due? }] } — normalize.
      const rawDue: any[] = Array.isArray(d?.due) ? d.due : Array.isArray(d?.cards) ? d.cards : Array.isArray(d) ? d : [];
      const nowMs = Date.now();
      const normDue = rawDue.map((row: any): KnowledgeDue => {
        const fact_key: string | undefined = row.fact_key || row.key || undefined;
        const name: string = row.name || row.concept || row.fact_key || row.key || '(unnamed)';
        let mastery: number = 0;
        if (typeof row.retrievability === 'number') mastery = row.retrievability;
        else if (typeof row.mastery === 'number') mastery = row.mastery;
        else if (typeof row.stability === 'number') mastery = Math.min(1, row.stability / 30);
        let days_overdue: number = 0;
        if (typeof row.days_overdue === 'number') days_overdue = row.days_overdue;
        else if (row.due) {
          const dueMs = typeof row.due === 'string' ? Date.parse(row.due) : (typeof row.due === 'number' ? row.due : NaN);
          if (!Number.isNaN(dueMs)) days_overdue = Math.max(0, (nowMs - dueMs) / 86_400_000);
        }
        return { name, mastery, days_overdue, fact_key };
      });
      setKnowledgeDue(normDue);
      // c2-433 / #337 followup: capture FSRS meta fields on the envelope —
      // due_cards (total count; may exceed the limit-50 slice we render)
      // and target_retention (0..1). Only set when the response came from
      // the FSRS branch AND at least one meta field is present.
      if (d && d.__fsrs) {
        const dc = typeof d.due_cards === 'number' ? d.due_cards : undefined;
        const tr = typeof d.target_retention === 'number' ? d.target_retention : undefined;
        setKnowledgeFsrsMeta(dc != null || tr != null ? { due_cards: dc, target_retention: tr } : null);
      } else {
        setKnowledgeFsrsMeta(null);
      }
      knowledgeLastFetchedRef.current = Date.now();
    } catch (e) {
      console.warn('knowledge fetch failed', e);
      setKnowledgeError((e as Error).message || 'Network error — is the backend reachable?');
    } finally {
      setKnowledgeLoading(false);
    }
  };
  // Tic-tac-toe state
  const { board: tttBoard, winner: tttWinner, play: tttPlay, reset: tttReset } = useTicTacToe();
  const [cmdQuery, setCmdQuery] = useState('');
  const [cmdIndex, setCmdIndex] = useState(0);
  const skills = SKILLS;
  const activeSkillMeta = skills.find(s => s.id === activeSkill) || skills[0];
  const [showHistory, setShowHistory] = useState(false);
  const [showActivity, setShowActivity] = useState(false);
  // Which AdminModal tab to open when the user triggers Admin via a
  // dedicated link (e.g., Activity menu → Logs). Reset to 'dashboard' on
  // close so normal Admin opens default to Dashboard.
  // c2-261: use AdminTab so this can't drift when new tabs are added
  // (was previously a narrow inline union missing 'inventory').
  const [adminInitialTab, setAdminInitialTab] = useState<AdminTab>('dashboard');
  // c2-433 / #312 + #284 followup: Classroom deep-link. Cmd+K entries for
  // Ledger / Drift / Runs bump this so ClassroomView opens on the right
  // sub-tab. Keyed with a nonce (tick) so re-clicking the same entry
  // re-activates it even if the sub was manually changed in-between.
  const [classroomInitialSub, setClassroomInitialSub] = useState<{ sub: string; tick: number } | null>(null);
  const openClassroomSub = (sub: string) => {
    setActiveView('classroom');
    setShowAdmin(false);
    setClassroomInitialSub(prev => ({ sub, tick: (prev?.tick ?? 0) + 1 }));
  };

  const avatarPresets = AVATAR_PRESETS;
  const [showAccountMenu, setShowAccountMenu] = useState(false);
  const accountMenuRef = useRef<HTMLDivElement>(null);
  const [serverChatLog, setServerChatLog] = useState<any[]>([]);
  const [activityTab, setActivityTab] = useState<'chat' | 'events' | 'system'>('chat');
  const [localEvents, setLocalEvents] = useState<Array<{ t: number; kind: string; data?: any }>>([]);

  // c2-398 / task 198: added xlarge step (1.35x) for a11y users needing
  // the big bump without OS-level zoom. Small=0.88 / Medium=1.0 / Large=1.15.
  const fontScale = settings.compactMode ? 0.85
    : settings.fontSize === 'small' ? 0.88
    : settings.fontSize === 'large' ? 1.15
    : settings.fontSize === 'xlarge' ? 1.35
    : 1.0;

  // Announce new assistant messages + tool completions to screen readers via a
  // visually-hidden aria-live region. Tracks last assistant id so we only speak
  // once per new message (not on every re-render).
  const [srAnnouncement, setSrAnnouncement] = useState('');
  const lastAnnouncedIdRef = useRef<number | null>(null);
  useEffect(() => {
    const lastAssistant = [...messages].reverse().find(m => m.role === 'assistant' && !(m as any)._streaming);
    if (lastAssistant && lastAssistant.id !== lastAnnouncedIdRef.current) {
      lastAnnouncedIdRef.current = lastAssistant.id;
      const preview = lastAssistant.content.slice(0, 80).replace(/\s+/g, ' ').trim();
      setSrAnnouncement(`AI responded: ${preview}${lastAssistant.content.length > 80 ? '…' : ''}`);
    }
  }, [messages]);
  // Shadow the module-scope C with a theme-bound palette, plus any custom overrides.
  // When autoTheme is on, override settings.theme with the OS preference
  // (dark or light only). Other explicit picks (midnight/forest/etc.) only
  // apply when auto-mode is off.
  const [osPrefersLight, setOsPrefersLight] = useState<boolean>(() =>
    typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: light)').matches
  );
  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    const handler = (e: MediaQueryListEvent) => setOsPrefersLight(e.matches);
    if (mq.addEventListener) mq.addEventListener('change', handler);
    else mq.addListener(handler);
    return () => {
      if (mq.removeEventListener) mq.removeEventListener('change', handler);
      else mq.removeListener(handler);
    };
  }, []);
  // Preview theme: when the user hovers a theme card in Settings we flip
  // the dashboard to that theme briefly so they can see the result before
  // committing. null = no preview, fall through to the persisted choice.
  const [previewTheme, setPreviewTheme] = useState<string | null>(null);
  const effectiveThemeKey = previewTheme
    ?? (settings.autoTheme ? (osPrefersLight ? 'light' : 'dark') : settings.theme);
  const baseTheme = THEMES[effectiveThemeKey] || DARK;
  const C = settings.customTheme ? { ...baseTheme, ...settings.customTheme } : baseTheme;

  // ---- UX telemetry: rolling event log captured in localStorage ----
  // Lets us (and the agent running training on the server) review what users
  // actually do. Capped at 500 entries to bound storage; exportable via
  // Settings. Privacy-friendly: nothing is sent off-device automatically.
  type LoggedEvent = { t: number; kind: string; data?: any };
  const LS_EVENTS_KEY = 'lfi_events_v1';
  const logEvent = (kind: string, data?: any) => {
    try {
      const raw = localStorage.getItem(LS_EVENTS_KEY);
      const arr: LoggedEvent[] = raw ? JSON.parse(raw) : [];
      arr.push({ t: Date.now(), kind, data });
      const trimmed = arr.slice(-500);
      localStorage.setItem(LS_EVENTS_KEY, JSON.stringify(trimmed));
      // c2-326: gate the debug print on developer mode. Regular users hit
      // the console-devtools "Verbose" filter anyway, but Eruda on mobile
      // and anyone with debug logs enabled were seeing one line per
      // mouse/keystroke — noisy. Storage write is unconditional (the Admin
      // Logs tab needs the record).
      if (settings.developerMode) console.debug('// SCC: event', kind, data);
    } catch { /* quota — drop */ }
  };
  const exportEvents = () => {
    try {
      const raw = localStorage.getItem(LS_EVENTS_KEY) || '[]';
      const blob = new Blob([raw], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `lfi-events-${new Date().toISOString().slice(0,19).replace(/:/g,'-')}.json`;
      document.body.appendChild(a); a.click(); a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) { console.warn('// SCC: export failed', e); }
  };

  const chatWsRef = useRef<WebSocket | null>(null);
  const telemetryWsRef = useRef<WebSocket | null>(null);
  // Active per-turn latency trace. Set on WS send, cleared on response.
  // turnTrace.ts logs each phase to diag for the Diag tab to surface.
  const currentTurnRef = useRef<TurnTrace | null>(null);
  // c2-433 / claude-0 ask: tri-state connection chip driven by how stale the
  // last backend frame is. Bumped by any chat or telemetry onmessage.
  // Derived into green/yellow/red via connHealth (see computation below).
  const lastBackendFrameRef = useRef<number>(Date.now());
  const [connTick, setConnTick] = useState(0);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // Ref-based send lock. Closes a race where rapid Enter presses can call
  // handleSend twice before React flushes setInput('') — the second call
  // reads stale `input` from closure and would double-post.
  const sendingRef = useRef(false);
  // BUG-FIX 2026-04-17 c0-008: cross-session message bleed. We capture the
  // conversation id at handleSend time so WS chunks can be routed to the
  // ORIGINATING conversation even if the user switches mid-stream. Without
  // this, setMessages (which writes to the active conversation) appended
  // chunks to the wrong convo.
  const streamingConvoIdRef = useRef<string>('');

  // ---- Helpers ----
  const getHost = () => {
    if (settings.apiHost && settings.apiHost.trim()) return settings.apiHost.trim();
    const h = window.location.hostname || '127.0.0.1';
    console.debug("// SCC: Resolved host:", h);
    return h;
  };

  // REGRESSION-GUARD: Previously used useAutoScroll(messagesEndRef) which conflicted
  // with Virtuoso's followOutput='smooth'. Now we use Virtuoso's imperative scrollToBottom
  // exclusively. The old useAutoScroll tried scrollIntoView on a div that Virtuoso manages
  // internally, causing wonky/jumpy scroll behavior.
  const scrollToBottom = useCallback(() => {
    chatViewRef.current?.scrollToBottom();
  }, []);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    // Small delay to let Virtuoso render the new item before scrolling
    const t = setTimeout(() => scrollToBottom(), 100);
    return () => clearTimeout(t);
  }, [messages.length, scrollToBottom]);

  // c2-433 / #313 pass 4: elapsed-tick interval lives inside useThinkingState.

  // c2-433 / task 236: substrate stats poll. /api/health returns
  // {ok, subsystems: {knowledge_concepts, psl_axioms_registered, ...}}.
  // 30s cadence — these counts move slowly (corpus ingest is the producer).
  // Silent failures: this is a peripheral metric; falling back to "no chip"
  // is the right behavior when the endpoint is unreachable.
  useEffect(() => {
    let cancelled = false;

    // c2-433 / #355: /api/health/extended is a 22ms one-call bundle
    // covering drift (11 metrics) + proof counts + contradictions_pending
    // + tuples_total + hdc_cache {cached,sample,coverage} + tier.
    // Try it first; fall back to the old 4-parallel pattern if the
    // endpoint 404s (older deployments without #355).
    const applyBundle = (d: any) => {
      if (cancelled || !d || typeof d !== 'object') return;
      // Tier — top-level OR nested subsystems.current_tier.
      const tier = d.tier ?? d.subsystems?.current_tier;
      if (typeof tier === 'string' && tier) {
        setCurrentTier(prev => prev === tier ? prev : tier);
      }
      // Substrate stats — canonical counts from #355 are drift.fresh_count /
      // axioms; older /api/health nests them under subsystems.
      const subs = d.subsystems || {};
      const drift = d.drift || {};
      const concepts = (typeof d.concepts === 'number' ? d.concepts
        : typeof subs.knowledge_concepts === 'number' ? subs.knowledge_concepts
        : typeof drift.concepts === 'number' ? drift.concepts
        : 0);
      const axioms = (typeof d.axioms === 'number' ? d.axioms
        : typeof subs.psl_axioms_registered === 'number' ? subs.psl_axioms_registered
        : 0);
      const chatTotal = typeof d.chat_total === 'number' ? d.chat_total
        : typeof d.chatTotal === 'number' ? d.chatTotal
        : 0;
      setSubstrateStats({ concepts, axioms, chatTotal });
      // claude-0 #403: stats_age_secs rides along on /api/health/extended.
      // null when the endpoint predates #403 or doesn't emit it.
      const age = typeof d.stats_age_secs === 'number' ? d.stats_age_secs
        : typeof d.stats_age === 'number' ? d.stats_age
        : null;
      setStatsAgeSecs(age);
      // Contradictions pending — top-level number per #355.
      const cp = typeof d.contradictions_pending === 'number' ? d.contradictions_pending
        : typeof drift.contradictions_pending === 'number' ? drift.contradictions_pending
        : null;
      if (cp != null) {
        setContradictionsPending(cp);
        if (prevContradictionsRef.current != null && cp > prevContradictionsRef.current) {
          setContradictionsPulseId(id => id + 1);
        }
        prevContradictionsRef.current = cp;
      }
      // HDC cache — new nested shape {cached, sample, coverage} OR legacy
      // {sample_cached, sample_size, coverage}.
      const hdc = d.hdc_cache;
      if (hdc && typeof hdc === 'object') {
        const sample_cached = typeof hdc.cached === 'number' ? hdc.cached
          : typeof hdc.sample_cached === 'number' ? hdc.sample_cached : 0;
        const sample_size = typeof hdc.sample === 'number' ? hdc.sample
          : typeof hdc.sample_size === 'number' ? hdc.sample_size : 0;
        let coverage: number = 0;
        if (typeof hdc.coverage === 'number') {
          coverage = hdc.coverage > 1 ? hdc.coverage / 100 : hdc.coverage;
        } else if (sample_size > 0) {
          coverage = Math.max(0, Math.min(1, sample_cached / sample_size));
        }
        setHdcCache({ coverage, sample_cached, sample_size });
      }
    };

    const loadBundled = async (): Promise<boolean> => {
      try {
        const r = await fetch(`http://${getHost()}:3000/api/health/extended`);
        if (!r.ok) return false;
        const d = await r.json();
        applyBundle(d);
        return true;
      } catch { return false; }
    };

    // c2-433 / task 236 (legacy): 4-parallel fallback when /api/health/extended
    // isn't live. Kept intact so older deployments still render every chip.
    const loadLegacy = async () => {
      try {
        const [hRes, mRes, cRes, xRes] = await Promise.all([
          fetch(`http://${getHost()}:3000/api/health`),
          fetch(`http://${getHost()}:3000/api/metrics`).catch(() => null as Response | null),
          fetch(`http://${getHost()}:3000/api/contradictions/recent`).catch(() => null as Response | null),
          fetch(`http://${getHost()}:3000/api/hdc/cache/stats`).catch(() => null as Response | null),
        ]);
        if (!hRes.ok) return;
        const d = await hRes.json();
        if (cancelled) return;
        const subs = d?.subsystems || {};
        let chatTotal = 0;
        if (mRes && mRes.ok) {
          try {
            const mtext = await mRes.text();
            const m = mtext.match(/^lfi_chat_total\s+(\d+(?:\.\d+)?)/m);
            if (m) chatTotal = Math.floor(Number(m[1]) || 0);
          } catch { /* metrics parse failure — leave chatTotal at 0 */ }
        }
        setSubstrateStats({
          concepts: typeof subs.knowledge_concepts === 'number' ? subs.knowledge_concepts : 0,
          axioms: typeof subs.psl_axioms_registered === 'number' ? subs.psl_axioms_registered : 0,
          chatTotal,
        });
        if (typeof subs.current_tier === 'string' && subs.current_tier) {
          setCurrentTier(prev => prev === subs.current_tier ? prev : subs.current_tier);
        }
        if (cRes && cRes.ok) {
          try {
            const cj = await cRes.json();
            let n: number | null = null;
            if (Array.isArray(cj)) n = cj.length;
            else if (cj && typeof cj === 'object') {
              if (typeof cj.pending === 'number') n = cj.pending;
              else if (typeof cj.count === 'number') n = cj.count;
              else if (Array.isArray(cj.items)) n = cj.items.length;
              else if (Array.isArray(cj.contradictions)) n = cj.contradictions.length;
            }
            setContradictionsPending(n);
            if (typeof n === 'number' && prevContradictionsRef.current != null && n > prevContradictionsRef.current) {
              setContradictionsPulseId(id => id + 1);
            }
            if (typeof n === 'number') prevContradictionsRef.current = n;
          } catch { /* contradictions parse failure — leave badge as-is */ }
        }
        if (xRes && xRes.ok) {
          try {
            const xj = await xRes.json();
            const sample_cached = typeof xj.sample_cached === 'number' ? xj.sample_cached : 0;
            const sample_size = typeof xj.sample_size === 'number' ? xj.sample_size : 0;
            let coverage: number = 0;
            if (typeof xj.coverage === 'number') {
              coverage = xj.coverage > 1 ? xj.coverage / 100 : xj.coverage;
            } else if (sample_size > 0) {
              coverage = Math.max(0, Math.min(1, sample_cached / sample_size));
            }
            setHdcCache({ coverage, sample_cached, sample_size });
          } catch { /* cache parse failure — leave chip as-is */ }
        }
      } catch { /* peripheral metric — silent */ }
    };

    const load = async () => {
      const ok = await loadBundled();
      if (!ok) await loadLegacy();
    };

    load();
    // #354: only tick when the tab is visible. document.hidden flips on
    // tab-switch / screen-off; we'd waste polls otherwise.
    const id = window.setInterval(() => {
      if (typeof document !== 'undefined' && document.hidden) return;
      load();
    }, 30_000);
    return () => { cancelled = true; window.clearInterval(id); };
  }, []);

  useEffect(() => {
    console.debug("// SCC: Persisting auth:", isAuthenticated);
    localStorage.setItem('lfi_auth', isAuthenticated.toString());
  }, [isAuthenticated]);

  // c2-433: install the diag logger ONCE on mount. After this runs,
  // window.diag is available for devtools console (diag.snapshot() /
  // diag.export() / diag.clear()) and console.warn + console.error
  // calls are mirrored into the ring buffer automatically. Also
  // subscribe to new entries so we can bump diagUnseenErrors and
  // flash the Admin-tab red-dot.
  useEffect(() => {
    diag.install();
    const unsub = diag.subscribe((e) => {
      if (e.level === 'error') {
        setDiagUnseenErrors(n => n + 1);
      }
    });
    return unsub;
  }, []);

  // c2-433 / #316 / #300: pre-send pipeline dry-run. When the user is
  // composing a query (not-empty, >= 6 chars, connected, not currently
  // streaming a reply), debounce ~450ms after the last keystroke and POST
  // /api/explain. The response (speech_act / extracted_concept / rag_top_facts
  // / causal_preview / topic_stack) drives a predicted-modules chip row
  // above the textarea so users can see what the substrate is about to do
  // BEFORE they hit send. AbortController cancels in-flight fetches when
  // the input changes again so only the latest query is awaited.
  useEffect(() => {
    const q = input.trim();
    if (!q || q.length < 6 || !isConnected || isThinking) {
      setExplainPreview(null);
      return;
    }
    // c2-433 / #307: skip fetching while the rate-limit cooldown is active.
    // The preview row will render a countdown chip instead.
    if (explainRateLimitUntil != null && Date.now() < explainRateLimitUntil) {
      return;
    }
    const ctrl = new AbortController();
    const t = window.setTimeout(() => {
      (async () => {
        try {
          const r = await fetch(`http://${getHost()}:3000/api/explain`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ query: q }),
            signal: ctrl.signal,
          });
          if (r.status === 429) {
            // c2-433 / #307: research capability cap is 10/300s per Claude
            // 0's spec. Honor Retry-After (seconds) when present; otherwise
            // fall back to 30s — enough to let the window slide without
            // camping on the endpoint.
            const retryAfter = r.headers.get('retry-after');
            const retrySec = retryAfter ? Math.max(1, Number(retryAfter) || 30) : 30;
            setExplainRateLimitUntil(Date.now() + retrySec * 1000);
            setExplainPreview(null);
            return;
          }
          if (!r.ok) {
            setExplainPreview(null);
            return;
          }
          const data = await r.json();
          setExplainPreview(data);
        } catch {
          // AbortError or network — silent (peripheral UX affordance).
          setExplainPreview(null);
        }
      })();
    }, 450);
    return () => { window.clearTimeout(t); ctrl.abort(); };
  }, [input, isConnected, isThinking, explainRateLimitUntil]);

  // c2-433 / #307: 1s ticker while the rate-limit is active so the chip
  // countdown updates. Cleared the moment the deadline passes.
  useEffect(() => {
    if (explainRateLimitUntil == null) return;
    const id = window.setInterval(() => {
      if (Date.now() >= explainRateLimitUntil) {
        setExplainRateLimitUntil(null);
      } else {
        setExplainRateLimitTick(t => t + 1);
      }
    }, 1000);
    return () => window.clearInterval(id);
  }, [explainRateLimitUntil]);

  // Disconnect banner: only show after 2s of !isConnected, hide instantly on
  // reconnect. Skips the initial pre-connect window (avoids flash on load).
  useEffect(() => {
    if (isConnected) { setShowDisconnectBanner(false); setBackendOffline(false); return; }
    const t = setTimeout(() => setShowDisconnectBanner(true), 2000);
    return () => clearTimeout(t);
  }, [isConnected]);

  // c0-027 / c2-411 fix: activeView must be declared before the useEffect
  // below that lists it as a dep — the old location near line 2189 caused
  // a TDZ on mount ("Cannot access 'Lt' before initialization"), because
  // the effect's deps array reads activeView during render and the
  // useState line hadn't executed yet.
  // 3-view app (Chat / Classroom / Admin). Admin is still a modal, but Chat
  // and Classroom are true top-level views that replace each other.
  // Hash-route-aware: #chat / #classroom / #admin hydrate the view on mount
  // and forward/back history traversal updates the active view.
  const [activeView, setActiveView] = useState<'chat' | 'classroom' | 'fleet' | 'library' | 'auditorium'>(() => {
    const h = (typeof window !== 'undefined' && window.location.hash.replace('#', '')) || 'chat';
    if (h === 'classroom') return 'classroom';
    if (h === 'fleet') return 'fleet';
    if (h === 'library') return 'library';
    if (h === 'auditorium') return 'auditorium';
    return 'chat';
  });

  // URL hash <-> activeView sync. First mount replaceState (no history
  // balloon); subsequent view changes pushState so the browser Back button
  // actually takes the user to the previous view. Admin is NOT in the hash
  // anymore — it's wired through useHistoryDialog below so Back closes it
  // without fighting the view machinery.
  const firstViewSyncRef = useRef(true);
  useEffect(() => {
    const want = activeView;
    const cur = window.location.hash.replace('#', '');
    if (cur !== want) {
      const url = `${window.location.pathname}${window.location.search}#${want}`;
      if (firstViewSyncRef.current) {
        window.history.replaceState(null, '', url);
      } else {
        window.history.pushState(null, '', url);
      }
    }
    firstViewSyncRef.current = false;
    if (want === 'classroom') setSrAnnouncement('Classroom view active');
    else if (want === 'fleet') setSrAnnouncement('Fleet view active');
    else if (want === 'library') setSrAnnouncement('Library view active');
    else if (want === 'auditorium') setSrAnnouncement('Auditorium view active');
    else setSrAnnouncement('Chat view active');
  }, [activeView]);
  useEffect(() => {
    const onHashChange = () => {
      const h = window.location.hash.replace('#', '');
      if (h === 'classroom') setActiveView('classroom');
      else if (h === 'fleet') setActiveView('fleet');
      else if (h === 'library') setActiveView('library');
      else if (h === 'auditorium') setActiveView('auditorium');
      else setActiveView('chat');
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  // Browser Back → close the topmost modal / popover / dialog. Each hook call
  // pushes a history entry when the surface opens; popstate closes it.
  // Programmatic close (e.g. clicking X) pops the stale entry so forward/back
  // stays balanced. Ordering mirrors Escape-key precedence in the key handler.
  // Tri-state connection chip tick: re-renders every 5s so the derived
  // connHealth (green/yellow/red) flips when frames go stale. Cheap — one
  // state increment, no fetches. #354: skip when tab hidden — conn state
  // can wait until the user is looking.
  useEffect(() => {
    const id = window.setInterval(() => {
      if (typeof document !== 'undefined' && document.hidden) return;
      setConnTick(t => t + 1);
    }, 5000);
    return () => window.clearInterval(id);
  }, []);

  useHistoryDialog(showAdmin, () => setShowAdmin(false), 'admin');
  useHistoryDialog(showSettings, () => setShowSettings(false), 'settings');
  useHistoryDialog(showCmdPalette, () => setShowCmdPalette(false), 'cmdk');
  useHistoryDialog(showActivity, () => setShowActivity(false), 'activity');
  useHistoryDialog(showKnowledge, () => setShowKnowledge(false), 'kb');
  useHistoryDialog(showShortcuts, () => setShowShortcuts(false), 'shortcuts');
  useHistoryDialog(showTerminal, () => setShowTerminal(false), 'xterm');
  useHistoryDialog(showTraining, () => setShowTraining(false), 'training');
  useHistoryDialog(!!showGame, () => setShowGame(null), 'game');
  useHistoryDialog(showTeach, () => setShowTeach(false), 'teach');

  // Derived connection health for the chip. Red: WS disconnected.
  // Yellow: WS says open but we haven't seen a frame in 15s (likely a
  // dead-but-not-yet-detected socket, e.g. LTE idle-kill). Green: alive.
  const connHealth: 'green' | 'yellow' | 'red' = !isConnected
    ? 'red'
    : (Date.now() - lastBackendFrameRef.current > 15000 ? 'yellow' : 'green');
  // eslint suppression: connTick is the re-render trigger for this derivation.
  void connTick;

  // Network-level online/offline listener. Reads navigator.onLine and keeps
  // a separate banner color (amber vs red) so users know if the problem is
  // their WiFi vs the server.
  useEffect(() => {
    const on = () => setNetworkOffline(false);
    const off = () => setNetworkOffline(true);
    window.addEventListener('online', on);
    window.addEventListener('offline', off);
    return () => { window.removeEventListener('online', on); window.removeEventListener('offline', off); };
  }, []);

  // c2-291: log uncaught errors + unhandled promise rejections so async
  // faults (WebSocket/fetch/setTimeout) show up in the client-event log
  // even when they escape AppErrorBoundary. Lightweight — just records,
  // no UI disruption. Message + filename clipped to 200 chars to keep the
  // event payload small.
  useEffect(() => {
    const onError = (e: ErrorEvent) => {
      logEvent('uncaught_error', {
        message: String(e.message || 'unknown').slice(0, 200),
        source: String(e.filename || '').slice(0, 200),
        line: e.lineno, col: e.colno,
      });
    };
    const onRejection = (e: PromiseRejectionEvent) => {
      const reason: any = e.reason;
      const msg = reason?.message ? String(reason.message) : String(reason);
      logEvent('unhandled_rejection', {
        message: msg.slice(0, 200),
        stack: reason?.stack ? String(reason.stack).slice(0, 400) : undefined,
      });
    };
    window.addEventListener('error', onError);
    window.addEventListener('unhandledrejection', onRejection);
    return () => {
      window.removeEventListener('error', onError);
      window.removeEventListener('unhandledrejection', onRejection);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Backend health probe — when WS is down, periodically GET /api/status to
  // distinguish "WS hiccup, REST still works" (transient) from "whole backend
  // gone" (worth telling the user to start the dev server). Only runs while
  // disconnected to avoid pestering the server when WS is healthy.
  useEffect(() => {
    if (isConnected) return;
    const probe = async () => {
      try {
        const ctrl = new AbortController();
        const to = setTimeout(() => ctrl.abort(), 4000);
        const res = await fetch(`http://${getHost()}:3000/api/status`, { signal: ctrl.signal });
        clearTimeout(to);
        setBackendOffline(!res.ok);
      } catch {
        setBackendOffline(true);
      }
    };
    probe();
    const id = setInterval(probe, 10000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isConnected]);

  // c2-433 / #313: toast auto-dismiss + showToast moved into useToastQueue.
  // App.tsx only consumes the array + the produce/dismiss callbacks above.

  // ---- Eruda FAB repositioning ----
  // Moves the Eruda floating action button above the input bar on mobile
  useEffect(() => {
    const moveEruda = () => {
      const erudaEntry = document.getElementById('eruda-entry-btn') ||
        document.querySelector('.eruda-entry-btn') as HTMLElement;
      if (erudaEntry) {
        console.debug("// SCC: Repositioning Eruda FAB");
        erudaEntry.style.bottom = isMobile ? '80px' : '20px';
        erudaEntry.style.right = '10px';
        erudaEntry.style.zIndex = '9998';
      }
    };
    // Try immediately and after a delay (Eruda may load asynchronously)
    moveEruda();
    const timer = setTimeout(moveEruda, 2000);
    return () => clearTimeout(timer);
  }, [isMobile, isAuthenticated]);

  // ---- WebSocket: Chat ----
  useEffect(() => {
    if (!isAuthenticated) {
      console.debug("// SCC: Skipping chat WS — not authenticated");
      return;
    }
    const wsUrl = `ws://${getHost()}:3000/ws/chat`;
    console.debug("// SCC: Connecting chat WS:", wsUrl);
    let reconnectTimer: ReturnType<typeof setTimeout>;
    // Exponential backoff for chat WS reconnect. Starts at 1s, doubles up to 30s,
    // resets to 1s on a successful open. Prior fixed 3s hammered the backend
    // during brief network blips AND waited too long after a server restart.
    let reconnectDelayMs = 1000;
    // c2-433 / task 264: track whether we've ever closed so the onopen
    // handler can distinguish "initial connect" (silent) from "reconnect
    // after a drop" (toast). Plain ws.onopen fires for both cases.
    let hasDisconnected = false;
    // #354 / claude-0 14:10 URGENT: debounce the "disconnected" UI flip
    // by 3s. Most reconnects complete <2s; flipping the chip + banner
    // immediately creates a flicker user reads as "connection is
    // unstable." Timer is cleared on the next onopen, so a clean
    // reconnect leaves no visible trace.
    let pendingDisconnectTimer: ReturnType<typeof setTimeout> | null = null;
    const RECONNECT_MAX_MS = 30000;

    const connect = () => {
      console.debug("// SCC: chat WS connect()");
      // c2-254 / #116: attempt in-flight → no countdown to show.
      setWsReconnectAt(null);
      const ws = new WebSocket(wsUrl);
      chatWsRef.current = ws;

      ws.onopen = () => {
        console.debug("// SCC: Chat WS OPEN");
        diag.info('ws-chat', 'open', { url: wsUrl, hadDisconnected: hasDisconnected });
        // #354: cancel any pending disconnect flip — fast reconnect
        // should leave no visible UI flicker.
        if (pendingDisconnectTimer) {
          clearTimeout(pendingDisconnectTimer);
          pendingDisconnectTimer = null;
        }
        setIsConnected(true);
        setWsReconnectAt(null);
        // c2-433 / task 264: toast on reconnect-after-drop, silent on the
        // initial mount-time connect. hasDisconnected flips true whenever
        // ws.onclose fires, so this branch only runs after the first drop.
        if (hasDisconnected) {
          showToast('Reconnected');
          hasDisconnected = false;
        }
        reconnectDelayMs = 1000; // reset backoff after healthy connect
        // c2-382 / BIG #177: drain the offline outbox. Each queued payload
        // gets replayed in FIFO order. If the send throws mid-drain (socket
        // closed again), we leave the remainder in place for the next
        // onopen. Successful sends have their corresponding user bubbles
        // re-flagged to drop the 'queued' badge.
        try {
          const raw = localStorage.getItem('lfi_outbox') || '[]';
          const queue = JSON.parse(raw) as Array<{ id: number; convId: string; content: string; incognito: boolean; at: number }>;
          if (!queue.length) return;
          const sent: number[] = [];
          for (const entry of queue) {
            try {
              ws.send(JSON.stringify({ content: entry.content, incognito: entry.incognito }));
              sent.push(entry.id);
            } catch {
              break; // socket broke mid-drain; keep the rest for next open
            }
          }
          const remaining = queue.filter(e => !sent.includes(e.id));
          localStorage.setItem('lfi_outbox', JSON.stringify(remaining));
          if (sent.length > 0) {
            // Unbadge the user messages we just sent. Best-effort across
            // all convos since _queued is client-only state.
            setConversations(prev => prev.map(c => ({
              ...c,
              messages: c.messages.map(m => (m as any)._queued && sent.some(sid => sid === (m as any)._queuedId) ? { ...m, _queued: false } : m),
            })));
            setMessages(prev => prev.map(m => (m as any)._queued ? { ...m, _queued: false } : m));
            logEvent('msg_queue_drained', { count: sent.length });
            // c2-433 / task 265: explicit toast so the user knows the
            // queued messages went through (otherwise the only signal is
            // the (Queued) badge silently disappearing). Singular/plural
            // for grammar.
            showToast(`Sent ${sent.length} queued message${sent.length === 1 ? '' : 's'}`);
          }
        } catch (err) {
          console.warn('// SCC: outbox drain failed', err);
        }
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          console.debug("// SCC: Chat msg:", msg.type);
          lastBackendFrameRef.current = Date.now();
          // turn-trace: mark the first frame after a send arrives. Subsequent
          // chunks are no-ops (turnTrace.ts dedupes on firstFrameAt).
          markFirstFrame(currentTurnRef.current, String(msg.type || 'unknown'));

          // Helper: apply a messages reducer to the conversation that owns this
          // in-flight WS exchange (captured at handleSend time). When that
          // convo is also the active one, also update the live `messages`
          // state. Without this, switching conversations mid-stream caused
          // chunks to bleed into the new active convo (BUG c0-008 #2).
          const applyToStreamingConvo = (reducer: (prev: ChatMessage[]) => ChatMessage[]) => {
            const targetId = streamingConvoIdRef.current;
            if (targetId) {
              setConversations(prev => prev.map(c => c.id === targetId
                ? { ...c, messages: reducer(c.messages), updatedAt: Date.now() } : c));
            }
            // c2-433 / task 260: read currentConversationIdRef.current (live)
            // instead of the closure-stale currentConversationId — without
            // this, switching convo mid-stream bled chunks into the new one.
            if (!targetId || targetId === currentConversationIdRef.current) {
              setMessages(reducer);
            }
          };

          if (msg.type === 'progress') {
            setThinkingStep(msg.step || 'Processing...');
            // c2-433 / #316: read cognitive_module if present. Accept either
            // top-level cognitive_module or nested in a modules object so the
            // client tracks whatever shape the backend ships first.
            const mod: string | undefined = msg.cognitive_module || msg.module;
            if (mod) ts.recordModule(mod);
            // c2-433 / #352: topic-context tracking. When the backend emits
            // a topic field on chat_progress (e.g. "volcanoes" for the
            // 4-turn chain Claude 0 verified), the UI surfaces it as a
            // small chip so users see what context the multi-turn pronoun
            // resolution is anchored to. Forward-compat scaffold — silent
            // when backend doesn't ship the field.
            const topic: string | undefined = msg.topic || msg.topic_context;
            if (topic && typeof topic === 'string') setActiveTopic(topic);
          } else if (msg.type === 'chat_chunk') {
            // Streaming: append partial text to the last assistant message,
            // or create one if this is the first chunk.
            setIsThinking(false);
            // c2-372 / task 105 / c2-433 #313 pass 8: accumulate chars into
            // the chat-streaming tracker. First chunk seeds startAt + chars;
            // subsequent chunks just grow the counter. growBy is a no-op
            // when n<=0 so empty chunks don't disturb the start time.
            const chunkLen = (msg.text || '').length;
            cstr.growBy(chunkLen);
            applyToStreamingConvo(prev => {
              const last = prev[prev.length - 1];
              if (last && last.role === 'assistant' && (last as any)._streaming) {
                return [...prev.slice(0, -1), { ...last, content: last.content + (msg.text || '') }];
              }
              return [...prev, {
                id: msgId(), role: 'assistant' as const,
                content: msg.text || '', timestamp: Date.now(),
                _streaming: true,
              } as any];
            });
          } else if (msg.type === 'chat_done') {
            // c2-372 / task 105: end of stream -- drop the timing chip.
            cstr.end();
            // c2-433 / #316: clear the active-module pulse so the next turn
            // starts clean. modulesUsed clears on the next handleSend so
            // the post-turn pill can still show which modules ran.
            setActiveModule(null);
            // c2-433 / task 247 + 255: log the modules-used set as a single
            // event per turn so the Activity log captures the cognitive
            // dispatch trace. Read from modulesUsedRef (mirrored from
            // state) since the WS handler closure captures stale state.
            const usedNow = modulesUsedRef.current;
            if (usedNow.size > 0) {
              logEvent('chat_modules_used', { modules: Array.from(usedNow) });
            }
            // OS notification when the user has tabbed away and opted in.
            // Requires prior permission grant; silently no-op otherwise.
            // c2-433 / task 261: read settingsRef + conversationsRef +
            // messagesRef to dodge closure-staleness — toggling
            // notifyOnReply mid-session now takes effect immediately.
            if (settingsRef.current.notifyOnReply && typeof Notification !== 'undefined' && Notification.permission === 'granted' && document.hidden) {
              try {
                // c2-290: include a preview of the actual reply rather than
                // a generic "Your response is ready." — read the last
                // assistant message from the streaming convo, strip
                // markdown, cap at 140 chars so OS notification boxes
                // don't mangle long replies.
                const streamingMessages = streamingConvoIdRef.current
                  ? (conversationsRef.current.find(c => c.id === streamingConvoIdRef.current)?.messages ?? messagesRef.current)
                  : messagesRef.current;
                const lastAssistant = [...streamingMessages].reverse().find(m => m.role === 'assistant');
                const rawPreview = lastAssistant?.content || 'Your response is ready.';
                const clean = stripMarkdown(rawPreview).replace(/\s+/g, ' ').trim();
                const body = clean.length > 140 ? clean.slice(0, 137) + '\u2026' : clean;
                const n = new Notification('PlausiDen AI replied', {
                  body,
                  tag: 'plausiden-reply',   // coalesces repeated replies
                  silent: false,
                });
                n.onclick = () => { try { window.focus(); n.close(); } catch {} };
              } catch { /* non-fatal */ }
            }
            // End of streaming — finalize the message.
            applyToStreamingConvo(prev => {
              const last = prev[prev.length - 1];
              if (last && (last as any)._streaming) {
                const { _streaming, ...clean } = last as any;
                return [...prev.slice(0, -1), {
                  ...clean,
                  mode: msg.mode, confidence: msg.confidence,
                  tier: msg.tier, intent: msg.intent,
                  reasoning: msg.reasoning, plan: msg.plan,
                  conclusion_id: msg.conclusion_id,
                }];
              }
              return prev;
            });
          } else if (msg.type === 'chat_response') {
            setIsThinking(false);
            setThinkingStart(null);
            markResponse(currentTurnRef.current, { contentLen: (msg.content || '').length });
            // After commit, mark the render phase. requestAnimationFrame fires
            // after React paints, so the t3 timestamp captures user-visible
            // latency, not just state-set latency.
            const t = currentTurnRef.current;
            requestAnimationFrame(() => { markRendered(t); });
            currentTurnRef.current = null;
            applyToStreamingConvo(prev => [...prev, {
              id: msgId(), role: 'assistant',
              content: msg.content || '',
              mode: msg.mode, confidence: msg.confidence,
              tier: msg.tier, intent: msg.intent,
              reasoning: msg.reasoning, plan: msg.plan,
              conclusion_id: msg.conclusion_id,
              timestamp: Date.now(),
            }]);
            // Don't sync tier from chat replies — user's selection in the
            // input-bar model dropdown is authoritative. Syncing here caused
            // the "snaps back" bug because the backend was reporting the tier
            // it actually USED (which may have been down-scaled by the router).
          } else if (msg.type === 'web_result') {
            console.debug("// SCC: Web result, sources:", msg.source_count);
            applyToStreamingConvo(prev => [...prev, {
              id: msgId(), role: 'web',
              content: `${msg.source_count} sources | trust: ${(msg.trust * 100).toFixed(0)}%\n\n${msg.summary}`,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'chat_error') {
            console.debug("// SCC: Chat error:", msg.error);
            setIsThinking(false);
            markResponse(currentTurnRef.current, { error: String(msg.error || 'unknown') });
            currentTurnRef.current = null;
            // c2-372: error also ends the stream -- clear the timing chip.
            cstr.end();
            applyToStreamingConvo(prev => {
              // c2-371 / task 79: remember the most recent user turn so the
              // Retry button has something to resend. Scanned from the end
              // to tolerate streaming_convo prepending system messages.
              const lastUser = [...prev].reverse().find(m => m.role === 'user');
              if (lastUser) {
                setLastErrorRetry({ userContent: lastUser.content, at: Date.now() });
              }
              return [...prev, {
                id: msgId(), role: 'system',
                content: `Error: ${msg.error}`, timestamp: Date.now(),
              }];
            });
          }
        } catch (e) {
          // c0-020/E3: JSON.parse or handler exceptions used to drop silently
          // to console. Surface them as a system message so users see the
          // dashboard didn't understand the server frame — and log an event
          // so the Admin Logs tab captures it too.
          console.error("// SCC: Chat parse error:", e);
          logEvent('ws_parse_error', { error: String((e as Error)?.message || e), preview: String(event.data).slice(0, 160) });
          applyToStreamingConvo(prev => [...prev, {
            id: msgId(), role: 'system',
            content: `Could not decode a server frame (${String((e as Error)?.message || e)}). Some AI output may be missing — check the Admin → Logs tab.`,
            timestamp: Date.now(),
          }]);
        }
      };

      ws.onclose = (ev) => {
        console.debug("// SCC: Chat WS CLOSED:", ev.code, ev.reason || '', 'reconnect in', reconnectDelayMs, 'ms');
        diag.warn('ws-chat', `close code=${ev.code}`, { code: ev.code, reason: ev.reason || '', wasClean: ev.wasClean, reconnectInMs: reconnectDelayMs });
        hasDisconnected = true;
        // #354 / claude-0 14:10: DO NOT flip isConnected immediately. A
        // 3s debounce covers the common case where reconnect completes
        // fast. Only if the socket is still down after 3s do we flap the
        // chip, banner, and countdown.
        if (pendingDisconnectTimer) clearTimeout(pendingDisconnectTimer);
        pendingDisconnectTimer = setTimeout(() => {
          pendingDisconnectTimer = null;
          setIsConnected(false);
          // Schedule the banner countdown only on the delayed flip so
          // sub-3s drops don't spam the reconnect UI.
          const jitter = Math.floor(Math.random() * 500);
          setWsReconnectAt(Date.now() + reconnectDelayMs + jitter);
        }, 3000);
        // turn-trace: socket died mid-turn — terminal frame for this turn.
        if (currentTurnRef.current) {
          markResponse(currentTurnRef.current, { error: `ws_close code=${ev.code}` });
          currentTurnRef.current = null;
        }
        // c2-433 / task 258 + 259: clear in-flight thinking + streaming
        // state when WS dies. Without this, a backend restart mid-stream
        // leaves the user staring at "Thinking…" forever (no chunks will
        // arrive on the reconnected socket — that turn is lost). When the
        // close interrupted an active stream (isThinkingRef.current), also
        // capture the last user message into lastError so the Retry pill
        // appears after reconnect. Same pattern as chat_error handler.
        if (isThinkingRef.current) {
          // Walk the messages array from the end to find the last user
          // turn — same pattern as chat_error. Read messagesRef to avoid
          // closure-staleness on the messages array.
          const ms = messagesRef.current;
          for (let i = ms.length - 1; i >= 0; i--) {
            if (ms[i].role === 'user') {
              cstr.setLastError({ userContent: ms[i].content, at: Date.now() });
              break;
            }
          }
        }
        setIsThinking(false);
        setThinkingStart(null);
        cstr.end();
        // Add 0-500ms jitter so a fleet of reconnecting clients doesn't stampede.
        const jitter = Math.floor(Math.random() * 500);
        reconnectTimer = setTimeout(connect, reconnectDelayMs + jitter);
        reconnectDelayMs = Math.min(reconnectDelayMs * 2, RECONNECT_MAX_MS);
      };

      ws.onerror = (ev) => {
        console.error("// SCC: Chat WS ERROR:", ev);
        diag.error('ws-chat', 'error event (onclose will follow)', ev as any);
        // #354: onerror typically pairs with onclose; let the
        // debounced onclose handler own the isConnected flip.
      };
    };

    connect();
    return () => {
      clearTimeout(reconnectTimer);
      if (pendingDisconnectTimer) clearTimeout(pendingDisconnectTimer);
      chatWsRef.current?.close();
    };
  }, [isAuthenticated]);

  // c2-254 / #116: tick every 500ms while wsReconnectAt is set so the
  // banner countdown re-renders. Stops automatically when the socket
  // reopens (wsReconnectAt cleared) so there's no idle interval running.
  useEffect(() => {
    if (wsReconnectAt == null) return;
    const id = setInterval(() => setWsTick(t => t + 1), 500);
    return () => clearInterval(id);
  }, [wsReconnectAt]);

  // ---- WebSocket: Telemetry ----
  useEffect(() => {
    if (!isAuthenticated) return;
    const wsUrl = `ws://${getHost()}:3000/ws/telemetry`;
    console.debug("// SCC: Connecting telemetry WS:", wsUrl);
    let reconnectTimer: ReturnType<typeof setTimeout>;
    // Telemetry is non-critical — start at 2s, cap at 60s. Resets on open.
    let reconnectDelayMs = 2000;
    const RECONNECT_MAX_MS = 60000;

    const connect = () => {
      const ws = new WebSocket(wsUrl);
      telemetryWsRef.current = ws;
      ws.onopen = () => { reconnectDelayMs = 2000; };
      ws.onmessage = (event) => {
        try {
          lastBackendFrameRef.current = Date.now();
          const msg = JSON.parse(event.data);
          if (msg.type === 'telemetry' && msg.data) {
            setStats(prev => ({ ...prev, ...msg.data }));
          }
        } catch (e) { console.error("// SCC: Telemetry parse error:", e); }
      };
      ws.onclose = () => {
        const jitter = Math.floor(Math.random() * 1000);
        reconnectTimer = setTimeout(connect, reconnectDelayMs + jitter);
        reconnectDelayMs = Math.min(reconnectDelayMs * 2, RECONNECT_MAX_MS);
      };
      ws.onerror = (ev) => console.error("// SCC: Telemetry WS ERROR:", ev);
    };

    connect();
    return () => { clearTimeout(reconnectTimer); telemetryWsRef.current?.close(); };
  }, [isAuthenticated]);

  // ---- Auth ----
  const handleLogin = async () => {
    console.debug("// SCC: handleLogin");
    setAuthError('');
    setAuthLoading(true);
    try {
      const url = `http://${getHost()}:3000/api/auth`;
      console.debug("// SCC: POST", url);
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: password }),
      });
      const data = await res.json();
      console.debug("// SCC: Auth response:", data);
      if (data.status === 'authenticated') setIsAuthenticated(true);
      else setAuthError('Sovereign key rejected.');
    } catch (e) {
      console.error("// SCC: Auth error:", e);
      setAuthError('Backend unreachable. Is the server running on port 3000?');
    } finally { setAuthLoading(false); }
  };

  const handleLogout = () => {
    console.debug("// SCC: Logout");
    localStorage.removeItem('lfi_auth');
    chatWsRef.current?.close();
    telemetryWsRef.current?.close();
    setIsAuthenticated(false);
    setMessages([]);
  };

  // ---- Tier Switch ----
  // Guards against the "snap back to Pulse" bug: always re-auth first (server
  // state is in-memory and resets on restart). Optimistically flips the UI
  // immediately so the select doesn't visibly revert while the request flies.
  const handleTierSwitch = async (tier: string) => {
    console.debug("// SCC: Switching tier to:", tier);
    const previous = currentTier;
    setTierSwitching(true);
    // Optimistic update — user sees the change instantly.
    setCurrentTier(tier);
    try {
      const host = getHost();
      // Re-auth first (idempotent, cheap, fixes post-server-restart flakes).
      await fetch(`http://${host}:3000/api/auth`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: 'CHANGE_ME_SET_LFI_SOVEREIGN_KEY' }),
      }).catch(() => {});
      const res = await fetch(`http://${host}:3000/api/tier`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier }),
      });
      const data = await res.json();
      console.debug("// SCC: Tier switch response:", data);
      if (data.status === 'ok') {
        setCurrentTier(data.tier);
        // c2-416: persist the selection so a reload doesn't snap back to
        // Pulse. Previously currentTier was session-only; settings.defaultTier
        // was a separate "start here on new sessions" knob. Users expected
        // the live selector to stick — so we now sync both.
        setSettings(s => ({ ...s, defaultTier: data.tier }));
        logEvent('tier_switched', { tier: data.tier });
      } else {
        // Revert optimistic update if backend rejected.
        setCurrentTier(previous);
        // Surface rejection so the user doesn't see the select silently reset.
        setMessages(prev => [...prev, {
          id: msgId(), role: 'system',
          content: `Couldn't switch tier: ${data.reason || data.status}. Try clicking Settings then close once to refresh auth.`,
          timestamp: Date.now(),
        }]);
      }
    } catch (e) {
      console.error("// SCC: Tier switch error:", e);
      setMessages(prev => [...prev, {
        id: msgId(), role: 'system',
        content: 'Tier switch failed — backend unreachable.',
        timestamp: Date.now(),
      }]);
    } finally { setTierSwitching(false); }
  };

  // ---- Admin actions ----
  // Tracks the last-fetch outcome for /api/facts so the UI can tell the difference
  // between "user hasn't clicked yet" (null), "server returned 0 results" (0),
  // and "fetch errored" (-1). The existing facts.length-gated render was invisible
  // when the server returned an empty array, which read to the user as "broken".
  const [factsFetchedAt, setFactsFetchedAt] = useState<number | null>(null);
  const [factsError, setFactsError] = useState<string | null>(null);

  const fetchFacts = async () => {
    console.debug("// SCC: Fetching facts");
    setAdminLoading('facts');
    setFactsError(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const res = await fetch(`http://${getHost()}:3000/api/facts`, { signal: ctrl.signal });
      clearTimeout(to);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setFacts(data.facts || []);
      setFactsFetchedAt(Date.now());
    } catch (e: any) {
      console.error("// SCC: Facts fetch error:", e);
      setFactsError(String(e?.message || e));
      setFactsFetchedAt(Date.now());
    } finally { setAdminLoading(''); }
  };

  // Centralised chat-log fetch: tracks auth/error/empty so the Activity modal can
  // show a meaningful message instead of the generic "no logged turns" line (which
  // was misleading when the fetch was actually rejected for auth).
  const [chatLogError, setChatLogError] = useState<string | null>(null);
  const [chatLogFetchedAt, setChatLogFetchedAt] = useState<number | null>(null);
  const fetchChatLog = async (limit = 50) => {
    setChatLogError(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const res = await fetch(`http://${getHost()}:3000/api/chat-log?limit=${limit}`, { signal: ctrl.signal });
      clearTimeout(to);
      const d = await res.json();
      if (d?.error) throw new Error(String(d.error));
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setServerChatLog(d.entries || []);
      setChatLogFetchedAt(Date.now());
    } catch (e: any) {
      setChatLogError(String(e?.message || e));
      setChatLogFetchedAt(Date.now());
    }
  };

  const [qosError, setQosError] = useState<string | null>(null);
  const [qosFetchedAt, setQosFetchedAt] = useState<number | null>(null);
  const fetchQos = async () => {
    console.debug("// SCC: Fetching QoS report");
    setAdminLoading('qos');
    setQosError(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const res = await fetch(`http://${getHost()}:3000/api/qos`, { signal: ctrl.signal });
      clearTimeout(to);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setQosReport(data);
      setQosFetchedAt(Date.now());
    } catch (e: any) {
      console.error("// SCC: QoS fetch error:", e);
      setQosError(String(e?.message || e));
      setQosFetchedAt(Date.now());
    } finally { setAdminLoading(''); }
  };

  const clearChat = () => {
    console.debug("// SCC: Clearing chat");
    // c2-292: capture the pre-clear list in the toast closure so the Undo
    // button restores it. Matches the soft-delete pattern used for
    // deleteConversation. Skip the undo toast when there was nothing to
    // clear — no-op paths shouldn't spam.
    const previous = messages;
    setMessages([]);
    if (previous.length > 0) {
      logEvent('clear_chat', { messages: previous.length });
      showToast(`Cleared ${previous.length} message${previous.length === 1 ? '' : 's'}`, () => {
        setMessages(previous);
        logEvent('clear_chat_undo', { messages: previous.length });
      });
    }
  };

  // Passwordless mode: auto-authenticate + push the user's preferred default
  // tier to the backend. Server state is in-memory and resets to Pulse on
  // every restart — pushing the default here is what makes "I set BigBrain
  // in Settings" actually stick across reloads.
  useEffect(() => {
    if (!isAuthenticated) return;
    (async () => {
      try {
        await fetch(`http://${getHost()}:3000/api/auth`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key: 'CHANGE_ME_SET_LFI_SOVEREIGN_KEY' }),
        });
        // Push user's default tier to the backend so the server starts on
        // whatever they locked in.
        if (settings.defaultTier && settings.defaultTier !== 'Pulse') {
          await fetch(`http://${getHost()}:3000/api/tier`, {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ tier: settings.defaultTier }),
          });
          setCurrentTier(settings.defaultTier);
        }
      } catch (e) { console.warn('// SCC: auto-auth failed', e); }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated]);

  // Global keyboard shortcuts — per Bible §6.5
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // c2-320: TOS dialog is a gate. No shortcut should fire underneath it
      // — including Cmd+K (palette), ? (shortcuts), auto-focus-on-keystroke.
      // Tab / Shift-Tab stay available for focus navigation within the
      // dialog so keyboard users can still reach the Accept button.
      if (!tosAccepted) return;
      const mod = e.metaKey || e.ctrlKey;
      const k = e.key.toLowerCase();

      // "?" opens the shortcuts cheatsheet. Skip when typing in inputs.
      if (e.key === '?' && !e.metaKey && !e.ctrlKey) {
        const target = e.target as HTMLElement | null;
        const isEditable = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
        if (!isEditable) { e.preventDefault(); setShowShortcuts(true); return; }
      }

      // Auto-focus chat input on a printable keystroke when no modal is open
      // and focus is on body/main (not an input). Matches ChatGPT/Claude UX:
      // user lands on the page, types — text goes into the chat box without
      // needing to click. Skip combos and named keys (Tab, Esc, Arrow*, etc.).
      // NOTE: focus alone loses the original keystroke (it fires while focus
      // is still on body), so we forward the character into `input` state and
      // preventDefault to stop any default behaviour like page scrolling on
      // Space.
      if (!mod && !e.altKey && e.key.length === 1) {
        const target = e.target as HTMLElement | null;
        const isEditable = !!(target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable));
        const anyModalOpen = showCmdPalette || showSettings || showKnowledge || showActivity || showGame || showShortcuts || pendingConfirm || !!showWelcome || showAdmin || showTerminal;
        // c0-020: only forward keystrokes to the chat input when the Chat
        // view is the active top-level section. Typing in Classroom should
        // not hijack to chat — users may be scanning tables etc.
        const inChatView = activeView === 'chat';
        if (!isEditable && !anyModalOpen && inChatView && inputRef.current) {
          e.preventDefault();
          setInput(prev => prev + e.key);
          inputRef.current.focus();
          return;
        }
      }

      if (mod && k === 'k') { e.preventDefault(); setShowCmdPalette(v => !v); setCmdQuery(''); setCmdIndex(0); }
      else if (mod && k === 'n') { e.preventDefault(); createNewConversation(); }
      else if (mod && k === 'd') { e.preventDefault(); setSettings(s => ({ ...s, developerMode: !s.developerMode })); }
      else if (mod && k === ',') { e.preventDefault(); setShowSettings(true); }
      else if (mod && k === 'e') { e.preventDefault(); inputRef.current?.focus(); }
      else if (mod && k === '/') { e.preventDefault(); inputRef.current?.focus(); }
      // c0-020: top-level view shortcuts. Full map:
      //   ⌘1 Chat, ⌘2 Classroom, ⌘3 Admin, ⌘4 Fleet, ⌘5 Library, ⌘6 Auditorium
      else if (mod && !e.shiftKey && (k === '1' || k === '2' || k === '3' || k === '4' || k === '5' || k === '6')) {
        e.preventDefault();
        if (k === '1') { setActiveView('chat'); setShowAdmin(false); }
        else if (k === '2') { setActiveView('classroom'); setShowAdmin(false); }
        else if (k === '3') { setShowAdmin(true); }
        else if (k === '4') { setActiveView('fleet'); setShowAdmin(false); }
        else if (k === '5') { setActiveView('library'); setShowAdmin(false); }
        else { setActiveView('auditorium'); setShowAdmin(false); }
      }
      else if (mod && e.shiftKey && k === 'k') { e.preventDefault(); setShowKnowledge(true); fetchKnowledge(); }
      else if (mod && e.shiftKey && k === 'd') {
        e.preventDefault();
        const themes: Array<typeof settings.theme> = ['dark','light','midnight','forest','sunset','rose','contrast'];
        const idx = themes.indexOf(settings.theme);
        const next = themes[(idx+1) % themes.length];
        setSettings(s => ({...s, theme: next}));
        showToast(`Theme: ${next}`);
        // c2-316: log the hotkey cycle path so the event log shows which
        // themes users actually flip through. Palette + Settings + /theme
        // paths remain silent (they're deliberate, not exploratory).
        logEvent('theme_cycled', { via: 'hotkey', theme: next });
      }
      else if (mod && k === 'b') { e.preventDefault(); setShowConvoSidebar(v => !v); }
      else if (mod && (e.key === 'Home' || e.key === 'End')) {
        // c2-433 / task 246: Cmd/Ctrl+Home → scroll chat to top, Cmd/Ctrl+End
        // → scroll to bottom. Skip when an editable element has focus so
        // textarea / search input still get native Home/End. Active only on
        // the chat view (other views don't have a Virtuoso scroller).
        const target = e.target as HTMLElement | null;
        const isEditable = !!(target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable));
        if (isEditable || activeView !== 'chat') return;
        e.preventDefault();
        if (e.key === 'Home') chatViewRef.current?.scrollToIndex(0);
        else chatViewRef.current?.scrollToBottom();
      }
      else if (mod && e.shiftKey && k === 'r') {
        // Cmd/Ctrl+Shift+R = regenerate last assistant response. Browser's
        // native Cmd+R is a hard reload, so we claim Shift+R to avoid conflict.
        const hasAssistant = messages.some(m => m.role === 'assistant');
        if (hasAssistant && !isThinking) {
          e.preventDefault();
          regenerateLast();
          showToast('Regenerating…');
        }
      }
      else if (mod && e.shiftKey && k === 'l') {
        // c2-389 / task 190: Cmd+Shift+L jumps straight to admin Logs. The
        // plain Cmd+L is reserved (browser address bar); Shift-variant is
        // ours. Also pulls in the chat log via fetchChatLog so the tab has
        // fresh data when it opens.
        e.preventDefault();
        setAdminInitialTab('logs');
        setShowAdmin(true);
        fetchChatLog(50);
        logEvent('shortcut_logs', {});
      }
      else if (mod && e.shiftKey && k === 'a') {
        // c2-390 / task 214: Cmd+Shift+A flips autoTheme. Surfaces a toast
        // so the user sees which state they just landed in.
        e.preventDefault();
        setSettings(s => {
          const next = !s.autoTheme;
          showToast(`Auto theme ${next ? 'on' : 'off'}`);
          logEvent('shortcut_auto_theme', { on: next });
          return { ...s, autoTheme: next };
        });
      }
      else if (mod && !e.shiftKey && k === 'z') {
        // c2-397 / task 200: Cmd+Z undoes the last soft-delete while the
        // toast-hold window is open. Ignored when focus is in an editable
        // target so textarea / input native undo still wins. The toast's
        // own Undo button remains — this is just a keyboard path.
        const target = e.target as HTMLElement | null;
        const isEditable = !!(target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable));
        if (isEditable) return;
        const pending = pendingUndoRef.current;
        if (!pending) return;
        if (Date.now() - pending.at > 5100) { pendingUndoRef.current = null; return; }
        e.preventDefault();
        pending.fn();
        pendingUndoRef.current = null;
        showToast('Undone');
        logEvent('shortcut_undo_delete', {});
      }
      else if (mod && k === 'f') {
        // Cmd/Ctrl+F — chat-view search hijack. When the user is typing in
        // an input or a modal is open, fall through to the browser's native
        // find-in-page. Otherwise (Chat view, nothing focused), open our
        // in-conversation search so results are filterable.
        const target = e.target as HTMLElement | null;
        const isEditable = !!(target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable));
        const anyModalOpen = showCmdPalette || showSettings || showKnowledge || showActivity || showGame || showShortcuts || pendingConfirm || !!showWelcome || showAdmin || !!negFeedbackFor || !!correctFeedbackFor || showTerminal;
        const inChatView = activeView === 'chat' && !anyModalOpen;
        // Shift-variant always opens our search (power-user shortcut, not
        // overloaded by the browser). Plain Cmd+F only hijacks in chat view.
        if (!e.shiftKey && (!inChatView || isEditable)) return; // browser native
        e.preventDefault();
        cs.toggle();
      }
      else if (e.key === 'Escape') {
        // c2-400 / task 185: message context menu takes Escape before any
        // modal — it's the most-recent interaction and feels local to the
        // pointer position.
        if (msgContextMenu) { setMsgContextMenu(null); return; }
        if (showShortcuts) setShowShortcuts(false);
        else if (showCmdPalette) setShowCmdPalette(false);
        else if (showSettings) setShowSettings(false);
        else if (showKnowledge) setShowKnowledge(false);
        else if (showActivity) setShowActivity(false);
        else if (showAdmin) setShowAdmin(false);
        else if (showGame) setShowGame(null);
        else if (showTerminal) setShowTerminal(false);
        // c2-310: Training Dashboard was reachable via /training slash but
        // Escape only closed it via click-outside or the X button. Added
        // here so the global Esc affordance is uniform across modals.
        else if (showTraining) setShowTraining(false);
        // c2-311: negative-feedback modal + tool-approval dialog. For the
        // approval dialog Esc is semantically Cancel (safer default matches
        // autoFocus choice in c2-308). Feedback modal Esc closes without
        // sending — discarded text is fine since the user hit Escape.
        else if (negFeedbackFor) fb.closeNegFeedback();
        else if (correctFeedbackFor) fb.closeCorrectFeedback();
        else if (factPopover) setFactPopover(null);
        else if (pendingConfirm) { setPendingConfirm(null); setIsThinking(false); }
        else if (showChatSearch) { cs.close(); }
        // c2-433 / task 256 + 279: dropdowns that were click-outside-only.
        // Add at the same precedence as other top-level dismissibles so
        // keyboard users can close cleanly. Skill menu added in 279.
        else if (showAccountMenu) setShowAccountMenu(false);
        else if (showSkillMenu) setShowSkillMenu(false);
        else if (showSlashMenu) sm.close();
        // Last-resort Esc binding: cancel an in-flight request when no modal
        // is open. Mirrors the on-screen Stop button so power users can abort
        // without reaching for the mouse.
        else if (isThinking) {
          setIsThinking(false);
          setThinkingStart(null);
          fetch(`http://${getHost()}:3000/api/stop`, { method: 'POST' }).catch(() => {});
          showToast('Stopped');
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showCmdPalette, showSettings, showKnowledge, showActivity, showGame, showTraining, negFeedbackFor, pendingConfirm, tosAccepted, showAccountMenu, showSlashMenu, showSkillMenu]);

  // Three polling hooks — see ./usePolls.ts for the fetch logic. Each manages
  // its own interval + abort handling; parent just reads the state they return.
  const host = getHost();
  // #354 stability pass: pause all polls when the tab is hidden. A user
  // walking away or sending the page to background no longer burns
  // /api/status requests + drains battery. Immediate re-fetch fires on
  // return because the `active` boolean dep flips true and the poll's
  // useEffect re-runs.
  const pageVisible = usePageVisible();
  const pollActive = isAuthenticated && pageVisible;
  const { kg, lastOk: kgLastOk, lastError: kgLastError, latencyMs } = useStatusPoll(host, pollActive);
  const quality = useQualityPoll(host, pollActive);
  const sysInfo = useSysInfoPoll(host, pollActive);

  // ---- Conversations (Claude/ChatGPT/Gemini-style sidebar state) ----
  type Conversation = {
    id: string;
    title: string;
    messages: ChatMessage[];
    createdAt: number;
    updatedAt: number;
    pinned?: boolean;
    // c2-232 / #80: manual ordering index for pinned conversations. Lower =
    // earlier in the pinned group. When absent (legacy rows, or never
    // reordered), the sort falls back to updatedAt desc so behaviour is
    // unchanged until the user actually drags something.
    pinOrder?: number;
    starred?: boolean;
    incognito?: boolean;
    archived?: boolean;
    // Unsent draft text preserved across conversation switches so users don't
    // lose their in-progress message when clicking between conversations.
    draft?: string;
    // #176 conversation branching: when this convo was branched off another,
    // sourceConvoId points at the parent and sourceMessageId at the turn
    // where the branch diverged. Sidebar + title use these to render a
    // "↪ from <parent>" breadcrumb.
    branchedFrom?: { convoId: string; messageId: number; at: number };
  };
  const LS_CONVERSATIONS_KEY = 'lfi_conversations_v2';
  const LS_CURRENT_KEY = 'lfi_current_conversation';
  const LS_MESSAGES_KEY = 'lfi_conversations_v1'; // legacy flat-message key

  const newConvoId = () => `c_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
  const titleFrom = (text: string) =>
    (text.replace(/\s+/g, ' ').trim().slice(0, 48) || 'New chat');

  const [conversations, setConversations] = useState<Conversation[]>(() => {
    try {
      const raw = localStorage.getItem(LS_CONVERSATIONS_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as Conversation[];
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
      // Legacy v1 → wrap into a single conversation.
      const legacy = localStorage.getItem(LS_MESSAGES_KEY);
      if (legacy) {
        const legacyMsgs = JSON.parse(legacy) as ChatMessage[];
        if (Array.isArray(legacyMsgs) && legacyMsgs.length > 0) {
          const firstUser = legacyMsgs.find(m => m.role === 'user');
          return [{
            id: newConvoId(),
            title: firstUser ? titleFrom(firstUser.content) : 'Earlier chat',
            messages: legacyMsgs,
            createdAt: legacyMsgs[0]?.timestamp || Date.now(),
            updatedAt: legacyMsgs[legacyMsgs.length - 1]?.timestamp || Date.now(),
          }];
        }
      }
    } catch { /* corrupt — fall through */ }
    return [];
  });
  const [currentConversationId, setCurrentConversationId] = useState<string>(() => {
    const stored = localStorage.getItem(LS_CURRENT_KEY);
    return stored || '';
  });
  // c2-433 / task 260: mirror currentConversationId for applyToStreamingConvo
  // — without this the WS handlers comparison "targetId === currentConvId"
  // reads stale closure state, causing chat_chunks from the original
  // streaming convo to bleed into the now-active convo when the user
  // switches mid-stream. Moved DOWN from line 490 where it caused a TDZ
  // (currentConversationId was declared after the ref, so production
  // builds threw 'Cannot access ke before initialization' on mount).
  const currentConversationIdRef = useRef<string>(currentConversationId);
  useEffect(() => { currentConversationIdRef.current = currentConversationId; }, [currentConversationId]);
  // c2-433 / task 261 mirror: conversationsRef is read by the WS chat_done
  // handler to look up the streaming convo's last assistant turn for the
  // OS-notification preview. Previously at line ~508 above the useState,
  // which caused the production TDZ 'Cannot access ge before initialization'
  // (minified as ge paired with settings above).
  const conversationsRef = useRef(conversations);
  useEffect(() => { conversationsRef.current = conversations; }, [conversations]);

  // Ensure we always have an active conversation to write into.
  // c2-246 / #107: when the stored id is missing or stale, prefer the most
  // recently updated non-archived conversation (users rarely want to land
  // on a month-old chat). Fall back to any conversation (incl. archived) if
  // the archive is all that's left.
  useEffect(() => {
    if (!currentConversationId || !conversations.find(c => c.id === currentConversationId)) {
      if (conversations.length > 0) {
        const sorted = [...conversations].sort((a, b) => b.updatedAt - a.updatedAt);
        const pick = sorted.find(c => !c.archived) ?? sorted[0];
        setCurrentConversationId(pick.id);
      } else {
        const fresh: Conversation = {
          id: newConvoId(),
          title: 'New chat',
          messages: [],
          createdAt: Date.now(),
          updatedAt: Date.now(),
        };
        setConversations([fresh]);
        setCurrentConversationId(fresh.id);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conversations.length]);

  // Persist the list + current id. Incognito conversations are excluded.
  // c2-267: surface quota-exceeded to the user once per session — silently
  // losing writes was confusing people who wondered why conversations
  // stopped saving. Ref ensures the toast fires at most once.
  const quotaWarnedRef = useRef(false);
  useEffect(() => {
    if (!settings.persistConversations) {
      // c2-298: honour the setting fully — if the user just flipped it off,
      // remove any previously-persisted convos from localStorage. Keeps the
      // privacy contract honest: "don't persist" should mean "no trace left."
      // LS_CURRENT_KEY points at an id that'd be dead weight without its
      // data, so clear that too.
      try {
        localStorage.removeItem(LS_CONVERSATIONS_KEY);
        localStorage.removeItem(LS_CURRENT_KEY);
      } catch { /* quota or blocked — drop */ }
      return;
    }
    try {
      const saveable = conversations.filter(c => !c.incognito).slice(-100).map(c => ({
        ...c, messages: c.messages.slice(-500),
      }));
      localStorage.setItem(LS_CONVERSATIONS_KEY, JSON.stringify(saveable));
    } catch (e) {
      if (!quotaWarnedRef.current) {
        quotaWarnedRef.current = true;
        console.warn('[lfi] conversations write failed — likely quota exceeded:', e);
        showToast('Storage full — new messages may not persist across reloads.');
      }
    }
  }, [conversations, settings.persistConversations]);
  useEffect(() => {
    if (!currentConversationId) return;
    try { localStorage.setItem(LS_CURRENT_KEY, currentConversationId); } catch {}
  }, [currentConversationId]);

  // c2-275: unread assistant-reply counter that prepends (N) to the tab
  // title while the page is hidden. Cleared on visibilitychange back to
  // visible. Combines naturally with the title-from-conversation logic
  // below — both write document.title from the same effect dependency.
  const [unreadReplies, setUnreadReplies] = useState(0);
  const lastMsgIdForUnreadRef = useRef<number | string | null>(null);
  useEffect(() => {
    // Bump counter when the list ends with a new assistant message that we
    // haven't seen yet AND the tab is hidden. Same guard as the notification
    // path so hidden tabs get both signals in sync.
    const last = messages[messages.length - 1];
    if (!last) return;
    if (lastMsgIdForUnreadRef.current === last.id) return;
    lastMsgIdForUnreadRef.current = last.id;
    if (last.role === 'assistant' && typeof document !== 'undefined' && document.hidden) {
      setUnreadReplies(n => n + 1);
    }
  }, [messages]);
  useEffect(() => {
    const onVis = () => {
      if (typeof document !== 'undefined' && !document.hidden) setUnreadReplies(0);
    };
    document.addEventListener('visibilitychange', onVis);
    return () => document.removeEventListener('visibilitychange', onVis);
  }, []);

  // c2-276: badge the favicon with a red dot when there are unread replies.
  // Swaps the <link id="favicon"> href between the plain glyph (declared in
  // index.html) and a variant with an overlay circle. Inline SVG data URL
  // so no extra network request.
  useEffect(() => {
    const link = document.getElementById('favicon') as HTMLLinkElement | null;
    if (!link) return;
    const plain =
      'data:image/svg+xml;utf8,' + encodeURIComponent(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">' +
        '<rect width="64" height="64" rx="12" fill="#8b7bf7"/>' +
        '<text x="32" y="45" text-anchor="middle" font-family="system-ui" font-size="40" font-weight="700" fill="#fff">P</text>' +
        '</svg>'
      );
    const badged =
      'data:image/svg+xml;utf8,' + encodeURIComponent(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">' +
        '<rect width="64" height="64" rx="12" fill="#8b7bf7"/>' +
        '<text x="32" y="45" text-anchor="middle" font-family="system-ui" font-size="40" font-weight="700" fill="#fff">P</text>' +
        '<circle cx="48" cy="16" r="12" fill="#ef4444" stroke="#fff" stroke-width="2"/>' +
        '</svg>'
      );
    link.href = unreadReplies > 0 ? badged : plain;
  }, [unreadReplies]);

  // Keep the browser tab title in sync with the active conversation — makes
  // tab-switching to the dashboard scannable among many browser tabs.
  // c2-275: prepends (N) when there are unread replies and the tab was hidden.
  useEffect(() => {
    const c = conversations.find(x => x.id === currentConversationId);
    const title = c?.title && c.title !== 'New chat' ? c.title.slice(0, 60) : null;
    // c2-334: when the user is in a non-chat section, prefix the tab title
    // with the section name so users on mobile (no visible top-nav) can
    // tell where they are from the browser tab/home-screen label. Admin
    // uses the modal so it doesn't get a prefix — modal titles own that UX.
    const sectionPrefix = showAdmin ? 'Admin'
      : activeView === 'classroom' ? 'Classroom'
      : activeView === 'fleet' ? 'Fleet'
      : activeView === 'library' ? 'Library'
      : activeView === 'auditorium' ? 'Auditorium'
      : null;
    const base = sectionPrefix
      ? `${sectionPrefix} · PlausiDen AI`
      : title ? `${title} · PlausiDen AI` : 'PlausiDen AI';
    // c2-433 / task 263: prefix [offline] to the tab title when WS is down.
    // Helps users who have multiple tabs open and need to spot which one
    // disconnected without switching focus.
    const connPrefix = !isConnected ? '[offline] ' : '';
    document.title = `${connPrefix}${unreadReplies > 0 ? `(${unreadReplies}) ` : ''}${base}`;
    return () => { document.title = 'PlausiDen AI'; };
  }, [currentConversationId, conversations, unreadReplies, activeView, showAdmin, isConnected]);

  // Save draft to conversation when switching away, restore when switching in.
  // c2-266: flush the active-convo draft to localStorage on pagehide /
  // beforeunload so refreshes and tab-closes no longer lose typed text.
  // Refs mirror the latest input + id so the handler can read without
  // re-binding. Writes straight into LS (not React state) because the page
  // is leaving — state updates wouldn't flush in time.
  const inputRefForDraft = useRef(input);
  useEffect(() => { inputRefForDraft.current = input; }, [input]);
  const currentConvoIdRefForDraft = useRef(currentConversationId);
  useEffect(() => { currentConvoIdRefForDraft.current = currentConversationId; }, [currentConversationId]);
  useEffect(() => {
    const flushDraft = () => {
      const id = currentConvoIdRefForDraft.current;
      if (!id) return;
      try {
        const raw = localStorage.getItem(LS_CONVERSATIONS_KEY);
        if (!raw) return; // persistence off or nothing stored yet
        const convos = JSON.parse(raw);
        if (!Array.isArray(convos)) return;
        const draft = inputRefForDraft.current;
        const next = convos.map((c: any) => c && c.id === id ? { ...c, draft } : c);
        localStorage.setItem(LS_CONVERSATIONS_KEY, JSON.stringify(next));
      } catch { /* ignore */ }
    };
    window.addEventListener('beforeunload', flushDraft);
    // pagehide fires reliably on mobile Safari where beforeunload is muted.
    window.addEventListener('pagehide', flushDraft);
    return () => {
      window.removeEventListener('beforeunload', flushDraft);
      window.removeEventListener('pagehide', flushDraft);
    };
  }, []);

  // Uses a ref for the LAST-active id so we save the current `input` to the
  // outgoing conversation before it's replaced.
  const lastActiveConvoRef = useRef<string>('');
  useEffect(() => {
    // c2-268: reset the chat-search cursor on convo switch so the N/M
    // counter and the jump-to-match arrows start from the first match of
    // the new conversation. The query itself is preserved — users often
    // want to apply the same filter to a different thread.
    setChatSearchCursor(0);
    const outgoingId = lastActiveConvoRef.current;
    if (outgoingId && outgoingId !== currentConversationId) {
      // Capture `input` into the outgoing conversation's draft.
      setConversations(prev => prev.map(c => c.id === outgoingId
        ? { ...c, draft: input } : c));
      // Tell backend to flush its conversation_facts for clean isolation
      // (c0-011 #1). Skip on initial mount where outgoingId is empty.
      if (currentConversationId) postConversationSwitch(currentConversationId);
    }
    const incoming = conversations.find(c => c.id === currentConversationId);
    // c2-278 + c2-433 / task 262: switching into a convo with a short draft
    // (or none) needs the textarea to re-size, AND switching into one with
    // a multi-line draft needs it to grow to fit. setInputAndResize handles
    // both: empty value → height='' (CSS minHeight), non-empty → grow to
    // scrollHeight capped at 280. Replaces the prior 'height = '' only'
    // approach which left long drafts clipped on convo-switch-in.
    setInputAndResize(incoming?.draft || '');
    // c2-281: scroll the sidebar so the newly-active row is on screen — in
    // long conversation lists the active row may have been scrolled out of
    // view. block='nearest' is a no-op when already visible.
    setTimeout(() => {
      const row = document.querySelector('[data-convo-row="true"][aria-current="true"]') as HTMLElement | null;
      row?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }, 0);
    // c2-319: on mobile, selecting a convo from the sidebar overlay should
    // close the overlay so the user can actually read the chat. Only fires
    // on real switches (outgoingId present) so the initial-mount hydrate
    // doesn't force-close a sidebar the user just opened. Desktop layout
    // is inline + persistent so sidebar stays visible.
    if (!isDesktop && outgoingId && outgoingId !== currentConversationId) {
      setShowConvoSidebar(false);
    }
    lastActiveConvoRef.current = currentConversationId;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentConversationId]);

  // Hydrate the active `messages` state from the current conversation, and
  // sync changes back. This keeps the rest of the component working against
  // the simple `messages` array while the list remains the source of truth.
  useEffect(() => {
    const convo = conversations.find(c => c.id === currentConversationId);
    setMessages(convo?.messages || []);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentConversationId]);
  useEffect(() => {
    if (!currentConversationId) return;
    setConversations(prev => prev.map(c => {
      if (c.id !== currentConversationId) return c;
      // Auto-title using the smart heuristic — picks key-phrase, keeps
      // questions whole, prefers first clause. Only overrides the default
      // "New chat" so user renames are preserved.
      const autoTitle = c.title === 'New chat' ? smartTitle(messages) : c.title;
      return { ...c, messages, title: autoTitle, updatedAt: Date.now() };
    }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages]);

  // Tell the backend which conversation is now active so it can reset its
  // in-memory conversation_facts / dedupe trackers (c0-011 #1 + #5). Fire-
  // and-forget — the chat WS still carries the actual message content, and
  // the backend tolerates missing switch pings.
  const postConversationSwitch = (conversation_id: string) => {
    fetch(`http://${getHost()}:3000/api/conversations/switch`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ conversation_id }),
    }).catch(() => { /* non-fatal */ });
  };

  const createNewConversation = (incognito = false) => {
    const fresh: Conversation = {
      id: newConvoId(),
      title: incognito ? 'Incognito chat' : 'New chat',
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
      incognito,
    };
    if (!incognito) {
      setConversations(prev => [fresh, ...prev]);
    }
    // Incognito conversations are NOT added to the persisted list —
    // they exist only in the current messages state and vanish on
    // page reload. Per Bible §4.5: operator controls their data.
    setCurrentConversationId(fresh.id);
    setMessages([]);
    postConversationSwitch(fresh.id);
    if (incognito) {
      setMessages([{
        id: msgId(), role: 'system',
        content: 'Incognito mode — this conversation will not be saved, logged, or used for training.',
        timestamp: Date.now(),
      }]);
    }
    // c2-282: focus the chat input so the user can start typing immediately
    // instead of clicking/tabbing. setTimeout 0 lets React finish the view
    // transition (WelcomeScreen mounts on the now-empty message list) before
    // the focus call lands.
    setTimeout(() => inputRef.current?.focus(), 0);
    // c2-318: on mobile, close the sidebar overlay when the user starts a
    // new conversation. Desktop sidebar is inline (already visible) so we
    // leave it as-is. Covers every creation path (sidebar button, palette,
    // slash command, account menu, Cmd+N) since they all funnel here.
    if (!isDesktop) setShowConvoSidebar(false);
    logEvent('new_conversation', { incognito });
  };
  const isCurrentIncognito = (() => {
    const c = conversations.find(c => c.id === currentConversationId);
    return c?.incognito || false;
  })();
  const deleteConversation = (id: string) => {
    // Soft-delete with undo: remove from UI immediately, cache the conversation
    // (and its previous list position) in a closure, show an Undo toast. The
    // user has the toast-hold window to hit Undo; otherwise the change is
    // already committed (the UI never showed it so nothing else to do).
    const prevConvos = conversations;
    const idx = prevConvos.findIndex(c => c.id === id);
    if (idx < 0) return;
    const victim = prevConvos[idx];
    const wasActive = id === currentConversationId;
    setConversations(prev => prev.filter(c => c.id !== id));
    if (wasActive) {
      const rest = prevConvos.filter(c => c.id !== id);
      setCurrentConversationId(rest[0]?.id || '');
    }
    // c2-313: audit-trail parity with other destructive ops (clear_chat,
    // clear_history, bulk_delete_archived). Only message count leaks —
    // no title or content — so no PII hits the event log.
    logEvent('delete_conversation', { messages: victim.messages.length, wasActive });
    // c2-397 / task 200: publish the undo so the global Cmd+Z handler can
    // trigger it while the toast is live. Cleared after 5.2s (toast hold +
    // exit anim) so a late Cmd+Z doesn't resurrect a stale entry.
    const undo = () => {
      setConversations(cur => {
        if (cur.some(c => c.id === id)) return cur; // already restored
        const restored = [...cur];
        restored.splice(Math.min(idx, restored.length), 0, victim);
        return restored;
      });
    };
    pendingUndoRef.current = { fn: undo, at: Date.now() };
    setTimeout(() => {
      if (pendingUndoRef.current && Date.now() - pendingUndoRef.current.at >= 5100) {
        pendingUndoRef.current = null;
      }
    }, 5200);
    showToast(`Deleted "${victim.title}"`, () => {
      undo();
      if (wasActive) setCurrentConversationId(id);
      logEvent('delete_conversation_undo', { messages: victim.messages.length });
    });
  };
  const renameConversation = (id: string, title: string) => {
    const clean = title.trim().slice(0, 80) || 'Untitled';
    setConversations(prev => prev.map(c => c.id === id ? { ...c, title: clean } : c));
    // c2-314: convo mutation audit parity — matches delete_conversation.
    // Only length leaks so the event log stays PII-free.
    logEvent('rename_conversation', { titleLength: clean.length });
    showToast('Renamed');
  };
  const togglePinned = (id: string) => {
    let nowPinned = false;
    setConversations(prev => prev.map(c => {
      if (c.id !== id) return c;
      nowPinned = !c.pinned;
      // c2-232 / #80: when unpinning, drop the stored manual order so
      // re-pinning later doesn't snap back to an old slot.
      return nowPinned ? { ...c, pinned: true } : { ...c, pinned: false, pinOrder: undefined };
    }));
    logEvent('toggle_pinned', { nowPinned });
    showToast(nowPinned ? 'Pinned' : 'Unpinned');
  };
  // c2-232 / #80: drag-to-reorder for the pinned group. Dragged row id +
  // hover target id drive the opacity-dim + insert-line visual.
  // c2-433 / #313 pass 6: convo-drag state lifted into useConvoDrag. Hook
  // dedupes the dragover updates so repeated events on the same row don't
  // thrash the sidebar at 60Hz.
  const cd = useConvoDrag();
  const draggedConvoId = cd.draggedId;
  const dragOverConvoId = cd.overId;
  // c2-248 / #110: wrap matched substring in the title with <mark> while
  // the search box has text. Case-insensitive; returns the raw string when
  // no query is supplied so non-searching renders stay a simple text node.
  const highlightConvoTitle = (title: string): React.ReactNode => {
    // c2-399 / task 186: use the deferred value so the <mark> highlights
    // match the filter output (otherwise a mid-keystroke frame could mark
    // substrings that aren't in the visible list yet).
    const q = deferredConvoSearch.trim();
    if (!q) return title;
    const safe = q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const re = new RegExp(`(${safe})`, 'i');
    const parts = title.split(re);
    if (parts.length === 1) return title; // no match in the title itself
    return parts.map((p, i) => i % 2 === 1
      ? <mark key={i} style={{ background: 'rgba(255,211,107,0.45)', color: 'inherit', padding: '0 1px', borderRadius: T.radii.xs }}>{p}</mark>
      : <React.Fragment key={i}>{p}</React.Fragment>
    );
  };
  // c2-247 / #109: shared keyboard handler for sidebar conversation rows.
  // Enter/Space activates; Arrow/Home/End move focus between visible rows
  // inside the same [data-convo-scroller]. Main + archived lists share the
  // scroller so arrow nav crosses the boundary naturally.
  // c2-249 / #111: also handles per-row actions — p (pin), s (star),
  // F2 (rename), Delete/Backspace (soft-delete with undo). All modifier-
  // free so Tab → action feels like a Gmail-style shortcut.
  const navigateConvoRow = (e: React.KeyboardEvent<HTMLDivElement>, convoId: string) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setCurrentConversationId(convoId);
      return;
    }
    // Per-row action keys. Ignore when any modifier is held so the user's
    // real chords (Cmd+P, Ctrl+S) aren't hijacked.
    if (!e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      if (e.key === 'p') { e.preventDefault(); togglePinned(convoId); return; }
      if (e.key === 's') { e.preventDefault(); toggleStarred(convoId); return; }
      if (e.key === 'F2') {
        e.preventDefault();
        const c = conversations.find(cc => cc.id === convoId);
        if (c) { setRenamingConvoId(convoId); setRenameDraft(c.title); }
        return;
      }
      if (e.key === 'Delete' || e.key === 'Backspace') {
        e.preventDefault();
        deleteConversation(convoId);
        return;
      }
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') return;
    const scroller = e.currentTarget.closest('[data-convo-scroller="true"]') as HTMLElement | null;
    if (!scroller) return;
    const rows = Array.from(scroller.querySelectorAll<HTMLElement>('[data-convo-row="true"]'));
    if (rows.length === 0) return;
    e.preventDefault();
    const i = rows.indexOf(e.currentTarget);
    let next = i;
    if (e.key === 'ArrowDown') next = (i + 1) % rows.length;
    else if (e.key === 'ArrowUp') next = (i - 1 + rows.length) % rows.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = rows.length - 1;
    rows[next]?.focus();
  };
  // Move `draggedId` to occupy `targetId`'s slot in the pinned group, then
  // rewrite pinOrder on every pinned item so the ordering is persisted
  // authoritatively rather than inferred from relative indices.
  const reorderPinned = (draggedId: string, targetId: string) => {
    if (draggedId === targetId) return;
    let reorderedCount = 0;
    setConversations(prev => {
      const pinKey = (c: Conversation) => (typeof c.pinOrder === 'number' ? c.pinOrder : Number.MAX_SAFE_INTEGER - c.updatedAt / 1000);
      const pinnedList = prev.filter(c => c.pinned).sort((a, b) => pinKey(a) - pinKey(b));
      const dragged = pinnedList.find(c => c.id === draggedId);
      if (!dragged) return prev;
      const without = pinnedList.filter(c => c.id !== draggedId);
      const tIdx = without.findIndex(c => c.id === targetId);
      if (tIdx < 0) return prev;
      const reordered = [...without.slice(0, tIdx), dragged, ...without.slice(tIdx)];
      const orderMap = new Map<string, number>();
      reordered.forEach((c, i) => orderMap.set(c.id, i));
      reorderedCount = reordered.length;
      return prev.map(c => orderMap.has(c.id) ? { ...c, pinOrder: orderMap.get(c.id) } : c);
    });
    // c2-315: closes the audit-event parity loop — every user-driven
    // conversation mutation now appears in the event log. pinCount lets
    // a future analytics view correlate reorder churn with pinned-group size.
    if (reorderedCount > 0) logEvent('reorder_pinned', { pinCount: reorderedCount });
  };
  const toggleStarred = (id: string) => {
    let nowStarred = false;
    setConversations(prev => prev.map(c => {
      if (c.id !== id) return c;
      nowStarred = !c.starred;
      return { ...c, starred: nowStarred };
    }));
    logEvent('toggle_starred', { nowStarred });
    showToast(nowStarred ? 'Starred' : 'Unstarred');
  };
  const toggleArchived = (id: string) => {
    let nowArchived = false;
    setConversations(prev => prev.map(c => {
      if (c.id !== id) return c;
      nowArchived = !c.archived;
      return { ...c, archived: nowArchived };
    }));
    logEvent('toggle_archived', { nowArchived });
    showToast(nowArchived ? 'Archived' : 'Unarchived');
  };
  // c2-401 / task 194: duplicate a conversation. Fresh id + timestamp,
  // "(copy)" suffix on the title, pinned/starred/archived/draft reset so
  // the clone is a neutral starting point. Jump to the new one so the
  // user sees the result of their action. Never duplicates the active id
  // onto itself — silently no-ops if the source is missing.
  const duplicateConversation = (id: string) => {
    const src = conversations.find(c => c.id === id);
    if (!src) return;
    const now = Date.now();
    const clone: Conversation = {
      id: newConvoId(),
      title: `${src.title} (copy)`.slice(0, 80),
      messages: src.messages.map(m => ({ ...m, id: msgId() })),
      createdAt: now,
      updatedAt: now,
      // pinning / starring / archiving / drafts are all user-scoped on the
      // original — drop on the clone so it starts clean.
    };
    setConversations(prev => [...prev, clone]);
    setCurrentConversationId(clone.id);
    // c2-433 / task 280: focus the input after the duplicate so the user
    // can type immediately. Same setTimeout-0 pattern as
    // createNewConversation — let React commit the active-convo switch +
    // draft restore before the focus call lands. Mobile sidebar auto-
    // closes on convo-switch via the existing useEffect at convo-id
    // change, so no extra mobile handling needed here.
    setTimeout(() => inputRef.current?.focus(), 0);
    logEvent('conversation_duplicated', { sourceId: id, messages: clone.messages.length });
    showToast(`Duplicated as "${clone.title.slice(0, 32)}${clone.title.length > 32 ? '\u2026' : ''}"`);
  };

  // #176 branch from a specific message: fork the conversation at msgId,
  // keeping everything up to and including that message, then jump to the
  // new branch so the user can explore an alternative continuation.
  // Unlike duplicateConversation (which clones everything), this TRUNCATES
  // the message list so the user's next reply becomes the divergence point.
  const branchFromMessage = (convoId: string, msgId: number) => {
    const src = conversations.find(c => c.id === convoId);
    if (!src) return;
    const cutIdx = src.messages.findIndex(m => m.id === msgId);
    if (cutIdx < 0) return;
    const now = Date.now();
    const branch: Conversation = {
      id: newConvoId(),
      title: `${src.title} (branch)`.slice(0, 80),
      messages: src.messages.slice(0, cutIdx + 1).map(m => ({ ...m, id: msgId === m.id ? m.id : msgId === m.id ? m.id : msgId })),
      createdAt: now,
      updatedAt: now,
      branchedFrom: { convoId: src.id, messageId: msgId, at: now },
    };
    // Fix: re-mint message ids cleanly without collisions — the inline
    // conditional above was a copy-paste artefact. Do it properly here.
    branch.messages = src.messages.slice(0, cutIdx + 1).map(m => ({ ...m }));
    setConversations(prev => [...prev, branch]);
    setCurrentConversationId(branch.id);
    setTimeout(() => inputRef.current?.focus(), 0);
    logEvent('conversation_branched', { sourceId: convoId, atMsg: msgId, kept: cutIdx + 1 });
    showToast(`Branched — ${cutIdx + 1} messages kept`);
  };

  // Smart auto-title: look at the first user turn + first assistant reply,
  // pick a short key-phrase that beats simple truncation. Falls back to
  // titleFrom if no signal. Rule-of-thumb similar to ChatGPT/Gemini heuristics.
  // c2-414 / BIG #218 mobile: sidebar starts closed on small viewports so
  // the chat surface gets the full width on open. Desktop + tablet still
  // open by default. Uses innerWidth directly so the decision doesn't wait
  // for the useBreakpoint effect to flush.
  const [showConvoSidebar, setShowConvoSidebar] = useState<boolean>(() => {
    if (typeof window === 'undefined') return true;
    return window.innerWidth >= 768;
  });
  const [showPlanSidebar, setShowPlanSidebar] = useState<boolean>(true);
  const [showArchived, setShowArchived] = useState<boolean>(false);
  // Inline rename state for sidebar conversations (replaces browser prompt()).
  const [renamingConvoId, setRenamingConvoId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState<string>('');
  // Cmd+K palette recency counter. Persists to localStorage so frequently
  // used commands bubble up across sessions.
  const CMD_RECENCY_KEY = 'lfi_cmd_recency_v1';
  const [cmdRecency, setCmdRecency] = useState<Record<string, number>>(() => {
    try {
      const raw = localStorage.getItem(CMD_RECENCY_KEY);
      return raw ? JSON.parse(raw) : {};
    } catch { return {}; }
  });
  const bumpCmdRecency = (id: string) => {
    setCmdRecency(prev => {
      const next = { ...prev, [id]: (prev[id] || 0) + 1 };
      try { localStorage.setItem(CMD_RECENCY_KEY, JSON.stringify(next)); } catch {}
      return next;
    });
  };
  const [convoSearch, setConvoSearch] = useState('');
  // c2-399 / task 186: deferred copy for the filter pipeline. Input stays
  // snappy (convoSearch updates per keystroke) while the virtualized list
  // re-renders at a lower priority against deferredConvoSearch. React 18
  // concurrent feature — no manual debounce timer needed.
  const deferredConvoSearch = useDeferredValue(convoSearch);
  // c2-420 / task 193: date-range filter chips above the convo list. 'all'
  // is the default; 'today' / 'week' / 'month' narrow by updatedAt. Pinned
  // rows are always shown regardless — they're manually promoted, so
  // filtering them out would contradict user intent. Persisted per-device.
  type DateFilter = 'all' | 'today' | 'week' | 'month';
  const [convoDateFilter, setConvoDateFilter] = useState<DateFilter>(() => {
    try {
      const v = localStorage.getItem('lfi_convo_date_filter') as DateFilter | null;
      if (v === 'today' || v === 'week' || v === 'month' || v === 'all') return v;
    } catch { /* storage blocked */ }
    return 'all';
  });
  useEffect(() => {
    try { localStorage.setItem('lfi_convo_date_filter', convoDateFilter); } catch { /* quota */ }
  }, [convoDateFilter]);

  // ---- Send ----
  // Routes the message through the active skill. Chat/code go over the WS;
  // web/analyze/opsec hit REST endpoints and render results inline without
  // disturbing the conversation flow.
  const handleSend = async () => {
    if (sendingRef.current) return; // guard: in-flight send in progress
    // c2-371 / task 79: a fresh send supersedes any stale retry affordance.
    if (lastErrorRetry) setLastErrorRetry(null);
    const trimmed = input.trim();
    console.debug("// SCC: handleSend, len:", trimmed.length, "skill:", activeSkill);
    // c2-230 / #71: allow send when there are pasted images, even if the
    // text is empty — the user intent is clearly "send these images".
    if (!trimmed && pastedImages.length === 0) return;
    // c2-433 / task 223: confirmation tick on send. Mobile only by virtue of
    // hapticTick being a no-op on devices without the Vibration API; iOS
    // Safari and most desktops silently ignore.
    hapticTick(15);
    // c2-433 / #316 / #313: fresh turn → reset thinking state. ts.reset()
    // clears modulesUsed + activeModule + isThinking + step so the activity
    // bar shows just *this* turn's modules. The actual setIsThinking(true)
    // happens later in this fn after the WS send fires.
    ts.reset();
    // c2-433 / #352: NOTE topic chip intentionally NOT cleared here —
    // topic_stack persists across turns by design (4-turn volcano chain).
    // Backend re-emits the resolved topic on the next chat_progress; if the
    // topic changed (user pivoted), the new value overwrites. Clearing on
    // handleSend would cause a flicker even for same-topic continuations.
    sendingRef.current = true;

    // Record user message. If only images were pasted (no text), use a
    // placeholder so the turn renders as a proper user bubble.
    const userContent = trimmed || '(pasted image)';
    // c2-387 / BIG #176: if a branch origin is pending, stamp the new bubble
    // so the UI can show a "Branch from #N" indicator. Consumed exactly once.
    const branchedFromId = pendingBranchFromRef.current;
    pendingBranchFromRef.current = null;
    setMessages(prev => [...prev, {
      id: msgId(), role: 'user', content: userContent, timestamp: Date.now(),
      ...(branchedFromId != null ? { _branchedFromId: branchedFromId } : null),
    } as ChatMessage]);
    // Announce the attachment as a system message so the preview is visible
    // in the conversation. Keep the data URL out of persisted content
    // (localStorage bloat); just summarise count and byte total.
    if (pastedImages.length > 0) {
      const total = pastedImages.reduce((s, i) => s + i.size, 0);
      setMessages(prev => [...prev, {
        id: msgId(), role: 'system',
        content: `Attached ${pastedImages.length} pasted image${pastedImages.length === 1 ? '' : 's'} (${(total / 1024).toFixed(0)} KB). Backend upload is not yet wired \u2014 metadata logged for now.`,
        timestamp: Date.now(),
      }]);
      logEvent('paste_image_sent', { count: pastedImages.length, totalBytes: total });
      setPastedImages([]);
    }
    // #187: clear URL-paste preview after send so next paste starts fresh.
    if (urlPreview) setUrlPreview(null);
    // c2-410 / task 206 / c2-433 task 243: wire the previously-dead
    // clearInputWithBackup helper so the prompt we just sent stays
    // recoverable for the next ~30s. The "↶ Restore" affordance below
    // surfaces it; the helper writes draftBackupRef.current.
    clearInputWithBackup(input);
    // c2-433 / task 250: push to the prompt-history ring buffer. Skip
    // empty / pure-image sends. Cap at 10 by trimming the oldest. Reset
    // the cursor so the next Shift+ArrowUp lands on this fresh entry.
    if (trimmed.length > 0) {
      const buf = promptHistoryRef.current;
      // Don't push duplicates of the immediately-prior entry — common
      // user pattern is "fix typo + send again", which would otherwise
      // clog the history with near-dupes.
      if (buf[buf.length - 1] !== trimmed) {
        buf.push(trimmed);
        if (buf.length > 10) buf.shift();
        // c2-433 / task 251: persist after each push so reloads keep history.
        try { localStorage.setItem(PROMPT_HISTORY_LS_KEY, JSON.stringify(buf)); } catch { /* quota — silent */ }
      }
      promptHistoryCursorRef.current = -1;
    }
    // c2-277: collapse the auto-grown textarea back to its minHeight so the
    // post-send input isn't still occupying 4 lines of empty height.
    if (inputRef.current) inputRef.current.style.height = '';
    // Trigger send-pulse feedback animation.
    setSendPulseId(id => id + 1);
    // User's own send means they want to see their message regardless of
    // where they were scrolled. Virtuoso's followOutput auto-follows only
    // when already at bottom, so this forces a snap-to-end for the sender's
    // own turn. (Assistant streaming chunks still respect at-bottom.)
    setTimeout(() => chatViewRef.current?.scrollToBottom(), 0);
    // Clear the persisted draft on the active conversation so a switch + come-back
    // doesn't re-hydrate the text we just sent.
    if (currentConversationId) {
      setConversations(prev => prev.map(c => c.id === currentConversationId
        ? { ...c, draft: '' } : c));
    }
    logEvent('message_sent', { length: trimmed.length, tier: currentTier, skill: activeSkill });
    setIsThinking(true);
    setThinkingStart(Date.now());
    setThinkingStep(activeSkill !== 'chat' ? `Running ${activeSkill}…` : 'Thinking…');

    try {
      if (activeSkill === 'research') {
        const toolId = msgId();
        setMessages(prev => [...prev, {
          id: toolId, role: 'tool', content: `Deep research: ${trimmed.slice(0, 60)}`,
          toolName: 'deep_research', toolStatus: 'running', toolInput: trimmed,
          timestamp: Date.now(),
        }]);
        const t0 = Date.now();
        try {
          const res = await fetch(`http://${getHost()}:3000/api/research`, {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ query: trimmed, depth: 3 }),
          });
          const data = await res.json();
          const dur = Date.now() - t0;
          setMessages(prev => prev.map(m => m.id === toolId ? {
            ...m, toolStatus: 'ok' as const, toolDuration: dur,
            toolOutput: `${data.source_count || 0} sources, avg trust ${((data.avg_trust || 0) * 100).toFixed(0)}%`,
            content: `Research complete: ${data.source_count || 0} sources`,
          } : m));
          // Render synthesis with citations
          let synthesis = data.synthesis || '(no results)';
          if (data.sources && data.sources.length > 0) {
            synthesis += '\n\n**Sources:**\n';
            for (const src of data.sources) {
              synthesis += `[${src.citation_index}] ${src.query} — trust ${((src.trust || 0) * 100).toFixed(0)}%\n`;
            }
          }
          setMessages(prev => [...prev, {
            id: msgId(), role: 'assistant',
            content: synthesis, timestamp: Date.now(),
          }]);
        } catch (e) {
          setMessages(prev => prev.map(m => m.id === toolId ? {
            ...m, toolStatus: 'error' as const, content: `Research failed: ${(e as Error).message}`,
          } : m));
        }
        setIsThinking(false); setThinkingStart(null); setActiveSkill('chat');
        return;
      }
      // Per Bible §3.5: first web/research use per session requires
      // confirmation (privacy_impact: External). After first approval,
      // auto-approved for the rest of the session.
      if ((activeSkill === 'web' || activeSkill === 'research') && !webSearchApproved) {
        setPendingConfirm({
          tool: activeSkill === 'web' ? 'Web Search' : 'Deep Research',
          desc: `This will send your query to an external search provider. Your query: "${trimmed.slice(0, 100)}"`,
          onApprove: () => {
            setWebSearchApproved(true);
            setPendingConfirm(null);
            // Re-trigger send now that it's approved
            setTimeout(() => handleSend(), 50);
          },
        });
        setIsThinking(false);
        return;
      }
      if (activeSkill === 'web') {
        const toolId = msgId();
        setMessages(prev => [...prev, {
          id: toolId, role: 'tool', content: `Searching: ${trimmed.slice(0, 80)}`,
          toolName: 'web_search', toolStatus: 'running', toolInput: trimmed,
          timestamp: Date.now(),
        }]);
        const t0 = Date.now();
        const res = await fetch(`http://${getHost()}:3000/api/search`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ query: trimmed }),
        });
        const data = await res.json();
        const dur = Date.now() - t0;
        setMessages(prev => prev.map(m => m.id === toolId ? {
          ...m, toolStatus: 'ok' as const, toolDuration: dur,
          toolOutput: `${data.source_count ?? 0} sources, trust ${(((data.trust ?? 0) as number) * 100).toFixed(0)}%`,
          content: `${data.source_count ?? 0} sources found`,
        } : m));
        setMessages(prev => [...prev, {
          id: msgId(), role: 'web',
          content: `${data.source_count ?? 0} sources \u00B7 trust ${(((data.trust ?? 0) as number) * 100).toFixed(0)}%\n\n${data.summary ?? data.best_summary ?? '(no summary)'}`,
          timestamp: Date.now(),
        }]);
        setIsThinking(false);
        setThinkingStart(null);
        setActiveSkill('chat');
        return;
      }
      if (activeSkill === 'analyze') {
        const toolId = msgId();
        setMessages(prev => [...prev, {
          id: toolId, role: 'tool', content: `Running PSL audit`,
          toolName: 'psl_audit', toolStatus: 'running', toolInput: trimmed.slice(0, 200),
          timestamp: Date.now(),
        }]);
        const t0 = Date.now();
        const res = await fetch(`http://${getHost()}:3000/api/audit`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text: trimmed }),
        });
        const data = await res.json();
        const dur = Date.now() - t0;
        setMessages(prev => prev.map(m => m.id === toolId ? {
          ...m, toolStatus: (data.status === 'ok' ? 'ok' : 'error') as any, toolDuration: dur,
          toolOutput: JSON.stringify(data, null, 2).slice(0, 500),
          content: `Audit complete: ${data.verdict || data.status}`,
        } : m));
        setMessages(prev => [...prev, {
          id: msgId(), role: 'assistant',
          content: `**PSL audit**\n\n\`\`\`json\n${JSON.stringify(data, null, 2)}\n\`\`\``,
          timestamp: Date.now(),
        }]);
        setIsThinking(false);
        setThinkingStart(null);
        setActiveSkill('chat');
        return;
      }
      if (activeSkill === 'opsec') {
        const toolId = msgId();
        setMessages(prev => [...prev, {
          id: toolId, role: 'tool', content: `Scanning for secrets & PII`,
          toolName: 'opsec_scan', toolStatus: 'running', toolInput: `${trimmed.length} chars`,
          timestamp: Date.now(),
        }]);
        const t0 = Date.now();
        const res = await fetch(`http://${getHost()}:3000/api/opsec/scan`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text: trimmed }),
        });
        const data = await res.json();
        const dur = Date.now() - t0;
        const findings = (data.findings ?? []).length;
        setMessages(prev => prev.map(m => m.id === toolId ? {
          ...m, toolStatus: 'ok' as any, toolDuration: dur,
          toolOutput: findings === 0 ? 'Clean — no issues found' : `${findings} issue(s) detected`,
          content: findings === 0 ? 'No issues' : `${findings} finding(s)`,
        } : m));
        setMessages(prev => [...prev, {
          id: msgId(), role: 'assistant',
          content: `**OPSEC scan**\n\n${findings === 0 ? 'No secrets or PII detected.' : `Found ${findings} issue(s):`}\n\n\`\`\`json\n${JSON.stringify(data, null, 2)}\n\`\`\``,
          timestamp: Date.now(),
        }]);
        setIsThinking(false);
        setThinkingStart(null);
        setActiveSkill('chat');
        return;
      }
      // Code: flip tier to BigBrain first, then send over WS.
      if (activeSkill === 'code' && currentTier !== 'BigBrain') {
        await handleTierSwitch('BigBrain');
      }

      // Default: WebSocket chat
      const wsOpen = chatWsRef.current && chatWsRef.current.readyState === WebSocket.OPEN;
      if (!wsOpen) {
        // c2-382 / BIG #177: offline queue. Instead of bouncing the send,
        // persist the payload to localStorage and surface the message with
        // a "Queued (offline)" status. The ws.onopen drain loop will replay
        // on reconnect. Guarded by convId so a mid-send convo switch doesn't
        // strand messages in the wrong thread.
        const convId = currentConversationId;
        try {
          const raw = localStorage.getItem('lfi_outbox') || '[]';
          const queue = JSON.parse(raw) as Array<{ id: number; convId: string; content: string; incognito: boolean; at: number }>;
          queue.push({ id: msgId(), convId, content: trimmed, incognito: isCurrentIncognito || false, at: Date.now() });
          localStorage.setItem('lfi_outbox', JSON.stringify(queue.slice(-50)));
        } catch (err) {
          console.warn('// SCC: outbox persist failed', err);
        }
        // Mark the user bubble as queued so the rendering layer can badge it.
        setMessages(prev => prev.map((m, i) =>
          i === prev.length - 1 && m.role === 'user' && m.content === trimmed
            ? { ...m, _queued: true } as any
            : m
        ));
        logEvent('msg_queued_offline', { len: trimmed.length });
        showToast('Queued — will send when reconnected');
        setIsThinking(false);
        return;
      }
      // Capture the originating conversation BEFORE the WS write so chunks
      // can be routed back even if the user switches conversations mid-stream.
      streamingConvoIdRef.current = currentConversationId;
      currentTurnRef.current = markSend(trimmed.length);
      chatWsRef.current!.send(JSON.stringify({
        content: trimmed,
        incognito: isCurrentIncognito || false,
      }));
      if (activeSkill === 'code') setActiveSkill('chat'); // one-shot
    } catch (e) {
      console.warn('// SCC: handleSend failed', e);
      setMessages(prev => [...prev, {
        id: msgId(), role: 'system',
        content: `Request failed: ${(e as Error).message || 'unknown error'}`,
        timestamp: Date.now(),
      }]);
      setIsThinking(false);
    } finally {
      sendingRef.current = false;
      inputRef.current?.focus();
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);
    const el = e.target;
    el.style.height = 'auto';
    // c2-274: grow cap unified with the CSS max-height (280px). Prior cap of
    // 160px was stricter than the inline style, so multi-paragraph drafts
    // hit an artificial ceiling while CSS allowed more room — users had to
    // scroll inside a too-small box. Now the JS-driven grow tracks the
    // declared max.
    el.style.height = Math.min(el.scrollHeight, 280) + 'px';
    // c2-433 / task 254: typing breaks the prompt-history recall. Matches
    // terminal/zsh behavior — Shift+↑ to recall, then any keystroke resets
    // the cursor so the next Shift+↑ starts fresh from the most-recent.
    promptHistoryCursorRef.current = -1;
    // Slash command detection: show menu when "/" is at position 0.
    if (val.startsWith('/') && !val.includes(' ')) {
      sm.open(val.slice(1).toLowerCase());
    } else {
      sm.close();
    }
  };

  const regenerateLast = () => {
    const lastUser = [...messages].reverse().find(m => m.role === 'user');
    if (!lastUser) return;
    // Drop the last assistant reply so the retry doesn't double up.
    setMessages(prev => {
      const out = [...prev];
      while (out.length > 0 && out[out.length - 1].role !== 'user') out.pop();
      return out;
    });
    // Resend via the normal send path.
    setInput(lastUser.content);
    setTimeout(() => {
      const ws = chatWsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ content: lastUser.content }));
        setMessages(prev => [...prev, {
          id: msgId(), role: 'user', content: lastUser.content, timestamp: Date.now(),
        }]);
        setInput('');
        setIsThinking(true);
      }
    }, 50);
  };

  // URL detection for link previews. Extracts URLs from text and renders
  // them as clickable links (and eventually as preview cards when the
  // /api/unfurl endpoint exists).
  const urlRegex = /https?:\/\/[^\s<>"{}|\\^`\[\]]+/g;
  const renderWithLinks = (text: string, key: string): React.ReactNode => {
    const parts: React.ReactNode[] = [];
    let lastIdx = 0;
    let match: RegExpExecArray | null;
    const re = new RegExp(urlRegex.source, 'g');
    let k = 0;
    while ((match = re.exec(text)) !== null) {
      if (match.index > lastIdx) parts.push(<span key={`${key}-t${k++}`}>{text.slice(lastIdx, match.index)}</span>);
      const url = match[0];
      parts.push(
        <a key={`${key}-l${k++}`} href={url} target="_blank" rel="noopener noreferrer"
          style={{ color: C.accent, textDecoration: 'underline', textDecorationColor: `${C.accent}44`, wordBreak: 'break-all' }}
          onClick={(e) => e.stopPropagation()}>
          {url.length > 60 ? url.slice(0, 57) + '...' : url}
        </a>
      );
      lastIdx = match.index + match[0].length;
    }
    if (lastIdx < text.length) parts.push(<span key={`${key}-t${k++}`}>{text.slice(lastIdx)}</span>);
    return parts.length > 0 ? <>{parts}</> : <>{text}</>;
  };

  // c2-433 / #317 / #299: shared fact-popover opener. Any UI surface that
  // has a fact_key + a DOMRect (chat [fact:KEY] chips, Ledger rows, future
  // widgets) calls this to get the same ancestry-first popover. Fetch chain:
  //   /api/library/fact/:key/ancestry  →  /api/facts/:key  →  /api/provenance/:key
  // The popover render branch auto-picks which sections to display based
  // on payload shape.
  const openFactKey = (key: string, rect: DOMRect) => {
    setFactPopover({
      key,
      x: rect.left + rect.width / 2,
      y: rect.bottom + 6,
      data: null, error: null, loading: true,
    });
    setFactPopoverRaw(false);
    logEvent('fact_key_opened', { key });
    // Cache-bust query param so a post-verify re-open bypasses the 5-10s
    // backend /api/facts cache. Cheap — the backend ignores unknown params.
    const bust = `_t=${Date.now()}`;
    const tryFetch = async (path: string) => {
      const sep = path.includes('?') ? '&' : '?';
      const r = await fetch(`http://${getHost()}:3000${path}${sep}${bust}`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return r.json();
    };
    (async () => {
      try {
        let data: any;
        try { data = await tryFetch(`/api/library/fact/${encodeURIComponent(key)}/ancestry`); }
        catch {
          try { data = await tryFetch(`/api/facts/${encodeURIComponent(key)}`); }
          catch { data = await tryFetch(`/api/provenance/${encodeURIComponent(key)}`); }
        }
        setFactPopover(prev => prev && prev.key === key ? { ...prev, data, loading: false } : prev);
      } catch (e: any) {
        setFactPopover(prev => prev && prev.key === key ? { ...prev, error: String(e?.message || e || 'fetch failed'), loading: false } : prev);
      }
    })();
    // c2-433 / #274: cross-lingual concept map. When the key looks like
    // concept:NAME (or concept/NAME), also fetch the translations list.
    // Silent-fail — the popover renders without the Translations section
    // if the endpoint isnt live or the id isnt linked.
    const conceptMatch = key.match(/^concept[:/](.+)$/);
    if (conceptMatch) {
      const conceptId = conceptMatch[1];
      (async () => {
        try {
          const r = await fetch(`http://${getHost()}:3000/api/concepts/${encodeURIComponent(conceptId)}/translations`);
          if (!r.ok) return;
          const tdata = await r.json();
          const list: any[] = Array.isArray(tdata) ? tdata
            : Array.isArray(tdata?.translations) ? tdata.translations
            : Array.isArray(tdata?.items) ? tdata.items
            : [];
          // Merge into the existing popover data under _translations so the
          // popover render can pick it up without schema collisions.
          setFactPopover(prev => prev && prev.key === key
            ? { ...prev, data: { ...(prev.data || {}), _translations: list } }
            : prev);
        } catch { /* silent */ }
      })();
    }
  };

  // Markdown renderer lives in ./markdown.tsx; we build a ctx each render so the
  // current theme key + copy-handler flow through. Cheap — just a tiny object.
  // Wrap copyToClipboard so every copy fires a 'Copied' toast — without this
  // the user has no signal whether the click took.
  const copyWithToast = async (text: string) => {
    await copyToClipboard(text);
    showToast('Copied');
  };
  const mdCtx: MarkdownCtx = {
    C, themeKey: settings.theme,
    onCopy: copyWithToast,
    onCopyEvent: (lang, length) => logEvent('code_copied', { lang, length }),
    highlight: chatSearch || undefined,
    // c2-433 / #317 / #299: clicking a [fact:KEY] chip opens the shared
    // fact popover anchored at the chip's rect. Delegates to openFactKey
    // (defined above) so other surfaces (Ledger tab, future widgets) can
    // reuse the same popover with identical fetch fallbacks.
    onFactKey: openFactKey,
  };
  const renderMessageBody = (text: string) => renderMdBody(text, mdCtx);

  // Per-conversation export as Markdown
  const tierColor = (t: string) => {
    if (t.includes('BigBrain')) return C.purple;
    if (t.includes('Bridge')) return C.yellow;
    return C.green;
  };

  // ============================================================
  // RENDER: Login
  // ============================================================
  if (!isAuthenticated) {
    console.debug("// SCC: Rendering login, breakpoint:", bp);
    // c2-433: LoginScreen is lazy. Wrap in Suspense — the root Suspense is
    // below this early-return so it doesn't catch this branch.
    return (
      <React.Suspense fallback={null}>
        <LoginScreen
          C={C} isMobile={isMobile} isDesktop={isDesktop}
          password={password} setPassword={setPassword}
          authError={authError} authLoading={authLoading}
          onLogin={handleLogin}
        />
      </React.Suspense>
    );
  }

  // ============================================================
  // RENDER: Main Console
  // ============================================================
  console.debug("// SCC: Rendering console, msgs:", messages.length, "bp:", bp);

  // Matches the input bar below so messages + composer line up on the same
  // vertical axis — prior version had chat at 1140 and input at 880 which
  // made the input look off-center. Claude/ChatGPT both use ~760px.
  const chatMaxWidth = isDesktop ? '760px' : isTablet ? '680px' : '100%';
  const chatPadding = isDesktop ? '24px 32px' : isTablet ? '20px 24px' : '12px 14px';
  const sidebarWidth = 300;
  const userBubbleMaxWidth = isDesktop ? '70%' : '88%';

  // Telemetry stats data — show actual RAM usage (used / total) so "it says
  // 50 GB" doesn't confuse: the backend reports *available*; convert to used.
  const ramTotal = stats.ram_total_mb || 0;
  const ramUsed = stats.ram_used_mb ?? Math.max(0, ramTotal - stats.ram_available_mb);
  const ramUsedFmt = formatRam(ramUsed);
  const ramTotalFmt = formatRam(ramTotal);
  const ramFmt = formatRam(stats.ram_available_mb); // kept for legacy header
  const ramLabel = ramTotal > 0 ? `${ramUsedFmt.value}/${ramTotalFmt.value}` : ramUsedFmt.value;
  const ramUnit = ramTotal > 0 ? ramTotalFmt.unit : ramUsedFmt.unit;
  const telemetryCards = [
    { label: 'RAM', value: ramLabel, unit: ramUnit, color: C.accent, bg: C.accentBg, border: C.accentBorder },
    { label: 'CPU', value: `${stats.cpu_temp_c.toFixed(0)}`, unit: '\u00B0C', color: stats.cpu_temp_c > 65 ? C.red : C.green, bg: stats.cpu_temp_c > 65 ? C.redBg : C.greenBg, border: stats.cpu_temp_c > 65 ? C.redBorder : C.greenBorder },
    // Facts uses compactNum so "56.4M" reads cleanly; raw `${kg.facts}` was 8+ digits and ran off the card on narrow sidebars.
    { label: 'Facts', value: kg.facts ? compactNum(kg.facts) : '—', unit: '', color: C.purple, bg: C.purpleBg, border: C.purpleBorder },
    // Prefer Sources over Concepts — concepts_count always returns 0 (backend in-memory store is small); sources_count is a real metric that moves as the agent ingests.
    { label: 'Sources', value: kg.sources ? String(kg.sources) : '—', unit: '', color: C.green, bg: C.greenBg, border: C.greenBorder },
  ];

  const renderTelemetryCard = (s: typeof telemetryCards[0], compact = false) => (
    <TelemetryCard key={s.label} C={C} card={s} compact={compact} />
  );

  // Desktop sidebar
  const renderSidebar = () => (
    <aside aria-label='Substrate telemetry and admin' style={{
      width: `${sidebarWidth}px`, flexShrink: 0,
      background: C.bgCard, borderLeft: `1px solid ${C.border}`,
      display: 'flex', flexDirection: 'column', overflowY: 'auto',
    }}>
      {/* Telemetry */}
      <SubstrateTelemetry
        C={C}
        cards={telemetryCards}
        lastOkMs={kgLastOk}
        thermalThrottled={stats.is_throttled}
        diskFree={sysInfo.disk_free}
        diskTotal={sysInfo.disk_total}
      />
      {/* Status */}
      <SidebarStatus
        C={C}
        isConnected={isConnected}
        currentTier={currentTier}
        tierColor={tierColor}
        thermalThrottled={stats.is_throttled}
        logicDensity={stats.logic_density}
        quality={quality}
        kgSources={kg.sources}
        diskFree={sysInfo.disk_free}
        diskTotal={sysInfo.disk_total}
      />
      {/* Admin actions */}
      <AdminActions
        C={C}
        adminLoading={adminLoading}
        onFetchFacts={fetchFacts}
        onFetchQos={fetchQos}
        onClearChat={clearChat}
        onOpenSettings={() => setShowSettings(true)}
      >
        <FactsPanel C={C} facts={facts} fetchedAt={factsFetchedAt} error={factsError} />
        <QosPanel C={C} report={qosReport} fetchedAt={qosFetchedAt} error={qosError} />
        <DomainsPanel C={C} host={host} />
        <AccuracyPanel C={C} host={host} />
      </AdminActions>
    </aside>
  );

  return (
    <React.Suspense fallback={null}>
    {/* Skip-to-main-content link for keyboard users. Visually hidden until
        focused; then it becomes a visible button that jumps past the sidebar. */}
    <a href='#main-content' className='lfi-skip-link'
      style={{
        position: 'absolute', left: '8px', top: '-40px',
        background: C.accent, color: '#fff',
        padding: '8px 12px', borderRadius: '0 0 8px 8px',
        fontSize: T.typography.sizeMd, fontWeight: 700, textDecoration: 'none',
        zIndex: 9999,
      }}>
      Skip to chat
    </a>
    {/* Visually-hidden live region: screen readers speak new assistant responses. */}
    <div aria-live='polite' aria-atomic='true' style={{
      position: 'absolute', width: '1px', height: '1px', padding: 0, margin: '-1px',
      overflow: 'hidden', clip: 'rect(0 0 0 0)', border: 0,
    }}>{srAnnouncement}</div>
    <div className='lfi-app-root' style={{
      // c2-411 / BIG #218 mobile: 100dvh shrinks with the mobile virtual
      // keyboard so the chat input stays reachable. The class-level CSS
      // below supplies a 100vh fallback for browsers without dvh support.
      display: 'flex', flexDirection: 'column', height: '100dvh', width: '100%',
      background: C.bg, color: C.text,
      fontFamily: C.font,
      overflow: 'hidden',
      fontSize: `${fontScale}em`,
    }}>
      {/* ========== NEGATIVE FEEDBACK MODAL (bug #4 from c0-008) ========== */}
      {negFeedbackFor && (
        <div onClick={() => fb.closeNegFeedback()}
          style={{
            position: 'fixed', inset: 0, zIndex: T.z.modal + 60,
            background: 'rgba(0,0,0,0.55)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            padding: T.spacing.lg,
          }}>
          <div role='dialog' aria-modal='true' aria-labelledby='scc-negfb-title'
            onClick={(e) => e.stopPropagation()}
            style={{
              width: '100%', maxWidth: '460px',
              background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
              padding: T.spacing.xl, boxShadow: T.shadows.modal,
            }}>
            <h3 id='scc-negfb-title' style={{
              margin: '0 0 6px', fontSize: T.typography.sizeXl,
              fontWeight: T.typography.weightBold, color: C.text,
            }}>What was wrong?</h3>
            <p style={{ margin: '0 0 16px', fontSize: T.typography.sizeMd, color: C.textSecondary, lineHeight: T.typography.lineLoose }}>
              Help PlausiDen learn from this. Your feedback never leaves your machine unless you opt-in to telemetry.
            </p>
            <label style={{ fontSize: T.typography.sizeSm, fontWeight: T.typography.weightSemibold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
              Category
            </label>
            <select value={negFeedbackCategory}
              onChange={(e) => setNegFeedbackCategory(e.target.value)}
              aria-label='Feedback category'
              style={{
                width: '100%', marginTop: '6px', marginBottom: '14px',
                padding: '10px 12px', background: C.bgInput,
                border: `1px solid ${C.borderSubtle}`, color: C.text,
                borderRadius: T.radii.md, fontFamily: 'inherit', fontSize: T.typography.sizeBody,
              }}>
              <option>Incorrect</option>
              <option>Unhelpful</option>
              <option>Off-topic</option>
              <option>Too verbose</option>
              <option>Needs more detail</option>
              <option>Other</option>
            </select>
            <label style={{ fontSize: T.typography.sizeSm, fontWeight: T.typography.weightSemibold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
              Details (optional)
            </label>
            <textarea autoFocus
              value={negFeedbackText}
              onChange={(e) => setNegFeedbackText(e.target.value)}
              onKeyDown={(e) => {
                // Keyboard submit: Cmd/Ctrl+Enter or Shift+Enter commits the
                // negative feedback without needing to reach the Send button.
                if (e.key === 'Enter' && (e.metaKey || e.ctrlKey || e.shiftKey)) {
                  e.preventDefault();
                  const submitBtn = (e.currentTarget.closest('[role="dialog"]')?.querySelector('button[data-role="submit-feedback"]')) as HTMLButtonElement | null;
                  submitBtn?.click();
                }
              }}
              aria-label='Detailed feedback'
              autoComplete='off' spellCheck={true}
              placeholder='What should the AI have said? (Cmd+Enter to send)'
              maxLength={2000}
              style={{
                width: '100%', marginTop: '6px', minHeight: '88px',
                padding: '10px 12px', background: C.bgInput,
                border: `1px solid ${C.borderSubtle}`, color: C.text,
                borderRadius: T.radii.md, fontFamily: 'inherit', fontSize: T.typography.sizeBody,
                resize: 'vertical', boxSizing: 'border-box',
              }} />
            <div style={{ display: 'flex', gap: T.spacing.sm, justifyContent: 'flex-end', marginTop: T.spacing.lg }}>
              <button onClick={() => fb.closeNegFeedback()}
                style={{
                  padding: '10px 18px', background: 'transparent',
                  border: `1px solid ${C.border}`, color: C.textMuted,
                  borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeMd,
                }}>Cancel</button>
              <button data-role='submit-feedback'
                onClick={() => {
                // c2-433 / #350: spec body shape — rating: up|down|correct +
                // optional comment. Category collapses into the comment prefix
                // ("Incorrect: actual user text") so the backend has both the
                // structured tag and the freeform note in one field.
                const target = negFeedbackFor!;
                const txt = negFeedbackText.trim();
                const comment = txt ? `${negFeedbackCategory}: ${txt}` : negFeedbackCategory;
                const body = JSON.stringify({
                  conversation_id: currentConversationId,
                  message_id: target.msgId,
                  conclusion_id: target.conclusionId,
                  rating: 'down',
                  comment,
                });
                fetch(`http://${getHost()}:3000/api/feedback`, {
                  method: 'POST', headers: { 'Content-Type': 'application/json' }, body,
                }).then(async r => {
                  if (!r.ok) throw new Error(`HTTP ${r.status}`);
                  // #377: surface training-actions count when present.
                  try {
                    const d: any = await r.json();
                    const n = typeof d?.training_actions_applied === 'number' ? d.training_actions_applied : 0;
                    if (n > 0) showToast(`Feedback applied → ${n} action${n === 1 ? '' : 's'}`);
                  } catch { /* no body — silent */ }
                }).catch((e) => {
                    console.warn('feedback (negative) POST failed', e);
                    showToast('Feedback didn\u2019t reach the server');
                  });
                logEvent('feedback_negative', { msgId: target.msgId, category: negFeedbackCategory });
                fb.closeNegFeedback();
                showToast('Feedback sent');
              }}
                style={{
                  padding: '10px 18px', background: C.accent, border: 'none',
                  color: '#fff', borderRadius: T.radii.md, cursor: 'pointer',
                  fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                  fontWeight: T.typography.weightSemibold,
                }}>Send</button>
            </div>
          </div>
        </div>
      )}
      {/* ========== CORRECT-THIS MODAL (c2-433 / #350) ========== */}
      {/* Distinct from the negative-feedback modal above — that one captures
          a complaint ("this was wrong, here's the category"); this one
          captures a *correction* ("here's what you should have said"). The
          backend stores rating='correct' + correction text and the Classroom
          feedback queue surfaces these for ingestion. The user's prior
          query + the AI reply are echoed in the modal so the user knows
          which exchange they're correcting (chat may have scrolled). */}
      {/* ========== TEACH-LFI MODAL (#347) ========== */}
      {/* Direct teach entry — standalone from the refusal-flow Teach CTA
          and from correct-this (which targets a specific message). Users
          can proactively add a fact without first getting a refusal.
          POSTs to /api/feedback with rating='correct' + no conclusion_id. */}
      {showTeach && (
        <div onClick={() => { setShowTeach(false); setTeachText(''); }}
          role='presentation'
          style={{
            position: 'fixed', inset: 0, zIndex: T.z.modal + 60,
            background: 'rgba(0,0,0,0.55)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            padding: isMobile ? T.spacing.sm : T.spacing.lg,
          }}>
          <div ref={teachDialogRef} onClick={(e) => e.stopPropagation()}
            role='dialog' aria-modal='true' aria-labelledby='scc-teach-title'
            style={{
              width: '100%', maxWidth: '560px',
              background: C.bgCard, border: `1px solid ${C.border}`,
              borderRadius: T.radii.xl, padding: isMobile ? T.spacing.md : T.spacing.xl,
              boxShadow: T.shadows.modal,
              display: 'flex', flexDirection: 'column', gap: T.spacing.md,
              maxHeight: '90dvh', overflow: 'auto',
            }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <h2 id='scc-teach-title' style={{
                margin: 0, fontSize: T.typography.sizeXl,
                fontWeight: T.typography.weightBlack,
                letterSpacing: '0.06em', textTransform: 'uppercase', color: C.text,
              }}>Teach LFI</h2>
              <button onClick={() => { setShowTeach(false); setTeachText(''); }}
                aria-label='Close teach modal'
                style={{
                  background: 'transparent', border: 'none', color: C.textMuted,
                  fontSize: T.typography.size2xl, cursor: 'pointer', padding: '0 6px',
                }}>{'\u2715'}</button>
            </div>
            <p style={{
              margin: 0, fontSize: T.typography.sizeSm, color: C.textMuted, lineHeight: 1.55,
            }}>
              Write a fact or piece of knowledge you want LFI to remember. Plain English is fine —
              the substrate will extract tuples. Examples: <em>"The Eiffel Tower is 330m tall."</em>{' '}
              or <em>"My dog's name is Maya."</em>
            </p>
            <textarea
              value={teachText}
              onChange={(e) => setTeachText(e.target.value)}
              autoFocus
              placeholder='e.g. Water boils at 100°C at sea level.'
              rows={5}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
                  (e.currentTarget.closest('[role=dialog]')?.querySelector('[data-teach-submit]') as HTMLButtonElement | null)?.click();
                }
              }}
              style={{
                width: '100%', minHeight: '120px', resize: 'vertical',
                padding: T.spacing.md,
                background: C.bgInput, color: C.text,
                border: `1px solid ${C.border}`, borderRadius: T.radii.md,
                fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                lineHeight: 1.5, boxSizing: 'border-box',
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: T.spacing.sm, flexWrap: 'wrap' }}>
              <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
                {teachText.length} chars · Cmd/Ctrl+Enter to send
              </span>
              <div style={{ display: 'flex', gap: T.spacing.sm }}>
                <button onClick={() => { setShowTeach(false); setTeachText(''); }}
                  disabled={teachSending}
                  style={{
                    padding: '8px 16px', background: 'transparent', color: C.textMuted,
                    border: `1px solid ${C.border}`, borderRadius: T.radii.md,
                    cursor: teachSending ? 'wait' : 'pointer',
                    fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                  }}>Cancel</button>
                <button
                  data-teach-submit
                  disabled={teachSending || !teachText.trim()}
                  onClick={async () => {
                    const txt = teachText.trim();
                    if (!txt) return;
                    setTeachSending(true);
                    try {
                      const r = await fetch(`http://${getHost()}:3000/api/feedback`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                          conversation_id: currentConversationId,
                          rating: 'correct',
                          comment: `Teach: ${txt}`,
                          correction: txt,
                        }),
                      });
                      if (!r.ok) throw new Error(`HTTP ${r.status}`);
                      // claude-0 14:20 #377: training_actions_applied on
                      // /api/feedback response reports how many brain.db
                      // writes the teach triggered. Show the count if > 0.
                      try {
                        const d: any = await r.json();
                        const n = typeof d?.training_actions_applied === 'number' ? d.training_actions_applied : 0;
                        if (n > 0) {
                          showToast(`Taught — ${n} action${n === 1 ? '' : 's'} applied`);
                        } else {
                          showToast('Sent to LFI for ingestion');
                        }
                      } catch {
                        showToast('Sent to LFI for ingestion');
                      }
                      logEvent('teach_fact', { len: txt.length });
                      setShowTeach(false);
                      setTeachText('');
                    } catch (e: any) {
                      showToast(`Teach failed: ${String(e?.message || e || 'unknown')}`);
                    } finally {
                      setTeachSending(false);
                    }
                  }}
                  style={{
                    padding: '8px 18px', background: teachSending || !teachText.trim() ? C.bgInput : C.accent,
                    color: teachSending || !teachText.trim() ? C.textMuted : '#fff',
                    border: 'none', borderRadius: T.radii.md,
                    cursor: teachSending ? 'wait' : teachText.trim() ? 'pointer' : 'not-allowed',
                    fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                    fontWeight: T.typography.weightSemibold,
                  }}>
                  {teachSending ? 'Sending…' : 'Teach'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
      {correctFeedbackFor && (
        <div onClick={() => fb.closeCorrectFeedback()}
          style={{
            position: 'fixed', inset: 0, zIndex: T.z.modal + 60,
            background: 'rgba(0,0,0,0.55)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            padding: T.spacing.lg,
          }}>
          <div role='dialog' aria-modal='true' aria-labelledby='lfi-correct-title'
            onClick={(e) => e.stopPropagation()}
            style={{
              width: '100%', maxWidth: '520px',
              background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
              padding: T.spacing.xl, boxShadow: T.shadows.modal,
              maxHeight: '90dvh', overflowY: 'auto',
            }}>
            <h3 id='lfi-correct-title' style={{
              margin: '0 0 6px', fontSize: T.typography.sizeXl,
              fontWeight: T.typography.weightBold, color: C.text,
            }}>Teach the system</h3>
            <p style={{ margin: '0 0 14px', fontSize: T.typography.sizeMd, color: C.textSecondary, lineHeight: T.typography.lineLoose }}>
              Paste what the AI <em>should</em> have said. Your correction goes into the Classroom feedback queue for ingestion.
            </p>
            {correctFeedbackFor.userQuery && (
              <div style={{
                fontSize: '11px', color: C.textMuted, marginBottom: '6px',
                textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                fontWeight: T.typography.weightSemibold,
              }}>You asked</div>
            )}
            {correctFeedbackFor.userQuery && (
              <div style={{
                padding: '8px 10px', marginBottom: '12px',
                background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                borderRadius: T.radii.md, fontSize: '12px', color: C.textSecondary,
                maxHeight: '80px', overflowY: 'auto', whiteSpace: 'pre-wrap',
              }}>{correctFeedbackFor.userQuery.length > 280 ? correctFeedbackFor.userQuery.slice(0, 280) + '…' : correctFeedbackFor.userQuery}</div>
            )}
            <div style={{
              fontSize: '11px', color: C.textMuted, marginBottom: '6px',
              textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
              fontWeight: T.typography.weightSemibold,
            }}>It replied</div>
            <div style={{
              padding: '8px 10px', marginBottom: '14px',
              background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
              borderRadius: T.radii.md, fontSize: '12px', color: C.textSecondary,
              maxHeight: '120px', overflowY: 'auto', whiteSpace: 'pre-wrap',
            }}>{correctFeedbackFor.lfiReply.length > 400 ? correctFeedbackFor.lfiReply.slice(0, 400) + '…' : correctFeedbackFor.lfiReply}</div>
            <label style={{ fontSize: T.typography.sizeSm, fontWeight: T.typography.weightSemibold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
              The right answer
            </label>
            <textarea autoFocus
              value={correctFeedbackText}
              onChange={(e) => setCorrectFeedbackText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  const submitBtn = (e.currentTarget.closest('[role="dialog"]')?.querySelector('button[data-role="submit-correction"]')) as HTMLButtonElement | null;
                  submitBtn?.click();
                }
              }}
              aria-label='Correct response'
              autoComplete='off' spellCheck={true}
              placeholder='Type the response the AI should have given. (Cmd/Ctrl+Enter to send)'
              maxLength={4000}
              style={{
                width: '100%', marginTop: '6px', minHeight: '120px',
                padding: '10px 12px', background: C.bgInput,
                border: `1px solid ${C.borderSubtle}`, color: C.text,
                borderRadius: T.radii.md, fontFamily: 'inherit', fontSize: T.typography.sizeBody,
                resize: 'vertical', boxSizing: 'border-box',
              }} />
            <div style={{ display: 'flex', gap: T.spacing.sm, justifyContent: 'flex-end', marginTop: T.spacing.lg }}>
              <button onClick={() => fb.closeCorrectFeedback()}
                style={{
                  padding: '10px 18px', background: 'transparent',
                  border: `1px solid ${C.border}`, color: C.textMuted,
                  borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeMd,
                }}>Cancel</button>
              <button data-role='submit-correction'
                disabled={correctFeedbackText.trim().length === 0}
                onClick={() => {
                  const target = correctFeedbackFor!;
                  const correction = correctFeedbackText.trim();
                  if (!correction) return;
                  fetch(`http://${getHost()}:3000/api/feedback`, {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                      conversation_id: currentConversationId,
                      message_id: target.msgId,
                      conclusion_id: target.conclusionId,
                      user_query: target.userQuery,
                      lfi_reply: target.lfiReply,
                      rating: 'correct',
                      correction,
                    }),
                  }).then(async r => {
                    if (!r.ok) throw new Error(`HTTP ${r.status}`);
                    // claude-0 14:20 #377: server now applies TrainingActions
                    // against brain.db and returns training_actions_applied.
                    // Surface the count so the user sees real self-improvement.
                    try {
                      const d: any = await r.json();
                      const n = typeof d?.training_actions_applied === 'number' ? d.training_actions_applied : 0;
                      const kinds: string[] = Array.isArray(d?.training_action_kinds) ? d.training_action_kinds : [];
                      if (n > 0) {
                        showToast(`Correction applied → ${n} action${n === 1 ? '' : 's'}${kinds.length ? ' (' + kinds.slice(0, 3).join(', ') + ')' : ''}`);
                      }
                    } catch { /* body missing training_actions — no-op */ }
                  }).catch((e) => {
                      console.warn('feedback (correct) POST failed', e);
                      showToast('Correction didn\u2019t reach the server');
                    });
                  logEvent('feedback_correct', { msgId: target.msgId, len: correction.length });
                  fb.closeCorrectFeedback();
                  showToast('Correction sent — thanks for teaching');
                }}
                style={{
                  padding: '10px 18px',
                  background: correctFeedbackText.trim().length === 0 ? C.bgInput : C.accent,
                  border: 'none',
                  color: correctFeedbackText.trim().length === 0 ? C.textDim : '#fff',
                  borderRadius: T.radii.md,
                  cursor: correctFeedbackText.trim().length === 0 ? 'not-allowed' : 'pointer',
                  fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                  fontWeight: T.typography.weightSemibold,
                }}>Send correction</button>
            </div>
          </div>
        </div>
      )}
      {/* ========== FACT-KEY POPOVER (c2-433 / #317) ========== */}
      {/* Anchored at the click coordinates rather than centered — the popover
          is a glance affordance, not a modal. Fixed-positioned + clamped to
          the viewport edges so it doesn't paint offscreen on phones. Esc /
          outside-click closes via the keydown handler chain + the backdrop. */}
      {factPopover && (() => {
        // c2-433 / mobile: cap popover width to viewport minus gutters so
        // the 320px default doesn't overflow 320px mobile viewports. On
        // 375px iPhone we still get 320. On 320px Android the popover
        // shrinks to fit.
        const margin = 8;
        const vw = typeof window !== 'undefined' ? window.innerWidth : 1024;
        const vh = typeof window !== 'undefined' ? window.innerHeight : 768;
        const W = Math.min(320, vw - margin * 2);
        const left = Math.min(Math.max(factPopover.x - W / 2, margin), vw - W - margin);
        const maxH = Math.max(160, vh - factPopover.y - margin - 20);
        return (
          <>
            <div onClick={() => setFactPopover(null)}
              style={{
                position: 'fixed', inset: 0, zIndex: T.z.modal + 40,
                background: 'transparent', cursor: 'default',
              }} />
            <div role='dialog' aria-modal='false' aria-label={`Fact ${factPopover.key}`}
              style={{
                position: 'fixed', left, top: factPopover.y,
                width: `${W}px`, maxHeight: `${maxH}px`,
                background: C.bgCard, border: `1px solid ${C.border}`,
                borderRadius: T.radii.lg, boxShadow: T.shadows.modal,
                zIndex: T.z.modal + 41, overflow: 'hidden',
                display: 'flex', flexDirection: 'column',
                animation: 'lfi-fadein 0.12s ease-out',
              }}>
              <div style={{
                display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                gap: T.spacing.sm, padding: '8px 12px',
                borderBottom: `1px solid ${C.borderSubtle}`, background: C.bgInput,
              }}>
                {/* c2-433 / task 242: clicking the key copies it to clipboard
                    so users can reference the fact-id in docs / tickets /
                    Slack without manual select. Tiny visual feedback (key
                    text flips to "copied" green for 1s). */}
                <button onClick={(e) => {
                  try { navigator.clipboard.writeText(factPopover.key); } catch { /* clipboard blocked */ }
                  const btn = e.currentTarget;
                  const orig = btn.textContent;
                  btn.textContent = 'copied';
                  btn.style.color = C.green;
                  window.setTimeout(() => { btn.textContent = orig; btn.style.color = C.accent; }, 1000);
                }}
                  title='Copy fact key to clipboard'
                  aria-label={`Copy fact key ${factPopover.key}`}
                  style={{
                    background: 'transparent', border: 'none',
                    fontSize: '11px', fontWeight: T.typography.weightBold, color: C.accent,
                    fontFamily: T.typography.fontMono, letterSpacing: '0.04em',
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    flex: 1, minWidth: 0, textAlign: 'left',
                    cursor: 'pointer', padding: 0,
                  }}>{factPopover.key}</button>
                {/* c2-433 / task 240: structured ↔ raw toggle. Hidden until
                    data lands so the toggle doesn't tease an empty switch
                    during loading. */}
                {factPopover.data && !factPopover.loading && (
                  <button onClick={() => setFactPopoverRaw(v => !v)}
                    title={factPopoverRaw ? 'Show structured view' : 'Show raw JSON'}
                    aria-label={factPopoverRaw ? 'Show structured view' : 'Show raw JSON'}
                    aria-pressed={factPopoverRaw}
                    style={{
                      background: factPopoverRaw ? C.accentBg : 'transparent',
                      border: `1px solid ${factPopoverRaw ? C.accentBorder : C.borderSubtle}`,
                      color: factPopoverRaw ? C.accent : C.textMuted,
                      cursor: 'pointer', padding: '2px 6px', fontSize: '9px',
                      fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold,
                      lineHeight: 1, borderRadius: T.radii.sm,
                      textTransform: 'uppercase', letterSpacing: '0.04em',
                      flexShrink: 0,
                    }}>JSON</button>
                )}
                <button onClick={() => setFactPopover(null)}
                  aria-label='Close fact popover'
                  style={{
                    background: 'transparent', border: 'none', color: C.textMuted,
                    cursor: 'pointer', padding: '0 4px', fontSize: '16px',
                    fontFamily: 'inherit', lineHeight: 1,
                    flexShrink: 0,
                  }}>×</button>
              </div>
              <div style={{ padding: '10px 12px', overflowY: 'auto', fontSize: T.typography.sizeSm }}>
                {factPopover.loading && (
                  <div style={{ color: C.textMuted, fontStyle: 'italic' }}>Loading…</div>
                )}
                {factPopover.error && !factPopover.loading && (
                  <div style={{ color: C.red, fontSize: T.typography.sizeXs, lineHeight: T.typography.lineLoose }}>
                    Couldn't fetch this fact: {factPopover.error}.<br/>
                    Backend may not yet expose <code style={{ fontFamily: T.typography.fontMono }}>/api/library/fact/:key/ancestry</code>, <code style={{ fontFamily: T.typography.fontMono }}>/api/facts/:key</code>, or <code style={{ fontFamily: T.typography.fontMono }}>/api/provenance/:key</code>.
                  </div>
                )}
                {factPopover.data && !factPopover.loading && (() => {
                  const d = factPopover.data;
                  const rows: Array<{ label: string; value: React.ReactNode; mono?: boolean }> = [];
                  if (d.subj || d.subject) rows.push({ label: 'Subject', value: d.subj || d.subject, mono: true });
                  if (d.pred || d.predicate) rows.push({ label: 'Predicate', value: d.pred || d.predicate, mono: true });
                  if (d.obj || d.object) rows.push({ label: 'Object', value: d.obj || d.object, mono: true });
                  if (d.source) rows.push({ label: 'Source', value: d.source });
                  if (d.psl_status || d.psl) rows.push({ label: 'PSL', value: d.psl_status || d.psl });
                  if (typeof d.trust === 'number') rows.push({ label: 'Trust', value: d.trust.toFixed(2), mono: true });
                  // c2-433 / #182 + #354: Lean4/Kimina verdict. Forward-compat —
                  // renders whenever the ancestry payload carries
                  // proof_status / verdict / lean_verdict / verifier_verdict.
                  // Colored badge: Proved=green, Rejected=red, Unreachable=yellow
                  // (verifier down; prior tier preserved per #354 NO-OP semantic),
                  // Error=red. Silent when absent. Appends checked_at relative
                  // timestamp (from #354 facts.checked_at column) below the badge
                  // when present.
                  const rawVerdict = d.proof_status ?? d.verdict ?? d.lean_verdict ?? d.verifier_verdict;
                  if (rawVerdict) {
                    const v = String(rawVerdict).toLowerCase();
                    const tone = v === 'proved' || v === 'valid' || v === 'ok' ? C.green
                      : v === 'rejected' || v === 'invalid' ? C.red
                      : v === 'unreachable' || v === 'timeout' || v === 'pending' ? C.yellow
                      : v === 'error' ? C.red
                      : C.textMuted;
                    const checkedAt = d.checked_at ?? d.verified_at ?? d.proof_checked_at;
                    const checkedMs = checkedAt ? (typeof checkedAt === 'number' ? checkedAt : Date.parse(String(checkedAt))) : null;
                    const proofHash: string | null = typeof d.proof_hash === 'string' ? d.proof_hash : null;
                    rows.push({
                      label: 'Verdict', mono: true,
                      value: (
                        <span style={{ display: 'inline-flex', flexDirection: 'column', gap: '3px' }}>
                          <span style={{
                            display: 'inline-flex', alignItems: 'center', gap: '4px',
                            padding: '1px 6px', fontSize: '10px',
                            background: `${tone}18`, border: `1px solid ${tone}55`,
                            color: tone, fontWeight: 800,
                            borderRadius: T.radii.sm,
                            letterSpacing: '0.04em', textTransform: 'uppercase',
                            alignSelf: 'flex-start',
                          }} title={`Lean4/Kimina verifier verdict: ${String(rawVerdict)}${proofHash ? ` · hash ${proofHash.slice(0, 12)}…` : ''}`}>
                            {String(rawVerdict)}
                          </span>
                          {checkedMs && !Number.isNaN(checkedMs) && (
                            <span style={{
                              fontSize: '9px', color: C.textDim,
                              fontFamily: T.typography.fontMono,
                            }} title={`Last verifier check: ${checkedAt}`}>
                              checked {formatRelative(checkedMs)}
                              {/* c2-433 / #354 followup: click-copy proof_hash.
                                  Mirrors the run_id / token_id / domain
                                  pattern. 1.5s green flash on success. */}
                              {proofHash && (
                                <button type='button'
                                  onClick={async (e) => {
                                    e.stopPropagation();
                                    try {
                                      await navigator.clipboard.writeText(proofHash);
                                      setCopiedProofHash(proofHash);
                                      window.setTimeout(() => {
                                        setCopiedProofHash(prev => prev === proofHash ? null : prev);
                                      }, 1500);
                                    } catch { /* clipboard blocked */ }
                                  }}
                                  title={copiedProofHash === proofHash ? `Copied ${proofHash}` : `${proofHash} — click to copy proof_hash`}
                                  aria-label={copiedProofHash === proofHash ? `Copied proof hash` : `Copy proof hash ${proofHash}`}
                                  style={{
                                    marginLeft: '6px',
                                    background: 'transparent', border: 'none', padding: 0,
                                    color: copiedProofHash === proofHash ? C.green : C.textDim,
                                    fontFamily: T.typography.fontMono, fontSize: '9px',
                                    cursor: 'pointer',
                                    textDecoration: 'underline',
                                    textDecorationColor: `${copiedProofHash === proofHash ? C.green : C.textDim}33`,
                                    textUnderlineOffset: '2px',
                                    transition: 'color 180ms',
                                  }}>{copiedProofHash === proofHash ? 'copied ✓' : `· h:${proofHash.slice(0, 8)}…`}</button>
                              )}
                            </span>
                          )}
                        </span>
                      ),
                    });
                  }
                  if (d.temporal_class || d.tier) rows.push({ label: 'Tier', value: d.temporal_class || d.tier });
                  if (d.explanation) rows.push({ label: 'Note', value: d.explanation });
                  // c2-433 / #299: ancestry payload arrays (versions /
                  // contradictions / inbound_edges / outbound_edges) — render
                  // each as a count-headed section with the first few items
                  // inline. 320px width means we show preview text, not full
                  // structured rows. Users who need the raw data hit JSON.
                  const versions: any[] = Array.isArray(d.versions) ? d.versions : [];
                  const contradictions: any[] = Array.isArray(d.contradictions) ? d.contradictions : [];
                  const inbound: any[] = Array.isArray(d.inbound_edges) ? d.inbound_edges : [];
                  const outbound: any[] = Array.isArray(d.outbound_edges) ? d.outbound_edges : [];
                  // c2-433 / #274: translations from /api/concepts/:id/translations
                  // (injected under _translations by openFactKey for concept: keys).
                  // Each entry: {language, text} tolerant to {lang, text} fallback.
                  const translations: any[] = Array.isArray(d._translations) ? d._translations : [];
                  const hasAncestry = versions.length + contradictions.length + inbound.length + outbound.length + translations.length > 0;
                  // c2-433 / task 240: raw view forced by toggle, OR the
                  // structured render found nothing recognisable in the
                  // payload. Either way the user gets the underlying JSON.
                  if (factPopoverRaw || (rows.length === 0 && !hasAncestry)) {
                    return (
                      <pre style={{
                        margin: 0, fontFamily: T.typography.fontMono,
                        fontSize: '10px', color: C.textSecondary,
                        whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                      }}>{JSON.stringify(d, null, 2).slice(0, 4000)}</pre>
                    );
                  }
                  const section = (label: string, items: any[], render: (it: any, i: number) => React.ReactNode, tone?: string) => (
                    items.length > 0 ? (
                      <div key={label} style={{ marginTop: '10px', paddingTop: '8px', borderTop: `1px dashed ${C.borderSubtle}` }}>
                        <div style={{
                          fontSize: '10px', color: tone || C.textMuted, fontWeight: 700,
                          textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: '4px',
                        }}>{label} <span style={{ color: C.textMuted, fontWeight: 500 }}>({items.length})</span></div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
                          {items.slice(0, 5).map((it, i) => (
                            <div key={i} style={{
                              fontSize: '11px', color: C.textSecondary,
                              fontFamily: T.typography.fontMono, wordBreak: 'break-word',
                              lineHeight: 1.35,
                            }}>{render(it, i)}</div>
                          ))}
                          {items.length > 5 && (
                            <div style={{ fontSize: '10px', color: C.textMuted, fontStyle: 'italic' }}>
                              + {items.length - 5} more (toggle JSON to see all)
                            </div>
                          )}
                        </div>
                      </div>
                    ) : null
                  );
                  const edgeText = (e: any): string => {
                    if (typeof e === 'string') return e;
                    if (!e || typeof e !== 'object') return String(e);
                    const pred = e.pred || e.predicate || e.relation || e.rel || '';
                    const tgt = e.target || e.tgt || e.to || e.obj || e.object || e.subj || e.subject || e.key || '';
                    const w = typeof e.weight === 'number' ? ` (${e.weight.toFixed(2)})` : '';
                    return pred && tgt ? `${pred} → ${tgt}${w}` : JSON.stringify(e).slice(0, 80);
                  };
                  const versionText = (v: any): string => {
                    if (typeof v === 'string') return v;
                    if (!v || typeof v !== 'object') return String(v);
                    const when = v.at || v.timestamp || v.created_at || '';
                    const val = v.value || v.obj || v.object || v.explanation || '';
                    const trust = typeof v.trust === 'number' ? ` [t=${v.trust.toFixed(2)}]` : '';
                    return when ? `${String(when).slice(0, 19)}${trust} — ${val}` : (val || JSON.stringify(v).slice(0, 80));
                  };
                  const contradictionText = (c: any): string => {
                    if (typeof c === 'string') return c;
                    if (!c || typeof c !== 'object') return String(c);
                    const a = c.side_a || c.a || c.this || '';
                    const b = c.side_b || c.b || c.other || '';
                    return a && b ? `${a}  ↔  ${b}` : JSON.stringify(c).slice(0, 100);
                  };
                  return (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {rows.map(r => (
                        <div key={r.label} style={{ display: 'flex', gap: '8px', alignItems: 'baseline' }}>
                          <span style={{
                            fontSize: '10px', color: C.textMuted, fontWeight: 700,
                            textTransform: 'uppercase', letterSpacing: '0.06em',
                            minWidth: '64px', flexShrink: 0,
                          }}>{r.label}</span>
                          <span style={{
                            color: C.text, fontSize: T.typography.sizeSm,
                            fontFamily: r.mono ? T.typography.fontMono : 'inherit',
                            wordBreak: 'break-word',
                          }}>{r.value}</span>
                        </div>
                      ))}
                      {section('Versions', versions, versionText)}
                      {section('Contradictions', contradictions, contradictionText, C.red)}
                      {section('Inbound', inbound, edgeText)}
                      {section('Outbound', outbound, edgeText)}
                      {/* c2-433 / #274: Translations row per-entry —
                          language tag in accent + the phrase. */}
                      {section('Translations', translations, (t: any) => {
                        const lang = String(t.language ?? t.lang ?? '—').toLowerCase();
                        const text = String(t.text ?? t.value ?? '');
                        return (
                          <span>
                            <span style={{ color: C.accent, fontWeight: 700, marginRight: '6px' }}>{lang}</span>
                            {text}
                          </span>
                        );
                      }, C.purple)}
                    </div>
                  );
                })()}
              </div>
              {/* c2-433 / #274 followup: concept-link inline form. Shows
                  only when the popover's key is a concept: key. Collapsed
                  state is a tiny "+ add translation" pill; expanded shows
                  a 2-col row (lang select + text input) + Save button.
                  Posts /api/concepts/link then re-runs openFactKey so the
                  Translations section refreshes with the new entry. */}
              {(() => {
                const isConcept = /^concept[:/]/.test(factPopover.key);
                if (!isConcept || !factPopover.data || factPopover.loading) return null;
                const conceptId = factPopover.key.replace(/^concept[:/]/, '');
                return (
                  <div style={{
                    padding: '6px 12px', borderTop: `1px solid ${C.borderSubtle}`,
                    background: C.bgCard,
                  }}>
                    {!factLinkOpen ? (
                      <button onClick={() => { setFactLinkOpen(true); setFactLinkErr(null); }}
                        style={{
                          background: 'transparent', border: 'none',
                          color: C.accent, cursor: 'pointer',
                          fontSize: '10px', fontWeight: 700,
                          fontFamily: T.typography.fontMono,
                          textTransform: 'uppercase', letterSpacing: '0.04em',
                          padding: 0,
                        }}>+ add translation</button>
                    ) : (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                          <select value={factLinkLang}
                            onChange={(e) => setFactLinkLang(e.target.value)}
                            aria-label='Translation language'
                            style={{
                              padding: '3px 6px', fontSize: '10px',
                              background: C.bgInput, color: C.text,
                              border: `1px solid ${C.borderSubtle}`,
                              borderRadius: T.radii.sm, fontFamily: T.typography.fontMono,
                            }}>
                            {['en','es','fr','de','it','pt','ja','zh','ko','ru','ar','hi'].map(l => (
                              <option key={l} value={l}>{l}</option>
                            ))}
                          </select>
                          <input type='text' value={factLinkText}
                            onChange={(e) => setFactLinkText(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === 'Escape') { e.stopPropagation(); setFactLinkOpen(false); setFactLinkText(''); setFactLinkErr(null); }
                            }}
                            placeholder='translation text'
                            aria-label='Translation text'
                            style={{
                              flex: '1 1 120px', minWidth: 0,
                              padding: '3px 6px', fontSize: '10px',
                              background: C.bgInput, color: C.text,
                              border: `1px solid ${C.borderSubtle}`,
                              borderRadius: T.radii.sm, fontFamily: 'inherit',
                              outline: 'none',
                            }} />
                        </div>
                        {factLinkErr && (
                          <div style={{ fontSize: '10px', color: C.red, fontFamily: T.typography.fontMono }}>
                            {factLinkErr}
                          </div>
                        )}
                        <div style={{ display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                          <button onClick={() => { setFactLinkOpen(false); setFactLinkText(''); setFactLinkErr(null); }}
                            style={{
                              padding: '3px 9px', fontSize: '10px',
                              fontWeight: T.typography.weightBold,
                              background: 'transparent', color: C.textMuted,
                              border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.sm,
                              cursor: 'pointer', fontFamily: 'inherit',
                              letterSpacing: '0.04em', textTransform: 'uppercase',
                            }}>Cancel</button>
                          <button disabled={factLinkSaving || !factLinkText.trim()}
                            onClick={async () => {
                              setFactLinkSaving(true);
                              setFactLinkErr(null);
                              try {
                                const r = await fetch(`http://${getHost()}:3000/api/concepts/link`, {
                                  method: 'POST',
                                  headers: { 'Content-Type': 'application/json' },
                                  body: JSON.stringify({ concept_id: conceptId, language: factLinkLang, text: factLinkText.trim() }),
                                });
                                if (!r.ok) throw new Error(`HTTP ${r.status}`);
                                showToast(`Linked ${factLinkLang}: ${factLinkText.trim().slice(0, 40)}`);
                                logEvent('concept_linked', { concept_id: conceptId, language: factLinkLang });
                                setFactLinkText('');
                                setFactLinkOpen(false);
                                // Re-fetch translations by re-opening the popover
                                // at the same anchor. Use a small dummy rect at
                                // the current position since we don't have the
                                // original.
                                const pr = factPopover;
                                if (pr) {
                                  openFactKey(pr.key, new DOMRect(pr.x - 10, pr.y - 26, 20, 20));
                                }
                              } catch (e: any) {
                                setFactLinkErr(String(e?.message || e || 'link failed'));
                              } finally {
                                setFactLinkSaving(false);
                              }
                            }}
                            style={{
                              padding: '3px 9px', fontSize: '10px',
                              fontWeight: T.typography.weightBold,
                              background: C.accentBg, color: C.accent,
                              border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm,
                              cursor: (factLinkSaving || !factLinkText.trim()) ? 'not-allowed' : 'pointer',
                              fontFamily: 'inherit',
                              letterSpacing: '0.04em', textTransform: 'uppercase',
                              opacity: (factLinkSaving || !factLinkText.trim()) ? 0.5 : 1,
                            }}>{factLinkSaving ? 'Saving…' : 'Save'}</button>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })()}
              {/* c2-433 / #354: Verify-now button. Visible when the fact
                  popover's data is loaded AND proof_status is missing or
                  still pending/unknown. Click POSTs /api/proof/verify
                  {fact_key} and re-opens the popover to pull the fresh
                  verdict. Hidden once a proved/rejected verdict is on
                  file so users don't spam the verifier on proved facts. */}
              {factPopover.data && !factPopover.loading && !factPopover.error && (() => {
                const ps = String(factPopover.data.proof_status ?? factPopover.data.verdict ?? factPopover.data.lean_verdict ?? '').toLowerCase();
                const needsVerify = !ps || ps === 'unknown' || ps === 'pending' || ps === 'unreachable' || ps === 'error';
                if (!needsVerify) return null;
                return (
                  <div style={{
                    padding: '6px 12px',
                    borderTop: `1px solid ${C.borderSubtle}`,
                    background: C.bgCard, display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                  }}>
                    <span style={{ flex: 1, fontSize: '10px', color: C.textMuted, fontFamily: T.typography.fontMono }}>
                      {ps === 'unreachable' ? 'verifier was down' : ps === 'pending' ? 'verifier queued' : 'unverified'}
                    </span>
                    <button disabled={factVerifying}
                      onClick={async () => {
                        const key = factPopover.key;
                        setFactVerifying(true);
                        try {
                          const r = await fetch(`http://${getHost()}:3000/api/proof/verify`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ fact_key: key }),
                          });
                          if (!r.ok) {
                            if (r.status === 404) {
                              showToast('Verify endpoint not live');
                            } else {
                              throw new Error(`HTTP ${r.status}`);
                            }
                          } else {
                            const data = await r.json().catch(() => ({}));
                            const v: string = data.verdict ?? data.proof_status ?? data.status ?? 'done';
                            showToast(`Verify: ${v}`);
                            logEvent('proof_verify_triggered', { fact_key: key, verdict: v });
                            // Optimistic local patch: update popover data immediately
                            // with the verdict the server just returned. Backend
                            // /api/facts cache can take 5-10s to invalidate, so
                            // an immediate re-fetch returns the OLD proof_status.
                            // We patch now + schedule a cache-busted refetch
                            // after 3s so the popover stays accurate through the
                            // cache window.
                            setFactPopover(prev => prev && prev.key === key && prev.data
                              ? { ...prev, data: { ...prev.data, proof_status: v, verdict: v, checked_at: new Date().toISOString() } }
                              : prev);
                            const pr = factPopover;
                            if (pr) window.setTimeout(() => {
                              openFactKey(pr.key, new DOMRect(pr.x - 10, pr.y - 26, 20, 20));
                            }, 3000);
                          }
                        } catch (e: any) {
                          showToast(`Verify failed: ${String(e?.message || e || 'unknown')}`);
                        } finally {
                          setFactVerifying(false);
                        }
                      }}
                      style={{
                        padding: '3px 10px', fontSize: '10px',
                        fontWeight: T.typography.weightBold,
                        background: C.accentBg, color: C.accent,
                        border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm,
                        cursor: factVerifying ? 'wait' : 'pointer',
                        fontFamily: 'inherit', letterSpacing: '0.04em',
                        textTransform: 'uppercase',
                        opacity: factVerifying ? 0.6 : 1,
                      }}>{factVerifying ? 'Verifying…' : 'Verify now'}</button>
                    {/* claude-0 13:12 ask: Dismiss fact — flags the fact as
                        wrong via /api/feedback so the ingestion pipeline
                        drops/downweights it. No modal — one click + toast. */}
                    <button disabled={factVerifying}
                      onClick={async () => {
                        const key = factPopover.key;
                        try {
                          const r = await fetch(`http://${getHost()}:3000/api/feedback`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({
                              conversation_id: currentConversationId,
                              rating: 'down',
                              correction: `fact wrong: ${key}`,
                              comment: `Dismiss fact ${key}`,
                            }),
                          });
                          if (!r.ok) throw new Error(`HTTP ${r.status}`);
                          // #377: show action count when applicable.
                          let suffix = '';
                          try {
                            const d: any = await r.json();
                            const n = typeof d?.training_actions_applied === 'number' ? d.training_actions_applied : 0;
                            if (n > 0) suffix = ` → ${n} action${n === 1 ? '' : 's'}`;
                          } catch { /* silent */ }
                          showToast(`Dismissed ${key}${suffix}`);
                          logEvent('fact_dismissed', { fact_key: key });
                          setFactPopover(null);
                        } catch (e: any) {
                          showToast(`Dismiss failed: ${String(e?.message || e || 'unknown')}`);
                        }
                      }}
                      title={`Mark "${factPopover.key}" as wrong — sends a down-vote + correction to /api/feedback`}
                      style={{
                        padding: '3px 10px', fontSize: '10px',
                        fontWeight: T.typography.weightBold,
                        background: C.redBg, color: C.red,
                        border: `1px solid ${C.redBorder || `${C.red}55`}`, borderRadius: T.radii.sm,
                        cursor: factVerifying ? 'wait' : 'pointer',
                        fontFamily: 'inherit', letterSpacing: '0.04em',
                        textTransform: 'uppercase',
                        opacity: factVerifying ? 0.6 : 1,
                      }}>Dismiss</button>
                  </div>
                );
              })()}
              {/* c2-433 / #337 followup: FSRS review footer. When the fact
                  popover is open AND we have a key + no active error, show
                  the 4-rating row (Again / Hard / Good / Easy → 1-4). Click
                  POSTs /api/fsrs/review + toasts + closes the popover. If
                  the fact isn't in fsrs_cards yet, the backend will 404 and
                  we surface a friendly "Not in FSRS" toast. Gives users a
                  one-click grade path for any [fact:KEY] they see in chat
                  — no KB modal round-trip. Hidden during load/error. */}
              {factPopover.data && !factPopover.loading && !factPopover.error && (
                <div style={{
                  display: 'flex', gap: '4px', padding: '8px 10px',
                  borderTop: `1px solid ${C.borderSubtle}`, background: C.bgInput,
                }}>
                  {([
                    { r: 1 as const, label: 'Again', hint: 'Forgot — schedule soon',   bg: C.redBg,                border: C.redBorder,    fg: C.red },
                    { r: 2 as const, label: 'Hard',  hint: 'Recalled with effort',    bg: C.yellowBg || C.bgInput, border: C.yellow,       fg: C.yellow },
                    { r: 3 as const, label: 'Good',  hint: 'Recalled normally',       bg: C.accentBg,              border: C.accentBorder, fg: C.accent },
                    { r: 4 as const, label: 'Easy',  hint: 'Trivial — longer interval', bg: C.greenBg || C.bgInput, border: C.green,       fg: C.green },
                  ]).map(b => (
                    <button key={b.r} disabled={factReviewing}
                      onClick={async () => {
                        const key = factPopover.key;
                        setFactReviewing(true);
                        try {
                          const r = await fetch(`http://${getHost()}:3000/api/fsrs/review`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ fact_key: key, rating: b.r }),
                          });
                          if (!r.ok) {
                            if (r.status === 404) {
                              showToast(`This fact isn't in FSRS yet — grade unchanged`);
                            } else {
                              throw new Error(`HTTP ${r.status}`);
                            }
                          } else {
                            showToast(`Reviewed: ${b.label}`);
                            logEvent('fsrs_reviewed', { fact_key: key, rating: b.r, via: 'popover' });
                          }
                          knowledgeLastFetchedRef.current = 0;
                          setFactPopover(null);
                        } catch (e: any) {
                          showToast(`Review failed: ${String(e?.message || e || 'unknown error')}`);
                        } finally {
                          setFactReviewing(false);
                        }
                      }}
                      title={`${b.hint} (${b.r})`}
                      aria-label={`Rate this fact: ${b.label}`}
                      style={{
                        flex: 1, padding: '5px 6px', fontSize: '10px',
                        fontWeight: T.typography.weightBold,
                        background: b.bg, color: b.fg,
                        border: `1px solid ${b.border}`,
                        borderRadius: T.radii.sm,
                        cursor: factReviewing ? 'wait' : 'pointer',
                        opacity: factReviewing ? 0.5 : 1,
                        fontFamily: 'inherit', letterSpacing: '0.02em',
                        textTransform: 'uppercase',
                      }}>{b.label}</button>
                  ))}
                </div>
              )}
            </div>
          </>
        );
      })()}
      {/* ========== MESSAGE RIGHT-CLICK CONTEXT MENU ========== */}
      {/* c2-400 / task 185: floating menu anchored at the right-click coords.
          Invisible full-screen backdrop catches outside clicks; Esc closes
          via the same path as other modals. menu items vary by role. */}
      {msgContextMenu && (() => {
        const m = msgContextMenu;
        const close = () => setMsgContextMenu(null);
        const copy = (text: string) => {
          copyToClipboard(text);
          showToast('Copied');
          logEvent('message_copied', { role: m.role, length: text.length, via: 'context-menu' });
          close();
        };
        const isLastAssistant = m.role === 'assistant' && messages[messages.length - 1]?.id === m.msgId;
        // Clamp the menu so it doesn't spill off the right / bottom edge.
        const MENU_W = 220, MENU_H = 180;
        const x = Math.min(m.x, window.innerWidth - MENU_W - 8);
        const y = Math.min(m.y, window.innerHeight - MENU_H - 8);
        const btnStyle: React.CSSProperties = {
          display: 'block', width: '100%', textAlign: 'left',
          padding: `${T.spacing.sm} ${T.spacing.md}`,
          background: 'transparent', border: 'none', color: C.text,
          cursor: 'pointer', fontFamily: 'inherit',
          fontSize: T.typography.sizeMd,
        };
        const onBtnHover = (e: React.MouseEvent<HTMLButtonElement>) => { e.currentTarget.style.background = C.bgHover; };
        const onBtnLeave = (e: React.MouseEvent<HTMLButtonElement>) => { e.currentTarget.style.background = 'transparent'; };
        return (
          <>
            <div onClick={close} onContextMenu={(e) => { e.preventDefault(); close(); }}
              style={{
                position: 'fixed', inset: 0, zIndex: T.z.modal + 50,
                background: 'transparent',
              }} />
            <div role='menu' aria-label={`${m.role} message actions`}
              style={{
                position: 'fixed', left: x, top: y, zIndex: T.z.modal + 51,
                width: MENU_W, background: C.bgCard,
                border: `1px solid ${C.border}`, borderRadius: T.radii.md,
                boxShadow: T.shadows.modal, padding: T.spacing.xs + ' 0',
                fontFamily: 'inherit',
              }}>
              <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                onClick={() => copy(m.content)}>Copy</button>
              <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                onClick={() => copy(stripMarkdown(m.content))}>Copy as plain text</button>
              <div role='separator' style={{ height: '1px', background: C.borderSubtle, margin: `${T.spacing.xs} 0` }} />
              {m.role === 'user' && (
                <>
                  <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                    onClick={() => { me.begin(m.msgId, m.content); close(); }}>Edit and resend</button>
                  <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                    onClick={() => {
                      // Fork: slice from this user message forward, preload the
                      // input + stamp the branch marker so handleSend labels
                      // the new turn as a branch. Matches the edit path.
                      const idx = messages.findIndex(mm => mm.id === m.msgId);
                      if (idx >= 0) setMessages(prev => prev.slice(0, idx));
                      setInputAndResize(m.content);
                      pendingBranchFromRef.current = m.msgId;
                      inputRef.current?.focus();
                      logEvent('message_forked', { via: 'context-menu' });
                      close();
                    }}>Fork from here</button>
                </>
              )}
              {m.role === 'assistant' && isLastAssistant && !isThinking && (
                <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                  onClick={() => { regenerateLast(); showToast('Regenerating…'); close(); }}>Regenerate</button>
              )}
              {/* #176: branch to a new conversation, preserving up to + including
                  this message. Unlike "Fork from here" (which truncates the
                  current convo in place), this creates a SEPARATE convo so
                  both paths survive for comparison. */}
              <button role='menuitem' style={btnStyle} onMouseEnter={onBtnHover} onMouseLeave={onBtnLeave}
                onClick={() => {
                  if (currentConversationId) branchFromMessage(currentConversationId, m.msgId);
                  close();
                }}>Branch to new conversation</button>
            </div>
          </>
        );
      })()}
      {/* ========== EPHEMERAL TOAST STACK (copy feedback, etc.) ========== */}
      {toasts.length > 0 && (
        <div style={{
          position: 'fixed', top: '20px', right: '20px', zIndex: T.z.toast + 10,
          display: 'flex', flexDirection: 'column', gap: T.spacing.sm,
          pointerEvents: 'none', // let individual toasts opt-in below
        }}>
          {toasts.map(t => (
            // c2-392 / task 189: click anywhere on the toast to dismiss.
            // The Undo button still wins via stopPropagation in its handler
            // so accidental clicks near the Undo affordance don't race.
            <div key={t.id} role='status' aria-live='polite'
              onClick={() => dismissToast(t.id)}
              title='Click to dismiss'
              style={{
                padding: `${T.spacing.sm} ${T.spacing.md}`,
                background: C.bgCard, color: C.text,
                border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.md,
                fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
                boxShadow: T.shadows.card,
                animation: t.exiting ? 'scc-toast-out 0.18s ease-in forwards' : 'scc-toast-in 0.18s ease-out',
                display: 'flex', alignItems: 'center', gap: T.spacing.md,
                pointerEvents: 'auto', cursor: 'pointer',
              }}>
              <span>{t.msg}</span>
              {t.onUndo && (
                <button onClick={(e) => {
                  e.stopPropagation(); // don't also fire the dismiss
                  t.onUndo?.();
                  // Dismiss just this toast, leave any siblings alone.
                  dismissToast(t.id);
                }}
                  style={{
                    background: 'transparent', border: `1px solid ${C.accentBorder}`,
                    color: C.accent, padding: '4px 10px', borderRadius: T.radii.sm,
                    fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                    cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
                  }}>Undo</button>
              )}
            </div>
          ))}
          <style>{`
            @keyframes scc-toast-in { from { opacity: 0; transform: translateY(-6px) } to { opacity: 1; transform: translateY(0) } }
            @keyframes scc-toast-out { from { opacity: 1; transform: translateY(0) } to { opacity: 0; transform: translateY(-6px) } }
          `}</style>
        </div>
      )}
      {/* ========== GLOBAL DISCONNECT BANNER ========== */}
      {(showDisconnectBanner || networkOffline) && (() => {
        // Precedence: network offline (amber) > backend offline (red) > WS
        // dropped (red). This tells the user where the problem actually is.
        const isNetwork = networkOffline;
        const bg = isNetwork ? C.yellowBg : C.redBg;
        const fg = isNetwork ? C.yellow : C.red;
        const border = isNetwork ? C.yellowBg /* approx */ : C.redBorder;
        // c2-254 / #116: if we're in the chat-WS backoff window, show the
        // countdown. remainingSec<=0 means the connect is in-flight — fall
        // through to the generic "reconnecting…" text.
        const remainingSec = wsReconnectAt ? Math.max(0, Math.ceil((wsReconnectAt - Date.now()) / 1000)) : 0;
        const msg = isNetwork
          ? 'Your device is offline — check your network connection'
          : backendOffline
            ? `Backend offline — start the server at ${getHost()}:3000`
            : remainingSec > 0
              ? `Connection lost — reconnecting in ${remainingSec}s…`
              : 'Connection lost — reconnecting…';
        return (
          <div role='status' aria-live='polite'
            style={{
              position: 'fixed', top: 0, left: 0, right: 0, zIndex: T.z.toast,
              background: bg, color: fg, borderBottom: `1px solid ${border}`,
              padding: `${T.spacing.sm} ${T.spacing.lg}`, textAlign: 'center',
              fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
              display: 'flex', alignItems: 'center', justifyContent: 'center', gap: T.spacing.sm,
            }}>
            <span style={{
              width: '8px', height: '8px', borderRadius: T.radii.round,
              background: fg, animation: 'scc-pulse 1.4s infinite ease-in-out',
            }} />
            <span>{msg}</span>
            <style>{`@keyframes scc-pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.4 } }`}</style>
          </div>
        );
      })()}
      {/* ========== TOOL CONFIRMATION DIALOG ========== */}
      {pendingConfirm && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: T.z.modal + 60,
          background: 'rgba(0,0,0,0.55)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', padding: T.spacing.lg,
        }}>
          <div role='dialog' aria-modal='true'
            aria-labelledby='scc-confirm-title' aria-describedby='scc-confirm-desc'
            style={{
            width: '100%', maxWidth: '440px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
            padding: T.spacing.xl, boxShadow: T.shadows.modal,
          }}>
            <h3 id='scc-confirm-title' style={{ margin: '0 0 8px', fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>
              {pendingConfirm.tool} requires approval
            </h3>
            <p id='scc-confirm-desc' style={{ margin: '0 0 18px', fontSize: T.typography.sizeMd, color: C.textSecondary, lineHeight: T.typography.lineLoose }}>
              {pendingConfirm.desc}
            </p>
            <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              {/* c2-308: Cancel gets autoFocus — safety-gated prompts
                  ("web search requires approval") should default to the
                  refusal so a reflexive Enter doesn't grant access. */}
              <button autoFocus
                onClick={() => { setPendingConfirm(null); setIsThinking(false); }}
                style={{
                  padding: '10px 18px', background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                }}>Cancel</button>
              <button onClick={pendingConfirm.onApprove}
                style={{
                  padding: '10px 18px', background: C.accent, border: 'none',
                  color: '#fff', borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeMd, fontWeight: 600,
                }}>Allow</button>
            </div>
          </div>
        </div>
      )}

      {/* ========== TERMS OF SERVICE (first run, before welcome) ========== */}
      {!tosAccepted && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: T.z.modal + 60,
          background: C.bg,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          padding: T.spacing.lg,
        }}>
          <div role='dialog' aria-modal='true' aria-labelledby='scc-tos-title'
            style={{
            width: '100%', maxWidth: '560px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
            padding: isMobile ? T.spacing.xl : '36px',
            boxShadow: '0 32px 80px rgba(0,0,0,0.5)',
          }}>
            <h1 id='scc-tos-title' style={{ margin: '0 0 8px', fontSize: '20px', fontWeight: T.typography.weightBold, color: C.text }}>
              PlausiDen <span style={{ color: C.accent }}>AI</span> — Terms of Use
            </h1>
            <p style={{ margin: '0 0 16px', fontSize: T.typography.sizeMd, color: C.textMuted }}>
              Please review before continuing.
            </p>
            <div style={{
              maxHeight: '300px', overflowY: 'auto',
              padding: T.spacing.lg, background: C.bgInput, borderRadius: T.radii.lg,
              fontSize: T.typography.sizeMd, lineHeight: T.typography.lineLoose, color: C.textSecondary,
              marginBottom: T.spacing.xl,
            }}>
              <p><strong>1. Sovereignty.</strong> PlausiDen AI runs entirely on your hardware. Your conversations, knowledge, and data never leave your machine unless you explicitly initiate it (e.g., web search, file export).</p>
              <p><strong>2. Privacy.</strong> No telemetry, analytics, or usage data is collected or transmitted. Diagnostics are local-only and off by default.</p>
              <p><strong>3. Data Ownership.</strong> Everything you create, teach, or store in PlausiDen AI belongs to you. PlausiDen Technologies LLC makes no claim to your data.</p>
              <p><strong>4. AI Limitations.</strong> PlausiDen AI can make mistakes. Verify important information independently. The AI's responses are not professional advice (legal, medical, financial, etc.).</p>
              <p><strong>5. Security.</strong> While we follow defense-in-depth practices (encrypted storage, PSL governance, provenance tracking), no system is perfectly secure. You are responsible for the security of your deployment environment.</p>
              <p><strong>6. Open Source.</strong> PlausiDen AI's core is open source. You may audit, modify, and redistribute the code under its license terms.</p>
              <p><strong>7. No Warranty.</strong> PlausiDen AI is provided as-is. PlausiDen Technologies LLC is not liable for any damages arising from its use.</p>
              <p style={{ marginTop: '12px', fontSize: T.typography.sizeXs, color: C.textDim }}>
                PlausiDen Technologies LLC &middot; <a href="https://plausiden.com" target="_blank" rel="noopener noreferrer" style={{ color: C.accent }}>plausiden.com</a>
              </p>
            </div>
            {/* c2-306: autoFocus so the user can accept with Enter without
                reaching for the mouse. Browser already fires click() on
                Enter/Space for focused buttons — no extra keydown handler
                needed. */}
            <button autoFocus onClick={() => {
              setTosAccepted(true);
              try { localStorage.setItem('lfi_tos_accepted', 'true'); } catch {}
              logEvent('tos_accepted', { version: '1.0' });
            }}
              style={{
                width: '100%', padding: '14px',
                background: C.accent, border: 'none',
                borderRadius: T.radii.lg, color: '#fff',
                fontSize: '15px', fontWeight: T.typography.weightBold,
                cursor: 'pointer', fontFamily: 'inherit',
              }}>
              I accept — continue
            </button>
          </div>
        </div>
      )}

      {/* ========== FIRST-RUN WELCOME ========== */}
      {showWelcome && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: T.z.modal + 50,
          background: 'rgba(0,0,0,0.70)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          padding: T.spacing.lg,
        }}>
          <div role='dialog' aria-modal='true' aria-labelledby='scc-welcome-title'
            style={{
            width: '100%', maxWidth: '520px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
            padding: isMobile ? T.spacing.xl : '36px',
            boxShadow: '0 32px 80px rgba(0,0,0,0.5)',
            textAlign: 'center',
          }}>
            <pre style={{
              margin: '0 auto 18px', color: C.accent,
              fontSize: '32px', fontWeight: 700, letterSpacing: '-0.01em',
              // c0-019: no glow/textShadow. Crisp plain title.
            }}>
            PlausiDen <span style={{ opacity: 0.7 }}>AI</span>
            </pre>
            <h1 id='scc-welcome-title' style={{ margin: '0 0 6px', fontSize: '22px', fontWeight: 700, color: C.text }}>
              Welcome to PlausiDen <span style={{ color: C.accent }}>AI</span>
            </h1>
            <p style={{ margin: '0 0 24px', fontSize: T.typography.sizeBody, color: C.textMuted, lineHeight: 1.6 }}>
              Sovereign AI that runs on your hardware. Private by default. Gets smarter over time.
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', marginBottom: '24px', textAlign: 'left' }}>
              {[
                // c2-307: render mac ⌘ vs non-mac Ctrl for the palette hint.
                { icon: '\u2328', title: `${mod()}+K`, desc: 'Command palette — search everything' },
                { icon: '/', title: '/commands', desc: 'Type / for slash commands' },
                { icon: '+', title: 'Tools', desc: 'Web search, code, analyze, OPSEC' },
                { icon: '\u{1F512}', title: 'Private', desc: 'Data stays on your machine' },
                { icon: '\u{1F9E0}', title: 'Learns', desc: 'Remembers facts across sessions' },
                { icon: '\u{1F3A8}', title: '7 Themes', desc: 'Settings \u2192 Appearance' },
              ].map((item, i) => (
                <div key={i} style={{
                  padding: '10px 12px', background: C.bgInput,
                  border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
                  display: 'flex', gap: '10px', alignItems: 'flex-start',
                }}>
                  <span style={{ fontSize: '18px', flexShrink: 0 }}>{item.icon}</span>
                  <div>
                    <div style={{ fontSize: T.typography.sizeMd, fontWeight: 600, color: C.text }}>{item.title}</div>
                    <div style={{ fontSize: T.typography.sizeXs, color: C.textDim }}>{item.desc}</div>
                  </div>
                </div>
              ))}
            </div>

            <button autoFocus onClick={dismissWelcome}
              style={{
                width: '100%', padding: '14px',
                background: C.accent, border: 'none',
                borderRadius: T.radii.xl, color: '#fff',
                fontSize: '15px', fontWeight: 700,
                cursor: 'pointer', fontFamily: 'inherit',
              }}>
              Get started
            </button>
            <p style={{ margin: '12px 0 0', fontSize: T.typography.sizeXs, color: C.textDim }}>
              Type /help anytime for a full reference. <a href="https://plausiden.com" target="_blank" rel="noopener noreferrer" style={{ color: C.accent }}>plausiden.com</a>
            </p>
          </div>
        </div>
      )}

      {/* ========== TRAINING DASHBOARD ========== */}
      {showTraining && (
        <div onClick={() => setShowTraining(false)}
          style={{
            position: 'fixed', inset: 0, zIndex: 230,
            background: 'rgba(0,0,0,0.55)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', padding: T.spacing.lg,
          }}>
          <div onClick={(e) => e.stopPropagation()}
            role='dialog' aria-modal='true' aria-labelledby='scc-training-title'
            style={{
              width: '100%', maxWidth: '750px', height: '80dvh',
              background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
              display: 'flex', flexDirection: 'column', overflow: 'hidden',
              boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
            }}>
            <div style={{
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
            }}>
              <h2 id='scc-training-title' style={{ margin: 0, fontSize: '16px', fontWeight: 700, color: C.text }}>Training Dashboard</h2>
              <button onClick={() => setShowTraining(false)}
                aria-label='Close training dashboard'
                style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
                {'\u2715'}
              </button>
            </div>
            <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
              {/* Live stats fetched on open */}
              <TrainingDashboardContent host={getHost()} C={C} totalFactsFallback={kg.facts || undefined} />
            </div>
          </div>
        </div>
      )}

      {/* ========== KNOWLEDGE BROWSER ========== */}
      {showKnowledge && (
        <KnowledgeBrowser
          C={C}
          facts={knowledgeFacts}
          concepts={knowledgeConcepts}
          due={knowledgeDue}
          fsrsMeta={knowledgeFsrsMeta}
          loading={knowledgeLoading}
          error={knowledgeError}
          onRetry={fetchKnowledge}
          onClose={() => setShowKnowledge(false)}
          // c2-405 / task 191: zero-state CTA. Close the KB and open Admin
          // → Training so the user can start ingestion.
          onOpenTraining={() => {
            setShowKnowledge(false);
            setAdminInitialTab('training');
            setShowAdmin(true);
          }}
          // c2-433 / #337: FSRS review POST handler. Grades a card (1-4)
          // against /api/fsrs/review, then refetches the due list so the
          // just-graded card disappears from view (or stays, with updated
          // mastery, if it was rated Again). Tiny toast on success, red
          // toast on failure. logEvent tracks the rating for the activity
          // log. Only runs when the due payload carried a fact_key.
          onReview={async (factKey, rating) => {
            try {
              const r = await fetch(`http://${getHost()}:3000/api/fsrs/review`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ fact_key: factKey, rating }),
              });
              if (!r.ok) throw new Error(`HTTP ${r.status}`);
              showToast(`Reviewed: ${['Again','Hard','Good','Easy'][rating - 1]}`);
              logEvent('fsrs_reviewed', { fact_key: factKey, rating });
              knowledgeLastFetchedRef.current = 0;
              fetchKnowledge();
            } catch (e: any) {
              showToast(`Review failed: ${String(e?.message || e || 'unknown error')}`);
            }
          }}
        />
      )}

      {/* ========== GAME MODAL ========== */}
      {showGame === 'tictactoe' && (
        <TicTacToeModal
          C={C}
          board={tttBoard}
          winner={tttWinner}
          onPlay={tttPlay}
          onReset={tttReset}
          onClose={() => setShowGame(null)}
        />
      )}

      {/* ========== TERMINAL MODAL ========== */}
      {/* c2-356 / task #67: xterm.js terminal overlay. Opened via the
          command palette (/terminal) or any future keyboard shortcut. */}
      {showTerminal && (
        <XTermModal C={C} onClose={() => setShowTerminal(false)} />
      )}

      {/* ========== COMMAND PALETTE (Cmd+K) ========== */}
      {showCmdPalette && (() => {
        const items: CmdPaletteItem[] = [
          { id: 'teach-lfi', label: 'Teach LFI a fact', hint: 'Add knowledge to the substrate', group: 'Actions',
            onRun: () => { setShowTeach(true); } },
          { id: 'new-chat', label: 'New chat', hint: 'Start a fresh conversation', group: 'Actions',
            shortcut: '$mod+N',
            onRun: () => { createNewConversation(); } },
          { id: 'clear-chat', label: 'Clear current chat', hint: 'Erase this conversation\'s messages', group: 'Actions',
            onRun: () => { clearChat(); } },
          // c2-401 / task 194: palette entry for the duplicate action.
          { id: 'duplicate-convo', label: 'Duplicate current conversation', hint: 'Clone to a new entry (copy) in the sidebar', group: 'Actions',
            onRun: () => { if (currentConversationId) duplicateConversation(currentConversationId); } },
          { id: 'toggle-sidebar', label: showConvoSidebar ? 'Hide sidebar' : 'Show sidebar', hint: 'Toggle conversations panel', group: 'Actions',
            shortcut: '$mod+B',
            onRun: () => { setShowConvoSidebar(v => !v); } },
          { id: 'toggle-theme', label: `Switch to ${settings.theme === 'dark' ? 'light' : 'dark'} theme`, hint: 'Flip appearance', group: 'Appearance',
            shortcut: '$mod+Shift+D',
            onRun: () => { setSettings(s => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' })); } },
          ...(['dark','light','midnight','forest','sunset','rose','contrast'] as const).map(t => ({
            id: `theme-${t}`, label: `Theme: ${t}`, hint: 'Apply this color scheme', group: 'Appearance',
            onRun: () => setSettings(s => ({ ...s, theme: t })),
          })),
          // c2-428 / #339 pivot: Model:Pulse/Bridge/BigBrain palette entries
          // removed. LFI is post-LLM — no transformer tiers.
          ...skills.filter(s => s.available).map(s => ({
            id: `skill-${s.id}`, label: `Use ${s.label}`, hint: s.hint, group: 'Skills',
            onRun: () => { setActiveSkill(s.id); inputRef.current?.focus(); },
          })),
          { id: 'view-chat', label: 'Go to Chat', hint: 'Top-level section', group: 'Navigate',
            shortcut: '$mod+1',
            onRun: () => { setActiveView('chat'); setShowAdmin(false); } },
          { id: 'view-classroom', label: 'Go to Classroom', hint: 'Training, grades, datasets', group: 'Navigate',
            shortcut: '$mod+2',
            onRun: () => { setActiveView('classroom'); setShowAdmin(false); } },
          // c2-433 / deep-link entries for the three live dashboards.
          // Clicking jumps to Classroom AND sets the sub-tab in one step.
          { id: 'view-classroom-ledger', label: 'Go to Ledger', hint: 'Pending contradictions', group: 'Navigate',
            onRun: () => { openClassroomSub('ledger'); } },
          { id: 'view-classroom-drift', label: 'Go to Drift', hint: 'System health trends', group: 'Navigate',
            onRun: () => { openClassroomSub('drift'); } },
          { id: 'view-classroom-runs', label: 'Go to Ingest Runs', hint: 'Active + recent ingest history', group: 'Navigate',
            onRun: () => { openClassroomSub('runs'); } },
          { id: 'view-admin', label: 'Open Admin console', hint: 'Dashboard, domains, system', group: 'Navigate',
            shortcut: '$mod+3',
            onRun: () => { setShowAdmin(true); } },
          { id: 'open-settings', label: 'Open settings', hint: 'All preferences', group: 'Navigate',
            shortcut: '$mod+,',
            onRun: () => { setShowSettings(true); } },
          { id: 'open-shortcuts', label: 'Keyboard shortcuts', hint: 'Reopen anytime', group: 'Navigate',
            shortcut: '?',
            onRun: () => { setShowShortcuts(true); } },
          { id: 'open-knowledge', label: 'Knowledge browser', hint: 'Facts, concepts, reviews', group: 'Navigate',
            onRun: () => { setShowKnowledge(true); fetchKnowledge(); } },
          { id: 'open-logs', label: 'Open activity logs', hint: 'Chat log + UI events', group: 'Navigate',
            shortcut: '$mod+Shift+L',
            onRun: () => { setAdminInitialTab('logs'); setShowAdmin(true); fetchChatLog(50); } },
          { id: 'open-admin-tokens', label: 'Open Admin → Tokens', hint: 'Issue / list / revoke capability tokens', group: 'Navigate',
            onRun: () => { setAdminInitialTab('tokens'); setShowAdmin(true); } },
          { id: 'open-admin-proof', label: 'Open Admin → Proof', hint: 'Lean4 verdict distribution across facts', group: 'Navigate',
            onRun: () => { setAdminInitialTab('proof'); setShowAdmin(true); } },
          { id: 'open-admin-diag', label: 'Open Admin → Diag', hint: 'Runtime diagnostic log (errors, warnings, events)', group: 'Navigate',
            onRun: () => { setAdminInitialTab('diag'); setShowAdmin(true); } },
          { id: 'open-user-guide', label: 'Open user guide', hint: 'Hands-on training guide — reading the UI, teach paths, troubleshooting', group: 'Help',
            onRun: () => { setAdminInitialTab('docs'); setShowAdmin(true); } },
          { id: 'start-tour', label: 'Start guided tour', hint: '60-second interactive walkthrough — desktop and mobile friendly', group: 'Help',
            onRun: () => { setShowTour(true); } },
          // c2-433: diag export — copy the runtime ring buffer (last 500
          // entries, includes auto-captured console warn/error + window
          // errors) to clipboard. Useful when filing an issue.
          { id: 'export-diag-logs', label: 'Export diagnostic logs', hint: 'Copy last 500 log entries + error captures to clipboard', group: 'Navigate',
            onRun: async () => {
              try {
                const payload = diag.export();
                await navigator.clipboard.writeText(payload);
                const snap = diag.snapshot();
                const counts = {
                  error: snap.filter(e => e.level === 'error').length,
                  warn: snap.filter(e => e.level === 'warn').length,
                };
                showToast(`Diag copied — ${snap.length} entries (${counts.error} err · ${counts.warn} warn)`);
              } catch (e: any) {
                showToast(`Diag copy failed: ${String(e?.message || e || 'unknown')}`);
              }
            },
          },
          { id: 'clear-diag-logs', label: 'Clear diagnostic logs', hint: 'Zero the local ring buffer + localStorage mirror', group: 'Navigate',
            onRun: () => {
              if (!window.confirm('Clear diagnostic log buffer? Not recoverable.')) return;
              diag.clear();
              showToast('Diag logs cleared');
            },
          },
          // c2-433 / #354: verify-fact-by-key palette entry. Fast path
          // for operators with a fact_key in clipboard — skips the
          // click-open-popover-click-Verify dance. Toasts the verdict
          // and also pops the popover so the full context is visible.
          // c2-433 / #357 + Tier-5 #38 lead-in: dump the last assistant
          // message's citations to clipboard as a formatted list. Useful
          // for ticket-filing + downstream audit.
          { id: 'copy-last-citations', label: "Copy last reply's citations", hint: 'Extract [fact:KEY] + source/similarity from the most recent assistant turn', group: 'Skills',
            onRun: () => {
              const last = [...messages].reverse().find(m => m.role === 'assistant' && (m.content || '').trim().length > 0);
              if (!last) { showToast('No assistant reply yet'); return; }
              // Parse [fact:KEY] (source: X, similarity N%) sequences in order.
              const combined = /\[(?:fact|k):([A-Za-z0-9_\-:]{1,80})\](?:\s*[\(\[]source:\s*([^,\)\]]+),\s*similarity\s+(\d+)%[\)\]])?/g;
              const rows: Array<{ key: string; source: string | null; similarity: number | null }> = [];
              const seen = new Set<string>();
              let m: RegExpExecArray | null;
              while ((m = combined.exec(last.content)) !== null) {
                const key = m[1];
                if (seen.has(key)) continue; // dedupe
                seen.add(key);
                rows.push({
                  key,
                  source: m[2]?.trim() || null,
                  similarity: m[3] ? Number(m[3]) : null,
                });
              }
              if (rows.length === 0) { showToast('No citations in the last reply'); return; }
              const sources = new Set(rows.map(r => r.source).filter(Boolean) as string[]);
              const lines = [
                `# Citations for reply at ${new Date(last.timestamp).toISOString()}`,
                `# ${rows.length} fact${rows.length === 1 ? '' : 's'} from ${sources.size} source${sources.size === 1 ? '' : 's'}`,
                '',
                ...rows.map(r => {
                  const meta = r.source ? `  [${r.source}${r.similarity != null ? ` ${r.similarity}%` : ''}]` : '';
                  return `- ${r.key}${meta}`;
                }),
              ].join('\n');
              navigator.clipboard.writeText(lines)
                .then(() => {
                  showToast(`Copied ${rows.length} citation${rows.length === 1 ? '' : 's'}`);
                  logEvent('copy_last_citations', { count: rows.length, source_count: sources.size });
                })
                .catch(() => showToast('Clipboard blocked'));
            },
          },
          { id: 'verify-fact', label: 'Verify fact by key', hint: 'POST /api/proof/verify — trigger Lean4 check on a fact', group: 'Skills',
            onRun: async () => {
              const key = window.prompt('Fact key (e.g. fact:water-boils-at-100c or concept:volcano)');
              if (!key || !key.trim()) return;
              const fk = key.trim();
              try {
                const r = await fetch(`http://${getHost()}:3000/api/proof/verify`, {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ fact_key: fk }),
                });
                if (!r.ok) {
                  if (r.status === 404) { showToast('Verify endpoint not live or fact not found'); return; }
                  throw new Error(`HTTP ${r.status}`);
                }
                const data = await r.json().catch(() => ({}));
                const v: string = data.verdict ?? data.proof_status ?? data.status ?? 'done';
                showToast(`Verify ${fk}: ${v}`);
                logEvent('proof_verify_triggered', { fact_key: fk, verdict: v, via: 'cmdk' });
                // Open the popover at viewport-center anchor so the new
                // verdict row + checked_at timestamp are visible.
                const vw = typeof window !== 'undefined' ? window.innerWidth : 1024;
                const vh = typeof window !== 'undefined' ? window.innerHeight : 768;
                openFactKey(fk, new DOMRect(vw / 2 - 10, vh / 3, 20, 20));
              } catch (e: any) {
                showToast(`Verify failed: ${String(e?.message || e || 'unknown')}`);
              }
            },
          },
          // c2-433 / #353: parse-english palette entry. Ad-hoc tool for
          // operators to sanity-check the English parser — paste a
          // sentence, get back the extracted tuples summarized by
          // canonical predicate. Full payload lands in logEvent for
          // post-hoc inspection via Activity modal.
          { id: 'parse-english', label: 'Parse English → tuples', hint: 'POST to /api/parse/english, toast predicate summary', group: 'Skills',
            onRun: async () => {
              const text = window.prompt('Parse English (paste one sentence)');
              if (!text || !text.trim()) return;
              try {
                const r = await fetch(`http://${getHost()}:3000/api/parse/english`, {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ text: text.trim() }),
                });
                if (!r.ok) throw new Error(`HTTP ${r.status}`);
                const data = await r.json();
                const tuples: any[] = Array.isArray(data) ? data
                  : Array.isArray(data?.tuples) ? data.tuples
                  : Array.isArray(data?.relations) ? data.relations
                  : [];
                if (tuples.length === 0) {
                  showToast('Parsed: no tuples extracted');
                } else {
                  const byPred = new Map<string, number>();
                  for (const t of tuples) {
                    const p = String(t.pred ?? t.predicate ?? t.p ?? '?');
                    byPred.set(p, (byPred.get(p) || 0) + 1);
                  }
                  const summary = [...byPred.entries()]
                    .sort((a, b) => b[1] - a[1])
                    .map(([p, n]) => `${n} ${p}`)
                    .join(' · ');
                  showToast(`Parsed ${tuples.length}: ${summary}`);
                }
                logEvent('parse_english', { text: text.trim(), tuples });
              } catch (e: any) {
                showToast(`Parse failed: ${String(e?.message || e || 'unknown')}`);
              }
            },
          },
          // c2-433 / #274: resolve phrase → concept via /api/concepts/resolve.
          // Uses window.prompt so we don't need to build a new modal surface.
          // On success, open the shared fact popover at the palette's position
          // anchored just below the viewport-center with a small dummy rect —
          // popover positioning logic clamps it to the visible area.
          { id: 'resolve-concept', label: 'Resolve phrase → concept', hint: 'Look up a concept_id by language + text', group: 'Skills',
            onRun: async () => {
              const raw = window.prompt('Resolve phrase (format: "lang text", e.g. "es volcán")');
              if (!raw) return;
              const trimmed = raw.trim();
              const parts = trimmed.split(/\s+/);
              const lang = parts.length > 1 && parts[0].length <= 4 ? parts.shift()! : 'en';
              const text = parts.join(' ');
              if (!text) { showToast('Resolve needs a phrase'); return; }
              try {
                const r = await fetch(`http://${getHost()}:3000/api/concepts/resolve?language=${encodeURIComponent(lang)}&text=${encodeURIComponent(text)}`);
                if (!r.ok) throw new Error(`HTTP ${r.status}`);
                const data = await r.json();
                const conceptId: string | null = data?.concept_id ?? data?.id ?? (typeof data === 'string' ? data : null);
                const fallback: boolean = data?.fallback === true || data?.linked === false;
                if (!conceptId) { showToast('No concept_id returned'); return; }
                showToast(`Resolved ${lang}:${text} → ${conceptId}${fallback ? ' (fallback)' : ''}`);
                logEvent('concept_resolved', { lang, text, concept_id: conceptId, fallback });
                // Anchor popover near viewport center. openFactKey computes
                // offsets from the rect, so any plausible on-screen rect works.
                const vw = typeof window !== 'undefined' ? window.innerWidth : 1024;
                const vh = typeof window !== 'undefined' ? window.innerHeight : 768;
                openFactKey(`concept:${conceptId}`, new DOMRect(vw / 2 - 10, vh / 3, 20, 20));
              } catch (e: any) {
                showToast(`Resolve failed: ${String(e?.message || e || 'unknown')}`);
              }
            },
          },
          { id: 'toggle-dev', label: `${settings.developerMode ? 'Disable' : 'Enable'} developer mode`, hint: 'Telemetry + plan panel', group: 'Navigate',
            shortcut: '$mod+D',
            onRun: () => { setSettings(s => ({ ...s, developerMode: !s.developerMode })); } },
          ...conversations.slice(0, 20).map(c => {
            // c2-422 / task 209: body excerpt for fuzzy matching.
            // Concatenates the last ~6 message bodies then caps at 500
            // chars so a long chat doesn't balloon the palette's in-memory
            // items array. Nothing is rendered — searchBody is score-only.
            const recent = c.messages.slice(-6).map(m => m.content).join(' ').replace(/\s+/g, ' ');
            const searchBody = recent.length > 500 ? recent.slice(0, 500) : recent;
            return {
              id: `convo-${c.id}`, label: c.title, hint: `${c.messages.length} message${c.messages.length === 1 ? '' : 's'}`, group: 'Conversations',
              searchBody,
              onRun: () => { setCurrentConversationId(c.id); },
            };
          }),
        ];
        return (
          <CommandPalette
            C={C} isMobile={isMobile}
            items={items}
            query={cmdQuery} setQuery={setCmdQuery}
            index={cmdIndex} setIndex={setCmdIndex}
            onClose={() => setShowCmdPalette(false)}
            onItemRun={(id) => { logEvent('cmd_palette_run', { id }); bumpCmdRecency(id); }}
            recency={cmdRecency}
          />
        );
      })()}

      {/* ========== ADMIN CONSOLE MODAL (c0-017) ========== */}
      {showAdmin && (
        // Local error boundary: if any Admin panel throws (bad shape from
        // /api/admin/dashboard, unexpected field, etc.) we only lose the
        // modal's contents, not the whole chat UI.
        <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent}
          inlineMode label="AdminModal"
          onReset={() => { setShowAdmin(false); setAdminInitialTab('dashboard'); }}>
          <AdminModal
            C={C}
            host={host}
            factsCount={kg.facts}
            sourcesCount={kg.sources}
            localEvents={localEvents}
            initialTab={adminInitialTab}
            isMobile={isMobile}
            onClose={() => { setShowAdmin(false); setAdminInitialTab('dashboard'); }}
          />
        </AppErrorBoundary>
      )}

      {/* ========== ACTIVITY / LOGS MODAL ========== */}
      {showActivity && (
        <ActivityModal
          C={C}
          tab={activityTab}
          onTabChange={(t) => setActivityTab(t)}
          onClose={() => setShowActivity(false)}
          serverChatLog={serverChatLog}
          chatLogError={chatLogError}
          chatLogFetchedAt={chatLogFetchedAt}
          onRefreshChatLog={() => fetchChatLog(50)}
          localEvents={localEvents}
          isConnected={isConnected}
          currentTier={currentTier}
          thermalThrottled={stats.is_throttled}
          ramLabel={`${ramFmt.value} ${ramFmt.unit}`}
          cpuTempC={stats.cpu_temp_c}
          factsLabel={kg.facts ? compactNum(kg.facts) : (kgLastOk ? '0' : kgLastError ? 'Unreachable' : 'Loading…')}
          conceptsLabel={kg.concepts ? String(kg.concepts) : (kgLastOk ? '0' : kgLastError ? 'Unreachable' : 'Loading…')}
          logicDensity={stats.logic_density}
          qosReport={qosReport}
          onRefreshQos={fetchQos}
          onRefreshFacts={fetchFacts}
          onClearLocalEvents={() => {
            setLocalEvents([]);
            showToast('Event log cleared');
            logEvent('events_cleared', {});
          }}
        />
      )}

      {/* ========== SHORTCUTS CHEATSHEET (opens with "?") ========== */}
      {showShortcuts && <ShortcutsModal C={C} onClose={() => setShowShortcuts(false)}
        onOpenUserGuide={() => { setShowShortcuts(false); setAdminInitialTab('docs'); setShowAdmin(true); }} />}
      {/* #352 interactive walkthrough. Desktop: spotlight next to target.
          Mobile: tooltip pinned to bottom, swipe to navigate. Wrapped in
          an inline error boundary so a step-render crash closes the tour
          instead of white-screening the app (#354 stability). */}
      {showTour && (
        <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent}
          inlineMode label="TourOverlay"
          onReset={() => setShowTour(false)}>
      <TourOverlay C={C} isMobile={isMobile} open={showTour}
        onClose={() => {
          setShowTour(false);
          try { localStorage.setItem('lfi_tour_seen_v1', '1'); } catch { /* silent */ }
        }}
        steps={[
          {
            key: 'intro',
            title: 'Welcome to PlausiDen',
            body: <>LFI is a post-LLM substrate — it only answers from what it knows. This 60-second tour shows how to use it and how to teach it new facts. You can skip any time with <kbd>Esc</kbd> or the ✕ button.</>,
          },
          {
            key: 'chat-input',
            target: '[data-tour="chat-input"]',
            title: 'Ask a question',
            body: <>Type here and press Enter. If LFI has the facts, you get substrate-composed prose with clickable <code>[fact:KEY]</code> citations. If it doesn't, you get an honest refusal — no fabrication.</>,
          },
          {
            key: 'teach',
            title: 'Teach LFI proactively',
            body: <>Press <kbd>Cmd/Ctrl+K</kbd> and choose "Teach LFI a fact", or type <code>/teach</code> in the chat input. Write a plain-English fact — LFI extracts tuples automatically. This is how you train it between refusals.</>,
          },
          {
            key: 'refusal',
            title: 'On refusal: one-click teach',
            body: <>When LFI refuses ("No HDC match" or "nothing clears the 0.70 trust threshold"), a yellow Teach LFI card appears right on the reply. Click it, write the answer, and LFI learns. Ask again — now it knows.</>,
          },
          {
            key: 'knowledge',
            target: 'aside [aria-label*="Knowledge"], [aria-label="Open knowledge browser"]',
            title: 'Browse and review',
            body: <>Open the Knowledge Browser to see everything LFI has learned, filter by keyword, and review due cards with FSRS (Again / Hard / Good / Easy). Reviewing is how LFI consolidates memory.</>,
          },
          {
            key: 'classroom',
            title: 'Classroom: drill into training state',
            body: <>The Classroom view (<kbd>Cmd/Ctrl+2</kbd>) has 12 sub-tabs covering ingestion runs, contradictions, drift, and reports. The Drift tab has one-click Kick-ingest / Encode-HDC / Auto-resolve-ledger buttons.</>,
          },
          {
            key: 'admin',
            title: 'Admin: backup, docs, diagnostics',
            body: <>The Admin console (<kbd>Cmd/Ctrl+3</kbd>) has 12 tabs: Dashboard (with Backup brain.db + Recent Teach Activity), Proof, Diag, and Docs (the full user guide). Use Backup before a risky ingest run.</>,
          },
          {
            key: 'help',
            target: '[data-tour="help-button"]',
            title: 'Find help from anywhere',
            body: <>This <strong>Help & guide</strong> button opens the full manual. You can also hit <kbd>?</kbd> for the keyboard cheatsheet, <kbd>Cmd/Ctrl+K</kbd> → "Open user guide", or <code>/guide</code> in chat.</>,
          },
          {
            key: 'done',
            title: "You're ready",
            body: <>That's the tour. Everything needed to use + train LFI is one click from anywhere. Re-run from <kbd>Cmd/Ctrl+K</kbd> → "Start guided tour" any time.</>,
          },
        ] as TourStep[]} />
        </AppErrorBoundary>
      )}

      {/* ========== SETTINGS MODAL ========== */}
      {showSettings && (
        <SettingsModal
          C={C} isMobile={isMobile}
          settings={settings as any}
          setSettings={setSettings as any}
          tab={settingsTab}
          onTabChange={(t) => setSettingsTab(t)}
          onPreviewTheme={setPreviewTheme}
          onClose={() => { setPreviewTheme(null); setShowSettings(false); }}
          currentTier={currentTier}
          host={getHost()}
          onTierSelect={(tier) => { setCurrentTier(tier); handleTierSwitch(tier); }}
          onExportEvents={() => { exportEvents(); logEvent('export_events', {}); }}
          onExportConversations={() => {
            try {
              const blob = new Blob([JSON.stringify(conversations, null, 2)], { type: 'application/json' });
              const url = URL.createObjectURL(blob);
              const a = document.createElement('a');
              a.href = url;
              a.download = `plausiden-conversations-${new Date().toISOString().slice(0,19).replace(/:/g,'-')}.json`;
              document.body.appendChild(a); a.click(); a.remove();
              setTimeout(() => URL.revokeObjectURL(url), 1000);
              logEvent('export_conversations', { count: conversations.length });
              showToast('Conversations exported');
            } catch (e) { console.warn(e); }
          }}
          onExportAllJson={() => {
            try {
              exportAllAsJson(conversations, settings);
              logEvent('export_all', { conversations: conversations.length });
              showToast('Full backup exported');
            } catch (e) { console.warn(e); showToast('Export failed'); }
          }}
          onImportBackup={async (file) => {
            // c2-241 / #102: validate schema, merge conversations (dedupe by
            // id — incoming wins on conflict), optionally replace settings.
            // Never wipe existing data on a bad file: everything validates
            // before any state is touched.
            try {
              const MAX = 50 * 1024 * 1024; // 50 MB cap — pathological backups blocked
              if (file.size > MAX) {
                showToast('Import failed: file too large');
                return;
              }
              const text = await file.text();
              const payload = JSON.parse(text);
              if (!payload || typeof payload !== 'object') throw new Error('not an object');
              if (payload.schemaVersion !== 1) throw new Error(`unsupported schemaVersion ${payload.schemaVersion}`);
              if (!Array.isArray(payload.conversations)) throw new Error('conversations missing');
              // Spot-check the first conversation to avoid admitting rubbish.
              const incoming = payload.conversations as Conversation[];
              if (incoming.length > 0) {
                const head = incoming[0] as any;
                if (typeof head?.id !== 'string' || !Array.isArray(head?.messages)) {
                  throw new Error('conversation shape invalid');
                }
              }
              const mergeSettings = !!payload.settings && confirm(
                `Import ${incoming.length} conversation${incoming.length === 1 ? '' : 's'} + replace settings?\n\n` +
                'Click OK to replace settings, Cancel to keep current settings (conversations still imported).'
              );
              // c2-296: compute added/updated before setConversations so the
              // summary toast can report both numbers. Previously the toast
              // only had incoming.length (total) — users who just imported a
              // backup to restore a few recent convos couldn't tell how many
              // of their existing chats were overwritten.
              const existingIds = new Set(conversations.map(c => c.id));
              let added = 0, updated = 0;
              for (const c of incoming) {
                if (!c || typeof c.id !== 'string') continue;
                if (existingIds.has(c.id)) updated++; else added++;
              }
              setConversations(prev => {
                const byId = new Map<string, Conversation>();
                for (const c of prev) byId.set(c.id, c);
                for (const c of incoming) {
                  if (!c || typeof c.id !== 'string') continue;
                  byId.set(c.id, c);
                }
                return Array.from(byId.values());
              });
              if (mergeSettings) {
                setSettings(payload.settings as any);
              }
              logEvent('import_backup', { added, updated, settingsReplaced: mergeSettings });
              const parts: string[] = [];
              if (added > 0) parts.push(`${added} new`);
              if (updated > 0) parts.push(`${updated} updated`);
              if (mergeSettings) parts.push('settings replaced');
              const summary = parts.length > 0 ? parts.join(', ') : `${incoming.length} conversation${incoming.length === 1 ? '' : 's'}`;
              showToast(`Imported: ${summary}`);
            } catch (e: any) {
              console.warn('[import-backup]', e);
              showToast(`Import failed: ${String(e?.message || e).slice(0, 80)}`);
            }
          }}
          onClearHistory={() => {
            // c2-288: include counts in the confirm so users see the stakes.
            const convoCount = conversations.length;
            const msgCount = conversations.reduce((sum, c) => sum + c.messages.length, 0);
            const label = convoCount === 1
              ? `1 conversation (${msgCount} message${msgCount === 1 ? '' : 's'})`
              : `${convoCount} conversations (${msgCount} messages)`;
            if (confirm(`Clear ${label} from this device?\n\nExport a Full backup first if you want to restore later.`)) {
              localStorage.removeItem(LS_MESSAGES_KEY);
              localStorage.removeItem(LS_CONVERSATIONS_KEY);
              setConversations([]); setMessages([]);
              logEvent('clear_history', { convoCount, msgCount });
            }
          }}
          onResetSettings={() => {
            if (confirm('Reset all settings to defaults?')) {
              setSettings(defaultSettings);
              logEvent('reset_settings', {});
            }
          }}
          onDeleteAccount={() => {
            if (!confirm('Delete account?\n\nErases every conversation, every setting, every logged event from this browser. Cannot be undone.')) return;
            if (!confirm('Really delete everything? Last chance.')) return;
            try {
              localStorage.removeItem(LS_MESSAGES_KEY);
              localStorage.removeItem(LS_CONVERSATIONS_KEY);
              localStorage.removeItem(LS_CURRENT_KEY);
              localStorage.removeItem(LS_EVENTS_KEY);
              localStorage.removeItem('lfi_settings');
              localStorage.removeItem('lfi_auth');
            } catch {}
            setMessages([]); setConversations([]); setSettings(defaultSettings);
            logEvent('account_deleted', {});
            alert('Account data erased. Reload to start fresh.');
            setShowSettings(false);
          }}
          conversationCount={conversations.length}
          messageCount={conversations.reduce((n, c) => n + c.messages.length, 0)}
        />
      )}

      {/* ========== HEADER ========== */}
      <header role='banner' aria-label='Dashboard header' style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: isDesktop ? '10px 24px' : '8px 14px',
        background: C.bgCard,
        borderBottom: `1px solid ${C.border}`,
        flexShrink: 0, zIndex: 50, minHeight: isMobile ? '48px' : '52px',
        // Bible §6.1: all tap targets ≥44px on mobile
        touchAction: 'manipulation',
      }}>
        {/* Left: sidebar toggle + status/inline stats */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <button onClick={() => setShowConvoSidebar(v => !v)}
            title={showConvoSidebar ? 'Hide chats sidebar' : 'Show chats sidebar'}
            aria-label={showConvoSidebar ? 'Hide chats sidebar' : 'Show chats sidebar'}
            aria-pressed={showConvoSidebar}
            style={{
              width: '36px', height: '36px', flexShrink: 0,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: showConvoSidebar ? C.accentBg : 'transparent',
              border: `1px solid ${showConvoSidebar ? C.accentBorder : C.border}`,
              borderRadius: T.radii.lg,
              color: showConvoSidebar ? C.accent : C.textMuted,
              cursor: 'pointer', fontFamily: 'inherit',
            }}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="4" width="18" height="16" rx="2"/>
              <line x1="9" y1="4" x2="9" y2="20"/>
            </svg>
          </button>
          {/* c2-433 / task 235: mobile-only New Chat button. Desktop has the
              prominent New Chat button in the open sidebar; mobile users had
              to open the drawer + tap inside it (3 taps). This collapses the
              flow to 1 tap. Hidden on tablet/desktop where the sidebar is
              persistent and the in-sidebar button serves. */}
          {isMobile && (
            <button onClick={() => createNewConversation()}
              title='New chat' aria-label='Start a new chat'
              style={{
                width: '36px', height: '36px', flexShrink: 0,
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                color: C.accent, borderRadius: T.radii.lg,
                cursor: 'pointer', fontFamily: 'inherit',
              }}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="5" x2="12" y2="19"/>
                <line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
            </button>
          )}
          {/* c2-415 / BIG #218 mobile: the "PlausiDen AI" wordmark eats ~90px
              on narrow viewports; tabs + hamburger + avatar already identify
              the app. Keep the incognito shield indicator — it's a security
              tell users need to see regardless of viewport. */}
          {!isMobile && (
            <div style={{ fontSize: T.typography.sizeMd, fontWeight: 800, letterSpacing: '0.02em', color: C.text, display: 'flex', alignItems: 'center', gap: '6px' }}>
              PlausiDen <span style={{ color: C.accent }}>AI</span>
              {isCurrentIncognito && (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={C.accent} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" title="Incognito mode active">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                </svg>
              )}
            </div>
          )}
          {isMobile && isCurrentIncognito && (
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={C.accent} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" title="Incognito mode active">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
            </svg>
          )}
          {/* Inline stats — developer-only per design review. */}
          {isDesktop && settings.developerMode && (
            <div style={{ display: 'flex', gap: T.spacing.lg, marginLeft: '8px', fontSize: T.typography.sizeSm, color: C.textDim }}>
              <span title={`Used ${ramUsedFmt.value} ${ramUsedFmt.unit} of ${ramTotalFmt.value} ${ramTotalFmt.unit} total`}>
                {ramTotal > 0 ? `${ramUsedFmt.value}/${ramTotalFmt.value} ${ramTotalFmt.unit}` : `${ramFmt.value} ${ramFmt.unit}`}
              </span>
              <span>{stats.cpu_temp_c.toFixed(0)}{'\u00B0'}C</span>
              <span style={{ color: tierColor(currentTier) }}>{currentTier}</span>
            </div>
          )}
        </div>

        {/* Center: view switcher — Chat / Classroom / Admin (c0-027).
            c2-433 / task 272: WAI-ARIA kbd nav (Arrow/Home/End + roving
            tabindex) per #178. Cmd+1/2/3 still work as the global jump;
            arrow-nav is the within-tablist standard. */}
        {(() => {
          const VIEWS = [
            { id: 'chat' as const,      label: 'Chat',      onClick: () => { setActiveView('chat'); setShowAdmin(false); } },
            { id: 'classroom' as const, label: 'Classroom', onClick: () => { setActiveView('classroom'); setShowAdmin(false); } },
            // c2-433: clicking Admin (any tab) dismisses the unseen-error
            // badge since the operator is now in the tools surface.
            { id: 'admin' as const,     label: 'Admin',     onClick: () => { setShowAdmin(true); setDiagUnseenErrors(0); } },
          ];
          const activeIdx = VIEWS.findIndex(v => (v.id === 'admin' ? showAdmin : (activeView === v.id && !showAdmin)));
          const onTabKey = (e: React.KeyboardEvent) => {
            if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight' && e.key !== 'Home' && e.key !== 'End') return;
            e.preventDefault();
            let next = activeIdx < 0 ? 0 : activeIdx;
            if (e.key === 'ArrowLeft') next = (next - 1 + VIEWS.length) % VIEWS.length;
            else if (e.key === 'ArrowRight') next = (next + 1) % VIEWS.length;
            else if (e.key === 'Home') next = 0;
            else if (e.key === 'End') next = VIEWS.length - 1;
            VIEWS[next].onClick();
          };
          return (
        <div role='tablist' aria-label='App sections' onKeyDown={onTabKey}
          style={{ display: 'flex', gap: '2px', order: 2, flexShrink: 0 }}>
          {VIEWS.map(v => {
            const isActive = v.id === 'admin' ? showAdmin : (activeView === v.id && !showAdmin);
            // c2-433 / #298: Classroom tab gets a small red badge with the
            // pending-contradictions count (from /api/contradictions/recent
            // polled every 30s). Only rendered when count > 0 — no zero-
            // state noise. Compact number formatting keeps the badge narrow
            // even at 100+ pending.
            const badge = (v.id === 'classroom' && typeof contradictionsPending === 'number' && contradictionsPending > 0)
              ? contradictionsPending : null;
            // c2-433: Admin tab gets a small red DOT (not a number) when
            // the diag ring buffer has unseen errors. Pairs with the
            // diag logger's auto-capture of console.error + window errors.
            // Dismissed the moment the user clicks Admin (see VIEWS above).
            const adminErrDot = (v.id === 'admin' && diagUnseenErrors > 0);
            const tabTitle = badge !== null ? `${badge} pending contradiction${badge === 1 ? '' : 's'} in the ledger`
              : adminErrDot ? `${diagUnseenErrors} unseen error${diagUnseenErrors === 1 ? '' : 's'} in the diag log`
              : undefined;
            return (
              <button key={v.id} onClick={v.onClick}
                role='tab' aria-selected={isActive}
                tabIndex={isActive ? 0 : -1}
                title={tabTitle}
                style={{
                  position: 'relative',
                  padding: isMobile ? '6px 10px' : '7px 14px',
                  fontSize: T.typography.sizeSm, fontWeight: 600,
                  background: isActive ? C.accentBg : 'transparent',
                  border: `1px solid ${isActive ? C.accentBorder : 'transparent'}`,
                  color: isActive ? C.accent : C.textMuted,
                  borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                  whiteSpace: 'nowrap',
                }}>
                {v.label}
                {adminErrDot && (
                  <span aria-label={`${diagUnseenErrors} unseen diag errors`}
                    style={{
                      position: 'absolute', top: '-3px', right: '-3px',
                      width: '9px', height: '9px',
                      background: C.red, borderRadius: '50%',
                      border: `1.5px solid ${C.bg}`,
                      animation: 'scc-badge-rise-pulse 2.4s ease-out 1',
                    }} />
                )}
                {badge !== null && (
                  <span key={contradictionsPulseId}
                    aria-label={`${badge} pending contradictions`}
                    style={{
                      position: 'absolute', top: '-4px', right: '-4px',
                      minWidth: '16px', height: '16px', padding: '0 4px',
                      background: C.red, color: '#fff',
                      fontSize: '9px', fontWeight: 800,
                      borderRadius: '8px', lineHeight: '16px',
                      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                      border: `1.5px solid ${C.bg}`,
                      fontFamily: T.typography.fontMono, letterSpacing: 0,
                      animation: contradictionsPulseId > 0 ? 'scc-badge-rise-pulse 1.8s ease-out 2' : undefined,
                      transformOrigin: 'center',
                    }}>{badge > 99 ? '99+' : badge}</span>
                )}
              </button>
            );
          })}
        </div>
          );
        })()}

        {/* c2-367 / task 94: prominent New Chat button. Sits in the header
            cluster just before the account menu so power users don't have
            to dig into the dropdown. Accent-tinted to distinguish from the
            muted header controls. Hidden on mobile where horizontal space
            is precious and the ⌘N shortcut + account dropdown cover it. */}
        {isDesktop && (
          <button onClick={() => createNewConversation(false)}
            title='New chat (⌘N)' aria-label='New chat'
            style={{
              order: 2, display: 'flex', alignItems: 'center', gap: '6px',
              padding: `${T.spacing.sm} ${T.spacing.md}`,
              background: C.accentBg, border: `1px solid ${C.accentBorder}`,
              color: C.accent, fontWeight: T.typography.weightSemibold,
              borderRadius: T.radii.sm, cursor: 'pointer', fontFamily: 'inherit',
              fontSize: T.typography.sizeSm, whiteSpace: 'nowrap',
              transition: `background ${T.motion.fast}, border-color ${T.motion.fast}`,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = C.accent;
              e.currentTarget.style.color = '#fff';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = C.accentBg;
              e.currentTarget.style.color = C.accent;
            }}>
            <svg width='14' height='14' viewBox='0 0 24 24' fill='none'
              stroke='currentColor' strokeWidth='2.4' strokeLinecap='round' strokeLinejoin='round'>
              <line x1='12' y1='5' x2='12' y2='19' />
              <line x1='5' y1='12' x2='19' y2='12' />
            </svg>
            New chat
          </button>
        )}
        {/* Right: account on the far right. `order: 3` in the flex header
            pushes it past the tier/theme cluster regardless of DOM order. */}
        <div style={{ position: 'relative', order: 3 }} ref={accountMenuRef}>
          <button onClick={() => setShowAccountMenu(v => !v)}
            title='Account'
            aria-label='Account menu'
            aria-haspopup='menu'
            aria-expanded={showAccountMenu}
            style={{
              display: 'flex', alignItems: 'center', gap: '10px',
              padding: '4px 10px 4px 4px',
              background: showAccountMenu ? C.bgHover : 'transparent',
              border: `1px solid ${showAccountMenu ? C.border : 'transparent'}`,
              borderRadius: T.radii.xl, cursor: 'pointer', fontFamily: 'inherit',
            }}>
            {/* Avatar — c2-433 / task 252b: mobile-only connection status
                dot rendered as a small ring on the avatar's bottom-right.
                Online = green, Offline = red. Mobile users have no other
                visual cue (the "Online" text label is desktop-only). */}
            <div style={{
              width: '30px', height: '30px', borderRadius: '50%',
              background: settings.avatarDataUrl ? `url(${settings.avatarDataUrl}) center/cover` : (settings.avatarGradient || `linear-gradient(135deg, ${C.accent}, ${C.purple})`),
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              flexShrink: 0, fontSize: T.typography.sizeMd, fontWeight: 800, color: '#fff',
              boxShadow: `0 0 0 1px ${C.border}`,
              position: 'relative',
            }}>
              {!settings.avatarDataUrl && (settings.displayName.trim().charAt(0).toUpperCase() || 'U')}
              {isMobile && (
                <span aria-label={connHealth === 'green' ? 'Connected' : connHealth === 'yellow' ? 'Connection stale' : 'Offline'}
                  title={connHealth === 'green' ? 'Connected to backend' : connHealth === 'yellow' ? 'Waiting for backend frames (>15s silence)' : 'Backend offline — reconnecting'}
                  style={{
                    position: 'absolute', bottom: '-2px', right: '-2px',
                    width: '10px', height: '10px', borderRadius: '50%',
                    background: connHealth === 'green' ? C.green : connHealth === 'yellow' ? '#eab308' : C.red,
                    border: `2px solid ${C.bg}`,
                  }} />
              )}
            </div>
            {!isMobile && (
              <div style={{ textAlign: 'left', lineHeight: 1.15 }}>
                <div style={{ fontSize: T.typography.sizeMd, fontWeight: 700, color: C.text, maxWidth: '140px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {settings.displayName || 'Account'}
                </div>
                <div style={{
                  fontSize: '10px',
                  color: connHealth === 'green' ? C.green : connHealth === 'yellow' ? '#eab308' : C.red,
                  fontWeight: 700, letterSpacing: '0.04em', marginTop: '2px',
                  display: 'flex', alignItems: 'center', gap: '6px',
                }}
                  title={`${connHealth === 'green' ? 'Connected — frames arriving' : connHealth === 'yellow' ? 'No frames for 15s+ — backend may be stalled' : 'Disconnected — reconnecting'}${statsAgeSecs != null ? ` · stats cached ${statsAgeSecs}s ago` : ''}`}>
                  <span>{connHealth === 'green' ? 'Online' : connHealth === 'yellow' ? 'Stale' : 'Offline'}</span>
                  {statsAgeSecs != null && statsAgeSecs > 60 && (
                    <span title={`Backend stats cache ${statsAgeSecs}s old — the 60s refresh loop may be blocked`}
                      style={{ color: '#eab308', fontWeight: 600 }}>· {statsAgeSecs}s</span>
                  )}
                  {/* c2-433 / task 236: substrate fill chip — concepts (RAG
                      facts in the HDC store) + axioms (PSL constraints
                      registered). Tells the user the substrate is non-empty
                      without making them open Knowledge or Library. Hidden
                      until first poll resolves so the chip doesn't render
                      with placeholder zeros. */}
                  {substrateStats && (
                    /* c2-433 / task 249+252: substrate chip is a clickable
                       drill-down — opens the Knowledge Browser. Now also
                       carries the lifetime chat count from /api/metrics
                       so users see how much the system has handled at a
                       glance. */
                    <button
                      onClick={() => { setShowKnowledge(true); fetchKnowledge(); }}
                      title={`${substrateStats.concepts.toLocaleString()} knowledge concepts · ${substrateStats.axioms.toLocaleString()} PSL axioms · ${substrateStats.chatTotal.toLocaleString()} chats handled lifetime${hdcCache ? ` · HDC cache coverage ${(hdcCache.coverage * 100).toFixed(0)}% (${hdcCache.sample_cached}/${hdcCache.sample_size})` : ''} — click to browse`}
                      aria-label='Open Knowledge Browser'
                      style={{
                        color: C.textDim, fontWeight: 600,
                        fontFamily: T.typography.fontMono, letterSpacing: 0,
                        background: 'transparent', border: 'none', padding: 0,
                        cursor: 'pointer',
                      }}>
                      · {compactNum(substrateStats.concepts)} facts · {substrateStats.axioms} ax{substrateStats.chatTotal > 0 ? ` · ${compactNum(substrateStats.chatTotal)} chats` : ''}{hdcCache && hdcCache.sample_size > 0 ? ` · ` : ''}
                      {hdcCache && hdcCache.sample_size > 0 && (
                        <span style={{
                          color: hdcCache.coverage >= 0.8 ? C.green : hdcCache.coverage >= 0.4 ? C.yellow : C.red,
                        }}>cache {(hdcCache.coverage * 100).toFixed(0)}%</span>
                      )}
                    </button>
                  )}
                </div>
              </div>
            )}
            {!isMobile && (
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke={C.textMuted} strokeWidth="2.5" style={{ marginLeft: '2px', transform: showAccountMenu ? 'rotate(180deg)' : 'rotate(0)', transition: 'transform 0.15s' }}>
                <polyline points="6 9 12 15 18 9"/>
              </svg>
            )}
          </button>

          {/* Account dropdown */}
          {showAccountMenu && (
            <>
              <div onClick={() => setShowAccountMenu(false)}
                style={{ position: 'fixed', inset: 0, zIndex: 180 }} />
              <div style={{
                position: 'absolute', top: '100%', right: 0, marginTop: '6px',
                width: '300px', zIndex: 190,
                background: C.bgCard, border: `1px solid ${C.border}`,
                borderRadius: T.radii.xxl, padding: '10px',
                boxShadow: '0 16px 40px rgba(0,0,0,0.35)',
                animation: 'lfi-fadein 0.15s ease-out',
              }}>
                {/* Profile header */}
                <div style={{ padding: '10px', display: 'flex', gap: '10px', alignItems: 'center', borderBottom: `1px solid ${C.borderSubtle}` }}>
                  <div style={{
                    width: '44px', height: '44px', borderRadius: '50%',
                    background: settings.avatarDataUrl ? `url(${settings.avatarDataUrl}) center/cover` : (settings.avatarGradient || `linear-gradient(135deg, ${C.accent}, ${C.purple})`),
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    fontSize: '17px', fontWeight: 800, color: '#fff',
                    flexShrink: 0,
                  }}>
                    {!settings.avatarDataUrl && (settings.displayName.trim().charAt(0).toUpperCase() || 'U')}
                  </div>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <input type='text'
                      value={settings.displayName}
                      onChange={(e) => setSettings(s => ({ ...s, displayName: e.target.value.slice(0, 40) }))}
                      aria-label='Display name'
                      autoComplete='name'
                      maxLength={40}
                      style={{
                        width: '100%', background: 'transparent', border: 'none', outline: 'none',
                        fontSize: T.typography.sizeBody, fontWeight: 700, color: C.text, fontFamily: 'inherit',
                        padding: 0,
                      }} />
                    <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, marginTop: '2px' }}>
                      Local account &middot; {conversations.length} chat{conversations.length === 1 ? '' : 's'}
                    </div>
                  </div>
                </div>

                {/* Menu — kept lean: common actions only. Avatar upload and
                    account deletion live in Settings (rare / irreversible). */}
                <div style={{ padding: '6px 0', display: 'flex', flexDirection: 'column' }}>
                  {/* Quick theme toggle — one of the most-used actions */}
                  <button onClick={() => setSettings(s => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' }))}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      {settings.theme === 'dark'
                        ? <circle cx="12" cy="12" r="5"/>
                        : <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>}
                    </svg>
                    {settings.theme === 'dark' ? 'Light mode' : 'Dark mode'}
                  </button>
                  {/* New chat — common */}
                  <button onClick={() => { createNewConversation(false); setShowAccountMenu(false); }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
                    </svg>
                    New chat
                  </button>
                  {/* Clear current chat — common */}
                  <button onClick={() => { clearChat(); setShowAccountMenu(false); }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><path d="M6 6l1 14a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-14"/>
                    </svg>
                    Clear this chat
                  </button>
                  <div style={{ height: '1px', background: C.borderSubtle, margin: '6px 4px' }} />
                  {/* Settings / logs — access to rarely-used stuff */}
                  <button onClick={() => { setShowAccountMenu(false); setShowSettings(true); }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <circle cx="12" cy="12" r="3"/>
                      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                    </svg>
                    Settings
                  </button>
                  <button onClick={() => { setShowAccountMenu(false); setAdminInitialTab('logs'); setShowAdmin(true);
                      fetchChatLog(50);
                    }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>
                    </svg>
                    Activity &amp; Logs
                  </button>
                  <div style={{ height: '1px', background: C.borderSubtle, margin: '6px 4px' }} />
                  <button onClick={() => {
                      if (!confirm('Log out? Your conversations remain saved.')) return;
                      handleLogout();
                      setShowAccountMenu(false);
                    }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: T.typography.sizeMd, fontFamily: 'inherit', textAlign: 'left', borderRadius: T.radii.lg }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>
                    </svg>
                    Log out
                  </button>
                </div>
              </div>
            </>
          )}

        </div>

        {/* Middle cluster: theme toggle only. Tier/model moved to the input
            bar per 2026-04-15 — single source of truth avoids the double-
            selector "it snaps back to default" bug the user was hitting.
            2026-04-17 c0-020: mobile "Stats" + "Admin" buttons removed.
            Admin is now reached via the centered [Chat][Classroom][Admin]
            tablist (works on all viewports) and mobile telemetry still has
            the toggle in the account menu. Keeps the header minimal. */}
        <div style={{ display: 'flex', alignItems: 'center', gap: isMobile ? '6px' : '10px', order: 2, marginLeft: 'auto' }}>
          {/* Theme toggle removed — accessible via account menu, Cmd+K palette,
              and Settings → Appearance. Keeping the header slim. */}
        </div>
      </header>

      {/* ========== COMPACT RESOURCE MONITOR BAR (c0-011 #7) ==========
          One-line always-visible strip beneath the header showing CPU temp,
          RAM used/total, disk free, facts count. Gated on developerMode so
          normal users see the slimmer chat-focused UI. */}
      {settings.developerMode && (
        <div role='status' aria-label='System resources'
          style={{
            display: 'flex', alignItems: 'center', gap: '14px',
            padding: '6px 14px', background: C.bgCard,
            borderBottom: `1px solid ${C.borderSubtle}`,
            fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono,
            color: C.textMuted, flexShrink: 0, overflowX: 'auto', whiteSpace: 'nowrap',
          }}>
          <span title='CPU temperature'>
            CPU <span style={{ color: stats.cpu_temp_c > 65 ? C.red : stats.cpu_temp_c > 50 ? C.yellow : C.green, fontWeight: 700 }}>
              {stats.cpu_temp_c.toFixed(0)}°C
            </span>
          </span>
          <span title={`Used ${ramUsedFmt.value} ${ramUsedFmt.unit} of ${ramTotalFmt.value} ${ramTotalFmt.unit} total`}>
            RAM <span style={{ color: C.accent, fontWeight: 700 }}>{ramLabel} {ramUnit}</span>
          </span>
          {sysInfo.disk_free != null && sysInfo.disk_total != null && (() => {
            const dp = diskPressure(sysInfo.disk_free, sysInfo.disk_total);
            if (!dp) return null;
            const color = dp.usedPct > 90 ? C.red : dp.usedPct > 75 ? C.yellow : C.green;
            return (
              <span title={`${dp.usedPct.toFixed(0)}% used · ${dp.freeGb.toFixed(1)} GB free`}>
                DISK <span style={{ color, fontWeight: 700 }}>{dp.freeGb.toFixed(1)} GB free</span>
              </span>
            );
          })()}
          <span title='Knowledge facts'>
            FACTS <span style={{ color: C.purple, fontWeight: 700 }}>{kg.facts ? compactNum(kg.facts) : (kgLastOk ? '0' : kgLastError ? 'offline' : '…')}</span>
          </span>
          <span title='Current tier'>
            TIER <span style={{ color: tierColor(currentTier), fontWeight: 700 }}>{currentTier}</span>
          </span>
          {stats.is_throttled && (
            <span style={{ color: C.red, fontWeight: 800, textTransform: 'uppercase' }}>⚠ Throttled</span>
          )}
          {latencyMs != null && (
            <span title='Avg /api/status RTT'>
              RTT <span style={{ color: latencyMs < 100 ? C.green : latencyMs < 500 ? C.yellow : C.red, fontWeight: 700 }}>
                {Math.round(latencyMs)}ms
              </span>
            </span>
          )}
        </div>
      )}

      {/* ========== TELEMETRY PANEL (mobile/tablet, collapsible) ========== */}
      {!isDesktop && showTelemetry && (
        <div style={{
          display: 'grid', gridTemplateColumns: isTablet ? 'repeat(4, 1fr)' : 'repeat(2, 1fr)',
          gap: T.spacing.sm, padding: '12px 14px', background: C.bgCard,
          borderBottom: `1px solid ${C.border}`, flexShrink: 0,
        }}>
          {telemetryCards.map(s => renderTelemetryCard(s))}
          {stats.is_throttled && (
            <div style={{
              gridColumn: '1 / -1', padding: '10px', background: C.redBg,
              border: `1px solid ${C.redBorder}`, borderRadius: T.radii.lg,
              textAlign: 'center', fontSize: T.typography.sizeSm, fontWeight: 800, color: C.red, textTransform: 'uppercase',
            }}>Thermal Throttle Active</div>
          )}
        </div>
      )}

      {/* Admin slide panel removed — replaced by the full-screen AdminModal
          rendered above (c0-017). The `showAdmin` state now drives that modal
          on all viewports. */}

      {/* ========== TOP-NAV — c0-037 #6 / c2-330, widened c2-333 ==========
          Visible section switcher. 6 destinations: Agora / Classroom / Admin
          / Fleet / Library / Auditorium. Hotkeys ⌘1..6 cover every target;
          this just surfaces the map.
          c2-333: render on tablet too (was desktop-only). On tablet the
          kbd chips hide to save room; on mobile the whole nav hides because
          the chat-first layout needs the vertical space. Horizontal scroll
          fallback so narrow widths don't clip the last tile. */}
      {!isMobile && (
        <nav role='navigation' aria-label='Top level sections'
          style={{
            display: 'flex', alignItems: 'stretch', gap: 0,
            background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`,
            padding: `0 ${T.spacing.lg}`, flexShrink: 0,
            overflowX: 'auto',
          }}>
          {([
            { id: 'chat', label: 'Agora', mod: '1', act: () => { setActiveView('chat'); setShowAdmin(false); } },
            { id: 'classroom', label: 'Classroom', mod: '2', act: () => { setActiveView('classroom'); setShowAdmin(false); } },
            { id: 'admin', label: 'Admin', mod: '3', act: () => { setShowAdmin(true); } },
            { id: 'fleet', label: 'Fleet', mod: '4', act: () => { setActiveView('fleet'); setShowAdmin(false); } },
            { id: 'library', label: 'Library', mod: '5', act: () => { setActiveView('library'); setShowAdmin(false); } },
            { id: 'auditorium', label: 'Auditorium', mod: '6', act: () => { setActiveView('auditorium'); setShowAdmin(false); } },
          ] as const).map(item => {
            const isActive = (item.id === 'admin' ? showAdmin : (activeView === item.id && !showAdmin));
            return (
              <button key={item.id} onClick={item.act}
                aria-current={isActive ? 'page' : undefined}
                title={`${item.label} (${mod()}${item.mod})`}
                style={{
                  background: 'transparent', border: 'none', cursor: 'pointer',
                  padding: `${T.spacing.sm} ${isDesktop ? T.spacing.lg : T.spacing.md}`,
                  fontFamily: 'inherit', fontSize: T.typography.sizeMd,
                  fontWeight: T.typography.weightSemibold,
                  color: isActive ? C.accent : C.textMuted,
                  borderBottom: `2px solid ${isActive ? C.accent : 'transparent'}`,
                  marginBottom: '-1px', display: 'flex', alignItems: 'center', gap: '6px',
                  transition: `color ${T.motion.fast}, border-color ${T.motion.fast}`,
                  whiteSpace: 'nowrap', flexShrink: 0,
                }}
                onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.color = C.text; }}
                onMouseLeave={(e) => { if (!isActive) e.currentTarget.style.color = C.textMuted; }}>
                {item.label}
                {isDesktop && (
                  <kbd className='lfi-shortcut-chip' aria-hidden='true' style={{
                    fontFamily: T.typography.fontMono,
                    fontSize: '10px', color: isActive ? C.accent : C.textDim,
                    opacity: 0.7,
                  }}>{modKey(item.mod)}</kbd>
                )}
              </button>
            );
          })}
        </nav>
      )}

      {/* ========== BODY: Conversation sidebar + Chat + Right sidebar ========== */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* LEFT: Conversations sidebar (Claude.ai / ChatGPT / Gemini style) */}
        {/* Overlay backdrop for mobile/tablet — click to close */}
        {!isDesktop && showConvoSidebar && (
          <div onClick={() => setShowConvoSidebar(false)}
            style={{
              position: 'fixed', inset: 0, zIndex: 90,
              background: 'rgba(0,0,0,0.45)',
              animation: 'lfi-fadein 0.18s ease-out',
            }} />
        )}
        {/* Sidebar — full height, slides with a width animation on desktop and
            a transform+overlay on mobile. REGRESSION-GUARD: prior setup used
            both translateX AND negative margin, which caused layout jitter and
            a "small/can't hide" feel. */}
        <aside aria-label='Conversations' style={{
            alignSelf: 'stretch',           // fill parent row height
            background: C.bgCard,
            borderRight: `1px solid ${C.border}`,
            display: 'flex', flexDirection: 'column', overflow: 'hidden',
            flexShrink: 0,
            ...(isDesktop ? {
              // Desktop: collapse width (0) when hidden, 360 when open — a
              // little wider per user ask, extends all the way to the bottom.
              width: showConvoSidebar ? '360px' : '0px',
              transition: 'width 0.22s cubic-bezier(0.4, 0, 0.2, 1)',
            } : {
              // c2-414 / BIG #218 mobile: widened a bit (was 320/86vw) so
              // the action-icon row isn't cramped against the title, and
              // use dvh so the sidebar is flush with the actual viewport
              // height (100vh is wrong when the address bar is visible).
              width: 'min(340px, 92vw)',
              position: 'fixed', top: 0, bottom: 0, left: 0, zIndex: 100,
              height: '100dvh',
              transform: showConvoSidebar ? 'translateX(0)' : 'translateX(-105%)',
              transition: 'transform 0.22s cubic-bezier(0.4, 0, 0.2, 1)',
              boxShadow: showConvoSidebar ? '2px 0 24px rgba(0,0,0,0.45)' : 'none',
            }),
          }}>
            <div style={{ padding: '10px 14px', borderBottom: `1px solid ${C.borderSubtle}` }}>
              <button onClick={() => createNewConversation()}
                title={`New chat (${mod()}+N)`}
                style={{
                  width: '100%', padding: '8px 12px', marginBottom: T.spacing.sm,
                  background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                  color: C.accent, borderRadius: T.radii.lg,
                  fontSize: T.typography.sizeMd, fontWeight: 700, cursor: 'pointer',
                  fontFamily: 'inherit', display: 'flex',
                  alignItems: 'center', justifyContent: 'center', gap: '6px',
                }}>
                <span style={{ fontSize: T.typography.sizeBody }}>{'\u002B'}</span> New chat
                {/* c2-264: shortcut hint on the primary sidebar CTA. kbd
                    chip uses the same muted-border styling as the Command
                    Palette item shortcuts so the language is consistent.
                    c2-412: hidden on mobile — no physical keyboard to
                    Ctrl+N with, so the chip was just noise. */}
                {!isMobile && (
                  <kbd aria-hidden='true' style={{
                    marginLeft: 'auto', fontFamily: T.typography.fontMono,
                    fontSize: '10px', color: C.accent, opacity: 0.7,
                    border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm,
                    padding: '1px 5px', letterSpacing: '0.02em',
                  }}>{modKey('N')}</kbd>
                )}
              </button>
              {/* c2-402 / task 199: today's user-message count across all
                  conversations. Muted pill under the CTA; hidden when
                  count is 0 so first-boot users don't see a silent 0. */}
              {(() => {
                const startOfToday = new Date();
                startOfToday.setHours(0, 0, 0, 0);
                const cutoff = startOfToday.getTime();
                let todayCount = 0;
                for (const c of conversations) {
                  for (const m of c.messages) {
                    if (m.role === 'user' && m.timestamp >= cutoff) todayCount++;
                  }
                }
                if (todayCount === 0) return null;
                return (
                  <div title={`${todayCount} message${todayCount === 1 ? '' : 's'} sent today across all conversations`}
                    style={{
                      fontSize: T.typography.sizeXs, color: C.textDim,
                      textAlign: 'center', marginBottom: T.spacing.sm,
                      fontFamily: T.typography.fontMono,
                    }}>
                    {todayCount} sent today
                  </div>
                );
              })()}
              <input
                type='search'
                aria-label='Search conversations'
                autoComplete='off'
                autoCorrect='off'
                spellCheck={false}
                value={convoSearch}
                onChange={(e) => setConvoSearch(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Escape' && convoSearch) { e.preventDefault(); setConvoSearch(''); } }}
                placeholder='Search conversations...'
                style={{
                  width: '100%', padding: '8px 10px',
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  borderRadius: T.radii.lg, outline: 'none',
                  color: C.text, fontFamily: 'inherit', fontSize: T.typography.sizeSm,
                  boxSizing: 'border-box',
                }}
                onFocus={(e) => e.currentTarget.style.borderColor = C.accent}
                onBlur={(e) => e.currentTarget.style.borderColor = C.borderSubtle}
              />
              {/* c2-420 / task 193: date-range chips. Pinned rows ignore
                  the filter so they stay visible at the top regardless.
                  'All' is the default. */}
              <div role='radiogroup' aria-label='Filter conversations by date range'
                style={{ display: 'flex', gap: '4px', marginTop: T.spacing.sm }}>
                {([
                  { id: 'all' as const,   label: 'All' },
                  { id: 'today' as const, label: 'Today' },
                  { id: 'week' as const,  label: 'Week' },
                  { id: 'month' as const, label: 'Month' },
                ]).map(c => {
                  const active = c.id === convoDateFilter;
                  return (
                    <button key={c.id} onClick={() => setConvoDateFilter(c.id)}
                      role='radio' aria-checked={active}
                      style={{
                        flex: 1, padding: '5px 0',
                        fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                        background: active ? C.accentBg : 'transparent',
                        border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                        color: active ? C.accent : C.textMuted,
                        borderRadius: T.radii.sm, cursor: 'pointer',
                        fontFamily: 'inherit',
                      }}>{c.label}</button>
                  );
                })}
              </div>
            </div>
            {/* c2-388 / BIG #179: switched from a flat scroll container to a
                flex column hosting GroupedVirtuoso for the main list and a
                natural-height archived section below. Virtuoso owns the
                scroll within its bounded flex:1 slot; archived section sits
                below with its own static height. The outer container keeps
                data-convo-scroller so navigateConvoRow's querySelectorAll
                still works when arrowing between focused rows. */}
            <div data-convo-scroller='true' style={{
              flex: 1, display: 'flex', flexDirection: 'column',
              minHeight: 0,  // lets Virtuoso's flex:1 child compute height
              padding: '8px',
            }}>
              {conversations.length === 0 && (
                <div style={{ padding: T.spacing.lg, textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeSm }}>
                  No conversations yet.
                </div>
              )}
              {(() => {
                // c2-420 / task 193: date-range cutoff. Pinned rows bypass
                // the filter (user manually promoted them). 'all' means no
                // cutoff.
                const dateCutoff = (() => {
                  const now = Date.now();
                  if (convoDateFilter === 'today') {
                    const d = new Date(); d.setHours(0, 0, 0, 0);
                    return d.getTime();
                  }
                  if (convoDateFilter === 'week') return now - 7 * 86400_000;
                  if (convoDateFilter === 'month') return now - 30 * 86400_000;
                  return 0;
                })();
                const filtered = conversations
                  .filter(c => {
                    if (c.archived && !showArchived) return false;
                    if (c.archived && showArchived) return false;
                    // c2-420: date filter — pinned always visible.
                    if (!c.pinned && dateCutoff > 0 && c.updatedAt < dateCutoff) return false;
                    if (!deferredConvoSearch.trim()) return true;
                    const q = deferredConvoSearch.toLowerCase();
                    if (c.title.toLowerCase().includes(q)) return true;
                    return c.messages.some(m => m.content.toLowerCase().includes(q));
                  })
                  .sort((a, b) => {
                    // Pinned first; within pinned prefer manual pinOrder,
                    // fall back to updatedAt desc.
                    if (!!a.pinned !== !!b.pinned) return a.pinned ? -1 : 1;
                    if (a.pinned && b.pinned) {
                      const ao = typeof a.pinOrder === 'number' ? a.pinOrder : Number.MAX_SAFE_INTEGER;
                      const bo = typeof b.pinOrder === 'number' ? b.pinOrder : Number.MAX_SAFE_INTEGER;
                      if (ao !== bo) return ao - bo;
                    }
                    return b.updatedAt - a.updatedAt;
                  });
                // c2-388 / BIG #179: build groups array so GroupedVirtuoso
                // can render each as a contiguous run with a sticky header.
                // Pinned first, then Starred (c2-409 / task 208), then each
                // day-bucket. flatItems is the full list in render order so
                // itemContent can index straight into it.
                //
                // #184 branch tree: child branches (conversations whose
                // branchedFrom.convoId points at a parent in the list) are
                // rendered inline directly underneath their parent with
                // depth=1, in the SAME group as the parent — even if the
                // child's updatedAt lands in a different day-bucket. This
                // keeps the fork visually adjacent to its source.
                type FlatItem = { convo: Conversation; depth: number };
                type Group = { label: string; items: FlatItem[] };
                // Build parent → children map once. Only count children that
                // are themselves in `filtered` so search filtering stays
                // honest.
                const filteredIds = new Set(filtered.map(c => c.id));
                const childrenMap = new Map<string, Conversation[]>();
                for (const c of filtered) {
                  const parent = c.branchedFrom?.convoId;
                  if (!parent || !filteredIds.has(parent)) continue;
                  const arr = childrenMap.get(parent) || [];
                  arr.push(c);
                  childrenMap.set(parent, arr);
                }
                // Recursive push so a branch-of-a-branch also shows up.
                const pushWithBranches = (c: Conversation, depth: number, group: Group, seen: Set<string>) => {
                  if (seen.has(c.id)) return;
                  seen.add(c.id);
                  group.items.push({ convo: c, depth });
                  const kids = childrenMap.get(c.id);
                  if (!kids || kids.length === 0) return;
                  kids.sort((a, b) => a.createdAt - b.createdAt);
                  for (const k of kids) pushWithBranches(k, depth + 1, group, seen);
                };
                const placedBranchIds = new Set<string>();
                const groups: Group[] = [];
                const pinned = filtered.filter(c => c.pinned);
                if (pinned.length > 0) {
                  const g: Group = { label: 'Pinned', items: [] };
                  for (const c of pinned) pushWithBranches(c, 0, g, placedBranchIds);
                  groups.push(g);
                }
                const starred = filtered.filter(c => !c.pinned && c.starred && !placedBranchIds.has(c.id));
                if (starred.length > 0) {
                  starred.sort((a, b) => b.updatedAt - a.updatedAt);
                  const g: Group = { label: 'Starred', items: [] };
                  for (const c of starred) pushWithBranches(c, 0, g, placedBranchIds);
                  groups.push(g);
                }
                for (const c of filtered) {
                  if (c.pinned || c.starred) continue;
                  // A conversation that's already been placed as a branch child
                  // under its parent must NOT appear again in its own day bucket.
                  if (placedBranchIds.has(c.id)) continue;
                  const bucket = formatDayBucket(c.updatedAt);
                  const last = groups[groups.length - 1];
                  const group = last && last.label === bucket ? last : (() => {
                    const g: Group = { label: bucket, items: [] };
                    groups.push(g);
                    return g;
                  })();
                  pushWithBranches(c, 0, group, placedBranchIds);
                }
                const groupCounts = groups.map(g => g.items.length);
                const flatItems: FlatItem[] = groups.flatMap(g => g.items);
                diag.debug('sidebar', 'virtuoso inputs', {
                  groups: groups.length,
                  groupCounts,
                  flatItems: flatItems.length,
                  filtered: filtered.length,
                  branchedPlaced: placedBranchIds.size,
                });
                // Guard: GroupedVirtuoso crashes internally when groupCounts
                // is empty (bug seen in v4.18: indexes an item before
                // bounds-checking). Early-return an empty-state card when
                // there are no conversations to render.
                if (flatItems.length === 0) {
                  diag.info('sidebar', 'empty state (no conversations)');
                  return (
                    <div style={{
                      padding: `${T.spacing.xl} ${T.spacing.md}`, textAlign: 'center',
                      color: C.textMuted, fontSize: T.typography.sizeSm, lineHeight: T.typography.lineNormal,
                    }}>
                      {deferredConvoSearch.trim()
                        ? <>No matches for <strong style={{ color: C.text }}>"{convoSearch.length > 30 ? convoSearch.slice(0, 30) + '\u2026' : convoSearch}"</strong></>
                        : <>No conversations yet. Start a new one with <kbd style={{ padding: '1px 5px', background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: 3, fontFamily: T.typography.fontMono, fontSize: '10px' }}>⌘N</kbd>.</>}
                      {deferredConvoSearch.trim() && (
                        <div style={{ marginTop: T.spacing.sm }}>
                          <button onClick={() => setConvoSearch('')}
                            style={{
                              padding: `${T.spacing.xs} ${T.spacing.md}`,
                              background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                              color: C.textSecondary, borderRadius: T.radii.sm,
                              fontSize: T.typography.sizeXs, cursor: 'pointer', fontFamily: 'inherit',
                            }}>Clear search</button>
                        </div>
                      )}
                    </div>
                  );
                }
                if (filtered.length === 0 && deferredConvoSearch.trim()) {
                  return (
                    <div style={{
                      padding: `${T.spacing.xl} ${T.spacing.md}`, textAlign: 'center',
                      color: C.textMuted, fontSize: T.typography.sizeSm, lineHeight: T.typography.lineNormal,
                    }}>
                      <div>No matches for <strong style={{ color: C.text }}>"{convoSearch.length > 30 ? convoSearch.slice(0, 30) + '\u2026' : convoSearch}"</strong></div>
                      <button onClick={() => setConvoSearch('')}
                        style={{
                          marginTop: T.spacing.sm, padding: `${T.spacing.xs} ${T.spacing.md}`,
                          background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                          color: C.textSecondary, borderRadius: T.radii.sm,
                          fontSize: T.typography.sizeXs, cursor: 'pointer', fontFamily: 'inherit',
                        }}>Clear search</button>
                    </div>
                  );
                }
                // c2-388 / BIG #179: group header render. Sticky is now
                // handled by GroupedVirtuoso itself (fixed-position relative
                // to the Scroller), so we drop the CSS position:sticky.
                const renderGroupHeader = (groupIndex: number) => {
                  const g = groups[groupIndex];
                  if (!g) return null;
                  return (
                    <div role='heading' aria-level={3} style={{
                      background: C.bg,
                      padding: `${T.spacing.sm} ${T.spacing.sm} ${T.spacing.xs}`,
                      fontSize: '10px', fontWeight: T.typography.weightBold,
                      color: C.textDim, textTransform: 'uppercase',
                      letterSpacing: T.typography.trackingLoose,
                      userSelect: 'none',
                    }}>{g.label} <span style={{ color: C.textMuted, fontWeight: T.typography.weightMedium }}>{'\u00B7 '}{g.convos.length}</span></div>
                  );
                };
                // c2-388: row render extracted so GroupedVirtuoso's
                // itemContent can map flatItems[index] → JSX without
                // inlining 150+ lines of handlers in the Virtuoso prop.
                // #184: depth param nests branch rows 14px per level with a
                // thin accent rule on the left so the fork structure is
                // visible at a glance.
                const renderConvoRow = (c: Conversation, depth: number = 0) => {
                  const isActive = c.id === currentConversationId;
                  return (
                    <div key={c.id}
                      onClick={() => setCurrentConversationId(c.id)}
                      role='button' tabIndex={0}
                      data-convo-row='true'
                      aria-label={`Open conversation: ${c.title}${c.pinned ? ' (pinned — drag to reorder)' : ''}`}
                      aria-current={isActive ? 'true' : undefined}
                      // c2-269: hover tooltip with metadata — created / last
                      // updated / message count / flags. Helps users pick
                      // between similarly-titled conversations.
                      // c2-408 / task 201: also include the last user
                      // message preview (160-char cap) so users get a
                      // content cue, not just metadata. Falls through to
                      // empty when the conversation has no user turns yet.
                      title={(() => {
                        const lastUser = [...c.messages].reverse().find(m => m.role === 'user');
                        const preview = lastUser
                          ? `\n\nLast: \u201C${lastUser.content.replace(/\s+/g, ' ').slice(0, 160)}${lastUser.content.length > 160 ? '\u2026' : ''}\u201D`
                          : '';
                        return `${c.title}\n${c.messages.length} message${c.messages.length === 1 ? '' : 's'}\nCreated: ${new Date(c.createdAt).toLocaleString()}\nUpdated: ${new Date(c.updatedAt).toLocaleString()}${c.pinned ? '\nPinned' : ''}${c.starred ? '\nStarred' : ''}${c.draft?.trim() ? '\nHas unsent draft' : ''}${preview}`;
                      })()}
                      onKeyDown={(e) => navigateConvoRow(e, c.id)}
                      draggable={!!c.pinned}
                      onDragStart={(e) => {
                        if (!c.pinned) return;
                        cd.begin(c.id);
                        try { e.dataTransfer.setData('text/plain', c.id); e.dataTransfer.effectAllowed = 'move'; } catch { /* not available in jsdom */ }
                      }}
                      onDragOver={(e) => {
                        if (!c.pinned || !draggedConvoId || draggedConvoId === c.id) return;
                        e.preventDefault();
                        try { e.dataTransfer.dropEffect = 'move'; } catch { /* */ }
                        cd.hover(c.id);
                      }}
                      onDragLeave={() => cd.leave(c.id)}
                      onDrop={(e) => {
                        if (!c.pinned || !draggedConvoId) return;
                        e.preventDefault();
                        reorderPinned(draggedConvoId, c.id);
                        cd.end();
                      }}
                      onDragEnd={() => cd.end()}
                      style={{
                        padding: '10px 12px', borderRadius: T.radii.md,
                        cursor: c.pinned ? (draggedConvoId === c.id ? 'grabbing' : 'grab') : 'pointer',
                        background: isActive ? C.accentBg : 'transparent',
                        border: `1px solid ${isActive ? C.accentBorder : 'transparent'}`,
                        marginBottom: '4px', display: 'flex',
                        alignItems: 'center', justifyContent: 'space-between', gap: '4px',
                        opacity: draggedConvoId === c.id ? 0.4 : 1,
                        // #184 tree: branch rows nest 16px per depth level with
                        // a thin accent rule along the left edge so the parent-
                        // child relationship reads at a glance.
                        marginLeft: depth > 0 ? `${depth * 16}px` : undefined,
                        borderLeft: depth > 0 ? `2px solid ${C.accent}55` : (`1px solid ${isActive ? C.accentBorder : 'transparent'}`),
                        // Insert-line hint on the drop target — 2px accent top border via
                        // inset box-shadow so it doesn't shift layout.
                        boxShadow: dragOverConvoId === c.id && draggedConvoId && draggedConvoId !== c.id
                          ? `inset 0 3px 0 0 ${C.accent}` : undefined,
                        // c0-020: smooth hover transition instead of instant snap.
                        transition: 'background-color 0.12s, border-color 0.12s, opacity 0.12s, box-shadow 0.12s',
                      }}
                      onMouseEnter={(e) => {
                        if (!isActive) e.currentTarget.style.background = C.bgHover;
                        const acts = e.currentTarget.querySelector('.convo-actions') as HTMLElement;
                        if (acts) acts.style.opacity = '1';
                      }}
                      onMouseLeave={(e) => {
                        if (!isActive) e.currentTarget.style.background = 'transparent';
                        const acts = e.currentTarget.querySelector('.convo-actions') as HTMLElement;
                        if (acts) acts.style.opacity = isActive ? '0.7' : '0';
                      }}
                    >
                      <div style={{ overflow: 'hidden', flex: 1 }}>
                        <div style={{
                          fontSize: T.typography.sizeMd, fontWeight: 600,
                          color: isActive ? C.accent : C.text,
                          whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                          display: 'flex', alignItems: 'center', gap: '6px',
                        }}>
                          {c.pinned && <span style={{ color: C.yellow, fontSize: T.typography.sizeXs }}>{'\u{1F4CC}'}</span>}
                          {c.starred && <span style={{ color: C.yellow, fontSize: T.typography.sizeXs }}>{'\u2605'}</span>}
                          {/* #176: branch marker. Hover shows parent title so
                              the user knows which convo this forked from. */}
                          {c.branchedFrom && (() => {
                            const parent = conversations.find(p => p.id === c.branchedFrom!.convoId);
                            const parentTitle = parent?.title || 'removed';
                            return (
                              <span title={`Branched from "${parentTitle}"`} aria-label={`Branched from ${parentTitle}`}
                                style={{ color: C.accent, fontSize: T.typography.sizeXs, flexShrink: 0 }}>↪</span>
                            );
                          })()}
                          {/* c2-245 / #106: unsent draft indicator. Hidden on
                              the active row since the textarea is the source
                              of truth there (c.draft may be stale). */}
                          {!isActive && c.draft && c.draft.trim().length > 0 && (
                            <span title='Unsent draft' aria-label='Has unsent draft'
                              style={{
                                display: 'inline-block', width: '7px', height: '7px',
                                borderRadius: '50%', background: C.accent,
                                flexShrink: 0,
                              }} />
                          )}
                          {renamingConvoId === c.id ? (
                            <input autoFocus type='text'
                              value={renameDraft}
                              onClick={(e) => e.stopPropagation()}
                              onChange={(e) => setRenameDraft(e.target.value)}
                              onBlur={() => {
                                const v = renameDraft.trim();
                                if (v && v !== c.title) renameConversation(c.id, v);
                                setRenamingConvoId(null);
                              }}
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') { e.preventDefault(); (e.currentTarget as HTMLInputElement).blur(); }
                                else if (e.key === 'Escape') { e.preventDefault(); setRenamingConvoId(null); }
                              }}
                              aria-label={`Rename ${c.title}`}
                              maxLength={80}
                              style={{
                                flex: 1, minWidth: 0,
                                background: C.bgInput, border: `1px solid ${C.accentBorder}`,
                                borderRadius: T.radii.sm, color: C.text, padding: '2px 6px',
                                fontSize: T.typography.sizeMd, fontFamily: 'inherit', outline: 'none',
                              }} />
                          ) : (
                            <span dir='auto'
                              style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}
                              onDoubleClick={(e) => {
                                // c0-020 polish: double-click the title to inline-rename.
                                e.stopPropagation();
                                setRenamingConvoId(c.id);
                                setRenameDraft(c.title);
                              }}>{highlightConvoTitle(c.title)}</span>
                          )}
                        </div>
                        <div style={{ fontSize: '10px', color: C.textDim, marginTop: '2px' }}>
                          {c.messages.length} msg &middot; {formatRelative(c.updatedAt)}
                        </div>
                        {/* c2-433 / task 251 + 258: body-match snippet. When
                            the search query landed in a message body (not the
                            title), show ±20 chars around the first match
                            with the match wrapped in <mark> for consistency
                            with the title highlighter. Title matches are
                            already highlighted by highlightConvoTitle. */}
                        {(() => {
                          const q = deferredConvoSearch.trim();
                          if (!q) return null;
                          if (c.title.toLowerCase().includes(q.toLowerCase())) return null;
                          const ql = q.toLowerCase();
                          for (const m of c.messages) {
                            const idx = m.content.toLowerCase().indexOf(ql);
                            if (idx < 0) continue;
                            const start = Math.max(0, idx - 20);
                            const end = Math.min(m.content.length, idx + q.length + 30);
                            const before = m.content.slice(start, idx).replace(/\s+/g, ' ');
                            const matched = m.content.slice(idx, idx + q.length);
                            const after = m.content.slice(idx + q.length, end).replace(/\s+/g, ' ');
                            return (
                              <div style={{
                                fontSize: '10px', color: C.textMuted, marginTop: '2px',
                                fontStyle: 'italic',
                                overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                              }}>
                                {start > 0 ? '\u2026' : ''}{before}
                                <mark style={{ background: 'rgba(255,211,107,0.45)', color: 'inherit', padding: '0 1px', borderRadius: T.radii.xs }}>{matched}</mark>
                                {after}{end < m.content.length ? '\u2026' : ''}
                              </div>
                            );
                          }
                          return null;
                        })()}
                      </div>
                      {/* Action icons — hover-only per design review. Uses
                          CSS class toggled by the parent's onMouseEnter/Leave.
                          Star stays visible when active for discoverability.
                          c2-414 / BIG #218 mobile: always visible on touch —
                          no hover to reveal them, and the context-menu path
                          is too hidden as a primary affordance. */}
                      <div className='convo-actions'
                        style={{
                          display: 'flex', gap: isMobile ? '4px' : '2px',
                          opacity: isMobile ? 0.7 : (isActive ? 0.7 : 0),
                          transition: 'opacity 0.12s',
                        }}>
                        {/* c2-433 / task 257: convo-row action buttons. Mobile
                            tap-target was ~14px (well under Bible §6.1 44px
                            minimum). Bumped padding + min-width on mobile so
                            the touchable area is at least ~30x32px (still
                            tight, but balanced against row height + density).
                            Desktop unchanged — hover still works fine at small
                            target sizes with mouse precision. */}
                        <button onClick={(e) => { e.stopPropagation(); toggleStarred(c.id); }}
                          title={c.starred ? 'Unstar' : 'Star'}
                          aria-label={c.starred ? `Unstar ${c.title}` : `Star ${c.title}`}
                          style={{
                            background: 'transparent', border: 'none',
                            color: c.starred ? C.yellow : C.textDim,
                            cursor: 'pointer',
                            fontSize: isMobile ? T.typography.sizeMd : T.typography.sizeSm,
                            padding: isMobile ? '6px 8px' : '2px 3px',
                            minWidth: isMobile ? '32px' : 'auto',
                            minHeight: isMobile ? '30px' : 'auto',
                          }}>{c.starred ? '\u2605' : '\u2606'}</button>
                        <button onClick={(e) => { e.stopPropagation(); togglePinned(c.id); }}
                          title={c.pinned ? 'Unpin' : 'Pin'}
                          aria-label={c.pinned ? `Unpin ${c.title}` : `Pin ${c.title}`}
                          style={{
                            background: 'transparent', border: 'none',
                            color: c.pinned ? C.yellow : C.textDim,
                            cursor: 'pointer',
                            fontSize: isMobile ? T.typography.sizeMd : T.typography.sizeXs,
                            padding: isMobile ? '6px 8px' : '2px 3px',
                            minWidth: isMobile ? '32px' : 'auto',
                            minHeight: isMobile ? '30px' : 'auto',
                          }}>{'\u{1F4CC}'}</button>
                        {/* c2-414 / BIG #218 mobile: rename + export are
                            hidden on touch to keep the row compact. Users
                            still have double-tap-title for rename and the
                            /export-txt + Admin → Export-all paths for
                            export. Star/pin/archive/delete stay — they're
                            the frequent actions. */}
                        {!isMobile && (
                          <>
                            <button onClick={(e) => {
                              e.stopPropagation();
                              setRenamingConvoId(c.id);
                              setRenameDraft(c.title);
                            }} title='Rename (or double-click title)' aria-label={`Rename ${c.title}`}
                              style={{
                                background: 'transparent', border: 'none', color: C.textDim,
                                cursor: 'pointer', fontSize: '10px', padding: '2px 3px',
                              }}>{'\u270E'}</button>
                            <button onClick={(e) => {
                              e.stopPropagation();
                              // Shift-click exports PDF; plain click exports md.
                              // Keeps the action bar compact without adding a
                              // third button, discoverable via the tooltip.
                              if (e.shiftKey) {
                                exportConversationPdf(c);
                                logEvent('conversation_exported_pdf', { id: c.id });
                              } else {
                                exportConversationMd(c);
                                logEvent('conversation_exported_md', { id: c.id });
                              }
                            }} title='Export as Markdown (Shift-click: PDF)' aria-label={`Export ${c.title} as Markdown, Shift-click for PDF`}
                              style={{
                                background: 'transparent', border: 'none', color: C.textDim,
                                cursor: 'pointer', fontSize: '10px', padding: '2px 3px',
                              }}>{'\u2B07'}</button>
                          </>
                        )}
                        <button onClick={(e) => { e.stopPropagation(); toggleArchived(c.id); }}
                          title={c.archived ? 'Unarchive' : 'Archive'}
                          aria-label={c.archived ? `Unarchive ${c.title}` : `Archive ${c.title}`}
                          style={{
                            background: 'transparent', border: 'none',
                            color: c.archived ? C.accent : C.textDim,
                            cursor: 'pointer',
                            fontSize: isMobile ? T.typography.sizeMd : T.typography.sizeXs,
                            padding: isMobile ? '6px 8px' : '2px 3px',
                            minWidth: isMobile ? '32px' : 'auto',
                            minHeight: isMobile ? '30px' : 'auto',
                          }}>{'\u{1F5C3}'}</button>
                        <button onClick={(e) => {
                          e.stopPropagation();
                          // Soft-delete — Undo in the resulting toast restores.
                          deleteConversation(c.id);
                        }} title='Delete' aria-label={`Delete ${c.title}`}
                          style={{
                            background: 'transparent', border: 'none', color: C.textDim,
                            cursor: 'pointer',
                            fontSize: isMobile ? T.typography.sizeMd : T.typography.sizeXs,
                            padding: isMobile ? '6px 8px' : '2px 3px',
                            minWidth: isMobile ? '32px' : 'auto',
                            minHeight: isMobile ? '30px' : 'auto',
                          }}>{'\u2715'}</button>
                      </div>
                    </div>
                  );
                };
                // c2-388: GroupedVirtuoso renders only visible rows + their
                // headers. style height:100% so it fills the flex:1 slot
                // of data-convo-scroller. overscan keeps drag-reorder smooth
                // by pre-rendering ±3 rows above/below the viewport.
                return (
                  <div style={{ flex: 1, minHeight: 0 }}>
                    <GroupedVirtuoso
                      style={{ height: '100%' }}
                      groupCounts={groupCounts}
                      groupContent={renderGroupHeader}
                      itemContent={(index) => {
                        const item = flatItems[index];
                        if (!item) return null;
                        return renderConvoRow(item.convo, item.depth);
                      }}
                      // Stable keys so re-renders don't remount the drag
                      // source mid-drop. group${i} and ${id} never collide.
                      computeItemKey={(index) => flatItems[index]?.convo.id ?? `i-${index}`}
                      increaseViewportBy={{ top: 200, bottom: 200 }}
                    />
                  </div>
                );
              })()}
              {/* Archived section — collapsible, hidden by default. Only appears
                  when at least one conversation is archived. */}
              {conversations.some(c => c.archived) && (
                <div style={{ marginTop: '12px', borderTop: `1px solid ${C.borderSubtle}`, paddingTop: '8px' }}>
                  <button onClick={() => setShowArchived(v => !v)}
                    aria-expanded={showArchived}
                    aria-controls='lfi-archived-section'
                    style={{
                      width: '100%', textAlign: 'left', padding: '6px 8px',
                      background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.textMuted, fontSize: T.typography.sizeXs, fontWeight: 700,
                      fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: '0.08em',
                      display: 'flex', alignItems: 'center', gap: '6px',
                    }}>
                    <span style={{ transform: showArchived ? 'rotate(90deg)' : 'rotate(0)', transition: 'transform 0.15s', display: 'inline-block' }}>{'\u25B8'}</span>
                    Archived ({conversations.filter(c => c.archived).length})
                  </button>
                  {/* c2-280: group the bulk actions + archived rows in a
                      single region so the toggle button's aria-controls
                      points at something concrete. hidden attr makes screen
                      readers skip the node when collapsed. */}
                  <div id='lfi-archived-section' role='region' aria-label='Archived conversations' hidden={!showArchived}>
                  {/* c2-244 / #105: bulk actions for the archive. Only shown
                      when the section is expanded so users can see what
                      they're about to touch. Both actions confirm first. */}
                  {showArchived && conversations.filter(c => c.archived).length > 0 && (
                    <div style={{
                      display: 'flex', gap: T.spacing.xs,
                      padding: `${T.spacing.xs} ${T.spacing.sm}`,
                      marginBottom: T.spacing.xs,
                    }}>
                      <button onClick={() => {
                        const archivedIds = conversations.filter(c => c.archived).map(c => c.id);
                        if (archivedIds.length === 0) return;
                        if (!confirm(`Unarchive ${archivedIds.length} conversation${archivedIds.length === 1 ? '' : 's'}?`)) return;
                        setConversations(prev => prev.map(c => c.archived ? { ...c, archived: false } : c));
                        logEvent('bulk_unarchive', { count: archivedIds.length });
                        showToast(`Unarchived ${archivedIds.length}`);
                      }}
                        style={{
                          flex: 1, padding: `${T.spacing.xs} ${T.spacing.sm}`,
                          background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                          color: C.accent, borderRadius: T.radii.sm, cursor: 'pointer',
                          fontFamily: 'inherit', fontSize: '10px',
                          fontWeight: T.typography.weightBold, textTransform: 'uppercase',
                          letterSpacing: '0.06em',
                        }}>Unarchive all</button>
                      <button onClick={() => {
                        const victims = conversations.filter(c => c.archived);
                        if (victims.length === 0) return;
                        if (!confirm(`Permanently delete ${victims.length} archived conversation${victims.length === 1 ? '' : 's'}?\n\nThis cannot be undone.`)) return;
                        const victimIds = new Set(victims.map(c => c.id));
                        setConversations(prev => prev.filter(c => !victimIds.has(c.id)));
                        if (currentConversationId && victimIds.has(currentConversationId)) {
                          setCurrentConversationId(null);
                        }
                        logEvent('bulk_delete_archived', { count: victims.length });
                        showToast(`Deleted ${victims.length} archived`);
                      }}
                        style={{
                          flex: 1, padding: `${T.spacing.xs} ${T.spacing.sm}`,
                          background: 'transparent', border: `1px solid ${C.redBorder}`,
                          color: C.red, borderRadius: T.radii.sm, cursor: 'pointer',
                          fontFamily: 'inherit', fontSize: '10px',
                          fontWeight: T.typography.weightBold, textTransform: 'uppercase',
                          letterSpacing: '0.06em',
                        }}>Delete all</button>
                    </div>
                  )}
                  {showArchived && conversations
                    .filter(c => c.archived)
                    .sort((a, b) => b.updatedAt - a.updatedAt)
                    .map(c => {
                      const isActive = c.id === currentConversationId;
                      return (
                        <div key={c.id} onClick={() => setCurrentConversationId(c.id)}
                          role='button' tabIndex={0}
                          data-convo-row='true'
                          aria-label={`Open archived conversation: ${c.title}`}
                          aria-current={isActive ? 'true' : undefined}
                          onKeyDown={(e) => navigateConvoRow(e, c.id)}
                          style={{
                            padding: '8px 12px', borderRadius: T.radii.lg, cursor: 'pointer',
                            background: isActive ? C.accentBg : 'transparent',
                            marginBottom: '2px', display: 'flex',
                            alignItems: 'center', justifyContent: 'space-between', gap: '4px',
                            opacity: 0.7,
                          }}>
                          <div style={{ overflow: 'hidden', flex: 1 }}>
                            <div style={{ fontSize: T.typography.sizeSm, color: C.textSecondary, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                              {c.title}
                            </div>
                          </div>
                          <button onClick={(e) => { e.stopPropagation(); toggleArchived(c.id); }}
                            title='Unarchive' aria-label={`Unarchive ${c.title}`}
                            style={{
                              background: 'transparent', border: 'none', color: C.accent,
                              cursor: 'pointer', fontSize: T.typography.sizeXs, padding: '2px 3px',
                            }}>{'\u21A9'}</button>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
            {/* Sidebar footer — minimal by default. Telemetry + host info
                only surface when Developer Mode is on, per 2026-04-15 design
                review (avoid "internal tool" vibes for general users). */}
            <div style={{
              padding: '12px', borderTop: `1px solid ${C.borderSubtle}`, fontSize: T.typography.sizeXs,
            }}>
              {settings.developerMode && (
                <>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: T.spacing.sm }}>
                    {telemetryCards.map(card => (
                      <div key={card.label} style={{
                        padding: '8px 10px', borderRadius: T.radii.lg,
                        background: card.bg, border: `1px solid ${card.border}`,
                      }}>
                        <div style={{ fontSize: '9px', color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em' }}>{card.label}</div>
                        <div style={{ fontSize: '15px', fontWeight: 800, color: card.color, marginTop: '2px' }}>
                          {card.value}<span style={{ fontSize: '10px', color: C.textDim, marginLeft: '2px' }}>{card.unit}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                  {(sysInfo.hostname || sysInfo.os) && (
                    <div title={`${sysInfo.os || ''} \u00B7 ${sysInfo.cpu_count || '?'} cores`}
                      style={{
                        marginTop: '8px', paddingTop: '8px',
                        borderTop: `1px solid ${C.borderSubtle}`,
                        fontSize: '10px', color: C.textDim, textAlign: 'center',
                        whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                      }}>
                      <span style={{ color: C.textMuted }}>{sysInfo.hostname || 'local'}</span>
                      {sysInfo.os && <span> &middot; {sysInfo.os.replace(' GNU/Linux Rolling', '').replace(' GNU/Linux', '')}</span>}
                    </div>
                  )}
                </>
              )}
              <div style={{
                marginTop: settings.developerMode ? '10px' : 0,
                display: 'flex', justifyContent: 'space-between',
                color: C.textDim, fontSize: '10px',
              }}>
                <span>{conversations.length} conversation{conversations.length === 1 ? '' : 's'}</span>
                <span style={{ color: isConnected ? C.green : C.red, fontWeight: 600 }}>
                  {isConnected ? '\u25CF Online' : '\u25CB Offline'}
                </span>
              </div>
              {/* #351 polish: Help button above the Settings gear. Single-
                  click opens the Admin Docs tab which renders the bundled
                  USER_GUIDE.md. Most discoverable path to the training
                  guide. */}
              <button onClick={() => { setAdminInitialTab('docs'); setShowAdmin(true); }}
                data-tour='help-button'
                title='User guide — hands-on training & troubleshooting'
                aria-label='Open user guide'
                style={{
                  width: '100%', marginTop: '10px', padding: '8px 10px',
                  display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                  background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textSecondary, borderRadius: T.radii.md,
                  cursor: 'pointer', fontFamily: 'inherit', fontSize: T.typography.sizeSm,
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; e.currentTarget.style.color = C.text; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = C.textSecondary; }}>
                <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true'>
                  <circle cx='12' cy='12' r='10' />
                  <path d='M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3' />
                  <line x1='12' y1='17' x2='12.01' y2='17' />
                </svg>
                Help &amp; guide
              </button>
              {/* c0-020 sidebar contract: settings gear at the bottom. Links
                  to the same Settings modal the header account menu opens. */}
              <button onClick={() => setShowSettings(true)}
                title='Settings' aria-label='Open settings'
                style={{
                  width: '100%', marginTop: '6px', padding: '8px 10px',
                  display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                  background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textSecondary, borderRadius: T.radii.md,
                  cursor: 'pointer', fontFamily: 'inherit', fontSize: T.typography.sizeSm,
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; e.currentTarget.style.color = C.text; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = C.textSecondary; }}>
                <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true'>
                  <circle cx='12' cy='12' r='3'/>
                  <path d='M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z'/>
                </svg>
                Settings
              </button>
            </div>
          </aside>

        {/* CHAT AREA — now a flex column so the input bar lives inside main
            and centers within the *available* width (shifts with the sidebar)
            instead of the viewport.
            c0-027: hidden when the Classroom view is active. Kept mounted
            with display:none so chat state (scroll pos, streaming messages)
            survives the view switch without re-render. */}
        {activeView === 'classroom' && (
          <React.Suspense fallback={
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.textMuted }}>
              Loading classroom…
            </div>
          }>
            {/* Local boundary — Classroom is data-heavy and renders third-
                party-shaped JSON from /api/admin/dashboard. A malformed field
                should scope to the Classroom pane, not the whole app. */}
            <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent} inlineMode label="ClassroomView">
              <ClassroomView C={C} host={host} isDesktop={isDesktop} localEvents={localEvents} onOpenFactKey={openFactKey} initialSub={classroomInitialSub} />
            </AppErrorBoundary>
          </React.Suspense>
        )}
        {/* c0-037 #2 / c2-328: Fleet is a full-page view like Classroom.
            Scoped in its own ErrorBoundary because orchestrator JSON can be
            raggedly shaped during bus recovery. */}
        {activeView === 'fleet' && (
          <React.Suspense fallback={
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.textMuted }}>
              Loading fleet…
            </div>
          }>
            <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent} inlineMode label="FleetView">
              <FleetView C={C} host={host} isDesktop={isDesktop} />
            </AppErrorBoundary>
          </React.Suspense>
        )}
        {/* c0-037 #3 / c2-329: Library full-page view. Same pattern — lazy +
            scoped boundary since /api/library/sources can vary across
            rollouts (name vs url, optional vetted/trust fields). */}
        {activeView === 'library' && (
          <React.Suspense fallback={
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.textMuted }}>
              Loading library…
            </div>
          }>
            <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent} inlineMode label="LibraryView">
              <LibraryView C={C} host={host} isDesktop={isDesktop} />
            </AppErrorBoundary>
          </React.Suspense>
        )}
        {/* c0-037 #12 / c2-331: Auditorium — AVP-2 state surface.
            Hybrid live/reference: tries /api/avp/status then the admin
            variant, falls through to the static 6-tier / 36-pass reference
            view with an inline "live stats unavailable" notice. */}
        {activeView === 'auditorium' && (
          <React.Suspense fallback={
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.textMuted }}>
              Loading auditorium…
            </div>
          }>
            <AppErrorBoundary themeBg={C.bg} themeText={C.text} themeAccent={C.accent} inlineMode label="AuditoriumView">
              <AuditoriumView C={C} host={host} isDesktop={isDesktop} />
            </AppErrorBoundary>
          </React.Suspense>
        )}
        <main id='main-content' role='main' aria-label='Chat'
          aria-busy={isThinking || undefined}
          onDragOver={(e) => {
            // c2-370 / task 84: accept drops only when a file is being dragged;
            // the pinned-convo reorder path sets text/plain, we ignore that
            // here by checking for 'Files' in the types list.
            if (!e.dataTransfer?.types.includes('Files')) return;
            e.preventDefault();
            try { e.dataTransfer.dropEffect = 'copy'; } catch { /* jsdom */ }
            if (!isDraggingFile) setIsDraggingFile(true);
          }}
          onDragLeave={(e) => {
            // Only clear when the drag leaves the chat pane entirely -- the
            // relatedTarget null check avoids flicker as the cursor crosses
            // child elements.
            if (e.currentTarget.contains(e.relatedTarget as Node)) return;
            setIsDraggingFile(false);
          }}
          onDrop={(e) => {
            if (!e.dataTransfer?.files.length) return;
            e.preventDefault();
            setIsDraggingFile(false);
            const files = Array.from(e.dataTransfer.files);
            // Text-ish files get their contents slurped into the input. Hard
            // cap at 100 KB so a 50 MB log file doesn't freeze the browser.
            const TEXT_MAX = 100 * 1024;
            const isTexty = (f: File) => {
              if (f.size > TEXT_MAX) return false;
              if (f.type.startsWith('text/')) return true;
              if (/\.(md|txt|json|yaml|yml|toml|csv|log|py|rs|ts|tsx|js|jsx|sh|go|rb|java|c|cpp|h|hpp)$/i.test(f.name)) return true;
              return false;
            };
            const textFiles = files.filter(isTexty);
            const reads = textFiles.map(f => f.text().then(t => `\n\`\`\`${/\.(ts|tsx|js|jsx|py|rs|go|rb|sh|md|json|yaml|toml)$/i.exec(f.name)?.[1] || ''}\n// ${f.name}\n${t}\n\`\`\`\n`));
            Promise.all(reads).then(chunks => {
              if (chunks.length === 0) return;
              const joined = chunks.join('');
              setInput(prev => prev + joined);
              logEvent('file_drop', { count: textFiles.length, totalBytes: textFiles.reduce((s, f) => s + f.size, 0) });
              setTimeout(() => inputRef.current?.focus(), 0);
            });
            const skipped = files.length - textFiles.length;
            if (skipped > 0) {
              showToast(`${skipped} file${skipped === 1 ? '' : 's'} skipped (non-text or >100KB)`);
            }
          }}
          style={{
          flex: 1, display: activeView === 'chat' ? 'flex' : 'none', flexDirection: 'column',
          overflow: 'hidden', minWidth: 0, position: 'relative',
        }}>
          {/* c2-370 / task 84: drag-and-drop overlay. Absolute-positioned
              so it fills the chat pane without reflowing the message list.
              pointer-events: none so the drop target stays the <main> below
              (otherwise the overlay would intercept the drop). */}
          {isDraggingFile && (
            <div aria-hidden='true' style={{
              position: 'absolute', inset: 0, zIndex: T.z.overlay,
              border: `2px dashed ${C.accent}`, background: C.accentBg,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              pointerEvents: 'none', borderRadius: T.radii.lg, margin: T.spacing.md,
            }}>
              <div style={{
                fontSize: T.typography.size2xl, color: C.accent,
                fontWeight: T.typography.weightBlack,
                textTransform: 'uppercase', letterSpacing: T.typography.trackingCap,
              }}>Drop to attach</div>
            </div>
          )}
          {/* Inline message search (Cmd+Shift+F). Slides down from the top of
              main while open; clearing the input or closing restores the full
              list. Filters the messages array passed to ChatView. */}
          {showChatSearch && (() => {
            // c2-257 / #119: shared match-jumper reused by the Enter
            // keyboard handler and the ↑/↓ buttons so the cursor state
            // stays authoritative.
            const jumpMatch = (dir: 1 | -1) => {
              const q = chatSearch.toLowerCase();
              if (!q) return;
              // c2-433 / fix: in 'filter' mode the rendered list is the
              // filtered subset, so scrollToIndex needs an index INTO that
              // subset, not into the full messages array. In 'highlight'
              // mode the rendered list is the full one. Compute indices
              // against the same list ChatView is rendering.
              const renderedList = chatSearchMode === 'filter'
                ? messages.filter(m => m.content?.toLowerCase().includes(q))
                : messages;
              const indices: number[] = [];
              renderedList.forEach((m, i) => { if (m.content?.toLowerCase().includes(q)) indices.push(i); });
              if (indices.length === 0) return;
              const nextCursor = (chatSearchCursor + dir + indices.length) % indices.length;
              setChatSearchCursor(nextCursor);
              const targetIdx = indices[nextCursor];
              chatViewRef.current?.scrollToIndex(targetIdx);
              // c2-433 / task 245: shared flash helper — see util.ts.
              const targetMsg = renderedList[targetIdx];
              if (targetMsg) flashMessageById(targetMsg.id, C.yellow);
            };
            const matchCount = chatSearch
              ? messages.filter(m => m.content?.toLowerCase().includes(chatSearch.toLowerCase())).length
              : 0;
            return (
            <div role='search' style={{
              padding: T.spacing.sm + ' ' + T.spacing.lg,
              background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`,
              // c2-293: flex-wrap so prev/next/mode/close don't clip on narrow
              // mobile widths. row-gap keeps wrapped controls from touching.
              display: 'flex', alignItems: 'center', flexWrap: 'wrap',
              gap: T.spacing.sm, rowGap: T.spacing.xs,
            }}>
              <input
                ref={chatSearchInputRef}
                type='search'
                aria-label='Search this conversation'
                placeholder='Search messages… (Enter to jump to next match)'
                autoComplete='off' spellCheck={false}
                value={chatSearch}
                onChange={(e) => setChatSearch(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Escape') { cs.close(); return; }
                  // c2-256 / #118 + c2-433 fix: Enter / Shift+Enter cycles
                  // through match indices. Now active in BOTH modes — the
                  // filter-mode block was a stale assumption ("jumping is
                  // redundant"); for long convos with 30+ matches, stepping
                  // through them in order is useful even when non-matches
                  // are hidden. jumpMatch already adapts indices to the
                  // rendered list (filter or highlight).
                  if (e.key === 'Enter' && chatSearch.trim()) {
                    e.preventDefault();
                    jumpMatch(e.shiftKey ? -1 : 1);
                  }
                }}
                style={{
                  flex: 1, padding: T.spacing.sm + ' ' + T.spacing.md,
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  borderRadius: T.radii.md, color: C.text, outline: 'none',
                  fontSize: T.typography.sizeMd, fontFamily: 'inherit',
                }}
              />
              <span style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono }}>
                {!chatSearch
                  ? `${messages.length} msgs`
                  : matchCount > 0
                    ? `${Math.min(chatSearchCursor + 1, matchCount)} / ${matchCount}`
                    : `0 of ${messages.length}`}
              </span>
              {/* c2-257 / #119 + c2-433 fix: visible prev/next match buttons.
                  Now shown in both modes once Enter-jump is enabled in filter
                  mode too. */}
              {chatSearch && matchCount > 0 && (
                <>
                  <button onClick={() => jumpMatch(-1)} aria-label='Previous match' title='Previous match (Shift+Enter)'
                    style={{
                      background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                      color: C.textMuted, borderRadius: T.radii.sm, cursor: 'pointer',
                      padding: '2px 6px', fontSize: T.typography.sizeSm, lineHeight: 1,
                      fontFamily: 'inherit',
                    }}>{'\u2191'}</button>
                  <button onClick={() => jumpMatch(1)} aria-label='Next match' title='Next match (Enter)'
                    style={{
                      background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                      color: C.textMuted, borderRadius: T.radii.sm, cursor: 'pointer',
                      padding: '2px 6px', fontSize: T.typography.sizeSm, lineHeight: 1,
                      fontFamily: 'inherit',
                    }}>{'\u2193'}</button>
                </>
              )}
              {/* Mode toggle: filter (hide non-matches) vs highlight (mark
                  inline, keep everything visible). */}
              <button onClick={() => setChatSearchMode(m => m === 'filter' ? 'highlight' : 'filter')}
                aria-label={`Search mode: ${chatSearchMode}. Click to switch.`}
                title={chatSearchMode === 'filter'
                  ? 'Filter mode — only matching messages. Click to show all + highlight.'
                  : 'Highlight mode — all messages visible. Click to filter to matches only.'}
                style={{
                  padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                  background: chatSearchMode === 'filter' ? C.accentBg : 'transparent',
                  border: `1px solid ${C.borderSubtle}`,
                  color: chatSearchMode === 'filter' ? C.accent : C.textMuted,
                  borderRadius: T.radii.sm, cursor: 'pointer',
                  fontFamily: 'inherit', textTransform: 'uppercase',
                }}>{chatSearchMode}</button>
              <button onClick={() => { cs.close(); }}
                aria-label='Close search'
                style={{
                  background: 'transparent', border: 'none', color: C.textMuted,
                  cursor: 'pointer', fontSize: '18px', padding: '4px 8px',
                }}>{'\u2715'}</button>
            </div>
            );
          })()}
          {/* c2-404 / task 195: reading-progress gauge. 2px bar at the top
              of the chat pane; width = chatTopIndex / (messages.length - 1).
              Hidden while streaming (isThinking) and on short convos (<10
              messages). Pure visual — doesn't affect layout or clicks. */}
          {!isThinking && messages.length >= 10 && (() => {
            const pct = Math.max(0, Math.min(100, (chatTopIndex / Math.max(1, messages.length - 1)) * 100));
            return (
              <div aria-hidden='true'
                style={{
                  position: 'absolute', top: 0, left: 0, right: 0, height: '2px',
                  background: 'transparent', zIndex: T.z.sticky,
                  pointerEvents: 'none',
                }}>
                <div style={{
                  height: '100%', width: `${pct}%`,
                  background: C.accent, opacity: 0.6,
                  transition: 'width 0.15s linear',
                }} />
              </div>
            );
          })()}
          {/* Floating "scroll to bottom" — appears when user has scrolled
              up away from the latest message in a non-empty chat. Avoids the
              UX trap where new AI replies arrive but the user is reading
              history and never sees them. */}
          {!chatAtBottom && messages.length > 0 && (() => {
            // c2-433 / task 249b: count new messages added since the user
            // scrolled up. Clamped at 0 + capped display at 99+.
            const baseLen = scrollAwayLengthRef.current;
            const newCount = baseLen != null ? Math.max(0, messages.length - baseLen) : 0;
            const newLabel = newCount > 99 ? '99+' : String(newCount);
            return (
            <button onClick={() => {
              chatViewRef.current?.scrollToBottom();
              scrollAwayLengthRef.current = null;
            }}
              aria-label={newCount > 0 ? `Scroll to latest message (${newCount} new)` : 'Scroll to latest message'}
              title={newCount > 0 ? `${newCount} new message${newCount === 1 ? '' : 's'} below` : 'Scroll to latest'}
              style={{
                // c2-289: bump the bottom offset by the safe-area inset so the
                // FAB doesn't tuck behind the chat input on iPhones with a
                // home-indicator bar. calc() falls back cleanly on platforms
                // without env() support (gives the original 120px).
                // c2-419 / BIG #218 mobile: extra offset + right-edge inset
                // on mobile so the FAB clears the input's action row + the
                // send button cleanly.
                position: 'absolute',
                bottom: `calc(${isMobile ? '160px' : '120px'} + env(safe-area-inset-bottom, 0px))`,
                right: isMobile ? '16px' : '24px',
                width: '40px', height: '40px', borderRadius: T.radii.round,
                background: C.bgCard, border: `1px solid ${C.accentBorder}`,
                color: C.accent, cursor: 'pointer', boxShadow: T.shadows.card,
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                zIndex: T.z.sticky, fontFamily: 'inherit',
              }}>
              <svg width='18' height='18' viewBox='0 0 24 24' fill='none'
                stroke='currentColor' strokeWidth='2.5' strokeLinecap='round' strokeLinejoin='round'>
                <line x1='12' y1='5' x2='12' y2='19' />
                <polyline points='19 12 12 19 5 12' />
              </svg>
              {/* c2-433 / task 249b: +N badge top-right of the FAB. Anchored
                  with negative offsets so it spills slightly outside the
                  circle without affecting layout. Hidden when count is 0. */}
              {newCount > 0 && (
                <span aria-hidden='true' style={{
                  position: 'absolute', top: '-6px', right: '-6px',
                  minWidth: '18px', height: '18px',
                  padding: newLabel.length > 2 ? '0 4px' : 0,
                  fontSize: '10px', fontWeight: T.typography.weightBold,
                  color: '#fff', background: C.accent,
                  borderRadius: T.radii.pill, border: `2px solid ${C.bg}`,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  fontFamily: T.typography.fontMono, lineHeight: 1,
                  boxSizing: 'border-box',
                }}>{newLabel}</span>
              )}
            </button>
            );
          })()}
          {/* c2-396 / task 210: mirror of scroll-to-bottom. Appears when
              the user has scrolled past the start of a long conversation
              (chatTopIndex > 4 — anything less already shows the inline
              "Today" separator so a top jump would be redundant). */}
          {chatTopIndex > 4 && messages.length > 10 && (
            <button onClick={() => chatViewRef.current?.scrollToIndex(0)}
              aria-label='Jump to top of conversation'
              title='Jump to top'
              style={{
                position: 'absolute', top: '64px',
                right: isMobile ? '16px' : '24px',
                width: '36px', height: '36px', borderRadius: T.radii.round,
                background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                color: C.textMuted, cursor: 'pointer', boxShadow: T.shadows.cardLight,
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                zIndex: T.z.sticky, fontFamily: 'inherit',
              }}>
              <svg width='16' height='16' viewBox='0 0 24 24' fill='none'
                stroke='currentColor' strokeWidth='2.5' strokeLinecap='round' strokeLinejoin='round'>
                <line x1='12' y1='19' x2='12' y2='5' />
                <polyline points='5 12 12 5 19 12' />
              </svg>
            </button>
          )}
          {/* Floating day-header — pinned to the top of the chat pane;
              shows the day of the topmost visible message. Only rendered
              when we have >0 messages and the topmost isn't the first (so
              it doesn't duplicate the inline "Today" separator). */}
          {(() => {
            const visible = (chatSearch && chatSearchMode === 'filter') ? messages.filter(m => m.content?.toLowerCase().includes(chatSearch.toLowerCase())) : messages;
            if (visible.length === 0) return null;
            const msg = visible[Math.min(chatTopIndex, visible.length - 1)];
            if (!msg) return null;
            // Hide when at the very top — the inline separator already
            // shows the day for the first batch.
            if (chatTopIndex <= 1) return null;
            return (
              <div aria-hidden='true'
                style={{
                  position: 'absolute', top: '8px', left: '50%',
                  transform: 'translateX(-50%)',
                  zIndex: T.z.sticky, pointerEvents: 'none',
                  padding: '4px 12px', borderRadius: T.radii.pill,
                  background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                  boxShadow: T.shadows.cardLight,
                  fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                  color: C.textSecondary, textTransform: 'uppercase',
                  letterSpacing: T.typography.trackingLoose,
                  fontFamily: 'inherit',
                }}>
                {formatDayBucket(msg.timestamp)}
              </div>
            );
          })()}
          <ChatView
            ref={chatViewRef}
            messages={(chatSearch && chatSearchMode === 'filter') ? messages.filter(m => m.content?.toLowerCase().includes(chatSearch.toLowerCase())) : messages}
            chatMaxWidth={chatMaxWidth}
            chatPadding={chatPadding}
            isDesktop={isDesktop}
            onAtBottomChange={(at) => {
              // c2-433 / task 249b: when user leaves the bottom, snapshot
              // the current message count so the FAB badge can compute
              // "new since departure". When back at bottom, clear the
              // snapshot — badge hides.
              setChatAtBottom(at);
              if (at) scrollAwayLengthRef.current = null;
              else if (scrollAwayLengthRef.current == null) scrollAwayLengthRef.current = messages.length;
            }}
            onVisibleRangeChange={(start) => setChatTopIndex(start)}
            C={C}
            // c2-362 / task 78: cluster consecutive same-role messages within
            // 60 seconds so rapid turns read as one block. Threshold held as
            // a literal here (60_000 ms) since it is UX policy rather than a
            // design token.
            isGroupedWithPrevious={(curr, prev) =>
              curr.role === prev.role && Math.abs(curr.timestamp - prev.timestamp) <= 60_000
            }
            renderEmpty={() => {
              // Pick the most-recent non-empty conversation that isn't the
              // currently-active one (which is empty — that's why we're in
              // the welcome state). Surface its title + last user turn as
              // a contextual "Continue" card.
              const recent = [...conversations]
                .filter(c => c.id !== currentConversationId && c.messages.length > 0)
                .sort((a, b) => b.updatedAt - a.updatedAt)[0];
              const lastUser = recent ? [...recent.messages].reverse().find(m => m.role === 'user') : null;
              const recentContext = recent ? {
                title: recent.title,
                lastUserMsg: lastUser ? (lastUser.content.length > 80 ? lastUser.content.slice(0, 80) + '…' : lastUser.content) : undefined,
              } : null;
              return (
                <WelcomeScreen
                  C={C} isDesktop={isDesktop}
                  onPickPrompt={(p) => { setInputAndResize(p); inputRef.current?.focus(); }}
                  recentContext={recentContext}
                  substrate={substrateStats}
                />
              );
            }}
            renderFooter={() => (
              <>
                {isThinking && (() => {
                  // Color shifts as latency grows so the user knows when the
                  // run is unusually slow without interrupting them. <15s green
                  // (normal), 15-30s yellow (slow), >30s red (probably stuck).
                  const slow = thinkingElapsed >= 15 && thinkingElapsed < 30;
                  const stuck = thinkingElapsed >= 30;
                  const dotColor = stuck ? C.red : slow ? C.yellow : C.accent;
                  const borderColor = stuck ? C.redBorder : slow ? C.accentBorder : C.borderSubtle;
                  return (
                  <div role="status" aria-live="polite" style={{
                    display: 'flex', alignItems: 'center', gap: T.spacing.md,
                    padding: '12px 16px', margin: '8px 0',
                    background: C.bgCard, border: `1px solid ${borderColor}`,
                    borderRadius: T.radii.xl, fontSize: T.typography.sizeMd,
                    transition: 'border-color 0.4s', flexWrap: 'wrap',
                  }}>
                    <div style={{ display: 'flex', gap: '5px', alignItems: 'center' }}>
                      {[0, 1, 2].map(i => (
                        <div key={i} style={{
                          width: '7px', height: '7px', background: dotColor, borderRadius: '50%',
                          animation: 'scc-bounce 1.4s infinite ease-in-out',
                          animationDelay: `${i * 0.16}s`,
                          transition: 'background 0.4s',
                        }} />
                      ))}
                    </div>
                    <span style={{ color: C.text, fontWeight: 500 }}>{thinkingStep || 'Thinking'}</span>
                    <span style={{ color: stuck ? C.red : C.textDim, fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono }}>
                      {Math.floor(thinkingElapsed / 60) > 0 ? `${Math.floor(thinkingElapsed / 60)}m ` : ''}{thinkingElapsed % 60}s
                    </span>
                    {/* c2-433 / #316: cognitive-module activity bar. Six chips
                        (RAG / Active-Inference / Causal / Analogy / Procedural
                        / Metacognitive) — the active one pulses accent, ones
                        that have run this turn stay solid-on, untouched ones
                        dim. Until the backend emits cognitive_module on
                        progress events the bar reads "Idle" and the chips all
                        stay dim. Hidden on mobile (the row is already busy);
                        accessible via the activity log. */}
                    {!isMobile && (() => {
                      // c2-433 / #316: short labels for the chip strip. "AI"
                      // for Active Inference was confusing — collides with
                      // the everyday meaning of "AI"; renamed to "ActI" so
                      // users don't read the chip as "Artificial Intelligence
                      // module is active." Tooltip carries the full name.
                      const mods = [
                        { id: 'rag', label: 'RAG', long: 'Retrieval-Augmented' },
                        { id: 'active_inference', label: 'ActI', long: 'Active Inference' },
                        { id: 'causal', label: 'Caus', long: 'Causal' },
                        { id: 'analogy', label: 'Anlg', long: 'Analogy' },
                        { id: 'procedural', label: 'Proc', long: 'Procedural' },
                        { id: 'metacognitive', label: 'Meta', long: 'Metacognitive' },
                      ];
                      const norm = (s: string) => s.toLowerCase().replace(/[^a-z]/g, '');
                      const activeNorm = activeModule ? norm(activeModule) : null;
                      const usedNorm = new Set(Array.from(modulesUsed).map(norm));
                      // c2-433 / task 250: detect when the backend is
                      // emitting a module name that doesn't map to any of
                      // the 6 known IDs (substring match in either
                      // direction). Surface as a "?" chip so users see the
                      // unknown module + can hover for the raw name.
                      const matchesAnyKnown = activeNorm == null || mods.some(m =>
                        activeNorm === m.id || activeNorm.includes(m.id) || m.id.includes(activeNorm));
                      return (
                        <div style={{
                          display: 'flex', gap: '4px', alignItems: 'center',
                          marginLeft: T.spacing.sm,
                        }}
                          aria-label='Cognitive modules active this turn'
                          title={activeModule ? `Active: ${activeModule}` : 'Cognitive modules — backend will light these as substrate dispatches'}>
                          {mods.map(m => {
                            const isActive = activeNorm != null && (activeNorm === m.id || activeNorm.includes(m.id) || m.id.includes(activeNorm));
                            const wasUsed = !isActive && Array.from(usedNorm).some(u => u === m.id || u.includes(m.id) || m.id.includes(u));
                            const fg = isActive ? C.accent : wasUsed ? C.text : C.textDim;
                            const bg = isActive ? C.accentBg : wasUsed ? C.bgInput : 'transparent';
                            const border = isActive ? C.accentBorder : C.borderSubtle;
                            return (
                              <span key={m.id} title={m.long}
                                style={{
                                  fontSize: '9px', fontWeight: 700,
                                  color: fg, background: bg, border: `1px solid ${border}`,
                                  padding: '2px 5px', borderRadius: T.radii.sm,
                                  fontFamily: T.typography.fontMono,
                                  letterSpacing: '0.04em', textTransform: 'uppercase',
                                  animation: isActive ? 'scc-pulse 1.6s ease-in-out infinite' : undefined,
                                }}>{m.label}</span>
                            );
                          })}
                          {!matchesAnyKnown && activeModule && (
                            <span title={`Unknown module: ${activeModule}`}
                              style={{
                                fontSize: '9px', fontWeight: 700,
                                color: C.yellow, background: C.yellowBg,
                                border: `1px solid ${C.yellow}`,
                                padding: '2px 5px', borderRadius: T.radii.sm,
                                fontFamily: T.typography.fontMono,
                                letterSpacing: '0.04em', textTransform: 'uppercase',
                                animation: 'scc-pulse 1.6s ease-in-out infinite',
                              }}>?</span>
                          )}
                        </div>
                      );
                    })()}
                    {stuck && thinkingElapsed < 45 && (
                      <span style={{ color: C.red, fontSize: T.typography.sizeXs, fontStyle: 'italic' }}>
                        unusually slow
                      </span>
                    )}
                    {/* c2-433 / #352: topic-context chip. Surfaces when the
                        backend emits chat_progress.topic — gives users a
                        visible signal that multi-turn pronoun resolution
                        is anchored ("them"/"it" → topic). Hidden on mobile
                        (row already busy) + when no topic yet. */}
                    {!isMobile && activeTopic && (
                      <span title={`Multi-turn topic context: ${activeTopic} — pronouns will resolve to this`}
                        style={{
                          fontSize: '10px', fontWeight: 700,
                          color: C.purple, background: C.purpleBg,
                          border: `1px solid ${C.purpleBorder}`,
                          padding: '2px 8px', borderRadius: T.radii.sm,
                          fontFamily: T.typography.fontMono,
                          letterSpacing: '0.04em',
                          maxWidth: '160px', overflow: 'hidden',
                          textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                        }}>topic: {activeTopic}</span>
                    )}
                    {/* c2-430: backend-stuck guardrail. After 45s with no
                        chunks, the thinking path is almost certainly
                        backend-side — either the substrate pipeline is
                        mid-refactor (post-LLM pivot is in progress) or the
                        WS handler errored silently. Surface a clear
                        message + Reload so the user isn't stuck staring
                        at a spinner forever. */}
                    {thinkingElapsed >= 45 && (
                      <div style={{ flexBasis: '100%', marginTop: T.spacing.xs,
                        fontSize: T.typography.sizeXs, color: C.red, lineHeight: T.typography.lineTight }}>
                        Backend isn&apos;t streaming a response. Hit <strong>Stop</strong> below
                        then retry, or reload if it persists — the LFI substrate pipeline
                        may be mid-refactor per the ongoing post-LLM pivot.
                      </div>
                    )}
                    <button onClick={() => {
                      setIsThinking(false);
                      setThinkingStart(null);
                      fetch(`http://${getHost()}:3000/api/stop`, { method: 'POST' }).catch(() => {});
                      logEvent('chat_stop', { elapsed: thinkingElapsed });
                      showToast('Stopped');
                    }}
                      title='Stop (Esc)' aria-label='Stop in-flight request'
                      style={{
                      marginLeft: 'auto', padding: '4px 12px', fontSize: T.typography.sizeSm,
                      background: thinkingElapsed >= 45 ? C.redBg : 'transparent',
                      border: `1px solid ${thinkingElapsed >= 45 ? C.redBorder : C.border}`,
                      color: thinkingElapsed >= 45 ? C.red : C.textMuted,
                      borderRadius: T.radii.md, cursor: 'pointer',
                      fontFamily: 'inherit', fontWeight: thinkingElapsed >= 45 ? T.typography.weightBold : T.typography.weightRegular,
                    }}>Stop</button>
                    {thinkingElapsed >= 60 && (
                      <button onClick={() => window.location.reload()}
                        aria-label='Reload the dashboard'
                        style={{
                          padding: '4px 12px', fontSize: T.typography.sizeSm,
                          background: 'transparent',
                          border: `1px solid ${C.border}`,
                          color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer',
                          fontFamily: 'inherit',
                        }}>Reload</button>
                    )}
                  </div>
                  );
                })()}
                <div ref={messagesEndRef} />
              </>
            )}
            renderMessage={(msg, index) => (
              <div data-msg-id={msg.id}>
                {(() => {
                  // Day separator: show when this message starts a new day vs
                  // the previous visible one. Sticky positioning would require
                  // restructuring Virtuoso scroll; a static separator is enough
                  // to give users their bearings while scrolling history.
                  const prev = index > 0 ? messages[index - 1] : null;
                  const sameDay = prev && new Date(prev.timestamp).toDateString() === new Date(msg.timestamp).toDateString();
                  if (sameDay) return null;
                  return (
                    <div role='separator' aria-label={formatDayBucket(msg.timestamp)}
                      style={{
                        textAlign: 'center', margin: '12px 0 18px',
                        fontSize: T.typography.sizeXs, fontWeight: 700,
                        color: C.textMuted, textTransform: 'uppercase',
                        letterSpacing: '0.10em',
                        display: 'flex', alignItems: 'center', gap: T.spacing.md,
                      }}>
                      <span style={{ flex: 1, height: '1px', background: C.borderSubtle }} />
                      <span>{formatDayBucket(msg.timestamp)}</span>
                      <span style={{ flex: 1, height: '1px', background: C.borderSubtle }} />
                    </div>
                  );
                })()}
                {msg.role === 'system' && <SystemMessage content={msg.content} C={C} />}
                {msg.role === 'web' && <WebMessage content={msg.content} C={C} isDesktop={isDesktop} />}
                {msg.role === 'tool' && (
                  <ToolMessage
                    msg={msg} C={C} isDesktop={isDesktop}
                    expanded={expandedTools.has(msg.id)}
                    onToggle={() => setExpandedTools(prev => {
                      const next = new Set(prev);
                      if (next.has(msg.id)) next.delete(msg.id); else next.add(msg.id);
                      return next;
                    })}
                  />
                )}
                {msg.role === 'user' && (msg as any)._queued && (
                  /* c2-382 / BIG #177: offline queue badge. Sits above the
                     user bubble so the regular edit/copy actions don't shift. */
                  <div style={{
                    display: 'flex', justifyContent: 'flex-end',
                    marginBottom: '4px',
                  }}>
                    <span aria-live='polite' style={{
                      fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                      color: C.yellow, background: C.yellowBg,
                      border: `1px solid ${C.yellow}`, borderRadius: T.radii.sm,
                      padding: `2px ${T.spacing.sm}`,
                      textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                    }}>Queued (offline)</span>
                  </div>
                )}
                {msg.role === 'user' && (msg as any)._branchedFromId != null && (() => {
                  /* c2-387 / BIG #176: branch-from indicator. Same header-
                     slot shape as the Queued badge; purple palette so
                     branching reads as a neutral navigation cue rather
                     than a warning or error.
                     c2-433 / task 241: now clickable — tap to scroll to the
                     parent message + flash a yellow highlight. The parent
                     id may have been deleted from the convo (edit-and-
                     resend trims the trailing slice), in which case the
                     badge stays inert (button disabled, tooltip explains). */
                  const parentId = (msg as any)._branchedFromId as number;
                  const parentIdx = messages.findIndex(m => m.id === parentId);
                  const found = parentIdx >= 0;
                  return (
                  <div style={{
                    display: 'flex', justifyContent: 'flex-end',
                    marginBottom: '4px',
                  }}>
                    <button onClick={() => {
                      if (!found) return;
                      chatViewRef.current?.scrollToIndex(parentIdx);
                      // c2-433 / task 245: shared flash helper — handles
                      // the 80ms-then-flash dance + the data-msg-id query.
                      flashMessageById(parentId, C.yellow);
                    }}
                      disabled={!found}
                      title={found ? `Branched from message #${parentId} — click to jump` : `Parent message #${parentId} no longer in this conversation`}
                      aria-label={found ? `Jump to parent message ${parentId}` : `Parent message ${parentId} unavailable`}
                      style={{
                        fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                        color: found ? C.purple : C.textDim,
                        background: found ? C.purpleBg : 'transparent',
                        border: `1px solid ${found ? C.purpleBorder : C.borderSubtle}`,
                        borderRadius: T.radii.sm,
                        padding: `2px ${T.spacing.sm}`,
                        textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                        fontFamily: 'inherit',
                        cursor: found ? 'pointer' : 'not-allowed',
                      }}>Branch</button>
                  </div>
                  );
                })()}
                {msg.role === 'user' && (
                  <UserMessage
                    msg={msg} C={C} isMobile={isMobile}
                    maxWidth={userBubbleMaxWidth}
                    editing={editingMsgId === msg.id}
                    editText={editText} setEditText={setEditText}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      setMsgContextMenu({ x: e.clientX, y: e.clientY, msgId: msg.id, role: 'user', content: msg.content });
                    }}
                    onBeginEdit={() => me.begin(msg.id, msg.content)}
                    onCancelEdit={() => me.cancel()}
                    onCommitEdit={(trimmed) => {
                      const idx = messages.findIndex(m => m.id === msg.id);
                      if (idx >= 0) setMessages(prev => prev.slice(0, idx));
                      me.cancel();
                      setInputAndResize(trimmed);
                      // c2-387 / BIG #176: tell the next handleSend to mark
                      // the new bubble as a branch from the edited message.
                      pendingBranchFromRef.current = msg.id;
                      setTimeout(() => handleSend(), 50);
                      logEvent('message_edited', { originalLen: msg.content.length, newLen: trimmed.length });
                    }}
                    onCopy={(text) => {
                      copyToClipboard(text);
                      showToast('Copied');
                      logEvent('message_copied', { role: 'user', length: text.length });
                    }}
                    formatTime={formatTime}
                  />
                )}
                {msg.role === 'assistant' && (() => {
                  // c2-393 / task 205: walk backwards from this assistant's
                  // index to find the closest user-turn timestamp. Skips
                  // system / web / tool rows interposed between user and
                  // assistant. undefined if none found — the chip hides.
                  let respondToTs: number | undefined;
                  for (let k = index - 1; k >= 0; k--) {
                    if (messages[k]?.role === 'user') { respondToTs = messages[k].timestamp; break; }
                  }
                  return (
                  <AssistantMessage
                    msg={msg} C={C} isMobile={isMobile} isDesktop={isDesktop}
                    isLast={messages[messages.length - 1]?.id === msg.id}
                    isThinking={isThinking}
                    respondToTs={respondToTs}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      setMsgContextMenu({ x: e.clientX, y: e.clientY, msgId: msg.id, role: 'assistant', content: msg.content });
                    }}
                    showReasoning={!!settings.showReasoning}
                    developerMode={!!settings.developerMode}
                    reasoningExpanded={expandedReasoning === msg.id}
                    renderBody={(text) => renderMessageBody(text)}
                    onToggleReasoning={() => setExpandedReasoning(expandedReasoning === msg.id ? null : msg.id)}
                    onRegenerate={regenerateLast}
                    onCopy={copyWithToast}
                    onOpenProvenance={(cid) => {
                      // c2-433 / #315: surface fetch failures so the user
                      // doesn't sit waiting for a system message that will
                      // never come (offline / backend down). Success path
                      // unchanged.
                      fetch(`http://${getHost()}:3000/api/provenance/${cid}`)
                        .then(r => { if (!r.ok) throw new Error(`HTTP ${r.status}`); return r.json(); })
                        .then(d => {
                          // c2-433 / task 245: stamp a stable id so the
                          // flash-after-append can target it.
                          const newId = msgId();
                          setMessages(prev => [...prev, {
                            id: newId, role: 'system',
                            content: `Provenance #${cid}:\n${d.explanation || JSON.stringify(d, null, 2).slice(0, 500)}`,
                            timestamp: Date.now(),
                          }]);
                          flashMessageById(newId, C.accent);
                        }).catch((e) => {
                          console.warn('provenance fetch failed', e);
                          showToast(`Couldn\u2019t load provenance #${cid}`);
                        });
                    }}
                    onFollowUpChip={(chip) => { setInputAndResize(chip); inputRef.current?.focus(); }}
                    onFeedbackPositive={() => {
                      // c2-433 / #350: rating='up' per Claude 0's spec
                      // (POST /api/feedback {rating: up|down|correct, ...}).
                      // conversation_id + lfi_reply included so the Classroom
                      // queue can render context without a follow-up fetch.
                      // c2-433 / #315: optimistically toast on send + only
                      // surface a follow-up "Feedback failed" toast if the
                      // network actually rejects. Keeps the happy path silent
                      // (user sees the green toast immediately) while the
                      // failure mode is no longer hidden.
                      hapticTick(15);
                      logEvent('feedback_positive', { msgId: msg.id });
                      fetch(`http://${getHost()}:3000/api/feedback`, {
                        method: 'POST', headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                          conversation_id: currentConversationId,
                          message_id: msg.id,
                          conclusion_id: (msg as any).conclusion_id,
                          lfi_reply: msg.content,
                          rating: 'up',
                        }),
                      }).then(r => { if (!r.ok) throw new Error(`HTTP ${r.status}`); })
                        .catch((e) => {
                          console.warn('feedback POST failed', e);
                          showToast('Feedback didn\u2019t reach the server');
                        });
                      showToast('Thanks for the feedback');
                    }}
                    onFeedbackNegative={() => {
                      hapticTick(15);
                      fb.openNegFeedback({ msgId: msg.id, conclusionId: (msg as any).conclusion_id });
                    }}
                    onFeedbackCorrect={() => {
                      hapticTick(15);
                      // Find the prior user turn so we can attach `user_query`
                      // for the Classroom queue. Walks backwards from current
                      // index to the most recent role==='user' message.
                      const idx = messages.findIndex(m => m.id === msg.id);
                      let userQuery: string | undefined;
                      for (let i = idx - 1; i >= 0; i--) {
                        if (messages[i].role === 'user') { userQuery = messages[i].content; break; }
                      }
                      fb.openCorrectFeedback({
                        msgId: msg.id,
                        conclusionId: (msg as any).conclusion_id,
                        lfiReply: msg.content,
                        userQuery,
                      });
                    }}
                    formatTime={formatTime}
                  />
                  );
                })()}
              </div>
            )}
          />

          {/* ========== INPUT BAR (inside main — centers with the chat column) ========== */}
          {/* Claude.ai-style: textarea on top, actions row below. Taller
              minimum height; model selector inline on the left; send on the
              right. Centered at the same 760 px column as the messages above. */}
          <div style={{
            padding: isDesktop ? '0 24px 18px' : '0 14px 12px',
            paddingBottom: isMobile ? 'max(14px, env(safe-area-inset-bottom))' : '18px',
            background: C.bg, flexShrink: 0,
          }}>
            <div style={{
              maxWidth: isDesktop ? '760px' : isTablet ? '680px' : '100%',
              margin: '0 auto',
              position: 'relative',
            }}>
              {/* c2-433 / task 244: post-turn modules-used pill. After a
                  turn completes, modulesUsed retains the modules that ran
                  for that turn (cleared on next handleSend). Surface them
                  in a pill above the input so users get a substrate-aware
                  after-action read without opening the activity log.
                  Hidden during streaming (the activity bar inside the
                  thinking indicator already shows live state) + when no
                  modules were recorded (backend hasn't started emitting
                  cognitive_module yet). */}
              {/* c2-433 / #307: rate-limit chip. Replaces the preview row
                  while the /api/explain cooldown is active. Counts down in
                  1s ticks via explainRateLimitTick. Dashed red border so it
                  doesn't feel alarming — the app isn't broken, just backing
                  off. */}
              {!isThinking && input.trim().length >= 6 && explainRateLimitUntil != null && Date.now() < explainRateLimitUntil && (() => {
                void explainRateLimitTick;
                const secs = Math.max(1, Math.ceil((explainRateLimitUntil - Date.now()) / 1000));
                return (
                  <div style={{
                    display: 'flex', justifyContent: 'flex-end',
                    gap: '6px', flexWrap: 'wrap', marginBottom: '4px',
                  }}>
                    <span title={`Preview endpoint rate-limited — resuming in ${secs}s (research cap is 10/300s)`}
                      style={{
                        display: 'inline-flex', alignItems: 'center', gap: '6px',
                        padding: '2px 9px', fontSize: '10px',
                        background: 'transparent',
                        border: `1px dashed ${C.red}77`, color: C.red,
                        borderRadius: T.radii.pill,
                        fontFamily: T.typography.fontMono, letterSpacing: '0.04em',
                      }}>
                      <span style={{ fontWeight: 700 }}>rate-limited</span>
                      <span style={{ color: C.red, opacity: 0.85 }}>{secs}s</span>
                    </span>
                  </div>
                );
              })()}
              {/* c2-433 / #316 / #300: predicted-modules preview row. While
                  the user is composing a query AND the debounced /api/explain
                  dry-run has returned, render a chip row with the predicted
                  speech_act, extracted concept, and active module indicators
                  (RAG facts count, causal preview, topic stack). Dashed
                  border + textDim color reads as "predicted, not yet run"
                  to differentiate from the solid modulesUsed row below.
                  Hidden during streaming (activity bar takes over), when
                  input is empty, and when no preview lives yet. */}
              {!isThinking && input.trim().length >= 6 && explainPreview && (() => {
                const p = explainPreview;
                const ragCount = Array.isArray(p.rag_top_facts) ? p.rag_top_facts.length : 0;
                const hasCausal = p.causal_preview && (Array.isArray(p.causal_preview) ? p.causal_preview.length > 0 : Object.keys(p.causal_preview).length > 0);
                const topic: string | null = (p.topic_stack && typeof p.topic_stack === 'object'
                  ? (p.topic_stack.current || p.topic_stack.top || null)
                  : (typeof p.topic_stack === 'string' ? p.topic_stack : null));
                // c2-433 / #300 followup: gate_verdicts — tolerant to three
                // shapes: array of {name|gate, passed|verdict}, plain object
                // {gate_name: 'pass'|'fail'|bool}, or array of strings (=
                // failing gates). Normalize to [{name, passed}] so the chip
                // can report totals + tooltip-list each gate.
                type GateV = { name: string; passed: boolean };
                let gates: GateV[] = [];
                const gv = p.gate_verdicts;
                if (Array.isArray(gv)) {
                  gates = gv.map((g: any): GateV => {
                    if (typeof g === 'string') return { name: g, passed: false };
                    const name = String(g.name ?? g.gate ?? g.id ?? 'gate');
                    const verdict = g.passed ?? g.pass ?? g.ok ?? g.verdict;
                    const passed = typeof verdict === 'boolean' ? verdict
                      : typeof verdict === 'string' ? /^(pass|ok|true|allow|green)$/i.test(verdict)
                      : false;
                    return { name, passed };
                  });
                } else if (gv && typeof gv === 'object') {
                  gates = Object.entries(gv).map(([name, v]: [string, any]): GateV => {
                    const passed = typeof v === 'boolean' ? v
                      : typeof v === 'string' ? /^(pass|ok|true|allow|green)$/i.test(v)
                      : !!v?.passed;
                    return { name, passed };
                  });
                }
                const failedGates = gates.filter(g => !g.passed);
                const hasGateInfo = gates.length > 0;
                const anySignal = !!p.speech_act || !!p.extracted_concept || ragCount > 0 || hasCausal || !!topic || hasGateInfo;
                if (!anySignal) return null;
                const chip = (label: string, value: string, title: string) => (
                  <span key={label} title={title} style={{
                    display: 'inline-flex', alignItems: 'center', gap: '6px',
                    padding: '2px 9px', fontSize: '10px',
                    background: 'transparent',
                    border: `1px dashed ${C.borderSubtle}`, color: C.textDim,
                    borderRadius: T.radii.pill,
                    fontFamily: T.typography.fontMono,
                    letterSpacing: '0.04em',
                    maxWidth: '180px', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis',
                  }}>
                    <span style={{ fontWeight: 700, color: C.textMuted }}>{label}</span>
                    <span style={{ color: C.textSecondary }}>{value}</span>
                  </span>
                );
                return (
                  <div style={{
                    display: 'flex', justifyContent: 'flex-end',
                    gap: '6px', flexWrap: 'wrap',
                    marginBottom: '4px',
                  }}
                    aria-label='Predicted substrate activity'
                    title='Pre-send pipeline dry-run (/api/explain)'>
                    {p.speech_act && chip('act', String(p.speech_act), `Detected speech act: ${p.speech_act}`)}
                    {p.extracted_concept && chip('concept', String(p.extracted_concept), `Concept: ${p.extracted_concept}`)}
                    {ragCount > 0 && chip('rag', `${ragCount} fact${ragCount === 1 ? '' : 's'}`, `${ragCount} retrieval hit${ragCount === 1 ? '' : 's'} queued`)}
                    {hasCausal && chip('causal', 'preview', 'Causal reasoning available for this query')}
                    {topic && chip('topic', topic, `Carrying topic: ${topic}`)}
                    {/* c2-433 / #300 followup: gate-verdicts chip. Tooltip
                        lists each gate with a check/cross. Red-bordered
                        when any gate fails (user's query will be blocked
                        downstream); green when all pass. Hidden when the
                        explain response carried no gate_verdicts at all. */}
                    {hasGateInfo && (() => {
                      const anyFailed = failedGates.length > 0;
                      const color = anyFailed ? C.red : C.green;
                      const border = anyFailed ? `${C.red}77` : `${C.green}77`;
                      const label = anyFailed
                        ? `${failedGates.length}/${gates.length} failing`
                        : `${gates.length} ok`;
                      const title = 'Pipeline gates:\n' + gates.map(g => `  ${g.passed ? '\u2713' : '\u2717'} ${g.name}`).join('\n');
                      return (
                        <span key='gates' title={title} style={{
                          display: 'inline-flex', alignItems: 'center', gap: '6px',
                          padding: '2px 9px', fontSize: '10px',
                          background: 'transparent',
                          border: `1px dashed ${border}`, color,
                          borderRadius: T.radii.pill,
                          fontFamily: T.typography.fontMono,
                          letterSpacing: '0.04em',
                        }}>
                          <span style={{ fontWeight: 700 }}>gates</span>
                          <span>{label}</span>
                        </span>
                      );
                    })()}
                  </div>
                );
              })()}
              {!isThinking && (modulesUsed.size > 0 || activeTopic) && (
                <div style={{
                  display: 'flex', justifyContent: 'flex-end',
                  gap: '6px', flexWrap: 'wrap',
                  marginBottom: '6px',
                }}>
                  {/* c2-433 / #352: persistent topic-context chip — same row
                      as modules-used. Shows what pronouns ("them", "it")
                      will resolve to since topic_stack persists across
                      turns. Click clears it (covers the case where backend
                      didn't pivot but the user wants a fresh context). */}
                  {activeTopic && (
                    <button onClick={() => setActiveTopic(null)}
                      title={`Multi-turn topic: ${activeTopic} — click to clear (next turn re-detects)`}
                      style={{
                        display: 'inline-flex', alignItems: 'center', gap: '6px',
                        padding: '3px 10px', fontSize: T.typography.sizeXs,
                        background: C.purpleBg,
                        border: `1px solid ${C.purpleBorder}`, color: C.purple,
                        borderRadius: T.radii.pill,
                        fontFamily: T.typography.fontMono,
                        cursor: 'pointer',
                        maxWidth: '240px', overflow: 'hidden',
                        textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      }}>
                      <span style={{ fontWeight: 700 }}>topic</span>
                      <span style={{ color: C.text }}>{activeTopic}</span>
                      <span style={{ opacity: 0.6, marginLeft: '2px' }}>{'\u2715'}</span>
                    </button>
                  )}
                  {modulesUsed.size > 0 && (
                    <span title={`Cognitive modules that contributed to the last turn: ${Array.from(modulesUsed).join(', ')}`}
                      style={{
                        display: 'inline-flex', alignItems: 'center', gap: '6px',
                        padding: '3px 10px', fontSize: T.typography.sizeXs,
                        background: C.bgInput,
                        border: `1px solid ${C.borderSubtle}`, color: C.textMuted,
                        borderRadius: T.radii.pill,
                        fontFamily: T.typography.fontMono,
                        // c2-433 / mobile: cap + ellipsize so a 6-module turn
                        // doesn't blow the pill past the chat input column.
                        maxWidth: '260px', overflow: 'hidden',
                        textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      }}>
                      <span style={{ color: C.accent, fontWeight: 700, fontFamily: 'inherit' }}>via</span>
                      <span style={{ color: C.text, overflow: 'hidden', textOverflow: 'ellipsis' }}>{Array.from(modulesUsed).join(' · ')}</span>
                    </span>
                  )}
                </div>
              )}
              {/* c2-433 / task 243: restore-last-prompt affordance. Renders
                  above the textarea only when (a) input is empty AND (b)
                  there's a backup AND (c) the backup is fresh (< 30s).
                  Click restores the prior draft + focuses the input.
                  draftBackupTick referenced so React re-evaluates the
                  freshness check on each backup write. */}
              {(() => {
                void draftBackupTick;
                if (input.length > 0) return null;
                const b = draftBackupRef.current;
                if (!b) return null;
                if (Date.now() - b.at > 30_000) return null;
                const preview = b.text.replace(/\s+/g, ' ').slice(0, 60);
                return (
                  <div style={{
                    display: 'flex', justifyContent: 'flex-end',
                    marginBottom: '6px',
                  }}>
                    <button onClick={restoreDraftBackup}
                      title={`Restore: ${b.text.slice(0, 200)}${b.text.length > 200 ? '…' : ''}`}
                      aria-label='Restore last sent prompt'
                      style={{
                        display: 'flex', alignItems: 'center', gap: '6px',
                        padding: '4px 10px', fontSize: T.typography.sizeXs,
                        background: 'transparent',
                        border: `1px solid ${C.borderSubtle}`, color: C.textMuted,
                        borderRadius: T.radii.pill, cursor: 'pointer',
                        fontFamily: 'inherit',
                        maxWidth: '420px',
                      }}>
                      <span style={{ color: C.accent, fontWeight: 700 }}>{'\u21BA'}</span>
                      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        Restore "{preview}{b.text.length > 60 ? '…' : ''}"
                      </span>
                    </button>
                  </div>
                );
              })()}
              {/* Slash command menu — pops above the input like Discord/Slack */}
              {showSlashMenu && (() => {
                const filtered = slashCommands.filter(c =>
                  !slashFilter || c.cmd.slice(1).startsWith(slashFilter) || c.label.toLowerCase().includes(slashFilter)
                );
                if (filtered.length === 0) return null;
                const clamped = Math.min(slashIndex, filtered.length - 1);
                return (
                  <div style={{
                    position: 'absolute', bottom: '100%', left: 0, right: 0,
                    marginBottom: '6px', maxHeight: '280px', overflowY: 'auto',
                    background: C.bgCard, border: `1px solid ${C.border}`,
                    borderRadius: T.radii.xxl, padding: '6px',
                    boxShadow: '0 -12px 40px rgba(0,0,0,0.35)',
                    animation: 'lfi-fadein 0.12s ease-out', zIndex: 50,
                  }}>
                    <div style={{ padding: '6px 10px', fontSize: '10px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em' }}>
                      Commands
                    </div>
                    {filtered.map((c, i) => (
                      <button key={c.cmd}
                        onClick={() => { c.run(); setInputAndResize(''); sm.close(); logEvent('slash_cmd', { cmd: c.cmd }); }}
                        onMouseEnter={() => sm.setIndex(i)}
                        style={{
                          width: '100%', textAlign: 'left', cursor: 'pointer',
                          padding: '8px 12px', background: i === clamped ? C.accentBg : 'transparent',
                          border: 'none', borderRadius: T.radii.lg, fontFamily: 'inherit',
                          color: C.text, display: 'flex', alignItems: 'center', gap: T.spacing.md,
                        }}>
                        <span style={{ fontSize: T.typography.sizeMd, fontWeight: 700, color: i === clamped ? C.accent : C.textSecondary, minWidth: '90px',
                          fontFamily: "'JetBrains Mono','Fira Code',monospace" }}>{c.cmd}</span>
                        <span style={{ fontSize: T.typography.sizeMd, color: C.textMuted }}>{c.desc}</span>
                      </button>
                    ))}
                  </div>
                );
              })()}

              {/* c2-372 / task 105 / c2-433 #339 vocab sweep: live throughput
                  chip during streaming. Was tokens/s + tokens count — replaced
                  with chars/s + char count, the post-LLM-honest unit (no
                  tokenizer assumed). Renders between chat list and input row,
                  hidden outside active streams, right-aligned. The
                  streamTimingTick dep keeps the chip refreshing without re-
                  rendering the whole tree. */}
              {streamTiming && (() => {
                // c2-433 / #313 pass 8: chars-per-second derived inside the
                // hook (cstr.charsPerSecond). streamTimingTick is referenced
                // so React re-runs this branch each tick without a manual
                // useState dep; the void keeps the unused-var lint quiet.
                void streamTimingTick;
                const cps = cstr.charsPerSecond ?? 0;
                const elapsedSec = (Date.now() - streamTiming.startAt) / 1000;
                return (
                  <div aria-live='polite' style={{
                    display: 'flex', justifyContent: 'flex-end',
                    marginBottom: T.spacing.sm,
                    fontSize: T.typography.sizeXs, color: C.textMuted,
                    fontFamily: T.typography.fontMono,
                  }}>
                    <span title={`${streamTiming.chars.toLocaleString()} characters streamed in ${elapsedSec.toFixed(1)}s`}
                      style={{
                        background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                        borderRadius: T.radii.sm,
                        padding: `2px ${T.spacing.sm}`,
                      }}>
                      {cps.toLocaleString()} chars/s <span style={{ color: C.textDim }}>({streamTiming.chars.toLocaleString()})</span>
                    </span>
                  </div>
                );
              })()}

              {/* c2-371 / task 79: retry affordance surfaced when the last
                  assistant turn errored. Renders above the input bar with
                  red-accent styling so it reads as a recovery action rather
                  than a primary CTA. Dismiss button lets the user drop the
                  prompt without resending. */}
              {lastErrorRetry && (
                <div role='alert' style={{
                  display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                  marginBottom: T.spacing.sm,
                  padding: `${T.spacing.sm} ${T.spacing.md}`,
                  background: C.redBg, border: `1px solid ${C.redBorder}`,
                  borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
                  color: C.red,
                }}>
                  <span style={{ flex: 1 }}>
                    <strong>Last reply failed.</strong>{' '}
                    <span style={{ color: C.textSecondary }}>Resend that prompt?</span>
                  </span>
                  <button
                    onClick={() => {
                      const prompt = lastErrorRetry.userContent;
                      setLastErrorRetry(null);
                      // c2-433: same setInputAndResize as the other recall
                      // paths so a multi-line failed prompt isn't briefly
                      // shown clipped in the textarea before handleSend
                      // clears it.
                      setInputAndResize(prompt);
                      // Give React a tick to commit the input state before
                      // handleSend reads from it.
                      setTimeout(() => { handleSend(); }, 50);
                      logEvent('chat_retry', { len: prompt.length });
                    }}
                    aria-label='Retry last message'
                    style={{
                      background: C.red, color: '#fff',
                      border: 'none', borderRadius: T.radii.sm,
                      padding: `${T.spacing.xs} ${T.spacing.md}`,
                      fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                      textTransform: 'uppercase', letterSpacing: '0.06em',
                      cursor: 'pointer', fontFamily: 'inherit',
                    }}>Retry</button>
                  <button
                    onClick={() => setLastErrorRetry(null)}
                    aria-label='Dismiss retry prompt'
                    style={{
                      background: 'transparent', border: 'none',
                      color: C.textMuted, cursor: 'pointer',
                      fontSize: T.typography.sizeMd, padding: '0 4px',
                    }}>{'\u2715'}</button>
                </div>
              )}

              <div
                key={`input-${sendPulseId}`}
                className={sendPulseId > 0 ? 'lfi-send-pulse' : undefined}
                style={{
                background: C.bgCard,
                // c0-019/020: professional rounded-card, 8px radius, no glow.
                // Ring halo only on focus via box-shadow in a muted accent.
                border: `1px solid ${input ? C.accent : C.border}`,
                borderRadius: T.radii.lg,
                transition: 'border-color 0.15s, box-shadow 0.15s',
                boxShadow: input ? `0 0 0 3px ${C.accentBg}` : '0 1px 2px rgba(15,17,23,0.24)',
                display: 'flex', flexDirection: 'column', position: 'relative',
              }}>
                {/* c2-361 / task 91: context-window usage indicator. Thin 3px
                    bar at the very top of the input wrapper. Uses the same
                    100k-char budget as the >70% meter below, computed as a
                    rough GPT-style token estimate (4 chars/token). Stays
                    invisible on empty input. Green (<50%), yellow (50-80%),
                    red (>80%) so the colors agree with the later char-count
                    meter without duplicating its logic. */}
                {input.length > 0 && (() => {
                  const pct = Math.min(1, input.length / 100000);
                  const color = pct < 0.5 ? C.green : pct < 0.8 ? C.yellow : C.red;
                  return (
                    <div aria-hidden='true' style={{
                      position: 'absolute', top: 0, left: 0, right: 0,
                      height: '3px', background: C.bgInput,
                      borderTopLeftRadius: T.radii.lg,
                      borderTopRightRadius: T.radii.lg,
                      overflow: 'hidden', pointerEvents: 'none',
                    }}>
                      <div style={{
                        width: `${pct * 100}%`, height: '100%',
                        background: color, transition: 'width 0.3s, background 0.3s',
                      }} />
                    </div>
                  );
                })()}
                {/* c2-413 / BIG #218 mobile + c2-433 #339: unified char
                    counter chip. Default subtle styling under 70% of the
                    100k char transport cap; turns amber 70–95% and red
                    >95% — colour is the only alarm signal, layout stays
                    identical so the chip doesn't jump positions.
                    Positioned top-right so it doesn't collide with the
                    Send button at bottom-right on narrow viewports.
                    Post-LLM vocabulary: "transport cap" not "context
                    window"; no tokenizer estimate. */}
                {input.length > 0 && (() => {
                  // c2-433 / #339 vocab sweep: dropped "tokens" + "context
                  // window" wording. Post-LLM positioning treats the input
                  // as raw chars; the cap is a transport limit, not a
                  // tokenizer-shaped budget. Title is plain "chars · cap".
                  const CAP = 100000;
                  const pct = input.length / CAP;
                  const loud = pct > 0.70;
                  const color = pct > 0.95 ? C.red : pct > 0.70 ? C.yellow : C.textDim;
                  const bg = pct > 0.95 ? C.redBg : pct > 0.70 ? C.accentBg : 'transparent';
                  return (
                    <div aria-live={loud ? 'polite' : 'off'}
                      title={`${input.length.toLocaleString()} of ${CAP.toLocaleString()} characters`}
                      style={{
                        position: 'absolute', top: '6px', right: '14px',
                        fontSize: '10px', fontWeight: loud ? 700 : 500,
                        color, fontFamily: T.typography.fontMono,
                        background: bg,
                        padding: bg === 'transparent' ? 0 : '2px 6px',
                        borderRadius: T.radii.sm,
                        pointerEvents: 'none',
                        whiteSpace: 'nowrap',
                        // c2-413: tabular-nums keeps width stable as digits
                        // tick, so the chip doesn't dance around the corner.
                        fontVariantNumeric: 'tabular-nums',
                      }}>
                      {input.length.toLocaleString()} chars
                    </div>
                  );
                })()}
              {/* #187: URL-paste title preview chip. Shown above the textarea
                  when a URL was pasted into an empty input. Click × to
                  dismiss, click chip to open the URL in a new tab. */}
              {urlPreview && (
                <div style={{
                  display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                  padding: '8px 14px 0', flexWrap: 'wrap',
                }}>
                  <a href={urlPreview.url} target='_blank' rel='noopener noreferrer'
                    title={urlPreview.url}
                    style={{
                      display: 'flex', alignItems: 'center', gap: '6px',
                      padding: '4px 10px', borderRadius: T.radii.sm,
                      background: C.bgInput, color: C.text,
                      border: `1px solid ${C.borderSubtle}`,
                      fontSize: T.typography.sizeXs, textDecoration: 'none',
                      maxWidth: '100%', minWidth: 0,
                    }}>
                    <span aria-hidden style={{ color: C.textMuted }}>🔗</span>
                    <span style={{
                      fontWeight: T.typography.weightBold,
                      color: urlPreview.loading ? C.textDim : C.text,
                      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      maxWidth: '420px',
                    }}>
                      {urlPreview.loading
                        ? 'Fetching title…'
                        : urlPreview.title || (() => {
                            try { return new URL(urlPreview.url).hostname; }
                            catch { return urlPreview.url; }
                          })()}
                    </span>
                    {urlPreview.error && !urlPreview.loading && (
                      <span title={urlPreview.error}
                        style={{ fontSize: '10px', color: C.textDim, fontStyle: 'italic' }}>
                        (no title)
                      </span>
                    )}
                  </a>
                  <button onClick={() => setUrlPreview(null)}
                    aria-label='Dismiss URL preview'
                    title='Dismiss'
                    style={{
                      width: '22px', height: '22px', borderRadius: '50%',
                      background: 'transparent', color: C.textMuted,
                      border: `1px solid ${C.borderSubtle}`,
                      fontSize: T.typography.sizeXs, lineHeight: 1, padding: 0,
                      cursor: 'pointer',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                    }}>{'\u2715'}</button>
                </div>
              )}
              {/* c2-230 / #71: paste-image preview strip. Rendered above the
                  textarea inside the input container so the thumbnails share
                  the input's focus ring. Empty state collapses to nothing. */}
              {pastedImages.length > 0 && (
                <div style={{
                  display: 'flex', flexWrap: 'wrap', gap: '6px',
                  padding: '10px 14px 0', alignItems: 'center',
                }}>
                  {pastedImages.map((img, idx) => {
                    // c2-286: include image type + size in alt/title so the
                    // preview carries more than "Pasted image preview" —
                    // assistive tech users get the metadata sighted users
                    // can infer from the thumbnail, and the hover tooltip
                    // matches.
                    const mimeShort = img.type.replace(/^image\//, '').toUpperCase();
                    const kb = (img.size / 1024).toFixed(0);
                    const label = `Pasted image ${idx + 1} (${mimeShort}, ${kb} KB)`;
                    return (
                    <div key={img.id} style={{ position: 'relative' }}>
                      <img src={img.dataUrl} alt={label} title={label}
                        style={{
                          width: '56px', height: '56px', objectFit: 'cover',
                          borderRadius: T.radii.sm, border: `1px solid ${C.border}`,
                          display: 'block',
                        }} />
                      <button onClick={() => setPastedImages(prev => prev.filter(p => p.id !== img.id))}
                        aria-label={`Remove ${label}`}
                        title='Remove'
                        style={{
                          position: 'absolute', top: isMobile ? '-8px' : '-6px', right: isMobile ? '-8px' : '-6px',
                          // c2-433 / Bible §6.1 tap-target: 18px was below
                          // the 44px recommendation. Bumped to 28px on
                          // mobile (still tight but visually proportional
                          // to a 56px thumbnail) so finger taps land
                          // reliably without accidentally missing.
                          width: isMobile ? '28px' : '18px',
                          height: isMobile ? '28px' : '18px',
                          borderRadius: '50%',
                          background: C.bg, color: C.text,
                          border: `1px solid ${C.border}`,
                          fontSize: isMobile ? T.typography.sizeSm : T.typography.sizeXs,
                          lineHeight: 1, padding: 0, cursor: 'pointer',
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                        }}>{'\u2715'}</button>
                    </div>
                    );
                  })}
                  <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginLeft: '4px' }}>
                    {pastedImages.length === 1 ? '1 image' : `${pastedImages.length} images`} ready {'\u2014'} backend upload not yet wired
                  </span>
                </div>
              )}
              <textarea
                ref={inputRef}
                data-tour='chat-input'
                aria-label='Chat message input'
                autoComplete='off'
                spellCheck={true}
                dir='auto'
                value={input}
                onChange={handleInputChange}
                onPaste={(e) => {
                  // c2-230 / #71: intercept image clipboard items before the
                  // browser tries to paste their base64 representation as text.
                  // Text-only pastes fall through to the default handler.
                  const items = Array.from(e.clipboardData?.items ?? []);
                  const imageItems = items.filter(it => it.kind === 'file' && it.type.startsWith('image/'));
                  // #187: URL-paste title preview. When the clipboard holds a
                  // single URL and input was empty, kick a background unfurl
                  // so the user sees what the link is before sending.
                  if (imageItems.length === 0) {
                    const text = e.clipboardData?.getData('text') || '';
                    const trimmed = text.trim();
                    const urlOnly = /^(https?:\/\/[^\s]+)$/i.test(trimmed);
                    if (urlOnly && input.trim() === '') {
                      const url = trimmed;
                      setUrlPreview({ url, title: null, loading: true, error: null });
                      (async () => {
                        // Try backend unfurl endpoints in order. Graceful 404.
                        const candidates = [
                          `/api/unfurl?url=${encodeURIComponent(url)}`,
                          `/api/fetch_title?url=${encodeURIComponent(url)}`,
                        ];
                        for (const path of candidates) {
                          try {
                            const r = await fetch(`http://${getHost()}:3000${path}`);
                            if (!r.ok) continue;
                            const d: any = await r.json().catch(() => null);
                            const title = d?.title || d?.og_title || d?.meta_title || null;
                            setUrlPreview(prev => prev && prev.url === url
                              ? { url, title, loading: false, error: title ? null : 'no title in response' }
                              : prev);
                            return;
                          } catch { /* try next */ }
                        }
                        setUrlPreview(prev => prev && prev.url === url
                          ? { url, title: null, loading: false, error: 'unfurl endpoint not available' }
                          : prev);
                      })();
                    }
                    return;
                  }
                  e.preventDefault();
                  const MAX_BYTES = 5 * 1024 * 1024; // 5 MB per image
                  for (const it of imageItems) {
                    const file = it.getAsFile();
                    if (!file) continue;
                    if (file.size > MAX_BYTES) {
                      // c2-433 / task 268: was a system message in the chat
                      // — clutters the convo history with a one-shot
                      // ephemeral error. Toast is the right surface for
                      // "your action couldn't complete because X."
                      showToast(`Image too large (${(file.size / 1024 / 1024).toFixed(1)}MB) — max 5MB`);
                      continue;
                    }
                    const reader = new FileReader();
                    reader.onload = () => {
                      const dataUrl = typeof reader.result === 'string' ? reader.result : '';
                      if (!dataUrl) return;
                      setPastedImages(prev => [...prev, { id: msgId(), dataUrl, size: file.size, type: file.type }]);
                    };
                    reader.readAsDataURL(file);
                  }
                  logEvent('paste_image', { count: imageItems.length });
                }}
                onKeyDown={(e) => {
                  // Slash menu keyboard nav.
                  if (showSlashMenu) {
                    const filtered = slashCommands.filter(c =>
                      !slashFilter || c.cmd.slice(1).startsWith(slashFilter) || c.label.toLowerCase().includes(slashFilter)
                    );
                    if (e.key === 'ArrowDown') { e.preventDefault(); sm.moveDown(filtered.length); return; }
                    if (e.key === 'ArrowUp') { e.preventDefault(); sm.moveUp(); return; }
                    if (e.key === 'Enter' || e.key === 'Tab') {
                      e.preventDefault();
                      const picked = filtered[Math.min(slashIndex, filtered.length - 1)];
                      if (picked) { picked.run(); setInputAndResize(''); sm.close(); logEvent('slash_cmd', { cmd: picked.cmd }); }
                      return;
                    }
                    if (e.key === 'Escape') { sm.close(); return; }
                  }
                  // c2-433 / task 250: prompt-history navigation via
                  // Shift+ArrowUp/Down. Walks the ring buffer of last 10
                  // sent prompts. ArrowUp from "not navigating" lands at
                  // the most recent. ArrowDown past the recent end clears
                  // the input. Falls back to the messages-array recall when
                  // the ring buffer is empty (legacy behavior on first use).
                  if (e.shiftKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
                    const buf = promptHistoryRef.current;
                    if (buf.length === 0) {
                      // Legacy fallback: pull the most recent user message
                      // from the messages array if there's no ring entries
                      // yet (fresh tab, no sends in this session).
                      if (e.key === 'ArrowUp' && !input.trim()) {
                        const lastUser = [...messages].reverse().find(m => m.role === 'user');
                        if (lastUser) {
                          e.preventDefault();
                          setInputAndResize(lastUser.content);
                          return;
                        }
                      }
                    } else {
                      e.preventDefault();
                      let cursor = promptHistoryCursorRef.current;
                      if (e.key === 'ArrowUp') {
                        cursor = cursor < 0 ? buf.length - 1 : Math.max(0, cursor - 1);
                      } else {
                        cursor = cursor < 0 ? -1 : Math.min(buf.length, cursor + 1);
                      }
                      promptHistoryCursorRef.current = cursor;
                      const value = (cursor >= 0 && cursor < buf.length) ? buf[cursor] : '';
                      setInputAndResize(value);
                      return;
                    }
                  }
                  // Cmd/Ctrl+Enter always sends, regardless of the sendOnEnter
                  // setting — gives power users a consistent shortcut even
                  // when they've turned off plain-Enter send.
                  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                    e.preventDefault(); handleSend(); return;
                  }
                  if (!settings.sendOnEnter) return;
                  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
                }}
                placeholder={
                  // c2-433 / task 267: context-aware placeholder. Mirrors
                  // current state (offline → queue, thinking → stop-first)
                  // so users get the most relevant nudge at a glance
                  // instead of a static "Ask PlausiDen anything…".
                  !isConnected
                    ? 'Backend offline — type to queue, sends on reconnect'
                    : isThinking
                      ? 'Type your next question — current reply is in flight'
                      : 'Ask PlausiDen anything…'
                }
                maxLength={100000}
                style={{
                  background: 'transparent', border: 'none', outline: 'none',
                  // c2-411 / BIG #218 mobile: bump to 16px on mobile — iOS
                  // Safari zooms the viewport when an input has font-size
                  // <16px focused. 15.5px on desktop is unchanged via the
                  // isMobile branch.
                  resize: 'none', fontSize: isMobile ? '16px' : '15.5px', lineHeight: '1.55',
                  padding: '18px 20px 10px',
                  color: C.text, fontFamily: 'inherit',
                  minHeight: '72px', maxHeight: '280px',
                }}
                rows={2}
              />
              {/* c2-432 mobile compaction: tighter gap + padding; never wrap
                  (extra overflow falls through to horizontal scroll) so the
                  Send button stays reachable without a row break. */}
              <div style={{
                display: 'flex', alignItems: 'center',
                gap: isMobile ? '4px' : '6px',
                padding: isMobile ? '4px 6px 8px' : '6px 10px 10px',
                position: 'relative',
                flexWrap: 'nowrap',
                overflowX: 'auto',
                scrollbarWidth: 'none',
              }}>
                {/* Skills "+" button — opens popover with all skills. Cleaner
                    than a wide scrolling row when you have 7+ tools. */}
                <div style={{ position: 'relative', flexShrink: 0 }}>
                  <button onClick={() => setShowSkillMenu(v => !v)}
                    title='Tools &amp; skills'
                    aria-label='Tools and skills'
                    style={{
                      width: '36px', height: '36px', cursor: 'pointer',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      background: activeSkill !== 'chat' ? C.accentBg : (showSkillMenu ? C.bgHover : 'transparent'),
                      border: `1px solid ${activeSkill !== 'chat' ? C.accentBorder : 'transparent'}`,
                      color: activeSkill !== 'chat' ? C.accent : C.textMuted,
                      borderRadius: T.radii.lg,
                    }}>
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
                    </svg>
                  </button>
                  {showSkillMenu && (
                    <>
                      <div onClick={() => setShowSkillMenu(false)}
                        style={{
                          position: 'fixed', inset: 0, zIndex: 170,
                          background: isMobile ? 'rgba(0,0,0,0.45)' : 'transparent',
                        }} />
                      <div style={isMobile ? {
                        // Mobile: bottom sheet — full width, anchored to bottom,
                        // respects safe-area-inset.
                        position: 'fixed', left: 0, right: 0, bottom: 0, zIndex: 180,
                        background: C.bgCard, border: `1px solid ${C.border}`,
                        borderRadius: '16px 16px 0 0', padding: '10px 10px max(14px, env(safe-area-inset-bottom))',
                        boxShadow: '0 -16px 40px rgba(0,0,0,0.45)',
                        animation: 'lfi-fadein 0.2s ease-out',
                        maxHeight: '60vh', overflowY: 'auto',
                      } : {
                        position: 'absolute', bottom: 'calc(100% + 8px)', left: 0,
                        width: '260px', zIndex: 180,
                        background: C.bgCard, border: `1px solid ${C.border}`,
                        borderRadius: T.radii.xxl, padding: '6px',
                        boxShadow: '0 16px 40px rgba(0,0,0,0.35)',
                        animation: 'lfi-fadein 0.15s ease-out',
                      }}>
                        {skills.map(s => {
                          const picked = activeSkill === s.id;
                          return (
                            <button key={s.id}
                              disabled={!s.available}
                              onClick={() => {
                                if (!s.available) return;
                                setActiveSkill(picked ? 'chat' : s.id);
                                setShowSkillMenu(false);
                                logEvent('skill_selected', { skill: s.id });
                              }}
                              style={{
                                width: '100%', display: 'flex', alignItems: 'center', gap: '10px',
                                padding: '10px 12px',
                                background: picked ? C.accentBg : 'transparent',
                                border: 'none', cursor: s.available ? 'pointer' : 'not-allowed',
                                color: picked ? C.accent : (s.available ? C.text : C.textDim),
                                borderRadius: T.radii.lg, fontFamily: 'inherit', textAlign: 'left',
                                opacity: s.available ? 1 : 0.55,
                              }}
                              onMouseEnter={(e) => { if (s.available && !picked) e.currentTarget.style.background = C.bgHover; }}
                              onMouseLeave={(e) => { if (!picked) e.currentTarget.style.background = 'transparent'; }}>
                              {s.icon}
                              <div style={{ flex: 1, minWidth: 0 }}>
                                <div style={{ fontSize: T.typography.sizeMd, fontWeight: 600 }}>
                                  {s.label}{!s.available && <span style={{ fontSize: '10px', marginLeft: '6px', color: C.textDim }}>soon</span>}
                                </div>
                                <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '2px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                  {s.hint}
                                </div>
                              </div>
                              {picked && <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="20 6 9 17 4 12"/></svg>}
                            </button>
                          );
                        })}
                      </div>
                    </>
                  )}
                </div>
                {/* Attach */}
                <label title='Attach file' aria-label='Attach file'
                  style={{
                    width: '36px', height: '36px', cursor: 'pointer',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', color: C.textMuted,
                    borderRadius: T.radii.lg, flexShrink: 0,
                  }}>
                  <input type='file' multiple style={{ display: 'none' }}
                    onChange={(e) => {
                      const files = Array.from(e.target.files || []);
                      if (files.length === 0) return;
                      // c2-305: cap per-file size to prevent users dragging
                      // 500MB binaries into a UI that can't do anything with
                      // them yet. 25MB matches typical ChatGPT/Claude limits
                      // so the ceiling won't surprise power users.
                      const MAX_BYTES = 25 * 1024 * 1024;
                      const accepted = files.filter(f => f.size <= MAX_BYTES);
                      const rejected = files.filter(f => f.size > MAX_BYTES);
                      if (rejected.length > 0) {
                        const names = rejected.map(f => `${f.name} (${(f.size / 1024 / 1024).toFixed(1)}MB)`).join(', ');
                        setMessages(prev => [...prev, {
                          id: msgId(), role: 'system',
                          content: `Skipped oversize: ${names}. Max 25MB per file.`,
                          timestamp: Date.now(),
                        }]);
                      }
                      if (accepted.length === 0) { e.target.value = ''; return; }
                      const names = accepted.map(f => f.name).join(', ');
                      const totalBytes = accepted.reduce((s, f) => s + f.size, 0);
                      const totalKb = (totalBytes / 1024).toFixed(0);
                      setMessages(prev => [...prev, {
                        id: msgId(), role: 'system',
                        content: `Attached: ${names} (${accepted.length} file${accepted.length === 1 ? '' : 's'}, ${totalKb} KB). Upload backend is not yet wired \u2014 names logged for now.`,
                        timestamp: Date.now(),
                      }]);
                      logEvent('file_attached', { count: accepted.length, totalBytes, names, rejected: rejected.length });
                      e.target.value = '';
                    }} />
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.49"/>
                  </svg>
                </label>
                {/* Voice */}
                <button title='Voice input' aria-label='Voice input'
                  onClick={() => {
                    const Rec: any = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
                    if (!Rec) {
                      setMessages(prev => [...prev, { id: msgId(), role: 'system',
                        content: 'Voice input needs a browser with SpeechRecognition (Chrome/Edge).',
                        timestamp: Date.now() }]);
                      return;
                    }
                    const rec = new Rec();
                    rec.lang = 'en-US'; rec.interimResults = false; rec.maxAlternatives = 1;
                    rec.onresult = (e: any) => {
                      const text = e.results?.[0]?.[0]?.transcript || '';
                      if (text) setInput(prev => (prev ? prev + ' ' : '') + text);
                    };
                    // c2-304: surface the listening state + error cases so the
                    // user isn't staring at a button that silently fails when
                    // mic permission is denied or the service can't reach the
                    // browser's speech endpoint.
                    rec.onstart = () => { showToast('Listening…'); };
                    rec.onerror = (e: any) => {
                      const err = e?.error || 'unknown';
                      const friendly = err === 'not-allowed' || err === 'service-not-allowed'
                        ? 'Microphone permission denied — enable it in browser settings.'
                        : err === 'no-speech' ? 'No speech detected.'
                        : err === 'audio-capture' ? 'No microphone found.'
                        : err === 'network' ? 'Voice service unreachable.'
                        : `Voice input failed: ${err}`;
                      showToast(friendly);
                      logEvent('voice_error', { error: err });
                    };
                    rec.onend = () => { logEvent('voice_ended', {}); };
                    rec.start();
                    logEvent('voice_started', {});
                  }}
                  style={{
                    width: '36px', height: '36px', cursor: 'pointer',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', color: C.textMuted, border: 'none',
                    borderRadius: T.radii.lg, flexShrink: 0,
                  }}>
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                    <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                    <line x1="12" y1="19" x2="12" y2="23"/>
                    <line x1="8" y1="23" x2="16" y2="23"/>
                  </svg>
                </button>
                {/* c2-428 / #339 pivot: architecturally LFI is post-LLM
                    (HDC/VSA/PSL/HDLM per LFI_SUPERSOCIETY_ARCHITECTURE.md)
                    so "tier" is cosmetic for now. Kept at the user's
                    explicit request until the activity-bar / cognitive
                    module surface (#316) lands to replace it with
                    something accurate. c2-429 restore. */}
                <select value={currentTier} disabled={tierSwitching}
                  onChange={(e) => handleTierSwitch(e.target.value)}
                  title={`Response tier — Pulse: fast / Bridge: balanced / BigBrain: deepest. Currently: ${currentTier}.`}
                  aria-label={`Response tier (currently ${currentTier})`}
                  style={{
                    padding: isMobile ? '6px 22px 6px 8px' : '7px 28px 7px 12px',
                    fontSize: isMobile ? T.typography.sizeSm : T.typography.sizeMd,
                    fontWeight: 600,
                    background: C.bgInput, color: C.text,
                    border: `1px solid ${C.border}`, borderRadius: T.radii.lg,
                    cursor: tierSwitching ? 'wait' : 'pointer', fontFamily: 'inherit',
                    appearance: 'none', WebkitAppearance: 'none',
                    backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%237f8296' d='M0 2l4 4 4-4z'/%3E%3C/svg%3E")`,
                    backgroundRepeat: 'no-repeat', backgroundPosition: isMobile ? 'right 6px center' : 'right 10px center',
                    flexShrink: 0,
                  }}>
                  <option value="Pulse">{isMobile ? 'Pulse' : 'Pulse \u00B7 fast'}</option>
                  <option value="Bridge">{isMobile ? 'Bridge' : 'Bridge \u00B7 balanced'}</option>
                  <option value="BigBrain">{isMobile ? 'BigBrain' : 'BigBrain \u00B7 deepest'}</option>
                </select>
                <div style={{ flex: 1, minWidth: isMobile ? '0' : '4px' }} />
                {/* c2-432 mobile: active-skill chip collapses to icon-only
                    on mobile so it doesn't push the Send button off-screen.
                    Desktop keeps icon + label. */}
                {activeSkill !== 'chat' && (
                  <button onClick={() => setActiveSkill('chat')}
                    title={`Clear active skill (${activeSkillMeta.label})`}
                    aria-label={`Clear active skill: ${activeSkillMeta.label}`}
                    style={{
                      display: 'flex', alignItems: 'center', gap: isMobile ? '4px' : '6px',
                      padding: isMobile ? '5px 8px' : '5px 10px',
                      fontSize: '11.5px', fontWeight: 600,
                      background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                      color: C.accent, borderRadius: T.radii.pill,
                      cursor: 'pointer', fontFamily: 'inherit', flexShrink: 0,
                    }}>
                    {activeSkillMeta.icon}
                    {!isMobile && <span>{activeSkillMeta.label}</span>}
                    <span style={{ opacity: 0.7, fontSize: '10px', marginLeft: '2px' }}>{'\u2715'}</span>
                  </button>
                )}
                {/* c2-432 mobile: removed redundant `N chars` inline counter —
                    the top-right counter chip (c2-413) already shows
                    chars + tokens in one line. */}
                {/* Send */}
                <button
                  onClick={handleSend}
                  // c2-303 + c2-433 / task 287: mirror the handleSend guard —
                  // an empty text box with pasted images queued is a valid
                  // send. Offline is no longer a disable condition: the
                  // outbox path persists the message + drains on reconnect.
                  // Title/aria already steer the user (Backend offline —
                  // message will queue) so enabling matches the affordance.
                  disabled={(!input.trim() && pastedImages.length === 0) || isThinking}
                  className="scc-send-btn"
                  // c2-433 / task 277: title + aria-label reflect the actual
                  // gating state. Disabled-because-offline / disabled-because-
                  // empty / disabled-because-thinking each get their own
                  // hover hint instead of a generic "Send (Enter)".
                  title={
                    isThinking ? 'AI is replying — wait or hit Stop'
                      : !isConnected ? 'Backend offline — message will queue'
                      : (!input.trim() && pastedImages.length === 0) ? 'Type a message to send'
                      : 'Send (Enter)'
                  }
                  aria-label={isThinking ? 'Sending…' : 'Send message'}
                  aria-busy={isThinking}
                  style={{
                    width: '36px', height: '36px',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    // c2-433 / task 287: accent applies whenever the action
                    // will succeed — including the offline-queue path.
                    // Drops to bgInput only when truly disabled (empty +
                    // no images, or mid-stream).
                    background: input.trim() && !isThinking ? C.accent : C.bgInput,
                    border: `1px solid ${input.trim() && !isThinking ? C.accent : C.border}`,
                    borderRadius: T.radii.lg,
                    color: input.trim() && !isThinking ? (settings.theme === 'light' ? '#fff' : '#000') : C.textDim,
                    // c2-433 / task 278: distinguish "wait" (mid-stream) from
                    // "default" (disabled because empty). 'wait' cursor on
                    // thinking matches user expectations for inflight async.
                    cursor: isThinking ? 'wait' : (input.trim() ? 'pointer' : 'default'),
                    flexShrink: 0, transition: 'all 0.15s',
                  }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="12" y1="19" x2="12" y2="5"/>
                    <polyline points="5 12 12 5 19 12"/>
                  </svg>
                </button>
              </div>
            </div>
            </div>
            <div style={{
              maxWidth: isDesktop ? '760px' : isTablet ? '680px' : '100%',
              margin: '8px auto 0', display: 'flex', justifyContent: 'space-between',
              fontSize: '10.5px', color: C.textDim, padding: '0 8px',
            }}>
              <span style={{ color: isConnected ? C.green : C.red, fontWeight: 700, display: 'inline-flex', alignItems: 'center', gap: '6px' }}>
                {isConnected ? 'Link active' : 'Reconnecting...'}
                {isConnected && latencyMs != null && (
                  <span style={{
                    fontSize: '9.5px', fontWeight: 600,
                    color: latencyMs < 100 ? C.green : latencyMs < 500 ? C.yellow : C.red,
                    background: latencyMs < 100 ? C.greenBg : latencyMs < 500 ? C.accentBg : C.redBg,
                    padding: '1px 6px', borderRadius: T.radii.sm,
                    fontFamily: T.typography.fontMono,
                  }} title='Avg round-trip of last 5 /api/status polls'>
                    {Math.round(latencyMs)}ms
                  </span>
                )}
              </span>
              {!isMobile && <span>PlausiDen AI can make mistakes. Verify important info.</span>}
              <span style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                <span
                  title='Open the command palette'
                  style={{ cursor: 'pointer', color: C.textMuted }}
                  onClick={() => { setShowCmdPalette(true); setCmdQuery(''); setCmdIndex(0); }}>
                  {modKey('K')}
                </span>
                <span style={{ cursor: 'pointer', color: C.textMuted }} onClick={() => { setInput('/'); sm.open(''); inputRef.current?.focus(); }}>
                  / commands
                </span>
                <a href="https://plausiden.com" target="_blank" rel="noopener noreferrer"
                  style={{ color: C.textDim, textDecoration: 'none', fontSize: '10px' }}
                  onMouseEnter={(e) => e.currentTarget.style.color = C.accent}
                  onMouseLeave={(e) => e.currentTarget.style.color = C.textDim}>
                  plausiden.com
                </a>
              </span>
            </div>
          </div>
        </main>

        {/* RIGHT: Telemetry + Admin sidebar (bug #39 from c0-008: user said
            admin/training/data panels were missing). Function renderSidebar
            was defined but never called — orphaned during an earlier refactor.
            Gated on isDesktop so the chat column gets full width on mobile;
            mobile users can reach admin via Cmd+K / Activity modal. */}
        {isDesktop && renderSidebar()}

        {/* RIGHT: Plan / Tasks sidebar — only when the latest assistant turn
            produced a plan, and user hasn't collapsed it. Matches the left
            sidebar's animation pattern. */}
        {(() => {
          // Plan panel is developer-only; regular users don't see reasoning scaffolding.
          if (!settings.developerMode) return null;
          const latestWithPlan = [...messages].reverse().find(m => m.role === 'assistant' && m.plan);
          if (!latestWithPlan || !latestWithPlan.plan) return null;
          const plan = latestWithPlan.plan;
          return (
            <aside aria-label='Plan panel' style={{
              alignSelf: 'stretch',
              background: C.bgCard,
              borderLeft: `1px solid ${C.border}`,
              display: 'flex', flexDirection: 'column', overflow: 'hidden',
              flexShrink: 0,
              ...(isDesktop ? {
                width: showPlanSidebar ? '300px' : '40px',
                transition: 'width 0.22s cubic-bezier(0.4, 0, 0.2, 1)',
              } : {
                width: showPlanSidebar ? '300px' : '0px', maxWidth: '86vw',
                position: 'fixed', top: 0, bottom: 0, right: 0, zIndex: 95,
                transition: 'width 0.22s cubic-bezier(0.4, 0, 0.2, 1)',
                boxShadow: showPlanSidebar ? '-2px 0 20px rgba(0,0,0,0.35)' : 'none',
              }),
            }}>
              {/* Header with collapse toggle */}
              <div style={{
                display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                padding: showPlanSidebar ? '14px' : '10px 6px',
                borderBottom: showPlanSidebar ? `1px solid ${C.borderSubtle}` : 'none',
              }}>
                {showPlanSidebar && (
                  <div style={{ fontSize: T.typography.sizeXs, fontWeight: 800, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.12em' }}>
                    Plan
                  </div>
                )}
                <button onClick={() => setShowPlanSidebar(v => !v)}
                  title={showPlanSidebar ? 'Collapse' : 'Expand'}
                  aria-label={showPlanSidebar ? 'Collapse plan sidebar' : 'Expand plan sidebar'}
                  aria-expanded={showPlanSidebar}
                  style={{
                    width: '28px', height: '28px',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', border: `1px solid ${C.border}`,
                    borderRadius: T.radii.md, color: C.textMuted, cursor: 'pointer', fontFamily: 'inherit',
                    margin: showPlanSidebar ? 0 : '0 auto',
                  }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                    {showPlanSidebar ? <polyline points="9 18 15 12 9 6"/> : <polyline points="15 18 9 12 15 6"/>}
                  </svg>
                </button>
              </div>

              {showPlanSidebar && (
                <div style={{ flex: 1, overflowY: 'auto', padding: '14px' }}>
                  <div style={{ fontSize: T.typography.sizeSm, color: C.text, fontWeight: 600, marginBottom: '4px' }}>
                    {plan.goal?.slice(0, 80) || 'Current plan'}
                  </div>
                  <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginBottom: '14px' }}>
                    {plan.steps} step{plan.steps === 1 ? '' : 's'}
                    {typeof plan.complexity === 'number' && ` \u00B7 complexity ${plan.complexity.toFixed(2)}`}
                  </div>
                  {/* Reuse msg.reasoning as step list if present; otherwise
                      show a numeric placeholder per step count. */}
                  {Array.isArray(latestWithPlan.reasoning) && latestWithPlan.reasoning.length > 0 ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {latestWithPlan.reasoning.map((step, i) => (
                        <div key={i} style={{
                          display: 'flex', gap: T.spacing.sm, padding: '8px 10px',
                          background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                          borderRadius: T.radii.lg, fontSize: '12.5px', color: C.textSecondary, lineHeight: 1.5,
                        }}>
                          <span style={{
                            flexShrink: 0, width: '18px', height: '18px', borderRadius: '50%',
                            background: C.accentBg, color: C.accent, fontSize: '10px',
                            fontWeight: 700, display: 'flex', alignItems: 'center', justifyContent: 'center',
                          }}>{i + 1}</span>
                          <span style={{ flex: 1 }}>{step}</span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div style={{ fontSize: T.typography.sizeSm, color: C.textDim, fontStyle: 'italic' }}>
                      Steps not expanded — enable "Show reasoning" in Settings to see them.
                    </div>
                  )}
                </div>
              )}
            </aside>
          );
        })()}
      </div>

      {/* ========== GLOBAL STYLES ========== */}
      <style>{`
        /* c0-019/020: thinking dots now a subtle scale pulse (0.6→1.0)
           instead of a scale(0)→scale(1) bounce — professional not bouncy. */
        @keyframes scc-bounce {
          0%,80%,100% { transform: scale(0.6); opacity: 0.45; }
          40% { transform: scale(1); opacity: 1; }
        }
        /* c2-358 / task 67: tool-running ring spinner. Used in
           MessageBubble.ToolMessage when toolStatus === 'running'. */
        @keyframes scc-spin {
          to { transform: rotate(360deg); }
        }
        /* c0-020 send-feedback: the input container briefly flashes an
           accent-tinted ring right after a message is sent so the user
           registers the action even if the message list scrolls off. */
        @keyframes lfi-send-pulse {
          0%   { box-shadow: 0 0 0 0 ${C.accent}; }
          60%  { box-shadow: 0 0 0 6px ${C.accentBg}; }
          100% { box-shadow: 0 0 0 0 transparent; }
        }
        .lfi-send-pulse { animation: lfi-send-pulse 0.4s ease-out; }
        /* c2-433 / #298 followup: contradictions-badge rise pulse. Fires
           when a new contradiction lands. Scale-up + color-flash + subtle
           ring, then settles back. Runs twice (animation: ... 2) so it's
           noticeable but not flashy. */
        @keyframes scc-badge-rise-pulse {
          0%   { transform: scale(1);    box-shadow: 0 0 0 0 ${C.red}aa; }
          30%  { transform: scale(1.45); box-shadow: 0 0 0 5px ${C.red}22; }
          60%  { transform: scale(1.1);  box-shadow: 0 0 0 3px ${C.red}11; }
          100% { transform: scale(1);    box-shadow: 0 0 0 0 transparent; }
        }
        @keyframes lfi-fadein {
          0% { opacity: 0; transform: translateY(8px); }
          100% { opacity: 1; transform: translateY(0); }
        }
        /* lfi-glow retained as a no-op (accentGlow is transparent per c0-019
           which disables glow system-wide). Kept so any lingering class refs
           don't throw; can be removed after a sweep confirms no usages. */
        @keyframes lfi-glow {
          0%,100% { opacity: 1; }
        }
        @keyframes lfi-cursor {
          0%,49% { opacity: 1; }
          50%,100% { opacity: 0; }
        }
        @keyframes lfi-typing {
          0%,60%,100% { transform: translateY(0); opacity: 0.35; }
          30% { transform: translateY(-4px); opacity: 1; }
        }
        .lfi-typing-dot {
          display: inline-block; width: 6px; height: 6px; margin: 0 2px;
          background: currentColor; border-radius: 50%;
          animation: lfi-typing 1.1s infinite ease-in-out;
        }
        .lfi-typing-dot:nth-child(2) { animation-delay: 0.18s; }
        .lfi-typing-dot:nth-child(3) { animation-delay: 0.36s; }
        /* Shimmer for skeleton loaders — low-amplitude so it doesn't fight content animations. */
        @keyframes lfi-shimmer {
          0%   { background-position: 100% 50%; }
          100% { background-position: -100% 50%; }
        }
        @media (prefers-reduced-motion: reduce) {
          [style*="lfi-shimmer"], [style*="animation: lfi-shimmer"] { animation: none !important; }
        }
        /* Active training indicator — soft pulsing halo so the dot reads as live. */
        @keyframes lfi-trainer-pulse {
          0%   { box-shadow: 0 0 0 0 ${C.greenBorder}; transform: scale(1); }
          70%  { box-shadow: 0 0 0 10px rgba(0,0,0,0); transform: scale(1.04); }
          100% { box-shadow: 0 0 0 0 rgba(0,0,0,0); transform: scale(1); }
        }
        .lfi-trainer-pulse {
          animation: lfi-trainer-pulse 1.6s infinite ease-out;
          border-radius: 50%;
          display: inline-block;
        }
        /* Smooth progress-bar fill animation */
        .lfi-progress-fill { transition: width 320ms cubic-bezier(0.22, 1, 0.36, 1); }
        /* Respect prefers-reduced-motion (c0-020 E4 a11y): disable all our
           decorative animations when the OS setting is on. Scroll-relevant
           animations (smooth-scroll) stay because they're functional. */
        @media (prefers-reduced-motion: reduce) {
          .lfi-trainer-pulse, .lfi-send-pulse, .lfi-typing-dot,
          [style*="animation: scc-bounce"],
          [style*="animation: scc-pulse"],
          [style*="animation: scc-toast-in"],
          [style*="animation: scc-toast-out"],
          [style*="animation: scc-skel-admin"],
          [style*="animation: scc-skel-cls"],
          [style*="animation: scc-skel"],
          /* c2-279: keep the refresh-button spinners in sync — they were
             added after the initial reduced-motion list was compiled
             (c2-258 admin, c2-259 classroom). */
          [style*="animation: scc-admin-spin"],
          [style*="animation: scc-cls-spin"],
          [style*="animation: lfi-fadein"] {
            animation: none !important;
          }
          /* Keep opacity/color transitions as-is — those are the cheap
             instant ones everyone expects even with reduced-motion. */
        }
        * { box-sizing: border-box; }
        /* c2-285: smooth the OS-driven autoTheme flip (dark <-> light at
           sunset) so users don't get a harsh palette snap. 300ms is slow
           enough to read as intentional, fast enough not to feel laggy. */
        body { margin: 0; padding: 0; overflow: hidden; background: ${C.bg}; color: ${C.text}; transition: background-color 300ms ease, color 300ms ease; }
        html { background: ${C.bg}; transition: background-color 300ms ease; }
        @media (prefers-reduced-motion: reduce) {
          body, html { transition: none; }
        }
        input::placeholder, textarea::placeholder { color: ${C.textDim}; }
        ::-webkit-scrollbar { width: 8px; height: 8px; }
        ::-webkit-scrollbar-track { background: transparent; }
        /* c2-324 / c0-035 #4: drive scrollbar thumb from the theme-resolved
           border tokens so midnight/forest/sunset/rose/contrast themes all
           get palette-appropriate scrollbars — previously anything !== 'light'
           fell through to the dark default rgba, which looked wrong on
           non-canonical themes. */
        ::-webkit-scrollbar-thumb { background: ${C.borderSubtle}; border-radius: 4px; }
        ::-webkit-scrollbar-thumb:hover { background: ${C.border}; }
        .scc-send-btn:hover:not(:disabled) { background: ${C.accentBg} !important; filter: brightness(1.15); border-color: ${C.accentBorder} !important; }
        select option { background: ${C.bgInput}; color: ${C.purple}; }
        button:active { transform: scale(0.97); }
        @media (hover: hover) {
          button:hover { filter: brightness(1.08); }
        }
        @media (hover: none) {
          button:hover { filter: none; }
        }
        /* Push Eruda FAB above our input bar */
        #eruda { z-index: 9999 !important; }
        .eruda-entry-btn { bottom: 80px !important; right: 10px !important; }
        /* Skip link: keep off-screen until focus lands on it, then slide into view. */
        .lfi-skip-link:focus { top: 0 !important; outline: none; box-shadow: ${C.focusRing}; }
        /* c2-411 / BIG #218 mobile: dvh fallback for browsers without support.
           Modern Chrome/Safari/Firefox all have dvh since 2022, so this just
           catches the long tail. @supports ensures we don't double-apply. */
        @supports not (height: 100dvh) {
          .lfi-app-root { height: 100vh !important; }
        }
        /* Tap highlight: disable on interactive elements so tap doesn't flash
           a translucent grey box on mobile Safari / Chrome. */
        button, a, [role="button"], [role="tab"], [role="option"], [role="menuitem"] {
          -webkit-tap-highlight-color: transparent;
        }
        /* c2-411 / BIG #218 mobile: icon-only buttons have inline width/
           height of 28–36px on desktop. On coarse pointers (touch) we force
           a 44x44 minimum so WCAG 2.1 §2.5.5 is met. Chat input toolbar +
           sidebar action icons were the worst offenders. Scope to actual
           icon buttons via a marker class so we don't inflate text buttons
           that are already 40px+ tall. */
        @media (pointer: coarse) {
          .lfi-icon-btn, button[aria-label]:not(.lfi-text-btn) {
            min-width: 44px !important;
            min-height: 44px !important;
          }
          /* iOS Safari zooms the viewport when a focused input has a
             font-size < 16px. Force the minimum on inputs / textareas /
             selects so focus doesn't teleport the layout. */
          input, textarea, select {
            font-size: max(16px, 1em);
          }
          /* c2-412 / BIG #218 mobile: decorative keyboard shortcut chips
             (⌘K next to a button label) are noise on touch — you can't
             chord with a thumb. Marker class makes them opt-in; keeps
             the ShortcutsModal content (which is informational) visible. */
          kbd.lfi-shortcut-chip { display: none !important; }
          /* c2-432 mobile: hide the webkit scrollbar on overflow:auto rows
             (chat input action bar, admin/classroom tab bars). The chrome
             looks dated on touch; scrolling still works via the finger. */
          ::-webkit-scrollbar { display: none; }
          /* Notch / dynamic-island safe area: the app-root sits under
             whatever OS chrome the page is drawn behind (viewport-fit=cover
             in index.html). Add the top inset so the header doesn't hide
             under a notch. Left/right handled via margin-auto + max-width. */
          .lfi-app-root { padding-top: env(safe-area-inset-top, 0px); }
        }
        /* c2-417: reverted the blanket full-screen-modal rule from c2-415.
           The Command Palette is a top-anchored popover, not a
           modal-experience surface — stretching it to 100dvh left a mostly-
           blank screen with a search input at the top. Each modal now
           keeps its existing mobile sizing (Settings, Admin, Knowledge,
           Activity already size sensibly via max-width:100% + dvh). */
        /* c0-020 E4 a11y: visible focus ring on any interactive element
           reached by keyboard. Mouse clicks suppress this because we use
           :focus-visible, which is WCAG 2.1 AA compliant.
           c2-383 / BIG #178: switched to C.focusRing (from design-system
           task 53) so CONTRAST theme gets its 3px yellow ring and each
           bespoke palette (forest/sunset/rose/midnight) picks up its
           theme-appropriate accent instead of a hardcoded 2px blue. */
        button:focus-visible, a:focus-visible, [role="button"]:focus-visible,
        [role="tab"]:focus-visible, [role="option"]:focus-visible,
        [role="tabpanel"]:focus-visible, [role="menuitem"]:focus-visible,
        [role="checkbox"]:focus-visible, [role="radio"]:focus-visible,
        [tabindex]:focus-visible {
          outline: none;
          box-shadow: ${C.focusRing};
          border-radius: 4px;
        }
        input:focus-visible, textarea:focus-visible, select:focus-visible {
          /* Inputs already have their own border-focus style; reinforce with
             the theme's focusRing so it's visible against any background. */
          outline: none;
          box-shadow: ${C.focusRing};
        }
      `}</style>
    </div>
    </React.Suspense>
  );
};

// Wrap in an error boundary so render-time exceptions surface a helpful recovery
// page instead of a white screen. Theme fallback uses the dark palette defaults
// since the boundary renders above the theme context.
const AppRoot: React.FC = () => (
  <AppErrorBoundary themeBg={DARK.bg} themeText={DARK.text} themeAccent={DARK.accent}>
    <SovereignCommandConsole />
  </AppErrorBoundary>
);

export default AppRoot;
