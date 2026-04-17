import React, { useEffect, useState, useMemo } from 'react';
import { T } from './tokens';
import { compactNum } from './util';

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

export const ClassroomView: React.FC<ClassroomViewProps> = ({ C, host, isDesktop, localEvents = [] }) => {
  const [sub, setSub] = useState<Sub>('profile');
  const [data, setData] = useState<DashboardShape | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  // c2-231 / #75: rolling history of per-domain counts, surfaced as
  // sparklines next to the coverage bars.
  const [history, setHistory] = useState<GradebookSnapshot[]>(() => loadGradebookHistory());

  const load = async () => {
    setLoading(true);
    setErr(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const res = await fetch(`http://${host}:3000/api/admin/dashboard`, { signal: ctrl.signal });
      clearTimeout(to);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setData(await res.json());
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
      <div role='tablist' aria-label='Classroom sections'
        onKeyDown={(e) => {
          const idx = SUBS.findIndex(s => s.id === sub);
          if (idx < 0) return;
          if (e.key === 'ArrowRight') { e.preventDefault(); setSub(SUBS[(idx + 1) % SUBS.length].id); }
          else if (e.key === 'ArrowLeft') { e.preventDefault(); setSub(SUBS[(idx - 1 + SUBS.length) % SUBS.length].id); }
          else if (e.key === 'Home') { e.preventDefault(); setSub(SUBS[0].id); }
          else if (e.key === 'End') { e.preventDefault(); setSub(SUBS[SUBS.length - 1].id); }
        }}
        style={{
          display: 'flex', gap: '2px', padding: '0 24px',
          borderBottom: `1px solid ${C.borderSubtle}`, overflowX: 'auto',
          background: C.bgCard,
        }}>
        {SUBS.map(s => (
          <button key={s.id} onClick={() => setSub(s.id)}
            role='tab' aria-selected={sub === s.id} title={s.hint}
            tabIndex={sub === s.id ? 0 : -1}
            style={{
              padding: '14px 16px', fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
              background: 'transparent', border: 'none', cursor: 'pointer',
              color: sub === s.id ? C.accent : C.textMuted,
              borderBottom: `2px solid ${sub === s.id ? C.accent : 'transparent'}`,
              marginBottom: '-1px', fontFamily: 'inherit', whiteSpace: 'nowrap',
            }}>{s.label}</button>
        ))}
        {/* c2-259 / #121: manual refresh pushed to the right margin. Tabs
            driven by fresh data (profile/curriculum/gradebook/lessons/reports)
            already auto-poll at 10s but users want a force-reload after a
            backend action. Spinner while load in-flight. */}
        <div style={{ flex: 1 }} />
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
      </div>

      {/* Body */}
      <div role='tabpanel' aria-label={sub}
        style={{ flex: 1, overflowY: 'auto', padding: '24px', maxWidth: '1200px', width: '100%', margin: '0 auto' }}>
        {err && (
          <div role='alert' style={{
            padding: '12px 14px', marginBottom: T.spacing.lg,
            background: C.redBg, border: `1px solid ${C.redBorder}`,
            color: C.red, borderRadius: T.radii.md, fontSize: T.typography.sizeMd,
          }}><strong>Could not load:</strong> {err}</div>
        )}

        {/* --- Student Profile --- */}
        {sub === 'profile' && (
          <div>
            {/* Skeleton on first load (no cached data) — silent on subsequent
                auto-refreshes so the grade doesn't re-skeleton every 10s. */}
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{ textAlign: 'center', marginBottom: T.spacing.xl }}>
                <div style={{
                  width: isDesktop ? '180px' : '140px', height: isDesktop ? '128px' : '96px',
                  margin: '0 auto', borderRadius: T.radii.lg,
                  background: `linear-gradient(90deg, ${C.bgCard} 0%, ${C.bgHover} 50%, ${C.bgCard} 100%)`,
                  backgroundSize: '200% 100%', animation: 'scc-skel-cls 1.3s ease-in-out infinite',
                }} />
                <style>{`@keyframes scc-skel-cls { 0% { background-position: 200% 0 } 100% { background-position: -200% 0 } }`}</style>
              </div>
            )}
            <div style={{ textAlign: 'center', marginBottom: T.spacing.xl, display: loading && !data ? 'none' : 'block' }}>
              <div style={{ fontSize: '11px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                Accuracy grade
              </div>
              <div style={{
                fontSize: isDesktop ? '128px' : '96px', fontWeight: T.typography.weightBlack,
                color: gradeColor(C, data?.score?.grade),
                lineHeight: 1, marginTop: '8px',
                fontFamily: 'ui-monospace, SFMono-Regular, monospace',
              }}>{data?.score?.grade || (loading ? '…' : '—')}</div>
              {typeof data?.score?.accuracy_score === 'number' && (
                <div style={{ fontSize: '15px', color: C.textSecondary, marginTop: '6px', fontFamily: 'ui-monospace, monospace' }}>
                  {data.score.accuracy_score.toFixed(1)} / 100
                </div>
              )}
            </div>
            {data?.score?.breakdown && (
              <div style={{
                maxWidth: '640px', margin: '0 auto', padding: T.spacing.lg,
                background: C.bgCard, border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
              }}>
                <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                  Strengths &amp; weaknesses
                </div>
                {(['quality', 'adversarial', 'coverage', 'training'] as const).map(k => {
                  const v = data.score?.breakdown?.[k];
                  if (typeof v !== 'number') return null;
                  const pc = v <= 1.5 ? v * 100 : v;
                  const col = pc >= 80 ? C.green : pc >= 60 ? C.yellow : C.red;
                  return (
                    <div key={k} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm, marginBottom: '8px' }}>
                      <span style={{ width: '110px', fontSize: '13px', color: C.text, textTransform: 'capitalize' }}>{k}</span>
                      <div style={{ flex: 1, background: C.bgInput, height: '12px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                        <div style={{ width: `${pc}%`, height: '100%', background: col, transition: 'width 0.4s' }} />
                      </div>
                      <span style={{ width: '56px', textAlign: 'right', fontSize: '13px', color: col, fontFamily: 'ui-monospace, monospace', fontWeight: T.typography.weightBold }}>{pc.toFixed(0)}</span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* --- Curriculum --- */}
        {sub === 'curriculum' && (
          <div>
            <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 16px' }}>Curriculum</h2>
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                {[0, 1, 2, 3, 4].map(i => (
                  <div key={i} style={{
                    height: '40px', borderRadius: T.radii.md,
                    background: `linear-gradient(90deg, ${C.bgCard} 0%, ${C.bgHover} 50%, ${C.bgCard} 100%)`,
                    backgroundSize: '200% 100%', animation: 'scc-skel-cls 1.3s ease-in-out infinite',
                    animationDelay: `${i * 0.08}s`,
                  }} />
                ))}
              </div>
            )}
            {data?.training_files && data.training_files.length > 0 ? (
              <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeMd }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: 'left', padding: '10px 14px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Dataset</th>
                      <th style={{ textAlign: 'right', padding: '10px 14px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Pairs</th>
                      <th style={{ textAlign: 'right', padding: '10px 14px', fontWeight: T.typography.weightBold, color: C.textSecondary, background: C.bgCard, borderBottom: `1px solid ${C.borderSubtle}` }}>Size</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[...data.training_files].sort((a, b) => b.pairs - a.pairs).map(f => (
                      <tr key={f.file}>
                        <td style={{ padding: '10px 14px', fontFamily: 'ui-monospace, monospace', color: C.text }}>{f.file}</td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', fontFamily: 'ui-monospace, monospace', color: C.accent, fontWeight: T.typography.weightBold }}>{f.pairs.toLocaleString()}</td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                {loading ? 'Loading curriculum…' : 'No training files reported.'}
              </div>
            )}
          </div>
        )}

        {/* --- Gradebook --- */}
        {sub === 'gradebook' && (
          <div>
            <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 16px' }}>Gradebook</h2>
            {loading && !data && (
              <div aria-busy='true' aria-live='polite' style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {[0, 1, 2, 3].map(i => (
                  <div key={i} style={{
                    height: '80px', borderRadius: T.radii.lg,
                    background: `linear-gradient(90deg, ${C.bgCard} 0%, ${C.bgHover} 50%, ${C.bgCard} 100%)`,
                    backgroundSize: '200% 100%', animation: 'scc-skel-cls 1.3s ease-in-out infinite',
                    animationDelay: `${i * 0.08}s`,
                  }} />
                ))}
              </div>
            )}
            <div style={{
              display: loading && !data ? 'none' : 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
              gap: T.spacing.md, marginBottom: T.spacing.xl,
            }}>
              <Stat C={C} label='Pass rate' value={(() => { const p = pctNorm(data?.training?.pass_rate); return p != null ? `${p.toFixed(1)}%` : '—'; })()} color={C.green} />
              <Stat C={C} label='Tested' value={data?.training?.total_tested != null ? compactNum(data.training.total_tested) : '—'} color={C.accent} />
              <Stat C={C} label='Correct' value={data?.training?.total_correct != null ? compactNum(data.training.total_correct) : '—'} color={C.green} />
              <Stat C={C} label='Avg quality' value={typeof data?.quality?.average === 'number' ? data.quality.average.toFixed(2) : '—'} color={C.yellow} />
            </div>
            {sortedDomains.length > 0 && (
              <div>
                <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                  Coverage by domain
                </div>
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
          <LibraryTab C={C} domains={sortedDomains} files={data?.training_files || []} />
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
    <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>{label}</div>
    <div style={{ fontSize: '26px', fontWeight: T.typography.weightBlack, color, marginTop: '4px', fontFamily: 'ui-monospace, monospace' }}>{value}</div>
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
            <span style={{ width: '160px', fontSize: '12px', color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{r.domain}</span>
            <div style={{ flex: 1, background: C.bgCard, height: '16px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
              <div style={{ width: `${(r.count / max) * 100}%`, height: '100%', background: colorFor(r.count), transition: 'width 0.4s' }} />
            </div>
            <div style={{ width: '64px', flexShrink: 0 }}>
              <Sparkline values={series} color={colorFor(r.count)} />
            </div>
            <span style={{ width: '96px', textAlign: 'right', fontSize: '12px', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{r.count.toLocaleString()}</span>
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
      <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Report Cards</h2>
      <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
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
        fontSize: '13px', color: C.textSecondary, lineHeight: 1.6,
      }}>
        {typeof data?.system?.uptime_hours === 'number' && (
          <div style={{ marginBottom: '8px' }}>
            <strong style={{ color: C.text }}>Server uptime:</strong> {data.system.uptime_hours.toFixed(1)} hours
          </div>
        )}
        {typeof data?.training?.sessions === 'number' && (
          <div style={{ marginBottom: '8px' }}>
            <strong style={{ color: C.text }}>Training sessions logged:</strong> {data.training.sessions.toLocaleString()}
          </div>
        )}
        {typeof data?.training?.learning_signals === 'number' && (
          <div style={{ marginBottom: '8px' }}>
            <strong style={{ color: C.text }}>Learning signals received:</strong> {data.training.learning_signals.toLocaleString()}
          </div>
        )}
        {typeof data?.training?.total_tested === 'number' && typeof data?.training?.total_correct === 'number' && (
          <div style={{ marginBottom: '8px' }}>
            <strong style={{ color: C.text }}>Evaluation record:</strong> {data.training.total_correct.toLocaleString()} correct of {data.training.total_tested.toLocaleString()} tested
          </div>
        )}
        {data?.quality?.high_quality_count != null && data?.quality?.low_quality_count != null && (
          <div style={{ marginBottom: '8px' }}>
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
      <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Test Center</h2>
      <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
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
        <div style={{ fontSize: '12px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
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
          <span style={{ fontSize: '11px', color: C.textDim }}>{auditInput.length}/10000</span>
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
            borderRadius: T.radii.md, fontSize: '12px',
          }}>{auditError}</div>
        )}
        {auditResult && (
          <pre style={{
            marginTop: T.spacing.md, padding: '12px', background: C.bgInput,
            border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
            fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '12px',
            color: C.text, whiteSpace: 'pre-wrap', overflowX: 'auto', maxHeight: '320px',
          }}>
            {JSON.stringify(auditResult, null, 2)}
          </pre>
        )}
      </div>
      {/* Rolling audit history — last 10, localStorage-backed */}
      {history.length > 0 && (
        <div style={{ marginTop: T.spacing.xl }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '10px' }}>
            <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
              History ({history.length})
            </div>
            <button onClick={clearHistory}
              style={{
                padding: '4px 10px', fontSize: '10px', fontWeight: T.typography.weightBold,
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
                    <span style={{ fontSize: '11px', color: C.textMuted, fontFamily: 'ui-monospace, monospace', flexShrink: 0 }}>
                      {new Date(h.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </span>
                    <span style={{ fontSize: '12px', color: C.text, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {h.prompt}
                    </span>
                    {h.verdict && <span style={{ fontSize: '11px', color, fontFamily: 'ui-monospace, monospace', fontWeight: T.typography.weightBold }}>{h.verdict}</span>}
                    <span style={{ color: C.textDim, fontSize: '10px' }}>{isOpen ? '▴' : '▾'}</span>
                  </button>
                  {isOpen && (
                    <pre style={{
                      margin: 0, padding: '10px 12px', background: C.bgInput,
                      borderTop: `1px solid ${C.borderSubtle}`,
                      fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '11px',
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
      <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Lesson Plans</h2>
      <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
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
          <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
            Active roster (by pairs)
          </div>
          <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
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
                      <td style={{ padding: '8px 12px', fontFamily: 'ui-monospace, monospace', color: C.text }}>{f.file}</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: 'ui-monospace, monospace', color: C.accent }}>{f.pairs.toLocaleString()}</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{f.size_mb.toFixed(1)} MB</td>
                      <td style={{ padding: '8px 12px', textAlign: 'right', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{share.toFixed(1)}%</td>
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
      <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Office Hours</h2>
      <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Review user feedback captured from thumbs-up/down on AI responses.
        Session-local only until /api/classroom/feedback aggregates server-side history.
      </p>
      <div style={{ display: 'flex', gap: T.spacing.md, marginBottom: T.spacing.xl }}>
        <div style={{ flex: 1, padding: T.spacing.md, background: C.greenBg, border: `1px solid ${C.greenBorder}`, borderRadius: T.radii.md }}>
          <div style={{ fontSize: '10px', fontWeight: T.typography.weightBold, color: C.green, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Positive</div>
          <div style={{ fontSize: '22px', fontWeight: T.typography.weightBlack, color: C.green, fontFamily: 'ui-monospace, monospace' }}>{posCount}</div>
        </div>
        <div style={{ flex: 1, padding: T.spacing.md, background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: T.radii.md }}>
          <div style={{ fontSize: '10px', fontWeight: T.typography.weightBold, color: C.red, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Negative</div>
          <div style={{ fontSize: '22px', fontWeight: T.typography.weightBlack, color: C.red, fontFamily: 'ui-monospace, monospace' }}>{negCount}</div>
        </div>
      </div>
      {feedback.length === 0 ? (
        <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted, fontSize: '13px', fontStyle: 'italic' }}>
          No feedback captured this session yet. Use 👍 / 👎 on any AI response to populate this log.
        </div>
      ) : (
        <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
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
                    <td style={{ padding: '8px 12px', color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>
                      {new Date(e.t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </td>
                    <td style={{ padding: '8px 12px', color: isPos ? C.green : C.red, fontWeight: T.typography.weightBold }}>
                      {isPos ? 'Positive' : 'Negative'}
                    </td>
                    <td style={{ padding: '8px 12px', color: C.text }}>{category}</td>
                    <td style={{ padding: '8px 12px', color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>{msgId}</td>
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

const LibraryTab: React.FC<{ C: any; domains: Array<{ domain: string; count: number }>; files: Array<{ file: string; pairs: number; size_mb: number }> }> = ({ C, domains, files }) => {
  const [q, setQ] = React.useState('');
  const normQ = q.trim().toLowerCase();
  const matchedDomains = normQ ? domains.filter(d => d.domain.toLowerCase().includes(normQ)) : domains;
  const matchedFiles = normQ ? files.filter(f => f.file.toLowerCase().includes(normQ)) : files;
  return (
    <div>
      <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>Library</h2>
      <p style={{ fontSize: '13px', color: C.textSecondary, margin: '0 0 16px', lineHeight: 1.55 }}>
        Browse what the AI has learned. Full-text search will land when /api/classroom/library supports it; for now you can filter
        domains and training files.
      </p>
      <input
        type='search' value={q} onChange={e => setQ(e.target.value)}
        autoComplete='off' spellCheck={false}
        placeholder={`Filter ${domains.length} domains / ${files.length} files…`}
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
          <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
            Domains ({matchedDomains.length})
          </div>
          {matchedDomains.length === 0 ? (
            <div style={{ fontSize: '13px', color: C.textDim, padding: '16px', textAlign: 'center' }}>No domains match.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              {matchedDomains.slice(0, 50).map(d => (
                <div key={d.domain} style={{
                  display: 'flex', justifyContent: 'space-between',
                  padding: '8px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
                  fontSize: '12px',
                }}>
                  <span style={{ color: C.text }}>{d.domain}</span>
                  <span style={{ color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>{d.count.toLocaleString()}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div>
          <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
            Training files ({matchedFiles.length})
          </div>
          {matchedFiles.length === 0 ? (
            <div style={{ fontSize: '13px', color: C.textDim, padding: '16px', textAlign: 'center' }}>No files match.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              {matchedFiles.slice(0, 50).map(f => (
                <div key={f.file} style={{
                  display: 'flex', justifyContent: 'space-between',
                  padding: '8px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
                  fontSize: '12px',
                }}>
                  <span style={{ color: C.text, fontFamily: 'ui-monospace, monospace' }}>{f.file}</span>
                  <span style={{ color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>{f.pairs.toLocaleString()} pairs</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const Placeholder: React.FC<{ C: any; title: string; body: string; data: unknown }> = ({ C, title, body, data }) => (
  <div>
    <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 12px' }}>{title}</h2>
    <div style={{
      padding: '24px', background: C.bgCard,
      border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
      fontSize: '14px', color: C.textSecondary, lineHeight: 1.6,
    }}>
      {body}
      {data !== null && (
        <pre style={{
          marginTop: T.spacing.md, padding: '12px', background: C.bgInput,
          border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
          fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '12px',
          color: C.textMuted, whiteSpace: 'pre-wrap', overflowX: 'auto', maxHeight: '240px',
        }}>
          {JSON.stringify(data, null, 2)}
        </pre>
      )}
    </div>
  </div>
);
