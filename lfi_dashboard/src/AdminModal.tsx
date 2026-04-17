import React, { useRef, useState, useMemo, useEffect } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';
// c2-337 / c0-auto-2 task 21: 28px heading font sourced from the cross-platform
// design-system so desktop/Android builds share the same scale (T.typography
// caps at 22px which is visually a large shrink for the big dashboard numbers).
import { typography as dsType } from './design-system';
import { compactNum, formatRelative } from './util';

// Full-screen admin modal per c0-017. Six tabs: Dashboard / Domains /
// Training / Quality / System / Logs. Replaces the prior sidebar-slide admin
// affordance which users found cramped. Sortable + filterable tables, big-
// number dashboard cards, bar-chart visualisations of domains + quality.

export type AdminTab = 'dashboard' | 'inventory' | 'domains' | 'training' | 'quality' | 'system' | 'fleet' | 'logs';

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
  C, host, onClose, factsCount, sourcesCount, initialTab = 'dashboard', localEvents = [],
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
      const valid: AdminTab[] = ['dashboard', 'inventory', 'domains', 'training', 'quality', 'system', 'fleet', 'logs'];
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
  const [sysInfo, setSysInfo] = useState<SystemShape | null>(null);
  const [fleet, setFleet] = useState<FleetShape | null>(null);
  const [logs, setLogs] = useState<string[] | null>(null);
  const [err, setErr] = useState<Record<AdminTab, string | null>>({
    dashboard: null, inventory: null, domains: null, training: null, quality: null, system: null, fleet: null, logs: null,
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
        setQuality(await fetchJson('/api/quality/report', ctrl.signal));
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
        padding: T.spacing.lg,
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
          padding: '14px 22px', borderBottom: `1px solid ${C.borderSubtle}`,
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
                fontSize: '22px', cursor: 'pointer', padding: '4px 10px',
              }}>{'\u2715'}</button>
          </div>
        </div>
        <style>{`@keyframes scc-admin-spin { to { transform: rotate(360deg); } }`}</style>

        {/* Tab bar — WAI-ARIA tablist with arrow-key navigation. */}
        <div role='tablist' aria-label='Admin sections'
          onKeyDown={(e) => {
            const all: AdminTab[] = ['dashboard', 'inventory', 'domains', 'training', 'quality', 'system', 'fleet', 'logs'];
            const idx = all.indexOf(tab);
            if (idx < 0) return;
            if (e.key === 'ArrowRight') { e.preventDefault(); setTab(all[(idx + 1) % all.length]); }
            else if (e.key === 'ArrowLeft') { e.preventDefault(); setTab(all[(idx - 1 + all.length) % all.length]); }
            else if (e.key === 'Home') { e.preventDefault(); setTab(all[0]); }
            else if (e.key === 'End') { e.preventDefault(); setTab(all[all.length - 1]); }
          }}
          style={{
            display: 'flex', gap: '2px', padding: '0 22px',
            borderBottom: `1px solid ${C.borderSubtle}`, overflowX: 'auto',
          }}>
          {([
            { id: 'dashboard', label: 'Dashboard' },
            { id: 'inventory', label: 'Inventory' },
            { id: 'domains', label: 'Domains' },
            { id: 'training', label: 'Training' },
            { id: 'quality', label: 'Quality' },
            { id: 'system', label: 'System' },
            { id: 'fleet', label: 'Fleet' },
            { id: 'logs', label: 'Logs' },
          ] as const).map(t => (
            <button key={t.id} onClick={() => setTab(t.id)}
              role='tab' aria-selected={tab === t.id}
              tabIndex={tab === t.id ? 0 : -1}
              style={{
                padding: '12px 16px', fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBold,
                background: 'transparent', border: 'none', cursor: 'pointer',
                color: tab === t.id ? C.accent : C.textMuted,
                borderBottom: `2px solid ${tab === t.id ? C.accent : 'transparent'}`,
                marginBottom: '-1px', fontFamily: 'inherit',
                whiteSpace: 'nowrap',
              }}>{t.label}</button>
          ))}
        </div>

        {/* Body */}
        <div role='tabpanel' aria-label={tab} style={{ flex: 1, overflowY: 'auto', padding: '20px 22px' }}>
          {/* ---------- Dashboard ---------- */}
          {tab === 'dashboard' && (
            <div>
              {err.dashboard && <AdminErr C={C} msg={err.dashboard} onRetry={() => loadTab('dashboard')} retrying={loading === 'dashboard'} />}
              {/* Skeleton loader — only shown while the first fetch is in
                  flight AND we don't yet have any cached data. Subsequent
                  refreshes render fresh data silently. Per c0-020. */}
              {loading === 'dashboard' && !dashboard && (
                <div aria-busy='true' aria-live='polite'>
                  <div style={{
                    height: '120px', marginBottom: T.spacing.xl, borderRadius: T.radii.lg,
                    background: `linear-gradient(90deg, ${C.bgInput} 0%, ${C.bgHover} 50%, ${C.bgInput} 100%)`,
                    backgroundSize: '200% 100%', animation: 'scc-skel-admin 1.3s ease-in-out infinite',
                  }} />
                  <div style={{
                    display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                    gap: T.spacing.md, marginBottom: T.spacing.xl,
                  }}>
                    {[0, 1, 2, 3, 4, 5].map(i => (
                      <div key={i} style={{
                        height: '70px', borderRadius: T.radii.lg,
                        background: `linear-gradient(90deg, ${C.bgInput} 0%, ${C.bgHover} 50%, ${C.bgInput} 100%)`,
                        backgroundSize: '200% 100%', animation: 'scc-skel-admin 1.3s ease-in-out infinite',
                        animationDelay: `${i * 0.08}s`,
                      }} />
                    ))}
                  </div>
                  <style>{`@keyframes scc-skel-admin { 0% { background-position: 200% 0 } 100% { background-position: -200% 0 } }`}</style>
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
                    <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                      Accuracy grade
                    </div>
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
                      <div style={{ fontSize: '13px', color: C.textSecondary, marginTop: '4px', fontFamily: T.typography.fontMono }}>
                        {dashboard.score.accuracy_score.toFixed(1)} / 100
                      </div>
                    )}
                  </div>
                  <div>
                    <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                      Score breakdown
                    </div>
                    {dashboard.score.breakdown && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        {(['quality', 'adversarial', 'coverage', 'training'] as const).map(k => {
                          const v = dashboard.score?.breakdown?.[k];
                          if (typeof v !== 'number') return null;
                          const pc = v <= 1.5 ? v * 100 : v;
                          const col = pc >= 80 ? C.green : pc >= 60 ? C.yellow : C.red;
                          return (
                            <div key={k} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                              <span style={{ width: '110px', fontSize: '12px', color: C.textSecondary, textTransform: 'capitalize' }}>{k}</span>
                              <div style={{ flex: 1, background: C.bg, height: '10px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                                <div style={{ width: `${pc}%`, height: '100%', background: col, transition: 'width 0.4s' }} />
                              </div>
                              <span style={{ width: '56px', textAlign: 'right', fontSize: '12px', color: col, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{pc.toFixed(0)}</span>
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
                    <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                      {card.label}
                    </div>
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
                    <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                      Top 10 domains by fact count
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {top.map(d => (
                        <div key={d.label} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                          <span style={{ width: '160px', fontSize: '12px', color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{d.label}</span>
                          <div style={{ flex: 1, background: C.bgInput, height: '18px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                            <div style={{
                              width: `${(d.value / max) * 100}%`, height: '100%',
                              background: countColor(d.value),
                              transition: 'width 0.4s',
                            }} />
                          </div>
                          <span style={{ width: '84px', textAlign: 'right', fontSize: '12px', fontFamily: T.typography.fontMono, color: C.textMuted }}>
                            {d.value.toLocaleString()}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                );
              })()}
              {/* Training files list from consolidated endpoint */}
              {dashboard?.training_files && dashboard.training_files.length > 0 && (
                <div>
                  <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                    Training datasets ({dashboard.training_files.length})
                  </div>
                  <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeMd }}>
                      <thead>
                        <tr>
                          <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}` }}>File</th>
                          <th style={{ textAlign: 'right', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}` }}>Pairs</th>
                          <th style={{ textAlign: 'right', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}` }}>Size</th>
                        </tr>
                      </thead>
                      <tbody>
                        {[...dashboard.training_files].sort((a, b) => b.pairs - a.pairs).map(f => (
                          <tr key={f.file}>
                            <td style={{ padding: '8px 12px', fontFamily: T.typography.fontMono, color: C.text }}>{f.file}</td>
                            <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.accent }}>{f.pairs.toLocaleString()}</td>
                            <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
              {dashboard?.system && (
                <div style={{ marginTop: T.spacing.xl, fontSize: '11px', color: C.textDim, textAlign: 'center' }}>
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
              <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
                What the backend knows about. Sources, facts, training files and domain coverage in one glance.
              </p>
              {err.inventory && (
                <div style={{ padding: T.spacing.md, borderRadius: T.radii.md, background: C.redBg, border: `1px solid ${C.redBorder}`, color: C.red, fontSize: '12px', marginBottom: T.spacing.md }}>
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
                      <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>{c.label}</div>
                      <div style={{ fontSize: '22px', fontWeight: T.typography.weightBlack, color: c.color, marginTop: '4px', fontFamily: T.typography.fontMono }}>{c.value}</div>
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
                const sorted = [...files].sort((a, b) => (b.pairs || 0) - (a.pairs || 0));
                return (
                  <div style={{ marginBottom: T.spacing.xl }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: '6px' }}>
                      <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                        Training files ({files.length})
                      </div>
                      <div style={{ fontSize: '11px', color: C.textDim, fontFamily: T.typography.fontMono }}>
                        {compactNum(totalPairs)} pairs · {totalMb.toFixed(1)} MB
                      </div>
                    </div>
                    <div style={{ maxHeight: '300px', overflowY: 'auto', border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.sm }}>
                      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
                        <thead style={{ position: 'sticky', top: 0, background: C.bgInput, zIndex: 1 }}>
                          <tr>
                            <th style={{ padding: '6px 10px', textAlign: 'left', fontWeight: T.typography.weightBold, color: C.textMuted, borderBottom: `1px solid ${C.borderSubtle}` }}>File</th>
                            <th style={{ padding: '6px 10px', textAlign: 'right', fontWeight: T.typography.weightBold, color: C.textMuted, borderBottom: `1px solid ${C.borderSubtle}` }}>Pairs</th>
                            <th style={{ padding: '6px 10px', textAlign: 'right', fontWeight: T.typography.weightBold, color: C.textMuted, borderBottom: `1px solid ${C.borderSubtle}` }}>Size (MB)</th>
                          </tr>
                        </thead>
                        <tbody>
                          {sorted.map((f, i) => (
                            <tr key={f.file} style={{ background: i % 2 === 0 ? 'transparent' : C.bgHover }}>
                              <td style={{ padding: '6px 10px', color: C.text, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '420px' }}>{f.file}</td>
                              <td style={{ padding: '6px 10px', textAlign: 'right', color: C.text, fontFamily: T.typography.fontMono }}>{compactNum(f.pairs)}</td>
                              <td style={{ padding: '6px 10px', textAlign: 'right', color: C.textMuted, fontFamily: T.typography.fontMono }}>{f.size_mb.toFixed(2)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
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
                    <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '6px' }}>
                      Top 10 domains
                    </div>
                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '6px 14px' }}>
                      {top.map(d => (
                        <div key={d.domain} style={{ display: 'flex', justifyContent: 'space-between', gap: T.spacing.sm, fontSize: '12px', fontFamily: T.typography.fontMono, padding: '3px 0' }}>
                          <span style={{ color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{d.domain}</span>
                          <span style={{ color: countColor(d.count) }}>{d.count.toLocaleString()}</span>
                        </div>
                      ))}
                    </div>
                    {doms.length > 10 && (
                      <div style={{ marginTop: '6px', fontSize: '11px', color: C.textDim }}>
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
              {err.domains && <AdminErr C={C} msg={err.domains} onRetry={() => loadTab('domains')} retrying={loading === 'domains'} />}
              <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeMd }}>
                  <thead>
                    <tr>
                      {([
                        { key: 'domain', label: 'Domain', align: 'left' as const },
                        { key: 'facts', label: 'Facts', align: 'right' as const },
                        { key: 'avg_quality', label: 'Avg Quality', align: 'right' as const },
                        { key: 'avg_length', label: 'Avg Length', align: 'right' as const },
                      ]).map(h => (
                        <th key={h.key as string}
                          onClick={() => setDomainSort(s => ({
                            key: h.key as keyof DomainRow,
                            dir: s.key === h.key && s.dir === 'desc' ? 'asc' : 'desc',
                          }))}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter' || e.key === ' ') {
                              e.preventDefault();
                              setDomainSort(s => ({
                                key: h.key as keyof DomainRow,
                                dir: s.key === h.key && s.dir === 'desc' ? 'asc' : 'desc',
                              }));
                            }
                          }}
                          role='button'
                          tabIndex={0}
                          aria-sort={
                            /* c2-273: surface sort state to screen readers.
                               WAI-ARIA sort token set: 'ascending' / 'descending' /
                               'none' (or absent). */
                            domainSort.key === h.key
                              ? (domainSort.dir === 'asc' ? 'ascending' : 'descending')
                              : 'none'
                          }
                          aria-label={`${h.label}, sortable${domainSort.key === h.key ? `, currently ${domainSort.dir === 'asc' ? 'ascending' : 'descending'}` : ''}`}
                          style={{
                            textAlign: h.align, padding: '10px 14px',
                            fontWeight: T.typography.weightBold, color: C.textSecondary,
                            background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}`,
                            cursor: 'pointer', userSelect: 'none', position: 'sticky', top: 0,
                          }}>
                          {h.label}{sortArrow(domainSort.key === h.key, domainSort.dir)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {filteredDomains.map((d, i) => (
                      <tr key={i} style={{ background: i % 2 === 0 ? 'transparent' : 'rgba(255,255,255,0.015)' }}>
                        <td style={{ padding: '10px 14px', fontWeight: T.typography.weightSemibold }}>{d.domain}</td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: countColor(d.facts), fontWeight: T.typography.weightBold, fontFamily: T.typography.fontMono }}>
                          {d.facts.toLocaleString()}
                        </td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: typeof d.avg_quality === 'number' ? qualityColor(d.avg_quality) : C.textMuted, fontFamily: T.typography.fontMono }}>
                          {typeof d.avg_quality === 'number' ? d.avg_quality.toFixed(2) : '—'}
                        </td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: C.textMuted, fontFamily: T.typography.fontMono }}>
                          {typeof d.avg_length === 'number' ? d.avg_length.toFixed(0) : '—'}
                        </td>
                      </tr>
                    ))}
                    {filteredDomains.length === 0 && (
                      <tr><td colSpan={4} style={{ padding: '28px', textAlign: 'center', color: C.textMuted, fontStyle: 'italic' }}>
                        {domains === null ? 'Loading…' : 'No domains match.'}
                      </td></tr>
                    )}
                  </tbody>
                </table>
              </div>
              <div style={{ marginTop: T.spacing.sm, fontSize: T.typography.sizeSm, color: C.textDim }}>
                {filteredDomains.length} of {domains?.length ?? 0} domains
                {domains && ` · ${domains.reduce((s, d) => s + d.facts, 0).toLocaleString()} facts total`}
              </div>
            </div>
          )}

          {/* ---------- Training ---------- */}
          {tab === 'training' && (
            <div>
              {err.training && <AdminErr C={C} msg={err.training} onRetry={() => loadTab('training')} retrying={loading === 'training'} />}
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
                          <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                            Pass rate
                          </div>
                          <div style={{
                            fontSize: dsType.sizes['3xl'], fontWeight: T.typography.weightBlack,
                            color: p != null ? (p >= 95 ? C.green : p >= 80 ? C.yellow : C.red) : C.textMuted,
                            marginTop: '4px', fontFamily: T.typography.fontMono,
                          }}>{p != null ? `${p.toFixed(1)}%` : '—'}</div>
                        </div>
                      );
                    })()}
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Samples</div>
                      <div style={{ fontSize: dsType.sizes['3xl'], fontWeight: T.typography.weightBlack, color: C.text, marginTop: '4px', fontFamily: T.typography.fontMono }}>
                        {typeof accuracy.samples === 'number' ? accuracy.samples.toLocaleString() : '—'}
                      </div>
                    </div>
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Last run</div>
                      <div style={{ fontSize: '13px', color: C.text, marginTop: '10px' }}>
                        {accuracy.last_run ? (typeof accuracy.last_run === 'number' ? new Date(accuracy.last_run * 1000).toLocaleString() : accuracy.last_run) : '—'}
                      </div>
                    </div>
                  </div>
                  {accuracy.per_domain && Object.keys(accuracy.per_domain).length > 0 && (
                    <div>
                      <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                        Accuracy by domain
                      </div>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        {Object.entries(accuracy.per_domain).sort((a, b) => (b[1] ?? 0) - (a[1] ?? 0)).map(([dom, v]) => {
                          const p = pctNorm(v) ?? 0;
                          return (
                            <div key={dom} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                              <span style={{ width: '160px', fontSize: '12px', color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{dom}</span>
                              <div style={{ flex: 1, background: C.bgInput, height: '14px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                                <div style={{
                                  width: `${p}%`, height: '100%',
                                  background: p >= 95 ? C.green : p >= 80 ? C.yellow : C.red,
                                }} />
                              </div>
                              <span style={{ width: '64px', textAlign: 'right', fontSize: '12px', fontFamily: T.typography.fontMono, color: C.textMuted }}>{p.toFixed(1)}%</span>
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
              {err.quality && <AdminErr C={C} msg={err.quality} onRetry={() => loadTab('quality')} retrying={loading === 'quality'} />}
              {quality ? (
                <>
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
                        <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                          Quality distribution
                        </div>
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
                                <span style={{ width: '120px', fontSize: '12px', color: C.text, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>{bucket}</span>
                                <div style={{ flex: 1, background: C.bgInput, height: '18px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                                  <div style={{ width: `${(n / max) * 100}%`, height: '100%', background: color, transition: 'width 0.4s' }} />
                                </div>
                                <span style={{ width: '96px', textAlign: 'right', fontSize: '12px', fontFamily: T.typography.fontMono, color: C.textMuted }}>
                                  {n.toLocaleString()} ({pct.toFixed(1)}%)
                                </span>
                              </div>
                            );
                          })}
                        </div>
                        <div style={{ marginTop: T.spacing.sm, fontSize: '11px', color: C.textDim, textAlign: 'right' }}>
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
              {err.system && <AdminErr C={C} msg={err.system} onRetry={() => loadTab('system')} retrying={loading === 'system'} />}
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
                                <span style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>{r.label}</span>
                                <span style={{ fontSize: '12px', color: r.color, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{r.right}</span>
                              </div>
                              <div style={{ background: C.bgInput, height: '14px', borderRadius: T.radii.xs, overflow: 'hidden', border: `1px solid ${C.borderSubtle}` }}>
                                <div style={{
                                  width: r.pct != null ? `${r.pct}%` : '0%', height: '100%',
                                  background: r.color, transition: 'width 0.4s',
                                }} />
                              </div>
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
              {err.fleet && <AdminErr C={C} msg={err.fleet} onRetry={() => loadTab('fleet')} retrying={loading === 'fleet'} />}
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
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
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
                                fontSize: '11px', color: C.textMuted, fontFamily: T.typography.fontMono,
                                marginBottom: '6px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                              }}>
                                {inst.current_task}
                              </div>
                            )}
                            <div style={{ display: 'flex', gap: T.spacing.md, fontSize: '11px', color: C.textMuted, fontFamily: T.typography.fontMono }}>
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
                  {fleet.timeline && fleet.timeline.length > 0 && (
                    <div>
                      <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                        Recent activity ({fleet.timeline.length})
                      </div>
                      <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden', maxHeight: '320px', overflowY: 'auto' }}>
                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
                          <thead>
                            <tr>
                              <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>When</th>
                              <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>Who</th>
                              <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>Event</th>
                            </tr>
                          </thead>
                          <tbody>
                            {fleet.timeline.slice(0, 100).map((row, i) => (
                              <tr key={i}>
                                <td style={{ padding: '6px 12px', color: C.textMuted, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>
                                  {typeof row.t === 'number' ? new Date(row.t * (row.t < 1e12 ? 1000 : 1)).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : row.t}
                                </td>
                                <td style={{ padding: '6px 12px', color: C.accent, fontFamily: T.typography.fontMono }}>{row.instance}</td>
                                <td style={{ padding: '6px 12px', color: C.text, fontFamily: T.typography.fontMono }}>{row.event}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          )}

          {/* ---------- Logs ---------- */}
          {tab === 'logs' && (
            <div>
              {err.logs && <AdminErr C={C} msg={err.logs} onRetry={() => loadTab('logs')} retrying={loading === 'logs'} />}
              {/* Server logs (primary) */}
              {logs && logs.length > 0 && (
                <div style={{ marginBottom: T.spacing.lg }}>
                  <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '6px' }}>
                    Server log ({logs.length} lines)
                  </div>
                  <pre style={{
                    margin: 0, padding: '16px', background: C.bgInput,
                    border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
                    fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: T.typography.sizeMd,
                    color: C.text, whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                    maxHeight: '45vh', overflowY: 'auto',
                  }}>{logs.slice(-500).join('\n')}</pre>
                </div>
              )}
              {(logs === null || logs.length === 0) && !err.logs && (
                <div style={{
                  padding: '16px', marginBottom: T.spacing.lg,
                  background: C.bgInput, border: `1px dashed ${C.borderSubtle}`,
                  borderRadius: T.radii.md, color: C.textMuted, fontSize: '13px', textAlign: 'center',
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
                      <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                        Client events ({filtered.length} of {localEvents.length}, this session)
                      </div>
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
                            fontSize: '12px', outline: 'none',
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
                              padding: '4px 8px', fontSize: '11px',
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
                            padding: '6px 12px', fontSize: '11px', fontWeight: T.typography.weightBold,
                            background: filtered.length === 0 ? C.bgInput : C.accentBg,
                            border: `1px solid ${filtered.length === 0 ? C.borderSubtle : C.accentBorder}`,
                            color: filtered.length === 0 ? C.textMuted : C.accent,
                            borderRadius: T.radii.sm,
                            cursor: filtered.length === 0 ? 'not-allowed' : 'pointer',
                            fontFamily: 'inherit', textTransform: 'uppercase',
                          }}>Export JSON</button>
                      </div>
                    </div>
                    {/* Kind-frequency pills — scannable summary. Click to
                        filter the table to that kind. Click again to clear. */}
                    <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap', marginBottom: '8px' }}>
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
                              padding: '3px 10px', fontSize: '11px',
                              background: active ? C.accentBg : C.bgInput,
                              border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                              color: C.text, borderRadius: '999px', cursor: 'pointer',
                              fontFamily: 'inherit', fontWeight: T.typography.weightSemibold,
                            }}>
                            <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: dotColor }} aria-hidden='true' />
                            <span style={{ fontFamily: T.typography.fontMono }}>{kind}</span>
                            <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{n}</span>
                          </button>
                        );
                      })}
                    </div>
                    <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden', maxHeight: '45vh', overflowY: 'auto' }}>
                      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
                        <thead>
                          <tr>
                            <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>Time</th>
                            <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>Kind</th>
                            <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}`, position: 'sticky', top: 0 }}>Data</th>
                          </tr>
                        </thead>
                        <tbody>
                          {filtered.slice(0, 200).map((e, i) => {
                            // Color dot to visually group kinds — positive
                            // signals green, negatives red, navigation neutral
                            // accent. Makes the scroll scan-able at a glance.
                            const dotColor =
                              e.kind.includes('error') || e.kind.includes('failed') || e.kind.includes('negative') ? C.red
                              : e.kind.includes('positive') || e.kind.includes('success') || e.kind.includes('done') ? C.green
                              : e.kind.includes('warn') || e.kind.includes('stop') ? C.yellow
                              : C.accent;
                            return (
                              <tr key={i}>
                                <td style={{ padding: '6px 12px', color: C.textMuted, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap' }}>
                                  {new Date(e.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                                </td>
                                <td style={{ padding: '6px 12px', fontFamily: T.typography.fontMono, color: C.text, whiteSpace: 'nowrap' }}>
                                  <span style={{ display: 'inline-block', width: '6px', height: '6px', borderRadius: '50%', background: dotColor, marginRight: '8px', verticalAlign: 'middle' }} aria-hidden='true' />
                                  <span style={{ color: dotColor }}>{e.kind}</span>
                                </td>
                                <td style={{ padding: '6px 12px', color: C.textMuted, fontFamily: T.typography.fontMono, maxWidth: '520px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                                  {e.data ? JSON.stringify(e.data) : ''}
                                </td>
                              </tr>
                            );
                          })}
                          {filtered.length === 0 && (
                            <tr><td colSpan={3} style={{ padding: '20px', textAlign: 'center', color: C.textMuted, fontStyle: 'italic' }}>No events match.</td></tr>
                          )}
                        </tbody>
                      </table>
                    </div>
                  </div>
                );
              })()}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// ---- Private helpers ----

// c2-302: accepts an optional onRetry so per-tab errors can offer inline
// recovery without scrolling up to the header refresh button.
const AdminErr: React.FC<{ C: any; msg: string; onRetry?: () => void; retrying?: boolean }> = ({ C, msg, onRetry, retrying }) => (
  <div role='alert' style={{
    padding: '12px 14px', marginBottom: T.spacing.md,
    background: C.redBg, border: `1px solid ${C.redBorder}`,
    color: C.red, borderRadius: T.radii.md, fontSize: T.typography.sizeMd,
    display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.md,
  }}>
    <span><strong>Could not load:</strong> {msg}</span>
    {onRetry && (
      <button onClick={onRetry} disabled={retrying}
        style={{
          background: 'transparent', border: `1px solid ${C.redBorder}`,
          color: C.red, borderRadius: T.radii.sm,
          padding: `${T.spacing.xs} ${T.spacing.md}`,
          cursor: retrying ? 'wait' : 'pointer',
          fontFamily: 'inherit', fontSize: T.typography.sizeXs,
          fontWeight: T.typography.weightBold, textTransform: 'uppercase',
          letterSpacing: '0.06em', flexShrink: 0,
        }}>{retrying ? 'Retrying…' : 'Retry'}</button>
    )}
  </div>
);

const DashCard: React.FC<{ C: any; label: string; value: string; color: string }> = ({ C, label, value, color }) => (
  <div style={{
    padding: '16px 18px', borderRadius: T.radii.xl,
    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
  }}>
    <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
      {label}
    </div>
    <div style={{ fontSize: '16px', fontWeight: T.typography.weightBlack, color, marginTop: '6px', fontFamily: T.typography.fontMono, wordBreak: 'break-word' }}>
      {value}
    </div>
  </div>
);
