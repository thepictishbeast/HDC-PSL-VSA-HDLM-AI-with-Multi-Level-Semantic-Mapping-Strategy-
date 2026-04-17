import React, { useState } from 'react';

// Training Accuracy panel (c0-016 B1 sub). Fetches /api/admin/training/accuracy
// on demand and displays as a compact metric grid — pass_rate (%), sample
// count, last run timestamp, plus any per-domain breakdown the backend ships.
//
// Lazy fetch (button-triggered) for the same reason as DomainsPanel: admin
// sidebar is already heavy, don't poll for something the user didn't open.

interface AccuracyReport {
  pass_rate?: number;       // 0..1 or 0..100 — we normalise
  samples?: number;
  last_run?: string | number;
  accuracy?: number;        // some backends use this alias
  per_domain?: Record<string, number>;
}

export interface AccuracyPanelProps {
  C: any;
  host: string;
}

const pct = (raw: number | undefined): string => {
  if (typeof raw !== 'number' || !isFinite(raw)) return '—';
  const v = raw <= 1.5 ? raw * 100 : raw; // matches the PSL heuristic used elsewhere
  return `${v.toFixed(1)}%`;
};

export const AccuracyPanel: React.FC<AccuracyPanelProps> = ({ C, host }) => {
  const [data, setData] = useState<AccuracyReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 8000);
      const res = await fetch(`http://${host}:3000/api/admin/training/accuracy`, { signal: ctrl.signal });
      clearTimeout(to);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setData(await res.json());
    } catch (e: any) {
      setError(String(e?.message || e || 'fetch failed'));
    } finally {
      setLoading(false);
    }
  };
  const rate = pct(data?.pass_rate ?? data?.accuracy);
  const rateColor = (() => {
    const raw = data?.pass_rate ?? data?.accuracy;
    if (typeof raw !== 'number') return C.textMuted;
    const v = raw <= 1.5 ? raw * 100 : raw;
    return v >= 95 ? C.green : v >= 80 ? C.yellow : C.red;
  })();
  return (
    <div style={{ marginTop: '12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
        <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>
          Training accuracy
        </div>
        <button onClick={load} disabled={loading}
          style={{
            padding: '4px 10px', fontSize: '10px', fontWeight: 700,
            background: C.accentBg, border: `1px solid ${C.accentBorder}`,
            color: C.accent, borderRadius: '6px', cursor: loading ? 'wait' : 'pointer',
            fontFamily: 'inherit', textTransform: 'uppercase',
          }}>{loading ? 'Loading…' : data ? 'Refresh' : 'Load'}</button>
      </div>
      {error && (
        <div role='alert' style={{
          fontSize: '11px', color: C.red, background: C.redBg,
          border: `1px solid ${C.redBorder}`, borderRadius: '6px',
          padding: '6px 8px',
        }}>{error}</div>
      )}
      {data && !error && (
        <>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '6px' }}>
            <div style={{
              padding: '8px 10px', borderRadius: '6px',
              background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            }}>
              <div style={{ fontSize: '9px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Pass rate</div>
              <div style={{ fontSize: '16px', fontWeight: 800, color: rateColor, fontFamily: 'ui-monospace, monospace', marginTop: '2px' }}>
                {rate}
              </div>
            </div>
            <div style={{
              padding: '8px 10px', borderRadius: '6px',
              background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            }}>
              <div style={{ fontSize: '9px', color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Samples</div>
              <div style={{ fontSize: '16px', fontWeight: 800, color: C.text, fontFamily: 'ui-monospace, monospace', marginTop: '2px' }}>
                {typeof data.samples === 'number' ? data.samples.toLocaleString() : '—'}
              </div>
            </div>
          </div>
          {data.last_run != null && (
            <div style={{ marginTop: '6px', fontSize: '10px', color: C.textDim, textAlign: 'center' }}>
              Last run: {typeof data.last_run === 'number' ? new Date(data.last_run * 1000).toLocaleString() : data.last_run}
            </div>
          )}
          {data.per_domain && Object.keys(data.per_domain).length > 0 && (
            <div style={{ marginTop: '8px' }}>
              <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 700, marginBottom: '4px', textTransform: 'uppercase', letterSpacing: '0.08em' }}>By domain</div>
              {Object.entries(data.per_domain)
                .sort((a, b) => (b[1] ?? 0) - (a[1] ?? 0))
                .map(([dom, v]) => (
                  <div key={dom} style={{
                    display: 'flex', justifyContent: 'space-between',
                    padding: '4px 8px', fontSize: '11px',
                    borderBottom: `1px solid ${C.borderSubtle}`,
                  }}>
                    <span style={{ color: C.textSecondary }}>{dom}</span>
                    <span style={{ color: C.text, fontFamily: 'ui-monospace, monospace' }}>{pct(v)}</span>
                  </div>
                ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};
