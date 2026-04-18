import React, { useEffect, useState } from 'react';
import { T } from './tokens';
// c2-347: shared stat/summary card (replaces the local Stat helper).
import { StatCard } from './components/StatCard';
// c2-378 / BIG #180: DataTable adoption for tier + findings tables.
import { DataTable } from './components';
import type { Column } from './components';
import { formatRelative } from './util';

// c0-037 #12 / c2-331: Auditorium — AVP-2 audit state surface.
// Stub in the sense that there is no dedicated backend endpoint yet.
// The page:
//  1) renders the 9-tier / 36-pass AVP-2 structure as static reference
//     data (the protocol itself is stable per AVP2_SUPERSOCIETY_PROTOCOL.md)
//  2) tries /api/avp/status (then /api/admin/avp/status) for any live
//     pass/findings data; if unavailable, falls through to the reference
//     view with an inline "live stats unavailable" notice.
// When the backend ships real AVP state, the same component renders it.

interface AvpStatus {
  passes_completed?: number;
  total_passes?: number;
  findings_total?: number;
  findings_fixed?: number;
  security_score?: number;       // 0..1 or 0..100 — normalised
  code_quality_score?: number;   // ditto
  last_run?: string | number;
  tier_progress?: Array<{ tier: number; name: string; status: 'pending' | 'in_progress' | 'passed' | 'failed' }>;
  recent_findings?: Array<{ id: string; title: string; severity: 'low' | 'medium' | 'high' | 'critical'; fixed?: boolean; ts?: number | string }>;
}

export interface AuditoriumViewProps {
  C: any;
  host: string;
  isDesktop: boolean;
}

// Reference data — Tier structure from AVP2_SUPERSOCIETY_PROTOCOL.md.
const TIERS: Array<{ tier: number; name: string; passes: number; range: string }> = [
  { tier: 1, name: 'Existence proof',    passes: 6, range: '1–6'   },
  { tier: 2, name: 'Failure resilience', passes: 6, range: '7–12'  },
  { tier: 3, name: 'Adversarial security', passes: 12, range: '13–24' },
  { tier: 4, name: 'UX/UI adversarial',  passes: 6, range: '25–30' },
  { tier: 5, name: 'Integration & ecosystem', passes: 3, range: '31–33' },
  { tier: 6, name: 'Meta-validation',    passes: 3, range: '34–36' },
];
const TOTAL_PASSES = TIERS.reduce((s, t) => s + t.passes, 0); // 36

const pctNorm = (raw: number | undefined): number | null => {
  if (typeof raw !== 'number' || !isFinite(raw)) return null;
  return raw <= 1.5 ? raw * 100 : raw;
};

export const AuditoriumView: React.FC<AuditoriumViewProps> = ({ C, host, isDesktop }) => {
  const [status, setStatus] = useState<AvpStatus | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);

  const load = async () => {
    setLoading(true);
    setErr(null);
    const tryPath = async (path: string) => {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 4000);
      try {
        const r = await fetch(`http://${host}:3000${path}`, { signal: ctrl.signal });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return (await r.json()) as AvpStatus;
      } finally { clearTimeout(to); }
    };
    try {
      let data: AvpStatus;
      try { data = await tryPath('/api/avp/status'); }
      catch { data = await tryPath('/api/admin/avp/status'); }
      setStatus(data);
      setLastUpdated(Date.now());
    } catch (e: any) {
      const m = String(e?.message || e || 'fetch failed');
      setErr(m.includes('abort') ? 'AVP status endpoint timed out.' : m);
      // Not a blocker — we still render the reference view below.
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { load(); /* eslint-disable-next-line */ }, []);

  const passesCompleted = status?.passes_completed ?? 0;
  const totalPasses = status?.total_passes ?? TOTAL_PASSES;
  const findingsTotal = status?.findings_total ?? null;
  const findingsFixed = status?.findings_fixed ?? null;
  const securityPct = pctNorm(status?.security_score);
  const qualityPct = pctNorm(status?.code_quality_score);

  const scoreColor = (v: number | null) => {
    if (v == null) return C.textMuted;
    return v >= 80 ? C.green : v >= 60 ? C.yellow : C.red;
  };

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
      background: C.bg, color: C.text, overflow: 'hidden',
      animation: 'lfi-fadein 0.18s ease-out',
    }}>
      <div style={{
        display: 'flex', alignItems: 'center', gap: T.spacing.md,
        padding: `${T.spacing.lg} ${T.spacing.xl}`,
        borderBottom: `1px solid ${C.borderSubtle}`, background: C.bgCard,
      }}>
        <h1 style={{
          margin: 0, fontSize: T.typography.sizeXl,
          fontWeight: T.typography.weightBlack, color: C.text,
          letterSpacing: T.typography.trackingCap, textTransform: 'uppercase',
        }}>Auditorium</h1>
        <span style={{
          fontSize: T.typography.sizeXs, color: C.textMuted,
          padding: '2px 8px', border: `1px solid ${C.borderSubtle}`,
          borderRadius: T.radii.sm, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
        }}>AVP-2</span>
        <div style={{ flex: 1 }} />
        {lastUpdated != null && !err && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            Updated {formatRelative(lastUpdated)}
          </span>
        )}
        <button onClick={load} disabled={loading} aria-label='Refresh AVP status'
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 8px', display: 'flex', alignItems: 'center',
            fontFamily: 'inherit',
          }}>
          <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor'
            strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round'
            style={loading ? { animation: 'scc-aud-spin 0.8s linear infinite' } : undefined}>
            <polyline points='23 4 23 10 17 10' />
            <polyline points='1 20 1 14 7 14' />
            <path d='M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15' />
          </svg>
        </button>
        <style>{`@keyframes scc-aud-spin { to { transform: rotate(360deg); } }`}</style>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: T.spacing.xl, maxWidth: '1200px', width: '100%', margin: '0 auto' }}>
        {err && (
          <div role='status' style={{
            padding: `${T.spacing.sm} ${T.spacing.md}`, marginBottom: T.spacing.lg,
            background: C.yellowBg, border: `1px solid ${C.yellowBorder || C.yellow}`,
            color: C.yellow, borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
          }}>
            Live AVP status unavailable ({err}). Showing protocol reference below.
          </div>
        )}

        {/* Headline stats — live if available, otherwise protocol totals. */}
        <div style={{
          display: 'grid', gridTemplateColumns: isDesktop ? 'repeat(auto-fit, minmax(200px, 1fr))' : 'repeat(2, 1fr)',
          gap: T.spacing.md, marginBottom: T.spacing.xl,
        }}>
          <StatCard C={C} label='Passes'
            value={`${passesCompleted} / ${totalPasses}`}
            color={passesCompleted >= totalPasses ? C.green : passesCompleted > 0 ? C.yellow : C.textMuted} />
          <StatCard C={C} label='Findings fixed'
            value={findingsTotal != null ? `${findingsFixed ?? 0} / ${findingsTotal}` : '—'}
            color={findingsTotal != null && findingsFixed != null
              ? (findingsFixed >= findingsTotal ? C.green
                 : findingsFixed / Math.max(findingsTotal, 1) >= 0.5 ? C.yellow : C.red)
              : C.textMuted} />
          <StatCard C={C} label='Security score'
            value={securityPct != null ? `${securityPct.toFixed(1)}%` : '—'}
            color={scoreColor(securityPct)} />
          <StatCard C={C} label='Code quality'
            value={qualityPct != null ? `${qualityPct.toFixed(1)}%` : '—'}
            color={scoreColor(qualityPct)} />
        </div>

        {/* Tier structure — always rendered from the protocol. */}
        <div style={{ marginBottom: T.spacing.xl }}>
          <div style={{
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
            color: C.textMuted, textTransform: 'uppercase',
            letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.sm,
          }}>
            Tier structure (AVP-2 §Loop)
          </div>
          {/* c2-378 / BIG #180: tier structure table -> DataTable. The
              status column derives both a label and a color from live
              progress + local rollup, so we bake both into the accessor
              rather than giving DataTable a custom row-style prop. */}
          {(() => {
            type TierRow = typeof TIERS[number];
            const statusFor = (t: TierRow, i: number): { label: string; color: string } => {
              const live = status?.tier_progress?.find(x => x.tier === t.tier);
              const raw = live?.status ?? (passesCompleted >= TIERS.slice(0, i + 1).reduce((s, tt) => s + tt.passes, 0) ? 'passed'
                : passesCompleted >= TIERS.slice(0, i).reduce((s, tt) => s + tt.passes, 0) ? 'in_progress' : 'pending');
              const color = raw === 'passed' ? C.green
                : raw === 'in_progress' ? C.yellow
                : raw === 'failed' ? C.red : C.textDim;
              return { label: raw.replace('_', ' '), color };
            };
            const cols: ReadonlyArray<Column<TierRow>> = [
              {
                id: 'tier', header: 'Tier', align: 'left', width: '72px',
                sortKey: (t) => t.tier,
                accessor: (t) => <span style={{ fontFamily: T.typography.fontMono, color: C.accent }}>T{t.tier}</span>,
              },
              {
                id: 'name', header: 'Name', align: 'left',
                sortKey: (t) => t.name.toLowerCase(),
                accessor: (t) => <span style={{ color: C.text }}>{t.name}</span>,
              },
              {
                id: 'passes', header: 'Passes', align: 'right', width: '80px',
                sortKey: (t) => t.passes,
                accessor: (t) => <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{t.passes}</span>,
              },
              {
                id: 'range', header: 'Range', align: 'right', width: '110px', sortable: false,
                accessor: (t) => <span style={{ color: C.textMuted, fontFamily: T.typography.fontMono }}>{t.range}</span>,
              },
              {
                id: 'status', header: 'Status', align: 'center', width: '130px', sortable: false,
                accessor: (t) => {
                  const i = TIERS.indexOf(t);
                  const s = statusFor(t, i);
                  return (
                    <span style={{
                      color: s.color, fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                      textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                    }}>{s.label}</span>
                  );
                },
              },
            ];
            return (
              <DataTable<TierRow> C={C}
                rows={TIERS as ReadonlyArray<TierRow> as TierRow[]}
                columns={cols}
                rowKey={(t) => t.tier}
                sort={{ col: 'tier', dir: 'asc' }} />
            );
          })()}
        </div>

        {/* Recent findings — live-only; hidden when backend has none. */}
        {status?.recent_findings && status.recent_findings.length > 0 && (
          <div>
            <div style={{
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
              color: C.textMuted, textTransform: 'uppercase',
              letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.sm,
            }}>Recent findings ({status.recent_findings.length})</div>
            {/* c2-378 / BIG #180: findings table -> DataTable. Severity
                sort is a custom key so critical > high > medium > low reads
                as "most urgent first" when sorted desc. */}
            {(() => {
              type FRow = (typeof status.recent_findings)[number];
              const sevRank = (s?: string): number =>
                s === 'critical' ? 4 : s === 'high' ? 3 : s === 'medium' ? 2 : s === 'low' ? 1 : 0;
              const rows = status.recent_findings.slice(0, 100);
              const cols: ReadonlyArray<Column<FRow>> = [
                {
                  id: 'id', header: 'ID', align: 'left', width: '100px',
                  sortKey: (f) => f.id || '',
                  accessor: (f, ) => <span style={{ color: C.accent, fontFamily: T.typography.fontMono }}>{f.id || '\u2014'}</span>,
                },
                {
                  id: 'title', header: 'Title', align: 'left',
                  sortKey: (f) => (f.title || '').toLowerCase(),
                  accessor: (f) => <span style={{ color: C.text }}>{f.title}</span>,
                },
                {
                  id: 'severity', header: 'Severity', align: 'center', width: '120px',
                  sortKey: (f) => sevRank(f.severity),
                  accessor: (f) => {
                    const col = f.severity === 'critical' || f.severity === 'high' ? C.red
                      : f.severity === 'medium' ? C.yellow : C.textMuted;
                    return (
                      <span style={{
                        color: col, fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                        textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                      }}>{f.severity}</span>
                    );
                  },
                },
                {
                  id: 'status', header: 'Status', align: 'center', width: '90px',
                  sortKey: (f) => f.fixed ? 1 : 0,
                  accessor: (f) => (
                    <span style={{ color: f.fixed ? C.green : C.textMuted, fontSize: T.typography.sizeBody }}>
                      {f.fixed ? '\u2714' : '\u2022'}
                    </span>
                  ),
                },
              ];
              return (
                <div style={{ maxHeight: '320px', overflowY: 'auto' }}>
                  <DataTable<FRow> C={C}
                    rows={rows}
                    columns={cols}
                    rowKey={(f) => f.id || `${f.title}-${f.severity}`} />
                </div>
              );
            })()}
          </div>
        )}

        {/* Verdict — always. AVP-2 §Ship verdict is always "STILL BROKEN". */}
        <div style={{
          marginTop: T.spacing.xl,
          padding: `${T.spacing.md} ${T.spacing.lg}`,
          background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
          borderRadius: T.radii.md, fontSize: T.typography.sizeSm,
          color: C.textSecondary, lineHeight: T.typography.lineLoose,
        }}>
          <strong style={{ color: C.text }}>Ship verdict:</strong> per AVP-2 §Ship verdict, the
          answer is always <em>STILL BROKEN</em>. Shipping is explicit risk
          acceptance, not a declaration of correctness. The loop resumes on
          the next commit.
        </div>
      </div>
    </div>
  );
};

// ---- Private helpers ----

// c2-347: the local Stat helper moved to components/StatCard.tsx.

const Th: React.FC<{ C: any; children: React.ReactNode; align?: 'left' | 'right' | 'center' }> = ({ C, children, align = 'left' }) => (
  <th style={{
    textAlign: align, padding: '8px 12px',
    fontWeight: T.typography.weightBold, color: C.textSecondary,
    background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}`,
    position: 'sticky', top: 0,
  }}>{children}</th>
);
