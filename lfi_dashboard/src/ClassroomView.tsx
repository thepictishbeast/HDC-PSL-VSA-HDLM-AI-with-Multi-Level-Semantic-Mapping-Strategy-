import React, { useEffect, useRef, useState, useMemo } from 'react';
import { T } from './tokens';
// c2-343: 18/22/26px heading sizes need the design-system scale since
// T.typography caps at 22; sourced cross-platform so desktop/Android match.
import { typography as dsType } from './design-system';
// c2-346 / task 24: shared uppercase meta-label component.
import { Label } from './components/Label';
import { StatCard } from './components/StatCard';
// c2-348 / task 28: shared error banner.
import { ErrorAlert } from './components/ErrorAlert';
// c2-349 / task 29: shared shimmer skeleton.
import { SkeletonLoader } from './components/SkeletonLoader';
// c2-350 / task 27: shared horizontal progress bar.
import { BarChart } from './components/BarChart';
// c2-351 / task 30: shared WAI-ARIA tablist.
import { TabBar } from './components/TabBar';
// c2-379 / BIG #180: shared sortable table.
import { DataTable } from './components';
import type { Column } from './components';
import { compactNum, formatRelative, exportGradeReportPdf, formatDayBucket } from './util';

// ClassroomView — full page (not modal) per c0-027. The "school" metaphor:
// the AI is the student, training data is the curriculum, evaluation
// results are the gradebook. Eight sub-sections; for now all draw from
// /api/admin/dashboard until the classroom-specific endpoints land.

type Sub = 'profile' | 'control' | 'curriculum' | 'gradebook' | 'lessons' | 'tests' | 'reports' | 'office' | 'library' | 'ledger' | 'drift' | 'runs';

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
  // c2-433 / #317 reuse: parent-provided opener for the shared fact
  // popover. The Ledger tab uses this to make fact_keys clickable so a
  // contradiction row can be inspected without leaving the Classroom.
  onOpenFactKey?: (key: string, rect: DOMRect) => void;
  // c2-433 / Cmd+K deep-link. Pass {sub: 'ledger' | 'drift' | 'runs' | ...,
  // tick: nonce} and the effect below will switch sub. The tick nonce
  // lets re-clicking the same entry re-activate the sub even if the user
  // manually navigated away in-between.
  initialSub?: { sub: string; tick: number } | null;
}

const SUBS: Array<{ id: Sub; label: string; hint: string }> = [
  { id: 'profile',    label: 'Student Profile', hint: 'Grade, strengths, weaknesses' },
  // c2-426 / c2-428 (#339 pivot): Ingestion Control. Previously framed as
  // "Training Control" — corrected per LFI_SUPERSOCIETY_ARCHITECTURE.md,
  // LFI is post-LLM (HDC/VSA/PSL/HDLM), there is no training loop, only
  // fact-ingestion batches.
  { id: 'control',    label: 'Ingestion Control', hint: 'Start / stop / corpus runs' },
  { id: 'curriculum', label: 'Curriculum',      hint: 'Training datasets + sizes' },
  { id: 'gradebook',  label: 'Gradebook',       hint: 'Pass/fail + trends' },
  { id: 'lessons',    label: 'Lesson Plans',    hint: 'Active training sessions' },
  { id: 'tests',      label: 'Test Center',     hint: 'Benchmarks + quizzes' },
  { id: 'reports',    label: 'Report Cards',    hint: 'Weekly progress' },
  { id: 'office',     label: 'Office Hours',    hint: 'Feedback review' },
  { id: 'library',    label: 'Library',         hint: 'Fact browser' },
  // c2-433 / #298 followup: Ledger tab. Surfaces the contradictions queue
  // the Classroom badge counts. Pending = upsert_fact disagreements where
  // both sides are ≥ 0.7 confidence.
  { id: 'ledger',     label: 'Ledger',          hint: 'Contradiction queue' },
  // c2-433 / #284: Drift tab. One-call /api/drift/snapshot bundle with 6
  // system-health metrics, session-ring history for trend sparklines.
  { id: 'drift',      label: 'Drift',           hint: 'System health trends' },
  // c2-433 / #312: Runs tab. Ingest history from /api/ingest/list, shows
  // running-first then recent finished runs.
  { id: 'runs',       label: 'Runs',            hint: 'Ingest run history' },
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
const CLASSROOM_SUBS: readonly Sub[] = ['profile','control','curriculum','gradebook','lessons','tests','reports','office','library','ledger','drift','runs'];

export const ClassroomView: React.FC<ClassroomViewProps> = ({ C, host, isDesktop, localEvents = [], onOpenFactKey, initialSub }) => {
  // c2-433 mobile-fix: compact paddings/tabs when not on desktop. Tablet
  // falls in between — treat anything sub-desktop as mobile for density,
  // the TabBar scrolls horizontally either way.
  const isMobile = !isDesktop;
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
  // c2-433 / Cmd+K deep-link: sync sub when parent updates initialSub
  // (with a new tick). The tick is what makes repeat-activation work —
  // clicking the same palette entry twice re-fires even if the user
  // clicked a different tab in-between.
  const lastInitialSubTickRef = useRef<number>(-1);
  useEffect(() => {
    if (!initialSub) return;
    if (initialSub.tick === lastInitialSubTickRef.current) return;
    lastInitialSubTickRef.current = initialSub.tick;
    if (CLASSROOM_SUBS.includes(initialSub.sub as Sub)) {
      setSub(initialSub.sub as Sub);
    }
  }, [initialSub]);
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
      // CSP connect-src = 'self' only, so :3002 fetches throw before
      // reaching the network. Route to the main backend path instead.
      const [ovRes, domRes] = await Promise.all([
        fetch(`http://${host}:3000/api/analytics/overview`, { signal: ctrl.signal }),
        fetch(`http://${host}:3000/api/analytics/domains`, { signal: ctrl.signal }),
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
    const liveTabs: Sub[] = ['profile', 'control', 'curriculum', 'gradebook', 'lessons', 'reports'];
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
        padding={isMobile ? '0 12px' : '0 24px'}
        compact={isMobile}
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
            {/* c2-421 / task 203: export the current grade report as a PDF.
                Uses the already-loaded data — no extra fetch. Hidden when
                data hasn't arrived yet so users don't get an empty PDF. */}
            {data && (
              <button onClick={() => exportGradeReportPdf(data as any)}
                aria-label='Export grade report as PDF'
                title='Export grade report as PDF'
                style={{
                  alignSelf: 'center', background: 'transparent',
                  border: `1px solid ${C.borderSubtle}`, color: C.textMuted,
                  borderRadius: T.radii.sm, cursor: 'pointer',
                  padding: '4px 10px', marginRight: T.spacing.xs,
                  fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                  textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                  fontFamily: 'inherit',
                }}>PDF</button>
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
        style={{ flex: 1, overflowY: 'auto', padding: isMobile ? T.spacing.md : T.spacing.xl, maxWidth: '1200px', width: '100%', margin: '0 auto' }}>
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

        {/* --- Training Control (c2-426) --- */}
        {sub === 'control' && (
          <TrainingControlPanel C={C} host={host} isDesktop={isDesktop} dashboardData={data} onDataRefresh={load} />
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
              // c2-379 / BIG #180: Curriculum table -> DataTable. Sort
              // state stays lifted (curricSort) so the existing keyboard-
              // shortcut hook + URL intent remain compatible. Filter is
              // applied upstream (`filtered`) since it lives in the input
              // above -- DataTable only sees already-filtered rows.
              type FRow = { file: string; pairs: number; size_mb: number };
              const cols: ReadonlyArray<Column<FRow>> = [
                {
                  id: 'file', header: 'Dataset', align: 'left',
                  sortKey: (f) => f.file.toLowerCase(),
                  accessor: (f) => <span style={{ fontFamily: T.typography.fontMono, color: C.text }}>{f.file}</span>,
                },
                {
                  id: 'pairs', header: 'Pairs', align: 'right',
                  sortKey: (f) => f.pairs,
                  accessor: (f) => (
                    <span style={{ fontFamily: T.typography.fontMono, color: C.accent, fontWeight: T.typography.weightBold }}>
                      {f.pairs.toLocaleString()}
                    </span>
                  ),
                },
                {
                  id: 'size', header: 'Size', align: 'right',
                  sortKey: (f) => f.size_mb,
                  accessor: (f) => <span style={{ fontFamily: T.typography.fontMono, color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</span>,
                },
              ];
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
                  <DataTable<FRow> C={C}
                    rows={filtered as FRow[]}
                    columns={cols}
                    rowKey={(f) => f.file}
                    sort={{ col: curricSort.col === 'size' ? 'size' : curricSort.col, dir: curricSort.dir }}
                    onSortChange={(next) => setCurricSort({ col: next.col as 'file' | 'pairs' | 'size', dir: next.dir })}
                    emptyText={q ? `No datasets match "${curricFilter}"` : 'No training files reported.'}
                    cellFontSize={T.typography.sizeMd} />
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
          <LessonsTab C={C} host={host} training={data?.training} files={data?.training_files || []} />
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
          <OfficeHoursTab C={C} host={host} events={localEvents} />
        )}

        {/* --- Library --- */}
        {sub === 'library' && (
          <LibraryTab C={C} host={host} domains={sortedDomains} files={data?.training_files || []} />
        )}

        {/* --- Ledger (contradictions queue) --- */}
        {sub === 'ledger' && (
          <LedgerTab C={C} host={host} onOpenFactKey={onOpenFactKey} />
        )}

        {/* --- Drift (system-health trend) --- */}
        {sub === 'drift' && (
          <DriftTab C={C} host={host} onJumpTo={(s) => setSub(s as Sub)} />
        )}

        {/* --- Runs (ingest history #312) --- */}
        {sub === 'runs' && (
          <IngestRunsTab C={C} host={host} />
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

// c2-426: dedicated Training Control panel. Surfaces trainer status +
// Start/Stop + per-domain sessions + recent-cycle pulse, all in one place
// so the user doesn't need to jump between /training modal and the
// Admin Training tab. Polls the 3 training endpoints on mount + every 5s
// while visible so progress feels live.
const TrainingControlPanel: React.FC<{
  C: any; host: string; isDesktop: boolean;
  dashboardData: DashboardShape | null;
  onDataRefresh: () => void;
}> = ({ C, host, dashboardData, onDataRefresh }) => {
  const [accuracy, setAccuracy] = useState<any | null>(null);
  const [sessions, setSessions] = useState<any | null>(null);
  const [busy, setBusy] = useState<null | 'start' | 'stop'>(null);
  const [toastMsg, setToastMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [nowTick, setNowTick] = useState(0);
  // c2-428 / #339 pivot: model_tier picker + lfi_training_tier LS key
  // REMOVED. LFI is post-LLM — no model selection applies. Ingestion
  // parameters (corpus, decomposer, priority) will live here once the
  // /api/ingest/* routes land.

  const fetchWithTimeout = async <T,>(path: string, ms = 8000): Promise<T> => {
    const ctrl = new AbortController();
    const to = setTimeout(() => ctrl.abort(), ms);
    try {
      const r = await fetch(`http://${host}:3000${path}`, { signal: ctrl.signal });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return (await r.json()) as T;
    } finally { clearTimeout(to); }
  };
  const refresh = async () => {
    const [a, s] = await Promise.allSettled([
      fetchWithTimeout<any>('/api/admin/training/accuracy'),
      fetchWithTimeout<any>('/api/admin/training/sessions'),
    ]);
    if (a.status === 'fulfilled') setAccuracy(a.value);
    if (s.status === 'fulfilled') setSessions(s.value);
    onDataRefresh();
  };
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [host]);
  // Tick for "last trained X ago" freshness without re-fetching.
  useEffect(() => {
    const id = setInterval(() => setNowTick(t => t + 1), 1000);
    return () => clearInterval(id);
  }, []);

  const control = async (action: 'start' | 'stop') => {
    setBusy(action);
    setToastMsg(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      // c2-428 / #339 pivot: start body carries no params for now — the
      // /api/ingest/start route when it lands will take {corpus, decomposer,
      // priority}. Legacy /api/admin/training/:action endpoint used here
      // until the ingest routes ship.
      const r = await fetch(`http://${host}:3000/api/admin/training/${action}`, {
        method: 'POST', signal: ctrl.signal,
      });
      clearTimeout(to);
      const respBody = await r.json().catch(() => ({}));
      if (!r.ok) throw new Error(respBody?.message || `HTTP ${r.status}`);
      setToastMsg({
        ok: true,
        text: respBody?.message || (action === 'start'
          ? 'Ingestion run started'
          : 'Ingestion stop requested'),
      });
      setTimeout(refresh, 500);
    } catch (e: any) {
      setToastMsg({ ok: false, text: `Could not ${action}: ${e?.message || e}` });
    } finally {
      setBusy(null);
    }
  };

  const trainingState = sessions?.training_state || {};
  const domainStateEntries: [string, any][] = Object.entries(trainingState);
  const nowSec = Date.now() / 1000;
  const anyRecentlyTrained = domainStateEntries.some(([, st]: any) => st?.last_trained && (nowSec - Number(st.last_trained)) < 300);
  const trainerActive = !!(sessions?.trainer_running) || anyRecentlyTrained;

  // Parse the most recent log line for the pulse indicator.
  const lastCycle = (() => {
    const log: string[] | undefined = accuracy?.recent_training_log;
    if (!Array.isArray(log) || log.length === 0) return null;
    for (let i = log.length - 1; i >= 0; i--) {
      const m = log[i].match(/^\[([^\]]+)\] cycle=(\d+) domain=(\w+) (.+)$/);
      if (m) {
        const when = Date.parse(m[1]);
        if (!Number.isNaN(when)) {
          const tail = m[4].trim();
          const state = tail.startsWith('batch=') ? 'in progress' : tail === 'done' ? 'done' : tail;
          return { ts: when, ageSec: Math.max(0, Math.floor((Date.now() - when) / 1000)), cycle: m[2], domain: m[3], state };
        }
      }
    }
    return null;
  })();
  void nowTick; // force re-eval of ageSec each tick

  const totalPairs = dashboardData?.overview?.total_training_pairs ?? (dashboardData?.training_files || []).reduce((s, f) => s + f.pairs, 0);
  const totalFacts = dashboardData?.overview?.total_facts;
  const learningSignals = accuracy?.learning_signals;
  const passRatePct = pctNorm(accuracy?.psl_calibration?.pass_rate) ?? pctNorm(accuracy?.pass_rate);

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, marginBottom: T.spacing.md, flexWrap: 'wrap' }}>
        <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: 0 }}>Ingestion Control</h2>
        <span aria-live='polite' style={{
          display: 'inline-flex', alignItems: 'center', gap: '6px',
          padding: `4px ${T.spacing.sm}`,
          background: trainerActive ? C.greenBg : C.bgInput,
          border: `1px solid ${trainerActive ? C.greenBorder : C.borderSubtle}`,
          color: trainerActive ? C.green : C.textMuted,
          borderRadius: T.radii.sm,
          fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
          textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
        }}>
          <span style={{
            width: '8px', height: '8px', borderRadius: '50%',
            background: trainerActive ? C.green : C.textDim,
            boxShadow: trainerActive ? `0 0 6px ${C.green}` : 'none',
          }} />
          {trainerActive ? 'Ingestion active' : 'Idle'}
        </span>
      </div>
      {/* Top-level start/stop controls */}
      <div style={{
        display: 'flex', gap: T.spacing.sm, marginBottom: T.spacing.lg, flexWrap: 'wrap',
        alignItems: 'center',
      }}>
        {/* c2-428 / #339 pivot: model_tier dropdown removed. LFI is post-LLM;
            there is no model to pick. Ingestion parameters (corpus,
            decomposer, priority, etc.) will render here once /api/ingest/*
            is live. */}
        <button onClick={() => control('start')}
          disabled={busy !== null || trainerActive}
          aria-label='Start ingestion run'
          style={{
            padding: `${T.spacing.sm} ${T.spacing.lg}`,
            fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBold,
            background: trainerActive ? C.bgInput : C.greenBg,
            border: `1px solid ${trainerActive ? C.borderSubtle : C.greenBorder}`,
            color: trainerActive ? C.textDim : C.green,
            borderRadius: T.radii.md,
            cursor: (busy !== null || trainerActive) ? 'not-allowed' : 'pointer',
            fontFamily: 'inherit',
            opacity: busy === 'start' ? 0.6 : 1,
          }}>{busy === 'start' ? 'Starting…' : 'Start ingestion'}</button>
        <button onClick={() => control('stop')}
          disabled={busy !== null || !trainerActive}
          aria-label='Stop ingestion run'
          style={{
            padding: `${T.spacing.sm} ${T.spacing.lg}`,
            fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBold,
            background: !trainerActive ? 'transparent' : C.redBg,
            border: `1px solid ${!trainerActive ? C.borderSubtle : C.redBorder}`,
            color: !trainerActive ? C.textDim : C.red,
            borderRadius: T.radii.md,
            cursor: (busy !== null || !trainerActive) ? 'not-allowed' : 'pointer',
            fontFamily: 'inherit',
            opacity: busy === 'stop' ? 0.6 : 1,
          }}>{busy === 'stop' ? 'Stopping…' : 'Stop ingestion'}</button>
        <button onClick={refresh} disabled={busy !== null}
          aria-label='Refresh ingestion status'
          style={{
            padding: `${T.spacing.sm} ${T.spacing.lg}`,
            fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBold,
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer',
            fontFamily: 'inherit',
          }}>Refresh</button>
      </div>
      {toastMsg && (
        <div role={toastMsg.ok ? 'status' : 'alert'} style={{
          marginBottom: T.spacing.md, padding: `${T.spacing.sm} ${T.spacing.md}`,
          background: toastMsg.ok ? C.greenBg : C.redBg,
          border: `1px solid ${toastMsg.ok ? C.greenBorder : C.redBorder}`,
          color: toastMsg.ok ? C.green : C.red,
          borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
        }}>{toastMsg.text}</div>
      )}
      {/* Most recent cycle pulse */}
      {lastCycle && (
        <div style={{
          padding: `${T.spacing.sm} ${T.spacing.md}`, marginBottom: T.spacing.lg,
          background: lastCycle.ageSec < 300 ? C.greenBg : C.bgInput,
          border: `1px solid ${lastCycle.ageSec < 300 ? C.greenBorder : C.borderSubtle}`,
          borderRadius: T.radii.md,
          display: 'flex', alignItems: 'center', gap: T.spacing.sm, fontSize: T.typography.sizeSm, flexWrap: 'wrap',
        }}>
          <span style={{
            display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%',
            background: lastCycle.ageSec < 300 ? C.green : C.textDim,
          }} />
          <span style={{ color: C.textMuted, fontWeight: T.typography.weightSemibold }}>Most recent cycle</span>
          <span style={{ color: C.text, fontFamily: T.typography.fontMono }}>#{lastCycle.cycle}</span>
          <span style={{ color: C.accent, fontWeight: T.typography.weightSemibold }}>{lastCycle.domain}</span>
          <span style={{ color: C.textMuted }}>{lastCycle.state}</span>
          <span style={{ marginLeft: 'auto', color: C.textDim, fontSize: T.typography.sizeXs }}>{formatRelative(lastCycle.ts)}</span>
        </div>
      )}
      {/* Summary stats */}
      <div style={{
        display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: T.spacing.sm, marginBottom: T.spacing.lg,
      }}>
        <StatCard C={C} label='Facts' value={typeof totalFacts === 'number' ? compactNum(totalFacts) : '—'} color={C.accent} />
        <StatCard C={C} label='Training pairs' value={totalPairs ? compactNum(totalPairs) : '—'} color={C.green} />
        <StatCard C={C} label='Domains' value={String(dashboardData?.domains?.length ?? domainStateEntries.length ?? 0)} color={C.purple} />
        <StatCard C={C} label='Pass rate' value={passRatePct != null ? `${passRatePct.toFixed(1)}%` : '—'} color={passRatePct != null && passRatePct >= 95 ? C.green : passRatePct != null && passRatePct >= 85 ? C.yellow : C.red} />
        <StatCard C={C} label='Learning signals' value={typeof learningSignals === 'number' ? compactNum(learningSignals) : '—'} color={C.green} />
        <StatCard C={C} label='Sessions' value={sessions?.training_state ? String(Object.values(trainingState).reduce((s: number, st: any) => s + (st?.sessions ?? 0), 0)) : '—'} color={C.accent} />
      </div>
      {/* Per-domain training status + rotator */}
      {domainStateEntries.length > 0 && (
        <div>
          <div style={{
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
            color: C.textMuted, textTransform: 'uppercase',
            letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.sm,
          }}>Per-domain ingestion state ({domainStateEntries.length})</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {domainStateEntries
              .sort(([, a]: any, [, b]: any) => (Number(b?.last_trained ?? 0)) - (Number(a?.last_trained ?? 0)))
              .map(([dom, st]: any) => {
                const last = Number(st?.last_trained ?? 0);
                const ageSec = last ? nowSec - last : null;
                const recent = ageSec != null && ageSec < 300;
                const sessionsN = st?.sessions ?? 0;
                return (
                  <div key={dom} style={{
                    display: 'flex', alignItems: 'center', gap: T.spacing.sm,
                    padding: `${T.spacing.sm} ${T.spacing.md}`,
                    background: recent ? C.greenBg : C.bgInput,
                    border: `1px solid ${recent ? C.greenBorder : C.borderSubtle}`,
                    borderRadius: T.radii.md,
                    fontSize: T.typography.sizeSm,
                  }}>
                    <span style={{
                      display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%',
                      background: recent ? C.green : C.textDim,
                    }} />
                    <span style={{ color: C.text, fontWeight: T.typography.weightSemibold, flex: '0 0 140px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{dom}</span>
                    <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono, flex: '0 0 90px' }}>{sessionsN} session{sessionsN === 1 ? '' : 's'}</span>
                    <span style={{ flex: 1 }} />
                    {recent && <span style={{ fontSize: T.typography.sizeXs, color: C.green, fontWeight: T.typography.weightBold, textTransform: 'uppercase' }}>LIVE</span>}
                    <span style={{ color: C.textDim, fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono }}>
                      {last ? `${formatRelative(last * 1000)}` : 'never'}
                    </span>
                  </div>
                );
              })}
          </div>
        </div>
      )}
      {/* Recent ingestion log tail */}
      {Array.isArray(accuracy?.recent_training_log) && accuracy.recent_training_log.length > 0 && (
        <div style={{ marginTop: T.spacing.lg }}>
          <div style={{
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
            color: C.textMuted, textTransform: 'uppercase',
            letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.sm,
          }}>Recent ingestion log (last 40)</div>
          <pre style={{
            padding: T.spacing.md, background: C.bgInput, borderRadius: T.radii.md,
            fontSize: T.typography.sizeXs, color: C.textSecondary,
            fontFamily: T.typography.fontMono,
            whiteSpace: 'pre-wrap', maxHeight: '260px', overflowY: 'auto',
            margin: 0, border: `1px solid ${C.borderSubtle}`,
          }}>{accuracy.recent_training_log.slice(-40).join('\n')}</pre>
        </div>
      )}
      {/* c0-ask-3 / #406 + #407: quick-ingest surface for paste text +
          URL fetch. Two small forms posting to /api/ingest/paste and
          /api/ingest/url respectively. Response shape is shared:
          { ok, ingested_count, skipped, title, source, tag,
            secrets_scrubbed, sample_keys[] }. Toast shows
          "Ingested N facts from X". Secrets scrubbed server-side; 200KB
          paste cap; http/https only for URL with 15s fetch timeout. */}
      <QuickIngestCard C={C} host={host} />
      {/* c2-428 / #339 pivot placeholder — /api/ingest/* routes land here
          once Claude 0 ships them. Surface will expose: corpus selection,
          decomposer choice (HDC / PSL / tuple-extraction pipeline),
          priority weight per domain, batch size, pause/resume. Metrics
          rendered below will be PSL axiom pass rate, tuple
          well-formedness, contradiction flags, provenance coverage — not
          LLM-style accuracy / loss / perplexity. */}
      <div style={{
        marginTop: T.spacing.xl, padding: T.spacing.md,
        background: C.bgInput, border: `1px dashed ${C.borderSubtle}`,
        borderRadius: T.radii.md, color: C.textDim, fontSize: T.typography.sizeXs,
      }}>
        <strong style={{ color: C.textMuted }}>Configuration</strong> — corpus selection, decomposer choice, per-domain priority, and batch-size knobs land here once <code style={{ fontFamily: T.typography.fontMono, color: C.accent }}>/api/ingest/*</code> is live. Tracked in the task queue.
      </div>
    </div>
  );
};

// c0-ask-3 / #406 + #407: paste + URL quick-ingest. Two tiny POST forms
// sharing a response-toast surface. Self-contained so the Classroom
// ingestion tab stays readable.
const QuickIngestCard: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [mode, setMode] = useState<'paste' | 'url'>('paste');
  const [pasteTitle, setPasteTitle] = useState('');
  const [pasteBody, setPasteBody] = useState('');
  const [pasteSource, setPasteSource] = useState('paste');
  const [pasteTag, setPasteTag] = useState('');
  const [urlValue, setUrlValue] = useState('');
  const [urlTag, setUrlTag] = useState('');
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<{ ok: boolean; text: string } | null>(null);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 7000);
    return () => clearTimeout(t);
  }, [toast]);

  const submit = async () => {
    setBusy(true);
    setToast(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 20000);
      let res: Response;
      if (mode === 'paste') {
        if (!pasteBody.trim()) throw new Error('Paste body is empty');
        res = await fetch(`http://${host}:3000/api/ingest/paste`, {
          method: 'POST', signal: ctrl.signal,
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            title: pasteTitle.trim() || 'Pasted text',
            body: pasteBody,
            source: pasteSource.trim() || 'paste',
            tag: pasteTag.trim() || undefined,
          }),
        });
      } else {
        if (!/^https?:\/\//i.test(urlValue.trim())) throw new Error('URL must start with http:// or https://');
        res = await fetch(`http://${host}:3000/api/ingest/url`, {
          method: 'POST', signal: ctrl.signal,
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            url: urlValue.trim(),
            tag: urlTag.trim() || undefined,
          }),
        });
      }
      clearTimeout(to);
      const j: any = await res.json().catch(() => ({}));
      if (!res.ok || j?.ok === false) {
        throw new Error(j?.error || j?.message || `HTTP ${res.status}`);
      }
      const count = j?.ingested_count ?? 0;
      const title = j?.title || (mode === 'paste' ? pasteTitle || 'paste' : urlValue);
      setToast({ ok: true, text: `Ingested ${count} fact${count === 1 ? '' : 's'} from "${title}"${j?.skipped ? ` (${j.skipped} skipped)` : ''}` });
      if (mode === 'paste') { setPasteTitle(''); setPasteBody(''); setPasteTag(''); }
      else { setUrlValue(''); setUrlTag(''); }
    } catch (e: any) {
      setToast({ ok: false, text: `Ingest failed: ${e?.message || e}` });
    } finally {
      setBusy(false);
    }
  };

  const tabBtn = (k: 'paste' | 'url', label: string): React.CSSProperties => ({
    padding: `${T.spacing.xs} ${T.spacing.md}`,
    background: mode === k ? C.accent : 'transparent',
    color: mode === k ? C.bg : C.textSecondary,
    border: `1px solid ${mode === k ? C.accent : C.borderSubtle}`,
    borderRadius: T.radii.md,
    fontSize: T.typography.sizeSm,
    fontWeight: 500,
    cursor: 'pointer',
    fontFamily: 'inherit',
  });
  const inputStyle: React.CSSProperties = {
    width: '100%', padding: T.spacing.sm,
    background: C.bgCard, color: C.text,
    border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
    fontSize: T.typography.sizeSm, fontFamily: 'inherit',
    boxSizing: 'border-box',
  };

  return (
    <div style={{
      marginTop: T.spacing.xl, padding: T.spacing.md,
      background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
      borderRadius: T.radii.md,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, marginBottom: T.spacing.sm }}>
        <strong style={{ color: C.text, fontSize: T.typography.sizeMd }}>Quick ingest</strong>
        <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>— paste text or fetch a URL into the fact base</span>
      </div>
      <div style={{ display: 'flex', gap: T.spacing.xs, marginBottom: T.spacing.sm }}>
        <button type="button" style={tabBtn('paste', 'Paste text')} onClick={() => setMode('paste')} aria-pressed={mode === 'paste'}>Paste text</button>
        <button type="button" style={tabBtn('url', 'From URL')} onClick={() => setMode('url')} aria-pressed={mode === 'url'}>From URL</button>
      </div>
      {mode === 'paste' ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.xs }}>
          <input
            style={inputStyle} type="text" placeholder="Title (optional)"
            value={pasteTitle} onChange={e => setPasteTitle(e.target.value)}
            aria-label="Paste title"
          />
          <textarea
            style={{ ...inputStyle, minHeight: '120px', fontFamily: T.typography.fontMono, resize: 'vertical' }}
            placeholder="Paste text here — each sentence becomes a fact at 0.75 confidence. Up to 200KB; 10–1000 chars per sentence; 2000 sentences max."
            value={pasteBody} onChange={e => setPasteBody(e.target.value)}
            aria-label="Paste body"
          />
          <div style={{ display: 'flex', gap: T.spacing.xs }}>
            <input
              style={{ ...inputStyle, flex: 1 }} type="text" placeholder="Source tag (default: paste)"
              value={pasteSource} onChange={e => setPasteSource(e.target.value)}
              aria-label="Paste source tag"
            />
            <input
              style={{ ...inputStyle, flex: 1 }} type="text" placeholder="Topic tag (optional)"
              value={pasteTag} onChange={e => setPasteTag(e.target.value)}
              aria-label="Paste topic tag"
            />
          </div>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: T.spacing.xs }}>
          <input
            style={inputStyle} type="url" placeholder="https://example.com/page"
            value={urlValue} onChange={e => setUrlValue(e.target.value)}
            aria-label="URL to fetch"
          />
          <input
            style={inputStyle} type="text" placeholder="Topic tag (optional)"
            value={urlTag} onChange={e => setUrlTag(e.target.value)}
            aria-label="URL topic tag"
          />
          <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>
            Backend strips scripts/styles + decodes entities, then routes the text through the paste pipeline. Scheme-gated to http/https with a 15s fetch timeout + 10/60s rate limit.
          </span>
        </div>
      )}
      <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center', marginTop: T.spacing.sm }}>
        <button
          type="button"
          aria-label={`Submit ${mode === 'paste' ? 'paste' : 'URL'} ingest`}
          onClick={submit}
          disabled={busy || (mode === 'paste' ? !pasteBody.trim() : !urlValue.trim())}
          style={{
            padding: `${T.spacing.sm} ${T.spacing.lg}`,
            background: C.accent, color: C.bg, border: 'none',
            borderRadius: T.radii.md, fontWeight: 600,
            fontSize: T.typography.sizeSm, cursor: busy ? 'wait' : 'pointer',
            opacity: busy ? 0.7 : 1, fontFamily: 'inherit',
          }}
        >{busy ? 'Ingesting…' : (mode === 'paste' ? 'Ingest paste' : 'Fetch + ingest')}</button>
        {toast && (
          <span style={{
            fontSize: T.typography.sizeXs,
            color: toast.ok ? C.green : C.red,
            fontWeight: 500,
          }}>{toast.text}</span>
        )}
      </div>
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

// c0-ask-4 / #405: rollup of external trainer sessions. Backend ships
// GET /api/trainer/sessions with {sessions:[{trainer, session_id, turns,
// up_count, down_count, correction_count, last_activity_ts}]}.
// Card polls every 10s and renders a DataTable with relative-time
// "last active" + up/down percentages. Empty state prompts the user to
// run scripts/gemini-trainer.sh.
interface TrainerSessionRow {
  trainer: string;
  session_id: string;
  turns: number;
  up_count?: number;
  down_count?: number;
  correction_count?: number;
  last_activity_ts?: number;       // epoch seconds
  last_activity?: string;          // ISO fallback
}
const TrainerSessionsCard: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [rows, setRows] = useState<TrainerSessionRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [nowTick, setNowTick] = useState(0);
  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const ctrl = new AbortController();
        const to = setTimeout(() => ctrl.abort(), 8000);
        const r = await fetch(`http://${host}:3000/api/trainer/sessions`, { signal: ctrl.signal });
        clearTimeout(to);
        if (!r.ok) {
          if (!cancelled) { setErr(`HTTP ${r.status}`); setRows([]); }
          return;
        }
        const j: any = await r.json().catch(() => ({}));
        if (cancelled) return;
        const list: TrainerSessionRow[] = Array.isArray(j?.sessions) ? j.sessions : Array.isArray(j) ? j : [];
        setRows(list);
        setErr(null);
      } catch (e: any) {
        if (!cancelled) { setErr(e?.message || 'unreachable'); setRows([]); }
      }
    };
    load();
    const id = setInterval(load, 10000);
    const tickId = setInterval(() => setNowTick(t => t + 1), 15000);
    return () => { cancelled = true; clearInterval(id); clearInterval(tickId); };
  }, [host]);

  const relTime = (ts?: number, iso?: string): string => {
    let seconds = 0;
    if (typeof ts === 'number' && isFinite(ts)) seconds = Date.now() / 1000 - ts;
    else if (iso) {
      const parsed = Date.parse(iso);
      if (!isNaN(parsed)) seconds = (Date.now() - parsed) / 1000;
      else return iso;
    } else return '—';
    if (seconds < 0) seconds = 0;
    if (seconds < 60) return `${Math.round(seconds)}s ago`;
    if (seconds < 3600) return `${Math.round(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.round(seconds / 3600)}h ago`;
    return `${Math.round(seconds / 86400)}d ago`;
  };
  // touch nowTick so the table re-renders relative-time strings.
  void nowTick;

  return (
    <div style={{ marginTop: T.spacing.xl }}>
      <Label color={C.textMuted} mb={T.spacing.md}>Trainer sessions</Label>
      {rows && rows.length > 0 ? (
        (() => {
          const cols: ReadonlyArray<Column<TrainerSessionRow>> = [
            {
              id: 'trainer', header: 'Trainer', align: 'left',
              sortKey: (r) => r.trainer.toLowerCase(),
              accessor: (r) => <span style={{ fontFamily: T.typography.fontMono, color: C.text }}>{r.trainer}</span>,
            },
            {
              id: 'session_id', header: 'Session', align: 'left',
              sortKey: (r) => r.session_id,
              accessor: (r) => <span style={{ fontFamily: T.typography.fontMono, color: C.textMuted, fontSize: T.typography.sizeXs }}>{r.session_id.slice(0, 12)}</span>,
            },
            {
              id: 'turns', header: 'Turns', align: 'right',
              sortKey: (r) => r.turns,
              accessor: (r) => <span style={{ fontFamily: T.typography.fontMono, color: C.accent }}>{r.turns.toLocaleString()}</span>,
            },
            {
              id: 'up', header: 'Up%', align: 'right',
              sortKey: (r) => (r.turns ? (r.up_count || 0) / r.turns : 0),
              accessor: (r) => {
                const pct = r.turns ? ((r.up_count || 0) / r.turns) * 100 : 0;
                return <span style={{ fontFamily: T.typography.fontMono, color: pct >= 80 ? C.green : pct >= 50 ? C.yellow : C.textMuted }}>{pct.toFixed(0)}%</span>;
              },
            },
            {
              id: 'down', header: 'Down%', align: 'right',
              sortKey: (r) => (r.turns ? (r.down_count || 0) / r.turns : 0),
              accessor: (r) => {
                const pct = r.turns ? ((r.down_count || 0) / r.turns) * 100 : 0;
                return <span style={{ fontFamily: T.typography.fontMono, color: pct >= 20 ? C.red : C.textMuted }}>{pct.toFixed(0)}%</span>;
              },
            },
            {
              id: 'corrections', header: 'Corrections', align: 'right',
              sortKey: (r) => r.correction_count || 0,
              accessor: (r) => <span style={{ fontFamily: T.typography.fontMono, color: (r.correction_count || 0) > 0 ? C.yellow : C.textMuted }}>{(r.correction_count || 0).toLocaleString()}</span>,
            },
            {
              id: 'last', header: 'Last active', align: 'right',
              sortKey: (r) => r.last_activity_ts || (r.last_activity ? Date.parse(r.last_activity) / 1000 : 0),
              accessor: (r) => <span style={{ fontFamily: T.typography.fontMono, color: C.textSecondary, fontSize: T.typography.sizeXs }}>{relTime(r.last_activity_ts, r.last_activity)}</span>,
            },
          ];
          return (
            <DataTable<TrainerSessionRow> C={C} rows={rows} columns={cols}
              rowKey={(r) => r.session_id}
              sort={{ col: 'last', dir: 'desc' }} />
          );
        })()
      ) : (
        <div style={{
          padding: T.spacing.md, background: C.bgInput,
          border: `1px dashed ${C.borderSubtle}`, borderRadius: T.radii.md,
          color: C.textDim, fontSize: T.typography.sizeXs, lineHeight: 1.55,
        }}>
          {err ? (
            <>
              <strong style={{ color: C.textMuted }}>Trainer sessions unavailable</strong> — {err}. Endpoint is <code style={{ fontFamily: T.typography.fontMono, color: C.accent }}>GET /api/trainer/sessions</code>.
            </>
          ) : (
            <>
              <strong style={{ color: C.textMuted }}>No external trainer sessions yet.</strong>{' '}
              Kick off <code style={{ fontFamily: T.typography.fontMono, color: C.accent }}>scripts/gemini-trainer.sh</code> or POST to <code style={{ fontFamily: T.typography.fontMono, color: C.accent }}>/api/trainer/turn</code> and rows will appear here.
            </>
          )}
        </div>
      )}
    </div>
  );
};

const LessonsTab: React.FC<{
  C: any;
  host: string;
  training?: DashboardShape['training'];
  files: Array<{ file: string; pairs: number; size_mb: number }>;
}> = ({ C, host, training, files }) => {
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
          {/* c2-379 / BIG #180: Lesson roster -> DataTable. Share column
              derives from totalPairs so the sortKey computes percentage. */}
          {(() => {
            type FRow = { file: string; pairs: number; size_mb: number };
            const rows = [...files].slice(0, 50) as FRow[];
            const cols: ReadonlyArray<Column<FRow>> = [
              {
                id: 'file', header: 'Dataset', align: 'left',
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
              {
                id: 'share', header: 'Share', align: 'right',
                sortKey: (f) => totalPairs > 0 ? f.pairs / totalPairs : 0,
                accessor: (f) => {
                  const share = totalPairs > 0 ? (f.pairs / totalPairs) * 100 : 0;
                  return <span style={{ fontFamily: T.typography.fontMono, color: C.textMuted }}>{share.toFixed(1)}%</span>;
                },
              },
            ];
            return (
              <DataTable<FRow> C={C} rows={rows} columns={cols}
                rowKey={(f) => f.file}
                sort={{ col: 'pairs', dir: 'desc' }} />
            );
          })()}
        </div>
      )}
      <TrainerSessionsCard C={C} host={host} />
    </div>
  );
};

// c2-433 / #350: Server-side feedback queue. Polls /api/feedback/recent every
// 10s while the tab is mounted; falls back to session-local events if the
// endpoint is unreachable (offline, backend mid-restart). Displays up/down/
// correct ratings with separate color treatment + a "Mark as ingested" hook
// for the future ingestion-decision API. Until that lands the row click just
// expands the correction text.
interface ServerFeedbackRow {
  id: number | string;
  // Backend ships ISO-ish "YYYY-MM-DD HH:MM:SS" strings via created_at; the
  // older `ts` (epoch ms) field is kept for the session-fallback rows + any
  // future schema bump.
  ts?: number | string;
  created_at?: string;
  // null/undefined when the feedback hasn't been ingested into the substrate
  // yet; presence + a string indicates Claude 0's reasoner has consumed it.
  processed_at?: string | null;
  conversation_id?: string;
  message_id?: number;
  conclusion_id?: number;
  user_query?: string;
  lfi_reply?: string;
  rating: 'up' | 'down' | 'correct' | string;
  correction?: string;
  comment?: string;
}

type FeedbackFilter = 'all' | 'up' | 'down' | 'correct';

const OfficeHoursTab: React.FC<{ C: any; host: string; events: Array<{ t: number; kind: string; data?: any }> }> = ({ C, host, events }) => {
  const [serverRows, setServerRows] = React.useState<ServerFeedbackRow[] | null>(null);
  const [serverErr, setServerErr] = React.useState<string | null>(null);
  const [expanded, setExpanded] = React.useState<Record<string, boolean>>({});
  // c2-433 / task 233 + 243: filter chip — narrows the queue to one rating
  // type. Persisted to localStorage so triagers who prefer "Down only" view
  // stay there across reloads. 'all' is the default for first-time visitors.
  const FILTER_KEY = 'lfi_office_hours_filter_v1';
  const [filter, setFilterState] = React.useState<FeedbackFilter>(() => {
    try {
      const raw = localStorage.getItem(FILTER_KEY);
      if (raw === 'up' || raw === 'down' || raw === 'correct' || raw === 'all') return raw;
    } catch { /* SSR / quota / private mode — fall through to default */ }
    return 'all';
  });
  const setFilter = React.useCallback((next: FeedbackFilter) => {
    setFilterState(next);
    try { localStorage.setItem(FILTER_KEY, next); } catch { /* quota — silent */ }
  }, []);
  // c2-433 / task 234b: last-fetched epoch + manual refresh affordance.
  // Triagers can hit the button to skip the 10s poll cadence when they
  // know a row was just submitted.
  const [lastFetched, setLastFetched] = React.useState<number | null>(null);
  // c2-433 / task 269: start refreshing=true so the first paint shows
  // "Refreshing…" instead of an inert "Refresh" button before the
  // mount-time useEffect fires its first load. Resolves the brief
  // mismatch where lastFetched is null + refreshing is false.
  const [refreshing, setRefreshing] = React.useState<boolean>(true);
  const loadRef = React.useRef<() => Promise<void>>(() => Promise.resolve());
  React.useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        setRefreshing(true);
        const r = await fetch(`http://${host}:3000/api/feedback/recent?limit=200`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const data = await r.json();
        if (cancelled) return;
        const rows: ServerFeedbackRow[] = Array.isArray(data) ? data : Array.isArray(data?.rows) ? data.rows : Array.isArray(data?.feedback) ? data.feedback : [];
        setServerRows(rows);
        setServerErr(null);
        setLastFetched(Date.now());
      } catch (e: any) {
        if (cancelled) return;
        setServerErr(String(e?.message || e || 'fetch failed'));
      } finally {
        if (!cancelled) setRefreshing(false);
      }
    };
    loadRef.current = load;
    load();
    const id = window.setInterval(load, 10_000);
    return () => { cancelled = true; window.clearInterval(id); };
  }, [host]);
  const fmtAge = (ms: number): string => {
    const s = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    return `${Math.floor(s / 3600)}h ago`;
  };

  // Normalise: server rows authoritative when available; otherwise use the
  // session-local events as a degraded view. Sort newest first. Backend
  // ships created_at not ts — read either, parse to epoch for sort.
  const tsOf = (r: ServerFeedbackRow): number => {
    if (typeof r.ts === 'number') return r.ts;
    if (r.ts) return Date.parse(String(r.ts));
    if (r.created_at) return Date.parse(r.created_at + 'Z'); // assume UTC
    return 0;
  };
  const normalised: ServerFeedbackRow[] = React.useMemo(() => {
    if (serverRows && serverRows.length > 0) {
      return serverRows.slice().sort((a, b) => tsOf(b) - tsOf(a));
    }
    // Fallback: synthesise rows from session events.
    return events
      .filter(e => e.kind === 'feedback_positive' || e.kind === 'feedback_negative' || e.kind === 'feedback_correct')
      .slice().reverse()
      .map((e, i) => ({
        id: `local-${i}-${e.t}`,
        ts: e.t,
        rating: e.kind === 'feedback_positive' ? 'up' : e.kind === 'feedback_correct' ? 'correct' : 'down',
        message_id: e.data?.msgId,
        comment: e.data?.category,
      } as ServerFeedbackRow));
  }, [serverRows, events]);

  const upCount = normalised.filter(r => r.rating === 'up').length;
  const downCount = normalised.filter(r => r.rating === 'down').length;
  const correctCount = normalised.filter(r => r.rating === 'correct').length;
  const total = normalised.length;
  const ratingColor = (r: string): string => r === 'up' ? C.green : r === 'correct' ? C.accent : C.red;
  const ratingLabel = (r: string): string => r === 'up' ? 'Up' : r === 'correct' ? 'Correction' : r === 'down' ? 'Down' : r;
  const ratingBg = (r: string): string => r === 'up' ? C.greenBg : r === 'correct' ? C.accentBg : C.redBg;
  const fmtRow = (row: ServerFeedbackRow): string => {
    const ms = tsOf(row);
    if (!ms) return '';
    const d = new Date(ms);
    const now = Date.now();
    if (now - ms < 24 * 3600_000) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    return d.toLocaleString();
  };

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.md, marginBottom: '12px', flexWrap: 'wrap' }}>
        <h2 style={{ fontSize: T.typography.size2xl, fontWeight: 600, color: C.text, margin: 0 }}>Office Hours</h2>
        <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
          {lastFetched && (
            <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}
              title={`Last fetched: ${new Date(lastFetched).toLocaleTimeString()}`}>
              {fmtAge(lastFetched)}
            </span>
          )}
          <button onClick={() => { void loadRef.current(); }}
            disabled={refreshing}
            title='Refetch from /api/feedback/recent'
            aria-label='Refresh feedback queue'
            style={{
              padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
              background: refreshing ? C.bgInput : 'transparent',
              border: `1px solid ${C.borderSubtle}`, color: refreshing ? C.textDim : C.textMuted,
              borderRadius: T.radii.md, cursor: refreshing ? 'wait' : 'pointer',
              fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
            }}>{refreshing ? 'Refreshing…' : 'Refresh'}</button>
        </div>
      </div>
      <p style={{ fontSize: T.typography.sizeMd, color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Live feed from <code>POST /api/feedback</code>. Up = thumbs-up, Down = thumbs-down with optional comment, Correction = user-supplied right answer for ingestion. {serverErr && <span style={{ color: C.yellow, marginLeft: '8px' }}>(server fallback: {serverErr} — showing session-local only)</span>}
      </p>
      <div style={{ display: 'flex', gap: T.spacing.md, marginBottom: T.spacing.xl, flexWrap: 'wrap' }}>
        {/* c2-433 / task 254: stat cards are now click-to-filter shortcuts —
            bigger tap targets than the chips below. Click a card → filter
            queue to that rating. Click again → reset to All. Active card
            gets a subtle ring so the bound state is visible. */}
        {([
          { id: 'up' as FeedbackFilter, label: 'Up', count: upCount, fg: C.green, bg: C.greenBg, border: C.greenBorder },
          { id: 'down' as FeedbackFilter, label: 'Down', count: downCount, fg: C.red, bg: C.redBg, border: C.redBorder },
          { id: 'correct' as FeedbackFilter, label: 'Corrections', count: correctCount, fg: C.accent, bg: C.accentBg, border: C.accentBorder },
        ]).map(c => {
          const active = filter === c.id;
          return (
            <button key={c.id}
              onClick={() => setFilter(active ? 'all' : c.id)}
              aria-pressed={active}
              title={active ? `Showing only ${c.label} — click to clear` : `Filter to ${c.label}`}
              style={{
                flex: 1, minWidth: '120px', padding: T.spacing.md,
                background: c.bg, border: `1px solid ${c.border}`, borderRadius: T.radii.md,
                cursor: 'pointer', textAlign: 'left', fontFamily: 'inherit',
                boxShadow: active ? `0 0 0 2px ${c.fg}` : 'none',
                transition: 'box-shadow 0.15s',
              }}>
              <Label color={c.fg}>{c.label}</Label>
              <div style={{ fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack, color: c.fg, fontFamily: T.typography.fontMono }}>{c.count}</div>
            </button>
          );
        })}
        {total > 0 && (() => {
          const pct = Math.round((upCount / total) * 100);
          const col = pct >= 70 ? C.green : pct >= 50 ? C.yellow : C.red;
          const bg = pct >= 70 ? C.greenBg : pct >= 50 ? C.yellowBg : C.redBg;
          const border = pct >= 70 ? C.greenBorder : pct >= 50 ? C.accentBorder : C.redBorder;
          return (
            <div style={{ flex: 1, minWidth: '120px', padding: T.spacing.md, background: bg, border: `1px solid ${border}`, borderRadius: T.radii.md }}>
              <Label color={col}>Sentiment</Label>
              <div style={{ fontSize: T.typography.size3xl, fontWeight: T.typography.weightBlack, color: col, fontFamily: T.typography.fontMono }}>{pct}%</div>
              <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, fontFamily: T.typography.fontMono, marginTop: '2px' }}>{total} total</div>
            </div>
          );
        })()}
      </div>
      {/* c2-433 / task 233: rating filter chips. Active chip is accent-tinted;
          others sit dim until clicked. Counts mirror the cards above so users
          see at a glance which segments have data. */}
      {normalised.length > 0 && (
        <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', marginBottom: T.spacing.md }}>
          {([
            { id: 'all', label: 'All', count: total },
            { id: 'up', label: 'Up', count: upCount },
            { id: 'down', label: 'Down', count: downCount },
            { id: 'correct', label: 'Corrections', count: correctCount },
          ] as Array<{ id: FeedbackFilter; label: string; count: number }>).map(c => {
            const active = filter === c.id;
            return (
              <button key={c.id} onClick={() => setFilter(c.id)}
                aria-pressed={active}
                style={{
                  padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                  background: active ? C.accentBg : 'transparent',
                  border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                  color: active ? C.accent : C.textMuted,
                  borderRadius: T.radii.pill, cursor: 'pointer', fontFamily: 'inherit',
                  textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                }}>{c.label} <span style={{ opacity: 0.7, fontFamily: T.typography.fontMono }}>({c.count})</span></button>
            );
          })}
        </div>
      )}
      {(() => {
        const view = filter === 'all' ? normalised : normalised.filter(r => r.rating === filter);
        if (normalised.length === 0) {
          return (
            <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeMd, fontStyle: 'italic' }}>
              No feedback captured yet. Use 👍 / 👎 / ✏️ on any AI response to populate this queue.
            </div>
          );
        }
        if (view.length === 0) {
          return (
            <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeMd, fontStyle: 'italic' }}>
              No <strong>{filter}</strong> feedback yet. <button onClick={() => setFilter('all')} style={{ background: 'transparent', border: 'none', color: C.accent, cursor: 'pointer', fontFamily: 'inherit', textDecoration: 'underline', padding: 0 }}>Show all.</button>
            </div>
          );
        }
        // c2-433 / task 285 + 286: day-bucket grouping via the shared
        // formatDayBucket helper from util.ts (was inline reimplementation
        // — DRY'd to match chat-list day separators + sidebar groupings).
        // Triagers scan the queue at-a-glance for "what came in today vs
        // older". Bucket header renders before the first row in each
        // bucket; tracked via prevBucket scan.
        const dayBucket = (ms: number): string => ms ? formatDayBucket(ms) : 'Unknown date';
        let prevBucket = '';
        return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {view.slice(0, 100).map(row => {
            const key = String(row.id);
            const isOpen = !!expanded[key];
            const hasDetail = !!(row.correction || row.comment || row.lfi_reply || row.user_query);
            // c2-433 / task 248: dim ingested rows (processed_at populated)
            // so triagers visually skip them and focus on the pending queue.
            // Hover restores opacity so the row is still inspectable.
            const isIngested = !!row.processed_at;
            const bucket = dayBucket(tsOf(row));
            const showBucket = bucket !== prevBucket;
            if (showBucket) prevBucket = bucket;
            return (
              <React.Fragment key={`${key}-frag`}>
              {showBucket && (
                <div role='heading' aria-level={3} style={{
                  fontSize: '10px', fontWeight: T.typography.weightBold,
                  color: C.textDim, textTransform: 'uppercase',
                  letterSpacing: T.typography.trackingLoose,
                  paddingLeft: '4px', marginTop: prevBucket === bucket ? '0' : T.spacing.xs,
                }}>{bucket}</div>
              )}
              <div key={key}
                onMouseEnter={(e) => { e.currentTarget.style.opacity = '1'; }}
                onMouseLeave={(e) => { e.currentTarget.style.opacity = isIngested ? '0.55' : '1'; }}
                style={{
                  border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
                  background: C.bgCard, overflow: 'hidden',
                  opacity: isIngested ? 0.55 : 1,
                  transition: 'opacity 0.15s',
                }}>
                <button onClick={() => hasDetail && setExpanded(p => ({ ...p, [key]: !isOpen }))}
                  disabled={!hasDetail}
                  style={{
                    width: '100%', display: 'flex', alignItems: 'center', gap: T.spacing.md,
                    padding: '10px 12px', background: 'transparent', border: 'none',
                    color: C.text, fontFamily: 'inherit', textAlign: 'left',
                    cursor: hasDetail ? 'pointer' : 'default',
                  }}>
                  <span style={{
                    fontSize: '11px', fontWeight: T.typography.weightBold,
                    color: ratingColor(row.rating), background: ratingBg(row.rating),
                    padding: '3px 8px', borderRadius: T.radii.sm,
                    fontFamily: T.typography.fontMono, flexShrink: 0, minWidth: '76px', textAlign: 'center',
                  }}>{ratingLabel(row.rating)}</span>
                  <span style={{ fontSize: '11px', color: C.textMuted, fontFamily: T.typography.fontMono, flexShrink: 0, minWidth: '70px' }}>{fmtRow(row)}</span>
                  {/* c2-433 / task 234: ingestion status badge. processed_at
                      null means Claude 0's reasoner hasn't consumed this row
                      yet; a string timestamp means it has. Helps triagers see
                      backlog at a glance. Hidden when the field isn't present
                      in the row at all (session-fallback shape). */}
                  {'processed_at' in row && (
                    <span title={row.processed_at ? `Ingested at ${row.processed_at}` : 'Awaiting ingestion'}
                      style={{
                        fontSize: '9px', fontWeight: T.typography.weightBold,
                        color: row.processed_at ? C.green : C.yellow,
                        background: row.processed_at ? C.greenBg : C.yellowBg,
                        border: `1px solid ${row.processed_at ? C.greenBorder : C.yellow}`,
                        padding: '1px 5px', borderRadius: T.radii.sm,
                        flexShrink: 0, fontFamily: T.typography.fontMono,
                        textTransform: 'uppercase', letterSpacing: '0.04em',
                      }}>{row.processed_at ? 'Ingested' : 'Pending'}</span>
                  )}
                  <span style={{ flex: 1, fontSize: '12px', color: C.textSecondary, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {row.correction
                      ? `→ ${row.correction.slice(0, 120)}${row.correction.length > 120 ? '…' : ''}`
                      : row.comment
                        ? row.comment.slice(0, 140)
                        : row.lfi_reply
                          ? row.lfi_reply.slice(0, 140) + (row.lfi_reply.length > 140 ? '…' : '')
                          : ''}
                  </span>
                  {row.message_id != null && (
                    <span style={{ fontSize: '10px', color: C.textDim, fontFamily: T.typography.fontMono, flexShrink: 0 }}>msg {row.message_id}</span>
                  )}
                  {hasDetail && (
                    <span style={{ fontSize: '10px', color: C.textDim, flexShrink: 0 }}>{isOpen ? '▾' : '▸'}</span>
                  )}
                </button>
                {isOpen && hasDetail && (
                  <div style={{ padding: '0 12px 12px', borderTop: `1px solid ${C.borderSubtle}` }}>
                    {row.user_query && (
                      <>
                        <div style={{ fontSize: '10px', color: C.textMuted, marginTop: '10px', textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, fontWeight: T.typography.weightSemibold }}>User asked</div>
                        <div style={{ fontSize: '12px', color: C.textSecondary, padding: '6px 8px', background: C.bgInput, borderRadius: T.radii.sm, marginTop: '4px', whiteSpace: 'pre-wrap', maxHeight: '120px', overflowY: 'auto' }}>{row.user_query}</div>
                      </>
                    )}
                    {row.lfi_reply && (
                      <>
                        <div style={{ fontSize: '10px', color: C.textMuted, marginTop: '10px', textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, fontWeight: T.typography.weightSemibold }}>LFI replied</div>
                        <div style={{ fontSize: '12px', color: C.textSecondary, padding: '6px 8px', background: C.bgInput, borderRadius: T.radii.sm, marginTop: '4px', whiteSpace: 'pre-wrap', maxHeight: '160px', overflowY: 'auto' }}>{row.lfi_reply}</div>
                      </>
                    )}
                    {row.correction && (
                      <>
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.sm, marginTop: '10px' }}>
                          <div style={{ fontSize: '10px', color: C.accent, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, fontWeight: T.typography.weightSemibold }}>User correction</div>
                          {/* c2-433 / task 239: copy correction text to clipboard.
                              Triagers often want to paste the corrected answer
                              elsewhere (review doc, ticket); without this they
                              had to manual-select inside a max-height scroller. */}
                          <button onClick={(e) => {
                            e.stopPropagation();
                            try { navigator.clipboard.writeText(row.correction || ''); } catch { /* clipboard API blocked */ }
                            const btn = e.currentTarget;
                            const orig = btn.textContent;
                            btn.textContent = 'Copied';
                            btn.style.color = C.green;
                            window.setTimeout(() => { btn.textContent = orig; btn.style.color = C.accent; }, 1200);
                          }}
                            title='Copy correction to clipboard'
                            style={{
                              padding: '2px 8px', fontSize: '10px', fontWeight: T.typography.weightBold,
                              background: 'transparent', border: `1px solid ${C.accentBorder}`,
                              color: C.accent, borderRadius: T.radii.sm, cursor: 'pointer',
                              fontFamily: 'inherit', textTransform: 'uppercase',
                              letterSpacing: T.typography.trackingLoose,
                            }}>Copy</button>
                        </div>
                        <div style={{ fontSize: '12px', color: C.text, padding: '6px 8px', background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm, marginTop: '4px', whiteSpace: 'pre-wrap', maxHeight: '200px', overflowY: 'auto' }}>{row.correction}</div>
                      </>
                    )}
                    {row.comment && (
                      <>
                        <div style={{ fontSize: '10px', color: C.textMuted, marginTop: '10px', textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, fontWeight: T.typography.weightSemibold }}>Comment</div>
                        <div style={{ fontSize: '12px', color: C.textSecondary, padding: '6px 8px', background: C.bgInput, borderRadius: T.radii.sm, marginTop: '4px', whiteSpace: 'pre-wrap' }}>{row.comment}</div>
                      </>
                    )}
                  </div>
                )}
              </div>
              </React.Fragment>
            );
          })}
        </div>
        );
      })()}
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
        // CSP blocks :3002; go straight to :3000 main backend.
        const data: any = await tryFetch(3000);
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

// c2-433 / #298 followup: Ledger tab — surfaces the contradictions queue
// the Classroom tab badge counts against. Polls /api/contradictions/recent
// every 15s. Tolerant to several payload shapes (array / {items} /
// {contradictions}), and each row is rendered with tolerant field pickup:
// side_a|a|this, side_b|b|other, fact_key|key, at|timestamp|created_at.
// Empty state explains the ≥ 0.7 confidence threshold so users know when
// to expect entries.
const LedgerTab: React.FC<{ C: any; host: string; onOpenFactKey?: (key: string, rect: DOMRect) => void }> = ({ C, host, onOpenFactKey }) => {
  const [rows, setRows] = React.useState<any[] | null>(null);
  const [loading, setLoading] = React.useState<boolean>(false);
  const [err, setErr] = React.useState<string | null>(null);
  const [lastFetched, setLastFetched] = React.useState<number | null>(null);
  const [resolving, setResolving] = React.useState<boolean>(false);
  const [resolveMsg, setResolveMsg] = React.useState<string | null>(null);
  // c2-433 / #298 followup: per-row resolve state — rowId -> 'a'|'b'|'dismiss'|'done'|'failed'.
  // Rows marked 'done' fade out until the next poll drops them entirely.
  const [rowResolving, setRowResolving] = React.useState<Record<string, string>>({});
  // c2-433 / #298 followup: copy-feedback for the JSON export. Flips to
  // Date.now() on click, self-reverts 2s later so the button label cycles
  // Copy → Copied ✓ → Copy. Matches the Drift tab export pattern.
  const [copiedAt, setCopiedAt] = React.useState<number>(0);

  const load = React.useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const r = await fetch(`http://${host}:3000/api/contradictions/recent`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json();
      const list: any[] = Array.isArray(data) ? data
        : Array.isArray(data?.items) ? data.items
        : Array.isArray(data?.contradictions) ? data.contradictions
        : [];
      setRows(list);
      setLastFetched(Date.now());
    } catch (e: any) {
      setErr(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  }, [host]);
  React.useEffect(() => {
    load();
    const id = window.setInterval(load, 15_000);
    return () => window.clearInterval(id);
  }, [load]);

  const runAutoResolve = async () => {
    setResolving(true);
    setResolveMsg(null);
    try {
      const r = await fetch(`http://${host}:3000/api/contradictions/auto-resolve`, { method: 'POST' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json().catch(() => ({}));
      const resolved = data.resolved ?? data.count ?? data.n ?? '?';
      const total = data.total ?? data.pending ?? null;
      setResolveMsg(total != null ? `Resolved ${resolved} of ${total}` : `Resolved ${resolved}`);
      load();
    } catch (e: any) {
      setResolveMsg(`Failed: ${String(e?.message || e || 'unknown')}`);
    } finally {
      setResolving(false);
      window.setTimeout(() => setResolveMsg(null), 4000);
    }
  };

  // c2-433 / #298 followup: per-row manual resolve. Backend expects the
  // row id + a winner side; we also try {side}, {keep}, and {verdict} for
  // shape tolerance. On success, optimistically mark the row 'done' (fade
  // out), then refetch 300ms later so the list drops the resolved row.
  const resolveRow = async (rowId: string, winner: 'a' | 'b' | 'dismiss') => {
    setRowResolving(prev => ({ ...prev, [rowId]: winner }));
    try {
      const r = await fetch(`http://${host}:3000/api/contradictions/${encodeURIComponent(rowId)}/resolve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ winner, side: winner, keep: winner, verdict: winner }),
      });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      setRowResolving(prev => ({ ...prev, [rowId]: 'done' }));
      window.setTimeout(() => { load(); }, 300);
    } catch (e: any) {
      setRowResolving(prev => ({ ...prev, [rowId]: 'failed' }));
      window.setTimeout(() => {
        setRowResolving(prev => { const n = { ...prev }; delete n[rowId]; return n; });
      }, 3000);
    }
  };

  const rowIdOf = (r: any): string | null => {
    const id = r.id ?? r.contradiction_id ?? r._id;
    return id != null ? String(id) : null;
  };

  const side = (r: any, which: 'a' | 'b'): string => {
    const v = which === 'a' ? (r.side_a ?? r.a ?? r.this) : (r.side_b ?? r.b ?? r.other);
    if (v == null) return '—';
    if (typeof v === 'string') return v;
    if (typeof v === 'object') {
      const pred = v.pred || v.predicate || '';
      const obj = v.obj || v.object || v.value || '';
      const src = v.source ? ` [${v.source}]` : '';
      const trust = typeof v.trust === 'number' ? ` (t=${v.trust.toFixed(2)})` : '';
      return (pred && obj) ? `${pred} ${obj}${src}${trust}` : JSON.stringify(v).slice(0, 80);
    }
    return String(v);
  };
  const factKeyOf = (r: any): string | null => r.fact_key || r.key || null;
  const timeOf = (r: any): number | null => {
    const t = r.at || r.timestamp || r.created_at;
    if (t == null) return null;
    if (typeof t === 'number') return t;
    const ms = Date.parse(String(t));
    return Number.isNaN(ms) ? null : ms;
  };

  const count = rows?.length ?? 0;

  return (
    <div style={{ padding: T.spacing.xl, display: 'flex', flexDirection: 'column', gap: T.spacing.md, minWidth: 0 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, flexWrap: 'wrap' }}>
        <h2 style={{ margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>
          Contradiction ledger
          {rows != null && <span style={{ marginLeft: T.spacing.sm, color: C.textMuted, fontSize: T.typography.sizeMd, fontWeight: 500 }}>({count})</span>}
        </h2>
        <div style={{ flex: 1 }} />
        {resolveMsg && (
          <span role='status' style={{
            fontSize: T.typography.sizeXs,
            color: resolveMsg.startsWith('Failed') ? C.red : C.green,
            fontFamily: T.typography.fontMono,
          }}>{resolveMsg}</span>
        )}
        {lastFetched != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            {formatRelative(lastFetched)}
          </span>
        )}
        <button onClick={runAutoResolve} disabled={resolving || count === 0}
          title='Apply source_trust weights across the pending ledger'
          style={{
            background: resolving ? C.bgInput : 'transparent',
            border: `1px solid ${C.borderSubtle}`, color: count === 0 ? C.textDim : C.textMuted,
            borderRadius: T.radii.sm, cursor: (resolving || count === 0) ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
          }}>{resolving ? 'Resolving…' : 'Auto-resolve'}</button>
        {/* c2-433 / #298 followup: Copy pending ledger as JSON for external
            triage / ticket-filing. Disabled when there's nothing to export. */}
        <button
          disabled={count === 0}
          onClick={async () => {
            const payload = {
              exported_at: new Date().toISOString(),
              pending: rows || [],
            };
            try {
              await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
              setCopiedAt(Date.now());
              window.setTimeout(() => setCopiedAt(0), 2000);
            } catch { /* clipboard blocked */ }
          }}
          title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${count} pending contradiction${count === 1 ? '' : 's'} as JSON`}
          style={{
            background: copiedAt > 0 ? `${C.green}18` : 'transparent',
            border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
            color: copiedAt > 0 ? C.green : count === 0 ? C.textDim : C.textMuted,
            borderRadius: T.radii.sm,
            cursor: count === 0 ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: count === 0 ? 0.5 : 1,
          }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
        <button onClick={load} disabled={loading}
          title={loading ? 'Refreshing…' : 'Refresh now'}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
          }}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      </div>

      {err && (
        <div role='alert' style={{
          padding: T.spacing.md, background: C.redBg,
          border: `1px solid ${C.redBorder}`, color: C.red,
          borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
        }}>Could not load ledger: {err}</div>
      )}

      {rows != null && count === 0 && !err && (
        <div style={{
          padding: T.spacing.xl, textAlign: 'center',
          color: C.textMuted, fontSize: T.typography.sizeSm, lineHeight: 1.55,
          background: C.bgCard, border: `1px dashed ${C.borderSubtle}`, borderRadius: T.radii.lg,
        }}>
          No pending contradictions. <br/>
          <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>
            Entries land here when <code style={{ fontFamily: T.typography.fontMono }}>upsert_fact</code> sees two sources disagree with both ≥ 0.7 confidence.
          </span>
        </div>
      )}

      {rows != null && count > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
          {rows.map((r, i) => {
            const fk = factKeyOf(r);
            const t = timeOf(r);
            const rowId = rowIdOf(r);
            const rowState = rowId ? rowResolving[rowId] : undefined;
            const isDone = rowState === 'done';
            const isFailed = rowState === 'failed';
            const isPending = rowState === 'a' || rowState === 'b' || rowState === 'dismiss';
            return (
              <div key={rowId || i} style={{
                padding: T.spacing.md, borderRadius: T.radii.md,
                background: C.bgCard, border: `1px solid ${isFailed ? C.redBorder : C.borderSubtle}`,
                display: 'flex', flexDirection: 'column', gap: '4px',
                opacity: isDone ? 0.4 : isPending ? 0.7 : 1,
                transition: 'opacity 200ms, border-color 200ms',
              }}>
                <div style={{
                  display: 'flex', justifyContent: 'space-between', alignItems: 'baseline',
                  gap: T.spacing.sm, flexWrap: 'wrap',
                }}>
                  {fk && (
                    onOpenFactKey ? (
                      <button onClick={(e) => onOpenFactKey(fk, e.currentTarget.getBoundingClientRect())}
                        title={`Open ancestry popover for ${fk}`}
                        aria-label={`Open fact ${fk}`}
                        style={{
                          fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
                          color: C.accent, fontWeight: 700,
                          background: 'transparent', border: 'none',
                          padding: 0, cursor: 'pointer',
                          textDecoration: 'underline', textDecorationColor: `${C.accent}55`,
                          textUnderlineOffset: '2px',
                        }}>{fk}</button>
                    ) : (
                      <span style={{
                        fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
                        color: C.accent, fontWeight: 700,
                      }}>{fk}</span>
                    )
                  )}
                  {t != null && (
                    <span style={{ fontFamily: T.typography.fontMono, fontSize: '10px', color: C.textDim }}>
                      {formatRelative(t)}
                    </span>
                  )}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, flexWrap: 'wrap' }}>
                  <span style={{
                    fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
                    color: C.text, flex: '1 1 160px', minWidth: 0, wordBreak: 'break-word',
                  }}>{side(r, 'a')}</span>
                  <span style={{ color: C.red, fontSize: T.typography.sizeMd, fontWeight: 900 }}>↔</span>
                  <span style={{
                    fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
                    color: C.text, flex: '1 1 160px', minWidth: 0, wordBreak: 'break-word',
                  }}>{side(r, 'b')}</span>
                </div>
                {r.verdict && (
                  <span style={{
                    alignSelf: 'flex-start', fontSize: '10px',
                    fontFamily: T.typography.fontMono, color: C.textMuted,
                    textTransform: 'uppercase', letterSpacing: '0.06em',
                  }}>{String(r.verdict)}</span>
                )}
                {/* c2-433 / #298 followup: per-row resolve actions. Backend
                    expects POST /api/contradictions/:id/resolve with a
                    winner side. Hidden when the row has no stable id
                    (auto-resolve + legacy paths still work via header). */}
                {rowId && !isDone && (
                  <div style={{
                    display: 'flex', gap: '6px', marginTop: '4px',
                    flexWrap: 'wrap', alignItems: 'center',
                  }}>
                    {isFailed && (
                      <span style={{ fontSize: '10px', color: C.red, fontFamily: T.typography.fontMono }}>
                        Resolve failed — try again
                      </span>
                    )}
                    <button onClick={() => resolveRow(rowId, 'a')} disabled={isPending}
                      title='Keep side A; discard side B'
                      style={{
                        padding: '3px 9px', fontSize: '10px', fontWeight: T.typography.weightBold,
                        background: C.accentBg, color: C.accent,
                        border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm,
                        cursor: isPending ? 'wait' : 'pointer',
                        fontFamily: 'inherit', letterSpacing: '0.04em',
                        opacity: isPending && rowState !== 'a' ? 0.4 : 1,
                      }}>{rowState === 'a' ? 'Keeping A…' : 'Keep A'}</button>
                    <button onClick={() => resolveRow(rowId, 'b')} disabled={isPending}
                      title='Keep side B; discard side A'
                      style={{
                        padding: '3px 9px', fontSize: '10px', fontWeight: T.typography.weightBold,
                        background: C.accentBg, color: C.accent,
                        border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.sm,
                        cursor: isPending ? 'wait' : 'pointer',
                        fontFamily: 'inherit', letterSpacing: '0.04em',
                        opacity: isPending && rowState !== 'b' ? 0.4 : 1,
                      }}>{rowState === 'b' ? 'Keeping B…' : 'Keep B'}</button>
                    <button onClick={() => resolveRow(rowId, 'dismiss')} disabled={isPending}
                      title='Dismiss this contradiction without picking a side'
                      style={{
                        padding: '3px 9px', fontSize: '10px', fontWeight: T.typography.weightBold,
                        background: 'transparent', color: C.textMuted,
                        border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.sm,
                        cursor: isPending ? 'wait' : 'pointer',
                        fontFamily: 'inherit', letterSpacing: '0.04em',
                        opacity: isPending && rowState !== 'dismiss' ? 0.4 : 1,
                      }}>{rowState === 'dismiss' ? 'Dismissing…' : 'Dismiss'}</button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

// c2-433 / #284: Drift tab. Polls /api/drift/snapshot every 60s for the
// one-call health bundle (fresh_ratio, stale_ratio, hdc_cache_ratio,
// contradictions_pending, feedback_negative_ratio_24h, fsrs_lapse_rate).
// Session-local ring buffer (last 60 samples = 1h @ 60s) drives a
// sparkline per metric so users can see whether the value is trending
// up or down. Pure frontend state — persisting would need localStorage
// but refresh-on-load is fine for a dashboard.
const DriftTab: React.FC<{ C: any; host: string; onJumpTo?: (sub: string) => void }> = ({ C, host, onJumpTo }) => {
  type DriftSnap = {
    fresh_ratio?: number;
    stale_ratio?: number;
    hdc_cache_ratio?: number;
    contradictions_pending?: number;
    feedback_negative_ratio_24h?: number;
    fsrs_lapse_rate?: number;
    tuples_total?: number;
    // c2-433 / #354 forward-compat: share of facts with a resolved Lean4
    // verdict (Proved or Rejected). Unreachable/Unknown excluded since the
    // NO-OP semantic means they could still be anything. Higher = more
    // claims have been through the verifier.
    proof_verified_ratio?: number;
    // c2-433 / #397 forward-compat: Global Workspace memory footprint as
    // a fraction of the configured cap. Higher = closer to eviction
    // pressure. Shipped as a ratio so a 64MB and a 2GB cap render on the
    // same card.
    workspace_fill_ratio?: number;
  };
  // c2-433 / #284 followup: hydrate Drift history from localStorage on mount
  // so the trend survives a reload. Capped at 60 entries (1h @ 60s). Stale
  // entries older than 4h are dropped on load so the sparkline doesn't
  // show a week-old baseline flat-line.
  const hydratedHistory = React.useMemo<Array<{ t: number; snap: DriftSnap }>>(() => {
    try {
      const raw = localStorage.getItem('lfi_drift_history_v1');
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      const fresh = parsed.filter((e: any) => e && typeof e.t === 'number' && e.snap && typeof e.snap === 'object' && Date.now() - e.t < 4 * 60 * 60 * 1000);
      return fresh.slice(-60);
    } catch { return []; }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // c2-433 / #284 followup: seed `current` from the last persisted entry so
  // the cards render immediately on reopen instead of flashing dashes for
  // the first 60s. Overwritten by the first live fetch.
  const [current, setCurrent] = React.useState<DriftSnap | null>(
    hydratedHistory.length > 0 ? hydratedHistory[hydratedHistory.length - 1].snap : null
  );
  const [history, setHistory] = React.useState<Array<{ t: number; snap: DriftSnap }>>(hydratedHistory);
  const [err, setErr] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState<boolean>(false);
  const [lastFetched, setLastFetched] = React.useState<number | null>(null);
  // c2-433 / #284 followup: copy-feedback tick. Flips to the write-time
  // epoch when the user clicks Copy and self-clears 2s later so the
  // button label reverts from "Copied ✓" → "Copy".
  const [copiedAt, setCopiedAt] = React.useState<number>(0);

  // c2-433 / #284 followup: writeback to localStorage on every history
  // mutation. Silent on quota failure — the sparkline still renders from
  // the in-memory array.
  React.useEffect(() => {
    if (history.length === 0) return;
    try {
      localStorage.setItem('lfi_drift_history_v1', JSON.stringify(history));
    } catch { /* quota or incognito — silent */ }
  }, [history]);

  const load = React.useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const r = await fetch(`http://${host}:3000/api/drift/snapshot`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const snap: DriftSnap = await r.json();
      setCurrent(snap);
      setLastFetched(Date.now());
      setHistory(prev => {
        const next = [...prev, { t: Date.now(), snap }];
        return next.length > 60 ? next.slice(next.length - 60) : next;
      });
    } catch (e: any) {
      setErr(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  }, [host]);
  React.useEffect(() => {
    load();
    const id = window.setInterval(load, 60_000);
    return () => window.clearInterval(id);
  }, [load]);

  // c2-433 / #284: per-metric health rules. Higher/lower is better varies
  // by metric, so each carries a ratingFn that maps value → color tier.
  // Formatter produces the big display label.
  type MetricSpec = {
    key: keyof DriftSnap;
    label: string;
    format: (v: number | undefined) => string;
    rating: (v: number | undefined) => string; // returns a C.* palette key
    isPct: boolean; // percent vs count
    hint: string;
    // c2-433 / #284 followup: destination sub-tab when the card is clicked.
    // Only set for metrics with a natural triage target; other cards stay
    // non-interactive (no cursor pointer, no button affordance).
    jumpTo?: string;
    // c2-433 / #284 followup #2: polarity — 'up' means higher is better,
    // 'down' means lower is better. Drives the color of the trend arrow
    // so an improving metric always renders green regardless of direction.
    polarity: 'up' | 'down';
    // c2-433 / #284 followup #3: rating thresholds surfaced in the tooltip
    // so users know WHY a metric is yellow vs red. Format is a compact
    // human string (e.g. "green ≥60% · yellow ≥30% · red below"). Optional
    // — the SPO tuples card has no thresholds since it's an absolute count.
    thresholds?: string;
  };
  const METRICS: MetricSpec[] = [
    {
      key: 'fresh_ratio', label: 'Fresh facts', isPct: true, polarity: 'up',
      hint: 'Share of facts with a recent upsert. Higher = substrate is learning.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v >= 0.6 ? C.green : v >= 0.3 ? C.yellow : C.red,
      thresholds: 'green ≥60% · yellow ≥30% · red below',
    },
    {
      key: 'stale_ratio', label: 'Stale facts', isPct: true, polarity: 'down',
      hint: 'Share of facts unchanged for a long time. Lower = fresher corpus.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v <= 0.3 ? C.green : v <= 0.6 ? C.yellow : C.red,
      thresholds: 'green ≤30% · yellow ≤60% · red above',
    },
    {
      key: 'hdc_cache_ratio', label: 'HDC cache', isPct: true, polarity: 'up',
      hint: 'Encode-cache hit ratio. Higher = less recompute per query.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v >= 0.8 ? C.green : v >= 0.4 ? C.yellow : C.red,
      thresholds: 'green ≥80% · yellow ≥40% · red below',
    },
    {
      key: 'contradictions_pending', label: 'Contradictions', isPct: false, polarity: 'down',
      hint: 'Open rows in the contradiction ledger. Click to jump to the Ledger.',
      format: v => v == null ? '—' : String(v),
      rating: v => v == null ? C.textMuted : v === 0 ? C.green : v <= 5 ? C.yellow : C.red,
      jumpTo: 'ledger',
      thresholds: 'green 0 · yellow ≤5 · red above',
    },
    {
      key: 'feedback_negative_ratio_24h', label: 'Neg feedback 24h', isPct: true, polarity: 'down',
      hint: 'Share of last-24h user feedback that was thumbs-down. Click to jump to Office Hours.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v <= 0.1 ? C.green : v <= 0.25 ? C.yellow : C.red,
      jumpTo: 'office',
      thresholds: 'green ≤10% · yellow ≤25% · red above',
    },
    {
      key: 'fsrs_lapse_rate', label: 'FSRS lapse', isPct: true, polarity: 'down',
      hint: 'Share of FSRS cards graded Again. Lower = retention is holding.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v <= 0.15 ? C.green : v <= 0.3 ? C.yellow : C.red,
      thresholds: 'green ≤15% · yellow ≤30% · red above',
    },
    {
      // c2-433 / #329 + #284 followup: SPO tuples card. Absolute count of
      // subject-predicate-object tuples extracted by /api/tuples/extract.
      // No threshold tiers — it's a corpus-size indicator, always accent-
      // colored (healthy-ish regardless of magnitude). Polarity up so the
      // trend arrow goes green when more tuples accrue between samples.
      // jumpTo: 'runs' makes the card a one-click path to the Extract Now
      // button — matches the contradictions → ledger / neg-feedback →
      // office deep-link pattern.
      key: 'tuples_total', label: 'SPO tuples', isPct: false, polarity: 'up',
      hint: 'Total subject-predicate-object tuples extracted (IsA, Causes, UsedFor, HasA, HasPrerequisite, PartOf). Click to jump to Runs where Extract Now lives.',
      format: v => v == null ? '—' : v.toLocaleString(),
      rating: v => v == null ? C.textMuted : C.accent,
      jumpTo: 'runs',
    },
    {
      // c2-433 / #354 forward-compat: proof coverage. Share of facts with
      // a resolved (Proved or Rejected) Lean4 verdict. Lights up the
      // moment /api/drift/snapshot starts emitting proof_verified_ratio.
      key: 'proof_verified_ratio', label: 'Proof coverage', isPct: true, polarity: 'up',
      hint: 'Share of facts with a resolved Lean4 verdict (Proved or Rejected). Unreachable/Unknown excluded. Higher = more claims verified. No jumpTo — drill in via Admin → Proof.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v >= 0.5 ? C.green : v >= 0.2 ? C.yellow : C.red,
      thresholds: 'green ≥50% · yellow ≥20% · red below',
    },
    {
      // c2-433 / #397 forward-compat: workspace fill ratio. Memory
      // footprint / configured cap. High ratio = eviction pressure. If
      // this stays > 80% for a while operators should bump the cap via
      // Settings → Behavior → Workspace capacity.
      key: 'workspace_fill_ratio', label: 'Workspace fill', isPct: true, polarity: 'down',
      hint: 'Global Workspace footprint as a fraction of the configured memory cap. High = eviction pressure; bump the cap in Settings → Behavior.',
      format: v => v == null ? '—' : `${(v * 100).toFixed(0)}%`,
      rating: v => v == null ? C.textMuted : v <= 0.4 ? C.green : v <= 0.75 ? C.yellow : C.red,
      thresholds: 'green ≤40% · yellow ≤75% · red above',
    },
  ];

  const seriesFor = (k: keyof DriftSnap): number[] =>
    history.map(h => h.snap[k]).filter((v): v is number => typeof v === 'number');

  // c2-433 / #284 followup #2: trend computation. Compare latest sample
  // vs the one ~5 samples (= ~5 min at the 60s cadence) back so the arrow
  // reflects recent direction, not the hour-long drift. Null when we
  // don't have enough history to compare.
  const trendFor = (spec: MetricSpec): { dir: 'up' | 'down' | 'flat'; deltaPct: number; improving: boolean } | null => {
    const s = seriesFor(spec.key);
    if (s.length < 2) return null;
    const latest = s[s.length - 1];
    const earlier = s[Math.max(0, s.length - 6)];
    const delta = latest - earlier;
    const base = Math.abs(earlier) < 1e-6 ? Math.max(Math.abs(latest), 1e-6) : Math.abs(earlier);
    const deltaPct = (delta / base) * 100;
    // Flat threshold: < 2% change relative to base.
    if (Math.abs(deltaPct) < 2) return { dir: 'flat', deltaPct, improving: true };
    const dir: 'up' | 'down' = delta > 0 ? 'up' : 'down';
    // Improving = direction matches the better-polarity for this metric.
    const improving = dir === spec.polarity;
    return { dir, deltaPct, improving };
  };

  return (
    <div style={{ padding: T.spacing.xl, display: 'flex', flexDirection: 'column', gap: T.spacing.md, minWidth: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, flexWrap: 'wrap' }}>
        <h2 style={{ margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>
          Drift snapshot
          <span style={{ marginLeft: T.spacing.sm, color: C.textMuted, fontSize: T.typography.sizeSm, fontWeight: 500 }}>
            every 60s · {history.length} sample{history.length === 1 ? '' : 's'}
          </span>
        </h2>
        {/* c2-433 / #284 followup: at-a-glance health summary. Tallies the
            per-metric rating into green / yellow / red counts, renders a
            color-tiered chip. Tier is driven by the worst bucket: any red
            → red chip, any yellow → yellow chip, all green → green chip.
            Hidden until at least one metric has a value (avoids the chip
            showing 0/6 healthy on mount). */}
        {current && (() => {
          let healthy = 0, warn = 0, crit = 0;
          for (const m of METRICS) {
            const v = current[m.key];
            const rc = m.rating(v);
            if (rc === C.green) healthy++;
            else if (rc === C.yellow) warn++;
            else if (rc === C.red) crit++;
          }
          const rated = healthy + warn + crit;
          if (rated === 0) return null;
          const tierColor = crit > 0 ? C.red : warn > 0 ? C.yellow : C.green;
          const tierBg = crit > 0 ? C.redBg : warn > 0 ? (C.yellowBg || `${C.yellow}18`) : (C.greenBg || `${C.green}18`);
          const label = crit > 0 || warn > 0
            ? `${healthy}/${rated} healthy · ${warn} warn · ${crit} crit`
            : `${healthy}/${rated} healthy`;
          return (
            <span role='status'
              title={`System health across 6 drift metrics — ${label}`}
              style={{
                display: 'inline-flex', alignItems: 'center', gap: '6px',
                padding: '3px 10px', fontSize: T.typography.sizeXs,
                fontWeight: T.typography.weightBold,
                background: tierBg, color: tierColor,
                border: `1px solid ${tierColor}55`,
                borderRadius: T.radii.pill,
                fontFamily: T.typography.fontMono,
                letterSpacing: '0.04em',
              }}>
              <span style={{
                width: '6px', height: '6px', borderRadius: '50%',
                background: tierColor, flexShrink: 0,
              }} />
              {label}
            </span>
          );
        })()}
        <div style={{ flex: 1 }} />
        {lastFetched != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            {formatRelative(lastFetched)}
          </span>
        )}
        {/* c2-433 / #284 followup: copy-JSON button. Serializes
            {exported_at, current, history} to clipboard for external
            trend analysis (excel, log diff, etc). Disabled when there's
            nothing to copy. */}
        <button
          disabled={!current && history.length === 0}
          onClick={async () => {
            const payload = {
              exported_at: new Date().toISOString(),
              current,
              history,
            };
            try {
              await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
              setCopiedAt(Date.now());
              window.setTimeout(() => setCopiedAt(0), 2000);
            } catch { /* clipboard blocked */ }
          }}
          title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${history.length} sample${history.length === 1 ? '' : 's'} + current snapshot as JSON`}
          style={{
            background: copiedAt > 0 ? `${C.green}18` : 'transparent',
            border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
            color: copiedAt > 0 ? C.green : C.textMuted,
            borderRadius: T.radii.sm,
            cursor: (!current && history.length === 0) ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: (!current && history.length === 0) ? 0.5 : 1,
          }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
        {/* c2-433 / #284 followup: clear session history + localStorage.
            Native confirm since this is destructive (loses sparkline
            trend). Hidden when history is empty. */}
        {history.length > 0 && (
          <button onClick={() => {
            if (!window.confirm(`Clear ${history.length} drift sample${history.length === 1 ? '' : 's'} from session + localStorage? The sparklines will reset.`)) return;
            try { localStorage.removeItem('lfi_drift_history_v1'); } catch { /* quota */ }
            setHistory([]);
          }}
            title={`Clear ${history.length} session sample${history.length === 1 ? '' : 's'} (sparklines reset)`}
            style={{
              background: 'transparent', border: `1px solid ${C.borderSubtle}`,
              color: C.textMuted, borderRadius: T.radii.sm,
              cursor: 'pointer',
              padding: '4px 10px', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            }}>Clear</button>
        )}
        <button onClick={load} disabled={loading}
          title={loading ? 'Refreshing…' : 'Refresh now'}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
          }}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      </div>

      {err && (
        <div role='alert' style={{
          padding: T.spacing.md, background: C.redBg,
          border: `1px solid ${C.redBorder}`, color: C.red,
          borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
        }}>Could not load drift snapshot: {err}. The endpoint <code style={{ fontFamily: T.typography.fontMono }}>/api/drift/snapshot</code> may not be exposed yet.</div>
      )}
      {/* claude-0 13:12 ask: 'Server won't start' inline runbook. Mirrors
          manager_guide.md §10.5 — shown when the drift fetch errored OR
          when history has zero samples after ≥1 attempt (likely server
          down). Click-to-expand so the default state is a single chip. */}
      {(err || (lastFetched != null && history.length === 0)) && (
        <ServerWontStartHelp C={C} />
      )}

      {/* claude-0 13:12 ask: one-click ops so solo-training users don't need
          to leave the Drift tab to push a corpus ingest, warm the HDC cache,
          or clean up the contradictions queue. Shared toast-based UX. */}
      <DriftQuickActions C={C} host={host} />

      <div style={{
        display: 'grid', gap: T.spacing.md,
        gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
      }}>
        {METRICS.map(m => {
          const v = current ? current[m.key] : undefined;
          const color = m.rating(v);
          const series = seriesFor(m.key);
          const canJump = !!(onJumpTo && m.jumpTo);
          const trend = trendFor(m);
          // c2-433 / #284 followup #2: trend arrow. Green when improving
          // (direction matches polarity), red when regressing, textDim
          // when flat (< 2% delta). Arrow glyph: ▲/▼/─ — compact enough
          // to sit in the label row next to the jump hint.
          const trendArrow = trend && (
            <span title={`${trend.dir === 'flat' ? 'Flat' : trend.improving ? 'Improving' : 'Worsening'} — ${trend.deltaPct >= 0 ? '+' : ''}${trend.deltaPct.toFixed(1)}% vs ~5 min ago`}
              style={{
                color: trend.dir === 'flat' ? C.textDim
                  : trend.improving ? C.green : C.red,
                fontSize: '10px', fontFamily: T.typography.fontMono,
                fontWeight: 800, letterSpacing: 0,
              }}>
              {trend.dir === 'up' ? '▲' : trend.dir === 'down' ? '▼' : '─'}
              {trend.dir !== 'flat' && <span style={{ marginLeft: '2px' }}>{Math.abs(trend.deltaPct).toFixed(0)}%</span>}
            </span>
          );
          const body = (
            <>
              <div style={{
                fontSize: '10px', color: C.textMuted,
                fontWeight: T.typography.weightSemibold,
                textTransform: 'uppercase', letterSpacing: '0.08em',
                display: 'flex', alignItems: 'center', gap: '6px',
              }}>
                <span style={{ flex: 1, textAlign: 'left' }}>{m.label}</span>
                {trendArrow}
                {canJump && <span aria-hidden='true' style={{
                  color: C.textDim, fontSize: '10px',
                }}>→</span>}
              </div>
              <div style={{
                display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                gap: T.spacing.sm,
              }}>
                <span style={{
                  fontSize: T.typography.sizeXxl || T.typography.sizeXl,
                  fontWeight: T.typography.weightBlack,
                  color,
                  fontFamily: T.typography.fontMono,
                }}>{m.format(v)}</span>
                {series.length >= 2 && (
                  <Sparkline values={series} color={color} width={72} height={20} />
                )}
              </div>
            </>
          );
          const commonStyle = {
            padding: T.spacing.md, borderRadius: T.radii.lg,
            background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
            display: 'flex', flexDirection: 'column' as const, gap: '6px',
            textAlign: 'left' as const, fontFamily: 'inherit',
            color: C.text,
          };
          // c2-433 / #284 followup #3: append threshold legend to the
          // tooltip when the metric has explicit boundaries, so hover
          // reveals why a value renders green/yellow/red.
          const fullTitle = m.thresholds ? `${m.hint}\n\nThresholds: ${m.thresholds}` : m.hint;
          return canJump ? (
            <button key={m.key} type='button' title={fullTitle}
              onClick={() => onJumpTo!(m.jumpTo!)}
              aria-label={`${m.label} — click to jump to ${m.jumpTo}`}
              style={{
                ...commonStyle,
                cursor: 'pointer',
              }}>
              {body}
            </button>
          ) : (
            <div key={m.key} title={fullTitle}
              style={commonStyle}>
              {body}
            </div>
          );
        })}
      </div>
    </div>
  );
};

// c2-433 / #312: Ingest runs tab. Consumes /api/ingest/list (running-first
// ordering guaranteed by backend per Claude 0's 04:25 spec). Polls every
// 10s while active; renders two groups — Running (accent border, live
// progress bars) and Finished (muted, status pill). Tolerant payload
// shape: array of rows or {runs|items|list: [...]} wrappers; per-row
// tolerant on run_id/started_at/progress/total/status fields.
const IngestRunsTab: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  const [runs, setRuns] = React.useState<any[] | null>(null);
  const [err, setErr] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState<boolean>(false);
  const [lastFetched, setLastFetched] = React.useState<number | null>(null);
  // c2-433 / #312 followup: filter input. Narrows both Running and Finished
  // sections by case-insensitive substring match on run_id / source / status.
  // Empty string == no filtering. Esc-clears-step-down pattern (consistent
  // with other search-bearing surfaces across the app).
  const [filterQ, setFilterQ] = React.useState<string>('');
  // c2-433 / #308: domain-gap scheduler — thinnest domains to ingest next.
  // Silent-fail so an unavailable endpoint just hides the panel.
  const [gaps, setGaps] = React.useState<any[] | null>(null);
  // c2-433 / #312 followup: Copy-to-clipboard feedback state. 2s label
  // flip matching the Drift + Ledger export pattern.
  const [copiedAt, setCopiedAt] = React.useState<number>(0);
  // c2-433 / #329: tuple-extraction counter + manual-run button state.
  // tupleCount tracks /api/tuples/count; extracting is true during the
  // POST /api/tuples/extract round-trip; extractMsg surfaces the +N added
  // or error result inline for ~4s.
  const [tupleCount, setTupleCount] = React.useState<number | null>(null);
  // c2-433: click-to-copy run_id flash state. Holds the id of the run
  // whose id was just copied; self-clears after 1.5s.
  const [copiedRunId, setCopiedRunId] = React.useState<string | null>(null);
  // c2-433: same pattern for Suggested Next domain click-copy.
  const [copiedDomain, setCopiedDomain] = React.useState<string | null>(null);
  const [extracting, setExtracting] = React.useState<boolean>(false);
  const [extractMsg, setExtractMsg] = React.useState<string | null>(null);

  const load = React.useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      // c2-433 / #308 + #329: fetch gaps + tuple count in parallel with
      // the list. Silent-fail on each so one endpoint outage doesn't
      // suppress the other panels.
      const [listRes, gapsRes, tcRes] = await Promise.all([
        fetch(`http://${host}:3000/api/ingest/list`),
        fetch(`http://${host}:3000/api/ingest/gaps?limit=10`).catch(() => null as Response | null),
        fetch(`http://${host}:3000/api/tuples/count`).catch(() => null as Response | null),
      ]);
      if (!listRes.ok) throw new Error(`HTTP ${listRes.status}`);
      const data = await listRes.json();
      const list: any[] = Array.isArray(data) ? data
        : Array.isArray(data?.runs) ? data.runs
        : Array.isArray(data?.items) ? data.items
        : Array.isArray(data?.list) ? data.list
        : [];
      setRuns(list);
      setLastFetched(Date.now());
      if (gapsRes && gapsRes.ok) {
        try {
          const gdata = await gapsRes.json();
          const glist: any[] = Array.isArray(gdata) ? gdata
            : Array.isArray(gdata?.gaps) ? gdata.gaps
            : Array.isArray(gdata?.items) ? gdata.items
            : Array.isArray(gdata?.domains) ? gdata.domains
            : [];
          setGaps(glist);
        } catch { /* parse error — leave gaps as-is */ }
      }
      // c2-433 / #329: tuples count is a plain number or {count:N}. Tolerant.
      if (tcRes && tcRes.ok) {
        try {
          const tdata = await tcRes.json();
          const n: number | null = typeof tdata === 'number' ? tdata
            : typeof tdata?.count === 'number' ? tdata.count
            : typeof tdata?.tuples === 'number' ? tdata.tuples
            : typeof tdata?.total === 'number' ? tdata.total : null;
          setTupleCount(n);
        } catch { /* leave tupleCount as-is */ }
      }
    } catch (e: any) {
      setErr(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  }, [host]);
  React.useEffect(() => {
    load();
    const id = window.setInterval(load, 10_000);
    return () => window.clearInterval(id);
  }, [load]);

  // c2-433 / #329: trigger tuple extraction. POST with limit=500 is the
  // canonical one-shot per Claude 0's spec. After success, refetch via
  // load() so the counter updates AND any new ingest progress is seen.
  const runExtract = async () => {
    setExtracting(true);
    setExtractMsg(null);
    const before = tupleCount ?? 0;
    try {
      const r = await fetch(`http://${host}:3000/api/tuples/extract`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ limit: 500 }),
      });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json().catch(() => ({}));
      const added: number | null = typeof data.added === 'number' ? data.added
        : typeof data.inserted === 'number' ? data.inserted
        : typeof data.new === 'number' ? data.new
        : typeof data.count === 'number' ? data.count - before : null;
      setExtractMsg(added != null ? `+${added.toLocaleString()} tuples` : `Extracted`);
      load();
    } catch (e: any) {
      setExtractMsg(`Extract failed: ${String(e?.message || e || 'unknown')}`);
    } finally {
      setExtracting(false);
      window.setTimeout(() => setExtractMsg(null), 4000);
    }
  };

  const statusOf = (r: any): string => String(r.status ?? r.state ?? '').toLowerCase();
  const isRunning = (r: any) => {
    const s = statusOf(r);
    return s === 'running' || s === 'active' || s === 'in_progress' || (!s && !r.finished_at);
  };
  const tsOf = (r: any, field: string): number | null => {
    const v = r[field];
    if (v == null) return null;
    if (typeof v === 'number') return v;
    const ms = Date.parse(String(v));
    return Number.isNaN(ms) ? null : ms;
  };
  const statusColor = (s: string): string => {
    if (s === 'running' || s === 'active' || s === 'in_progress') return C.accent;
    if (s === 'finished' || s === 'complete' || s === 'done' || s === 'success') return C.green;
    if (s === 'failed' || s === 'error') return C.red;
    if (s === 'cancelled' || s === 'canceled' || s === 'aborted') return C.yellow;
    return C.textMuted;
  };

  // c2-433 / #312 followup: apply filter before splitting so both groups
  // shrink together. Matches any of: run_id, source label, status string,
  // label/tag fields. Match is case-insensitive substring.
  const fLower = filterQ.trim().toLowerCase();
  const matchesFilter = (r: any): boolean => {
    if (!fLower) return true;
    const id = String(r.run_id ?? r.id ?? r.uuid ?? '').toLowerCase();
    const src = String(r.source ?? r.script ?? r.name ?? r.corpus ?? '').toLowerCase();
    const st = statusOf(r);
    const label = String(r.label ?? '').toLowerCase();
    return id.includes(fLower) || src.includes(fLower) || st.includes(fLower) || label.includes(fLower);
  };
  const allRunning: any[] = (runs || []).filter(isRunning);
  const allFinished: any[] = (runs || []).filter(r => !isRunning(r));
  const running: any[] = allRunning.filter(matchesFilter);
  const finished: any[] = allFinished.filter(matchesFilter);

  const renderRow = (r: any, i: number, group: 'running' | 'finished') => {
    const id: string = String(r.run_id ?? r.id ?? r.uuid ?? `run-${i}`);
    const source: string = r.source ?? r.script ?? r.name ?? r.corpus ?? '(unnamed)';
    const progress: number | null = typeof r.progress === 'number' ? r.progress : null;
    const total: number | null = typeof r.total === 'number' ? r.total : null;
    const ratio: number | null = (progress != null && total != null && total > 0)
      ? Math.max(0, Math.min(1, progress / total))
      : (typeof r.ratio === 'number' ? r.ratio
         : typeof r.percent === 'number' ? (r.percent > 1 ? r.percent / 100 : r.percent)
         : null);
    const startedAt = tsOf(r, 'started_at') ?? tsOf(r, 'start_ts') ?? tsOf(r, 'created_at');
    const finishedAt = tsOf(r, 'finished_at') ?? tsOf(r, 'end_ts') ?? tsOf(r, 'completed_at');
    const durMs = (startedAt && finishedAt) ? (finishedAt - startedAt)
      : (startedAt && group === 'running') ? (Date.now() - startedAt)
      : null;
    const durLabel = durMs != null ? formatRelative(Date.now() - durMs).replace(' ago', '') : null;
    const status = statusOf(r) || (group === 'running' ? 'running' : 'done');
    const sColor = statusColor(status);
    return (
      <div key={id}
        style={{
          padding: '10px 12px', borderRadius: T.radii.md,
          background: C.bgCard,
          border: `1px solid ${group === 'running' ? C.accentBorder : C.borderSubtle}`,
          display: 'flex', flexDirection: 'column', gap: '6px',
        }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: T.spacing.sm, flexWrap: 'wrap' }}>
          {/* c2-433: click run_id to copy the full value — saves ticket-
              filers from selecting around the truncation ellipsis. 1.5s
              green flash confirms the write. */}
          <button type='button'
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(id);
                setCopiedRunId(id);
                window.setTimeout(() => {
                  setCopiedRunId(prev => prev === id ? null : prev);
                }, 1500);
              } catch { /* clipboard blocked */ }
            }}
            title={copiedRunId === id ? `Copied ${id}` : `${id} — click to copy`}
            aria-label={copiedRunId === id ? `Copied run id ${id}` : `Copy run id ${id}`}
            style={{
              background: 'transparent', border: 'none',
              padding: 0, cursor: 'pointer',
              fontFamily: T.typography.fontMono, fontSize: T.typography.sizeXs,
              color: copiedRunId === id ? C.green : C.accent, fontWeight: 700,
              textDecoration: 'underline', textDecorationColor: `${copiedRunId === id ? C.green : C.accent}55`,
              textUnderlineOffset: '2px',
              transition: 'color 180ms',
            }}>{copiedRunId === id ? `copied ${id.slice(0, 18)}…` : (id.length > 24 ? id.slice(0, 24) + '…' : id)}</button>
          <span style={{
            fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
            color: C.text, flex: 1, minWidth: 0,
            overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
          }} title={source}>{source}</span>
          <span title={`Status: ${status}`}
            style={{
              fontSize: '9px', fontWeight: 800,
              color: sColor, background: `${sColor}18`,
              border: `1px solid ${sColor}55`,
              borderRadius: T.radii.sm, padding: '1px 6px',
              textTransform: 'uppercase', letterSpacing: '0.06em',
              fontFamily: T.typography.fontMono,
            }}>{status}</span>
        </div>
        {(ratio != null || progress != null) && (
          <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, minWidth: 0 }}>
            <div style={{
              flex: 1, height: '6px', background: C.bgInput,
              borderRadius: T.radii.xs, overflow: 'hidden',
            }}>
              <div style={{
                width: `${Math.round((ratio ?? 0) * 100)}%`, height: '100%',
                background: group === 'running' ? C.accent : C.green,
                transition: 'width 300ms',
              }} />
            </div>
            <span style={{
              fontFamily: T.typography.fontMono, fontSize: '10px',
              color: C.textSecondary, minWidth: '90px', textAlign: 'right',
            }}>{progress != null ? progress.toLocaleString() : '—'}{total != null ? ` / ${total.toLocaleString()}` : ''}{ratio != null ? ` · ${Math.round(ratio * 100)}%` : ''}</span>
          </div>
        )}
        <div style={{
          display: 'flex', gap: T.spacing.md, fontSize: '10px',
          fontFamily: T.typography.fontMono, color: C.textDim,
          flexWrap: 'wrap',
        }}>
          {startedAt && <span>started {formatRelative(startedAt)}</span>}
          {finishedAt && <span>finished {formatRelative(finishedAt)}</span>}
          {durLabel && group === 'running' && <span>running {durLabel}</span>}
          {durLabel && group === 'finished' && <span>took {durLabel}</span>}
        </div>
      </div>
    );
  };

  return (
    <div style={{ padding: T.spacing.xl, display: 'flex', flexDirection: 'column', gap: T.spacing.md, minWidth: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md, flexWrap: 'wrap' }}>
        <h2 style={{ margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>
          Ingest runs
          {runs != null && <span style={{ marginLeft: T.spacing.sm, color: C.textMuted, fontSize: T.typography.sizeMd, fontWeight: 500 }}>
            {fLower
              ? `(${running.length}/${allRunning.length} running · ${finished.length}/${allFinished.length} finished)`
              : `(${running.length} running · ${finished.length} finished)`}
          </span>}
        </h2>
        <div style={{ flex: 1 }} />
        {/* c2-433 / #329: tuples counter + Extract Now. Chip shows current
            tupleCount (SI-compact), click fires POST /api/tuples/extract
            {limit:500}. Inline message pill after for ~4s. Hidden when the
            count endpoint never returned (fresh install / endpoint not
            exposed) so the chip doesnt render with a dash. */}
        {tupleCount != null && (
          <>
            {extractMsg && (
              <span role='status' style={{
                fontSize: T.typography.sizeXs,
                color: extractMsg.startsWith('Extract failed') ? C.red : C.green,
                fontFamily: T.typography.fontMono,
              }}>{extractMsg}</span>
            )}
            <button onClick={runExtract} disabled={extracting}
              title={`${tupleCount.toLocaleString()} tuples extracted · click to run POST /api/tuples/extract {limit:500}`}
              style={{
                background: extracting ? C.bgInput : 'transparent',
                border: `1px solid ${C.borderSubtle}`,
                color: C.textMuted, borderRadius: T.radii.sm,
                cursor: extracting ? 'wait' : 'pointer',
                padding: '4px 10px', fontFamily: 'inherit',
                fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
                display: 'inline-flex', alignItems: 'center', gap: '6px',
              }}>
              <span style={{ color: C.accent, fontFamily: T.typography.fontMono }}>
                {compactNum(tupleCount)}
              </span>
              <span>{extracting ? 'Extracting…' : 'Extract'}</span>
            </button>
          </>
        )}
        {/* c2-433 / #312 followup: search input. Esc clears when non-empty,
            else propagates (closes parent modal / moves on). */}
        {runs != null && runs.length > 0 && (
          <input type='search' value={filterQ}
            onChange={(e) => setFilterQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape' && filterQ) {
                e.preventDefault();
                e.stopPropagation();
                setFilterQ('');
              }
            }}
            autoComplete='off' spellCheck={false}
            placeholder='Filter runs…'
            aria-label='Filter ingest runs by id, source, or status'
            style={{
              // c2-433 / mobile fix: flex-basis lets the filter shrink on
              // narrow viewports instead of forcing a fixed 180px which
              // pushed the Refresh button off-screen.
              padding: '4px 10px', flex: '1 1 140px', maxWidth: '220px',
              background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
              color: C.text, borderRadius: T.radii.sm,
              fontFamily: 'inherit', fontSize: T.typography.sizeXs,
              outline: 'none',
            }} />
        )}
        {lastFetched != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            {formatRelative(lastFetched)}
          </span>
        )}
        {/* c2-433 / #312 followup: export runs + gaps snapshot as JSON.
            Matches the Drift + Ledger Copy pattern. */}
        <button
          disabled={!runs || runs.length === 0}
          onClick={async () => {
            const payload = {
              exported_at: new Date().toISOString(),
              runs: runs || [],
              gaps: gaps || [],
            };
            try {
              await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
              setCopiedAt(Date.now());
              window.setTimeout(() => setCopiedAt(0), 2000);
            } catch { /* clipboard blocked */ }
          }}
          title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${runs?.length ?? 0} run${runs?.length === 1 ? '' : 's'} + ${gaps?.length ?? 0} suggested domains as JSON`}
          style={{
            background: copiedAt > 0 ? `${C.green}18` : 'transparent',
            border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
            color: copiedAt > 0 ? C.green : (runs && runs.length > 0 ? C.textMuted : C.textDim),
            borderRadius: T.radii.sm,
            cursor: (!runs || runs.length === 0) ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: (!runs || runs.length === 0) ? 0.5 : 1,
          }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
        <button onClick={load} disabled={loading}
          title={loading ? 'Refreshing…' : 'Refresh now (auto-refreshes every 10s)'}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
          }}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      </div>

      {err && (
        <div role='alert' style={{
          padding: T.spacing.md, background: C.redBg,
          border: `1px solid ${C.redBorder}`, color: C.red,
          borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
        }}>Could not load ingest runs: {err}. The endpoint <code style={{ fontFamily: T.typography.fontMono }}>/api/ingest/list</code> may not be exposed yet.</div>
      )}

      {runs != null && runs.length === 0 && !err && (
        <div style={{
          padding: T.spacing.xl, textAlign: 'center',
          color: C.textMuted, fontSize: T.typography.sizeSm, lineHeight: 1.55,
          background: C.bgCard, border: `1px dashed ${C.borderSubtle}`, borderRadius: T.radii.lg,
        }}>
          No ingest runs yet.<br/>
          <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>
            Python ingest scripts register runs via <code style={{ fontFamily: T.typography.fontMono }}>POST /api/ingest/start</code> and stream progress — entries land here the moment one begins.
          </span>
        </div>
      )}

      {/* c2-433 / #312 followup: filter-zero-state. Differentiates "no runs
          at all" from "no runs match the filter." */}
      {runs != null && runs.length > 0 && fLower && running.length === 0 && finished.length === 0 && (
        <div style={{
          padding: T.spacing.lg, textAlign: 'center',
          color: C.textMuted, fontSize: T.typography.sizeSm, fontStyle: 'italic',
          background: C.bgCard, border: `1px dashed ${C.borderSubtle}`, borderRadius: T.radii.lg,
        }}>
          No runs match "{filterQ}". <button onClick={() => setFilterQ('')}
            style={{
              background: 'transparent', border: 'none', color: C.accent,
              cursor: 'pointer', fontFamily: 'inherit', textDecoration: 'underline',
              padding: 0, fontSize: T.typography.sizeSm,
            }}>Clear filter.</button>
        </div>
      )}

      {/* c2-433 / #308: Suggested Next — thinnest domains by composite gap
          score. Helps operators pick what to ingest next. Hidden when
          empty (fresh install) or when the endpoint didn't respond. Row
          colors: higher gap_score = redder (more under-served). */}
      {gaps && gaps.length > 0 && (
        <section aria-label='Suggested next ingests'>
          <div style={{
            display: 'flex', alignItems: 'baseline', gap: '6px',
            fontSize: '10px', fontWeight: 700, color: C.textMuted,
            textTransform: 'uppercase', letterSpacing: '0.1em',
            marginBottom: '6px',
          }}>
            <span>Suggested next ({gaps.length})</span>
            <span title='Thinnest domains: 1/ln(fact_count+1) − recent_7d/10k'
              style={{
                color: C.textDim, fontFamily: T.typography.fontMono,
                textTransform: 'none', letterSpacing: 0,
                fontWeight: 500,
              }}>· by gap score</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {gaps.map((g: any, i: number) => {
              const domain: string = g.domain ?? g.name ?? '(unknown)';
              const fc = typeof g.fact_count === 'number' ? g.fact_count
                : typeof g.facts === 'number' ? g.facts : null;
              const recent = typeof g.recent_ingest_7d === 'number' ? g.recent_ingest_7d
                : typeof g.recent_7d === 'number' ? g.recent_7d : null;
              const score = typeof g.gap_score === 'number' ? g.gap_score
                : typeof g.score === 'number' ? g.score : null;
              const scoreColor = score == null ? C.textMuted
                : score >= 0.8 ? C.red
                : score >= 0.5 ? C.yellow
                : C.green;
              return (
                <div key={`${domain}-${i}`} style={{
                  display: 'flex', alignItems: 'center',
                  gap: T.spacing.sm, padding: '7px 12px',
                  background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                  borderRadius: T.radii.md, minWidth: 0,
                }}>
                  <span style={{
                    fontSize: '10px', color: C.textMuted,
                    fontFamily: T.typography.fontMono, fontWeight: T.typography.weightBold,
                    minWidth: '22px', textAlign: 'right',
                  }}>#{i + 1}</span>
                  {/* c2-433: click the domain to copy — operators paste
                      into batch-ingest configs. 1.5s green flash. */}
                  <button type='button'
                    onClick={async () => {
                      try {
                        await navigator.clipboard.writeText(domain);
                        setCopiedDomain(domain);
                        window.setTimeout(() => {
                          setCopiedDomain(prev => prev === domain ? null : prev);
                        }, 1500);
                      } catch { /* clipboard blocked */ }
                    }}
                    title={copiedDomain === domain ? `Copied ${domain}` : `${domain} — click to copy`}
                    aria-label={copiedDomain === domain ? `Copied domain ${domain}` : `Copy domain ${domain}`}
                    style={{
                      flex: 1, minWidth: 0, overflow: 'hidden',
                      textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      color: copiedDomain === domain ? C.green : C.text,
                      fontFamily: T.typography.fontMono,
                      fontSize: T.typography.sizeSm,
                      background: 'transparent', border: 'none', padding: 0,
                      cursor: 'pointer', textAlign: 'left',
                      textDecoration: 'underline',
                      textDecorationColor: `${copiedDomain === domain ? C.green : C.text}22`,
                      textUnderlineOffset: '2px',
                      transition: 'color 180ms',
                    }}>{copiedDomain === domain ? `copied ${domain}` : domain}</button>
                  <span style={{
                    display: 'flex', gap: '10px', alignItems: 'center',
                    flexShrink: 0, fontSize: '10px',
                    fontFamily: T.typography.fontMono, color: C.textDim,
                  }}>
                    {fc != null && <span title={`${fc.toLocaleString()} total facts`}>{fc.toLocaleString()}f</span>}
                    {recent != null && <span title={`${recent.toLocaleString()} facts in the last 7d`}>7d:{recent.toLocaleString()}</span>}
                  </span>
                  <span style={{
                    minWidth: '60px', textAlign: 'right',
                    fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
                    fontWeight: T.typography.weightBlack, color: scoreColor,
                  }} title={`Gap score ${score != null ? score.toFixed(3) : '—'} (higher = thinner domain)`}>
                    {score != null ? score.toFixed(3) : '—'}
                  </span>
                </div>
              );
            })}
          </div>
        </section>
      )}

      {running.length > 0 && (
        <section aria-label='Running ingests'>
          <div style={{
            fontSize: '10px', fontWeight: 700, color: C.accent,
            textTransform: 'uppercase', letterSpacing: '0.1em',
            marginBottom: '6px',
          }}>Running ({running.length})</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {running.map((r, i) => renderRow(r, i, 'running'))}
          </div>
        </section>
      )}
      {finished.length > 0 && (
        <section aria-label='Finished ingests'>
          <div style={{
            fontSize: '10px', fontWeight: 700, color: C.textMuted,
            textTransform: 'uppercase', letterSpacing: '0.1em',
            marginBottom: '6px', marginTop: running.length > 0 ? T.spacing.md : 0,
          }}>Finished ({finished.length})</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {finished.map((r, i) => renderRow(r, i, 'finished'))}
          </div>
        </section>
      )}
    </div>
  );
};

// claude-0 13:12 ask: 'Server won't start' inline runbook. Mirrors
// manager_guide.md §10.5 so users have the recovery steps on the same
// page where they see the drift-fetch failure. Collapsed by default.
const ServerWontStartHelp: React.FC<{ C: any }> = ({ C }) => {
  const [open, setOpen] = React.useState(false);
  return (
    <div style={{
      padding: T.spacing.sm + ' ' + T.spacing.md,
      background: (C.yellowBg || `${C.yellow}18`),
      border: `1px solid ${C.yellow}55`,
      borderRadius: T.radii.md,
      fontSize: T.typography.sizeSm,
    }}>
      <button onClick={() => setOpen(v => !v)}
        aria-expanded={open}
        style={{
          background: 'transparent', border: 'none', color: C.yellow,
          fontFamily: 'inherit', fontWeight: T.typography.weightBold,
          cursor: 'pointer', padding: 0, textAlign: 'left',
          fontSize: T.typography.sizeSm,
        }}>
        {open ? '▾' : '▸'} Server won't start? — recovery runbook
      </button>
      {open && (
        <div style={{ marginTop: T.spacing.sm, color: C.text, lineHeight: 1.55 }}>
          <p style={{ margin: `0 0 ${T.spacing.sm}`, color: C.textMuted }}>
            Run these from a shell. Full reference: <code style={{ fontFamily: T.typography.fontMono }}>docs/manager_guide.md</code> §10.5 or the Docs tab in Admin.
          </p>
          <ol style={{ margin: 0, paddingLeft: '20px' }}>
            <li><strong>Service status (authoritative — claude-0 13:55 ship):</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>systemctl status plausiden-server</code>
              — <span style={{ color: C.textMuted }}>shows active/failed + last log lines. Restart=always so it auto-retries every 5s on crash.</span>
            </li>
            <li style={{ marginTop: T.spacing.sm }}><strong>Hit the health endpoint directly:</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>curl -sS http://localhost:3000/api/health | head</code>
            </li>
            <li style={{ marginTop: T.spacing.sm }}><strong>Which process holds :3000?</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>ss -ltnp 'sport = :3000'</code>
              — <span style={{ color: C.textMuted }}>expect systemd-managed plausiden-server; any other PID (stale nohup) should be killed.</span>
            </li>
            <li style={{ marginTop: T.spacing.sm }}><strong>Tail the service log for the failure reason:</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>journalctl -u plausiden-server --since '10 min ago' --no-pager | tail -60</code>
            </li>
            <li style={{ marginTop: T.spacing.sm }}><strong>brain.db locked (WAL checkpoint hung)?</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>fuser ~/LFI-data/brain.db</code>
              — kill the holder, then restart the service (next step).
            </li>
            <li style={{ marginTop: T.spacing.sm }}><strong>Restart:</strong><br />
              <code style={{ fontFamily: T.typography.fontMono, background: C.bgInput, padding: '1px 6px', borderRadius: 3 }}>systemctl restart plausiden-server</code>
              — <span style={{ color: C.textMuted }}>canonical path; the old nohup pattern should not recur.</span>
            </li>
          </ol>
        </div>
      )}
    </div>
  );
};

// claude-0 13:12 ask: three one-click ops on the Drift tab so solo-training
// users can run the most common maintenance from one surface. Each button
// POSTs a known endpoint, shows inline status, and surfaces errors in a
// scoped message without taking down the tab.
const DriftQuickActions: React.FC<{ C: any; host: string }> = ({ C, host }) => {
  type Op = 'ingest' | 'encode' | 'resolve';
  const [running, setRunning] = React.useState<Op | null>(null);
  const [message, setMessage] = React.useState<{ op: Op; text: string; ok: boolean } | null>(null);
  const run = async (op: Op) => {
    setRunning(op); setMessage(null);
    const paths: Record<Op, string> = {
      ingest: '/api/ingest/start',
      encode: '/api/hdc/cache/encode',
      resolve: '/api/contradictions/auto-resolve',
    };
    try {
      const r = await fetch(`http://${host}:3000${paths[op]}`, { method: 'POST' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const d: any = await r.json().catch(() => ({}));
      const summary = op === 'ingest'
        ? `Ingest kicked — run_id ${d?.run_id || d?.id || 'pending'}`
        : op === 'encode'
          ? `HDC cache encode started — ${d?.queued ?? d?.count ?? '?'} tuples queued`
          : `Auto-resolved ${d?.resolved ?? d?.count ?? '?'} contradictions`;
      setMessage({ op, text: summary, ok: true });
    } catch (e: any) {
      setMessage({ op, text: `Failed: ${String(e?.message || e || 'unknown')}`, ok: false });
    } finally {
      setRunning(null);
    }
  };
  const btn = (op: Op, label: string, hint: string) => {
    const isRun = running === op;
    const anyRun = running !== null;
    return (
      <button onClick={() => run(op)} disabled={anyRun}
        title={hint}
        style={{
          flex: '1 1 160px', minWidth: 0,
          padding: '10px 14px',
          background: isRun ? C.accentBg : C.bgCard,
          color: isRun ? C.accent : C.text,
          border: `1px solid ${isRun ? C.accent : C.borderSubtle}`,
          borderRadius: T.radii.md,
          fontFamily: 'inherit',
          fontSize: T.typography.sizeSm,
          fontWeight: T.typography.weightBold,
          cursor: anyRun ? (isRun ? 'wait' : 'not-allowed') : 'pointer',
          textAlign: 'left', lineHeight: 1.3,
          opacity: anyRun && !isRun ? 0.5 : 1,
        }}>
        <div style={{ fontWeight: T.typography.weightBlack, marginBottom: 2 }}>
          {isRun ? `${label}…` : label}
        </div>
        <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 500 }}>{hint}</div>
      </button>
    );
  };
  return (
    <div style={{
      padding: T.spacing.md,
      border: `1px solid ${C.borderSubtle}`,
      borderRadius: T.radii.md,
      background: C.bgInput,
    }}>
      <div style={{
        fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
        color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em',
        marginBottom: T.spacing.sm,
      }}>Quick actions</div>
      <div style={{ display: 'flex', gap: T.spacing.sm, flexWrap: 'wrap' }}>
        {btn('ingest', 'Kick ingest', 'POST /api/ingest/start — corpus run')}
        {btn('encode', 'Encode HDC cache', 'POST /api/hdc/cache/encode — warm embeddings')}
        {btn('resolve', 'Auto-resolve ledger', 'POST /api/contradictions/auto-resolve')}
      </div>
      {message && (
        <div role='status' style={{
          marginTop: T.spacing.sm, padding: '6px 10px',
          background: message.ok ? (C.greenBg || `${C.green}18`) : C.redBg,
          color: message.ok ? C.green : C.red,
          border: `1px solid ${message.ok ? C.green : C.red}55`,
          borderRadius: T.radii.sm,
          fontSize: T.typography.sizeXs, lineHeight: 1.4,
        }}>{message.text}</div>
      )}
    </div>
  );
};
