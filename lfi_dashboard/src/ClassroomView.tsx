import React, { useEffect, useState, useMemo } from 'react';
import { T } from './tokens';
// c2-343: 18/22/26px heading sizes need the design-system scale since
// T.typography caps at 22; sourced cross-platform so desktop/Android match.
import { typography as dsType } from './design-system';
// c2-346 / task 24: shared uppercase meta-label component.
import { Label } from './components/Label';
// c2-348 / task 28: shared error banner.
import { ErrorAlert } from './components/ErrorAlert';
// c2-349 / task 29: shared shimmer skeleton.
import { SkeletonLoader } from './components/SkeletonLoader';
// c2-350 / task 27: shared horizontal progress bar.
import { BarChart } from './components/BarChart';
// c2-351 / task 30: shared WAI-ARIA tablist.
import { TabBar } from './components/TabBar';
import { compactNum, formatRelative } from './util';

// ClassroomView — full page (not modal) per c0-027. The "school" metaphor:
// the AI is the student, training data is the curriculum, evaluation
// results are the gradebook. Eight sub-sections; for now all draw from
// /api/admin/dashboard until the classroom-specific endpoints land.

type Sub = 'profile' | 'curriculum' | 'gradebook' | 'lessons' | 'tests' | 'reports' | 'office' | 'library';

interface DashboardShape {
  overview?: { total_facts?: number; total_sources?: number; cve_facts?: number; adversarial_facts?: number; total_training_pairs?: number };
  quality?: { average?: number; high_quality_count?: number; low_quality_count?: number; high_quality_pct?: number };
  training?: { sessions?: number; learning_signals?: number; total_tested?: number; total_correct?: number; pass_rate?: number };
  score?: { accuracy_score?: number; grade?: string; breakdown?: { quality?: number; adversarial?: number; coverage?: number; training?: number } };
  domains?: Array<{ domain: string; count: number }>;
  training_files?: Array<{ file: string; pairs: number; size_mb: number }>;
  system?: { uptime_hours?: number; server_version?: string };
}

export interface ClassroomViewProps {
  C: any;
  host: string;
  isDesktop: boolean;
  // Optional: recent feedback/UI events captured locally. When provided,
  // Office Hours renders a quick activity log instead of a placeholder.
  localEvents?: Array<{ t: number; kind: string; data?: any }>;
}

const SUBS: Array<{ id: Sub; label: string; hint: string }> = [
  { id: 'profile',    label: 'Student Profile', hint: 'Grade, strengths, weaknesses' },
  { id: 'curriculum', label: 'Curriculum',      hint: 'Training datasets + sizes' },
  { id: 'gradebook',  label: 'Gradebook',       hint: 'Pass/fail + trends' },
  { id: 'lessons',    label: 'Lesson Plans',    hint: 'Active training sessions' },
  { id: 'tests',      label: 'Test Center',     hint: 'Benchmarks + quizzes' },
  { id: 'reports',    label: 'Report Cards',    hint: 'Weekly progress' },
  { id: 'office',     label: 'Office Hours',    hint: 'Feedback review' },
  { id: 'library',    label: 'Library',         hint: 'Fact browser' },
];

const gradeColor = (C: any, grade: string | undefined): string => {
  const g = grade || '';
  if (g.startsWith('A')) return C.green;
  if (g.startsWith('B')) return C.accent;
  if (g.startsWith('C')) return C.yellow;
  return C.red;
};
const pctNorm = (raw: number | undefined): number | null => {
  if (typeof raw !== 'number' || !isFinite(raw)) return null;
  return raw <= 1.5 ? raw * 100 : raw;
};

// c2-231 / #75: per-domain history snapshots. We don't have a backend
// time-series endpoint yet, so we snapshot domain counts each time the
// dashboard is polled and roll a bounded buffer in localStorage. 24 samples
// at the 10s poll cadence = the last ~4 minutes of activity — enough for a
// "trending up / flat / down" hint without blowing out storage.
const GRADEBOOK_HISTORY_KEY = 'lfi_gradebook_history_v1';
const GRADEBOOK_HISTORY_MAX = 24;
// Minimum gap between persisted snapshots — defends against React-strict
// double-invoke and manual refresh thrash writing every 50 ms.
const GRADEBOOK_SNAPSHOT_MIN_GAP_MS = 8_000;
interface GradebookSnapshot { ts: number; counts: Record<string, number> }
const loadGradebookHistory = (): GradebookSnapshot[] => {
  try {
    const raw = localStorage.getItem(GRADEBOOK_HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((s: any) => s && typeof s.ts === 'number' && s.counts && typeof s.counts === 'object')
      .slice(-GRADEBOOK_HISTORY_MAX);
  } catch { return []; }
};
const saveGradebookSnapshot = (prev: GradebookSnapshot[], counts: Record<string, number>): GradebookSnapshot[] => {
  const now = Date.now();
  const last = prev[prev.length - 1];
  if (last && (now - last.ts) < GRADEBOOK_SNAPSHOT_MIN_GAP_MS) return prev;
  const next = [...prev, { ts: now, counts }].slice(-GRADEBOOK_HISTORY_MAX);
  try { localStorage.setItem(GRADEBOOK_HISTORY_KEY, JSON.stringify(next)); } catch { /* quota */ }
  return next;
};
// Project a snapshot list into per-domain ordered series.
const projectHistory = (snaps: GradebookSnapshot[]): Record<string, number[]> => {
  const out: Record<string, number[]> = {};
  for (const s of snaps) {
    for (const [domain, count] of Object.entries(s.counts)) {
      if (!out[domain]) out[domain] = [];
      out[domain].push(count);
    }
  }
  return out;
};

// c2-260 / #122: persist active sub-tab so a reopen lands where the user
// left off. Validated against the known set to guard against stale strings.
const CLASSROOM_SUB_KEY = 'lfi_classroom_sub';
const CLASSROOM_SUBS: readonly Sub[] = ['profile','curriculum','gradebook','lessons','tests','reports','office','library'];

export const ClassroomView: React.FC<ClassroomViewProps> = ({ C, host, isDesktop, localEvents = [] }) => {
  const [sub, setSub] = useState<Sub>(() => {
    try {
      const stored = localStorage.getItem(CLASSROOM_SUB_KEY) as Sub | null;
      if (stored && CLASSROOM_SUBS.includes(stored)) return stored;
    } catch { /* storage blocked */ }
    return 'profile';
  });
  useEffect(() => {
    try { localStorage.setItem(CLASSROOM_SUB_KEY, sub); } catch { /* quota */ }
  }, [sub]);
  const [data, setData] = useState<DashboardShape | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  // c2-261: last successful fetch timestamp, surfaced next to the refresh
  // button as "Updated Xs ago" so users know staleness at a glance.
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  // c2-365 / tasks 125+126: sortable + filterable Curriculum table state.
  // Default sort matches the previous fixed behaviour (pairs desc) so the
  // page looks identical on first render.
  const [curricFilter, setCurricFilter] = useState('');
  const [curricSort, setCurricSort] = useState<{ col: 'file' | 'pairs' | 'size'; dir: 'asc' | 'desc' }>({ col: 'pairs', dir: 'desc' });
  // c2-369 / task 129: rolling pass-rate series for the sparkline. 10-point
  // cap so the chart stays readable; sessionStorage-backed so a full page
  // reload starts fresh (reflecting the intent that this is a live session
  // indicator, not a long-term trend).
  const [passRateSeries, setPassRateSeries] = useState<number[]>(() => {
    try {
      const raw = sessionStorage.getItem('scc_pass_rate_series');
      return raw ? (JSON.parse(raw) as number[]).slice(-10) : [];
    } catch { return []; }
  });
  // c2-369: push each fresh pass_rate observation into the sparkline series,
  // dedup adjacent identical values so the chart isn't flat-lined by a
  // paused backend, cap at 10 samples.
  useEffect(() => {
    const p = pctNorm(data?.training?.pass_rate);
    if (p == null) return;
    setPassRateSeries(prev => {
      if (prev.length > 0 && Math.abs(prev[prev.length - 1] - p) < 0.01) return prev;
      const next = [...prev, p].slice(-10);
      try { sessionStorage.setItem('scc_pass_rate_series', JSON.stringify(next)); } catch {}
      return next;
    });
  }, [data?.training?.pass_rate]);
  // c2-231 / #75: rolling history of per-domain counts, surfaced as
  // sparklines next to the coverage bars.
  const [history, setHistory] = useState<GradebookSnapshot[]>(() => loadGradebookHistory());

  const load = async () => {
    setLoading(true);
    setErr(null);
    // c2-321 / c0-035 #1: prefer the analytics service on :3002 — it
    // returns /analytics/overview + /analytics/domains in ~0.4s vs the
    // 60s timeout path that hits /api/admin/dashboard on :3000. Parallel
    // two-endpoint fetch is merged into the same DashboardShape the rest of
    // the component already consumes. If :3002 isn't up (older deployments
    // during rollout), fall back to the original consolidated endpoint.
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 4000);
      const [ovRes, domRes] = await Promise.all([
        fetch(`http://${host}:3002/analytics/overview`, { signal: ctrl.signal }),
        fetch(`http://${host}:3002/analytics/domains`, { signal: ctrl.signal }),
      ]);
      clearTimeout(to);
      if (!ovRes.ok || !domRes.ok) throw new Error(`HTTP overview=${ovRes.status} domains=${domRes.status}`);
      const overview: any = await ovRes.json();
      const domainsPayload: any = await domRes.json();
      const domainsArr: Array<{ domain: string; count: number }> =
        Array.isArray(domainsPayload?.domains) ? domainsPayload.domains
        : Array.isArray(domainsPayload) ? domainsPayload
        : [];
      // The analytics service keeps its own shape; project into the existing
      // DashboardShape the UI already knows how to render so no downstream
      // tab had to change.
      const shaped: DashboardShape = {
        overview: overview?.overview ?? overview,
        quality: overview?.quality,
        training: overview?.training,
        score: overview?.score,
        domains: domainsArr,
        training_files: overview?.training_files,
        system: overview?.system,
      };
      setData(shaped);
      setLastUpdated(Date.now());
      setLoading(false);
      return;
    } catch (e: any) {
      // Fall through to the legacy endpoint on :3000 — keeps the page
      // working during rollout or when the analytics service is down.
      console.debug('// SCC: classroom analytics(:3002) unreachable, falling back to /api/admin/dashboard:', e?.message || e);
    }
    try {
      const ctrl2 = new AbortController();
      const to2 = setTimeout(() => ctrl2.abort(), 10000);
      const res = await fetch(`http://${host}:3000/api/admin/dashboard`, { signal: ctrl2.signal });
      clearTimeout(to2);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setData(await res.json());
      setLastUpdated(Date.now());
    } catch (e: any) {
      const m = String(e?.message || e || 'fetch failed');
      setErr(m.includes('abort') ? 'Backend busy — request timed out. Try again in a moment.' : m);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { load(); /* eslint-disable-next-line */ }, []);
  // Auto-refresh every 10s per c0-027, but pause on tabs that are entirely
  // driven by local interaction (Test Center: user is typing in the audit
  // field; Office Hours: only reads localEvents; Library: user is typing in
  // the filter). Keeps background polling from disrupting typing.
  useEffect(() => {
    const liveTabs: Sub[] = ['profile', 'curriculum', 'gradebook', 'lessons', 'reports'];
    if (!liveTabs.includes(sub)) return;
    const id = setInterval(load, 10000);
    return () => clearInterval(id);
    // eslint-disable-next-line
  }, [sub]);

  const sortedDomains = useMemo(() => {
    const arr = data?.domains || [];
    return [...arr].sort((a, b) => b.count - a.count);
  }, [data?.domains]);

  // Snapshot domain counts on each successful load. Only fires when the
  // domain list arrives and looks sensible; the helper enforces the
  // minimum-gap + bounded-buffer invariants so effects running twice
  // (React Strict Mode) can't corrupt state.
  useEffect(() => {
    if (!data?.domains || data.domains.length === 0) return;
    const counts: Record<string, number> = {};
    for (const d of data.domains) counts[d.domain] = d.count;
    setHistory(prev => saveGradebookSnapshot(prev, counts));
  }, [data?.domains]);
  const historyByDomain = useMemo(() => projectHistory(history), [history]);

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
      background: C.bg, color: C.text, overflow: 'hidden',
      animation: 'lfi-fadein 0.18s ease-out',
    }}>
      {/* Sub-nav — WAI-ARIA tablist with arrow-key navigation. */}
      <TabBar<Sub> C={C} label='Classroom sections'
        padding='0 24px'
        background={C.bgCard}
        tabs={SUBS.map(s => ({ id: s.id, label: s.label, title: s.hint }))}
        active={sub}
        onChange={setSub}
        rightContent={(
          /* c2-259 / #121: manual refresh pushed to the right margin. Tabs
             driven by fresh data (profile/curriculum/gradebook/lessons/reports)
             already auto-poll at 10s but users want a force-reload after a
             backend action. Spinner while load in-flight. */
          <>
            {/* c2-261: staleness indicator — hidden until the first successful
                fetch so it doesn't flash "Updated 0s ago" before data lands. */}
            {lastUpdated != null && (
              <span aria-live='polite' style={{
                alignSelf: 'center', fontSize: T.typography.sizeXs, color: C.textDim,
                marginRight: T.spacing.sm, fontFamily: T.typography.fontMono,
              }}>Updated {formatRelative(lastUpdated)}</span>
            )}
            <button onClick={load} disabled={loading} aria-label='Refresh classroom data'
              title={loading ? 'Refreshing…' : 'Refresh (auto-refreshes every 10s on live tabs)'}
              style={{
                alignSelf: 'center', background: 'transparent',
                border: `1px solid ${C.borderSubtle}`, color: C.textMuted,
                borderRadius: T.radii.sm, cursor: loading ? 'wait' : 'pointer',
                padding: '4px 8px', marginRight: T.spacing.md,
                display: 'flex', alignItems: 'center', fontFamily: 'inherit',
              }}>
              <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor'
                strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round'
                style={loading ? { animation: 'scc-cls-spin 0.8s linear infinite' } : undefined}>
                <polyline points='23 4 23 10 17 10' />
                <polyline points='1 20 1 14 7 14' />
                <path d='M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15' />
              </svg>
            </button>
            <style>{`@keyframes scc-cls-spin { to { transform: rotate(360deg); } }`}</style>
          </>
        )} />

      {/* Body */}
      <div role='tabpanel' aria-label={sub}
        style={{ flex: 1, overflowY: 'auto', padding: T.spacing.xl, maxWidth: '1200px', width: '100%', margin: '0 auto' }}>
        {err && (
          <ErrorAlert C={C} message={err} onRetry={load} retrying={loading} mb={T.spacing.lg} />
        )}

        {/* --- Student Profile --- */}
        {sub === 'profile' && (
          <div>
            {/* Skeleton on first load (no cached data) — silent on subsequent
                auto-refreshes so the grade doesn't re-skeleton every 10s. */}
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{ textAlign: 'center', marginBottom: T.spacing.xl }}>
                <SkeletonLoader C={C}
                  width={isDesktop ? '180px' : '140px'}
                  height={isDesktop ? '128px' : '96px'}
                  style={{ margin: '0 auto' }} />
              </div>
            )}
            <div style={{ textAlign: 'center', marginBottom: T.spacing.xl, display: loading && !data ? 'none' : 'block' }}>
              <Label color={C.textMuted}>
                Accuracy grade
              </Label>
              <div style={{
                fontSize: isDesktop ? '128px' : '96px', fontWeight: T.typography.weightBlack,
                color: gradeColor(C, data?.score?.grade),
                lineHeight: 1, marginTop: '8px',
                fontFamily: T.typography.fontMono,
              }}>{data?.score?.grade || (loading ? '…' : '—')}</div>
              {typeof data?.score?.accuracy_score === 'number' && (
                <div style={{ fontSize: T.typography.sizeLg, color: C.textSecondary, marginTop: '6px', fontFamily: T.typography.fontMono }}>
                  {data.score.accuracy_score.toFixed(1)} / 100
                </div>
              )}
            </div>
            {data?.score?.breakdown && (
              <div style={{
                maxWidth: '640px', margin: '0 auto', padding: T.spacing.lg,
                background: C.bgCard, border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
              }}>
                {/* c2-366 / task 118: radar chart of the 4 breakdown scores.
                    Renders as a square SVG with 4 axes (N/E/S/W) and a filled
                    polygon whose vertices sit at distance proportional to
                    the score. Labels ride just outside each axis endpoint.
                    Hidden when any metric is non-numeric so we don't draw a
                    degenerate triangle. */}
                {(() => {
                  const keys = ['quality', 'adversarial', 'coverage', 'training'] as const;
                  const pts = keys.map(k => {
                    const v = data.score?.breakdown?.[k];
                    if (typeof v !== 'number') return null;
                    return v <= 1.5 ? v * 100 : v;
                  });
                  if (pts.some(p => p == null)) return null;
                  const size = 200;
                  const c = size / 2;
                  const r = size / 2 - 20;   // leave room for labels
                  // axis angles: top, right, bottom, left
                  const angles = [-Math.PI / 2, 0, Math.PI / 2, Math.PI];
                  const toXY = (pc: number, i: number) => {
                    const rr = (pc / 100) * r;
                    return [c + rr * Math.cos(angles[i]), c + rr * Math.sin(angles[i])];
                  };
                  const axisXY = (i: number) => [c + r * Math.cos(angles[i]), c + r * Math.sin(angles[i])];
                  const labelXY = (i: number) => [c + (r + 14) * Math.cos(angles[i]), c + (r + 14) * Math.sin(angles[i])];
                  const poly = pts.map((pc, i) => toXY(pc as number, i).join(',')).join(' ');
                  return (
                    <div style={{ display: 'flex', justifyContent: 'center', marginBottom: T.spacing.md }}>
                      <svg width={size} height={size} aria-label='Breakdown radar chart'
                        style={{ display: 'block' }}>
                        {/* concentric guide rings at 25/50/75/100 % */}
                        {[0.25, 0.5, 0.75, 1].map(f => (
                          <circle key={f} cx={c} cy={c} r={r * f}
                            fill='none' stroke={C.borderSubtle} strokeWidth={1} />
                        ))}
                        {/* axes */}
                        {angles.map((_, i) => {
                          const [ax, ay] = axisXY(i);
                          return <line key={i} x1={c} y1={c} x2={ax} y2={ay}
                            stroke={C.borderSubtle} strokeWidth={1} />;
                        })}
                        {/* filled polygon */}
                        <polygon points={poly}
                          fill={C.accentBg} stroke={C.accent} strokeWidth={2} />
                        {/* axis labels */}
                        {keys.map((k, i) => {
                          const [lx, ly] = labelXY(i);
                          return (
                            <text key={k} x={lx} y={ly}
                              fontSize={T.typography.sizeXs} fill={C.textMuted}
                              textAnchor='middle' dominantBaseline='middle'
                              style={{ textTransform: 'capitalize' }}>{k}</text>
                          );
                        })}
                      </svg>
                    </div>
                  );
                })()}
                <Label color={C.textMuted} mb={T.spacing.md}>
                  Strengths &amp; weaknesses
                </Label>
                {(['quality', 'adversarial', 'coverage', 'training'] as const).map(k => {
                  const v = data.score?.breakdown?.[k];
                  if (typeof v !== 'number') return null;
                  const pc = v <= 1.5 ? v * 100 : v;
                  const col = pc >= 80 ? C.green : pc >= 60 ? C.yellow : C.red;
                  return (
                    <div key={k} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, marginBottom: T.spacing.sm }}>
                      <span style={{ width: '110px', fontSize: T.typography.sizeMd, color: C.text, textTransform: 'capitalize' }}>{k}</span>
                      <BarChart C={C} value={pc} color={col} height='12px' trackBg={C.bgInput} style={{ flex: 1 }} />
                      <span style={{ width: '56px', textAlign: 'right', fontSize: T.typography.sizeMd, color: col, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{pc.toFixed(0)}</span>
                    </div>
                  );
                })}
                {/* c2-363 / task 120: auto-detect strengths (>=80) and
                    weaknesses (<60) and surface them as a compact summary row
                    at the bottom of the breakdown card. Skips any metric that
                    falls into the middle (60-79) tier -- those are neither
                    highlights nor concerns. If the row is empty (all middle)
                    we render nothing. */}
                {(() => {
                  const entries = (['quality', 'adversarial', 'coverage', 'training'] as const)
                    .map(k => {
                      const v = data.score?.breakdown?.[k];
                      if (typeof v !== 'number') return null;
                      const pc = v <= 1.5 ? v * 100 : v;
                      return { k, pc };
                    })
                    .filter((e): e is { k: string; pc: number } => e !== null);
                  const strengths = entries.filter(e => e.pc >= 80);
                  const weaknesses = entries.filter(e => e.pc < 60);
                  if (strengths.length === 0 && weaknesses.length === 0) return null;
                  return (
                    <div style={{
                      display: 'flex', gap: T.spacing.md, flexWrap: 'wrap',
                      marginTop: T.spacing.md, paddingTop: T.spacing.md,
                      borderTop: `1px solid ${C.borderSubtle}`,
                      fontSize: T.typography.sizeSm,
                    }}>
                      {strengths.length > 0 && (
                        <span style={{ color: C.green }}>
                          <strong>Strengths:</strong> {strengths.map(e => e.k).join(', ')}
                        </span>
                      )}
                      {weaknesses.length > 0 && (
                        <span style={{ color: C.red }}>
                          <strong>Weaknesses:</strong> {weaknesses.map(e => e.k).join(', ')}
                        </span>
                      )}
                    </div>
                  );
                })()}
              </div>
            )}
          </div>
        )}

        {/* --- Curriculum --- */}
        {sub === 'curriculum' && (
          <div>
            <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 16px' }}>Curriculum</h2>
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.sm }}>
                {[0, 1, 2, 3, 4].map(i => (
                  <SkeletonLoader key={i} C={C} height='40px' borderRadius={T.radii.md} delay={i * 0.08} />
                ))}
              </div>
            )}
            {data?.training_files && data.training_files.length > 0 ? (() => {
              // c2-365 / tasks 125+126: filter + sort pipeline. Filtering
              // happens before sort so the sort doesn't run on hidden rows.
              // Case-insensitive substring match on file name only.
              const q = curricFilter.trim().toLowerCase();
              const filtered = q
                ? data.training_files.filter(f => f.file.toLowerCase().includes(q))
                : data.training_files;
              const sorted = [...filtered].sort((a, b) => {
                const sign = curricSort.dir === 'asc' ? 1 : -1;
                if (curricSort.col === 'file') return sign * a.file.localeCompare(b.file);
                if (curricSort.col === 'pairs') return sign * (a.pairs - b.pairs);
                return sign * (a.size_mb - b.size_mb);
              });
              const toggleSort = (col: 'file' | 'pairs' | 'size') =>
                setCurricSort(s => s.col === col
                  ? { col, dir: s.dir === 'asc' ? 'desc' : 'asc' }
                  : { col, dir: col === 'file' ? 'asc' : 'desc' });
              const arrow = (col: 'file' | 'pairs' | 'size') =>
                curricSort.col !== col ? '' : curricSort.dir === 'asc' ? ' \u25B2' : ' \u25BC';
              const ariaSort = (col: 'file' | 'pairs' | 'size'): 'ascending' | 'descending' | 'none' =>
                curricSort.col !== col ? 'none' : curricSort.dir === 'asc' ? 'ascending' : 'descending';
              const thBase: React.CSSProperties = {
                padding: '10px 14px', fontWeight: T.typography.weightBold,
                color: C.textSecondary, background: C.bgCard,
                borderBottom: `1px solid ${C.borderSubtle}`, cursor: 'pointer',
                userSelect: 'none',
              };
              return (
                <>
                  <div style={{ marginBottom: T.spacing.md }}>
                    <input type='search' value={curricFilter}
                      onChange={(e) => setCurricFilter(e.target.value)}
                      onKeyDown={(e) => { if (e.key === 'Escape') setCurricFilter(''); }}
                      placeholder={`Filter datasets... (${data.training_files.length})`}
                      aria-label='Filter curriculum datasets'
                      style={{
                        width: '100%', maxWidth: '420px',
                        padding: `${T.spacing.sm} ${T.spacing.md}`,
                        background: C.bgInput,
                        border: `1px solid ${C.borderSubtle}`,
                        borderRadius: T.radii.sm,
                        color: C.text, fontSize: T.typography.sizeSm,
                        fontFamily: 'inherit', outline: 'none',
                      }} />
                    {q && (
                      <span style={{
                        marginLeft: T.spacing.md, fontSize: T.typography.sizeXs,
                        color: C.textMuted, fontFamily: T.typography.fontMono,
                      }}>{filtered.length} match{filtered.length === 1 ? '' : 'es'}</span>
                    )}
                  </div>
                  <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeMd }}>
                      <thead>
                        <tr>
                          <th onClick={() => toggleSort('file')}
                            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleSort('file'); } }}
                            tabIndex={0} role='button' aria-sort={ariaSort('file')}
                            style={{ ...thBase, textAlign: 'left' }}>Dataset{arrow('file')}</th>
                          <th onClick={() => toggleSort('pairs')}
                            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleSort('pairs'); } }}
                            tabIndex={0} role='button' aria-sort={ariaSort('pairs')}
                            style={{ ...thBase, textAlign: 'right' }}>Pairs{arrow('pairs')}</th>
                          <th onClick={() => toggleSort('size')}
                            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleSort('size'); } }}
                            tabIndex={0} role='button' aria-sort={ariaSort('size')}
                            style={{ ...thBase, textAlign: 'right' }}>Size{arrow('size')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {sorted.length === 0 ? (
                          <tr><td colSpan={3} style={{ padding: '20px', textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeSm }}>
                            No datasets match "{curricFilter}"
                          </td></tr>
                        ) : sorted.map(f => (
                          <tr key={f.file}>
                            <td style={{ padding: '10px 14px', fontFamily: T.typography.fontMono, color: C.text }}>{f.file}</td>
                            <td style={{ padding: '10px 14px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.accent, fontWeight: T.typography.weightBold }}>{f.pairs.toLocaleString()}</td>
                            <td style={{ padding: '10px 14px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              );
            })() : (
              <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                {loading ? 'Loading curriculum…' : 'No training files reported.'}
              </div>
            )}
          </div>
        )}

        {/* --- Gradebook --- */}
        {sub === 'gradebook' && (
          <div>
            <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 16px' }}>Gradebook</h2>
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {[0, 1, 2, 3].map(i => (
                  <SkeletonLoader key={i} C={C} height='80px' delay={i * 0.08} />
                ))}
              </div>
            )}
            <div style={{
              display: loading && !data ? 'none' : 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
              gap: T.spacing.md, marginBottom: T.spacing.xl,
            }}>
              {/* c2-369 / task 129: Pass rate Stat now renders with a
                  below-value sparkline. When the series has 2+ samples we
                  draw an SVG polyline inside the card body. */}
              {(() => {
                const p = pctNorm(data?.training?.pass_rate);
                const valueText = p != null ? `${p.toFixed(1)}%` : '—';
                const color = p == null ? C.textMuted : p >= 95 ? C.green : p >= 80 ? C.yellow : C.red;
                const series = passRateSeries;
                const sparkW = 160, sparkH = 32;
                const minV = Math.min(...series, 0);
                const maxV = Math.max(...series, 100);
                const span = Math.max(1, maxV - minV);
                const toPt = (v: number, i: number) => {
                  const x = series.length === 1 ? sparkW / 2 : (i / (series.length - 1)) * sparkW;
                  const y = sparkH - ((v - minV) / span) * sparkH;
                  return `${x},${y}`;
                };
                return (
                  <div style={{
                    padding: `${T.spacing.md} ${T.spacing.lg}`, borderRadius: T.radii.md,
                    background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                  }}>
                    <Label color={C.textMuted}>Pass rate</Label>
                    <div style={{
                      fontSize: '24px', fontWeight: T.typography.weightBlack,
                      color, marginTop: T.spacing.xs, fontFamily: T.typography.fontMono,
                    }}>{valueText}</div>
                    {series.length >= 2 && (
                      <svg width={sparkW} height={sparkH} style={{ marginTop: '4px', display: 'block' }}
                        aria-label={`Pass rate trend, ${series.length} samples`}>
                        <polyline points={series.map((v, i) => toPt(v, i)).join(' ')}
                          fill='none' stroke={color} strokeWidth={2}
                          strokeLinecap='round' strokeLinejoin='round' />
                        {series.map((v, i) => {
                          const [x, y] = toPt(v, i).split(',').map(Number);
                          return <circle key={i} cx={x} cy={y} r={2} fill={color} />;
                        })}
                      </svg>
                    )}
                  </div>
                );
              })()}
              <Stat C={C} label='Tested' value={data?.training?.total_tested != null ? compactNum(data.training.total_tested) : '—'} color={C.accent} />
              <Stat C={C} label='Correct' value={data?.training?.total_correct != null ? compactNum(data.training.total_correct) : '—'} color={C.green} />
              <Stat C={C} label='Avg quality' value={typeof data?.quality?.average === 'number' ? data.quality.average.toFixed(2) : '—'} color={C.yellow} />
            </div>
            {/* c2-368 / task 131: quality distribution histogram. The backend
                currently only exposes aggregate buckets (high/low counts +
                average), not a per-fact quality array, so a true 10-bin
                histogram isn't renderable. Until the backend adds /api/
                classroom/quality_distribution we render the 3-bin view
                (low / mid / high) from the available counts so the surface
                is not empty. Fill colors match the stat-card accents. */}
            {data?.quality && (typeof data.quality.high_quality_count === 'number' ||
                typeof data.quality.low_quality_count === 'number') && (() => {
              const hi = data.quality.high_quality_count ?? 0;
              const lo = data.quality.low_quality_count ?? 0;
              const total = hi + lo;
              const mid = 0; // placeholder until backend exposes per-fact bins
              const bins = [
                { label: 'Low',  n: lo,  col: C.red },
                { label: 'Mid',  n: mid, col: C.yellow },
                { label: 'High', n: hi,  col: C.green },
              ];
              const max = Math.max(...bins.map(b => b.n), 1);
              if (total === 0) return null;
              const width = 420;
              const height = 140;
              const barW = (width - 60) / bins.length;
              return (
                <div style={{ marginBottom: T.spacing.xl }}>
                  <Label color={C.textMuted} mb={T.spacing.md}>
                    Quality distribution
                  </Label>
                  <svg width={width} height={height} aria-label='Quality distribution histogram'>
                    {bins.map((b, i) => {
                      const h = (b.n / max) * (height - 30);
                      const x = 40 + i * barW + barW * 0.15;
                      const w = barW * 0.7;
                      const y = (height - 20) - h;
                      return (
                        <g key={b.label}>
                          <rect x={x} y={y} width={w} height={h} fill={b.col} rx={3} />
                          <text x={x + w / 2} y={y - 4}
                            fontSize={T.typography.sizeXs}
                            fill={C.textSecondary} textAnchor='middle'
                            fontFamily={T.typography.fontMono}>
                            {b.n.toLocaleString()}
                          </text>
                          <text x={x + w / 2} y={height - 4}
                            fontSize={T.typography.sizeXs}
                            fill={C.textMuted} textAnchor='middle'>
                            {b.label}
                          </text>
                        </g>
                      );
                    })}
                  </svg>
                </div>
              );
            })()}
            {sortedDomains.length > 0 && (
              <div>
                <Label color={C.textMuted} mb={T.spacing.md}>
                  Coverage by domain
                </Label>
                <DomainBars C={C} rows={sortedDomains.slice(0, 15)} historyByDomain={historyByDomain} />
              </div>
            )}
          </div>
        )}

        {/* --- Lesson Plans --- */}
        {sub === 'lessons' && (
          <LessonsTab C={C} training={data?.training} files={data?.training_files || []} />
        )}

        {/* --- Test Center --- */}
        {sub === 'tests' && (
          <TestCenterTab C={C} host={host} data={data} />
        )}

        {/* --- Report Cards --- */}
        {sub === 'reports' && (
          <ReportsTab C={C} data={data} sortedDomains={sortedDomains} />
        )}

        {/* --- Office Hours --- */}
        {sub === 'office' && (
          <OfficeHoursTab C={C} events={localEvents} />
        )}

        {/* --- Library --- */}
        {sub === 'library' && (
          <LibraryTab C={C} host={host} domains={sortedDomains} files={data?.training_files || []} />
        )}
      </div>
    </div>
  );
};

// ---- Private helpers ----

const Stat: React.FC<{ C: any; label: string; value: string; color: string }> = ({ C, label, value, color }) => (
  <div style={{
    padding: '16px 18px', borderRadius: T.radii.lg,
    background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
  }}>
    <Label color={C.textMuted}>{label}</Label>
    <div style={{ fontSize: dsType.sizes['2xl'], fontWeight: T.typography.weightBlack, color, marginTop: '4px', fontFamily: T.typography.fontMono }}>{value}</div>
  </div>
);

// Inline, dependency-free sparkline. Renders empty when <2 samples are
// available so a fresh page doesn't flash "flat line" artefacts. Color
// matches the bar so the eye groups them as one row.
const Sparkline: React.FC<{ values: number[]; color: string; width?: number; height?: number }> = ({ values, color, width = 64, height = 18 }) => {
  if (!values || values.length < 2) {
    return <span aria-hidden='true' style={{ display: 'inline-block', width, height }} />;
  }
  const max = Math.max(...values);
  const min = Math.min(...values);
  const range = max - min;
  const step = width / (values.length - 1);
  // Flat line: put it vertically centered so it reads as "no change".
  const y = (v: number) => range === 0 ? height / 2 : height - ((v - min) / range) * (height - 2) - 1;
  const points = values.map((v, i) => `${(i * step).toFixed(1)},${y(v).toFixed(1)}`).join(' ');
  const first = values[0]; const last = values[values.length - 1];
  const trendSymbol = last > first ? '\u2191' : last < first ? '\u2193' : '\u2192';
  return (
    <svg width={width} height={height}
      role='img' aria-label={`Trend ${trendSymbol} (${values.length} samples, latest ${last.toLocaleString()})`}
      style={{ display: 'block' }}>
      <polyline fill='none' stroke={color} strokeWidth='1.5'
        strokeLinecap='round' strokeLinejoin='round' points={points} />
    </svg>
  );
};

const DomainBars: React.FC<{
  C: any;
  rows: Array<{ domain: string; count: number }>;
  historyByDomain?: Record<string, number[]>;
}> = ({ C, rows, historyByDomain = {} }) => {
  const max = Math.max(...rows.map(r => r.count), 1);
  const colorFor = (n: number) => n > 10000 ? C.green : n > 1000 ? C.yellow : C.red;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
      {rows.map(r => {
        const series = historyByDomain[r.domain] || [];
        return (
          <div key={r.domain} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
            <span style={{ width: '160px', fontSize: T.typography.sizeSm, color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{r.domain}</span>
            <BarChart C={C} value={(r.count / max) * 100} color={colorFor(r.count)} style={{ flex: 1 }} />
            <div style={{ width: '64px', flexShrink: 0 }}>
              <Sparkline values={series} color={colorFor(r.count)} />
            </div>
            <span style={{ width: '96px', textAlign: 'right', fontSize: T.typography.sizeSm, fontFamily: T.typography.fontMono, color: C.textMuted }}>{r.count.toLocaleString()}</span>
          </div>
        );
      })}
    </div>
  );
};

const ReportsTab: React.FC<{ C: any; data: DashboardShape | null; sortedDomains: Array<{ domain: string; count: number }> }> = ({ C, data, sortedDomains }) => {
  const topDomain = sortedDomains[0];
  const totalFacts = data?.overview?.total_facts;
  const totalPairs = data?.overview?.total_training_pairs ?? (data?.training_files || []).reduce((s, f) => s + f.pairs, 0);
  const adv = data?.overview?.adversarial_facts ?? 0;
  const avgQ = data?.quality?.average;
  const passRate = pctNorm(data?.training?.pass_rate);
  const grade = data?.score?.grade || '—';
  return (
    <div>
      <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Report Cards</h2>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Point-in-time scorecard. A proper weekly rollup (deltas vs last week) will populate once
        /api/classroom/reports ships historical aggregates.
      </p>
      {/* Big scorecard grid */}
      <div style={{
        display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
        gap: T.spacing.md, marginBottom: T.spacing.xl,
      }}>
        <Stat C={C} label='Grade' value={grade} color={(() => {
          if (grade.startsWith('A')) return C.green;
          if (grade.startsWith('B')) return C.accent;
          if (grade.startsWith('C')) return C.yellow;
          if (grade === '—') return C.textMuted;
          return C.red;
        })()} />
        <Stat C={C} label='Pass rate' value={passRate != null ? `${passRate.toFixed(1)}%` : '—'} color={passRate == null ? C.textMuted : passRate >= 95 ? C.green : passRate >= 80 ? C.yellow : C.red} />
        <Stat C={C} label='Avg quality' value={typeof avgQ === 'number' ? avgQ.toFixed(2) : '—'} color={typeof avgQ === 'number' ? (avgQ >= 0.8 ? C.green : avgQ >= 0.5 ? C.yellow : C.red) : C.textMuted} />
        <Stat C={C} label='Total facts' value={typeof totalFacts === 'number' ? totalFacts.toLocaleString() : '—'} color={C.purple} />
        <Stat C={C} label='Training pairs' value={totalPairs ? totalPairs.toLocaleString() : '—'} color={C.accent} />
        <Stat C={C} label='Adversarial' value={adv ? adv.toLocaleString() : '—'} color={C.red} />
        <Stat C={C} label='Domains' value={sortedDomains.length ? String(sortedDomains.length) : '—'} color={C.textSecondary} />
        <Stat C={C} label='Top domain' value={topDomain ? topDomain.domain : '—'} color={C.green} />
      </div>
      <div style={{
        padding: T.spacing.lg, background: C.bgCard,
        border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
        fontSize: T.typography.sizeMd, color: C.textSecondary, lineHeight: 1.6,
      }}>
        {typeof data?.system?.uptime_hours === 'number' && (
          <div style={{ marginBottom: T.spacing.sm }}>
            <strong style={{ color: C.text }}>Server uptime:</strong> {data.system.uptime_hours.toFixed(1)} hours
          </div>
        )}
        {typeof data?.training?.sessions === 'number' && (
          <div style={{ marginBottom: T.spacing.sm }}>
            <strong style={{ color: C.text }}>Training sessions logged:</strong> {data.training.sessions.toLocaleString()}
          </div>
        )}
        {typeof data?.training?.learning_signals === 'number' && (
          <div style={{ marginBottom: T.spacing.sm }}>
            <strong style={{ color: C.text }}>Learning signals received:</strong> {data.training.learning_signals.toLocaleString()}
          </div>
        )}
        {typeof data?.training?.total_tested === 'number' && typeof data?.training?.total_correct === 'number' && (
          <div style={{ marginBottom: T.spacing.sm }}>
            <strong style={{ color: C.text }}>Evaluation record:</strong> {data.training.total_correct.toLocaleString()} correct of {data.training.total_tested.toLocaleString()} tested
          </div>
        )}
        {data?.quality?.high_quality_count != null && data?.quality?.low_quality_count != null && (
          <div style={{ marginBottom: T.spacing.sm }}>
            <strong style={{ color: C.text }}>Quality distribution:</strong> {data.quality.high_quality_count.toLocaleString()} high &middot; {data.quality.low_quality_count.toLocaleString()} low
          </div>
        )}
      </div>
    </div>
  );
};

interface AuditHistoryEntry { t: number; prompt: string; verdict?: string; passed?: boolean; raw?: any }
const AUDIT_HISTORY_KEY = 'lfi_audit_history_v1';
const AUDIT_HISTORY_CAP = 10;

const TestCenterTab: React.FC<{ C: any; host: string; data: DashboardShape | null }> = ({ C, host, data }) => {
  const [auditInput, setAuditInput] = React.useState('');
  const [auditResult, setAuditResult] = React.useState<any>(null);
  const [auditError, setAuditError] = React.useState<string | null>(null);
  const [auditLoading, setAuditLoading] = React.useState(false);
  // Rolling history of the last 10 audits, persisted to localStorage so
  // the user can revisit past verdicts across page reloads.
  const [history, setHistory] = React.useState<AuditHistoryEntry[]>(() => {
    try {
      const raw = localStorage.getItem(AUDIT_HISTORY_KEY);
      return raw ? JSON.parse(raw) as AuditHistoryEntry[] : [];
    } catch { return []; }
  });
  const [expandedHistoryIdx, setExpandedHistoryIdx] = React.useState<number | null>(null);
  const runAudit = async () => {
    const text = auditInput.trim();
    if (!text) return;
    setAuditLoading(true);
    setAuditError(null);
    setAuditResult(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const res = await fetch(`http://${host}:3000/api/audit`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text }), signal: ctrl.signal,
      });
      clearTimeout(to);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      setAuditResult(json);
      // Push into history (newest first, cap at 10), persist to localStorage.
      const verdict: string | undefined = json?.verdict || json?.status;
      const passed: boolean | undefined = typeof json?.pass === 'boolean' ? json.pass
        : typeof json?.passed === 'boolean' ? json.passed
        : (verdict && /pass|ok|true/i.test(String(verdict)));
      const entry: AuditHistoryEntry = { t: Date.now(), prompt: text, verdict, passed, raw: json };
      const next = [entry, ...history].slice(0, AUDIT_HISTORY_CAP);
      setHistory(next);
      try { localStorage.setItem(AUDIT_HISTORY_KEY, JSON.stringify(next)); } catch { /* quota */ }
    } catch (e: any) {
      setAuditError(String(e?.message || e || 'fetch failed'));
    } finally {
      setAuditLoading(false);
    }
  };
  const clearHistory = () => {
    setHistory([]);
    try { localStorage.removeItem(AUDIT_HISTORY_KEY); } catch {}
  };
  const psl = data?.quality?.psl_calibration;
  return (
    <div>
      <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Test Center</h2>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Run a PSL audit against any text using the existing /api/audit endpoint. PSL calibration below shows the
        system-wide pass rate the last time a full sweep ran.
      </p>
      {/* Calibration status card */}
      <div style={{
        display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
        gap: T.spacing.md, marginBottom: T.spacing.xl,
      }}>
        <Stat C={C} label='PSL pass rate' value={(() => {
          const p = pctNorm(psl?.pass_rate ?? data?.training?.pass_rate);
          return p != null ? `${p.toFixed(1)}%` : '—';
        })()} color={(() => {
          const p = pctNorm(psl?.pass_rate ?? data?.training?.pass_rate);
          return p == null ? C.textMuted : p >= 95 ? C.green : p >= 80 ? C.yellow : C.red;
        })()} />
        <Stat C={C} label='PSL status' value={psl?.status || '—'} color={C.accent} />
        <Stat C={C} label='Last run' value={psl?.last_run ? String(psl.last_run) : '—'} color={C.textSecondary} />
        <Stat C={C} label='Tested' value={data?.training?.total_tested != null ? data.training.total_tested.toLocaleString() : '—'} color={C.purple} />
      </div>
      {/* Ad-hoc audit */}
      <div style={{
        padding: T.spacing.lg, border: `1px solid ${C.borderSubtle}`,
        borderRadius: T.radii.md, background: C.bgCard,
      }}>
        <div style={{ fontSize: T.typography.sizeSm, fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.md }}>
          Ad-hoc PSL audit
        </div>
        <textarea value={auditInput}
          onChange={(e) => setAuditInput(e.target.value)}
          placeholder='Paste a statement, citation, or fact claim to audit…'
          aria-label='Audit input'
          autoComplete='off' spellCheck
          maxLength={10000}
          style={{
            width: '100%', minHeight: '96px', padding: '10px 12px',
            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
            fontSize: T.typography.sizeBody, outline: 'none', resize: 'vertical', boxSizing: 'border-box',
          }} />
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: T.spacing.md }}>
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim }}>{auditInput.length}/10000</span>
          <button onClick={runAudit} disabled={auditLoading || !auditInput.trim()}
            style={{
              padding: '8px 18px', background: auditLoading || !auditInput.trim() ? C.bgInput : C.accent,
              color: auditLoading || !auditInput.trim() ? C.textMuted : '#fff',
              border: 'none', borderRadius: T.radii.sm, cursor: auditLoading ? 'wait' : (auditInput.trim() ? 'pointer' : 'not-allowed'),
              fontFamily: 'inherit', fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
            }}>{auditLoading ? 'Auditing…' : 'Run audit'}</button>
        </div>
        {auditError && (
          <div role='alert' style={{
            marginTop: T.spacing.md, padding: '8px 12px',
            background: C.redBg, border: `1px solid ${C.redBorder}`, color: C.red,
            borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
          }}>{auditError}</div>
        )}
        {auditResult && (
          <>
            {/* c2-364 / task 143: confidence meter above the JSON. Reads
                from the common 'confidence' / 'score' / 'probability' fields;
                skipped entirely if none is present. Gradient red -> yellow
                -> green via threshold color, not a CSS gradient, so the
                color jumps rather than interpolates -- easier to read at
                a glance than a smooth rainbow. */}
            {(() => {
              const raw = (auditResult && typeof auditResult === 'object')
                ? (auditResult.confidence ?? auditResult.score ?? auditResult.probability)
                : null;
              const n = typeof raw === 'number' ? raw : null;
              if (n == null) return null;
              const v01 = Math.max(0, Math.min(1, n > 1.5 ? n / 100 : n));
              const col = v01 < 0.33 ? C.red : v01 < 0.67 ? C.yellow : C.green;
              return (
                <div style={{
                  marginTop: T.spacing.md, display: 'flex',
                  alignItems: 'center', gap: T.spacing.sm,
                }}>
                  <span style={{ width: '96px', fontSize: T.typography.sizeSm, color: C.textMuted }}>
                    Confidence
                  </span>
                  <BarChart C={C} value={v01 * 100} color={col} height='12px'
                    trackBg={C.bgInput} style={{ flex: 1 }} />
                  <span style={{
                    width: '56px', textAlign: 'right',
                    fontSize: T.typography.sizeSm, color: col,
                    fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold,
                  }}>{(v01 * 100).toFixed(0)}%</span>
                </div>
              );
            })()}
            <pre style={{
              marginTop: T.spacing.md, padding: T.spacing.md, background: C.bgInput,
              border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
              fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
              color: C.text, whiteSpace: 'pre-wrap', overflowX: 'auto', maxHeight: '320px',
            }}>
              {JSON.stringify(auditResult, null, 2)}
            </pre>
          </>
        )}
      </div>
      {/* Rolling audit history — last 10, localStorage-backed */}
      {history.length > 0 && (
        <div style={{ marginTop: T.spacing.xl }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: T.spacing.md }}>
            <Label color={C.textMuted}>
              History ({history.length})
            </Label>
            <button onClick={clearHistory}
              style={{
                padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                color: C.textMuted, borderRadius: T.radii.sm, cursor: 'pointer',
                fontFamily: 'inherit', textTransform: 'uppercase',
              }}>Clear</button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {history.map((h, i) => {
              const isOpen = expandedHistoryIdx === i;
              const color = h.passed === true ? C.green : h.passed === false ? C.red : C.textMuted;
              return (
                <div key={h.t} style={{
                  border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
                  background: C.bgCard, overflow: 'hidden',
                }}>
                  <button onClick={() => setExpandedHistoryIdx(isOpen ? null : i)}
                    aria-expanded={isOpen}
                    style={{
                      width: '100%', display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                      padding: '10px 12px', background: 'transparent', border: 'none',
                      cursor: 'pointer', fontFamily: 'inherit', textAlign: 'left',
                    }}>
                    <span style={{
                      width: '8px', height: '8px', borderRadius: '50%', background: color, flexShrink: 0,
                    }} aria-hidden='true' />
                    <span style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono, flexShrink: 0 }}>
                      {new Date(h.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </span>
                    <span style={{ fontSize: T.typography.sizeSm, color: C.text, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {h.prompt}
                    </span>
                    {h.verdict && <span style={{ fontSize: T.typography.sizeXs, color, fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold }}>{h.verdict}</span>}
                    <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>{isOpen ? '▴' : '▾'}</span>
                  </button>
                  {isOpen && (
                    <pre style={{
                      margin: 0, padding: '10px 12px', background: C.bgInput,
                      borderTop: `1px solid ${C.borderSubtle}`,
                      fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
                      color: C.text, whiteSpace: 'pre-wrap', overflowX: 'auto', maxHeight: '240px',
                    }}>{JSON.stringify(h.raw, null, 2)}</pre>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

const LessonsTab: React.FC<{
  C: any;
  training?: DashboardShape['training'];
  files: Array<{ file: string; pairs: number; size_mb: number }>;
}> = ({ C, training, files }) => {
  const totalPairs = files.reduce((s, f) => s + f.pairs, 0);
  const totalMb = files.reduce((s, f) => s + f.size_mb, 0);
  return (
    <div>
      <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Lesson Plans</h2>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Snapshot of the training roster. Full run-control (start/stop/queue) lands when /api/classroom/lessons
        exposes session controls; for now this reflects what the consolidated dashboard reports.
      </p>
      <div style={{
        display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
        gap: T.spacing.md, marginBottom: T.spacing.xl,
      }}>
        <Stat C={C} label='Sessions' value={typeof training?.sessions === 'number' ? training.sessions.toLocaleString() : '—'} color={C.accent} />
        <Stat C={C} label='Learning signals' value={typeof training?.learning_signals === 'number' ? training.learning_signals.toLocaleString() : '—'} color={C.purple} />
        <Stat C={C} label='Total pairs' value={totalPairs ? totalPairs.toLocaleString() : '—'} color={C.green} />
        <Stat C={C} label='Total size' value={totalMb ? `${totalMb.toFixed(1)} MB` : '—'} color={C.yellow} />
      </div>
      {files.length > 0 && (
        <div>
          <Label color={C.textMuted} mb={T.spacing.md}>
            Active roster (by pairs)
          </Label>
          <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeSm }}>
              <thead>
                <tr>
                  <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Dataset</th>
                  <th style={{ textAlign: 'right', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Pairs</th>
                  <th style={{ textAlign: 'right', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Size</th>
                  <th style={{ textAlign: 'right', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Share</th>
                </tr>
              </thead>
              <tbody>
                {[...files].sort((a, b) => b.pairs - a.pairs).slice(0, 50).map(f => {
                  const share = totalPairs > 0 ? (f.pairs / totalPairs) * 100 : 0;
                  return (
                    <tr key={f.file}>
                      <td style={{ padding: '8px 12px', fontFamily: T.typography.fontMono, color: C.text }}>{f.file}</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.accent }}>{f.pairs.toLocaleString()}</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: T.typography.fontMono, color: C.textMuted }}>{share.toFixed(1)}%</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};

const OfficeHoursTab: React.FC<{ C: any; events: Array<{ t: number; kind: string; data?: any }> }> = ({ C, events }) => {
  const feedback = events
    .filter(e => e.kind === 'feedback_positive' || e.kind === 'feedback_negative')
    .slice()
    .reverse();
  const posCount = feedback.filter(e => e.kind === 'feedback_positive').length;
  const negCount = feedback.length - posCount;
  return (
    <div>
      <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Office Hours</h2>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Review user feedback captured from thumbs-up/down on AI responses.
        Session-local only until /api/classroom/feedback aggregates server-side history.
      </p>
      <div style={{ display: 'flex', gap: T.spacing.md, marginBottom: T.spacing.xl }}>
        <div style={{ flex: 1, padding: T.spacing.md, background: C.greenBg, border: `1px solid ${C.greenBorder}`, borderRadius: T.radii.md }}>
          <Label color={C.green}>Positive</Label>
          <div style={{ fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack, color: C.green, fontFamily: T.typography.fontMono }}>{posCount}</div>
        </div>
        <div style={{ flex: 1, padding: T.spacing.md, background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: T.radii.md }}>
          <Label color={C.red}>Negative</Label>
          <div style={{ fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack, color: C.red, fontFamily: T.typography.fontMono }}>{negCount}</div>
        </div>
        {/* c2-365 / task 152: overall sentiment card. Green at >=70%,
            yellow at 50-70%, red below 50%. Hidden when no feedback has
            been captured yet -- division by zero + "0% positive" on an
            empty log is noise rather than information. */}
        {feedback.length > 0 && (() => {
          const pct = Math.round((posCount / feedback.length) * 100);
          const col = pct >= 70 ? C.green : pct >= 50 ? C.yellow : C.red;
          const bg = pct >= 70 ? C.greenBg : pct >= 50 ? C.yellowBg : C.redBg;
          const border = pct >= 70 ? C.greenBorder : pct >= 50 ? C.accentBorder : C.redBorder;
          return (
            <div style={{
              flex: 1, padding: T.spacing.md,
              background: bg, border: `1px solid ${border}`,
              borderRadius: T.radii.md,
            }}>
              <Label color={col}>Sentiment</Label>
              <div style={{
                fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack,
                color: col, fontFamily: T.typography.fontMono,
              }}>{pct}%</div>
              <div style={{
                fontSize: T.typography.sizeXs, color: C.textMuted,
                fontFamily: T.typography.fontMono, marginTop: '2px',
              }}>{feedback.length} total</div>
            </div>
          );
        })()}
      </div>
      {feedback.length === 0 ? (
        <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeMd, fontStyle: 'italic' }}>
          No feedback captured this session yet. Use 👍 / 👎 on any AI response to populate this log.
        </div>
      ) : (
        <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeSm }}>
            <thead>
              <tr>
                <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>When</th>
                <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Rating</th>
                <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Category</th>
                <th style={{ textAlign: 'left', padding: '8px 12px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Detail</th>
              </tr>
            </thead>
            <tbody>
              {feedback.slice(0, 50).map((e, i) => {
                const isPos = e.kind === 'feedback_positive';
                const category = e.data?.category || (isPos ? '—' : '—');
                const msgId = e.data?.msgId != null ? `msg ${e.data.msgId}` : '';
                return (
                  <tr key={i}>
                    <td style={{ padding: '8px 12px', color: C.textMuted, fontFamily: T.typography.fontMono }}>
                      {new Date(e.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </td>
                    <td style={{ padding: '8px 12px', color: isPos ? C.green : C.red, fontWeight: T.typography.weightBold }}>
                      {isPos ? 'Positive' : 'Negative'}
                    </td>
                    <td style={{ padding: '8px 12px', color: C.text }}>{category}</td>
                    <td style={{ padding: '8px 12px', color: C.textMuted, fontFamily: T.typography.fontMono }}>{msgId}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

// c2-323 / c0-035 #3: sources row in Library pulls /api/library/sources
// (reports Claude 0's 360 sources). Prefer :3002 (analytics split service)
// with :3000 fallback. Cached per-mount since sources rarely change — no
// auto-refresh.
interface SourceRow { url?: string; name?: string; domain?: string; trust?: number; facts?: number }

const LibraryTab: React.FC<{ C: any; host: string; domains: Array<{ domain: string; count: number }>; files: Array<{ file: string; pairs: number; size_mb: number }> }> = ({ C, host, domains, files }) => {
  const [q, setQ] = React.useState('');
  const [sources, setSources] = React.useState<SourceRow[] | null>(null);
  const [sourcesErr, setSourcesErr] = React.useState<string | null>(null);
  React.useEffect(() => {
    const ctrl = new AbortController();
    const to = setTimeout(() => ctrl.abort(), 6000);
    const tryFetch = async (port: number) => {
      const r = await fetch(`http://${host}:${port}/api/library/sources`, { signal: ctrl.signal });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return r.json();
    };
    (async () => {
      try {
        let data: any;
        try { data = await tryFetch(3002); }
        catch { data = await tryFetch(3000); }
        const arr: SourceRow[] = Array.isArray(data?.sources) ? data.sources : Array.isArray(data) ? data : [];
        setSources(arr);
      } catch (e: any) {
        setSourcesErr(String(e?.message || e || 'fetch failed'));
      } finally { clearTimeout(to); }
    })();
    return () => { clearTimeout(to); ctrl.abort(); };
  }, [host]);
  const normQ = q.trim().toLowerCase();
  const matchedDomains = normQ ? domains.filter(d => d.domain.toLowerCase().includes(normQ)) : domains;
  const matchedFiles = normQ ? files.filter(f => f.file.toLowerCase().includes(normQ)) : files;
  const matchedSources = !sources ? [] : (normQ
    ? sources.filter(s =>
        (s.url || '').toLowerCase().includes(normQ) ||
        (s.name || '').toLowerCase().includes(normQ) ||
        (s.domain || '').toLowerCase().includes(normQ))
    : sources);
  return (
    <div>
      <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Library</h2>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Browse what the AI has learned — sources the knowledge was drawn from, the domains they map to, and the training files generated.
      </p>
      <input
        type='search' value={q} onChange={e => setQ(e.target.value)}
        onKeyDown={(e) => { if (e.key === 'Escape' && q) { e.preventDefault(); setQ(''); } }}
        autoComplete='off' spellCheck={false}
        placeholder={`Filter ${domains.length} domains / ${files.length} files${sources ? ` / ${sources.length} sources` : ''}…`}
        aria-label='Library search'
        style={{
          width: '100%', padding: '10px 12px', marginBottom: T.spacing.lg,
          background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
          borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
          fontSize: T.typography.sizeBody, outline: 'none',
        }}
      />
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: T.spacing.lg }}>
        <div>
          <Label color={C.textMuted} mb={T.spacing.md}>
            Domains ({matchedDomains.length})
          </Label>
          {matchedDomains.length === 0 ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.lg, textAlign: 'center' }}>No domains match.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.xs }}>
              {matchedDomains.slice(0, 50).map(d => (
                <div key={d.domain} style={{
                  display: 'flex', justifyContent: 'space-between',
                  padding: '8px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
                  fontSize: T.typography.sizeSm,
                }}>
                  <span style={{ color: C.text }}>{d.domain}</span>
                  <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{d.count.toLocaleString()}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div>
          <Label color={C.textMuted} mb={T.spacing.md}>
            Training files ({matchedFiles.length})
          </Label>
          {matchedFiles.length === 0 ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.lg, textAlign: 'center' }}>No files match.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.xs }}>
              {matchedFiles.slice(0, 50).map(f => (
                <div key={f.file} style={{
                  display: 'flex', justifyContent: 'space-between',
                  padding: '8px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
                  fontSize: T.typography.sizeSm,
                }}>
                  <span style={{ color: C.text, fontFamily: T.typography.fontMono }}>{f.file}</span>
                  <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{f.pairs.toLocaleString()} pairs</span>
                </div>
              ))}
            </div>
          )}
        </div>
        {/* c2-323 / c0-035 #3: sources inventory. Renders once the fetch
            resolves — loading and error states are explicit so users know
            whether the backend has the endpoint up. */}
        <div>
          <Label color={C.textMuted} mb={T.spacing.md}>
            Sources ({sources ? matchedSources.length : '…'})
          </Label>
          {sourcesErr ? (
            <div style={{ fontSize: T.typography.sizeSm, color: C.red, padding: '10px 12px', background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: T.radii.md }}>
              Sources unavailable: {sourcesErr}
            </div>
          ) : !sources ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.lg, textAlign: 'center' }} aria-busy='true'>Loading sources…</div>
          ) : matchedSources.length === 0 ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.lg, textAlign: 'center' }}>No sources match.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.xs, maxHeight: '420px', overflowY: 'auto' }}>
              {matchedSources.slice(0, 400).map((s, i) => {
                const label = s.name || s.url || s.domain || `(source ${i + 1})`;
                const tail = typeof s.facts === 'number' ? `${s.facts.toLocaleString()} facts`
                  : typeof s.trust === 'number' ? `trust ${(s.trust * 100).toFixed(0)}%`
                  : '';
                return (
                  <div key={`${label}-${i}`} style={{
                    display: 'flex', justifyContent: 'space-between', gap: T.spacing.sm,
                    padding: '8px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
                    fontSize: T.typography.sizeSm,
                  }}>
                    <span style={{ color: C.text, fontFamily: T.typography.fontMono, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', minWidth: 0, flex: 1 }}
                      title={s.url || label}>{label}</span>
                    {tail && <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono, flexShrink: 0 }}>{tail}</span>}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const Placeholder: React.FC<{ C: any; title: string; body: string; data: unknown }> = ({ C, title, body, data }) => (
  <div>
    <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: '0 0 12px' }}>{title}</h2>
    <div style={{
      padding: T.spacing.xl, background: C.bgCard,
      border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
      fontSize: T.typography.sizeBody, color: C.textSecondary, lineHeight: 1.6,
    }}>
      {body}
      {data !== null && (
        <pre style={{
          marginTop: T.spacing.md, padding: T.spacing.md, background: C.bgInput,
          border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
          fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: T.typography.sizeSm,
          color: C.textMuted, whiteSpace: 'pre-wrap', overflowX: 'auto', maxHeight: '240px',
        }}>{JSON.stringify(data, null, 2)}</pre>
      )}
    </div>
  </div>
);
 