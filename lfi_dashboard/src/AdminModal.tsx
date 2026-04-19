import React, { useRef, useState, useMemo, useEffect } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';
// c2-337 / c0-auto-2 task 21: 28px heading font sourced from the cross-platform
// design-system so desktop/Android builds share the same scale (T.typography
// caps at 22px which is visually a large shrink for the big dashboard numbers).
import { typography as dsType } from './design-system';
import { compactNum, formatRelative } from './util';
// c2-345 / c0-auto-2 task 24: shared uppercase meta-label component.
import { Label } from './components/Label';
// c2-348 / task 28: shared error banner (replaces local AdminErr).
import { ErrorAlert } from './components/ErrorAlert';
// c2-349 / task 29: shared shimmer skeleton.
import { SkeletonLoader } from './components/SkeletonLoader';
// c2-350 / task 27: shared horizontal progress bar.
import { BarChart } from './components/BarChart';
// c2-351 / task 30: shared WAI-ARIA tablist.
import { TabBar } from './components/TabBar';
// c2-375 / BIG #180: shared sortable table.
import { DataTable } from './components';
import type { Column } from './components';
// c2-433: diag ring buffer — consumed by the Diag tab.
import { diag } from './diag';
import type { DiagEntry, DiagLevel } from './diag';

// Full-screen admin modal per c0-017. Six tabs: Dashboard / Domains /
// Training / Quality / System / Logs. Replaces the prior sidebar-slide admin
// affordance which users found cramped. Sortable + filterable tables, big-
// number dashboard cards, bar-chart visualisations of domains + quality.

export type AdminTab = 'dashboard' | 'inventory' | 'domains' | 'training' | 'quality' | 'system' | 'fleet' | 'logs' | 'tokens' | 'proof' | 'diag' | 'docs';

interface FleetInstance {
  id: string;
  name?: string;
  role?: string;
  status?: string;          // 'running' | 'idle' | 'error' | ...
  last_seen?: number | string;
  current_task?: string;
  tasks_completed?: number;
  tasks_pending?: number;
}
interface FleetShape {
  instances?: FleetInstance[];
  timeline?: Array<{ t: number | string; instance: string; event: string; data?: any }>;
  stats?: { total_tasks?: number; completed?: number; running?: number };
}

interface DashboardShape {
  overview?: { total_facts?: number; total_sources?: number; cve_facts?: number; adversarial_facts?: number; total_training_pairs?: number };
  quality?: { average?: number; high_quality_count?: number; low_quality_count?: number; high_quality_pct?: number };
  training?: { sessions?: number; learning_signals?: number; total_tested?: number; total_correct?: number; pass_rate?: number; psl_calibration?: any };
  score?: { accuracy_score?: number; grade?: string; breakdown?: { quality?: number; adversarial?: number; coverage?: number; training?: number } };
  domains?: Array<{ domain: string; count: number }>;
  training_files?: Array<{ file: string; pairs: number; size_mb: number }>;
  system?: { uptime_hours?: number; server_version?: string };
}
interface DomainRow { domain: string; facts: number; avg_quality?: number; avg_length?: number }
interface AccuracyShape {
  pass_rate?: number; accuracy?: number; samples?: number;
  last_run?: string | number;
  per_domain?: Record<string, number>;
}
interface QualityShape {
  adversarial_count?: number;
  distinct_sources?: number;
  fts5_enabled?: boolean;
  psl_calibration?: { pass_rate?: number; status?: string; last_run?: string };
  // Distribution keyed by bucket label (e.g., '0.0-0.2', '0.2-0.4') OR by
  // score-name (high/medium/low). We render whichever shape appears.
  quality_distribution?: Record<string, number>;
  high_quality_count?: number;
  low_quality_count?: number;
  average?: number;
}
interface SystemShape {
  hostname?: string; os?: string; cpu_count?: number; cpu_model?: string;
  disk_root_free_bytes?: number; disk_root_total_bytes?: number;
  ram_total_mb?: number; ram_available_mb?: number; cpu_temp_c?: number;
  ollama?: { status?: string; model?: string };
  uptime_seconds?: number;
}

export interface AdminModalProps {
  C: any;
  host: string;
  onClose: () => void;
  // Top-line facts/sources already polled at app level — reuse instead of
  // refetching per modal open.
  factsCount: number;
  sourcesCount: number;
  initialTab?: AdminTab;
  // Optional client-side event log. Used as a fallback source in the Logs
  // tab when /api/admin/logs is unavailable, so users still see something.
  localEvents?: Array<{ t: number; kind: string; data?: any }>;
  // c2-433 mobile-fix: when true, backdrop padding collapses + header
  // padding halves so the 10-tab TabBar + dashboard grid get maximum
  // viewport width.
  isMobile?: boolean;
}

const fmtBytes = (n?: number): string => {
  if (typeof n !== 'number' || n < 0) return '—';
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${n} B`;
};
const fmtSeconds = (s?: number): string => {
  if (typeof s !== 'number' || s < 0) return '—';
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  return d > 0 ? `${d}d ${h}h` : h > 0 ? `${h}h ${m}m` : `${m}m`;
};
const pctNorm = (raw: number | undefined): number | null => {
  if (typeof raw !== 'number' || !isFinite(raw)) return null;
  return raw <= 1.5 ? raw * 100 : raw;
};

export const AdminModal: React.FC<AdminModalProps> = ({
  C, host, onClose, factsCount, sourcesCount, initialTab = 'dashboard', localEvents = [], isMobile = false,
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  // c2-260 / #122: persist the active tab so a reopen lands the user where
  // they left off. Explicit initialTab prop (e.g. /logs slash command) wins
  // over the stored preference so jump-links still work.
  const [tab, setTab] = useState<AdminTab>(() => {
    if (initialTab !== 'dashboard') return initialTab;
    try {
      const stored = localStorage.getItem('lfi_admin_tab') as AdminTab | null;
      const valid: AdminTab[] = ['dashboard', 'inventory', 'domains', 'training', 'quality', 'system', 'fleet', 'logs', 'tokens', 'proof', 'diag', 'docs'];
      if (stored && valid.includes(stored)) return stored;
    } catch { /* storage blocked */ }
    return initialTab;
  });
  useEffect(() => {
    try { localStorage.setItem('lfi_admin_tab', tab); } catch { /* quota */ }
  }, [tab]);
  const [dashboard, setDashboard] = useState<DashboardShape | null>(null);
  const [domains, setDomains] = useState<DomainRow[] | null>(null);
  const [accuracy, setAccuracy] = useState<AccuracyShape | null>(null);
  const [quality, setQuality] = useState<QualityShape | null>(null);
  // c2-424 / task 207: rolling history of quality.average so we can show
  // a sparkline above the Quality tab. Bounded buffer + min-gap to stop
  // rapid tab-visits spamming the array.
  type QualitySnap = { ts: number; avg: number; highCount: number; lowCount: number };
  const [qualityHistory, setQualityHistory] = useState<QualitySnap[]>(() => {
    try {
      const raw = localStorage.getItem('lfi_quality_history_v1');
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((s: any) => s && typeof s.ts === 'number' && typeof s.avg === 'number').slice(-48);
    } catch { return []; }
  });
  const [sysInfo, setSysInfo] = useState<SystemShape | null>(null);
  const [fleet, setFleet] = useState<FleetShape | null>(null);
  const [logs, setLogs] = useState<string[] | null>(null);
  // c2-433 / #305: Merkle-chained security-audit integrity. /api/audit/chain/verify
  // returns {valid: bool, broken_at_idx?: number}. Poll every 60s while the
  // Admin modal is open; banner turns red when the chain has been
  // tampered with. Null = not yet fetched (banner hidden).
  const [chainVerify, setChainVerify] = useState<null | { valid: boolean; broken_at_idx?: number | null; error?: string }>(null);
  // c2-433 / #305 followup: expandable panel under the banner showing the
  // last N audit-chain entries from /api/audit/chain/recent. Auto-expands
  // when valid=false so the operator can inspect the events around the
  // broken index without a click.
  const [chainExpanded, setChainExpanded] = useState<boolean>(false);
  const [chainRecent, setChainRecent] = useState<null | any[]>(null);
  const [chainRecentErr, setChainRecentErr] = useState<string | null>(null);
  const [chainRecentLoading, setChainRecentLoading] = useState<boolean>(false);
  // c2-433 / #305 followup: last-success verify timestamp. Appended to
  // the green banner as "· verified Ns ago" so operators know the chain
  // confirmation isn't stale. Updates on every verify that returns
  // valid:true; an error or invalid doesn't reset this — the red banner
  // takes over and this becomes hidden.
  const [lastChainVerifiedAt, setLastChainVerifiedAt] = useState<number | null>(null);
  const [err, setErr] = useState<Record<AdminTab, string | null>>({
    dashboard: null, inventory: null, domains: null, training: null, quality: null, system: null, fleet: null, logs: null, tokens: null, proof: null, diag: null,
  });
  const [loading, setLoading] = useState<AdminTab | null>(null);
  // c2-272: per-tab last-successful-load timestamp. Mirrors the Classroom
  // "Updated Xs ago" affordance so users know how stale the numbers are,
  // especially important on tabs with long auto-refresh intervals.
  const [lastLoadedAt, setLastLoadedAt] = useState<Partial<Record<AdminTab, number>>>({});

  const fetchJson = async <T,>(path: string, signal: AbortSignal, port: number = 3000): Promise<T> => {
    const res = await fetch(`http://${host}:${port}${path}`, { signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json() as Promise<T>;
  };
  // c2-322 / c0-035 #2: orchestrator fleet data lives on :3001. Try there
  // first; fall back to :3000 during rollout so older deployments without
  // the split service still render the Fleet tab.
  const fetchFleet = async (signal: AbortSignal): Promise<FleetShape> => {
    try { return await fetchJson<FleetShape>('/api/orchestrator/dashboard', signal, 3001); }
    catch { return await fetchJson<FleetShape>('/api/orchestrator/dashboard', signal, 3000); }
  };
  const loadTab = async (t: AdminTab) => {
    setLoading(t);
    setErr(e => ({ ...e, [t]: null }));
    const ctrl = new AbortController();
    const to = setTimeout(() => ctrl.abort(), 10000);
    try {
      if (t === 'dashboard' || t === 'inventory') {
        // c0-026: use the consolidated endpoint — returns overview + quality
        // + training + score + domains + training_files + system in one call.
        // c2-234 / #66: Inventory tab reads the same payload (no extra
        // endpoint required), so we share the fetch path.
        try {
          setDashboard(await fetchJson('/api/admin/dashboard', ctrl.signal));
        } catch {
          // Fall back to the per-feature endpoints if /dashboard isn't up yet.
        }
      }
      if (t === 'domains') {
        const data: any = await fetchJson('/api/admin/training/domains', ctrl.signal);
        const arr: DomainRow[] = Array.isArray(data?.domains) ? data.domains : Array.isArray(data) ? data : [];
        setDomains(arr.sort((a, b) => b.facts - a.facts));
      }
      if (t === 'training') {
        setAccuracy(await fetchJson('/api/admin/training/accuracy', ctrl.signal));
      }
      if (t === 'quality') {
        const q = await fetchJson<QualityShape>('/api/quality/report', ctrl.signal);
        setQuality(q);
        // c2-424 / task 207: snapshot for the Quality tab sparkline.
        // 10-min min gap so repeatedly opening the tab doesn't flood the
        // buffer. localStorage persists across reloads.
        if (typeof q?.average === 'number' && isFinite(q.average)) {
          setQualityHistory(prev => {
            const last = prev[prev.length - 1];
            if (last && Date.now() - last.ts < 10 * 60_000) return prev;
            const next: QualitySnap[] = [...prev, {
              ts: Date.now(), avg: q.average!,
              highCount: q.high_quality_count ?? 0,
              lowCount: q.low_quality_count ?? 0,
            }].slice(-48);
            try { localStorage.setItem('lfi_quality_history_v1', JSON.stringify(next)); } catch { /* quota */ }
            return next;
          });
        }
      }
      if (t === 'system') {
        setSysInfo(await fetchJson('/api/system/info', ctrl.signal));
      }
      if (t === 'fleet') {
        setFleet(await fetchFleet(ctrl.signal));
      }
      if (t === 'logs') {
        try {
          const data: any = await fetchJson('/api/admin/logs', ctrl.signal);
          setLogs(Array.isArray(data?.lines) ? data.lines : Array.isArray(data) ? data : []);
        } catch {
          setLogs([]);
        }
      }
      // c2-272: only stamp when the outer try cleared without throwing.
      // Nested catches above (dashboard + logs fallbacks) shallow-swallow
      // per-endpoint failures, so reaching here means the tab is usable.
      setLastLoadedAt(prev => ({ ...prev, [t]: Date.now() }));
    } catch (e: any) {
      const m = String(e?.message || e || 'fetch failed');
      // Distinguish AbortError from real HTTP errors so the user knows
      // "backend is busy" vs "endpoint returned 500".
      const friendly = m.includes('abort') || m.includes('aborted')
        ? 'Backend is busy — request timed out after 10s. It may be running a long DB transaction (e.g. WAL checkpoint). Click Refresh to retry.'
        : m.startsWith('HTTP') ? `${m} — backend returned an error. Is the route registered?` : m;
      setErr(x => ({ ...x, [t]: friendly }));
    } finally {
      clearTimeout(to);
      setLoading(null);
    }
  };
  // Auto-load the active tab on mount + when tab changes (if data missing).
  useEffect(() => { loadTab(tab); /* eslint-disable-next-line */ }, [tab]);

  // c0-022: Training tab auto-refreshes every 5s so the user sees live
  // progress (pairs generated, accuracy over time). Pauses when the tab
  // isn't active to avoid wasted requests.
  useEffect(() => {
    if (tab !== 'training') return;
    const id = setInterval(() => { loadTab('training'); }, 5000);
    return () => clearInterval(id);
    // eslint-disable-next-line
  }, [tab]);

  // c0-026: Dashboard tab auto-refreshes every 10s against the consolidated
  // /api/admin/dashboard endpoint. Paused when user switches tabs.
  useEffect(() => {
    if (tab !== 'dashboard') return;
    const id = setInterval(() => { loadTab('dashboard'); }, 10000);
    return () => clearInterval(id);
    // eslint-disable-next-line
  }, [tab]);

  // c2-433 / #305: chain verify poll. Hits /api/audit/chain/verify on open
  // + every 60s while the modal is mounted. A red Integrity banner appears
  // when valid=false or the fetch itself errors — chain tampering and
  // backend unreachability are both things the operator needs to see.
  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const r = await fetch(`http://${host}:3000/api/audit/chain/verify`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const data = await r.json();
        if (cancelled) return;
        const valid = data.valid === true;
        const brokenAt = typeof data.broken_at_idx === 'number' ? data.broken_at_idx
          : (typeof data.broken_idx === 'number' ? data.broken_idx : null);
        setChainVerify({ valid, broken_at_idx: brokenAt });
        if (valid) setLastChainVerifiedAt(Date.now());
        // c2-433 / #305 followup: auto-expand the recent-entries panel
        // the moment the chain breaks so the operator sees the events
        // around the broken index without clicking. Idempotent: once
        // the operator closes it manually (sets to false), a later
        // verify that still reports broken won't re-open — we only
        // auto-open on transition to invalid.
        if (!valid) setChainExpanded(prev => prev ? prev : true);
      } catch (e: any) {
        if (!cancelled) setChainVerify({ valid: false, error: String(e?.message || e || 'unreachable') });
      }
    };
    check();
    const id = window.setInterval(check, 60_000);
    return () => { cancelled = true; window.clearInterval(id); };
  }, [host]);

  // c2-433 / #305 followup: fetch /api/audit/chain/recent when the panel
  // expands. Refetches on every expand so operators see fresh data.
  useEffect(() => {
    if (!chainExpanded) return;
    let cancelled = false;
    (async () => {
      setChainRecentLoading(true);
      setChainRecentErr(null);
      try {
        const r = await fetch(`http://${host}:3000/api/audit/chain/recent?limit=20`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const data = await r.json();
        if (cancelled) return;
        const list: any[] = Array.isArray(data) ? data
          : Array.isArray(data?.entries) ? data.entries
          : Array.isArray(data?.items) ? data.items
          : Array.isArray(data?.chain) ? data.chain
          : [];
        setChainRecent(list);
      } catch (e: any) {
        if (!cancelled) setChainRecentErr(String(e?.message || e || 'fetch failed'));
      } finally {
        if (!cancelled) setChainRecentLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [chainExpanded, host]);

  // ---- Tables: shared sort + filter state ----
  const [domainSort, setDomainSort] = useState<{ key: keyof DomainRow; dir: 'asc' | 'desc' }>({ key: 'facts', dir: 'desc' });
  const [domainFilter, setDomainFilter] = useState('');
  const [logFilter, setLogFilter] = useState('');
  const filteredDomains = useMemo(() => {
    if (!domains) return [];
    const q = domainFilter.trim().toLowerCase();
    const filt = q ? domains.filter(d => d.domain.toLowerCase().includes(q)) : domains;
    return [...filt].sort((a, b) => {
      const av = a[domainSort.key] ?? 0;
      const bv = b[domainSort.key] ?? 0;
      if (typeof av === 'string' || typeof bv === 'string') {
        const s = String(av).localeCompare(String(bv));
        return domainSort.dir === 'asc' ? s : -s;
      }
      return domainSort.dir === 'asc' ? (av as number) - (bv as number) : (bv as number) - (av as number);
    });
  }, [domains, domainFilter, domainSort]);

  const qualityColor = (q: number) => q > 0.8 ? C.green : q >= 0.5 ? C.yellow : C.red;
  const countColor = (n: number) => n > 10000 ? C.green : n > 1000 ? C.yellow : C.red;

  const sortArrow = (active: boolean, dir: 'asc' | 'desc') =>
    active ? (dir === 'asc' ? ' \u25B2' : ' \u25BC') : '';

  return (
    <div onClick={onClose}
      role='presentation'
      style={{
        position: 'fixed', inset: 0, zIndex: T.z.modal + 40,
        background: C.overlayBg,
        display: 'flex', alignItems: 'stretch', justifyContent: 'center',
        // c2-433 mobile: zero backdrop padding on mobile so the admin
        // body fills the viewport. Desktop keeps the 16-24px gutter.
        padding: isMobile ? 0 : T.spacing.lg,
      }}>
      <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-admin-title'
        onClick={(e) => e.stopPropagation()}
        style={{
          width: '100%', maxWidth: '1240px', height: '100%',
          background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
          display: 'flex', flexDirection: 'column', overflow: 'hidden',
          boxShadow: T.shadows.modal,
        }}>
        {/* Header */}
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: isMobile ? '10px 12px' : '14px 22px',
          borderBottom: `1px solid ${C.borderSubtle}`,
          gap: T.spacing.sm, flexWrap: 'wrap',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md }}>
            <svg width='20' height='20' viewBox='0 0 24 24' fill='none' stroke={C.accent} strokeWidth='2' strokeLinecap='round' strokeLinejoin='round'>
              <rect x='3' y='3' width='7' height='7' /><rect x='14' y='3' width='7' height='7' />
              <rect x='3' y='14' width='7' height='7' /><rect x='14' y='14' width='7' height='7' />
            </svg>
            <h2 id='scc-admin-title' style={{
              margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBlack,
              letterSpacing: '0.08em', textTransform: 'uppercase', color: C.text,
            }}>Admin Console</h2>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
            {/* c2-272: staleness indicator per active tab. Hidden until first
                successful fetch so it does not flash 0s before data lands. */}
            {lastLoadedAt[tab] != null && (
              <span aria-live='polite' style={{
                fontSize: T.typography.sizeXs, color: C.textDim,
                fontFamily: T.typography.fontMono,
              }}>Updated {formatRelative(lastLoadedAt[tab]!)}</span>
            )}
            {/* c2-258 / #120: manual refresh for the active tab. Spins the
                icon while a load is in-flight so the user sees progress.
                Covers every tab via the shared loadTab dispatcher. */}
            <button onClick={() => loadTab(tab)} aria-label={`Refresh ${tab}`}
              title={`Refresh ${tab} (auto-refreshes for some tabs)`}
              disabled={loading === tab}
              style={{
                background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                color: C.textMuted, borderRadius: T.radii.sm,
                cursor: loading === tab ? 'wait' : 'pointer',
                padding: '4px 8px', display: 'flex', alignItems: 'center',
                fontFamily: 'inherit', fontSize: T.typography.sizeXs,
              }}>
              <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor'
                strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round'
                style={loading === tab ? { animation: 'scc-admin-spin 0.8s linear infinite' } : undefined}>
                <polyline points='23 4 23 10 17 10' />
                <polyline points='1 20 1 14 7 14' />
                <path d='M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15' />
              </svg>
            </button>
            <button onClick={onClose} aria-label='Close admin'
              style={{
                background: 'transparent', border: 'none', color: C.textMuted,
                fontSize: T.typography.size3xl, cursor: 'pointer', padding: '4px 10px',
              }}>{'\u2715'}</button>
          </div>
        </div>
        <style>{`@keyframes scc-admin-spin { to { transform: rotate(360deg); } }`}</style>

        {/* c2-433 / #305: Merkle-chained audit-log Integrity banner. Red if
            /api/audit/chain/verify returned valid:false (chain has been
            tampered with — broken_at_idx points at the first corrupt entry)
            OR the endpoint itself was unreachable. Green pill when the
            chain verifies. Hidden during the initial verify (null state)
            to avoid a flash on open. */}
        {chainVerify && (
          chainVerify.valid ? (
            <button type='button'
              onClick={() => setChainExpanded(v => !v)}
              title={chainExpanded ? 'Hide recent audit-chain entries' : 'Show recent audit-chain entries'}
              aria-expanded={chainExpanded}
              style={{
                display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                padding: isMobile ? '6px 12px' : '6px 22px',
                fontSize: T.typography.sizeXs,
                background: `${C.green}15`, color: C.green,
                borderBottom: `1px solid ${C.borderSubtle}`,
                border: 'none', borderTopLeftRadius: 0, borderTopRightRadius: 0,
                fontFamily: T.typography.fontMono, fontWeight: 700,
                letterSpacing: '0.04em', width: '100%',
                cursor: 'pointer', textAlign: 'left',
              }}>
              <span style={{
                width: '6px', height: '6px', borderRadius: '50%',
                background: C.green, flexShrink: 0,
              }} />
              <span style={{ flexShrink: 0 }}>AUDIT CHAIN VERIFIED</span>
              {/* c2-433 / #305 followup: last-verified timestamp. Gives
                  operators a recency signal so a stale banner (e.g. after
                  network drop) doesnt falsely reassure. min-width:0 + the
                  enclosing span let the relative-time string shrink or
                  ellipsize on narrow viewports instead of pushing the
                  chevron off-row. */}
              {lastChainVerifiedAt != null && (
                <span style={{ opacity: 0.7, fontWeight: 500, letterSpacing: 0, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  · verified {formatRelative(lastChainVerifiedAt)}
                </span>
              )}
              <span aria-hidden='true' style={{ marginLeft: 'auto', opacity: 0.7, flexShrink: 0 }}>{chainExpanded ? '▴' : '▾'}</span>
            </button>
          ) : (
            <button type='button' role='alert'
              onClick={() => setChainExpanded(v => !v)}
              title={chainVerify.error ? `Verify endpoint unreachable: ${chainVerify.error} — click for recent entries` : 'The security audit chain has been tampered with — click for recent entries'}
              aria-expanded={chainExpanded}
              style={{
                display: 'flex', alignItems: 'center', gap: T.spacing.md,
                padding: isMobile ? '10px 12px' : '10px 22px',
                fontSize: T.typography.sizeSm,
                background: C.redBg, color: C.red,
                borderBottom: `1px solid ${C.redBorder}`,
                border: 'none', borderTopLeftRadius: 0, borderTopRightRadius: 0,
                fontWeight: T.typography.weightBold,
                animation: 'scc-admin-integrity-pulse 2s ease-in-out infinite',
                width: '100%', cursor: 'pointer', textAlign: 'left',
                fontFamily: 'inherit',
              }}>
              <svg width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true' style={{ flexShrink: 0 }}>
                <path d='M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z' />
                <line x1='12' y1='9' x2='12' y2='13' />
                <line x1='12' y1='17' x2='12.01' y2='17' />
              </svg>
              {/* c2-433 / mobile: wordBreak lets long unreachable-endpoint
                  strings (e.g. HTTP 0 on a hostname that never resolved)
                  wrap inside the flex:1 column instead of forcing the
                  chevron off-row. min-width:0 ensures the flex child can
                  actually shrink below its intrinsic content width. */}
              <span style={{ flex: 1, minWidth: 0, wordBreak: 'break-word' }}>
                {chainVerify.error ? (
                  <>Audit chain verify failed — endpoint unreachable ({chainVerify.error}). Chain integrity cannot be confirmed.</>
                ) : chainVerify.broken_at_idx != null ? (
                  <>Audit chain integrity <strong>BROKEN</strong> at entry #{chainVerify.broken_at_idx} — a security audit event has been tampered with or the chain was truncated.</>
                ) : (
                  <>Audit chain integrity <strong>BROKEN</strong> — chain is invalid (broken index not reported).</>
                )}
              </span>
              <span aria-hidden='true' style={{ opacity: 0.7, fontSize: T.typography.sizeMd, flexShrink: 0 }}>{chainExpanded ? '▴' : '▾'}</span>
            </button>
          )
        )}
        {/* c2-433 / #305 followup: expandable recent-entries panel. Fetches
            /api/audit/chain/recent?limit=20 and renders a compact mono
            table (idx, ts, event, hash-prefix). Auto-opens on chain
            tampering; manually toggled via the banner click. The hash
            column shows just the first 10 chars of the hex to keep the
            row narrow. */}
        {chainVerify && chainExpanded && (
          <div style={{
            padding: isMobile ? '8px 12px' : '8px 22px',
            borderBottom: `1px solid ${C.borderSubtle}`,
            background: C.bgInput, maxHeight: '220px', overflowY: 'auto',
          }}>
            {chainRecentLoading && (
              <div style={{ color: C.textMuted, fontSize: T.typography.sizeXs, fontStyle: 'italic' }}>
                Loading recent audit entries…
              </div>
            )}
            {chainRecentErr && !chainRecentLoading && (
              <div role='alert' style={{ color: C.red, fontSize: T.typography.sizeXs }}>
                Could not load recent entries: {chainRecentErr}
              </div>
            )}
            {chainRecent && !chainRecentLoading && !chainRecentErr && chainRecent.length === 0 && (
              <div style={{ color: C.textDim, fontSize: T.typography.sizeXs, fontStyle: 'italic' }}>
                No entries yet — the audit chain is empty.
              </div>
            )}
            {chainRecent && !chainRecentLoading && chainRecent.length > 0 && (() => {
              // c2-433 / #307: extract severity per row + compute a
              // high-severity count. Anything tagged "High" (usually an
              // auth rate-limit rejection per Claude 0's 04:05 spec) is
              // what brute-force attempts look like in the chain.
              const sevOf = (e: any): string => String(e.severity ?? e.level ?? '').toLowerCase();
              const isHigh = (e: any) => {
                const s = sevOf(e);
                return s === 'high' || s === 'critical' || s === 'severe';
              };
              const highCount = chainRecent.filter(isHigh).length;
              const sevColor = (s: string): string => {
                if (s === 'high' || s === 'critical' || s === 'severe') return C.red;
                if (s === 'warn' || s === 'warning' || s === 'medium') return C.yellow;
                if (s === 'low' || s === 'info' || s === 'debug') return C.textDim;
                return C.textMuted;
              };
              return (
                <>
                  {highCount > 0 && (
                    <div role='status' style={{
                      marginBottom: '8px', padding: '6px 10px',
                      background: C.redBg, border: `1px solid ${C.redBorder}`,
                      borderRadius: T.radii.sm, color: C.red,
                      fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                      fontFamily: T.typography.fontMono, letterSpacing: '0.04em',
                      display: 'flex', alignItems: 'center', gap: '6px',
                    }}>
                      <svg width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2.4' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true'>
                        <path d='M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z' />
                        <line x1='12' y1='9' x2='12' y2='13' />
                        <line x1='12' y1='17' x2='12.01' y2='17' />
                      </svg>
                      <span>{highCount} high-severity event{highCount === 1 ? '' : 's'} in the last {chainRecent.length} — likely rate-limited auth (brute-force signal)</span>
                    </div>
                  )}
                  <div style={{
                    // c2-433 / mobile fix: fixed column widths summed to
                    // 380px and overflowed narrow (375px iPhone) viewports.
                    // Use flexible minmax — idx/sev/hash shrink before the
                    // event column, and the grid reflows below container.
                    display: 'grid', gridTemplateColumns: 'minmax(40px, 60px) minmax(110px, 160px) minmax(0, 1fr) minmax(40px, 60px) minmax(60px, 100px)',
                    gap: '6px 12px', fontSize: '10px',
                    fontFamily: T.typography.fontMono,
                  }}>
                    <div style={{ color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Idx</div>
                    <div style={{ color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Timestamp</div>
                    <div style={{ color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Event</div>
                    <div style={{ color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Sev</div>
                    <div style={{ color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Hash</div>
                    {chainRecent.map((e: any, i: number) => {
                      const idx = e.idx ?? e.index ?? e.id ?? i;
                      const ts = e.ts ?? e.timestamp ?? e.created_at ?? '';
                      const tsStr = typeof ts === 'string' ? ts : typeof ts === 'number' ? new Date(ts).toISOString() : '';
                      const kind = e.event ?? e.kind ?? e.type ?? e.action ?? 'event';
                      const payload = e.data ?? e.payload ?? e.details ?? null;
                      const hash = e.hash ?? e.prev_hash ?? e.digest ?? '';
                      const hashShort = typeof hash === 'string' ? hash.slice(0, 10) : '';
                      const isBroken = chainVerify.broken_at_idx != null && idx === chainVerify.broken_at_idx;
                      const sev = sevOf(e);
                      const high = isHigh(e);
                      return (
                        <React.Fragment key={`${idx}-${i}`}>
                          <div style={{
                            color: isBroken ? C.red : high ? C.red : C.accent,
                            fontWeight: (isBroken || high) ? 900 : 700,
                          }}>{isBroken ? '⚠ ' : high ? '⚠ ' : ''}#{idx}</div>
                          <div style={{ color: C.textSecondary, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                            {tsStr ? tsStr.slice(0, 19).replace('T', ' ') : '—'}
                          </div>
                          <div style={{ color: high ? C.red : C.text, fontWeight: high ? 700 : 400, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                            title={payload ? `${kind} · ${JSON.stringify(payload).slice(0, 200)}` : String(kind)}>
                            {String(kind)}
                          </div>
                          <div style={{
                            color: sevColor(sev),
                            fontWeight: high ? 900 : 600,
                            textTransform: 'uppercase', letterSpacing: '0.04em',
                          }} title={sev ? `severity: ${sev}` : 'no severity reported'}>
                            {sev ? sev.slice(0, 4) : '—'}
                          </div>
                          <div style={{ color: C.textDim }}>{hashShort ? `${hashShort}…` : '—'}</div>
                        </React.Fragment>
                      );
                    })}
                  </div>
                </>
              );
            })()}
          </div>
        )}
        <style>{`@keyframes scc-admin-integrity-pulse { 0%, 100% { box-shadow: inset 0 0 0 0 ${C.red}00; } 50% { box-shadow: inset 0 -2px 0 0 ${C.red}aa; } }`}</style>

        {/* Tab bar — WAI-ARIA tablist with arrow-key navigation. */}
        <TabBar<AdminTab> C={C} label='Admin sections'
          padding={isMobile ? '0 12px' : '0 22px'}
          compact={isMobile}
          weight={T.typography.weightBold}
          tabs={[
            { id: 'dashboard', label: 'Dashboard' },
            { id: 'inventory', label: 'Inventory' },
            { id: 'domains', label: 'Domains' },
            { id: 'training', label: 'Training' },
            { id: 'quality', label: 'Quality' },
            { id: 'system', label: 'System' },
            { id: 'fleet', label: 'Fleet' },
            { id: 'logs', label: 'Logs' },
            { id: 'tokens', label: 'Tokens' },
            { id: 'proof', label: 'Proof' },
            { id: 'diag', label: 'Diag' },
            { id: 'docs', label: 'Docs' },
          ]}
          active={tab}
          onChange={setTab} />

        {/* Body */}
        <div role='tabpanel' aria-label={tab} style={{ flex: 1, overflowY: 'auto', padding: isMobile ? '14px 12px' : '20px 22px' }}>
          {/* ---------- Dashboard ---------- */}
          {tab === 'dashboard' && (
            <div>
              {err.dashboard && <ErrorAlert C={C} message={err.dashboard} onRetry={() => loadTab('dashboard')} retrying={loading === 'dashboard'} />}
              {/* Skeleton loader — only shown while the first fetch is in
                  flight AND we don't yet have any cached data. Subsequent
                  refreshes render fresh data silently. Per c0-020. */}
              {loading === 'dashboard' && !dashboard && (
                <div aria-busy='true' aria-live='polite'>
                  <SkeletonLoader C={C} base='input' height='120px' style={{ marginBottom: T.spacing.xl }} />
                  <div style={{
                    display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                    gap: T.spacing.md, marginBottom: T.spacing.xl,
                  }}>
                    {[0, 1, 2, 3, 4, 5].map(i => (
                      <SkeletonLoader key={i} C={C} base='input' height='70px' delay={i * 0.08} />
                    ))}
                  </div>
                </div>
              )}
              {/* Accuracy grade + score breakdown from /api/admin/dashboard */}
              {dashboard?.score && (
                <div style={{
                  display: 'grid',
                  // Stack on narrow viewports so neither column gets squeezed.
                  gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
                  gap: T.spacing.lg, marginBottom: T.spacing.xl,
                  padding: T.spacing.lg, borderRadius: T.radii.lg,
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                }}>
                  <div style={{ textAlign: 'center' }}>
                    <Label color={C.textMuted}>
                      Accuracy grade
                    </Label>
                    <div style={{
                      fontSize: '72px', fontWeight: T.typography.weightBlack,
                      color: (() => {
                        const g = dashboard.score?.grade || '';
                        if (g.startsWith('A')) return C.green;
                        if (g.startsWith('B')) return C.accent;
                        if (g.startsWith('C')) return C.yellow;
                        return C.red;
                      })(),
                      lineHeight: 1, marginTop: '4px',
                      fontFamily: T.typography.fontMono,
                    }}>{dashboard.score.grade || '—'}</div>
                    {typeof dashboard.score.accuracy_score === 'number' && (
                      <div style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, marginTop: '4px', fontFamily: T.typography.fontMono }}>
                        {dashboard.score.accuracy_score.toFixed(1)} / 100
                      </div>
                    )}
                  </div>
                  <div>
                    <Label color={C.textMuted} mb={T.spacing.md}>
                      Score breakdown
                    </Label>
                    {dashboard.score.breakdown && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.sm }}>
                        {(['quality', 'adversarial', 'coverage', 'training'] as const).map(k => {
                          const v = dashboard.score?.breakdown?.[k];
                          if (typeof v !== 'number') return null;
                          const pc = v <= 1.5 ? v * 100 : v;
                          const col = pc >= 80 ? C.green : pc >= 60 ? C.yellow : C.red;
                          return (
                            <div key={k} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                              <span style={{ width: '110px', fontSize: T.typography.sizeSm, color: C.textSecondary, textTransform: 'capitalize' }}>{k}</span>
                              <BarChart C={C} value={pc} color={col} height='10px' trackBg={C.bg} style={{ flex: 1 }} />
                              <span style={{ width: '56px', textAlign: 'right', fontSize: T.typography.sizeSm, color: col, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{pc.toFixed(0)}</span>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              )}
              <div style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {[
                  { label: 'Facts', value: (() => {
                    const v = dashboard?.overview?.total_facts ?? factsCount;
                    return v ? compactNum(v) : '—';
                  })(), color: C.purple },
                  { label: 'Sources', value: (() => {
                    const v = dashboard?.overview?.total_sources ?? sourcesCount;
                    return v ? String(v) : '—';
                  })(), color: C.green },
                  { label: 'Training pairs', value: dashboard?.overview?.total_training_pairs != null ? compactNum(dashboard.overview.total_training_pairs) : '—', color: C.accent },
                  { label: 'Pass rate', value: (() => {
                    const p = pctNorm(dashboard?.training?.pass_rate ?? accuracy?.pass_rate ?? accuracy?.accuracy);
                    return p != null ? `${p.toFixed(1)}%` : '—';
                  })(), color: C.yellow },
                  { label: 'Adversarial', value: dashboard?.overview?.adversarial_facts != null
                    ? compactNum(dashboard.overview.adversarial_facts)
                    : (quality?.adversarial_count != null ? compactNum(quality.adversarial_count) : '—'), color: C.red },
                  { label: 'Avg quality', value: (() => {
                    const q = dashboard?.quality?.average;
                    return typeof q === 'number' ? q.toFixed(2) : '—';
                  })(), color: C.green },
                ].map(card => (
                  <div key={card.label} style={{
                    padding: '16px 18px', borderRadius: T.radii.lg,
                    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  }}>
                    <Label color={C.textMuted}>
                      {card.label}
                    </Label>
                    <div style={{ fontSize: dsType.sizes['3xl'], fontWeight: T.typography.weightBlack, color: card.color, marginTop: '4px', fontFamily: T.typography.fontMono }}>
                      {card.value}
                    </div>
                  </div>
                ))}
              </div>
              {/* Domain bar chart — prefer consolidated dashboard.domains
                  when present ({domain, count} shape), fall back to domains
                  state ({domain, facts}). */}
              {(() => {
                type R = { label: string; value: number };
                const source: R[] =
                  dashboard?.domains && dashboard.domains.length > 0
                    ? dashboard.domains.map(d => ({ label: d.domain, value: d.count }))
                    : (domains || []).map(d => ({ label: d.domain, value: d.facts }));
                if (source.length === 0) return null;
                const top = [...source].sort((a, b) => b.value - a.value).slice(0, 10);
                const max = Math.max(...top.map(d => d.value), 1);
                return (
                  <div style={{ marginBottom: T.spacing.xl }}>
                    <Label color={C.textMuted} mb={T.spacing.md}>
                      Top 10 domains by fact count
                    </Label>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {top.map(d => (
                        <div key={d.label} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                          <span style={{ width: '160px', fontSize: T.typography.sizeSm, color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{d.label}</span>
                          <BarChart C={C} value={(d.value / max) * 100} color={countColor(d.value)} height='18px' trackBg={C.bgInput} style={{ flex: 1 }} />
                          <span style={{ width: '84px', textAlign: 'right', fontSize: T.typography.sizeSm, fontFamily: T.typography.fontMono, color: C.textMuted }}>
                            {d.value.toLocaleString()}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                );
              })()}
              {/* Training files list from consolidated endpoint. c2-375:
                  migrated to DataTable -- columns now sortable for free. */}
              {dashboard?.training_files && dashboard.training_files.length > 0 && (() => {
                type Row = { file: string; pairs: number; size_mb: number };
                const cols: ReadonlyArray<Column<Row>> = [
                  {
                    id: 'file', header: 'File', align: 'left',
                    sortKey: (f) => f.file.toLowerCase(),
                    accessor: (f) => <span style={{ fontFamily: T.typography.fontMono, color: C.text }}>{f.file}</span>,
                  },
                  {
                    id: 'pairs', header: 'Pairs', align: 'right',
                    sortKey: (f) => f.pairs,
                    accessor: (f) => <span style={{ fontFamily: T.typography.fontMono, color: C.accent }}>{f.pairs.toLocaleString()}</span>,
                  },
                  {
                    id: 'size', header: 'Size', align: 'right',
                    sortKey: (f) => f.size_mb,
                    accessor: (f) => <span style={{ fontFamily: T.typography.fontMono, color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</span>,
                  },
                ];
                return (
                  <div>
                    <Label color={C.textMuted} mb={T.spacing.md}>
                      Training datasets ({dashboard.training_files.length})
                    </Label>
                    <DataTable<Row> C={C}
                      rows={dashboard.training_files}
                      columns={cols}
                      rowKey={(f) => f.file}
                      sort={{ col: 'pairs', dir: 'desc' }}
                      cellFontSize={T.typography.sizeMd} />
                  </div>
                );
              })()}
              {dashboard?.system && (
                <div style={{ marginTop: T.spacing.xl, fontSize: T.typography.sizeXs, color: C.textDim, textAlign: 'center' }}>
                  Server v{dashboard.system.server_version || '?'}
                  {typeof dashboard.system.uptime_hours === 'number' && ` · uptime ${dashboard.system.uptime_hours.toFixed(1)}h`}
                  {' · auto-refreshes every 10s'}
                </div>
              )}
            </div>
          )}

          {/* ---------- Inventory (c2-234 / #66) ---------- */}
          {/* A single-page "what's in the system" view. Pure render from the
              shared /api/admin/dashboard payload so it doesn't double-fetch. */}
          {tab === 'inventory' && (
            <div>
              <h3 style={{ margin: 0, marginBottom: T.spacing.md, fontSize: T.typography.sizeLg, fontWeight: T.typography.weightSemibold, color: C.text }}>
                Data Inventory
              </h3>
              <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
                What the backend knows about. Sources, facts, training files and domain coverage in one glance.
              </p>
              {err.inventory && (
                <div style={{ padding: T.spacing.md, borderRadius: T.radii.md, background: C.redBg, border: `1px solid ${C.redBorder}`, color: C.red, fontSize: T.typography.sizeSm, marginBottom: T.spacing.md }}>
                  {err.inventory}
                </div>
              )}
              {/* ---- Big-number row ---- */}
              <div style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {(() => {
                  const ov = dashboard?.overview || {};
                  const cards: Array<{ label: string; value: string; color: string }> = [
                    { label: 'Total facts',    value: compactNum(ov.total_facts ?? factsCount),  color: C.accent },
                    { label: 'Sources',        value: compactNum(ov.total_sources ?? sourcesCount), color: C.green },
                    { label: 'Training pairs', value: compactNum(ov.total_training_pairs ?? 0),  color: C.yellow },
                    { label: 'CVE facts',      value: compactNum(ov.cve_facts ?? 0),             color: C.accent },
                    { label: 'Adversarial',    value: compactNum(ov.adversarial_facts ?? 0),     color: C.red },
                    { label: 'Domains',        value: compactNum(dashboard?.domains?.length ?? 0), color: C.green },
                  ];
                  return cards.map(c => (
                    <div key={c.label} style={{
                      padding: '14px 16px', borderRadius: T.radii.md,
                      background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                    }}>
                      <Label color={C.textMuted}>{c.label}</Label>
                      <div style={{ fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack, color: c.color, marginTop: '4px', fontFamily: T.typography.fontMono }}>{c.value}</div>
                    </div>
                  ));
                })()}
              </div>
              {/* ---- Training files table ---- */}
              {(() => {
                const files = dashboard?.training_files || [];
                if (files.length === 0) {
                  return (
                    <div style={{ padding: T.spacing.lg, borderRadius: T.radii.md, background: C.bgInput, border: `1px dashed ${C.borderSubtle}`, color: C.textDim, fontSize: '12.5px', textAlign: 'center' }}>
                      No training files reported. When /api/admin/dashboard returns training_files, they'll appear here.
                    </div>
                  );
                }
                const totalPairs = files.reduce((s, f) => s + (f.pairs || 0), 0);
                const totalMb = files.reduce((s, f) => s + (f.size_mb || 0), 0);
                // c2-375: Training tab files migrated to DataTable so the
                // columns sort for free (previously fixed pairs desc).
                type TFRow = { file: string; pairs: number; size_mb: number };
                const cols: ReadonlyArray<Column<TFRow>> = [
                  {
                    id: 'file', header: 'File', align: 'left',
                    sortKey: (f) => f.file.toLowerCase(),
                    accessor: (f) => (
                      <span style={{
                        color: C.text, fontFamily: T.typography.fontMono,
                        whiteSpace: 'nowrap', overflow: 'hidden',
                        textOverflow: 'ellipsis', display: 'inline-block',
                        maxWidth: '420px',
                      }}>{f.file}</span>
                    ),
                  },
                  {
                    id: 'pairs', header: 'Pairs', align: 'right',
                    sortKey: (f) => f.pairs,
                    accessor: (f) => <span style={{ color: C.text, fontFamily: T.typography.fontMono }}>{compactNum(f.pairs)}</span>,
                  },
                  {
                    id: 'size', header: 'Size (MB)', align: 'right',
                    sortKey: (f) => f.size_mb,
                    accessor: (f) => <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{f.size_mb.toFixed(2)}</span>,
                  },
                ];
                return (
                  <div style={{ marginBottom: T.spacing.xl }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: '6px' }}>
                      <Label color={C.textMuted}>
                        Training files ({files.length})
                      </Label>
                      <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
                        {compactNum(totalPairs)} pairs · {totalMb.toFixed(1)} MB
                      </div>
                    </div>
                    <div style={{ maxHeight: '300px', overflowY: 'auto' }}>
                      <DataTable<TFRow> C={C}
                        rows={files as TFRow[]}
                        columns={cols}
                        rowKey={(f) => f.file}
                        sort={{ col: 'pairs', dir: 'desc' }} />
                    </div>
                  </div>
                );
              })()}
              {/* ---- Top domains quick list ---- */}
              {(() => {
                const doms = dashboard?.domains || [];
                if (doms.length === 0) return null;
                const top = [...doms].sort((a, b) => b.count - a.count).slice(0, 10);
                return (
                  <div>
                    <Label color={C.textMuted} mb={'6px'}>
                      Top 10 domains
                    </Label>
                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '6px 14px' }}>
                      {top.map(d => (
                        <div key={d.domain} style={{ display: 'flex', justifyContent: 'space-between', gap: T.spacing.sm, fontSize: T.typography.sizeSm, fontFamily: T.typography.fontMono, padding: '3px 0' }}>
                          <span style={{ color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{d.domain}</span>
                          <span style={{ color: countColor(d.count) }}>{d.count.toLocaleString()}</span>
                        </div>
                      ))}
                    </div>
                    {doms.length > 10 && (
                      <div style={{ marginTop: '6px', fontSize: T.typography.sizeXs, color: C.textDim }}>
                        +{doms.length - 10} more — see <button onClick={() => setTab('domains')} style={{ background: 'transparent', border: 'none', color: C.accent, cursor: 'pointer', padding: 0, fontFamily: 'inherit', fontSize: 'inherit' }}>Domains</button> for the full filterable table.
                      </div>
                    )}
                  </div>
                );
              })()}
            </div>
          )}

          {/* ---------- Domains (searchable + sortable table) ---------- */}
          {tab === 'domains' && (
            <div>
              <div style={{ display: 'flex', gap: T.spacing.md, marginBottom: T.spacing.md, alignItems: 'center' }}>
                <input
                  type='search' autoComplete='off' spellCheck={false}
                  aria-label='Filter domains'
                  placeholder={`Filter ${domains?.length ?? 0} domains…`}
                  value={domainFilter}
                  onChange={(e) => setDomainFilter(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Escape' && domainFilter) { e.preventDefault(); setDomainFilter(''); } }}
                  style={{
                    flex: 1, padding: '10px 12px',
                    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                    borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
                    fontSize: T.typography.sizeBody, outline: 'none',
                  }} />
                <button onClick={() => loadTab('domains')}
                  disabled={loading === 'domains'}
                  style={{
                    padding: '10px 16px', background: C.accentBg, color: C.accent,
                    border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.md,
                    fontFamily: 'inherit', fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBold,
                    cursor: loading === 'domains' ? 'wait' : 'pointer',
                  }}>{loading === 'domains' ? 'Loading…' : 'Refresh'}</button>
              </div>
              {err.domains && <ErrorAlert C={C} message={err.domains} onRetry={() => loadTab('domains')} retrying={loading === 'domains'} />}
              {/* c2-377 / BIG #180: domains table migrated to DataTable.
                  Existing domainSort + filteredDomains state is lifted into
                  DataTable via sort+onSortChange so the DomainRow type-safe
                  SortKey stays authoritative. Filter input above is
                  unchanged -- DataTable consumes the already-filtered
                  array. */}
              {(() => {
                const cols: ReadonlyArray<Column<DomainRow>> = [
                  {
                    id: 'domain', header: 'Domain', align: 'left',
                    sortKey: (d) => d.domain.toLowerCase(),
                    accessor: (d) => <span style={{ fontWeight: T.typography.weightSemibold }}>{d.domain}</span>,
                  },
                  {
                    id: 'facts', header: 'Facts', align: 'right',
                    sortKey: (d) => d.facts,
                    accessor: (d) => (
                      <span style={{ color: countColor(d.facts), fontWeight: T.typography.weightBold, fontFamily: T.typography.fontMono }}>
                        {d.facts.toLocaleString()}
                      </span>
                    ),
                  },
                  {
                    id: 'avg_quality', header: 'Avg Quality', align: 'right',
                    sortKey: (d) => typeof d.avg_quality === 'number' ? d.avg_quality : -1,
                    accessor: (d) => (
                      <span style={{ color: typeof d.avg_quality === 'number' ? qualityColor(d.avg_quality) : C.textMuted, fontFamily: T.typography.fontMono }}>
                        {typeof d.avg_quality === 'number' ? d.avg_quality.toFixed(2) : '\u2014'}
                      </span>
                    ),
                  },
                  {
                    id: 'avg_length', header: 'Avg Length', align: 'right',
                    sortKey: (d) => typeof d.avg_length === 'number' ? d.avg_length : -1,
                    accessor: (d) => (
                      <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>
                        {typeof d.avg_length === 'number' ? d.avg_length.toFixed(0) : '\u2014'}
                      </span>
                    ),
                  },
                ];
                return (
                  <DataTable<DomainRow> C={C}
                    rows={filteredDomains}
                    columns={cols}
                    rowKey={(d) => d.domain}
                    sort={{ col: domainSort.key as string, dir: domainSort.dir }}
                    onSortChange={(next) => setDomainSort({ key: next.col as keyof DomainRow, dir: next.dir })}
                    emptyText={domains === null ? 'Loading\u2026' : 'No domains match.'}
                    cellFontSize={T.typography.sizeMd} />
                );
              })()}
              <div style={{ marginTop: T.spacing.sm, fontSize: T.typography.sizeSm, color: C.textDim }}>
                {filteredDomains.length} of {domains?.length ?? 0} domains
                {domains && ` · ${domains.reduce((s, d) => s + d.facts, 0).toLocaleString()} facts total`}
              </div>
            </div>
          )}

          {/* ---------- Training ---------- */}
          {tab === 'training' && (
            <div>
              {err.training && <ErrorAlert C={C} message={err.training} onRetry={() => loadTab('training')} retrying={loading === 'training'} />}
              {accuracy ? (
                <>
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: T.spacing.md, marginBottom: T.spacing.xl }}>
                    {(() => {
                      const p = pctNorm(accuracy.pass_rate ?? accuracy.accuracy);
                      return (
                        <div style={{
                          padding: '16px 18px', borderRadius: T.radii.xl,
                          background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                        }}>
                          <Label color={C.textMuted}>
                            Pass rate
                          </Label>
                          <div style={{
                            fontSize: dsType.sizes['3xl'], fontWeight: T.typography.weightBlack,
                            color: p != null ? (p >= 95 ? C.green : p >= 80 ? C.yellow : C.red) : C.textMuted,
                            marginTop: '4px', fontFamily: T.typography.fontMono,
                          }}>{p != null ? `${p.toFixed(1)}%` : '—'}</div>
                        </div>
                      );
                    })()}
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <Label color={C.textMuted}>Samples</Label>
                      <div style={{ fontSize: dsType.sizes['3xl'], fontWeight: T.typography.weightBlack, color: C.text, marginTop: '4px', fontFamily: T.typography.fontMono }}>
                        {typeof accuracy.samples === 'number' ? accuracy.samples.toLocaleString() : '—'}
                      </div>
                    </div>
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <Label color={C.textMuted}>Last run</Label>
                      <div style={{ fontSize: T.typography.sizeMd, color: C.text, marginTop: '10px' }}>
                        {accuracy.last_run ? (typeof accuracy.last_run === 'number' ? new Date(accuracy.last_run * 1000).toLocaleString() : accuracy.last_run) : '—'}
                      </div>
                    </div>
                  </div>
                  {accuracy.per_domain && Object.keys(accuracy.per_domain).length > 0 && (
                    <div>
                      <Label color={C.textMuted} mb={T.spacing.md}>
                        Accuracy by domain
                      </Label>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        {Object.entries(accuracy.per_domain).sort((a, b) => (b[1] ?? 0) - (a[1] ?? 0)).map(([dom, v]) => {
                          const p = pctNorm(v) ?? 0;
                          return (
                            <div key={dom} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                              <span style={{ width: '160px', fontSize: T.typography.sizeSm, color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{dom}</span>
                              <div style={{ flex: 1, background: C.bgInput, height: '14px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                                <div style={{
                                  width: `${p}%`, height: '100%',
                                  background: p >= 95 ? C.green : p >= 80 ? C.yellow : C.red,
                                }} />
                              </div>
                              <span style={{ width: '64px', textAlign: 'right', fontSize: T.typography.sizeSm, fontFamily: T.typography.fontMono, color: C.textMuted }}>{p.toFixed(1)}%</span>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </>
              ) : (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'training' ? 'Loading…' : 'No training data loaded.'}
                </div>
              )}
            </div>
          )}

          {/* ---------- Quality ---------- */}
          {tab === 'quality' && (
            <div>
              {err.quality && <ErrorAlert C={C} message={err.quality} onRetry={() => loadTab('quality')} retrying={loading === 'quality'} />}
              {quality ? (
                <>
                  {/* c2-424 / task 207: quality.average sparkline. Rendered
                      above the headline DashCards so trend-at-a-glance is
                      the first thing users see when the tab opens. Hidden
                      until we have >=2 snapshots so a fresh install doesn't
                      flash a flat line. */}
                  {qualityHistory.length >= 2 && (() => {
                    const values = qualityHistory.map(s => s.avg);
                    const max = Math.max(...values);
                    const min = Math.min(...values);
                    const range = max - min;
                    const W = 240, H = 32;
                    const step = W / (values.length - 1);
                    const y = (v: number) => range === 0 ? H / 2 : H - ((v - min) / range) * (H - 2) - 1;
                    const points = values.map((v, i) => `${(i * step).toFixed(1)},${y(v).toFixed(1)}`).join(' ');
                    const latest = values[values.length - 1];
                    const first = values[0];
                    const delta = latest - first;
                    const trendColor = delta > 0.01 ? C.green : delta < -0.01 ? C.red : C.textDim;
                    return (
                      <div style={{
                        display: 'flex', alignItems: 'center', gap: T.spacing.lg,
                        padding: T.spacing.md, marginBottom: T.spacing.lg,
                        background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                        borderRadius: T.radii.md, flexWrap: 'wrap',
                      }}>
                        <div>
                          <div style={{
                            fontSize: '10px', color: C.textMuted, textTransform: 'uppercase',
                            letterSpacing: T.typography.trackingLoose, fontWeight: T.typography.weightBold,
                          }}>Avg quality trend</div>
                          <div style={{
                            fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBlack,
                            color: trendColor, fontFamily: T.typography.fontMono,
                          }}>
                            {latest.toFixed(3)} <span style={{ fontSize: T.typography.sizeXs, opacity: 0.7 }}>
                              {delta >= 0 ? '+' : ''}{delta.toFixed(3)}
                            </span>
                          </div>
                        </div>
                        <svg width={W} height={H} role='img'
                          aria-label={`Quality trend, ${qualityHistory.length} samples, latest ${latest.toFixed(3)}`}
                          style={{ display: 'block' }}>
                          <polyline fill='none' stroke={trendColor} strokeWidth='1.8'
                            strokeLinecap='round' strokeLinejoin='round' points={points} />
                        </svg>
                        <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono, marginLeft: 'auto' }}>
                          {qualityHistory.length} samples &middot; last {new Date(qualityHistory[qualityHistory.length - 1].ts).toLocaleString()}
                        </span>
                      </div>
                    );
                  })()}
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: T.spacing.md, marginBottom: T.spacing.xl }}>
                    <DashCard C={C} label='Adversarial' value={quality.adversarial_count != null ? compactNum(quality.adversarial_count) : '—'} color={C.red} />
                    <DashCard C={C} label='Distinct sources' value={quality.distinct_sources != null ? String(quality.distinct_sources) : '—'} color={C.purple} />
                    <DashCard C={C} label='PSL pass rate' value={(() => {
                      const p = pctNorm(quality.psl_calibration?.pass_rate);
                      return p != null ? `${p.toFixed(1)}%` : '—';
                    })()} color={C.green} />
                    <DashCard C={C} label='FTS5 index' value={quality.fts5_enabled ? 'enabled' : 'disabled'} color={quality.fts5_enabled ? C.green : C.red} />
                    <DashCard C={C} label='PSL status' value={quality.psl_calibration?.status || '—'} color={C.accent} />
                    <DashCard C={C} label='PSL last run' value={quality.psl_calibration?.last_run || '—'} color={C.textSecondary} />
                  </div>
                  {/* Quality distribution histogram — only renders when the
                      backend actually includes the shape. Sorts buckets by
                      key so numeric bucket labels (0.0-0.2 etc.) come first. */}
                  {quality.quality_distribution && Object.keys(quality.quality_distribution).length > 0 && (() => {
                    const entries = Object.entries(quality.quality_distribution).sort((a, b) => a[0].localeCompare(b[0], undefined, { numeric: true }));
                    const max = Math.max(...entries.map(([, v]) => v), 1);
                    const total = entries.reduce((s, [, v]) => s + v, 0);
                    return (
                      <div>
                        <Label color={C.textMuted} mb={T.spacing.md}>
                          Quality distribution
                        </Label>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          {entries.map(([bucket, n]) => {
                            // Color-grade buckets by their label — numeric
                            // (0.x) low→red→yellow→green; named (low/med/high)
                            // just map directly.
                            const numericMid = (() => {
                              const m = bucket.match(/(\d+\.?\d*)/g);
                              if (!m) return null;
                              const nums = m.map(parseFloat);
                              return (nums.reduce((a, b) => a + b, 0) / nums.length);
                            })();
                            const color = numericMid != null
                              ? (numericMid >= 0.75 ? C.green : numericMid >= 0.5 ? C.yellow : C.red)
                              : (bucket.includes('high') ? C.green : bucket.includes('low') ? C.red : bucket.includes('med') ? C.yellow : C.accent);
                            const pct = (n / total) * 100;
                            return (
                              <div key={bucket} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                                <span style={{ width: '120px', fontSize: T.typography.sizeSm, color: C.text, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>{bucket}</span>
                                <BarChart C={C} value={(n / max) * 100} color={color} height='18px' trackBg={C.bgInput} style={{ flex: 1 }} />
                                <span style={{ width: '96px', textAlign: 'right', fontSize: T.typography.sizeSm, fontFamily: T.typography.fontMono, color: C.textMuted }}>
                                  {n.toLocaleString()} ({pct.toFixed(1)}%)
                                </span>
                              </div>
                            );
                          })}
                        </div>
                        <div style={{ marginTop: T.spacing.sm, fontSize: T.typography.sizeXs, color: C.textDim, textAlign: 'right' }}>
                          Total: {total.toLocaleString()}
                        </div>
                      </div>
                    );
                  })()}
                </>
              ) : (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'quality' ? 'Loading…' : 'No quality data loaded.'}
                </div>
              )}
            </div>
          )}

          {/* ---------- System ---------- */}
          {tab === 'system' && (
            <div>
              {err.system && <ErrorAlert C={C} message={err.system} onRetry={() => loadTab('system')} retrying={loading === 'system'} />}
              {sysInfo ? (
                <>
                  {/* Resource gauges — CPU temp / RAM / Disk as horizontal
                      bars so saturation is obvious at a glance. */}
                  <div style={{ marginBottom: T.spacing.xl }}>
                    {(() => {
                      // CPU temp gauge (0 baseline, 100°C ceiling is a fine
                      // upper for desktop — most CPUs throttle at 95-100°C).
                      const temp = sysInfo.cpu_temp_c;
                      const tempPct = typeof temp === 'number' ? Math.min(100, Math.max(0, temp)) : 0;
                      const tempColor = temp == null ? C.textMuted : temp > 80 ? C.red : temp > 65 ? C.yellow : C.green;
                      // RAM: if total available, compute used%; else show
                      // available MB raw.
                      const ramTotal = sysInfo.ram_total_mb;
                      const ramAvail = sysInfo.ram_available_mb;
                      const ramUsedPct = (typeof ramTotal === 'number' && ramTotal > 0 && typeof ramAvail === 'number')
                        ? Math.min(100, Math.max(0, ((ramTotal - ramAvail) / ramTotal) * 100)) : null;
                      const ramColor = ramUsedPct == null ? C.textMuted : ramUsedPct > 90 ? C.red : ramUsedPct > 75 ? C.yellow : C.green;
                      // Disk: used% from free/total.
                      const diskFree = sysInfo.disk_root_free_bytes;
                      const diskTotal = sysInfo.disk_root_total_bytes;
                      const diskUsedPct = (typeof diskFree === 'number' && typeof diskTotal === 'number' && diskTotal > 0)
                        ? Math.min(100, Math.max(0, ((diskTotal - diskFree) / diskTotal) * 100)) : null;
                      const diskColor = diskUsedPct == null ? C.textMuted : diskUsedPct > 90 ? C.red : diskUsedPct > 75 ? C.yellow : C.green;
                      const rows: Array<{ label: string; pct: number | null; color: string; right: string }> = [
                        { label: 'CPU temp', pct: typeof temp === 'number' ? tempPct : null, color: tempColor, right: typeof temp === 'number' ? `${temp.toFixed(0)}°C` : '—' },
                        { label: 'RAM used', pct: ramUsedPct, color: ramColor, right: ramUsedPct != null ? `${ramUsedPct.toFixed(0)}% · ${ramAvail ?? '?'} MB free` : (typeof ramAvail === 'number' ? `${ramAvail} MB free` : '—') },
                        { label: 'Disk used', pct: diskUsedPct, color: diskColor, right: diskUsedPct != null ? `${diskUsedPct.toFixed(0)}% · ${fmtBytes(diskFree)} free / ${fmtBytes(diskTotal)}` : '—' },
                      ];
                      return (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.md }}>
                          {rows.map(r => (
                            <div key={r.label}>
                              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: '6px' }}>
                                <span style={{ fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>{r.label}</span>
                                <span style={{ fontSize: T.typography.sizeSm, color: r.color, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{r.right}</span>
                              </div>
                              <BarChart C={C} value={r.pct ?? 0} color={r.color} height='14px'
                                trackBg={C.bgInput}
                                style={{ border: `1px solid ${C.borderSubtle}` }} />
                            </div>
                          ))}
                        </div>
                      );
                    })()}
                  </div>
                  {/* Descriptive cards remain for info that isn't gauge-shaped */}
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: T.spacing.md }}>
                    <DashCard C={C} label='Hostname' value={sysInfo.hostname || '—'} color={C.accent} />
                    <DashCard C={C} label='OS' value={sysInfo.os || '—'} color={C.textSecondary} />
                    <DashCard C={C} label='CPU cores' value={sysInfo.cpu_count != null ? String(sysInfo.cpu_count) : '—'} color={C.purple} />
                    <DashCard C={C} label='Uptime' value={fmtSeconds(sysInfo.uptime_seconds)} color={C.yellow} />
                    {sysInfo.ollama && (
                      <DashCard C={C} label='Ollama' value={sysInfo.ollama.status || '—'} color={sysInfo.ollama.status === 'up' ? C.green : C.red} />
                    )}
                  </div>
                </>
              ) : (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'system' ? 'Loading…' : 'No system data loaded.'}
                </div>
              )}
            </div>
          )}

          {/* ---------- Fleet (c0-031 autonomous directive #7) ---------- */}
          {tab === 'fleet' && (
            <div>
              {err.fleet && <ErrorAlert C={C} message={err.fleet} onRetry={() => loadTab('fleet')} retrying={loading === 'fleet'} />}
              {fleet === null && !err.fleet && (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'fleet' ? 'Loading fleet…' : 'Fleet endpoint not yet responsive.'}
                </div>
              )}
              {fleet && (
                <>
                  {fleet.stats && (
                    <div style={{
                      display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
                      gap: T.spacing.md, marginBottom: T.spacing.xl,
                    }}>
                      <DashCard C={C} label='Instances' value={String(fleet.instances?.length ?? 0)} color={C.accent} />
                      <DashCard C={C} label='Tasks total' value={typeof fleet.stats.total_tasks === 'number' ? String(fleet.stats.total_tasks) : '—'} color={C.purple} />
                      <DashCard C={C} label='Running' value={typeof fleet.stats.running === 'number' ? String(fleet.stats.running) : '—'} color={C.yellow} />
                      <DashCard C={C} label='Completed' value={typeof fleet.stats.completed === 'number' ? String(fleet.stats.completed) : '—'} color={C.green} />
                    </div>
                  )}
                  {fleet.instances && fleet.instances.length > 0 && (
                    <div style={{
                      display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
                      gap: T.spacing.md, marginBottom: T.spacing.xl,
                    }}>
                      {fleet.instances.map(inst => {
                        const statusColor = inst.status === 'running' ? C.green
                          : inst.status === 'error' ? C.red
                          : inst.status === 'idle' ? C.yellow : C.textMuted;
                        return (
                          <div key={inst.id} style={{
                            padding: T.spacing.lg, borderRadius: T.radii.md,
                            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                          }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: T.spacing.sm }}>
                              <div style={{ fontSize: T.typography.sizeBody, fontWeight: T.typography.weightBold, color: C.text }}>
                                {inst.name || inst.id}
                              </div>
                              <span style={{
                                display: 'inline-flex', alignItems: 'center', gap: '6px',
                                fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                                color: statusColor, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                              }}>
                                <span style={{ width: '8px', height: '8px', borderRadius: '50%', background: statusColor }} aria-hidden='true' />
                                {inst.status || 'unknown'}
                              </span>
                            </div>
                            {inst.role && (
                              <div style={{ fontSize: T.typography.sizeSm, color: C.textSecondary, marginBottom: '6px' }}>
                                {inst.role}
                              </div>
                            )}
                            {inst.current_task && (
                              <div style={{
                                padding: '6px 8px', background: C.bg, borderRadius: T.radii.sm,
                                fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono,
                                marginBottom: '6px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                              }}>
                                {inst.current_task}
                              </div>
                            )}
                            <div style={{ display: 'flex', gap: T.spacing.md, fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono }}>
                              {typeof inst.tasks_completed === 'number' && <span>✓ {inst.tasks_completed}</span>}
                              {typeof inst.tasks_pending === 'number' && <span>⏳ {inst.tasks_pending}</span>}
                              {inst.last_seen && <span style={{ marginLeft: 'auto' }}>
                                last seen {typeof inst.last_seen === 'number' ? new Date(inst.last_seen * (inst.last_seen < 1e12 ? 1000 : 1)).toLocaleTimeString() : inst.last_seen}
                              </span>}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                  {fleet.timeline && fleet.timeline.length > 0 && (() => {
                    // c2-376 / BIG #180: AdminModal Fleet tab timeline now
                    // uses DataTable. Same shape as FleetView's timeline
                    // migration (c2-373), different row cap (100 here vs 200
                    // there) because this is the embedded preview rather
                    // than the standalone page.
                    type TRow = { t: number | string; instance: string; event: string };
                    const toTs = (r: TRow): number => typeof r.t === 'number'
                      ? r.t * (r.t < 1e12 ? 1000 : 1)
                      : new Date(r.t).getTime();
                    const cols: ReadonlyArray<Column<TRow>> = [
                      {
                        id: 'when', header: 'When', width: '110px',
                        sortKey: toTs,
                        accessor: (r) => (
                          <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>
                            {new Date(toTs(r)).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                          </span>
                        ),
                      },
                      {
                        id: 'who', header: 'Who', width: '30%',
                        sortKey: (r) => r.instance,
                        accessor: (r) => <span style={{ color: C.accent, fontFamily: T.typography.fontMono }}>{r.instance}</span>,
                      },
                      {
                        id: 'event', header: 'Event',
                        sortKey: (r) => r.event,
                        accessor: (r) => <span style={{ color: C.text, fontFamily: T.typography.fontMono }}>{r.event}</span>,
                      },
                    ];
                    const rows = fleet.timeline.slice(0, 100) as TRow[];
                    return (
                      <div>
                        <Label color={C.textMuted} mb={T.spacing.md}>
                          Recent activity ({fleet.timeline.length})
                        </Label>
                        <div style={{ maxHeight: '320px', overflowY: 'auto' }}>
                          <DataTable<TRow> C={C}
                            rows={rows}
                            columns={cols}
                            rowKey={(r) => `${toTs(r)}-${r.instance}-${r.event}`}
                            sort={{ col: 'when', dir: 'desc' }} />
                        </div>
                      </div>
                    );
                  })()}
                </>
              )}
            </div>
          )}

          {/* ---------- Logs ---------- */}
          {tab === 'logs' && (
            <div>
              {err.logs && <ErrorAlert C={C} message={err.logs} onRetry={() => loadTab('logs')} retrying={loading === 'logs'} />}
              {/* Server logs (primary) */}
              {logs && logs.length > 0 && (
                <div style={{ marginBottom: T.spacing.lg }}>
                  <Label color={C.textMuted} mb={'6px'}>
                    Server log ({logs.length} lines)
                  </Label>
                  <pre style={{
                    margin: 0, padding: T.spacing.lg, background: C.bgInput,
                    border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
                    fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: T.typography.sizeMd,
                    color: C.text, whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                    maxHeight: '45vh', overflowY: 'auto',
                  }}>{logs.slice(-500).join('\n')}</pre>
                </div>
              )}
              {(logs === null || logs.length === 0) && !err.logs && (
                <div style={{
                  padding: T.spacing.lg, marginBottom: T.spacing.lg,
                  background: C.bgInput, border: `1px dashed ${C.borderSubtle}`,
                  borderRadius: T.radii.md, color: C.textMuted, fontSize: T.typography.sizeMd, textAlign: 'center',
                }}>
                  {loading === 'logs' ? 'Loading server log…' : 'Server /api/admin/logs endpoint not available yet — showing client events only.'}
                </div>
              )}
              {/* Client-side event log (fallback / supplement) */}
              {localEvents && localEvents.length > 0 && (() => {
                const q = logFilter.trim().toLowerCase();
                const filtered = [...localEvents].reverse().filter(e =>
                  !q || e.kind.toLowerCase().includes(q) || (e.data && JSON.stringify(e.data).toLowerCase().includes(q))
                );
                // Count unique kinds for the summary-pill row. Sorted by
                // frequency desc so high-signal kinds come first.
                const kindCounts = new Map<string, number>();
                for (const e of localEvents) kindCounts.set(e.kind, (kindCounts.get(e.kind) || 0) + 1);
                const sortedKinds = [...kindCounts.entries()].sort((a, b) => b[1] - a[1]);
                return (
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.md, marginBottom: '6px', flexWrap: 'wrap' }}>
                      <Label color={C.textMuted}>
                        Client events ({filtered.length} of {localEvents.length}, this session)
                      </Label>
                      <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center' }}>
                        <input
                          type='search' value={logFilter} onChange={e => setLogFilter(e.target.value)}
                          placeholder='Filter kind or data…'
                          autoComplete='off' spellCheck={false}
                          aria-label='Filter client events'
                          onKeyDown={(e) => { if (e.key === 'Escape' && logFilter) { e.preventDefault(); setLogFilter(''); } }}
                          style={{
                            minWidth: '200px', padding: '6px 10px',
                            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                            borderRadius: T.radii.sm, color: C.text, fontFamily: 'inherit',
                            fontSize: T.typography.sizeSm, outline: 'none',
                          }}
                        />
                        {/* c2-262: explicit Clear button — browsers render
                            the type=search ✕ inconsistently in dark themes.
                            Esc in the input does the same thing. */}
                        {logFilter && (
                          <button onClick={(e) => {
                            setLogFilter('');
                            // c2-300: return focus to the filter input so the
                            // user can keep typing without a tab/click
                            // detour. Previous sibling walk avoids a new ref.
                            const input = (e.currentTarget.previousElementSibling as HTMLInputElement | null);
                            input?.focus?.();
                          }}
                            aria-label='Clear filter'
                            title='Clear filter (Esc)'
                            style={{
                              background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                              color: C.textMuted, borderRadius: T.radii.sm, cursor: 'pointer',
                              padding: '4px 8px', fontSize: T.typography.sizeXs,
                              fontFamily: 'inherit', textTransform: 'uppercase',
                              letterSpacing: '0.06em',
                            }}>Clear</button>
                        )}
                        {/* Pills removed below in favor of a dedicated row. */}
                        <button onClick={() => {
                          // Export the currently-filtered events as JSON so the
                          // user can attach them to a support ticket without
                          // copying from the table manually.
                          const payload = { exportedAt: new Date().toISOString(), filter: logFilter, events: filtered };
                          const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
                          a.download = `plausiden-events-${stamp}.json`;
                          document.body.appendChild(a); a.click(); a.remove();
                          URL.revokeObjectURL(url);
                        }}
                          aria-label='Export client events as JSON'
                          title={filtered.length === 0 ? 'No events to export' : 'Export filtered events as JSON'}
                          disabled={filtered.length === 0}
                          style={{
                            padding: '6px 12px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                            background: filtered.length === 0 ? C.bgInput : C.accentBg,
                            border: `1px solid ${filtered.length === 0 ? C.borderSubtle : C.accentBorder}`,
                            color: filtered.length === 0 ? C.textMuted : C.accent,
                            borderRadius: T.radii.sm,
                            cursor: filtered.length === 0 ? 'not-allowed' : 'pointer',
                            fontFamily: 'inherit', textTransform: 'uppercase',
                          }}>Export JSON</button>
                        {/* c2-385 / BIG #183: CSV export alongside the JSON
                            one. Spreadsheet-friendly format: t (ISO), kind,
                            data (JSON-stringified, double-quote-escaped per
                            RFC 4180). Useful for quick pivot-table triage
                            of a session's client events. */}
                        <button onClick={() => {
                          const escape = (v: string) => `"${v.replace(/"/g, '""')}"`;
                          const header = 'timestamp,kind,data\n';
                          const body = filtered.map(e => [
                            escape(new Date(e.t).toISOString()),
                            escape(e.kind),
                            escape(e.data ? JSON.stringify(e.data) : ''),
                          ].join(',')).join('\n');
                          const blob = new Blob([header + body + '\n'], { type: 'text/csv;charset=utf-8' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
                          a.download = `plausiden-events-${stamp}.csv`;
                          document.body.appendChild(a); a.click(); a.remove();
                          URL.revokeObjectURL(url);
                        }}
                          aria-label='Export client events as CSV'
                          title={filtered.length === 0 ? 'No events to export' : 'Export filtered events as CSV'}
                          disabled={filtered.length === 0}
                          style={{
                            marginLeft: T.spacing.xs,
                            padding: '6px 12px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                            background: filtered.length === 0 ? C.bgInput : C.accentBg,
                            border: `1px solid ${filtered.length === 0 ? C.borderSubtle : C.accentBorder}`,
                            color: filtered.length === 0 ? C.textMuted : C.accent,
                            borderRadius: T.radii.sm,
                            cursor: filtered.length === 0 ? 'not-allowed' : 'pointer',
                            fontFamily: 'inherit', textTransform: 'uppercase',
                          }}>Export CSV</button>
                      </div>
                    </div>
                    {/* Kind-frequency pills — scannable summary. Click to
                        filter the table to that kind. Click again to clear. */}
                    <div style={{ display: 'flex', gap: T.spacing.xs, flexWrap: 'wrap', marginBottom: T.spacing.sm }}>
                      {sortedKinds.slice(0, 12).map(([kind, n]) => {
                        const active = logFilter.trim().toLowerCase() === kind.toLowerCase();
                        const dotColor =
                          kind.includes('error') || kind.includes('failed') || kind.includes('negative') ? C.red
                          : kind.includes('positive') || kind.includes('success') || kind.includes('done') ? C.green
                          : kind.includes('warn') || kind.includes('stop') ? C.yellow
                          : C.accent;
                        return (
                          <button key={kind} onClick={() => setLogFilter(active ? '' : kind)}
                            title={active ? 'Clear filter' : `Filter to ${kind}`}
                            style={{
                              display: 'inline-flex', alignItems: 'center', gap: '6px',
                              padding: '3px 10px', fontSize: T.typography.sizeXs,
                              background: active ? C.accentBg : C.bgInput,
                              border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                              color: C.text, borderRadius: T.radii.pill, cursor: 'pointer',
                              fontFamily: 'inherit', fontWeight: T.typography.weightSemibold,
                            }}>
                            <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: dotColor }} aria-hidden='true' />
                            <span style={{ fontFamily: T.typography.fontMono }}>{kind}</span>
                            <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{n}</span>
                          </button>
                        );
                      })}
                    </div>
                    {/* c2-377 / BIG #180: client events table migrated to
                        DataTable. Preserves dot-color semantics via the
                        Kind accessor; maxHeight scroll container kept since
                        DataTable doesn't own vertical sizing. */}
                    {(() => {
                      type ERow = { t: number; kind: string; data?: unknown };
                      const rows = filtered.slice(0, 200) as ERow[];
                      const dotColorFor = (kind: string): string =>
                        kind.includes('error') || kind.includes('failed') || kind.includes('negative') ? C.red
                        : kind.includes('positive') || kind.includes('success') || kind.includes('done') ? C.green
                        : kind.includes('warn') || kind.includes('stop') ? C.yellow
                        : C.accent;
                      const cols: ReadonlyArray<Column<ERow>> = [
                        {
                          id: 'time', header: 'Time', align: 'left', width: '110px',
                          sortKey: (e) => e.t,
                          accessor: (e) => (
                            <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>
                              {new Date(e.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                            </span>
                          ),
                        },
                        {
                          id: 'kind', header: 'Kind', align: 'left', width: '180px',
                          sortKey: (e) => e.kind,
                          accessor: (e) => {
                            const col = dotColorFor(e.kind);
                            return (
                              <span style={{ fontFamily: T.typography.fontMono, color: col, whiteSpace: 'nowrap' }}>
                                <span aria-hidden='true' style={{
                                  display: 'inline-block', width: '6px', height: '6px',
                                  borderRadius: '50%', background: col, marginRight: '8px',
                                  verticalAlign: 'middle',
                                }} />
                                {e.kind}
                              </span>
                            );
                          },
                        },
                        {
                          id: 'data', header: 'Data', align: 'left', sortable: false,
                          accessor: (e) => (
                            <span style={{
                              color: C.textMuted, fontFamily: T.typography.fontMono,
                              maxWidth: '520px', overflow: 'hidden',
                              textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                              display: 'inline-block',
                            }}>{e.data ? JSON.stringify(e.data) : ''}</span>
                          ),
                        },
                      ];
                      return (
                        <div style={{ maxHeight: '45vh', overflowY: 'auto' }}>
                          <DataTable<ERow> C={C}
                            rows={rows}
                            columns={cols}
                            rowKey={(e) => `${e.t}-${e.kind}`}
                            sort={{ col: 'time', dir: 'desc' }}
                            emptyText='No events match.' />
                        </div>
                      );
                    })()}
                  </div>
                );
              })()}
            </div>
          )}

          {/* c2-433 / #303 capability tokens. Issue/list/revoke scoped
              bearer creds. Backend stores SHA-256 hash at rest; we only
              see the raw token once on issue, in the response — it's
              shown in a one-time copy card, then never again. Every
              issue+revoke is audit-chain logged. */}
          {tab === 'tokens' && (
            <TokensTab C={C} host={host} />
          )}

          {/* c2-433 / #354 proof-stats panel. Reads /api/proof/stats and
              shows the distribution of Lean4 proof verdicts across the
              facts table. Unreachable-verifier is a NO-OP so Unknown
              stays Unknown rather than being downgraded. */}
          {tab === 'proof' && (
            <ProofTab C={C} host={host} />
          )}

          {/* c2-433: in-app diag viewer — reads the diag ring buffer +
              subscribes to new entries. Filter by level + search, Copy
              + Clear. Avoids the DevTools-console path. */}
          {tab === 'diag' && (
            <DiagTab C={C} />
          )}
          {/* claude-0 11:15 ask: surface docs/manager_guide.md inside the
              admin console so operators don't need to SSH into the server
              to read it. Forward-compat — renders helpful 404 card when
              the endpoint isn't yet live. */}
          {tab === 'docs' && (
            <DocsTab C={C} host={host} />
          )}
        </div>
      </div>
    </div>
  );
};

// ---- Private helpers ----

// c2-348: AdminErr helper moved to components/ErrorAlert.tsx.

const DashCard: React.FC<{ C: any; label: string; value: string; color: string }> = ({ C, label, value, color }) => (
  <div style={{
    padding: '16px 18px', borderRadius: T.radii.xl,
    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
  }}>
    <Label color={C.textMuted}>
      {label}
    </Label>
    <div style={{ fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBlack, color, marginTop: '6px', fontFamily: T.typography.fontMono, wordBreak: 'break-word' }}>
      {value}
    </div>
  </div>
);

// c2-433 / #303: capability-tokens manager. List + issue + revoke against
// /api/capability/tokens. The raw secret is only visible once — in the
// POST response — so we surface a one-time "fresh token" card with a copy
// button. Every issue and revoke is audit-chain logged server-side, so
// rate-limit abuse is already visible in the Integrity banner expand.
// Known capabilities per Claude 0's #307 spec: auth, research, hdc_encode.
// Tolerant parser: the list GET may return array / {tokens} / {items}.
const KNOWN_CAPABILITIES = ['auth', 'research', 'hdc_encode', 'ingest', 'admin'] as const;
const TokensTab: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [tokens, setTokens] = useState<any[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [issueCapability, setIssueCapability] = useState<string>(KNOWN_CAPABILITIES[0]);
  const [issueTtl, setIssueTtl] = useState<string>('86400'); // 1 day default
  const [issueLabel, setIssueLabel] = useState<string>('');
  const [issuing, setIssuing] = useState<boolean>(false);
  const [issueErr, setIssueErr] = useState<string | null>(null);
  const [freshToken, setFreshToken] = useState<{ token: string; capability: string; id?: string } | null>(null);
  const [revokingId, setRevokingId] = useState<string | null>(null);
  // c2-433 / #303 followup: two-click confirm pattern. First click sets
  // confirmingRevokeId to the token id + arms a 3s timeout. Second click
  // within that window actually fires the DELETE. Clicking a different
  // token's Revoke or letting the timeout expire cancels the confirm.
  // Prevents fat-finger revocation of live credentials.
  const [confirmingRevokeId, setConfirmingRevokeId] = useState<string | null>(null);
  const confirmTimeoutRef = useRef<number | null>(null);
  // c2-433 / #303 followup: capability filter. 'all' | one of
  // KNOWN_CAPABILITIES | 'revoked'. Lets operators narrow a busy token
  // list to a single scope, or view just the revoked entries waiting
  // for cleanup.
  const [filterCap, setFilterCap] = useState<string>('all');
  // c2-433: per-row click-copy token-id flash state. Holds the id just
  // copied; self-clears after 1.5s. Matches the Ingest-Runs pattern.
  const [copiedTokenId, setCopiedTokenId] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setErr(null);
    try {
      const r = await fetch(`http://${host}:3000/api/capability/tokens`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json();
      const list: any[] = Array.isArray(data) ? data
        : Array.isArray(data?.tokens) ? data.tokens
        : Array.isArray(data?.items) ? data.items
        : [];
      setTokens(list);
    } catch (e: any) {
      setErr(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    load();
    // c2-433 / #303 followup: 60s auto-refresh so expiry urgency coloring
    // advances without manual refresh. Cheap — /api/capability/tokens is
    // a list-only GET scoped to the operator's session.
    const id = window.setInterval(load, 60_000);
    return () => window.clearInterval(id);
    // eslint-disable-next-line
  }, [host]);
  // Unmount cleanup for the revoke-confirm timeout so we don't fire a
  // setState into a stale component if the user closes Admin mid-confirm.
  useEffect(() => () => {
    if (confirmTimeoutRef.current) window.clearTimeout(confirmTimeoutRef.current);
  }, []);

  const issue = async () => {
    setIssuing(true);
    setIssueErr(null);
    setFreshToken(null);
    try {
      const body: Record<string, any> = { capability: issueCapability };
      const ttlSec = Number(issueTtl);
      if (!Number.isNaN(ttlSec) && ttlSec > 0) body.ttl_seconds = ttlSec;
      if (issueLabel.trim()) body.label = issueLabel.trim();
      const r = await fetch(`http://${host}:3000/api/capability/tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json();
      const rawToken = data.token ?? data.secret ?? data.bearer ?? data.value;
      if (rawToken) {
        setFreshToken({ token: String(rawToken), capability: issueCapability, id: data.id ?? data.token_id });
      }
      setIssueLabel('');
      load();
    } catch (e: any) {
      setIssueErr(String(e?.message || e || 'issue failed'));
    } finally {
      setIssuing(false);
    }
  };

  const revoke = async (id: string) => {
    // Clear confirm state the moment we start the POST so a successful
    // response can replace the row cleanly.
    setConfirmingRevokeId(null);
    if (confirmTimeoutRef.current) {
      window.clearTimeout(confirmTimeoutRef.current);
      confirmTimeoutRef.current = null;
    }
    setRevokingId(id);
    try {
      const r = await fetch(`http://${host}:3000/api/capability/tokens/${encodeURIComponent(id)}`, {
        method: 'DELETE',
      });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      load();
    } catch (e: any) {
      setErr(`Revoke failed: ${String(e?.message || e || 'unknown')}`);
    } finally {
      setRevokingId(null);
    }
  };

  // c2-433 / #303 followup: arm the 3s confirm window. If the user clicks
  // Revoke on a different token while a confirm is armed elsewhere, the
  // previous window gets cancelled and the new one starts. Second click
  // on the same id calls revoke() directly.
  const armRevokeConfirm = (id: string) => {
    if (confirmingRevokeId === id) {
      // Second click — actually revoke.
      revoke(id);
      return;
    }
    if (confirmTimeoutRef.current) {
      window.clearTimeout(confirmTimeoutRef.current);
    }
    setConfirmingRevokeId(id);
    confirmTimeoutRef.current = window.setTimeout(() => {
      setConfirmingRevokeId(null);
      confirmTimeoutRef.current = null;
    }, 3000);
  };

  // c2-433 / #303 followup: copyToken gains a 2s Copied ✓ feedback tick
  // so the one-time fresh-token card's Copy button confirms the write
  // before the secret scrolls away forever.
  const [tokenCopiedAt, setTokenCopiedAt] = useState<number>(0);
  const copyToken = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setTokenCopiedAt(Date.now());
      window.setTimeout(() => setTokenCopiedAt(0), 2000);
    } catch { /* blocked */ }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.lg }}>
      {err && <ErrorAlert C={C} message={err} onRetry={load} retrying={loading} />}

      {/* One-time fresh-token card. Shown for one render after a successful
          POST, dismissable; the raw secret is lost the moment it closes. */}
      {freshToken && (
        <div role='alert' style={{
          padding: T.spacing.lg, borderRadius: T.radii.xl,
          background: `${C.green}12`, border: `2px solid ${C.green}`,
          display: 'flex', flexDirection: 'column', gap: T.spacing.sm,
        }}>
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.sm,
          }}>
            <Label color={C.green}>New token · {freshToken.capability}</Label>
            <button onClick={() => setFreshToken(null)}
              aria-label='Dismiss fresh token card'
              style={{
                background: 'transparent', border: 'none', color: C.textMuted,
                fontSize: T.typography.size2xl, cursor: 'pointer', padding: 0,
                lineHeight: 1,
              }}>{'\u2715'}</button>
          </div>
          <div style={{
            fontSize: T.typography.sizeXs, color: C.textSecondary, lineHeight: 1.5,
          }}>
            Copy this secret now — it's hashed at rest and never shown again.
          </div>
          <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center' }}>
            <code style={{
              flex: 1, padding: '8px 10px', background: C.bgInput,
              border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.sm,
              fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
              color: C.text, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
            }}>{freshToken.token}</code>
            <button onClick={() => copyToken(freshToken.token)}
              title={tokenCopiedAt > 0 ? 'Secret copied to clipboard' : 'Copy secret to clipboard'}
              style={{
                background: tokenCopiedAt > 0 ? C.green : C.accent,
                color: '#fff', border: 'none',
                borderRadius: T.radii.sm, padding: '6px 12px',
                cursor: 'pointer', fontFamily: 'inherit', fontWeight: T.typography.weightBold,
                fontSize: T.typography.sizeXs, letterSpacing: '0.04em',
                transition: 'background 180ms',
                flexShrink: 0,
              }}>{tokenCopiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
          </div>
        </div>
      )}

      {/* Issue form */}
      <div style={{
        padding: T.spacing.lg, borderRadius: T.radii.xl,
        background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
        display: 'flex', flexDirection: 'column', gap: T.spacing.sm,
      }}>
        <Label color={C.textMuted}>Issue a new token</Label>
        <div style={{ display: 'flex', gap: T.spacing.sm, flexWrap: 'wrap', alignItems: 'center' }}>
          <select value={issueCapability} onChange={e => setIssueCapability(e.target.value)}
            aria-label='Capability scope'
            style={{
              padding: '6px 10px', background: C.bgCard,
              border: `1px solid ${C.borderSubtle}`, color: C.text,
              borderRadius: T.radii.sm, fontFamily: 'inherit',
              fontSize: T.typography.sizeSm,
            }}>
            {KNOWN_CAPABILITIES.map(c => <option key={c} value={c}>{c}</option>)}
          </select>
          <input type='number' min='60' step='60' value={issueTtl}
            onChange={e => setIssueTtl(e.target.value)}
            aria-label='TTL in seconds'
            placeholder='TTL (seconds)'
            style={{
              // c2-433 / mobile: flex-basis lets the TTL input shrink on
              // narrow viewports instead of forcing a hard 130px.
              padding: '6px 10px', flex: '1 1 110px', maxWidth: '150px',
              minWidth: 0, background: C.bgCard,
              border: `1px solid ${C.borderSubtle}`, color: C.text,
              borderRadius: T.radii.sm, fontFamily: T.typography.fontMono,
              fontSize: T.typography.sizeSm,
            }} />
          <input type='text' value={issueLabel}
            onChange={e => setIssueLabel(e.target.value)}
            placeholder='Label (optional)'
            aria-label='Token label'
            style={{
              flex: 1, minWidth: '160px',
              padding: '6px 10px', background: C.bgCard,
              border: `1px solid ${C.borderSubtle}`, color: C.text,
              borderRadius: T.radii.sm, fontFamily: 'inherit',
              fontSize: T.typography.sizeSm,
            }} />
          <button onClick={issue} disabled={issuing}
            style={{
              background: C.accent, color: '#fff', border: 'none',
              borderRadius: T.radii.sm, padding: '6px 14px',
              cursor: issuing ? 'wait' : 'pointer', fontFamily: 'inherit',
              fontWeight: T.typography.weightBold,
              fontSize: T.typography.sizeSm,
              letterSpacing: '0.04em',
            }}>{issuing ? 'Issuing…' : 'Issue'}</button>
        </div>
        {/* c2-433 / #303 followup: humanize the TTL seconds input so
            operators don't have to mentally convert 86400 → 1 day.
            Blank when input is empty or non-numeric; explicit "no
            expiry" when value is zero. */}
        {(() => {
          const s = Number(issueTtl);
          if (!issueTtl.trim()) return null;
          if (Number.isNaN(s)) {
            return <div style={{ fontSize: '10px', color: C.red, fontFamily: T.typography.fontMono, marginTop: '4px' }}>not a number</div>;
          }
          if (s <= 0) {
            return <div style={{ fontSize: '10px', color: C.textMuted, fontFamily: T.typography.fontMono, marginTop: '4px' }}>= no expiry</div>;
          }
          const humanize = (sec: number): string => {
            if (sec < 60) return `${sec}s`;
            const mins = Math.floor(sec / 60);
            const rs = sec % 60;
            if (mins < 60) return rs > 0 ? `${mins}m ${rs}s` : `${mins}m`;
            const hrs = Math.floor(mins / 60);
            const rm = mins % 60;
            if (hrs < 24) return rm > 0 ? `${hrs}h ${rm}m` : `${hrs}h`;
            const days = Math.floor(hrs / 24);
            const rh = hrs % 24;
            if (days < 7) return rh > 0 ? `${days}d ${rh}h` : `${days}d`;
            const weeks = Math.floor(days / 7);
            const rd = days % 7;
            return rd > 0 ? `${weeks}w ${rd}d` : `${weeks}w`;
          };
          return (
            <div style={{
              fontSize: '10px', color: C.textDim, fontFamily: T.typography.fontMono,
              marginTop: '4px',
            }}>= {humanize(Math.floor(s))}</div>
          );
        })()}
        {/* c2-433 / #303 followup: TTL presets. One-click common durations
            so admins don't retype raw seconds. Active preset (matches the
            current TTL exactly) gets an accent border as a visual confirm. */}
        <div style={{ display: 'flex', gap: '4px', marginTop: '4px', flexWrap: 'wrap' }}>
          {([
            { label: '1h', seconds: 3600 },
            { label: '1d', seconds: 86400 },
            { label: '7d', seconds: 604800 },
            { label: '30d', seconds: 2592000 },
          ]).map(p => {
            const isActive = Number(issueTtl) === p.seconds;
            return (
              <button key={p.label} type='button'
                onClick={() => setIssueTtl(String(p.seconds))}
                title={`Set TTL to ${p.label} (${p.seconds.toLocaleString()}s)`}
                style={{
                  padding: '2px 8px', fontSize: '10px',
                  fontWeight: T.typography.weightBold,
                  background: isActive ? C.accentBg : 'transparent',
                  border: `1px solid ${isActive ? C.accentBorder : C.borderSubtle}`,
                  color: isActive ? C.accent : C.textMuted,
                  borderRadius: T.radii.sm,
                  cursor: 'pointer', fontFamily: T.typography.fontMono,
                  letterSpacing: '0.04em',
                }}>{p.label}</button>
            );
          })}
        </div>
        {issueErr && (
          <div style={{ color: C.red, fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono }}>
            {issueErr}
          </div>
        )}
      </div>

      {/* Existing tokens */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: T.spacing.sm }}>
          <Label color={C.textMuted}>
            Active tokens{tokens ? ` (${tokens.length})` : ''}
          </Label>
          <button onClick={load} disabled={loading}
            style={{
              background: 'transparent', border: `1px solid ${C.borderSubtle}`,
              color: C.textMuted, borderRadius: T.radii.sm,
              cursor: loading ? 'wait' : 'pointer',
              padding: '4px 10px', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            }}>{loading ? 'Refreshing…' : 'Refresh'}</button>
        </div>
        {/* c2-433 / #303 followup: capability filter chips. Renders when
            there are any tokens. All = no filter. Each capability chip
            shows the count for that capability; Revoked shows the count
            of revoked entries. Active chip gets accent border + bg. */}
        {tokens != null && tokens.length > 0 && (() => {
          const capCounts: Record<string, number> = { all: tokens.length };
          let revokedCount = 0;
          for (const t of tokens) {
            const isRevoked = t.revoked === true || t.active === false;
            if (isRevoked) { revokedCount++; continue; }
            const cap: string = t.capability ?? t.scope ?? '?';
            capCounts[cap] = (capCounts[cap] || 0) + 1;
          }
          const chips: Array<{ id: string; label: string; count: number }> = [
            { id: 'all', label: 'All', count: capCounts.all },
            ...KNOWN_CAPABILITIES
              .filter(c => (capCounts[c] || 0) > 0)
              .map(c => ({ id: c, label: c, count: capCounts[c] })),
            ...(revokedCount > 0 ? [{ id: 'revoked', label: 'Revoked', count: revokedCount }] : []),
          ];
          if (chips.length <= 2) return null; // No point rendering when everything is one capability
          return (
            <div role='tablist' aria-label='Filter tokens by capability'
              style={{ display: 'flex', gap: '4px', flexWrap: 'wrap', marginBottom: T.spacing.sm }}>
              {chips.map(c => {
                const isActive = filterCap === c.id;
                return (
                  <button key={c.id} onClick={() => setFilterCap(c.id)}
                    role='tab' aria-selected={isActive}
                    style={{
                      padding: '3px 9px', fontSize: '10px',
                      fontWeight: T.typography.weightBold,
                      background: isActive ? C.accentBg : 'transparent',
                      border: `1px solid ${isActive ? C.accentBorder : C.borderSubtle}`,
                      color: isActive ? C.accent : C.textMuted,
                      borderRadius: T.radii.sm,
                      cursor: 'pointer', fontFamily: T.typography.fontMono,
                      letterSpacing: '0.04em', textTransform: 'uppercase',
                    }}>{c.label} <span style={{ opacity: 0.6, marginLeft: '4px' }}>{c.count}</span></button>
                );
              })}
            </div>
          );
        })()}
        {tokens != null && tokens.length === 0 && !err && (
          <div style={{
            padding: T.spacing.lg, textAlign: 'center',
            color: C.textMuted, fontSize: T.typography.sizeSm,
            background: C.bgInput, border: `1px dashed ${C.borderSubtle}`,
            borderRadius: T.radii.md,
          }}>
            No active capability tokens. Issue one above to scope a bearer credential.
          </div>
        )}
        {tokens != null && tokens.length > 0 && (() => {
          const visible = tokens.filter((t: any) => {
            if (filterCap === 'all') return true;
            const isRevoked = t.revoked === true || t.active === false;
            if (filterCap === 'revoked') return isRevoked;
            return !isRevoked && (t.capability ?? t.scope ?? '?') === filterCap;
          });
          if (visible.length === 0) {
            return (
              <div style={{
                padding: T.spacing.lg, textAlign: 'center',
                color: C.textMuted, fontSize: T.typography.sizeSm, fontStyle: 'italic',
                background: C.bgInput, border: `1px dashed ${C.borderSubtle}`, borderRadius: T.radii.md,
              }}>
                No tokens match the {filterCap} filter. <button onClick={() => setFilterCap('all')}
                  style={{
                    background: 'transparent', border: 'none', color: C.accent,
                    cursor: 'pointer', fontFamily: 'inherit', textDecoration: 'underline',
                    padding: 0, fontSize: T.typography.sizeSm,
                  }}>Clear filter.</button>
              </div>
            );
          }
          return (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {visible.map((t: any, i: number) => {
              const id: string = String(t.id ?? t.token_id ?? t.uuid ?? i);
              const capability: string = t.capability ?? t.scope ?? '?';
              const label: string = t.label ?? '';
              const hashPrefix: string = (t.hash ?? t.token_hash ?? '').toString().slice(0, 12);
              const createdAt = t.created_at ?? t.issued_at;
              const expiresAt = t.expires_at ?? t.expiry;
              const revoked = t.revoked === true || t.active === false;
              return (
                <div key={id} style={{
                  padding: '10px 12px', borderRadius: T.radii.md,
                  background: C.bgCard,
                  border: `1px solid ${revoked ? C.redBorder : C.borderSubtle}`,
                  opacity: revoked ? 0.55 : 1,
                  display: 'flex', alignItems: 'center', gap: T.spacing.sm, flexWrap: 'wrap',
                }}>
                  <span style={{
                    fontSize: '9px', fontWeight: 800,
                    color: revoked ? C.red : C.accent,
                    background: revoked ? C.redBg : C.accentBg,
                    border: `1px solid ${revoked ? C.redBorder : C.accentBorder}`,
                    borderRadius: T.radii.sm, padding: '1px 6px',
                    fontFamily: T.typography.fontMono,
                    textTransform: 'uppercase', letterSpacing: '0.06em',
                  }}>{revoked ? 'revoked' : capability}</span>
                  {/* c2-433: click the truncated token id to copy the
                      full value to clipboard. 1.5s green flash confirms. */}
                  <button type='button'
                    onClick={async () => {
                      try {
                        await navigator.clipboard.writeText(id);
                        setCopiedTokenId(id);
                        window.setTimeout(() => {
                          setCopiedTokenId(prev => prev === id ? null : prev);
                        }, 1500);
                      } catch { /* clipboard blocked */ }
                    }}
                    title={copiedTokenId === id ? `Copied ${id}` : `${id} — click to copy`}
                    aria-label={copiedTokenId === id ? `Copied token id ${id}` : `Copy token id ${id}`}
                    style={{
                      background: 'transparent', border: 'none', padding: 0,
                      fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
                      color: copiedTokenId === id ? C.green : C.textMuted,
                      cursor: 'pointer',
                      textDecoration: 'underline',
                      textDecorationColor: `${copiedTokenId === id ? C.green : C.textMuted}33`,
                      textUnderlineOffset: '2px',
                      transition: 'color 180ms',
                      fontWeight: 600,
                    }}>{copiedTokenId === id ? 'copied \u2713' : `#${id.slice(0, 8)}`}</button>
                  {hashPrefix && (
                    <span title={`SHA-256 hash ${hashPrefix}…`}
                      style={{
                        fontFamily: T.typography.fontMono, fontSize: '10px', color: C.textDim,
                      }}>h:{hashPrefix}…</span>
                  )}
                  {label && (
                    <span style={{
                      flex: 1, minWidth: 0, color: C.text,
                      fontSize: T.typography.sizeSm,
                      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    }}>{label}</span>
                  )}
                  {!label && <span style={{ flex: 1 }} />}
                  {createdAt && (
                    <span style={{
                      fontFamily: T.typography.fontMono, fontSize: '10px', color: C.textDim,
                    }} title={`Issued ${createdAt}`}>iss {formatRelative(typeof createdAt === 'number' ? createdAt : Date.parse(String(createdAt)))}</span>
                  )}
                  {expiresAt && (() => {
                    // c2-433 / #303 followup: expiry urgency coloring. Red
                    // when already expired, red when < 5m remaining, yellow
                    // when < 1h remaining, textDim otherwise. Makes
                    // near-expiry tokens visible at a scan so operators
                    // rotate before a service cliff.
                    const expMs = typeof expiresAt === 'number' ? expiresAt : Date.parse(String(expiresAt));
                    const remaining = expMs - Date.now();
                    const expired = remaining <= 0;
                    const urgent = remaining > 0 && remaining < 5 * 60 * 1000;
                    const soon = remaining >= 5 * 60 * 1000 && remaining < 60 * 60 * 1000;
                    const color = expired || urgent ? C.red : soon ? C.yellow : C.textDim;
                    const weight = (expired || urgent) ? 700 : 400;
                    return (
                      <span style={{
                        fontFamily: T.typography.fontMono, fontSize: '10px',
                        color, fontWeight: weight,
                      }} title={`Expires ${expiresAt}${expired ? ' (already expired)' : ''}`}>
                        {expired ? 'expired' : 'exp'} {formatRelative(expMs)}
                      </span>
                    );
                  })()}
                  {!revoked && (() => {
                    const isConfirming = confirmingRevokeId === id;
                    const isRevoking = revokingId === id;
                    return (
                      <button onClick={() => armRevokeConfirm(id)} disabled={isRevoking}
                        title={isConfirming ? 'Click again within 3s to revoke this token' : 'Revoke this token (requires a second click to confirm)'}
                        style={{
                          background: isConfirming ? C.red : 'transparent',
                          color: isConfirming ? '#fff' : C.red,
                          border: `1px solid ${C.redBorder}`,
                          borderRadius: T.radii.sm, padding: '3px 9px',
                          cursor: isRevoking ? 'wait' : 'pointer',
                          fontFamily: 'inherit', fontWeight: T.typography.weightBold,
                          fontSize: '10px', letterSpacing: '0.04em',
                          textTransform: 'uppercase',
                          animation: isConfirming ? 'scc-admin-integrity-pulse 1.2s ease-in-out infinite' : undefined,
                        }}>{isRevoking ? 'Revoking…' : isConfirming ? 'Confirm?' : 'Revoke'}</button>
                    );
                  })()}
                </div>
              );
            })}
          </div>
          );
        })()}
      </div>
    </div>
  );
};

// c2-433 / #354: Proof-stats panel. Reads /api/proof/stats every 30s and
// renders a small 4-metric grid: Proved / Rejected / Unreachable / Unknown.
// Tolerant parser accepts array / {stats} / {counts} / plain map of
// status→count. Unreachable is yellow (verifier was down, facts kept
// their prior tier); Unknown is muted (never attempted). Proved is green,
// Rejected is red.
const ProofTab: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [stats, setStats] = useState<Record<string, number>>({});
  const [total, setTotal] = useState<number | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [lastFetched, setLastFetched] = useState<number | null>(null);

  const load = async () => {
    setLoading(true);
    setErr(null);
    const pickCount = (v: any): number => typeof v === 'number' ? v : typeof v?.count === 'number' ? v.count : 0;
    // c2-433 / #355: parse /api/proof/stats (preferred) OR extract the
    // .proof bundle from /api/health/extended (fallback). Both populate
    // a {status: count} map — the buckets renderer is shape-agnostic.
    const parseStatsPayload = (data: any): { counts: Record<string, number>; tot: number; total: number | null } => {
      const counts: Record<string, number> = {};
      let tot = 0;
      if (Array.isArray(data)) {
        for (const row of data) {
          const status = String(row.status ?? row.verdict ?? row.name ?? '').toLowerCase();
          const n = pickCount(row);
          if (status) { counts[status] = (counts[status] || 0) + n; tot += n; }
        }
      } else if (data && typeof data === 'object') {
        if (data.stats && typeof data.stats === 'object') {
          for (const [k, v] of Object.entries(data.stats)) {
            const n = typeof v === 'number' ? v : pickCount(v);
            counts[k.toLowerCase()] = n; tot += n;
          }
        } else if (data.counts && typeof data.counts === 'object') {
          for (const [k, v] of Object.entries(data.counts)) {
            const n = typeof v === 'number' ? v : pickCount(v);
            counts[k.toLowerCase()] = n; tot += n;
          }
        } else {
          for (const [k, v] of Object.entries(data)) {
            if (typeof v === 'number') { counts[k.toLowerCase()] = v; tot += v; }
          }
        }
      }
      return { counts, tot, total: typeof data?.total === 'number' ? data.total : null };
    };
    try {
      let counts: Record<string, number> = {};
      let tot = 0;
      let total: number | null = null;
      try {
        const r = await fetch(`http://${host}:3000/api/proof/stats`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const data = await r.json();
        const parsed = parseStatsPayload(data);
        counts = parsed.counts; tot = parsed.tot; total = parsed.total;
      } catch {
        // c2-433 / #355 fallback: pull proof counts out of the extended
        // bundle. Shape per spec: {proof: {proved, rejected, pending_sample}}.
        // pending_sample → we classify as 'pending' (yellow tier).
        const r2 = await fetch(`http://${host}:3000/api/health/extended`);
        if (!r2.ok) throw new Error(`HTTP ${r2.status}`);
        const data2 = await r2.json();
        const p = data2?.proof || {};
        const proved = typeof p.proved === 'number' ? p.proved : 0;
        const rejected = typeof p.rejected === 'number' ? p.rejected : 0;
        const pending = typeof p.pending_sample === 'number' ? p.pending_sample
          : typeof p.pending === 'number' ? p.pending : 0;
        counts = { proved, rejected, pending };
        tot = proved + rejected + pending;
        total = tot;
      }
      setStats(counts);
      setTotal(total ?? tot);
      setLastFetched(Date.now());
    } catch (e: any) {
      setErr(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    load();
    const id = window.setInterval(load, 30_000);
    return () => window.clearInterval(id);
    // eslint-disable-next-line
  }, [host]);

  // Canonical 4 verdict buckets with color tier + which aliases accumulate.
  const BUCKETS: Array<{ key: string; label: string; tone: string; aliases: string[]; hint: string }> = [
    { key: 'proved',     label: 'Proved',      tone: C.green,     aliases: ['proved','valid','ok'],             hint: 'Lean4/Kimina accepted the proof' },
    { key: 'rejected',   label: 'Rejected',    tone: C.red,       aliases: ['rejected','invalid','contradicted'], hint: 'Verifier rejected the proof' },
    { key: 'unreachable', label: 'Unreachable', tone: C.yellow,    aliases: ['unreachable','timeout','pending'], hint: 'Verifier was down or slow — prior tier preserved' },
    { key: 'unknown',    label: 'Unknown',     tone: C.textMuted, aliases: ['unknown','error','unchecked','none'], hint: 'Never attempted — no verdict yet' },
  ];

  const countFor = (aliases: string[]): number => aliases.reduce((acc, a) => acc + (stats[a] || 0), 0);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.lg }}>
      {err && <ErrorAlert C={C} message={err} onRetry={load} retrying={loading} />}

      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, flexWrap: 'wrap' }}>
        <Label color={C.textMuted}>Proof verdicts</Label>
        {total != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            {total.toLocaleString()} fact{total === 1 ? '' : 's'} tracked
          </span>
        )}
        <div style={{ flex: 1 }} />
        {lastFetched != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            Updated {formatRelative(lastFetched)}
          </span>
        )}
        <button onClick={load} disabled={loading}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
          }}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      </div>

      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
        gap: T.spacing.md,
      }}>
        {BUCKETS.map(b => {
          const n = countFor(b.aliases);
          const pct = total && total > 0 ? (n / total) * 100 : null;
          return (
            <div key={b.key} title={b.hint}
              style={{
                padding: T.spacing.md, borderRadius: T.radii.lg,
                background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                display: 'flex', flexDirection: 'column', gap: '6px',
              }}>
              <div style={{
                fontSize: '10px', color: C.textMuted,
                fontWeight: T.typography.weightSemibold,
                textTransform: 'uppercase', letterSpacing: '0.08em',
              }}>{b.label}</div>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: T.spacing.sm }}>
                <span style={{
                  fontSize: T.typography.sizeXl,
                  fontWeight: T.typography.weightBlack, color: b.tone,
                  fontFamily: T.typography.fontMono,
                }}>{n.toLocaleString()}</span>
                {pct != null && (
                  <span style={{
                    fontSize: '10px', color: C.textDim,
                    fontFamily: T.typography.fontMono,
                  }}>{pct.toFixed(1)}%</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Unclassified bucket (tolerant leftover from parser) */}
      {(() => {
        const known = new Set<string>();
        for (const b of BUCKETS) for (const a of b.aliases) known.add(a);
        const leftover = Object.entries(stats).filter(([k]) => !known.has(k));
        if (leftover.length === 0) return null;
        return (
          <div style={{
            padding: T.spacing.md, borderRadius: T.radii.md,
            background: C.bgInput, border: `1px dashed ${C.borderSubtle}`,
            fontSize: T.typography.sizeXs, color: C.textMuted,
            fontFamily: T.typography.fontMono,
          }}>
            Unrecognized verdict buckets: {leftover.map(([k, v]) => `${k}:${v}`).join(' · ')}
          </div>
        );
      })()}
    </div>
  );
};

// c2-433: in-app diag viewer. Reads the diag ring buffer and subscribes
// to new entries so the list updates in real time as console.warn/error
// and window errors flow in. Filter by level + free-text search over
// source/message/data. Copy-JSON + Clear actions. Stats chip at the top
// shows error/warn counts so operators see health at a glance.
const DiagTab: React.FC<{ C: any }> = ({ C }) => {
  const [entries, setEntries] = useState<DiagEntry[]>(() => diag.snapshot());
  const [levelFilter, setLevelFilter] = useState<DiagLevel | 'all'>('all');
  const [query, setQuery] = useState<string>('');
  const [copiedAt, setCopiedAt] = useState<number>(0);
  const [autoScroll, setAutoScroll] = useState<boolean>(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unsub = diag.subscribe(() => {
      setEntries(diag.snapshot());
    });
    return unsub;
  }, []);

  // Auto-scroll to bottom when new entries arrive (unless user disabled).
  useEffect(() => {
    if (!autoScroll || !scrollRef.current) return;
    scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [entries, autoScroll]);

  const q = query.trim().toLowerCase();
  const visible = entries.filter(e => {
    if (levelFilter !== 'all' && e.level !== levelFilter) return false;
    if (!q) return true;
    if (e.source.toLowerCase().includes(q)) return true;
    if (e.message.toLowerCase().includes(q)) return true;
    if (e.data && JSON.stringify(e.data).toLowerCase().includes(q)) return true;
    return false;
  });

  const counts = {
    debug: entries.filter(e => e.level === 'debug').length,
    info:  entries.filter(e => e.level === 'info').length,
    warn:  entries.filter(e => e.level === 'warn').length,
    error: entries.filter(e => e.level === 'error').length,
  };
  const toneFor = (l: DiagLevel): string =>
    l === 'error' ? C.red :
    l === 'warn'  ? C.yellow :
    l === 'info'  ? C.accent :
                    C.textMuted;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.md, minHeight: 0 }}>
      {/* Header: counts + actions */}
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, flexWrap: 'wrap' }}>
        <Label color={C.textMuted}>Diagnostic log ({entries.length} entries)</Label>
        <span style={{ fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs, color: C.textDim }}>
          <span style={{ color: counts.error > 0 ? C.red : C.textDim }}>{counts.error} err</span>
          {' · '}
          <span style={{ color: counts.warn > 0 ? C.yellow : C.textDim }}>{counts.warn} warn</span>
          {' · '}
          <span style={{ color: C.textDim }}>{counts.info} info · {counts.debug} debug</span>
        </span>
        <div style={{ flex: 1 }} />
        <button
          disabled={entries.length === 0}
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(diag.export());
              setCopiedAt(Date.now());
              window.setTimeout(() => setCopiedAt(0), 2000);
            } catch { /* blocked */ }
          }}
          title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${entries.length} diag entries + metadata as JSON`}
          style={{
            background: copiedAt > 0 ? `${C.green}18` : 'transparent',
            border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
            color: copiedAt > 0 ? C.green : C.textMuted,
            borderRadius: T.radii.sm,
            cursor: entries.length === 0 ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: entries.length === 0 ? 0.5 : 1,
          }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
        <button
          disabled={entries.length === 0}
          onClick={() => {
            if (!window.confirm(`Clear ${entries.length} diag entries? Not recoverable.`)) return;
            diag.clear();
            setEntries([]);
          }}
          style={{
            background: 'transparent',
            border: `1px solid ${C.borderSubtle}`,
            color: entries.length === 0 ? C.textDim : C.textMuted,
            borderRadius: T.radii.sm,
            cursor: entries.length === 0 ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: entries.length === 0 ? 0.5 : 1,
          }}>Clear</button>
      </div>

      {/* Filter row: level chips + query */}
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, flexWrap: 'wrap' }}>
        <div role='tablist' aria-label='Filter by level'
          style={{ display: 'flex', gap: '4px' }}>
          {([
            { id: 'all', label: 'all', count: entries.length },
            { id: 'error', label: 'error', count: counts.error },
            { id: 'warn', label: 'warn', count: counts.warn },
            { id: 'info', label: 'info', count: counts.info },
            { id: 'debug', label: 'debug', count: counts.debug },
          ]).map(b => {
            const active = levelFilter === b.id;
            const tone = b.id === 'error' ? C.red
              : b.id === 'warn' ? C.yellow
              : b.id === 'info' ? C.accent
              : b.id === 'debug' ? C.textMuted
              : C.accent;
            return (
              <button key={b.id} onClick={() => setLevelFilter(b.id as DiagLevel | 'all')}
                role='tab' aria-selected={active}
                style={{
                  padding: '3px 9px', fontSize: '10px',
                  fontWeight: T.typography.weightBold,
                  background: active ? `${tone}18` : 'transparent',
                  border: `1px solid ${active ? tone : C.borderSubtle}`,
                  color: active ? tone : C.textMuted,
                  borderRadius: T.radii.sm,
                  cursor: 'pointer', fontFamily: T.typography.fontMono,
                  letterSpacing: '0.04em', textTransform: 'uppercase',
                }}>{b.label} <span style={{ opacity: 0.6, marginLeft: '3px' }}>{b.count}</span></button>
            );
          })}
        </div>
        <input type='search' value={query} onChange={e => setQuery(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Escape' && query) { e.preventDefault(); e.stopPropagation(); setQuery(''); } }}
          placeholder='Filter by source, message, or data…'
          aria-label='Filter diag entries'
          style={{
            flex: '1 1 180px', minWidth: 0, maxWidth: '280px',
            padding: '4px 10px',
            background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
            color: C.text, borderRadius: T.radii.sm,
            fontFamily: 'inherit', fontSize: T.typography.sizeXs,
            outline: 'none',
          }} />
        <label style={{
          display: 'inline-flex', alignItems: 'center', gap: '4px',
          fontSize: '10px', color: C.textMuted,
          fontFamily: T.typography.fontMono, letterSpacing: '0.04em',
          cursor: 'pointer',
        }}>
          <input type='checkbox' checked={autoScroll} onChange={e => setAutoScroll(e.target.checked)} />
          auto-scroll
        </label>
      </div>

      {/* Entries list */}
      <div ref={scrollRef} style={{
        flex: 1, minHeight: '200px', maxHeight: '60vh', overflowY: 'auto',
        background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
        borderRadius: T.radii.md, padding: T.spacing.sm,
        fontFamily: T.typography.fontMono, fontSize: '11px',
      }}>
        {visible.length === 0 ? (
          <div style={{
            textAlign: 'center', color: C.textMuted, fontStyle: 'italic',
            padding: T.spacing.xl, fontSize: T.typography.sizeSm,
          }}>
            {entries.length === 0 ? 'No diagnostic events yet. Log lines from console.warn/error and window errors land here.'
              : `No entries match ${query ? `"${query}"` : levelFilter}.`}
          </div>
        ) : visible.map((e, i) => {
          const d = new Date(e.ts);
          const timestr = `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}.${d.getMilliseconds().toString().padStart(3, '0')}`;
          return (
            <div key={`${e.ts}-${i}`} style={{
              display: 'grid',
              gridTemplateColumns: 'minmax(80px, auto) minmax(40px, auto) minmax(60px, auto) 1fr',
              gap: '8px', padding: '3px 0',
              borderBottom: `1px dashed ${C.borderSubtle}`,
              lineHeight: 1.35,
            }}>
              <span style={{ color: C.textDim }}>{timestr}</span>
              <span style={{
                color: toneFor(e.level),
                fontWeight: T.typography.weightBold,
                textTransform: 'uppercase', letterSpacing: '0.04em',
              }}>{e.level}</span>
              <span style={{ color: C.accent, fontWeight: 700 }}>{e.source}</span>
              <span style={{
                color: C.text, wordBreak: 'break-word',
                whiteSpace: 'pre-wrap',
              }}>
                {e.message}
                {e.data !== undefined && (
                  <span style={{ color: C.textMuted, marginLeft: '6px' }}>
                    {' '}{typeof e.data === 'object' ? JSON.stringify(e.data).slice(0, 200) : String(e.data)}
                  </span>
                )}
                {e.stack && (
                  <details style={{ marginTop: '2px' }}>
                    <summary style={{ color: C.textDim, cursor: 'pointer', fontSize: '10px' }}>stack</summary>
                    <pre style={{ margin: '4px 0', color: C.textDim, fontSize: '10px', whiteSpace: 'pre-wrap' }}>{e.stack}</pre>
                  </details>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

// Tiny markdown renderer for the Docs tab. Intentionally minimal — headers,
// paragraphs, fenced code, inline code, bold, italic, bullets. Not meant
// to render arbitrary user-supplied markdown (that's what markdown.tsx's
// renderMessageBody handles); this is for trusted backend-shipped docs.
const renderDocsMarkdown = (src: string, C: any): React.ReactNode[] => {
  const out: React.ReactNode[] = [];
  const lines = src.split('\n');
  let i = 0;
  let k = 0;
  const renderInline = (s: string, baseKey: string): React.ReactNode[] => {
    const nodes: React.ReactNode[] = [];
    let rest = s;
    let j = 0;
    while (rest.length > 0) {
      const bold = rest.match(/^\*\*([^*]+)\*\*/);
      const code = rest.match(/^`([^`]+)`/);
      const ital = rest.match(/^\*([^*]+)\*/);
      if (bold) {
        nodes.push(<strong key={`${baseKey}-b${j++}`}>{bold[1]}</strong>);
        rest = rest.slice(bold[0].length);
      } else if (code) {
        nodes.push(<code key={`${baseKey}-c${j++}`} style={{
          background: C.bgInput, padding: '1px 5px', borderRadius: 3,
          fontFamily: T.typography.fontMono, fontSize: '0.92em',
        }}>{code[1]}</code>);
        rest = rest.slice(code[0].length);
      } else if (ital) {
        nodes.push(<em key={`${baseKey}-i${j++}`}>{ital[1]}</em>);
        rest = rest.slice(ital[0].length);
      } else {
        const nextSpecial = rest.search(/\*\*|`|\*/);
        if (nextSpecial === -1) { nodes.push(rest); break; }
        nodes.push(rest.slice(0, nextSpecial));
        rest = rest.slice(nextSpecial);
      }
    }
    return nodes;
  };
  while (i < lines.length) {
    const line = lines[i];
    // Fenced code block
    const fence = line.match(/^```(\w*)/);
    if (fence) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) { codeLines.push(lines[i]); i++; }
      i++;
      out.push(
        <pre key={`pre${k++}`} style={{
          background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
          borderRadius: T.radii.md, padding: T.spacing.sm,
          fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
          overflow: 'auto', margin: `${T.spacing.sm} 0`,
          color: C.text, whiteSpace: 'pre', wordBreak: 'normal',
        }}><code>{codeLines.join('\n')}</code></pre>
      );
      continue;
    }
    // Headers
    const h1 = line.match(/^# (.+)/);
    const h2 = line.match(/^## (.+)/);
    const h3 = line.match(/^### (.+)/);
    if (h1) {
      out.push(<h1 key={`h${k++}`} style={{ fontSize: T.typography.size2xl, margin: `${T.spacing.lg} 0 ${T.spacing.sm}`, color: C.text }}>{renderInline(h1[1], `h${k}`)}</h1>);
      i++; continue;
    }
    if (h2) {
      out.push(<h2 key={`h${k++}`} style={{ fontSize: T.typography.sizeXl, margin: `${T.spacing.lg} 0 ${T.spacing.sm}`, color: C.text }}>{renderInline(h2[1], `h${k}`)}</h2>);
      i++; continue;
    }
    if (h3) {
      out.push(<h3 key={`h${k++}`} style={{ fontSize: T.typography.sizeLg, margin: `${T.spacing.md} 0 ${T.spacing.xs}`, color: C.text }}>{renderInline(h3[1], `h${k}`)}</h3>);
      i++; continue;
    }
    // Bullet list (collect consecutive)
    if (/^\s*[-*] /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*[-*] /.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*] /, ''));
        i++;
      }
      out.push(
        <ul key={`ul${k++}`} style={{ margin: `${T.spacing.sm} 0`, paddingLeft: 24, color: C.text }}>
          {items.map((it, idx) => <li key={idx}>{renderInline(it, `li${k}-${idx}`)}</li>)}
        </ul>
      );
      continue;
    }
    // Blank line → paragraph break
    if (!line.trim()) { i++; continue; }
    // Paragraph — collect until blank
    const para: string[] = [];
    while (i < lines.length && lines[i].trim() && !/^(#|```|\s*[-*] )/.test(lines[i])) {
      para.push(lines[i]);
      i++;
    }
    out.push(
      <p key={`p${k++}`} style={{ margin: `${T.spacing.xs} 0`, lineHeight: 1.6, color: C.text }}>
        {renderInline(para.join(' '), `p${k}`)}
      </p>
    );
  }
  return out;
};

// Docs tab — fetches a backend-shipped markdown doc (claude-0's
// manager_guide.md) and renders it in-app. Falls back to a helpful
// "not yet shipped" card when the endpoint 404s so older deployments
// don't show a broken tab.
const DocsTab: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [text, setText] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [source, setSource] = useState<string>('');
  const load = React.useCallback(async () => {
    setLoading(true); setErr(null);
    // Try several known locations in order. First hit wins.
    const candidates = [
      `/docs/manager_guide.md`,
      `/api/docs/manager_guide`,
      `/api/docs/manager_guide.md`,
    ];
    for (const path of candidates) {
      try {
        const url = `http://${host}:3000${path}`;
        const r = await fetch(url);
        if (!r.ok) continue;
        const body = await r.text();
        if (body && body.length > 10) {
          setText(body);
          setSource(path);
          setLoading(false);
          return;
        }
      } catch { /* try next */ }
    }
    setLoading(false);
    setErr('manager_guide.md not available yet — claude-0 is shipping the endpoint. Try again in a minute.');
  }, [host]);
  useEffect(() => { load(); }, [load]);
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, marginBottom: T.spacing.md }}>
        <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono }}>
          {loading ? 'Loading…' : source ? `source: ${source}` : 'not available'}
        </div>
        <div style={{ flex: 1 }} />
        <button onClick={load} disabled={loading}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 10px', fontSize: T.typography.sizeXs,
            fontWeight: T.typography.weightBold, fontFamily: 'inherit',
          }}>Reload</button>
        {text && (
          <button onClick={() => { try { navigator.clipboard?.writeText(text); } catch { /* silent */ } }}
            style={{
              background: 'transparent', border: `1px solid ${C.borderSubtle}`,
              color: C.textMuted, borderRadius: T.radii.sm,
              cursor: 'pointer', padding: '4px 10px',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
              fontFamily: 'inherit',
            }}>Copy</button>
        )}
      </div>
      {err && !loading && (
        <div style={{
          padding: T.spacing.md, borderRadius: T.radii.md,
          border: `1px solid ${C.borderSubtle}`, background: C.bgInput,
          color: C.textMuted, fontSize: T.typography.sizeSm, lineHeight: 1.6,
        }}>
          <div style={{ fontWeight: T.typography.weightBold, marginBottom: T.spacing.xs, color: C.text }}>
            Docs not loaded
          </div>
          {err}
        </div>
      )}
      {text && !loading && (
        <div style={{
          background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
          borderRadius: T.radii.lg, padding: T.spacing.lg,
          fontSize: T.typography.sizeSm,
        }}>
          {renderDocsMarkdown(text, C)}
        </div>
      )}
    </div>
  );
};

