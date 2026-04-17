import React, { useEffect, useMemo, useState } from 'react';
import { T } from './tokens';
// c2-347: shared stat/summary card (replaces the local Stat helper).
import { StatCard } from './components/StatCard';
// c2-374 / BIG #180: DataTable adoption for the sources inventory.
import { DataTable } from './components';
import type { Column } from './components';
import { compactNum, formatRelative } from './util';

// c0-037 #3 / c2-329: standalone Library page. Fetches /api/library/sources
// (c0-035 #3 — the 360+ sources corpus) from :3002 first with :3000 fallback.
// Sortable filterable table + headline stats. Replaces the minor "Sources"
// column I added to ClassroomView's Library tab — this is the real surface
// Claude 0 asked for. Classroom's Library tab stays as a quick in-context
// glance; this page is the full inventory.

interface SourceRow {
  url?: string;
  name?: string;
  domain?: string;
  trust?: number;
  facts?: number;
  avg_quality?: number;
  vetted?: boolean;
}

export interface LibraryViewProps {
  C: any;
  host: string;
  isDesktop: boolean;
}

type SortKey = 'name' | 'facts' | 'avg_quality' | 'domain' | 'vetted';

export const LibraryView: React.FC<LibraryViewProps> = ({ C, host, isDesktop }) => {
  const [sources, setSources] = useState<SourceRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [q, setQ] = useState('');
  const [sort, setSort] = useState<{ key: SortKey; dir: 'asc' | 'desc' }>({ key: 'facts', dir: 'desc' });

  const load = async () => {
    setLoading(true);
    setErr(null);
    const tryPort = async (port: number) => {
      const ctrl = new AbortController();
      const to = setTimeout(() => ctrl.abort(), 6000);
      try {
        const r = await fetch(`http://${host}:${port}/api/library/sources`, { signal: ctrl.signal });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return await r.json();
      } finally { clearTimeout(to); }
    };
    try {
      let data: any;
      try { data = await tryPort(3002); } catch { data = await tryPort(3000); }
      const arr: SourceRow[] = Array.isArray(data?.sources) ? data.sources : Array.isArray(data) ? data : [];
      setSources(arr);
      setLastUpdated(Date.now());
    } catch (e: any) {
      const m = String(e?.message || e || 'fetch failed');
      setErr(m.includes('abort') ? 'Library service timed out.' : m);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { load(); /* eslint-disable-next-line */ }, []);

  const filtered = useMemo(() => {
    if (!sources) return [];
    const normQ = q.trim().toLowerCase();
    const base = normQ
      ? sources.filter(s =>
          (s.name || '').toLowerCase().includes(normQ) ||
          (s.url || '').toLowerCase().includes(normQ) ||
          (s.domain || '').toLowerCase().includes(normQ))
      : sources;
    const sorted = [...base].sort((a, b) => {
      const av: any = a[sort.key] ?? (sort.key === 'name' ? (a.url || '') : -Infinity);
      const bv: any = b[sort.key] ?? (sort.key === 'name' ? (b.url || '') : -Infinity);
      if (typeof av === 'string' || typeof bv === 'string') {
        const s = String(av).localeCompare(String(bv));
        return sort.dir === 'asc' ? s : -s;
      }
      if (typeof av === 'boolean' || typeof bv === 'boolean') {
        const s = (av ? 1 : 0) - (bv ? 1 : 0);
        return sort.dir === 'asc' ? s : -s;
      }
      return sort.dir === 'asc' ? (av as number) - (bv as number) : (bv as number) - (av as number);
    });
    return sorted;
  }, [sources, q, sort]);

  const totals = useMemo(() => {
    if (!sources) return null;
    const facts = sources.reduce((s, r) => s + (r.facts || 0), 0);
    const vetted = sources.filter(s => s.vetted).length;
    const unvetted = sources.length - vetted;
    const avgQ = (() => {
      const withQ = sources.filter(s => typeof s.avg_quality === 'number');
      if (withQ.length === 0) return null;
      return withQ.reduce((acc, r) => acc + (r.avg_quality as number), 0) / withQ.length;
    })();
    return { count: sources.length, facts, vetted, unvetted, avgQ };
  }, [sources]);

  const sortArrow = (key: SortKey) => sort.key === key
    ? (sort.dir === 'asc' ? ' \u25B2' : ' \u25BC') : '';
  const toggleSort = (key: SortKey) => setSort(prev => ({
    key,
    dir: prev.key === key && prev.dir === 'desc' ? 'asc' : 'desc',
  }));

  const qualityColor = (v: number) => v >= 0.8 ? C.green : v >= 0.5 ? C.yellow : C.red;

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
      background: C.bg, color: C.text, overflow: 'hidden',
      animation: 'lfi-fadein 0.18s ease-out',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: T.spacing.md,
        padding: `${T.spacing.lg} ${T.spacing.xl}`,
        borderBottom: `1px solid ${C.borderSubtle}`, background: C.bgCard,
      }}>
        <h1 style={{
          margin: 0, fontSize: T.typography.sizeXl,
          fontWeight: T.typography.weightBlack, color: C.text,
          letterSpacing: T.typography.trackingCap, textTransform: 'uppercase',
        }}>Library</h1>
        <div style={{ flex: 1 }} />
        {lastUpdated != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: 'ui-monospace, monospace' }}>
            Updated {formatRelative(lastUpdated)}
          </span>
        )}
        <button onClick={load} disabled={loading} aria-label='Refresh library'
          title={loading ? 'Refreshing…' : 'Refresh'}
          style={{
            background: 'transparent', border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted, borderRadius: T.radii.sm,
            cursor: loading ? 'wait' : 'pointer',
            padding: '4px 8px', display: 'flex', alignItems: 'center',
            fontFamily: 'inherit',
          }}>
          <svg width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='currentColor'
            strokeWidth='2.2' strokeLinecap='round' strokeLinejoin='round'
            style={loading ? { animation: 'scc-lib-spin 0.8s linear infinite' } : undefined}>
            <polyline points='23 4 23 10 17 10' />
            <polyline points='1 20 1 14 7 14' />
            <path d='M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15' />
          </svg>
        </button>
        <style>{`@keyframes scc-lib-spin { to { transform: rotate(360deg); } }`}</style>
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
            <span><strong>Could not load library:</strong> {err}</span>
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
        {sources === null && !err && (
          <div aria-busy='true' style={{ padding: T.spacing.xxxl, textAlign: 'center', color: C.textMuted }}>
            Loading sources…
          </div>
        )}
        {sources && totals && (
          <>
            {/* Headline stats */}
            <div style={{
              display: 'grid', gridTemplateColumns: isDesktop ? 'repeat(auto-fit, minmax(180px, 1fr))' : 'repeat(2, 1fr)',
              gap: T.spacing.md, marginBottom: T.spacing.xl,
            }}>
              <StatCard C={C} label='Sources' value={String(totals.count)} color={C.accent} />
              <StatCard C={C} label='Facts' value={compactNum(totals.facts)} color={C.purple} />
              <StatCard C={C} label='Vetted' value={`${totals.vetted} / ${totals.count}`} color={C.green} />
              <StatCard C={C} label='Avg quality' value={totals.avgQ != null ? totals.avgQ.toFixed(2) : '—'} color={totals.avgQ != null ? qualityColor(totals.avgQ) : C.textMuted} />
            </div>

            {/* Filter */}
            <input
              type='search' value={q} onChange={e => setQ(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Escape' && q) { e.preventDefault(); setQ(''); } }}
              autoComplete='off' spellCheck={false}
              placeholder={`Filter ${totals.count} sources by name, URL, or domain…`}
              aria-label='Filter sources'
              style={{
                width: '100%', padding: '10px 12px', marginBottom: T.spacing.lg,
                background: C.bgCard, border: `1px solid ${C.borderSubtle}`,
                borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
                fontSize: T.typography.sizeBody, outline: 'none', boxSizing: 'border-box',
              }}
            />

            {filtered.length === 0 ? (
              <div style={{ padding: T.spacing.xl, textAlign: 'center', color: C.textMuted, fontSize: T.typography.sizeSm }}>
                No sources match "{q}".
              </div>
            ) : (() => {
              // c2-374 / BIG #180: DataTable adoption. Column descriptors
              // replace the 40-line inline <tr>/<td> block. Sort state
              // remains external (useState<{key, dir}>) so the toggle +
              // sortArrow callers above keep working -- DataTable consumes
              // it via the sort/onSortChange lift.
              type Row = typeof filtered[number];
              const cap = Math.min(filtered.length, 500);
              const rows = filtered.slice(0, cap);
              const cols: ReadonlyArray<Column<Row>> = [
                {
                  id: 'name', header: 'Source', align: 'left',
                  sortKey: (s) => (s.name || s.url || '').toLowerCase(),
                  accessor: (s) => {
                    const label = s.name || s.url || '(source)';
                    return (
                      <span title={s.url || label} style={{
                        color: C.text, fontFamily: T.typography.fontMono,
                        whiteSpace: 'nowrap', overflow: 'hidden',
                        textOverflow: 'ellipsis', display: 'inline-block',
                        maxWidth: '320px',
                      }}>{label}</span>
                    );
                  },
                },
                {
                  id: 'domain', header: 'Domain', align: 'left',
                  sortKey: (s) => (s.domain || '').toLowerCase(),
                  accessor: (s) => <span style={{ color: C.textSecondary }}>{s.domain || '\u2014'}</span>,
                },
                {
                  id: 'facts', header: 'Facts', align: 'right',
                  sortKey: (s) => typeof s.facts === 'number' ? s.facts : -1,
                  accessor: (s) => (
                    <span style={{ color: C.text, fontFamily: T.typography.fontMono }}>
                      {typeof s.facts === 'number' ? s.facts.toLocaleString() : '\u2014'}
                    </span>
                  ),
                },
                {
                  id: 'avg_quality', header: 'Avg Q', align: 'right',
                  sortKey: (s) => typeof s.avg_quality === 'number' ? s.avg_quality : -1,
                  accessor: (s) => (
                    <span style={{
                      color: typeof s.avg_quality === 'number' ? qualityColor(s.avg_quality) : C.textMuted,
                      fontFamily: T.typography.fontMono,
                    }}>{typeof s.avg_quality === 'number' ? s.avg_quality.toFixed(2) : '\u2014'}</span>
                  ),
                },
                {
                  id: 'vetted', header: 'Vetted', align: 'center',
                  sortKey: (s) => s.vetted == null ? 0 : s.vetted ? 2 : 1,
                  accessor: (s) => (
                    s.vetted == null ? <span style={{ color: C.textMuted }}>{'\u2014'}</span>
                    : s.vetted ? <span style={{ color: C.green, fontSize: T.typography.sizeBody }} aria-label='vetted' title='vetted'>{'\u2714'}</span>
                    : <span style={{ color: C.red, fontSize: T.typography.sizeBody }} aria-label='unvetted' title='unvetted'>{'\u2716'}</span>
                  ),
                },
              ];
              return (
                <div style={{ maxHeight: '60vh', overflowY: 'auto' }}>
                  <DataTable<Row> C={C} rows={rows} columns={cols}
                    rowKey={(s, /* i */) => `${s.name || s.url || ''}-${s.facts ?? 0}`}
                    sort={{ col: sort.key, dir: sort.dir }}
                    onSortChange={(next) => setSort({ key: next.col as SortKey, dir: next.dir })} />
                  {filtered.length > 500 && (
                    <div style={{ padding: '6px 12px', background: C.bgInput, borderTop: `1px solid ${C.borderSubtle}`, fontSize: T.typography.sizeXs, color: C.textDim, textAlign: 'center' }}>
                      Showing 500 of {filtered.length} matches. Filter to narrow.
                    </div>
                  )}
                </div>
              );
            })()}
          </>
        )}
      </div>
    </div>
  );
};

// ---- Private helpers ----

// c2-347: the local Stat helper moved to components/StatCard.tsx.

const Th: React.FC<{
  C: any; children: React.ReactNode; onClick: () => void; align?: 'left' | 'right' | 'center'; 'aria-sort': 'ascending' | 'descending' | 'none';
}> = ({ C, children, onClick, align = 'left', ...rest }) => (
  <th onClick={onClick}
    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick(); } }}
    role='button' tabIndex={0}
    aria-sort={rest['aria-sort']}
    style={{
      textAlign: align, padding: '8px 12px',
      fontWeight: T.typography.weightBold, color: C.textSecondary,
      background: C.bgInput, borderBottom: `1px solid ${C.borderSubtle}`,
      cursor: 'pointer', userSelect: 'none', position: 'sticky', top: 0, zIndex: 1,
    }}>{children}</th>
);
