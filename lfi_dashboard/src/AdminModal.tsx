import React, { useRef, useState, useMemo, useEffect } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';
import { compactNum } from './util';

// Full-screen admin modal per c0-017. Six tabs: Dashboard / Domains /
// Training / Quality / System / Logs. Replaces the prior sidebar-slide admin
// affordance which users found cramped. Sortable + filterable tables, big-
// number dashboard cards, bar-chart visualisations of domains + quality.

export type AdminTab = 'dashboard' | 'domains' | 'training' | 'quality' | 'system' | 'logs';

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
  quality_distribution?: Record<string, number>;
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
  C, host, onClose, factsCount, sourcesCount, initialTab = 'dashboard',
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  const [tab, setTab] = useState<AdminTab>(initialTab);
  const [domains, setDomains] = useState<DomainRow[] | null>(null);
  const [accuracy, setAccuracy] = useState<AccuracyShape | null>(null);
  const [quality, setQuality] = useState<QualityShape | null>(null);
  const [sysInfo, setSysInfo] = useState<SystemShape | null>(null);
  const [logs, setLogs] = useState<string[] | null>(null);
  const [err, setErr] = useState<Record<AdminTab, string | null>>({
    dashboard: null, domains: null, training: null, quality: null, system: null, logs: null,
  });
  const [loading, setLoading] = useState<AdminTab | null>(null);

  const fetchJson = async <T,>(path: string, signal: AbortSignal): Promise<T> => {
    const res = await fetch(`http://${host}:3000${path}`, { signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json() as Promise<T>;
  };
  const loadTab = async (t: AdminTab) => {
    setLoading(t);
    setErr(e => ({ ...e, [t]: null }));
    const ctrl = new AbortController();
    const to = setTimeout(() => ctrl.abort(), 10000);
    try {
      if (t === 'dashboard' || t === 'domains') {
        const data: any = await fetchJson('/api/admin/training/domains', ctrl.signal);
        const arr: DomainRow[] = Array.isArray(data?.domains) ? data.domains : Array.isArray(data) ? data : [];
        setDomains(arr.sort((a, b) => b.facts - a.facts));
      }
      if (t === 'dashboard' || t === 'training') {
        setAccuracy(await fetchJson('/api/admin/training/accuracy', ctrl.signal));
      }
      if (t === 'dashboard' || t === 'quality') {
        setQuality(await fetchJson('/api/quality/report', ctrl.signal));
      }
      if (t === 'dashboard' || t === 'system') {
        setSysInfo(await fetchJson('/api/system/info', ctrl.signal));
      }
      if (t === 'logs') {
        try {
          const data: any = await fetchJson('/api/admin/logs', ctrl.signal);
          setLogs(Array.isArray(data?.lines) ? data.lines : Array.isArray(data) ? data : []);
        } catch {
          setLogs([]);
        }
      }
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

  // ---- Tables: shared sort + filter state ----
  const [domainSort, setDomainSort] = useState<{ key: keyof DomainRow; dir: 'asc' | 'desc' }>({ key: 'facts', dir: 'desc' });
  const [domainFilter, setDomainFilter] = useState('');
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
        background: 'rgba(0,0,0,0.65)',
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
          <button onClick={onClose} aria-label='Close admin'
            style={{
              background: 'transparent', border: 'none', color: C.textMuted,
              fontSize: '22px', cursor: 'pointer', padding: '4px 10px',
            }}>{'\u2715'}</button>
        </div>

        {/* Tab bar */}
        <div role='tablist' aria-label='Admin sections'
          style={{
            display: 'flex', gap: '2px', padding: '0 22px',
            borderBottom: `1px solid ${C.borderSubtle}`, overflowX: 'auto',
          }}>
          {([
            { id: 'dashboard', label: 'Dashboard' },
            { id: 'domains', label: 'Domains' },
            { id: 'training', label: 'Training' },
            { id: 'quality', label: 'Quality' },
            { id: 'system', label: 'System' },
            { id: 'logs', label: 'Logs' },
          ] as const).map(t => (
            <button key={t.id} onClick={() => setTab(t.id)}
              role='tab' aria-selected={tab === t.id}
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
              <div style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: T.spacing.md, marginBottom: T.spacing.xl,
              }}>
                {[
                  { label: 'Facts', value: factsCount ? compactNum(factsCount) : '—', color: C.purple },
                  { label: 'Sources', value: sourcesCount ? String(sourcesCount) : '—', color: C.green },
                  { label: 'Domains', value: domains ? String(domains.length) : '—', color: C.accent },
                  { label: 'Accuracy', value: (() => {
                    const p = pctNorm(accuracy?.pass_rate ?? accuracy?.accuracy);
                    return p != null ? `${p.toFixed(1)}%` : '—';
                  })(), color: C.yellow },
                  { label: 'Adversarial', value: quality?.adversarial_count != null ? compactNum(quality.adversarial_count) : '—', color: C.red },
                  { label: 'CPU temp', value: sysInfo?.cpu_temp_c != null ? `${sysInfo.cpu_temp_c.toFixed(0)}°C` : '—', color: (sysInfo?.cpu_temp_c ?? 0) > 65 ? C.red : C.green },
                ].map(card => (
                  <div key={card.label} style={{
                    padding: '16px 18px', borderRadius: T.radii.xl,
                    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  }}>
                    <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                      {card.label}
                    </div>
                    <div style={{ fontSize: '28px', fontWeight: T.typography.weightBlack, color: card.color, marginTop: '4px', fontFamily: 'ui-monospace, SFMono-Regular, monospace' }}>
                      {card.value}
                    </div>
                  </div>
                ))}
              </div>
              {/* Domain bar chart */}
              {domains && domains.length > 0 && (() => {
                const top = [...domains].sort((a, b) => b.facts - a.facts).slice(0, 10);
                const max = Math.max(...top.map(d => d.facts), 1);
                return (
                  <div>
                    <div style={{ fontSize: '11px', fontWeight: T.typography.weightBold, color: C.textMuted, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose, marginBottom: '10px' }}>
                      Top 10 domains by fact count
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {top.map(d => (
                        <div key={d.domain} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                          <span style={{ width: '140px', fontSize: '12px', color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{d.domain}</span>
                          <div style={{ flex: 1, background: C.bgInput, height: '18px', borderRadius: T.radii.xs, overflow: 'hidden' }}>
                            <div style={{
                              width: `${(d.facts / max) * 100}%`, height: '100%',
                              background: countColor(d.facts),
                              transition: 'width 0.4s',
                            }} />
                          </div>
                          <span style={{ width: '72px', textAlign: 'right', fontSize: '12px', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>
                            {d.facts.toLocaleString()}
                          </span>
                        </div>
                      ))}
                    </div>
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
              {err.domains && <AdminErr C={C} msg={err.domains} />}
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
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: countColor(d.facts), fontWeight: T.typography.weightBold, fontFamily: 'ui-monospace, monospace' }}>
                          {d.facts.toLocaleString()}
                        </td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: typeof d.avg_quality === 'number' ? qualityColor(d.avg_quality) : C.textMuted, fontFamily: 'ui-monospace, monospace' }}>
                          {typeof d.avg_quality === 'number' ? d.avg_quality.toFixed(2) : '—'}
                        </td>
                        <td style={{ padding: '10px 14px', textAlign: 'right', color: C.textMuted, fontFamily: 'ui-monospace, monospace' }}>
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
              {err.training && <AdminErr C={C} msg={err.training} />}
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
                          <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
                            Pass rate
                          </div>
                          <div style={{
                            fontSize: '28px', fontWeight: T.typography.weightBlack,
                            color: p != null ? (p >= 95 ? C.green : p >= 80 ? C.yellow : C.red) : C.textMuted,
                            marginTop: '4px', fontFamily: 'ui-monospace, monospace',
                          }}>{p != null ? `${p.toFixed(1)}%` : '—'}</div>
                        </div>
                      );
                    })()}
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Samples</div>
                      <div style={{ fontSize: '28px', fontWeight: T.typography.weightBlack, color: C.text, marginTop: '4px', fontFamily: 'ui-monospace, monospace' }}>
                        {typeof accuracy.samples === 'number' ? accuracy.samples.toLocaleString() : '—'}
                      </div>
                    </div>
                    <div style={{ padding: '16px 18px', borderRadius: T.radii.xl, background: C.bgInput, border: `1px solid ${C.borderSubtle}` }}>
                      <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>Last run</div>
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
                              <span style={{ width: '64px', textAlign: 'right', fontSize: '12px', fontFamily: 'ui-monospace, monospace', color: C.textMuted }}>{p.toFixed(1)}%</span>
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
              {err.quality && <AdminErr C={C} msg={err.quality} />}
              {quality ? (
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: T.spacing.md }}>
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
              {err.system && <AdminErr C={C} msg={err.system} />}
              {sysInfo ? (
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: T.spacing.md }}>
                  <DashCard C={C} label='Hostname' value={sysInfo.hostname || '—'} color={C.accent} />
                  <DashCard C={C} label='OS' value={sysInfo.os || '—'} color={C.textSecondary} />
                  <DashCard C={C} label='CPU cores' value={sysInfo.cpu_count != null ? String(sysInfo.cpu_count) : '—'} color={C.purple} />
                  <DashCard C={C} label='CPU temp' value={sysInfo.cpu_temp_c != null ? `${sysInfo.cpu_temp_c.toFixed(0)}°C` : '—'} color={(sysInfo.cpu_temp_c ?? 0) > 65 ? C.red : C.green} />
                  <DashCard C={C} label='RAM available' value={sysInfo.ram_available_mb != null ? `${sysInfo.ram_available_mb} MB` : '—'} color={C.accent} />
                  <DashCard C={C} label='Disk free' value={fmtBytes(sysInfo.disk_root_free_bytes)} color={C.green} />
                  <DashCard C={C} label='Disk total' value={fmtBytes(sysInfo.disk_root_total_bytes)} color={C.textSecondary} />
                  <DashCard C={C} label='Uptime' value={fmtSeconds(sysInfo.uptime_seconds)} color={C.yellow} />
                  {sysInfo.ollama && (
                    <DashCard C={C} label='Ollama' value={sysInfo.ollama.status || '—'} color={sysInfo.ollama.status === 'up' ? C.green : C.red} />
                  )}
                </div>
              ) : (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'system' ? 'Loading…' : 'No system data loaded.'}
                </div>
              )}
            </div>
          )}

          {/* ---------- Logs ---------- */}
          {tab === 'logs' && (
            <div>
              {err.logs && <AdminErr C={C} msg={err.logs} />}
              {logs === null && !err.logs && (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  {loading === 'logs' ? 'Loading…' : 'Logs endpoint not available yet.'}
                </div>
              )}
              {logs && logs.length === 0 && !err.logs && (
                <div style={{ padding: '40px', textAlign: 'center', color: C.textMuted }}>
                  Log endpoint exists but returned no lines.
                </div>
              )}
              {logs && logs.length > 0 && (
                <pre style={{
                  margin: 0, padding: '16px', background: C.bgInput,
                  border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
                  fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: T.typography.sizeMd,
                  color: C.text, whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                  maxHeight: '60vh', overflowY: 'auto',
                }}>{logs.slice(-500).join('\n')}</pre>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// ---- Private helpers ----

const AdminErr: React.FC<{ C: any; msg: string }> = ({ C, msg }) => (
  <div role='alert' style={{
    padding: '12px 14px', marginBottom: T.spacing.md,
    background: C.redBg, border: `1px solid ${C.redBorder}`,
    color: C.red, borderRadius: T.radii.md, fontSize: T.typography.sizeMd,
  }}>
    <strong>Could not load:</strong> {msg}
  </div>
);

const DashCard: React.FC<{ C: any; label: string; value: string; color: string }> = ({ C, label, value, color }) => (
  <div style={{
    padding: '16px 18px', borderRadius: T.radii.xl,
    background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
  }}>
    <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: T.typography.weightBold, textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose }}>
      {label}
    </div>
    <div style={{ fontSize: '16px', fontWeight: T.typography.weightBlack, color, marginTop: '6px', fontFamily: 'ui-monospace, monospace', wordBreak: 'break-word' }}>
      {value}
    </div>
  </div>
);
