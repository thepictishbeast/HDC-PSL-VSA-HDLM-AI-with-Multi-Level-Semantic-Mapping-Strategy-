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

import React, { useState, useEffect, useRef, useCallback } from 'react';
import hljs from 'highlight.js/lib/core';
import rust from 'highlight.js/lib/languages/rust';
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import python from 'highlight.js/lib/languages/python';
import bash from 'highlight.js/lib/languages/bash';
import json_lang from 'highlight.js/lib/languages/json';
import sql from 'highlight.js/lib/languages/sql';
import css from 'highlight.js/lib/languages/css';
import xml from 'highlight.js/lib/languages/xml';
import go from 'highlight.js/lib/languages/go';
import 'highlight.js/styles/github-dark.css';
import { compactNum, formatRam, formatTime, copyToClipboard, diskPressure, smartTitle, exportConversationMd } from './util';
import { TrainingDashboardContent } from './TrainingDashboard';
import { AppErrorBoundary } from './AppErrorBoundary';
import { LoginScreen } from './LoginScreen';
import { SKILLS, AVATAR_PRESETS, type Skill as CatalogSkill } from './catalogs';
import { SystemMessage, WebMessage, ToolMessage, UserMessage, AssistantMessage } from './MessageBubble';
// Code-splitting: the overlays below are only rendered on user action, so we
// load their code on demand. Cuts the initial JS bundle by ~1000 lines of TSX.
import { type CmdPaletteItem } from './CommandPalette';
import { DARK, THEMES } from './themes';
import { WelcomeScreen } from './WelcomeScreen';
import { FactsPanel } from './FactsPanel';
import { QosPanel } from './QosPanel';
import { TelemetryCard } from './TelemetryCards';
import { SidebarStatus } from './SidebarStatus';
import { SubstrateTelemetry } from './SubstrateTelemetry';
import { AdminActions } from './AdminActions';
import { renderMessageBody as renderMdBody, type MarkdownCtx } from './markdown';
import { useTicTacToe } from './useTicTacToe';
import { useStatusPoll, useQualityPoll, useSysInfoPoll } from './usePolls';
import { useAutoScroll } from './useAutoScroll';
import { ChatView } from './ChatView';

const TicTacToeModal = React.lazy(() => import('./TicTacToeModal').then(m => ({ default: m.TicTacToeModal })));
const KnowledgeBrowser = React.lazy(() => import('./KnowledgeBrowser').then(m => ({ default: m.KnowledgeBrowser })));
const ActivityModal = React.lazy(() => import('./ActivityModal').then(m => ({ default: m.ActivityModal })));
const CommandPalette = React.lazy(() => import('./CommandPalette').then(m => ({ default: m.CommandPalette })));
const SettingsModal = React.lazy(() => import('./SettingsModal').then(m => ({ default: m.SettingsModal })));

hljs.registerLanguage('rust', rust);
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('js', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('ts', typescript);
hljs.registerLanguage('python', python);
hljs.registerLanguage('py', python);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('sh', bash);
hljs.registerLanguage('json', json_lang);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('css', css);
hljs.registerLanguage('html', xml);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('go', go);

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
  const [isConnected, setIsConnected] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const [thinkingStart, setThinkingStart] = useState<number | null>(null);
  const [thinkingStep, setThinkingStep] = useState<string>('');
  const [thinkingElapsed, setThinkingElapsed] = useState<number>(0);
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

  // Persistent settings (localStorage-backed). A single object keeps storage
  // compact and makes future additions one-line.
  type Settings = {
    theme: 'dark' | 'light' | 'midnight' | 'forest' | 'sunset' | 'contrast' | 'rose';
    fontSize: 'small' | 'medium' | 'large';
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
  const [settingsTab, setSettingsTab] = useState<'profile' | 'appearance' | 'behavior' | 'data'>('profile');

  // Active skill/tool for the next message (like Perplexity Focus, Gemini Extensions,
  // Claude Code tool routing). Real backends wired: chat (WS), web (api/search),
  // analyze (api/audit), opsec (api/opsec/scan). Image/research/code stubbed until
  // backend support lands; clicking the chip makes that clear.
  type Skill = CatalogSkill;
  const [activeSkill, setActiveSkill] = useState<Skill>('chat');
  const [showSkillMenu, setShowSkillMenu] = useState(false);
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [slashFilter, setSlashFilter] = useState('');
  const [slashIndex, setSlashIndex] = useState(0);

  type SlashCmd = { cmd: string; label: string; desc: string; run: () => void };
  const slashCommands: SlashCmd[] = [
    { cmd: '/new', label: 'New chat', desc: 'Start a fresh conversation',
      run: () => createNewConversation() },
    { cmd: '/clear', label: 'Clear chat', desc: 'Erase current messages',
      run: () => clearChat() },
    { cmd: '/theme', label: 'Toggle theme', desc: 'Switch dark / light',
      run: () => setSettings(s => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' })) },
    { cmd: '/settings', label: 'Open settings', desc: 'All preferences',
      run: () => setShowSettings(true) },
    { cmd: '/logs', label: 'Activity logs', desc: 'Chat log + UI events',
      run: () => { setShowActivity(true); fetchChatLog(50); } },
    { cmd: '/pulse', label: 'Model: Pulse', desc: 'Fast tier',
      run: () => handleTierSwitch('Pulse') },
    { cmd: '/bridge', label: 'Model: Bridge', desc: 'Balanced tier',
      run: () => handleTierSwitch('Bridge') },
    { cmd: '/bigbrain', label: 'Model: BigBrain', desc: 'Deepest reasoning',
      run: () => handleTierSwitch('BigBrain') },
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
    { cmd: '/help', label: 'Help & docs', desc: 'Commands, shortcuts, tips, and feedback guide',
      run: () => {
        const cmdList = slashCommands.filter(c => c.cmd !== '/help').map(c => `  ${c.cmd.padEnd(14)} ${c.desc}`).join('\n');
        const help = `**PlausiDen AI — Quick Reference**

**Slash commands** (type / in the input):
${cmdList}

**Keyboard shortcuts:**
  Ctrl+K          Command palette (search everything)
  Ctrl+N          New conversation
  Ctrl+D          Toggle developer mode
  Ctrl+,          Open settings
  Ctrl+E          Focus input
  Ctrl+Shift+K    Knowledge browser
  Esc             Close any modal

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
  };
  const [showKnowledge, setShowKnowledge] = useState(false);
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
  const [editingMsgId, setEditingMsgId] = useState<number | null>(null);
  const [editText, setEditText] = useState('');
  const [knowledgeFacts, setKnowledgeFacts] = useState<Array<{ key: string; value: string }>>([]);
  const [knowledgeConcepts, setKnowledgeConcepts] = useState<Array<{ name: string; mastery: number; review_count: number }>>([]);
  const [knowledgeDue, setKnowledgeDue] = useState<Array<{ name: string; mastery: number; days_overdue: number }>>([]);
  const fetchKnowledge = async () => {
    const host = getHost();
    try {
      const [f, c, d] = await Promise.all([
        fetch(`http://${host}:3000/api/facts`).then(r => r.json()),
        fetch(`http://${host}:3000/api/knowledge/concepts`).then(r => r.json()),
        fetch(`http://${host}:3000/api/knowledge/due`).then(r => r.json()),
      ]);
      setKnowledgeFacts(f.facts || []);
      setKnowledgeConcepts(c.concepts || []);
      setKnowledgeDue(d.due || []);
    } catch (e) { console.warn('knowledge fetch failed', e); }
  };
  // Tic-tac-toe state
  const { board: tttBoard, winner: tttWinner, play: tttPlay, reset: tttReset } = useTicTacToe();
  const [cmdQuery, setCmdQuery] = useState('');
  const [cmdIndex, setCmdIndex] = useState(0);
  const skills = SKILLS;
  const activeSkillMeta = skills.find(s => s.id === activeSkill) || skills[0];
  const [showHistory, setShowHistory] = useState(false);
  const [showActivity, setShowActivity] = useState(false);

  const avatarPresets = AVATAR_PRESETS;
  const [showAccountMenu, setShowAccountMenu] = useState(false);
  const accountMenuRef = useRef<HTMLDivElement>(null);
  const [serverChatLog, setServerChatLog] = useState<any[]>([]);
  const [activityTab, setActivityTab] = useState<'chat' | 'events' | 'system'>('chat');
  const [localEvents, setLocalEvents] = useState<Array<{ t: number; kind: string; data?: any }>>([]);

  const fontScale = settings.compactMode ? 0.85 : (settings.fontSize === 'small' ? 0.88 : settings.fontSize === 'large' ? 1.15 : 1.0);
  // Shadow the module-scope C with a theme-bound palette, plus any custom overrides.
  const baseTheme = THEMES[settings.theme] || DARK;
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
      console.debug('// SCC: event', kind, data);
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
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // ---- Helpers ----
  const getHost = () => {
    if (settings.apiHost && settings.apiHost.trim()) return settings.apiHost.trim();
    const h = window.location.hostname || '127.0.0.1';
    console.debug("// SCC: Resolved host:", h);
    return h;
  };

  const scrollToBottom = useAutoScroll(messagesEndRef, messages.length);

  // Tick elapsed seconds on the thinking indicator once per second while active.
  useEffect(() => {
    if (!isThinking || thinkingStart == null) { setThinkingElapsed(0); return; }
    setThinkingElapsed(0);
    const id = setInterval(() => {
      setThinkingElapsed(Math.floor((Date.now() - thinkingStart) / 1000));
    }, 1000);
    return () => clearInterval(id);
  }, [isThinking, thinkingStart]);

  useEffect(() => {
    console.debug("// SCC: Persisting auth:", isAuthenticated);
    localStorage.setItem('lfi_auth', isAuthenticated.toString());
  }, [isAuthenticated]);

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
    const RECONNECT_MAX_MS = 30000;

    const connect = () => {
      console.debug("// SCC: chat WS connect()");
      const ws = new WebSocket(wsUrl);
      chatWsRef.current = ws;

      ws.onopen = () => {
        console.debug("// SCC: Chat WS OPEN");
        setIsConnected(true);
        reconnectDelayMs = 1000; // reset backoff after healthy connect
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          console.debug("// SCC: Chat msg:", msg.type);

          if (msg.type === 'progress') {
            setThinkingStep(msg.step || 'Processing...');
          } else if (msg.type === 'chat_chunk') {
            // Streaming: append partial text to the last assistant message,
            // or create one if this is the first chunk.
            setIsThinking(false);
            setMessages(prev => {
              const last = prev[prev.length - 1];
              if (last && last.role === 'assistant' && last._streaming) {
                return [...prev.slice(0, -1), { ...last, content: last.content + (msg.text || '') }];
              }
              return [...prev, {
                id: msgId(), role: 'assistant' as const,
                content: msg.text || '', timestamp: Date.now(),
                _streaming: true,
              } as any];
            });
          } else if (msg.type === 'chat_done') {
            // End of streaming — finalize the message.
            setMessages(prev => {
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
            setMessages(prev => [...prev, {
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
            setMessages(prev => [...prev, {
              id: msgId(), role: 'web',
              content: `${msg.source_count} sources | trust: ${(msg.trust * 100).toFixed(0)}%\n\n${msg.summary}`,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'chat_error') {
            console.debug("// SCC: Chat error:", msg.error);
            setIsThinking(false);
            setMessages(prev => [...prev, {
              id: msgId(), role: 'system',
              content: `Error: ${msg.error}`, timestamp: Date.now(),
            }]);
          }
        } catch (e) {
          console.error("// SCC: Chat parse error:", e);
        }
      };

      ws.onclose = (ev) => {
        console.debug("// SCC: Chat WS CLOSED:", ev.code, 'reconnect in', reconnectDelayMs, 'ms');
        setIsConnected(false);
        // Add 0-500ms jitter so a fleet of reconnecting clients doesn't stampede.
        const jitter = Math.floor(Math.random() * 500);
        reconnectTimer = setTimeout(connect, reconnectDelayMs + jitter);
        reconnectDelayMs = Math.min(reconnectDelayMs * 2, RECONNECT_MAX_MS);
      };

      ws.onerror = (ev) => {
        console.error("// SCC: Chat WS ERROR:", ev);
        setIsConnected(false);
      };
    };

    connect();
    return () => { clearTimeout(reconnectTimer); chatWsRef.current?.close(); };
  }, [isAuthenticated]);

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
    setMessages([]);
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
      const mod = e.metaKey || e.ctrlKey;
      const k = e.key.toLowerCase();

      if (mod && k === 'k') { e.preventDefault(); setShowCmdPalette(v => !v); setCmdQuery(''); setCmdIndex(0); }
      else if (mod && k === 'n') { e.preventDefault(); createNewConversation(); }
      else if (mod && k === 'd') { e.preventDefault(); setSettings(s => ({ ...s, developerMode: !s.developerMode })); }
      else if (mod && k === ',') { e.preventDefault(); setShowSettings(true); }
      else if (mod && k === 'e') { e.preventDefault(); inputRef.current?.focus(); }
      else if (mod && k === '/') { e.preventDefault(); inputRef.current?.focus(); }
      else if (mod && e.shiftKey && k === 'k') { e.preventDefault(); setShowKnowledge(true); fetchKnowledge(); }
      else if (mod && e.shiftKey && k === 'd') { e.preventDefault(); const themes: Array<typeof settings.theme> = ['dark','light','midnight','forest','sunset','rose','contrast']; const idx = themes.indexOf(settings.theme); setSettings(s => ({...s, theme: themes[(idx+1) % themes.length]})); }
      else if (mod && k === 'b') { e.preventDefault(); setShowConvoSidebar(v => !v); }
      else if (e.key === 'Escape') {
        if (showCmdPalette) setShowCmdPalette(false);
        else if (showSettings) setShowSettings(false);
        else if (showKnowledge) setShowKnowledge(false);
        else if (showActivity) setShowActivity(false);
        else if (showGame) setShowGame(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showCmdPalette, showSettings, showKnowledge, showActivity, showGame]);

  // Three polling hooks — see ./usePolls.ts for the fetch logic. Each manages
  // its own interval + abort handling; parent just reads the state they return.
  const host = getHost();
  const { kg, lastOk: kgLastOk } = useStatusPoll(host, isAuthenticated);
  const quality = useQualityPoll(host, isAuthenticated);
  const sysInfo = useSysInfoPoll(host, isAuthenticated);

  // ---- Conversations (Claude/ChatGPT/Gemini-style sidebar state) ----
  type Conversation = {
    id: string;
    title: string;
    messages: ChatMessage[];
    createdAt: number;
    updatedAt: number;
    pinned?: boolean;
    starred?: boolean;
    incognito?: boolean;
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

  // Ensure we always have an active conversation to write into.
  useEffect(() => {
    if (!currentConversationId || !conversations.find(c => c.id === currentConversationId)) {
      if (conversations.length > 0) {
        setCurrentConversationId(conversations[0].id);
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
  useEffect(() => {
    if (!settings.persistConversations) return;
    try {
      const saveable = conversations.filter(c => !c.incognito).slice(-100).map(c => ({
        ...c, messages: c.messages.slice(-500),
      }));
      localStorage.setItem(LS_CONVERSATIONS_KEY, JSON.stringify(saveable));
    } catch { /* quota exceeded */ }
  }, [conversations, settings.persistConversations]);
  useEffect(() => {
    if (!currentConversationId) return;
    try { localStorage.setItem(LS_CURRENT_KEY, currentConversationId); } catch {}
  }, [currentConversationId]);

  // Keep the browser tab title in sync with the active conversation — makes
  // tab-switching to the dashboard scannable among many browser tabs.
  useEffect(() => {
    const c = conversations.find(x => x.id === currentConversationId);
    const title = c?.title && c.title !== 'New chat' ? c.title.slice(0, 60) : null;
    document.title = title ? `${title} · PlausiDen AI` : 'PlausiDen AI';
    return () => { document.title = 'PlausiDen AI'; };
  }, [currentConversationId, conversations]);

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
    if (incognito) {
      setMessages([{
        id: msgId(), role: 'system',
        content: 'Incognito mode — this conversation will not be saved, logged, or used for training.',
        timestamp: Date.now(),
      }]);
    }
  };
  const isCurrentIncognito = (() => {
    const c = conversations.find(c => c.id === currentConversationId);
    return c?.incognito || false;
  })();
  const deleteConversation = (id: string) => {
    setConversations(prev => prev.filter(c => c.id !== id));
    if (id === currentConversationId) {
      const rest = conversations.filter(c => c.id !== id);
      setCurrentConversationId(rest[0]?.id || '');
    }
  };
  const renameConversation = (id: string, title: string) => {
    const clean = title.trim().slice(0, 80) || 'Untitled';
    setConversations(prev => prev.map(c => c.id === id ? { ...c, title: clean } : c));
  };
  const togglePinned = (id: string) => setConversations(prev =>
    prev.map(c => c.id === id ? { ...c, pinned: !c.pinned } : c));
  const toggleStarred = (id: string) => setConversations(prev =>
    prev.map(c => c.id === id ? { ...c, starred: !c.starred } : c));

  // Smart auto-title: look at the first user turn + first assistant reply,
  // pick a short key-phrase that beats simple truncation. Falls back to
  // titleFrom if no signal. Rule-of-thumb similar to ChatGPT/Gemini heuristics.
  const [showConvoSidebar, setShowConvoSidebar] = useState<boolean>(true);
  const [showPlanSidebar, setShowPlanSidebar] = useState<boolean>(true);
  const [convoSearch, setConvoSearch] = useState('');

  // ---- Send ----
  // Routes the message through the active skill. Chat/code go over the WS;
  // web/analyze/opsec hit REST endpoints and render results inline without
  // disturbing the conversation flow.
  const handleSend = async () => {
    const trimmed = input.trim();
    console.debug("// SCC: handleSend, len:", trimmed.length, "skill:", activeSkill);
    if (!trimmed) return;

    // Record user message.
    setMessages(prev => [...prev, {
      id: msgId(), role: 'user', content: trimmed, timestamp: Date.now()
    }]);
    setInput('');
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
        setMessages(prev => [...prev, {
          id: msgId(), role: 'system',
          content: 'Not connected yet \u2014 give the link a moment and try again.',
          timestamp: Date.now(),
        }]);
        setIsThinking(false);
        return;
      }
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
      inputRef.current?.focus();
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 160) + 'px';
    // Slash command detection: show menu when "/" is at position 0.
    if (val.startsWith('/') && !val.includes(' ')) {
      setShowSlashMenu(true);
      setSlashFilter(val.slice(1).toLowerCase());
      setSlashIndex(0);
    } else {
      setShowSlashMenu(false);
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

  // Markdown renderer lives in ./markdown.tsx; we build a ctx each render so the
  // current theme key + copy-handler flow through. Cheap — just a tiny object.
  const mdCtx: MarkdownCtx = {
    C, themeKey: settings.theme,
    onCopy: copyToClipboard,
    onCopyEvent: (lang, length) => logEvent('code_copied', { lang, length }),
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
    return (
      <LoginScreen
        C={C} isMobile={isMobile} isDesktop={isDesktop}
        password={password} setPassword={setPassword}
        authError={authError} authLoading={authLoading}
        onLogin={handleLogin}
      />
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
    <aside style={{
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
      </AdminActions>
    </aside>
  );

  return (
    <React.Suspense fallback={null}>
    <div style={{
      display: 'flex', flexDirection: 'column', height: '100vh', width: '100%',
      background: C.bg, color: C.text,
      fontFamily: C.font,
      overflow: 'hidden',
      fontSize: `${fontScale}em`,
    }}>
      {/* ========== TOOL CONFIRMATION DIALOG ========== */}
      {pendingConfirm && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 260,
          background: 'rgba(0,0,0,0.55)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '16px',
        }}>
          <div style={{
            width: '100%', maxWidth: '440px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
            padding: '24px', boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
          }}>
            <h3 style={{ margin: '0 0 8px', fontSize: '16px', fontWeight: 700, color: C.text }}>
              {pendingConfirm.tool} requires approval
            </h3>
            <p style={{ margin: '0 0 18px', fontSize: '13px', color: C.textSecondary, lineHeight: 1.6 }}>
              {pendingConfirm.desc}
            </p>
            <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              <button onClick={() => { setPendingConfirm(null); setIsThinking(false); }}
                style={{
                  padding: '10px 18px', background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit', fontSize: '13px',
                }}>Cancel</button>
              <button onClick={pendingConfirm.onApprove}
                style={{
                  padding: '10px 18px', background: C.accent, border: 'none',
                  color: '#fff', borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: '13px', fontWeight: 600,
                }}>Allow</button>
            </div>
          </div>
        </div>
      )}

      {/* ========== TERMS OF SERVICE (first run, before welcome) ========== */}
      {!tosAccepted && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 260,
          background: C.bg,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          padding: '16px',
        }}>
          <div style={{
            width: '100%', maxWidth: '560px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '16px',
            padding: isMobile ? '24px' : '36px',
            boxShadow: '0 32px 80px rgba(0,0,0,0.5)',
          }}>
            <h1 style={{ margin: '0 0 8px', fontSize: '20px', fontWeight: 700, color: C.text }}>
              PlausiDen <span style={{ color: C.accent }}>AI</span> — Terms of Use
            </h1>
            <p style={{ margin: '0 0 16px', fontSize: '13px', color: C.textMuted }}>
              Please review before continuing.
            </p>
            <div style={{
              maxHeight: '300px', overflowY: 'auto',
              padding: '16px', background: C.bgInput, borderRadius: '10px',
              fontSize: '13px', lineHeight: 1.7, color: C.textSecondary,
              marginBottom: '20px',
            }}>
              <p><strong>1. Sovereignty.</strong> PlausiDen AI runs entirely on your hardware. Your conversations, knowledge, and data never leave your machine unless you explicitly initiate it (e.g., web search, file export).</p>
              <p><strong>2. Privacy.</strong> No telemetry, analytics, or usage data is collected or transmitted. Diagnostics are local-only and off by default.</p>
              <p><strong>3. Data Ownership.</strong> Everything you create, teach, or store in PlausiDen AI belongs to you. PlausiDen Technologies LLC makes no claim to your data.</p>
              <p><strong>4. AI Limitations.</strong> PlausiDen AI can make mistakes. Verify important information independently. The AI's responses are not professional advice (legal, medical, financial, etc.).</p>
              <p><strong>5. Security.</strong> While we follow defense-in-depth practices (encrypted storage, PSL governance, provenance tracking), no system is perfectly secure. You are responsible for the security of your deployment environment.</p>
              <p><strong>6. Open Source.</strong> PlausiDen AI's core is open source. You may audit, modify, and redistribute the code under its license terms.</p>
              <p><strong>7. No Warranty.</strong> PlausiDen AI is provided as-is. PlausiDen Technologies LLC is not liable for any damages arising from its use.</p>
              <p style={{ marginTop: '12px', fontSize: '11px', color: C.textDim }}>
                PlausiDen Technologies LLC &middot; <a href="https://plausiden.com" target="_blank" rel="noopener noreferrer" style={{ color: C.accent }}>plausiden.com</a>
              </p>
            </div>
            <button onClick={() => {
              setTosAccepted(true);
              try { localStorage.setItem('lfi_tos_accepted', 'true'); } catch {}
              logEvent('tos_accepted', { version: '1.0' });
            }}
              style={{
                width: '100%', padding: '14px',
                background: C.accent, border: 'none',
                borderRadius: '10px', color: '#fff',
                fontSize: '15px', fontWeight: 700,
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
          position: 'fixed', inset: 0, zIndex: 250,
          background: 'rgba(0,0,0,0.70)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          padding: '16px',
        }}>
          <div style={{
            width: '100%', maxWidth: '520px',
            background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '16px',
            padding: isMobile ? '24px' : '36px',
            boxShadow: '0 32px 80px rgba(0,0,0,0.5)',
            textAlign: 'center',
          }}>
            <pre style={{
              margin: '0 auto 18px', color: C.accent,
              fontSize: '32px', fontWeight: 800, letterSpacing: '0.04em',
              textShadow: `0 0 24px ${C.accentGlow}`,
            }}>
            PlausiDen <span style={{ opacity: 0.7 }}>AI</span>
            </pre>
            <h1 style={{ margin: '0 0 6px', fontSize: '22px', fontWeight: 700, color: C.text }}>
              Welcome to PlausiDen <span style={{ color: C.accent }}>AI</span>
            </h1>
            <p style={{ margin: '0 0 24px', fontSize: '14px', color: C.textMuted, lineHeight: 1.6 }}>
              Sovereign AI that runs on your hardware. Private by default. Gets smarter over time.
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', marginBottom: '24px', textAlign: 'left' }}>
              {[
                { icon: '\u2328', title: 'Ctrl+K', desc: 'Command palette — search everything' },
                { icon: '/', title: '/commands', desc: 'Type / for slash commands' },
                { icon: '+', title: 'Tools', desc: 'Web search, code, analyze, OPSEC' },
                { icon: '\u{1F512}', title: 'Private', desc: 'Data stays on your machine' },
                { icon: '\u{1F9E0}', title: 'Learns', desc: 'Remembers facts across sessions' },
                { icon: '\u{1F3A8}', title: '7 Themes', desc: 'Settings \u2192 Appearance' },
              ].map((item, i) => (
                <div key={i} style={{
                  padding: '10px 12px', background: C.bgInput,
                  border: `1px solid ${C.borderSubtle}`, borderRadius: '8px',
                  display: 'flex', gap: '10px', alignItems: 'flex-start',
                }}>
                  <span style={{ fontSize: '18px', flexShrink: 0 }}>{item.icon}</span>
                  <div>
                    <div style={{ fontSize: '13px', fontWeight: 600, color: C.text }}>{item.title}</div>
                    <div style={{ fontSize: '11px', color: C.textDim }}>{item.desc}</div>
                  </div>
                </div>
              ))}
            </div>

            <button onClick={dismissWelcome}
              style={{
                width: '100%', padding: '14px',
                background: C.accent, border: 'none',
                borderRadius: '10px', color: '#fff',
                fontSize: '15px', fontWeight: 700,
                cursor: 'pointer', fontFamily: 'inherit',
              }}>
              Get started
            </button>
            <p style={{ margin: '12px 0 0', fontSize: '11px', color: C.textDim }}>
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
            display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '16px',
          }}>
          <div onClick={(e) => e.stopPropagation()}
            style={{
              width: '100%', maxWidth: '750px', height: '80vh',
              background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
              display: 'flex', flexDirection: 'column', overflow: 'hidden',
              boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
            }}>
            <div style={{
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
            }}>
              <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 700, color: C.text }}>Training Dashboard</h2>
              <button onClick={() => setShowTraining(false)}
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
          onClose={() => setShowKnowledge(false)}
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

      {/* ========== COMMAND PALETTE (Cmd+K) ========== */}
      {showCmdPalette && (() => {
        const items: CmdPaletteItem[] = [
          { id: 'new-chat', label: 'New chat', hint: 'Start a fresh conversation', group: 'Actions',
            onRun: () => { createNewConversation(); } },
          { id: 'clear-chat', label: 'Clear current chat', hint: 'Erase this conversation\'s messages', group: 'Actions',
            onRun: () => { clearChat(); } },
          { id: 'toggle-sidebar', label: showConvoSidebar ? 'Hide sidebar' : 'Show sidebar', hint: 'Toggle conversations panel', group: 'Actions',
            onRun: () => { setShowConvoSidebar(v => !v); } },
          { id: 'toggle-theme', label: `Switch to ${settings.theme === 'dark' ? 'light' : 'dark'} theme`, hint: 'Flip appearance', group: 'Appearance',
            onRun: () => { setSettings(s => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' })); } },
          ...(['dark','light','midnight','forest','sunset','rose','contrast'] as const).map(t => ({
            id: `theme-${t}`, label: `Theme: ${t}`, hint: 'Apply this color scheme', group: 'Appearance',
            onRun: () => setSettings(s => ({ ...s, theme: t })),
          })),
          ...(['Pulse','Bridge','BigBrain']).map(tier => ({
            id: `tier-${tier}`, label: `Model: ${tier}`, hint: tier === 'Pulse' ? 'Fast' : tier === 'Bridge' ? 'Balanced' : 'Deepest', group: 'Model',
            onRun: () => { handleTierSwitch(tier); },
          })),
          ...skills.filter(s => s.available).map(s => ({
            id: `skill-${s.id}`, label: `Use ${s.label}`, hint: s.hint, group: 'Skills',
            onRun: () => { setActiveSkill(s.id); inputRef.current?.focus(); },
          })),
          { id: 'open-settings', label: 'Open settings', hint: 'All preferences', group: 'Navigate',
            onRun: () => { setShowSettings(true); } },
          { id: 'open-knowledge', label: 'Knowledge browser', hint: 'Facts, concepts, reviews', group: 'Navigate',
            onRun: () => { setShowKnowledge(true); fetchKnowledge(); } },
          { id: 'open-logs', label: 'Open activity logs', hint: 'Chat log + UI events', group: 'Navigate',
            onRun: () => { setShowActivity(true); fetchChatLog(50); } },
          { id: 'toggle-dev', label: `${settings.developerMode ? 'Disable' : 'Enable'} developer mode`, hint: 'Telemetry + plan panel', group: 'Navigate',
            onRun: () => { setSettings(s => ({ ...s, developerMode: !s.developerMode })); } },
          ...conversations.slice(0, 20).map(c => ({
            id: `convo-${c.id}`, label: c.title, hint: `${c.messages.length} message${c.messages.length === 1 ? '' : 's'}`, group: 'Conversations',
            onRun: () => { setCurrentConversationId(c.id); },
          })),
        ];
        return (
          <CommandPalette
            C={C} isMobile={isMobile}
            items={items}
            query={cmdQuery} setQuery={setCmdQuery}
            index={cmdIndex} setIndex={setCmdIndex}
            onClose={() => setShowCmdPalette(false)}
            onItemRun={(id) => logEvent('cmd_palette_run', { id })}
          />
        );
      })()}

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
          localEvents={localEvents}
          isConnected={isConnected}
          currentTier={currentTier}
          thermalThrottled={stats.is_throttled}
          ramLabel={`${ramFmt.value} ${ramFmt.unit}`}
          cpuTempC={stats.cpu_temp_c}
          factsLabel={`${kg.facts}`}
          conceptsLabel={`${kg.concepts}`}
          logicDensity={stats.logic_density}
          qosReport={qosReport}
          onRefreshQos={fetchQos}
          onRefreshFacts={fetchFacts}
        />
      )}

      {/* ========== SETTINGS MODAL ========== */}
      {showSettings && (
        <SettingsModal
          C={C} isMobile={isMobile}
          settings={settings as any}
          setSettings={setSettings as any}
          tab={settingsTab}
          onTabChange={(t) => setSettingsTab(t)}
          onClose={() => setShowSettings(false)}
          currentTier={currentTier}
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
            } catch (e) { console.warn(e); }
          }}
          onClearHistory={() => {
            if (confirm('Clear all saved conversations from this device?')) {
              localStorage.removeItem(LS_MESSAGES_KEY);
              localStorage.removeItem(LS_CONVERSATIONS_KEY);
              setConversations([]); setMessages([]);
            }
          }}
          onResetSettings={() => {
            if (confirm('Reset all settings to defaults?')) setSettings(defaultSettings);
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
      <header style={{
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
            style={{
              width: '36px', height: '36px', flexShrink: 0,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: showConvoSidebar ? C.accentBg : 'transparent',
              border: `1px solid ${showConvoSidebar ? C.accentBorder : C.border}`,
              borderRadius: '8px',
              color: showConvoSidebar ? C.accent : C.textMuted,
              cursor: 'pointer', fontFamily: 'inherit',
            }}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="4" width="18" height="16" rx="2"/>
              <line x1="9" y1="4" x2="9" y2="20"/>
            </svg>
          </button>
          <div style={{ fontSize: '13px', fontWeight: 800, letterSpacing: '0.02em', color: C.text, display: 'flex', alignItems: 'center', gap: '6px' }}>
            PlausiDen <span style={{ color: C.accent }}>AI</span>
            {/* Per Bible §4.5: subtle shield icon when PlausiDen/incognito mode
                is active. No text label — just the icon. */}
            {isCurrentIncognito && (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={C.accent} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" title="Incognito mode active">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
            )}
          </div>
          {/* Inline stats — developer-only per design review. */}
          {isDesktop && settings.developerMode && (
            <div style={{ display: 'flex', gap: '16px', marginLeft: '8px', fontSize: '12px', color: C.textDim }}>
              <span title={`Used ${ramUsedFmt.value} ${ramUsedFmt.unit} of ${ramTotalFmt.value} ${ramTotalFmt.unit} total`}>
                {ramTotal > 0 ? `${ramUsedFmt.value}/${ramTotalFmt.value} ${ramTotalFmt.unit}` : `${ramFmt.value} ${ramFmt.unit}`}
              </span>
              <span>{stats.cpu_temp_c.toFixed(0)}{'\u00B0'}C</span>
              <span style={{ color: tierColor(currentTier) }}>{currentTier}</span>
            </div>
          )}
        </div>

        {/* Right: account on the far right. `order: 3` in the flex header
            pushes it past the tier/theme cluster regardless of DOM order. */}
        <div style={{ position: 'relative', order: 3 }} ref={accountMenuRef}>
          <button onClick={() => setShowAccountMenu(v => !v)}
            title='Account'
            style={{
              display: 'flex', alignItems: 'center', gap: '10px',
              padding: '4px 10px 4px 4px',
              background: showAccountMenu ? C.bgHover : 'transparent',
              border: `1px solid ${showAccountMenu ? C.border : 'transparent'}`,
              borderRadius: '10px', cursor: 'pointer', fontFamily: 'inherit',
            }}>
            {/* Avatar */}
            <div style={{
              width: '30px', height: '30px', borderRadius: '50%',
              background: settings.avatarDataUrl ? `url(${settings.avatarDataUrl}) center/cover` : (settings.avatarGradient || `linear-gradient(135deg, ${C.accent}, ${C.purple})`),
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              flexShrink: 0, fontSize: '13px', fontWeight: 800, color: '#fff',
              boxShadow: `0 0 0 1px ${C.border}`,
            }}>
              {!settings.avatarDataUrl && (settings.displayName.trim().charAt(0).toUpperCase() || 'U')}
            </div>
            {!isMobile && (
              <div style={{ textAlign: 'left', lineHeight: 1.15 }}>
                <div style={{ fontSize: '13px', fontWeight: 700, color: C.text, maxWidth: '140px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {settings.displayName || 'Account'}
                </div>
                <div style={{
                  fontSize: '10px', color: isConnected ? C.green : C.red,
                  fontWeight: 700, letterSpacing: '0.04em', marginTop: '2px',
                }}>
                  {isConnected ? 'Online' : 'Offline'}
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
                borderRadius: '12px', padding: '10px',
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
                      style={{
                        width: '100%', background: 'transparent', border: 'none', outline: 'none',
                        fontSize: '14px', fontWeight: 700, color: C.text, fontFamily: 'inherit',
                        padding: 0,
                      }} />
                    <div style={{ fontSize: '11px', color: C.textMuted, marginTop: '2px' }}>
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
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
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
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
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
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
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
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
                    onMouseEnter={(e) => e.currentTarget.style.background = C.bgHover}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <circle cx="12" cy="12" r="3"/>
                      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                    </svg>
                    Settings
                  </button>
                  <button onClick={() => { setShowAccountMenu(false); setShowActivity(true);
                      fetchChatLog(50);
                    }}
                    style={{ display: 'flex', alignItems: 'center', gap: '10px',
                      padding: '10px 12px', background: 'transparent', border: 'none', cursor: 'pointer',
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
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
                      color: C.text, fontSize: '13px', fontFamily: 'inherit', textAlign: 'left', borderRadius: '8px' }}
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
            selector "it snaps back to default" bug the user was hitting. */}
        <div style={{ display: 'flex', alignItems: 'center', gap: isMobile ? '6px' : '10px', order: 2, marginLeft: 'auto' }}>
          {/* Stats toggle (mobile/tablet) */}
          {!isDesktop && (
            <button onClick={() => setShowTelemetry(!showTelemetry)} style={{
              padding: '5px 10px', fontSize: '11px', fontWeight: 700,
              background: showTelemetry ? C.accentBg : 'transparent',
              border: `1px solid ${showTelemetry ? C.accentBorder : C.border}`, borderRadius: '8px',
              color: showTelemetry ? C.accent : C.textMuted,
              cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
            }}>Stats</button>
          )}

          {/* Admin toggle (mobile/tablet) */}
          {!isDesktop && (
            <button onClick={() => setShowAdmin(!showAdmin)} style={{
              padding: '5px 10px', fontSize: '11px', fontWeight: 700,
              background: showAdmin ? C.purpleBg : 'transparent',
              border: `1px solid ${showAdmin ? C.purpleBorder : C.border}`, borderRadius: '8px',
              color: showAdmin ? C.purple : C.textMuted,
              cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
            }}>Admin</button>
          )}

          {/* Theme toggle removed — accessible via account menu, Cmd+K palette,
              and Settings → Appearance. Keeping the header slim. */}
        </div>
      </header>

      {/* ========== TELEMETRY PANEL (mobile/tablet, collapsible) ========== */}
      {!isDesktop && showTelemetry && (
        <div style={{
          display: 'grid', gridTemplateColumns: isTablet ? 'repeat(4, 1fr)' : 'repeat(2, 1fr)',
          gap: '8px', padding: '12px 14px', background: C.bgCard,
          borderBottom: `1px solid ${C.border}`, flexShrink: 0,
        }}>
          {telemetryCards.map(s => renderTelemetryCard(s))}
          {stats.is_throttled && (
            <div style={{
              gridColumn: '1 / -1', padding: '10px', background: C.redBg,
              border: `1px solid ${C.redBorder}`, borderRadius: '8px',
              textAlign: 'center', fontSize: '12px', fontWeight: 800, color: C.red, textTransform: 'uppercase',
            }}>Thermal Throttle Active</div>
          )}
        </div>
      )}

      {/* ========== ADMIN PANEL (mobile/tablet, collapsible) ========== */}
      {!isDesktop && showAdmin && (
        <div style={{
          padding: '14px', background: C.bgCard,
          borderBottom: `1px solid ${C.border}`, flexShrink: 0,
        }}>
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button onClick={fetchFacts} disabled={adminLoading === 'facts'} style={{
              padding: '8px 14px', fontSize: '11px', fontWeight: 700, color: C.accent,
              background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '8px',
              cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
            }}>{adminLoading === 'facts' ? 'Loading...' : 'Facts'}</button>
            <button onClick={fetchQos} disabled={adminLoading === 'qos'} style={{
              padding: '8px 14px', fontSize: '11px', fontWeight: 700, color: C.purple,
              background: C.purpleBg, border: `1px solid ${C.purpleBorder}`, borderRadius: '8px',
              cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
            }}>{adminLoading === 'qos' ? 'Loading...' : 'QoS'}</button>
            <button onClick={clearChat} style={{
              padding: '8px 14px', fontSize: '11px', fontWeight: 700, color: C.textMuted,
              background: 'transparent', border: `1px solid ${C.border}`, borderRadius: '8px',
              cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
            }}>Clear Chat</button>
          </div>
          {/* Inline results */}
          {facts.length > 0 && (
            <div style={{ marginTop: '10px', maxHeight: '150px', overflowY: 'auto', fontSize: '11px' }}>
              {facts.map((f, i) => (
                <div key={i} style={{ padding: '4px 0', borderBottom: `1px solid ${C.borderSubtle}` }}>
                  <span style={{ color: C.accent, fontWeight: 700 }}>{f.key}</span>
                  <span style={{ color: C.textDim }}> = </span>
                  <span style={{ color: C.textSecondary }}>{f.value}</span>
                </div>
              ))}
            </div>
          )}
          {qosReport && (
            <pre style={{ marginTop: '10px', fontSize: '10px', color: C.textMuted, whiteSpace: 'pre-wrap', maxHeight: '150px', overflowY: 'auto' }}>
              {JSON.stringify(qosReport, null, 2).slice(0, 400)}
            </pre>
          )}
        </div>
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
        <aside style={{
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
              width: '320px', maxWidth: '86vw',
              position: 'fixed', top: 0, bottom: 0, left: 0, zIndex: 100,
              transform: showConvoSidebar ? 'translateX(0)' : 'translateX(-105%)',
              transition: 'transform 0.22s cubic-bezier(0.4, 0, 0.2, 1)',
              boxShadow: showConvoSidebar ? '2px 0 24px rgba(0,0,0,0.45)' : 'none',
            }),
          }}>
            <div style={{ padding: '10px 14px', borderBottom: `1px solid ${C.borderSubtle}` }}>
              <button onClick={() => createNewConversation()}
                style={{
                  width: '100%', padding: '8px 12px', marginBottom: '8px',
                  background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                  color: C.accent, borderRadius: '8px',
                  fontSize: '13px', fontWeight: 700, cursor: 'pointer',
                  fontFamily: 'inherit', display: 'flex',
                  alignItems: 'center', justifyContent: 'center', gap: '6px',
                }}>
                <span style={{ fontSize: '14px' }}>{'\u002B'}</span> New chat
              </button>
              <input
                type='text'
                value={convoSearch}
                onChange={(e) => setConvoSearch(e.target.value)}
                placeholder='Search conversations...'
                style={{
                  width: '100%', padding: '8px 10px',
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  borderRadius: '8px', outline: 'none',
                  color: C.text, fontFamily: 'inherit', fontSize: '12px',
                  boxSizing: 'border-box',
                }}
                onFocus={(e) => e.currentTarget.style.borderColor = C.accent}
                onBlur={(e) => e.currentTarget.style.borderColor = C.borderSubtle}
              />
            </div>
            <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
              {conversations.length === 0 && (
                <div style={{ padding: '16px', textAlign: 'center', color: C.textMuted, fontSize: '12px' }}>
                  No conversations yet.
                </div>
              )}
              {conversations
                .filter(c => {
                  if (!convoSearch.trim()) return true;
                  const q = convoSearch.toLowerCase();
                  if (c.title.toLowerCase().includes(q)) return true;
                  return c.messages.some(m => m.content.toLowerCase().includes(q));
                })
                .sort((a, b) => {
                  // Pinned first (most-recent pinned at top), then the rest
                  // by most-recent activity. Starred is orthogonal, shown via
                  // an icon but doesn't affect order.
                  if (!!a.pinned !== !!b.pinned) return a.pinned ? -1 : 1;
                  return b.updatedAt - a.updatedAt;
                })
                .map(c => {
                  const isActive = c.id === currentConversationId;
                  return (
                    <div key={c.id}
                      onClick={() => setCurrentConversationId(c.id)}
                      style={{
                        padding: '10px 12px', borderRadius: '8px', cursor: 'pointer',
                        background: isActive ? C.accentBg : 'transparent',
                        border: `1px solid ${isActive ? C.accentBorder : 'transparent'}`,
                        marginBottom: '4px', display: 'flex',
                        alignItems: 'center', justifyContent: 'space-between', gap: '4px',
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
                          fontSize: '13px', fontWeight: 600,
                          color: isActive ? C.accent : C.text,
                          whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                          display: 'flex', alignItems: 'center', gap: '6px',
                        }}>
                          {c.pinned && <span style={{ color: C.yellow, fontSize: '11px' }}>{'\u{1F4CC}'}</span>}
                          {c.starred && <span style={{ color: C.yellow, fontSize: '11px' }}>{'\u2605'}</span>}
                          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{c.title}</span>
                        </div>
                        <div style={{ fontSize: '10px', color: C.textDim, marginTop: '2px' }}>
                          {c.messages.length} msg &middot; {new Date(c.updatedAt).toLocaleDateString([], { month: 'short', day: 'numeric' })}
                        </div>
                      </div>
                      {/* Action icons — hover-only per design review. Uses
                          CSS class toggled by the parent's onMouseEnter/Leave.
                          Star stays visible when active for discoverability. */}
                      <div className='convo-actions'
                        style={{
                          display: 'flex', gap: '2px',
                          opacity: isActive ? 0.7 : 0,
                          transition: 'opacity 0.12s',
                        }}>
                        <button onClick={(e) => { e.stopPropagation(); toggleStarred(c.id); }}
                          title={c.starred ? 'Unstar' : 'Star'}
                          style={{
                            background: 'transparent', border: 'none',
                            color: c.starred ? C.yellow : C.textDim,
                            cursor: 'pointer', fontSize: '12px', padding: '2px 3px',
                          }}>{c.starred ? '\u2605' : '\u2606'}</button>
                        <button onClick={(e) => { e.stopPropagation(); togglePinned(c.id); }}
                          title={c.pinned ? 'Unpin' : 'Pin'}
                          style={{
                            background: 'transparent', border: 'none',
                            color: c.pinned ? C.yellow : C.textDim,
                            cursor: 'pointer', fontSize: '11px', padding: '2px 3px',
                          }}>{'\u{1F4CC}'}</button>
                        <button onClick={(e) => {
                          e.stopPropagation();
                          const next = prompt('Rename conversation', c.title);
                          if (next !== null) renameConversation(c.id, next);
                        }} title='Rename'
                          style={{
                            background: 'transparent', border: 'none', color: C.textDim,
                            cursor: 'pointer', fontSize: '10px', padding: '2px 3px',
                          }}>{'\u270E'}</button>
                        <button onClick={(e) => {
                          e.stopPropagation();
                          exportConversationMd(c);
                          logEvent('conversation_exported_md', { id: c.id });
                        }} title='Export as Markdown'
                          style={{
                            background: 'transparent', border: 'none', color: C.textDim,
                            cursor: 'pointer', fontSize: '10px', padding: '2px 3px',
                          }}>{'\u2B07'}</button>
                        <button onClick={(e) => {
                          e.stopPropagation();
                          if (confirm(`Delete "${c.title}"?`)) deleteConversation(c.id);
                        }} title='Delete'
                          style={{
                            background: 'transparent', border: 'none', color: C.textDim,
                            cursor: 'pointer', fontSize: '11px', padding: '2px 3px',
                          }}>{'\u2715'}</button>
                      </div>
                    </div>
                  );
                })}
            </div>
            {/* Sidebar footer — minimal by default. Telemetry + host info
                only surface when Developer Mode is on, per 2026-04-15 design
                review (avoid "internal tool" vibes for general users). */}
            <div style={{
              padding: '12px', borderTop: `1px solid ${C.borderSubtle}`, fontSize: '11px',
            }}>
              {settings.developerMode && (
                <>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
                    {telemetryCards.map(card => (
                      <div key={card.label} style={{
                        padding: '8px 10px', borderRadius: '8px',
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
            </div>
          </aside>

        {/* CHAT AREA — now a flex column so the input bar lives inside main
            and centers within the *available* width (shifts with the sidebar)
            instead of the viewport. */}
        <main style={{
          flex: 1, display: 'flex', flexDirection: 'column',
          overflow: 'hidden', minWidth: 0,
        }}>
          <ChatView
            messages={messages}
            chatMaxWidth={chatMaxWidth}
            chatPadding={chatPadding}
            isDesktop={isDesktop}
            renderEmpty={() => (
              <WelcomeScreen
                C={C} isDesktop={isDesktop}
                onPickPrompt={(p) => { setInput(p); inputRef.current?.focus(); }}
              />
            )}
            renderFooter={() => (
              <>
                {isThinking && (
                  <div role="status" aria-live="polite" style={{
                    display: 'flex', alignItems: 'center', gap: '12px',
                    padding: '12px 16px', margin: '8px 0',
                    background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                    borderRadius: '10px', fontSize: '13px',
                  }}>
                    <div style={{ display: 'flex', gap: '5px', alignItems: 'center' }}>
                      {[0, 1, 2].map(i => (
                        <div key={i} style={{
                          width: '7px', height: '7px', background: C.accent, borderRadius: '50%',
                          animation: 'scc-bounce 1.4s infinite ease-in-out',
                          animationDelay: `${i * 0.16}s`,
                        }} />
                      ))}
                    </div>
                    <span style={{ color: C.text, fontWeight: 500 }}>{thinkingStep || 'Thinking'}</span>
                    <span style={{ color: C.textDim, fontSize: '11px', fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' }}>
                      {Math.floor(thinkingElapsed / 60) > 0 ? `${Math.floor(thinkingElapsed / 60)}m ` : ''}{thinkingElapsed % 60}s
                    </span>
                    <button onClick={() => {
                      setIsThinking(false);
                      setThinkingStart(null);
                      fetch(`http://${getHost()}:3000/api/stop`, { method: 'POST' }).catch(() => {});
                      logEvent('chat_stop', { elapsed: thinkingElapsed });
                    }} style={{
                      marginLeft: 'auto', padding: '4px 12px', fontSize: '12px',
                      background: 'transparent', border: `1px solid ${C.border}`,
                      color: C.textMuted, borderRadius: '6px', cursor: 'pointer',
                      fontFamily: 'inherit',
                    }}>Stop</button>
                  </div>
                )}
                <div ref={messagesEndRef} />
              </>
            )}
            renderMessage={(msg) => (
              <>
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
                {msg.role === 'user' && (
                  <UserMessage
                    msg={msg} C={C} isMobile={isMobile}
                    maxWidth={userBubbleMaxWidth}
                    editing={editingMsgId === msg.id}
                    editText={editText} setEditText={setEditText}
                    onBeginEdit={() => { setEditingMsgId(msg.id); setEditText(msg.content); }}
                    onCancelEdit={() => setEditingMsgId(null)}
                    onCommitEdit={(trimmed) => {
                      const idx = messages.findIndex(m => m.id === msg.id);
                      if (idx >= 0) setMessages(prev => prev.slice(0, idx));
                      setEditingMsgId(null);
                      setInput(trimmed);
                      setTimeout(() => handleSend(), 50);
                      logEvent('message_edited', { originalLen: msg.content.length, newLen: trimmed.length });
                    }}
                    formatTime={formatTime}
                  />
                )}
                {msg.role === 'assistant' && (
                  <AssistantMessage
                    msg={msg} C={C} isMobile={isMobile} isDesktop={isDesktop}
                    isLast={messages[messages.length - 1]?.id === msg.id}
                    isThinking={isThinking}
                    showReasoning={!!settings.showReasoning}
                    developerMode={!!settings.developerMode}
                    reasoningExpanded={expandedReasoning === msg.id}
                    renderBody={(text) => renderMessageBody(text)}
                    onToggleReasoning={() => setExpandedReasoning(expandedReasoning === msg.id ? null : msg.id)}
                    onRegenerate={regenerateLast}
                    onCopy={copyToClipboard}
                    onOpenProvenance={(cid) => {
                      fetch(`http://${getHost()}:3000/api/provenance/${cid}`)
                        .then(r => r.json())
                        .then(d => {
                          setMessages(prev => [...prev, {
                            id: msgId(), role: 'system',
                            content: `Provenance #${cid}:\n${d.explanation || JSON.stringify(d, null, 2).slice(0, 500)}`,
                            timestamp: Date.now(),
                          }]);
                        }).catch(() => {});
                    }}
                    onFollowUpChip={(chip) => { setInput(chip); inputRef.current?.focus(); }}
                    onFeedbackPositive={() => { logEvent('feedback_positive', { msgId: msg.id }); }}
                    onFeedbackNegative={() => {
                      const feedback = prompt('What should the AI have said instead? (optional)');
                      logEvent('feedback_negative', { msgId: msg.id, feedback: feedback || '' });
                    }}
                    formatTime={formatTime}
                  />
                )}
              </>
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
                    borderRadius: '12px', padding: '6px',
                    boxShadow: '0 -12px 40px rgba(0,0,0,0.35)',
                    animation: 'lfi-fadein 0.12s ease-out', zIndex: 50,
                  }}>
                    <div style={{ padding: '6px 10px', fontSize: '10px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em' }}>
                      Commands
                    </div>
                    {filtered.map((c, i) => (
                      <button key={c.cmd}
                        onClick={() => { c.run(); setInput(''); setShowSlashMenu(false); logEvent('slash_cmd', { cmd: c.cmd }); }}
                        onMouseEnter={() => setSlashIndex(i)}
                        style={{
                          width: '100%', textAlign: 'left', cursor: 'pointer',
                          padding: '8px 12px', background: i === clamped ? C.accentBg : 'transparent',
                          border: 'none', borderRadius: '8px', fontFamily: 'inherit',
                          color: C.text, display: 'flex', alignItems: 'center', gap: '12px',
                        }}>
                        <span style={{ fontSize: '13px', fontWeight: 700, color: i === clamped ? C.accent : C.textSecondary, minWidth: '90px',
                          fontFamily: "'JetBrains Mono','Fira Code',monospace" }}>{c.cmd}</span>
                        <span style={{ fontSize: '13px', color: C.textMuted }}>{c.desc}</span>
                      </button>
                    ))}
                  </div>
                );
              })()}

              <div style={{
                background: C.bgCard,
                border: `1px solid ${input ? C.borderFocus : C.border}`,
                borderRadius: '16px',
                transition: 'border-color 0.2s, box-shadow 0.2s',
                boxShadow: input ? `0 0 0 4px ${C.accentGlow}` : `0 2px 18px rgba(0,0,0,0.12)`,
                display: 'flex', flexDirection: 'column',
              }}>
              <textarea
                ref={inputRef}
                aria-label='Chat message input'
                value={input}
                onChange={handleInputChange}
                onKeyDown={(e) => {
                  // Slash menu keyboard nav.
                  if (showSlashMenu) {
                    const filtered = slashCommands.filter(c =>
                      !slashFilter || c.cmd.slice(1).startsWith(slashFilter) || c.label.toLowerCase().includes(slashFilter)
                    );
                    if (e.key === 'ArrowDown') { e.preventDefault(); setSlashIndex(i => Math.min(i + 1, filtered.length - 1)); return; }
                    if (e.key === 'ArrowUp') { e.preventDefault(); setSlashIndex(i => Math.max(i - 1, 0)); return; }
                    if (e.key === 'Enter' || e.key === 'Tab') {
                      e.preventDefault();
                      const picked = filtered[Math.min(slashIndex, filtered.length - 1)];
                      if (picked) { picked.run(); setInput(''); setShowSlashMenu(false); logEvent('slash_cmd', { cmd: picked.cmd }); }
                      return;
                    }
                    if (e.key === 'Escape') { setShowSlashMenu(false); return; }
                  }
                  if (!settings.sendOnEnter) return;
                  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
                }}
                placeholder={settings.sendOnEnter ? 'Message PlausiDen AI' : 'Message PlausiDen AI (click send when ready)'}
                style={{
                  background: 'transparent', border: 'none', outline: 'none',
                  resize: 'none', fontSize: '15.5px', lineHeight: '1.55',
                  padding: '18px 20px 10px',
                  color: C.text, fontFamily: 'inherit',
                  minHeight: '72px', maxHeight: '280px',
                }}
                rows={2}
              />
              <div style={{
                display: 'flex', alignItems: 'center', gap: '6px',
                padding: '6px 10px 10px', position: 'relative',
              }}>
                {/* Skills "+" button — opens popover with all skills. Cleaner
                    than a wide scrolling row when you have 7+ tools. */}
                <div style={{ position: 'relative', flexShrink: 0 }}>
                  <button onClick={() => setShowSkillMenu(v => !v)}
                    title='Tools &amp; skills'
                    style={{
                      width: '36px', height: '36px', cursor: 'pointer',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      background: activeSkill !== 'chat' ? C.accentBg : (showSkillMenu ? C.bgHover : 'transparent'),
                      border: `1px solid ${activeSkill !== 'chat' ? C.accentBorder : 'transparent'}`,
                      color: activeSkill !== 'chat' ? C.accent : C.textMuted,
                      borderRadius: '8px',
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
                        borderRadius: '12px', padding: '6px',
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
                                borderRadius: '8px', fontFamily: 'inherit', textAlign: 'left',
                                opacity: s.available ? 1 : 0.55,
                              }}
                              onMouseEnter={(e) => { if (s.available && !picked) e.currentTarget.style.background = C.bgHover; }}
                              onMouseLeave={(e) => { if (!picked) e.currentTarget.style.background = 'transparent'; }}>
                              {s.icon}
                              <div style={{ flex: 1, minWidth: 0 }}>
                                <div style={{ fontSize: '13px', fontWeight: 600 }}>
                                  {s.label}{!s.available && <span style={{ fontSize: '10px', marginLeft: '6px', color: C.textDim }}>soon</span>}
                                </div>
                                <div style={{ fontSize: '11px', color: C.textDim, marginTop: '2px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
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
                <label title='Attach file'
                  style={{
                    width: '36px', height: '36px', cursor: 'pointer',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', color: C.textMuted,
                    borderRadius: '8px', flexShrink: 0,
                  }}>
                  <input type='file' multiple style={{ display: 'none' }}
                    onChange={(e) => {
                      const files = Array.from(e.target.files || []);
                      if (files.length === 0) return;
                      const names = files.map(f => f.name).join(', ');
                      setMessages(prev => [...prev, {
                        id: msgId(), role: 'system',
                        content: `Attached: ${names} (${files.length} file${files.length === 1 ? '' : 's'}). Upload backend is not yet wired \u2014 names logged for now.`,
                        timestamp: Date.now(),
                      }]);
                      logEvent('file_attached', { count: files.length, names });
                      e.target.value = '';
                    }} />
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.49"/>
                  </svg>
                </label>
                {/* Voice */}
                <button title='Voice input'
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
                    rec.start();
                    logEvent('voice_started', {});
                  }}
                  style={{
                    width: '36px', height: '36px', cursor: 'pointer',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', color: C.textMuted, border: 'none',
                    borderRadius: '8px', flexShrink: 0,
                  }}>
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                    <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                    <line x1="12" y1="19" x2="12" y2="23"/>
                    <line x1="8" y1="23" x2="16" y2="23"/>
                  </svg>
                </button>
                {/* Model selector — replaces the header tier dropdown, right
                    where ChatGPT/Gemini put theirs. Labels user-friendly. */}
                <select value={currentTier} disabled={tierSwitching}
                  onChange={(e) => handleTierSwitch(e.target.value)}
                  title='Model'
                  style={{
                    padding: '7px 28px 7px 12px', fontSize: '13px', fontWeight: 600,
                    background: C.bgInput, color: C.text,
                    border: `1px solid ${C.border}`, borderRadius: '8px',
                    cursor: tierSwitching ? 'wait' : 'pointer', fontFamily: 'inherit',
                    appearance: 'none', WebkitAppearance: 'none',
                    backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%237f8296' d='M0 2l4 4 4-4z'/%3E%3C/svg%3E")`,
                    backgroundRepeat: 'no-repeat', backgroundPosition: 'right 10px center',
                  }}>
                  <option value="Pulse">LFI Pulse &middot; fast</option>
                  <option value="Bridge">LFI Bridge &middot; balanced</option>
                  <option value="BigBrain">LFI BigBrain &middot; deepest</option>
                </select>
                <div style={{ flex: 1 }} />
                {/* Active-skill chip: visible when non-default so the user
                    always knows which pipeline their next send will use. */}
                {activeSkill !== 'chat' && (
                  <button onClick={() => setActiveSkill('chat')}
                    title='Clear active skill'
                    style={{
                      display: 'flex', alignItems: 'center', gap: '6px',
                      padding: '5px 10px', fontSize: '11.5px', fontWeight: 600,
                      background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                      color: C.accent, borderRadius: '999px',
                      cursor: 'pointer', fontFamily: 'inherit',
                    }}>
                    {activeSkillMeta.icon}
                    <span>{activeSkillMeta.label}</span>
                    <span style={{ opacity: 0.7, fontSize: '10px', marginLeft: '2px' }}>{'\u2715'}</span>
                  </button>
                )}
                <span style={{ fontSize: '11px', color: C.textDim, paddingRight: '4px' }}>
                  {input.length > 0 ? `${input.length} chars` : ''}
                </span>
                {/* Send */}
                <button
                  onClick={handleSend}
                  disabled={!input.trim() || !isConnected}
                  className="scc-send-btn"
                  title='Send (Enter)'
                  aria-label='Send message'
                  style={{
                    width: '36px', height: '36px',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: input.trim() && isConnected ? C.accent : C.bgInput,
                    border: `1px solid ${input.trim() && isConnected ? C.accent : C.border}`,
                    borderRadius: '8px',
                    color: input.trim() && isConnected ? (settings.theme === 'light' ? '#fff' : '#000') : C.textDim,
                    cursor: input.trim() && isConnected ? 'pointer' : 'default',
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
              <span style={{ color: isConnected ? C.green : C.red, fontWeight: 700 }}>
                {isConnected ? 'Link active' : 'Reconnecting...'}
              </span>
              <span>PlausiDen AI can make mistakes. Verify important info.</span>
              <span style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                <span
                  title='Open the command palette'
                  style={{ cursor: 'pointer', color: C.textMuted }}
                  onClick={() => { setShowCmdPalette(true); setCmdQuery(''); setCmdIndex(0); }}>
                  {navigator.platform.toLowerCase().includes('mac') ? '\u2318K' : 'Ctrl+K'}
                </span>
                <span style={{ cursor: 'pointer', color: C.textMuted }} onClick={() => { setInput('/'); setShowSlashMenu(true); setSlashFilter(''); inputRef.current?.focus(); }}>
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
            <aside style={{
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
                  <div style={{ fontSize: '11px', fontWeight: 800, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.12em' }}>
                    Plan
                  </div>
                )}
                <button onClick={() => setShowPlanSidebar(v => !v)}
                  title={showPlanSidebar ? 'Collapse' : 'Expand'}
                  style={{
                    width: '28px', height: '28px',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent', border: `1px solid ${C.border}`,
                    borderRadius: '6px', color: C.textMuted, cursor: 'pointer', fontFamily: 'inherit',
                    margin: showPlanSidebar ? 0 : '0 auto',
                  }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                    {showPlanSidebar ? <polyline points="9 18 15 12 9 6"/> : <polyline points="15 18 9 12 15 6"/>}
                  </svg>
                </button>
              </div>

              {showPlanSidebar && (
                <div style={{ flex: 1, overflowY: 'auto', padding: '14px' }}>
                  <div style={{ fontSize: '12px', color: C.text, fontWeight: 600, marginBottom: '4px' }}>
                    {plan.goal?.slice(0, 80) || 'Current plan'}
                  </div>
                  <div style={{ fontSize: '11px', color: C.textDim, marginBottom: '14px' }}>
                    {plan.steps} step{plan.steps === 1 ? '' : 's'}
                    {typeof plan.complexity === 'number' && ` \u00B7 complexity ${plan.complexity.toFixed(2)}`}
                  </div>
                  {/* Reuse msg.reasoning as step list if present; otherwise
                      show a numeric placeholder per step count. */}
                  {Array.isArray(latestWithPlan.reasoning) && latestWithPlan.reasoning.length > 0 ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {latestWithPlan.reasoning.map((step, i) => (
                        <div key={i} style={{
                          display: 'flex', gap: '8px', padding: '8px 10px',
                          background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                          borderRadius: '8px', fontSize: '12.5px', color: C.textSecondary, lineHeight: 1.5,
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
                    <div style={{ fontSize: '12px', color: C.textDim, fontStyle: 'italic' }}>
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
        @keyframes scc-bounce {
          0%,80%,100% { transform: scale(0); opacity: 0.5; }
          40% { transform: scale(1); opacity: 1; }
        }
        @keyframes lfi-fadein {
          0% { opacity: 0; transform: translateY(8px); }
          100% { opacity: 1; transform: translateY(0); }
        }
        @keyframes lfi-glow {
          0%,100% { text-shadow: 0 0 12px ${C.accentGlow}; }
          50% { text-shadow: 0 0 24px ${C.accentGlow}, 0 0 4px ${C.accent}; }
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
        /* Respect prefers-reduced-motion: switch pulse to a static tint */
        @media (prefers-reduced-motion: reduce) {
          .lfi-trainer-pulse { animation: none; }
        }
        * { box-sizing: border-box; }
        body { margin: 0; padding: 0; overflow: hidden; background: ${C.bg}; color: ${C.text}; }
        html { background: ${C.bg}; }
        input::placeholder, textarea::placeholder { color: ${C.textDim}; }
        ::-webkit-scrollbar { width: 8px; height: 8px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: ${settings.theme === 'light' ? 'rgba(20,30,60,0.15)' : 'rgba(255,255,255,0.10)'}; border-radius: 4px; }
        ::-webkit-scrollbar-thumb:hover { background: ${settings.theme === 'light' ? 'rgba(20,30,60,0.28)' : 'rgba(255,255,255,0.18)'}; }
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
