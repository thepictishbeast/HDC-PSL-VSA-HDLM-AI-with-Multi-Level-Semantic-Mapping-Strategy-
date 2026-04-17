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

export const ClassroomView: React.FC<ClassroomViewProps> = ({ C, host, isDesktop }) => {
  const [sub, setSub] = useState<Sub>('profile');
  const [data, setData] = useState<DashboardShape | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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
  // Auto-refresh active sub every 10s per c0-027.
  useEffect(() => {
    const id = setInterval(load, 10000);
    return () => clearInterval(id);
    // eslint-disable-next-line
  }, []);

  const sortedDomains = useMemo(() => {
    const arr = data?.domains || [];
    return [...arr].sort((a, b) => b.count - a.count);
  }, [data?.domains]);

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
      background: C.bg, color: C.text, overflow: 'hidden',
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
                <DomainBars C={C} rows={sortedDomains.slice(0, 15)} />
              </div>
            )}
          </div>
        )}

        {/* --- Lesson Plans --- */}
        {sub === 'lessons' && (
          <Placeholder C={C} title='Lesson Plans'
            body='Active training sessions, queue, and run controls land here once /api/classroom/lessons is live.'
            data={data?.training} />
        )}

        {/* --- Test Center --- */}
        {sub === 'tests' && (
          <Placeholder C={C} title='Test Center'
            body='Run benchmarks and quick quizzes against the current model. Awaiting /api/classroom/test endpoint.'
            data={null} />
        )}

        {/* --- Report Cards --- */}
        {sub === 'reports' && (
          <div>
            <h2 style={{ fontSize: '18px', fontWeight: 600, color: C.text, margin: '0 0 16px' }}>Report Cards</h2>
            <div style={{
              padding: T.spacing.lg, background: C.bgCard,
              border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
            }}>
              <div style={{ fontSize: '13px', color: C.textSecondary, lineHeight: 1.6 }}>
                {typeof data?.system?.uptime_hours === 'number' && (
                  <div style={{ marginBottom: '10px' }}>
                    <strong style={{ color: C.text }}>Uptime:</strong> {data.system.uptime_hours.toFixed(1)} hours
                  </div>
                )}
                {typeof data?.training?.sessions === 'number' && (
                  <div style={{ marginBottom: '10px' }}>
                    <strong style={{ color: C.text }}>Training sessions:</strong> {data.training.sessions.toLocaleString()}
                  </div>
                )}
                {typeof data?.training?.learning_signals === 'number' && (
                  <div style={{ marginBottom: '10px' }}>
                    <strong style={{ color: C.text }}>Learning signals:</strong> {data.training.learning_signals.toLocaleString()}
                  </div>
                )}
              </div>
              <div style={{ marginTop: T.spacing.md, fontSize: '12px', color: C.textDim, fontStyle: 'italic' }}>
                Weekly progress rollup will populate when /api/classroom/reports is available.
              </div>
            </div>
          </div>
        )}

        {/* --- Office Hours --- */}
        {sub === 'office' && (
          <Placeholder C={C} title='Office Hours'
            body='Review user feedback (thumbs-down categories + free-text). Awaiting /api/classroom/feedback endpoint. Until then, feedback is logged to logs and forwarded via /api/feedback.'
            data={null} />
        )}

        {/* --- Library --- */}
        {sub === 'library' && (
          <Placeholder C={C} title='Library'
            body='Searchable fact browser. Use the existing Knowledge Browser (Cmd+Shift+K) for now; this section will land when /api/classroom/library supports pagination + full-text.'
            data={null} />
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

const DomainBars: React.FC<{ C: any; rows: Array<{ domain: string; count: number }> }> = ({ C, rows }) => {
  const max = Math.max(...rows.map(r => r.count), 1);
  const colorFor = (n: number) => n > 10000 ? C.green : n > 1000 ? C.yellow : C.red;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
      {rows.map(r => (
        <div key={r.domain} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
          <span style={{ width: '160px', fontSize: '12px', color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{r.domain}</span>
          <div style={{ flex: 1, background: C.bgCard, height: '16px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
            <div style={{ width: `${(r.count / max) * 100}%`, height: '100%', background: colorFor(r.count), transition: 'width 0.4s' }} />
          </div>
          <span style={{ width: '96px', textAlign: 'right', fontSize: '12px', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{r.count.toLocaleString()}</span>
        </div>
      ))}
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
