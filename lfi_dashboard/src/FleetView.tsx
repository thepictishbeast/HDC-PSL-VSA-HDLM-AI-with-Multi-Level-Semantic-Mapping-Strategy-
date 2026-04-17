import React, { useEffect, useState } from 'react';
import { T } from './tokens';
import { formatRelative } from './util';

// c0-037 #2 / c2-328: standalone Fleet dashboard page (was Admin tab only).
// Fetches /api/orchestrator/dashboard on :3001 (c0-035 split service) and
// falls back to :3000 during rollout. Rendering mirrors the AdminModal
// fleet tab — stat cards, per-instance cards, activity timeline — so users
// get the same view whether they land here via hash route #fleet or via
// the Admin modal's Fleet tab. Auto-refreshes every 5s; pauses nothing
// (fleet data is cheap, orchestrator already scopes it).

interface FleetInstance {
  id: string;
  name?: string;
  role?: string;
  status?: string;
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

export interface FleetViewProps {
  C: any;
  host: string;
  isDesktop: boolean;
}

export const FleetView: React.FC<FleetViewProps> = ({ C, host, isDesktop }) => {
  const [fleet, setFleet] = useState<FleetShape | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);

  const load = async () => {
    setLoading(true);
    setErr(null);
    const tryPort = async (port: number) => {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 4000);
      try {
        const r = await fetch(`http://${host}:${port}/api/orchestrator/dashboard`, { signal: ctrl.signal });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return (await r.json()) as FleetShape;
      } finally { clearTimeout(to); }
    };
    try {
      let data: FleetShape;
      try { data = await tryPort(3001); } catch { data = await tryPort(3000); }
      setFleet(data);
      setLastUpdated(Date.now());
    } catch (e: any) {
      const m = String(e?.message || e || 'fetch failed');
      setErr(m.includes('abort') ? 'Orchestrator timed out.' : m);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { load(); /* eslint-disable-next-line */ }, []);
  useEffect(() => {
    const id = setInterval(load, 5000);
    return () => clearInterval(id);
    // eslint-disable-next-line
  }, []);

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
      background: C.bg, color: C.text, overflow: 'hidden',
      animation: 'lfi-fadein 0.18s ease-out',
    }}>
      {/* Header row with title + refresh */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: T.spacing.md,
        padding: `${T.spacing.lg} ${T.spacing.xl}`,
        borderBottom: `1px solid ${C.borderSubtle}`, background: C.bgCard,
      }}>
        <h1 style={{
          margin: 0, fontSize: T.typography.sizeXl,
          fontWeight: T.typography.weightBlack, color: C.text,
          letterSpacing: T.typography.trackingCap, textTransform: 'uppercase',
        }}>Fleet</h1>
        <div style={{ flex: 1 }} />
        {lastUpdated != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: 'ui-monospace, monospace' }}>
            Updated {formatRelative(lastUpdated)}
          </span>
        )}
        <button onClick={load} disabled={loading} aria-label='Refresh fleet data'
          title={loading ? 'Refreshing…' : 'Refresh (auto-refresh every 5s)'}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 8px', display: 'flex', alignItems: 'center',
            fontFamily: 'inherit',
          }}>
          <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor'
            strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round'
            style={loading ? { animation: 'scc-fleet-spin 0.8s linear infinite' } : undefined}>
            <polyline points='23 4 23 10 17 10' />
            <polyline points='1 20 1 14 7 14' />
            <path d='M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15' />
          </svg>
        </button>
        <style>{`@keyframes scc-fleet-spin { to { transform: rotate(360deg); } }`}</style>
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflowY: 'auto', padding: T.spacing.xl, maxWidth: '1200px', width: '100%', margin: '0 auto' }}>
        {err && (
          <div role='alert' style={{
            padding: `${T.spacing.md} ${T.spacing.lg}`, marginBottom: T.spacing.lg,
            background: C.redBg, border: `1px solid ${C.redBorder}`,
            color: C.red, borderRadius: T.radii.md,
            display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: T.spacing.md,
          }}>
            <span><strong>Could not load fleet:</strong> {err}</span>
            <button onClick={load} disabled={loading}
              style={{
                background: 'transparent', border: `1px solid ${C.redBorder}`,
                color: C.red, borderRadius: T.radii.sm,
                padding: `${T.spacing.xs} ${T.spacing.md}`,
                cursor: loading ? 'wait' : 'pointer',
                fontFamily: 'inherit', fontSize: T.typography.sizeXs,
                fontWeight: T.typography.weightBold, textTransform: 'uppercase',
                letterSpacing: '0.06em',
              }}>{loading ? 'Retrying…' : 'Retry'}</button>
          </div>
        )}
        {fleet === null && !err && (
          <div aria-busy='true' style={{ padding: T.spacing.xxxl, textAlign: 'center', color: C.textMuted }}>
            Loading orchestrator…
          </div>
        )}
        {fleet && (
          <>
            {fleet.stats && (
              <div style={{
                display: 'grid', gridTemplateColumns: isDesktop ? 'repeat(auto-fit, minmax(180px, 1fr))' : 'repeat(2, 1fr)',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                <Stat C={C} label='Instances' value={String(fleet.instances?.length ?? 0)} color={C.accent} />
                <Stat C={C} label='Tasks total' value={typeof fleet.stats.total_tasks === 'number' ? String(fleet.stats.total_tasks) : '—'} color={C.purple} />
                <Stat C={C} label='Running' value={typeof fleet.stats.running === 'number' ? String(fleet.stats.running) : '—'} color={C.yellow} />
                <Stat C={C} label='Completed' value={typeof fleet.stats.completed === 'number' ? String(fleet.stats.completed) : '—'} color={C.green} />
              </div>
            )}
            {fleet.instances && fleet.instances.length > 0 && (
              <div style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {fleet.instances.map(inst => (
                  <InstanceCard key={inst.id} C={C} inst={inst} />
                ))}
              </div>
            )}
            {fleet.timeline && fleet.timeline.length > 0 && (
              <div>
                <div style={{
                  fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                  color: C.textMuted, textTransform: 'uppercase',
                  letterSpacing: T.typography.trackingLoose, marginBottom: T.spacing.sm,
                }}>
                  Recent activity ({fleet.timeline.length})
                </div>
                <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'hidden', maxHeight: '420px', overflowY: 'auto' }}>
                  <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: T.typography.sizeSm }}>
                    <thead>
                      <tr>
                        <Th C={C}>When</Th>
                        <Th C={C}>Who</Th>
                        <Th C={C}>Event</Th>
                      </tr>
                    </thead>
                    <tbody>
                      {fleet.timeline.slice(0, 200).map((row, i) => (
                        <tr key={i} style={{ background: i % 2 === 0 ? 'transparent' : C.bgHover }}>
                          <td style={{ padding: '6px 12px', color: C.textMuted, fontFamily: 'ui-monospace, monospace', whiteSpace: 'nowrap' }}>
                            {typeof row.t === 'number' ? new Date(row.t * (row.t < 1e12 ? 1000 : 1)).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : row.t}
                          </td>
                          <td style={{ padding: '6px 12px', color: C.accent, fontFamily: 'ui-monospace, monospace' }}>{row.instance}</td>
                          <td style={{ padding: '6px 12px', color: C.text, fontFamily: 'ui-monospace, monospace' }}>{row.event}</td>
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
    </div>
  );
};

// ---- Private helpers ----

const Stat: React.FC<{ C: any; label: string; value: string; color: string }> = ({ C, label, value, color }) => (
  <div style={{
    padding: `${T.spacing.md} ${T.spacing.lg}`, borderRadius: T.radii.md,
    background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
  }}>
    <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>{label}</div>
    <div style={{ fontSize: '24px', fontWeight: T.typography.weightBlack, color, marginTop: '4px', fontFamily: 'ui-monospace, monospace' }}>{value}</div>
  </div>
);

const InstanceCard: React.FC<{ C: any; inst: FleetInstance }> = ({ C, inst }) => {
  const statusColor = inst.status === 'running' ? C.green
    : inst.status === 'error' ? C.red
    : inst.status === 'idle' ? C.yellow : C.textMuted;
  return (
    <div style={{
      padding: T.spacing.lg, borderRadius: T.radii.md,
      background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
        <div style={{ fontSize: T.typography.sizeBody, fontWeight: T.typography.weightBold, color: C.text }}>
          {inst.name || inst.id}
        </div>
        <span style={{
          display: 'inline-flex', alignItems: 'center', gap: '6px',
          fontSize: '10px', fontWeight: T.typography.weightBold,
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
        <div title={inst.current_task} style={{
          padding: '6px 8px', background: C.bgInput, borderRadius: T.radii.sm,
          fontSize: '11px', color: C.textMuted, fontFamily: 'ui-monospace, monospace',
          marginBottom: '6px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {inst.current_task}
        </div>
      )}
      <div style={{ display: 'flex', gap: T.spacing.md, fontSize: '11px', color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>
        {typeof inst.tasks_completed === 'number' && <span>{'\u2713'} {inst.tasks_completed}</span>}
        {typeof inst.tasks_pending === 'number' && <span>{'\u29D6'} {inst.tasks_pending}</span>}
        {inst.last_seen && <span style={{ marginLeft: 'auto' }}>
          last seen {typeof inst.last_seen === 'number' ? new Date(inst.last_seen * (inst.last_seen < 1e12 ? 1000 : 1)).toLocaleTimeString() : inst.last_seen}
        </span>}
      </div>
    </div>
  );
};

const Th: React.FC<{ C: any; children: React.ReactNode }> = ({ C, children }) => (
  <th style={{
    textAlign: 'left', padding: '8px 12px',
    fontWeight: T.typography.weightBold, color: C.textSecondary,
    background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}`,
    position: 'sticky', top: 0,
  }}>{children}</th>
);
