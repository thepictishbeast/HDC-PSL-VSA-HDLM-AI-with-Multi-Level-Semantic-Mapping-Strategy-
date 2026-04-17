import React from 'react';
import { compactNum } from './util';

// Animated shimmer placeholder used while a fetch is in flight. Relies on the
// `@keyframes lfi-shimmer` defined in App.tsx's global <style> block.
export const Skeleton: React.FC<{
  w?: string | number;
  h?: string | number;
  radius?: number;
  style?: React.CSSProperties;
}> = ({ w = '100%', h = 14, radius = 6, style }) => (
  <div style={{
    width: typeof w === 'number' ? `${w}px` : w,
    height: typeof h === 'number' ? `${h}px` : h,
    borderRadius: radius,
    background: 'linear-gradient(90deg, rgba(255,255,255,0.04) 0%, rgba(255,255,255,0.12) 50%, rgba(255,255,255,0.04) 100%)',
    backgroundSize: '200% 100%',
    animation: 'lfi-shimmer 1.4s infinite linear',
    ...style,
  }} />
);

export interface TrainingDashboardProps {
  host: string;
  C: any;
}

type DomainRow = {
  domain: string;
  fact_count: number;
  avg_quality: number | null;
  avg_length: number | null;
};

export function TrainingDashboardContent({ host, C }: TrainingDashboardProps) {
  // Three independent fetches — one slow/failed endpoint should not black out the
  // whole panel. Each slice tracks its own state so we can render partial data.
  const [accuracy, setAccuracy] = React.useState<any | null>(null);
  const [domains, setDomains] = React.useState<DomainRow[] | null>(null);
  const [sessions, setSessions] = React.useState<any | null>(null);
  const [errors, setErrors] = React.useState<{ accuracy?: string; domains?: string; sessions?: string }>({});
  const [lastUpdated, setLastUpdated] = React.useState<number | null>(null);
  const [control, setControl] = React.useState<{ busy: 'start' | 'stop' | null; msg: string | null; ok: boolean }>({ busy: null, msg: null, ok: true });

  const refetch = React.useCallback(async () => {
    const mk = (path: string, timeoutMs = 15000) => async () => {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), timeoutMs);
      try {
        const r = await fetch(`http://${host}:3000${path}`, { signal: ctrl.signal });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return await r.json();
      } finally { clearTimeout(to); }
    };
    const [a, d, s] = await Promise.allSettled([
      mk('/api/admin/training/accuracy')(),
      mk('/api/admin/training/domains')(),
      mk('/api/admin/training/sessions')(),
    ]);
    const nextErrors: { accuracy?: string; domains?: string; sessions?: string } = {};
    if (a.status === 'fulfilled') setAccuracy(a.value); else nextErrors.accuracy = String((a.reason as any)?.message || a.reason);
    if (d.status === 'fulfilled') setDomains(Array.isArray(d.value?.domains) ? d.value.domains : []); else nextErrors.domains = String((d.reason as any)?.message || d.reason);
    if (s.status === 'fulfilled') setSessions(s.value); else nextErrors.sessions = String((s.reason as any)?.message || s.reason);
    setErrors(nextErrors);
    setLastUpdated(Date.now());
  }, [host]);

  const controlTrainer = React.useCallback(async (action: 'start' | 'stop') => {
    setControl({ busy: action, msg: null, ok: true });
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 10000);
      const r = await fetch(`http://${host}:3000/api/admin/training/${action}`, { method: 'POST', signal: ctrl.signal });
      clearTimeout(to);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const body = await r.json().catch(() => ({}));
      setControl({ busy: null, msg: body?.message || `Trainer ${action} requested`, ok: true });
      setTimeout(refetch, 500);
    } catch (e: any) {
      setControl({ busy: null, msg: `Failed to ${action}: ${e?.message || e}`, ok: false });
    }
  }, [host, refetch]);

  React.useEffect(() => {
    refetch();
    const id = setInterval(refetch, 30000);
    return () => clearInterval(id);
  }, [refetch]);

  const totalFacts: number | null = accuracy?.total_facts ?? null;
  const adversarialFacts: number | null = accuracy?.adversarial_facts ?? null;
  const psl = accuracy?.psl_calibration || null;
  const passRatePct: number | null = typeof psl?.pass_rate === 'number' ? psl.pass_rate * 100 : null;
  const reasoningChains: number | null = accuracy?.reasoning_chains ?? null;

  const trainingState = sessions?.training_state || {};
  const domainStateEntries: [string, any][] = Object.entries(trainingState);
  const anyRecentlyTrained = domainStateEntries.some(([, st]: any) => st?.last_trained && (Date.now() / 1000 - Number(st.last_trained)) < 300);
  const trainerActive = !!(sessions?.trainer_running) || anyRecentlyTrained;

  const allFailed = errors.accuracy && errors.domains && errors.sessions;
  const firstLoad = lastUpdated == null;

  if (firstLoad) {
    return (
      <div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '10px', marginBottom: '20px' }}>
          {[0, 1, 2].map(i => (
            <div key={i} style={{ padding: '14px', background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: '10px' }}>
              <Skeleton w={80} h={24} style={{ margin: '0 auto 8px' }} />
              <Skeleton w={96} h={10} style={{ margin: '0 auto' }} />
            </div>
          ))}
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '10px', marginBottom: '20px' }}>
          {[0, 1, 2, 3].map(i => (
            <div key={i} style={{ padding: '14px', background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: '10px' }}>
              <Skeleton w={64} h={20} style={{ margin: '0 auto 6px' }} />
              <Skeleton w={88} h={8} style={{ margin: '0 auto 4px' }} />
              <Skeleton w={72} h={6} style={{ margin: '0 auto' }} />
            </div>
          ))}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          {[0, 1, 2, 3, 4, 5].map(i => (
            <div key={i} style={{ padding: '8px 14px', background: C.bgInput, borderRadius: '8px', border: `1px solid ${C.borderSubtle}` }}>
              <Skeleton h={14} />
            </div>
          ))}
        </div>
      </div>
    );
  }
  if (allFailed) {
    return (
      <div style={{ padding: '24px', textAlign: 'center', color: C.textMuted }}>
        <div style={{ fontSize: '14px', color: C.red, marginBottom: '6px' }}>Training endpoints unreachable</div>
        <div style={{ fontSize: '12px', color: C.textDim }}>Backend may be restarting or the DB is in a write-lock window.</div>
        <button onClick={refetch} style={{
          marginTop: '12px', padding: '6px 14px', background: C.bgInput,
          border: `1px solid ${C.borderSubtle}`, borderRadius: '6px', color: C.text,
          fontSize: '12px', cursor: 'pointer',
        }}>Retry now</button>
      </div>
    );
  }

  return (
    <div>
      {/* Summary cards */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '10px', marginBottom: '20px' }}>
        <div style={{ padding: '14px', background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '24px', fontWeight: 800, color: C.accent }}>{compactNum(totalFacts)}</div>
          <div style={{ fontSize: '11px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>Facts in DB</div>
        </div>
        <div style={{ padding: '14px', background: C.greenBg, border: `1px solid ${C.greenBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '24px', fontWeight: 800, color: C.green }}>{domains?.length ?? domainStateEntries.length ?? '—'}</div>
          <div style={{ fontSize: '11px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>Domains</div>
        </div>
        <div style={{ padding: '14px', background: trainerActive ? C.greenBg : C.redBg, border: `1px solid ${trainerActive ? C.greenBorder : C.redBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '24px', fontWeight: 800, color: trainerActive ? C.green : C.red, lineHeight: 1 }}>
            <span
              className={trainerActive ? 'lfi-trainer-pulse' : undefined}
              style={{
                display: 'inline-block', width: '14px', height: '14px', borderRadius: '50%',
                background: trainerActive ? C.green : 'transparent',
                border: trainerActive ? 'none' : `2px solid ${C.red}`,
                verticalAlign: 'middle',
              }}
            />
          </div>
          <div style={{ fontSize: '11px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '6px' }}>
            {trainerActive ? 'Trainer Active' : 'Trainer Idle'}
          </div>
          <div style={{ display: 'flex', gap: '6px', justifyContent: 'center', marginTop: '8px' }}>
            <button
              onClick={() => controlTrainer('start')}
              disabled={control.busy !== null || trainerActive}
              style={{
                flex: 1, padding: '5px 8px', fontSize: '10px', fontWeight: 700,
                textTransform: 'uppercase', letterSpacing: '0.06em',
                color: trainerActive ? C.textDim : C.green,
                background: trainerActive ? 'transparent' : C.greenBg,
                border: `1px solid ${trainerActive ? C.borderSubtle : C.greenBorder}`,
                borderRadius: '6px',
                cursor: (control.busy !== null || trainerActive) ? 'not-allowed' : 'pointer',
                opacity: control.busy === 'start' ? 0.55 : 1,
              }}
            >{control.busy === 'start' ? '…' : 'Start'}</button>
            <button
              onClick={() => controlTrainer('stop')}
              disabled={control.busy !== null || !trainerActive}
              style={{
                flex: 1, padding: '5px 8px', fontSize: '10px', fontWeight: 700,
                textTransform: 'uppercase', letterSpacing: '0.06em',
                color: !trainerActive ? C.textDim : C.red,
                background: !trainerActive ? 'transparent' : C.redBg,
                border: `1px solid ${!trainerActive ? C.borderSubtle : C.redBorder}`,
                borderRadius: '6px',
                cursor: (control.busy !== null || !trainerActive) ? 'not-allowed' : 'pointer',
                opacity: control.busy === 'stop' ? 0.55 : 1,
              }}
            >{control.busy === 'stop' ? '…' : 'Stop'}</button>
          </div>
          {control.msg && (
            <div style={{
              marginTop: '6px', fontSize: '10px',
              color: control.ok ? C.textMuted : C.red,
              lineHeight: 1.3,
            }}>{control.msg}</div>
          )}
        </div>
      </div>

      {/* Quality & Security metrics row — all values live from /api/admin/training/accuracy */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '10px', marginBottom: '20px' }}>
        <div style={{ padding: '14px', background: C.greenBg, border: `1px solid ${C.greenBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '20px', fontWeight: 800, color: C.green }}>
            {passRatePct != null ? `${passRatePct.toFixed(1)}%` : '—'}
          </div>
          <div style={{ fontSize: '10px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>PSL Pass Rate</div>
          <div style={{ fontSize: '9px', color: C.textDim, marginTop: '2px' }}>
            {psl?.target ? `Target: ${psl.target}` : 'Target: 95-98%'}
          </div>
          <div style={{ height: '4px', marginTop: '8px', background: 'rgba(255,255,255,0.08)', borderRadius: '2px', overflow: 'hidden' }}>
            <div
              className="lfi-progress-fill"
              style={{
                height: '100%',
                width: passRatePct != null ? `${Math.max(0, Math.min(100, passRatePct))}%` : '0%',
                background: passRatePct == null ? C.textDim : (passRatePct >= 95 ? C.green : passRatePct >= 85 ? C.yellow : C.red),
              }}
            />
          </div>
        </div>
        <div style={{ padding: '14px', background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '20px', fontWeight: 800, color: C.accent }}>{compactNum(adversarialFacts)}</div>
          <div style={{ fontSize: '10px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>Adversarial Facts</div>
          <div style={{ fontSize: '9px', color: C.textDim, marginTop: '2px' }}>ANLI + FEVER + TruthfulQA</div>
        </div>
        <div style={{ padding: '14px', background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '20px', fontWeight: 800, color: C.accent }}>{compactNum(reasoningChains)}</div>
          <div style={{ fontSize: '10px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>Reasoning Chains</div>
          <div style={{ fontSize: '9px', color: C.textDim, marginTop: '2px' }}>Self-play + teacher</div>
        </div>
        <div style={{ padding: '14px', background: C.greenBg, border: `1px solid ${C.greenBorder}`, borderRadius: '10px', textAlign: 'center' }}>
          <div style={{ fontSize: '20px', fontWeight: 800, color: C.green }}>
            {accuracy?.learning_signals != null ? compactNum(accuracy.learning_signals) : '—'}
          </div>
          <div style={{ fontSize: '10px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginTop: '4px' }}>Learning Signals</div>
          <div style={{ fontSize: '9px', color: C.textDim, marginTop: '2px' }}>Corrections + gaps + PSL</div>
        </div>
      </div>

      {/* Per-domain breakdown + heatmap (from /api/admin/training/domains + session timestamps) */}
      {domains && domains.length > 0 && (() => {
        const maxFacts = Math.max(...domains.map((d) => d.fact_count || 0), 1);
        const nowSec = Date.now() / 1000;
        const heatColor = (fc: number, q: number | null) => {
          const share = fc / maxFacts;
          const qMul = q == null ? 1 : Math.max(0.5, Math.min(1, q));
          const alpha = Math.max(0.12, Math.min(0.9, share * qMul));
          return `rgba(139, 123, 247, ${alpha.toFixed(2)})`;
        };
        return (
          <div style={{ marginBottom: '20px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
              <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em' }}>
                Per-Domain Coverage ({domains.length})
              </div>
              <div style={{ fontSize: '9px', color: C.textDim }}>bar width = fact share · tint = quality</div>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              {[...domains].sort((a, b) => b.fact_count - a.fact_count).map((d) => {
                const st = trainingState[d.domain] || {};
                const pct = Math.round((d.fact_count / maxFacts) * 100);
                const recent = st.last_trained && (nowSec - Number(st.last_trained)) < 300;
                return (
                  <div key={d.domain} style={{
                    position: 'relative',
                    padding: '8px 14px', background: C.bgInput, borderRadius: '8px',
                    border: `1px solid ${recent ? C.greenBorder : C.borderSubtle}`,
                    overflow: 'hidden',
                  }}>
                    <div
                      className="lfi-progress-fill"
                      style={{
                        position: 'absolute', inset: 0,
                        width: `${pct}%`, background: heatColor(d.fact_count, d.avg_quality),
                        pointerEvents: 'none',
                      }}
                    />
                    <div style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: '12px', fontSize: '12px' }}>
                      <span style={{ fontWeight: 600, color: C.text, minWidth: '110px' }}>{d.domain}</span>
                      <span style={{ color: C.textMuted }}>{compactNum(d.fact_count)} facts</span>
                      {d.avg_quality != null && (
                        <span style={{ color: C.textMuted }}>q={d.avg_quality.toFixed(2)}</span>
                      )}
                      {st.sessions != null && (
                        <span style={{ color: C.textMuted }}>{st.sessions} sessions</span>
                      )}
                      <div style={{ flex: 1 }} />
                      {recent && (
                        <span style={{ fontSize: '9px', color: C.green, fontWeight: 700, letterSpacing: '0.08em' }}>LIVE</span>
                      )}
                      <span style={{ fontSize: '10px', color: C.textDim }}>
                        {st.last_trained ? new Date(st.last_trained * 1000).toLocaleTimeString() : 'never'}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        );
      })()}

      {/* Recent training log (from /api/admin/training/accuracy) */}
      {Array.isArray(accuracy?.recent_training_log) && accuracy.recent_training_log.length > 0 && (
        <div>
          <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: '10px' }}>
            Recent Training Log
          </div>
          <pre style={{
            padding: '12px', background: C.bgInput, borderRadius: '8px',
            fontSize: '11px', color: C.textSecondary,
            fontFamily: "'JetBrains Mono', monospace",
            whiteSpace: 'pre-wrap', maxHeight: '200px', overflowY: 'auto',
            margin: 0,
          }}>
            {accuracy.recent_training_log.slice(-40).join('\n')}
          </pre>
        </div>
      )}

      {/* Freshness + error footnote */}
      <div style={{ marginTop: '16px', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '10px', color: C.textDim }}>
        <span>
          {lastUpdated ? `Updated ${new Date(lastUpdated).toLocaleTimeString()}` : ''}
          {(errors.accuracy || errors.domains || errors.sessions) ? ` \u00B7 partial (${Object.keys(errors).join(', ')} failed)` : ''}
        </span>
        <button onClick={refetch} style={{
          padding: '4px 10px', background: 'transparent',
          border: `1px solid ${C.borderSubtle}`, borderRadius: '6px', color: C.textMuted,
          fontSize: '10px', cursor: 'pointer',
        }}>Refresh</button>
      </div>
    </div>
  );
}
