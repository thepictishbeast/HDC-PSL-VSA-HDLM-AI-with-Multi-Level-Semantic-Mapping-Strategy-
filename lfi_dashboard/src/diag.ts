/**
 * diag.ts — structured diagnostic logging for runtime issues.
 *
 * Goal: when something breaks on a user's device, we have enough context
 * to debug without asking them to reproduce. Three channels:
 *
 *   1. Explicit: `diag.info('foo', {...})`, `diag.error(...)` — fires from code.
 *   2. Auto-capture: window.onerror + unhandledrejection are routed here.
 *   3. Console hijack: `console.warn` / `console.error` from our code is
 *      mirrored into the diag ring buffer (native console still fires too).
 *
 * Storage: in-memory ring buffer (last 500 entries) + localStorage mirror
 * of the last 200 (post-reload survival). Exceeding quota is silently
 * dropped — never blocks the app.
 *
 * Export: `diag.export()` returns the buffer as JSON for copy-to-clipboard
 * + a Cmd+K entry that surfaces it.
 *
 * Wire it from App.tsx ONCE on mount:
 *   import { diag } from './diag'; useEffect(() => diag.install(), []);
 */

export type DiagLevel = 'debug' | 'info' | 'warn' | 'error';

export interface DiagEntry {
  ts: number;                      // epoch ms
  level: DiagLevel;
  source: string;                  // free-form tag, e.g. 'ws' / 'popover' / 'auto'
  message: string;                 // human-readable summary
  data?: unknown;                  // structured payload (shallow-stringified on write)
  stack?: string;                  // when level=error
}

const LS_KEY = 'lfi_diag_v1';
const RING_CAP = 500;              // in-memory cap
const LS_CAP = 200;                // localStorage cap
const WARN_QUOTA_PERSIST = 80;     // only persist when level ≥ info; debug stays in-memory

let buf: DiagEntry[] = [];
let installed = false;
let originalWarn: typeof console.warn | null = null;
let originalError: typeof console.error | null = null;
const listeners = new Set<(e: DiagEntry) => void>();

// Deep-ish JSON-safe clone. Circular refs → '[circular]'; functions dropped.
const safeClone = (x: unknown, depth = 3): unknown => {
  if (x == null || typeof x !== 'object') return x;
  if (depth <= 0) return '[…]';
  if (x instanceof Error) return { _error: true, name: x.name, message: x.message, stack: x.stack };
  try {
    // Fast path: JSON-round-trip.
    return JSON.parse(JSON.stringify(x, (_k, v) => (typeof v === 'function' ? '[fn]' : v)));
  } catch {
    // Fallback: shallow copy with truncation.
    const out: Record<string, unknown> = {};
    try {
      for (const k of Object.keys(x as Record<string, unknown>)) {
        const v = (x as Record<string, unknown>)[k];
        out[k] = typeof v === 'object' ? '[unserializable]' : v;
      }
    } catch { /* ignore */ }
    return out;
  }
};

const push = (e: DiagEntry) => {
  buf.push(e);
  if (buf.length > RING_CAP) buf.splice(0, buf.length - RING_CAP);
  // Persist only at info+ to avoid churning localStorage with debug noise.
  const levelRank = { debug: 0, info: 1, warn: 2, error: 3 }[e.level];
  if (levelRank >= 1) persist();
  for (const fn of listeners) {
    try { fn(e); } catch { /* ignore listener errors */ }
  }
};

let persistTimer: number | null = null;
const persist = () => {
  if (persistTimer != null) return;
  persistTimer = (typeof window !== 'undefined' ? window.setTimeout(() => {
    persistTimer = null;
    try {
      const slice = buf.slice(-LS_CAP);
      localStorage.setItem(LS_KEY, JSON.stringify(slice));
    } catch { /* quota / private-mode — silent */ }
  }, 300) : null);
};

const hydrate = () => {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      buf = parsed.filter(e => e && typeof e.ts === 'number' && typeof e.level === 'string');
    }
  } catch { /* silent */ }
};

const emit = (level: DiagLevel, source: string, message: string, data?: unknown) => {
  const e: DiagEntry = {
    ts: Date.now(),
    level,
    source,
    message: String(message).slice(0, 500),
    data: data === undefined ? undefined : safeClone(data),
  };
  if (level === 'error' && data instanceof Error) e.stack = data.stack;
  push(e);
};

export const diag = {
  debug: (source: string, message: string, data?: unknown) => emit('debug', source, message, data),
  info:  (source: string, message: string, data?: unknown) => emit('info',  source, message, data),
  warn:  (source: string, message: string, data?: unknown) => emit('warn',  source, message, data),
  error: (source: string, message: string, data?: unknown) => emit('error', source, message, data),

  /** Return a snapshot copy of the ring buffer. */
  snapshot(): DiagEntry[] {
    return buf.slice();
  },

  /** Export as pretty JSON for clipboard. Includes runtime metadata. */
  export(): string {
    return JSON.stringify({
      exported_at: new Date().toISOString(),
      user_agent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
      viewport: typeof window !== 'undefined' ? { w: window.innerWidth, h: window.innerHeight } : null,
      counts: {
        debug: buf.filter(e => e.level === 'debug').length,
        info:  buf.filter(e => e.level === 'info').length,
        warn:  buf.filter(e => e.level === 'warn').length,
        error: buf.filter(e => e.level === 'error').length,
        total: buf.length,
      },
      entries: buf,
    }, null, 2);
  },

  /** Zero out the buffer + localStorage. */
  clear() {
    buf = [];
    try { localStorage.removeItem(LS_KEY); } catch { /* silent */ }
  },

  /** Subscribe to new entries. Returns an unsubscribe fn. */
  subscribe(fn: (e: DiagEntry) => void): () => void {
    listeners.add(fn);
    return () => { listeners.delete(fn); };
  },

  /**
   * One-shot setup: hijack console.warn/error for capture, install
   * window.onerror + unhandledrejection handlers, hydrate from
   * localStorage. Idempotent — safe to call twice.
   */
  install() {
    if (installed || typeof window === 'undefined') return;
    installed = true;
    hydrate();
    // Hijack console.warn/error. Preserve originals so the devtools
    // panel still shows the raw message + source map.
    originalWarn = console.warn;
    originalError = console.error;
    console.warn = (...args: unknown[]) => {
      try {
        emit('warn', 'console', args.map(a => typeof a === 'string' ? a : JSON.stringify(safeClone(a))).join(' '), args.length > 0 && typeof args[0] === 'object' ? args[0] : undefined);
      } catch { /* don't let diag break logging */ }
      originalWarn!.apply(console, args);
    };
    console.error = (...args: unknown[]) => {
      try {
        emit('error', 'console', args.map(a => typeof a === 'string' ? a : JSON.stringify(safeClone(a))).join(' '), args.length > 0 && typeof args[0] === 'object' ? args[0] : undefined);
      } catch { /* don't let diag break logging */ }
      originalError!.apply(console, args);
    };
    // Window-level unhandled errors.
    window.addEventListener('error', (ev) => {
      emit('error', 'window.error', ev.message || 'unknown', {
        source: ev.filename,
        line: ev.lineno,
        column: ev.colno,
        stack: ev.error?.stack,
      });
    });
    window.addEventListener('unhandledrejection', (ev) => {
      const reason: any = ev.reason;
      emit('error', 'unhandled-rejection', String(reason?.message || reason || 'unknown rejection'), reason);
    });
    emit('info', 'diag', 'installed', {
      ua: navigator.userAgent,
      viewport: { w: window.innerWidth, h: window.innerHeight },
    });
  },

  /** Restore original console methods + detach listeners (tests only). */
  uninstall() {
    if (!installed) return;
    installed = false;
    if (originalWarn) console.warn = originalWarn;
    if (originalError) console.error = originalError;
  },
};

// Dev convenience: expose on window so devtools console can poke it.
if (typeof window !== 'undefined') {
  (window as unknown as { diag: typeof diag }).diag = diag;
}

// ---- Auto-report + manual report ----
// The user shouldn't have to copy-paste diag logs for every bug. Two paths:
//
//   1. sendDiagReport(opts): POST the current buffer to the backend.
//      Tries /api/diag/report first, then /api/csp-report (looser schema
//      but at least a structured sink), then a last-resort /api/audit
//      POST with the JSON inlined. 3s AbortController per attempt.
//
//   2. Auto-triggered when ≥5 errors land in a 30s window and the user
//      hasn't already sent a report this session. Throttle flag lives
//      in sessionStorage.

interface SendReportOpts {
  label?: string;          // which boundary / surface caught
  context?: string;        // free-form extra note
  host?: string;           // backend host override (defaults to window.location.hostname)
}

const REPORT_PATHS = ['/api/diag/report', '/api/audit', '/api/csp-report'];
const REPORT_THROTTLE_KEY = 'lfi_diag_reported_ts';

async function postWithTimeout(url: string, body: string, ms = 3000): Promise<boolean> {
  const ctrl = new AbortController();
  const tid = setTimeout(() => ctrl.abort(), ms);
  try {
    const r = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
      signal: ctrl.signal,
    });
    clearTimeout(tid);
    return r.ok;
  } catch {
    clearTimeout(tid);
    return false;
  }
}

export async function sendDiagReport(opts: SendReportOpts = {}): Promise<boolean> {
  if (typeof window === 'undefined') return false;
  const host = opts.host || window.location.hostname || 'localhost';
  const payload = JSON.stringify({
    label: opts.label || 'manual',
    context: opts.context || '',
    exported_at: new Date().toISOString(),
    href: window.location.href,
    user_agent: navigator.userAgent,
    viewport: { w: window.innerWidth, h: window.innerHeight },
    buffer: JSON.parse(diag.export()),
  });
  for (const path of REPORT_PATHS) {
    const url = `http://${host}:3000${path}`;
    if (await postWithTimeout(url, payload, 3000)) {
      diag.info('diag-report', `sent to ${path}`, { bytes: payload.length });
      try { sessionStorage.setItem(REPORT_THROTTLE_KEY, String(Date.now())); } catch { /* silent */ }
      return true;
    }
  }
  diag.warn('diag-report', 'all endpoints unreachable or returned non-2xx', { tried: REPORT_PATHS });
  return false;
}

// Auto-report hook: when errors cross a threshold, fire a one-shot
// background report. Gated by a sessionStorage throttle so one bad
// session doesn't spam the backend.
let autoReportScheduled: number | null = null;
const AUTO_REPORT_THRESHOLD = 5;   // errors
const AUTO_REPORT_WINDOW_MS = 30_000;
const AUTO_REPORT_DEBOUNCE_MS = 1500;
const AUTO_REPORT_COOLDOWN_MS = 5 * 60_000; // don't re-report more than once per 5 min

if (typeof window !== 'undefined') {
  diag.subscribe((e) => {
    if (e.level !== 'error') return;
    try {
      const lastStr = sessionStorage.getItem(REPORT_THROTTLE_KEY);
      if (lastStr) {
        const last = Number(lastStr);
        if (isFinite(last) && Date.now() - last < AUTO_REPORT_COOLDOWN_MS) return;
      }
    } catch { /* silent */ }
    // Count recent errors in the ring.
    const cutoff = Date.now() - AUTO_REPORT_WINDOW_MS;
    const recent = diag.snapshot().filter(x => x.level === 'error' && x.ts >= cutoff).length;
    if (recent < AUTO_REPORT_THRESHOLD) return;
    if (autoReportScheduled != null) return;
    autoReportScheduled = window.setTimeout(async () => {
      autoReportScheduled = null;
      await sendDiagReport({ label: 'auto-threshold', context: `${recent} errors in last ${AUTO_REPORT_WINDOW_MS / 1000}s` });
    }, AUTO_REPORT_DEBOUNCE_MS);
  });
}
