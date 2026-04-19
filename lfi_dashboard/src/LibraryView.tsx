import React, { useEffect, useMemo, useRef, useState } from 'react';
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

type SortKey = 'name' | 'facts' | 'avg_quality' | 'domain' | 'vetted' | 'trust';

// c2-433 / #293: per-source trust resolution key. A source row carries both
// url and name; we key by url when present (stable identifier even when the
// display name changes upstream), else name, else the domain. Matches the
// convention the backend's source_trust table is expected to use.
const sourceKey = (s: SourceRow): string =>
  (s.url || s.name || s.domain || '').toString();

export const LibraryView: React.FC<LibraryViewProps> = ({ C, host, isDesktop }) => {
  const [sources, setSources] = useState<SourceRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [q, setQ] = useState('');
  const [sort, setSort] = useState<{ key: SortKey; dir: 'asc' | 'desc' }>({ key: 'facts', dir: 'desc' });
  // c2-433 / #293: source-trust map loaded from /api/sources/trust. Keyed by
  // sourceKey(row) and merged over any trust field already present on the
  // SourceRow (trustMap wins — it reflects user edits). pendingTrust tracks
  // which keys have an in-flight PUT so the slider can show a saving hint.
  // trustErr holds the most recent PUT failure for toast-less surfacing.
  const [trustMap, setTrustMap] = useState<Record<string, number>>({});
  const [pendingTrust, setPendingTrust] = useState<Record<string, 'saving' | 'saved' | 'failed'>>({});
  const trustTimersRef = useRef<Record<string, number>>({});
  const [autoResolving, setAutoResolving] = useState(false);
  const [autoResolveResult, setAutoResolveResult] = useState<string | null>(null);
  // c2-433: Copy-JSON export feedback. 2s Copy → Copied ✓ flip matching
  // the Drift/Ledger/Runs/KB export pattern.
  const [copiedAt, setCopiedAt] = useState<number>(0);
  // c2-433 / #285: corpus marketplace — ranked top-N by composite score
  // (0.4·trust + 0.3·avg_q + 0.2·vetted + 0.1·log_size). Silent-fail so
  // the rest of Library renders even if the endpoint isn't live yet.
  const [marketplace, setMarketplace] = useState<any[] | null>(null);
  // c2-433 / #311: per-source quality dimensions keyed by source key.
  // Surfaced via click-to-expand on a marketplace row so operators can
  // diagnose why a corpus ranks low without leaving the panel.
  const [qualityMap, setQualityMap] = useState<Record<string, any>>({});
  const [expandedMarketKey, setExpandedMarketKey] = useState<string | null>(null);

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

  // c2-433 / #293: fetch the source-trust map on mount (and refresh whenever
  // the sources list reloads). Tolerant to three response shapes: array of
  // {source, trust}, {trust: {source: value}} map, or {sources: [{source,
  // trust}]}. Silent on failure (falls back to whatever trust fields were
  // inlined in /api/library/sources).
  const loadTrust = async () => {
    try {
      const r = await fetch(`http://${host}:3000/api/sources/trust`);
      if (!r.ok) return;
      const data = await r.json();
      const map: Record<string, number> = {};
      if (Array.isArray(data)) {
        for (const row of data) {
          const k: string = row.source || row.source_id || row.url || row.name || '';
          if (k && typeof row.trust === 'number') map[k] = row.trust;
        }
      } else if (data && typeof data === 'object') {
        if (data.trust && typeof data.trust === 'object' && !Array.isArray(data.trust)) {
          for (const [k, v] of Object.entries(data.trust)) {
            if (typeof v === 'number') map[k] = v;
          }
        } else if (Array.isArray(data.sources)) {
          for (const row of data.sources) {
            const k: string = row.source || row.source_id || row.url || row.name || '';
            if (k && typeof row.trust === 'number') map[k] = row.trust;
          }
        }
      }
      setTrustMap(map);
    } catch { /* peripheral — silent */ }
  };
  useEffect(() => { loadTrust(); /* eslint-disable-next-line */ }, [host]);

  // c2-433 / #285: fetch ranked marketplace. Top 10 is enough for the
  // panel — operators scanning for best sources to mine get the same
  // information without pagination. Tolerant parser: array / {sources}
  // / {items} / {ranked} wrappers.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await fetch(`http://${host}:3000/api/corpus/marketplace?limit=10`);
        if (!r.ok) return;
        const data = await r.json();
        if (cancelled) return;
        const list: any[] = Array.isArray(data) ? data
          : Array.isArray(data?.sources) ? data.sources
          : Array.isArray(data?.items) ? data.items
          : Array.isArray(data?.ranked) ? data.ranked
          : [];
        setMarketplace(list);
      } catch { /* silent */ }
    })();
    return () => { cancelled = true; };
  }, [host]);

  // c2-433 / #311: fetch per-source quality dimensions. Tolerant parser —
  // accepts array of rows keyed by source|name|url, or an object map keyed
  // directly by source. Normalize into a Record<sourceKey, dims>. Silent-
  // fail preserves the marketplace panel when the endpoint isnt live.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await fetch(`http://${host}:3000/api/library/quality`);
        if (!r.ok) return;
        const data = await r.json();
        if (cancelled) return;
        const map: Record<string, any> = {};
        const pickKey = (row: any): string | null =>
          row.source || row.source_id || row.url || row.name || null;
        if (Array.isArray(data)) {
          for (const row of data) { const k = pickKey(row); if (k) map[k] = row; }
        } else if (data && typeof data === 'object') {
          if (Array.isArray(data.sources)) {
            for (const row of data.sources) { const k = pickKey(row); if (k) map[k] = row; }
          } else if (Array.isArray(data.items)) {
            for (const row of data.items) { const k = pickKey(row); if (k) map[k] = row; }
          } else {
            // Plain map shape: {source_key: {dims}}
            for (const [k, v] of Object.entries(data)) {
              if (v && typeof v === 'object') map[k] = v;
            }
          }
        }
        setQualityMap(map);
      } catch { /* silent */ }
    })();
    return () => { cancelled = true; };
  }, [host]);

  // c2-433 / #293: debounced PUT. Optimistic local update on each slider
  // move; actual /api/sources/trust write fires 400ms after the last change
  // so dragging the slider doesn't generate a burst of writes.
  const pushTrust = (key: string, value: number) => {
    setTrustMap(prev => ({ ...prev, [key]: value }));
    const existing = trustTimersRef.current[key];
    if (existing) window.clearTimeout(existing);
    trustTimersRef.current[key] = window.setTimeout(async () => {
      setPendingTrust(prev => ({ ...prev, [key]: 'saving' }));
      try {
        const r = await fetch(`http://${host}:3000/api/sources/trust`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ source: key, source_id: key, trust: value }),
        });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        setPendingTrust(prev => ({ ...prev, [key]: 'saved' }));
        window.setTimeout(() => {
          setPendingTrust(prev => { const n = { ...prev }; delete n[key]; return n; });
        }, 1200);
      } catch {
        setPendingTrust(prev => ({ ...prev, [key]: 'failed' }));
        window.setTimeout(() => {
          setPendingTrust(prev => { const n = { ...prev }; delete n[key]; return n; });
        }, 3000);
      }
    }, 400);
  };

  // c2-433 / #293: bulk auto-resolve contradictions using current source_trust
  // weights. One-shot POST; result string surfaces inline in the header for
  // ~4s (e.g. "Resolved 3 of 7"). Failures show the error.
  const runAutoResolve = async () => {
    setAutoResolving(true);
    setAutoResolveResult(null);
    try {
      const r = await fetch(`http://${host}:3000/api/contradictions/auto-resolve`, { method: 'POST' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json().catch(() => ({}));
      const resolved = data.resolved ?? data.count ?? data.n ?? '?';
      const total = data.total ?? data.pending ?? null;
      setAutoResolveResult(total != null ? `Resolved ${resolved} of ${total}` : `Resolved ${resolved}`);
    } catch (e: any) {
      setAutoResolveResult(`Auto-resolve failed: ${String(e?.message || e || 'unknown')}`);
    } finally {
      setAutoResolving(false);
      window.setTimeout(() => setAutoResolveResult(null), 4000);
    }
  };

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
        {autoResolveResult && (
          <span role='status'
            title={autoResolveResult}
            style={{
              fontSize: T.typography.sizeXs,
              color: autoResolveResult.startsWith('Auto-resolve failed') ? C.red : C.green,
              fontFamily: T.typography.fontMono,
              maxWidth: '260px', overflow: 'hidden',
              textOverflow: 'ellipsis', whiteSpace: 'nowrap',
            }}>{autoResolveResult}</span>
        )}
        <button onClick={runAutoResolve} disabled={autoResolving}
          title='Apply source_trust weights to the contradictions ledger'
          aria-label='Auto-resolve contradictions'
          style={{
            background: autoResolving ? C.bgInput : 'transparent',
            border: `1px solid ${C.borderSubtle}`,
            color: C.textMuted,
            borderRadius: T.radii.sm,
            cursor: autoResolving ? 'wait' : 'pointer',
            padding: '4px 10px',
            fontFamily: 'inherit',
            fontSize: T.typography.sizeXs,
            fontWeight: T.typography.weightSemibold,
            whiteSpace: 'nowrap',
          }}>{autoResolving ? 'Resolving…' : 'Auto-resolve'}</button>
        {lastUpdated != null && (
          <span style={{ fontSize: T.typography.sizeXs, color: C.textDim, fontFamily: T.typography.fontMono }}>
            Updated {formatRelative(lastUpdated)}
          </span>
        )}
        {/* c2-433: Copy-JSON export for the Library snapshot. Bundles
            sources + marketplace top-N + trust map + quality dims into
            one paste. 2s Copied ✓ feedback. Disabled when sources null. */}
        <button
          disabled={!sources || sources.length === 0}
          onClick={async () => {
            const payload = {
              exported_at: new Date().toISOString(),
              sources: sources || [],
              marketplace: marketplace || [],
              trust: trustMap,
              quality: qualityMap,
            };
            try {
              await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
              setCopiedAt(Date.now());
              window.setTimeout(() => setCopiedAt(0), 2000);
            } catch { /* clipboard blocked */ }
          }}
          title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${sources?.length ?? 0} sources + marketplace + trust/quality as JSON`}
          style={{
            background: copiedAt > 0 ? `${C.green}18` : 'transparent',
            border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
            color: copiedAt > 0 ? C.green : (sources && sources.length > 0 ? C.textMuted : C.textDim),
            borderRadius: T.radii.sm,
            cursor: (!sources || sources.length === 0) ? 'not-allowed' : 'pointer',
            padding: '4px 10px', fontFamily: 'inherit',
            fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
            opacity: (!sources || sources.length === 0) ? 0.5 : 1,
            whiteSpace: 'nowrap',
          }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
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

            {/* c2-433 / #285: corpus marketplace top-10. Ranked by composite
                score (0.4·trust + 0.3·avg_q + 0.2·vetted + 0.1·log_size)
                per the endpoint's spec. Compact list above the full sources
                DataTable so operators spot the high-value sources first.
                Hidden when fetch failed or returned empty. */}
            {marketplace && marketplace.length > 0 && (
              <div style={{ marginBottom: T.spacing.xl }}>
                <div style={{
                  display: 'flex', alignItems: 'baseline', gap: T.spacing.sm,
                  marginBottom: T.spacing.sm, flexWrap: 'wrap',
                }}>
                  <h3 style={{
                    margin: 0, fontSize: T.typography.sizeMd,
                    fontWeight: T.typography.weightBold, color: C.text,
                    textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
                  }}>Marketplace</h3>
                  <span title='Composite: 0.4·trust + 0.3·avg_q + 0.2·vetted + 0.1·log_size'
                    style={{
                      fontSize: T.typography.sizeXs, color: C.textMuted,
                      fontFamily: T.typography.fontMono,
                    }}>top {marketplace.length} · composite score</span>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  {marketplace.map((row: any, i: number) => {
                    const name: string = row.name || row.url || row.source || '(unnamed)';
                    const url: string | undefined = row.url;
                    const rowKey: string = row.source || row.source_id || url || name;
                    const score: number | null = typeof row.score === 'number' ? row.score
                      : typeof row.composite === 'number' ? row.composite
                      : typeof row.rank_score === 'number' ? row.rank_score
                      : null;
                    const trust = typeof row.trust === 'number' ? row.trust : null;
                    const avgQ = typeof row.avg_quality === 'number' ? row.avg_quality : (typeof row.avg_q === 'number' ? row.avg_q : null);
                    const vetted = row.vetted === true;
                    const facts = typeof row.facts === 'number' ? row.facts : (typeof row.log_size === 'number' ? row.log_size : null);
                    const scoreColor = score == null ? C.textMuted : score >= 0.7 ? C.green : score >= 0.4 ? C.yellow : C.red;
                    const isExpanded = expandedMarketKey === rowKey;
                    const dims = qualityMap[rowKey] || null;
                    return (
                      <React.Fragment key={`${name}-${i}`}>
                        <button type='button'
                          onClick={() => setExpandedMarketKey(prev => prev === rowKey ? null : rowKey)}
                          aria-expanded={isExpanded}
                          aria-label={`${isExpanded ? 'Collapse' : 'Expand'} quality dimensions for ${name}`}
                          style={{
                            // c2-433 / mobile: flex-wrap lets the inline
                            // stats chip drop below the name on narrow
                            // viewports instead of crushing the score.
                            display: 'flex', alignItems: 'center',
                            gap: T.spacing.sm, padding: '8px 12px',
                            background: isExpanded ? C.bgInput : C.bgCard,
                            border: `1px solid ${isExpanded ? C.accentBorder : C.borderSubtle}`,
                            borderRadius: T.radii.md, minWidth: 0,
                            cursor: 'pointer', textAlign: 'left',
                            fontFamily: 'inherit', color: C.text,
                            width: '100%', flexWrap: 'wrap',
                          }}>
                          <span style={{
                            fontSize: '10px', color: C.textMuted, fontFamily: T.typography.fontMono,
                            fontWeight: T.typography.weightBold, minWidth: '22px',
                            textAlign: 'right',
                          }}>#{i + 1}</span>
                          <span style={{
                            flex: '1 1 140px', minWidth: 0, overflow: 'hidden',
                            textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                            color: C.text, fontFamily: T.typography.fontMono,
                            fontSize: T.typography.sizeSm,
                          }} title={url || name}>{name}</span>
                          <span style={{
                            display: 'flex', gap: '6px', alignItems: 'center',
                            flexShrink: 0, fontSize: '10px', flexWrap: 'wrap',
                            fontFamily: T.typography.fontMono, color: C.textDim,
                          }}>
                            {trust != null && <span title={`trust ${trust.toFixed(2)}`}>t:{trust.toFixed(2)}</span>}
                            {avgQ != null && <span title={`avg quality ${avgQ.toFixed(2)}`} style={{ color: qualityColor(avgQ) }}>q:{avgQ.toFixed(2)}</span>}
                            {vetted && <span title='Vetted source' style={{ color: C.green }}>✓</span>}
                            {facts != null && <span title={`${facts.toLocaleString()} facts`}>{compactNum(facts)}f</span>}
                          </span>
                          <span style={{
                            minWidth: '52px', textAlign: 'right',
                            fontFamily: T.typography.fontMono,
                            fontSize: T.typography.sizeSm, fontWeight: T.typography.weightBlack,
                            color: scoreColor,
                          }}>{score != null ? score.toFixed(3) : '—'}</span>
                          <span aria-hidden='true' style={{
                            color: C.textMuted, fontSize: '10px', minWidth: '12px', textAlign: 'center',
                          }}>{isExpanded ? '▴' : '▾'}</span>
                        </button>
                        {/* c2-433 / #311: expanded quality dimensions row.
                            Renders the 6 per-source signals from
                            /api/library/quality so operators see WHY a
                            source lands where it does on the composite
                            score. When qualityMap[rowKey] is missing, show
                            an informational placeholder. */}
                        {isExpanded && (
                          <div style={{
                            padding: '10px 14px',
                            background: C.bgCard,
                            border: `1px solid ${C.accentBorder}`, borderTop: 'none',
                            borderRadius: `0 0 ${T.radii.md} ${T.radii.md}`,
                            marginTop: '-4px', marginBottom: '4px',
                          }}>
                            {!dims ? (
                              <div style={{
                                fontSize: T.typography.sizeXs, color: C.textDim, fontStyle: 'italic',
                              }}>
                                Quality dimensions not yet loaded for this source. /api/library/quality may not expose it.
                              </div>
                            ) : (() => {
                              const pct = (v: any) => typeof v === 'number'
                                ? `${(v * 100).toFixed(0)}%` : '—';
                              const num = (v: any) => typeof v === 'number'
                                ? v.toLocaleString() : '—';
                              const cell = (label: string, value: string, tone: string, title: string) => (
                                <div title={title} style={{
                                  display: 'flex', flexDirection: 'column', gap: '2px',
                                  minWidth: 0,
                                }}>
                                  <span style={{
                                    fontSize: '9px', color: C.textMuted, fontWeight: 700,
                                    textTransform: 'uppercase', letterSpacing: '0.06em',
                                  }}>{label}</span>
                                  <span style={{
                                    fontSize: T.typography.sizeSm, color: tone,
                                    fontFamily: T.typography.fontMono, fontWeight: 700,
                                  }}>{value}</span>
                                </div>
                              );
                              const fc = dims.fact_count ?? dims.facts;
                              const aq = dims.avg_quality ?? dims.avg_q;
                              const vr = dims.vetted_ratio;
                              const pc = dims.provenance_coverage ?? dims.provenance_cov;
                              const cr = dims.contradiction_rate;
                              const tr = dims.trust;
                              return (
                                <div style={{
                                  display: 'grid',
                                  gridTemplateColumns: 'repeat(auto-fit, minmax(110px, 1fr))',
                                  gap: T.spacing.sm,
                                }}>
                                  {cell('Facts', num(fc), C.text, 'Total facts ingested from this source')}
                                  {cell('Avg Q', typeof aq === 'number' ? aq.toFixed(2) : '—', typeof aq === 'number' ? qualityColor(aq) : C.textMuted, 'Average quality score across this sources facts')}
                                  {cell('Vetted', pct(vr), typeof vr === 'number' ? (vr >= 0.8 ? C.green : vr >= 0.5 ? C.yellow : C.red) : C.textMuted, 'Share of facts that have been human-vetted')}
                                  {cell('Provenance', pct(pc), typeof pc === 'number' ? (pc >= 0.8 ? C.green : pc >= 0.5 ? C.yellow : C.red) : C.textMuted, 'Share of facts with full provenance metadata')}
                                  {cell('Contradict', pct(cr), typeof cr === 'number' ? (cr <= 0.1 ? C.green : cr <= 0.25 ? C.yellow : C.red) : C.textMuted, 'Share of facts that contradict other sources')}
                                  {cell('Trust', typeof tr === 'number' ? tr.toFixed(2) : '—', typeof tr === 'number' ? qualityColor(tr) : C.textMuted, 'Current trust weight (adjustable via the slider below)')}
                                </div>
                              );
                            })()}
                          </div>
                        )}
                      </React.Fragment>
                    );
                  })}
                </div>
              </div>
            )}

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
              // task 213: virtualization removes the prior 500-row cap. The
              // full filtered set is handed to TableVirtuoso, which only paints
              // the visible slice. 360+ source rows used to all paint at once
              // (60vh container, ~120 visible) — now the cost scales with the
              // viewport, not the corpus size.
              type Row = typeof filtered[number];
              const rows = filtered;
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
                  // c2-433 / #293: per-source trust slider. The effective
                  // value is trustMap[key] (user-edited + server-backed)
                  // falling back to the inline row.trust (whatever
                  // /api/library/sources shipped). Slider drag updates the
                  // local map immediately; the PUT fires 400ms after the
                  // last move via pushTrust. A tiny status dot to the right
                  // of the slider signals the in-flight PUT state (saving
                  // / saved / failed).
                  id: 'trust', header: 'Trust', align: 'center',
                  sortKey: (s) => {
                    const k = sourceKey(s);
                    const v = k && trustMap[k] != null ? trustMap[k] : (typeof s.trust === 'number' ? s.trust : -1);
                    return v;
                  },
                  accessor: (s) => {
                    const k = sourceKey(s);
                    const v = k && trustMap[k] != null ? trustMap[k] : (typeof s.trust === 'number' ? s.trust : 0.5);
                    const status = k ? pendingTrust[k] : undefined;
                    const dot = status === 'saving' ? { bg: C.yellow, title: 'Saving…' }
                      : status === 'saved' ? { bg: C.green, title: 'Saved' }
                      : status === 'failed' ? { bg: C.red, title: 'Save failed' }
                      : null;
                    return (
                      <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', justifyContent: 'center' }}>
                        <input type='range' min={0} max={1} step={0.05} value={v}
                          onChange={(e) => { if (k) pushTrust(k, Number(e.target.value)); }}
                          disabled={!k}
                          aria-label={`Trust for ${s.name || s.url || 'source'}`}
                          title={k ? `Trust weight for ${s.name || s.url} — drag to adjust` : 'Source lacks a stable key'}
                          style={{
                            width: '80px', height: '14px',
                            accentColor: qualityColor(v),
                            cursor: k ? 'pointer' : 'not-allowed',
                          }}
                        />
                        <span style={{
                          fontFamily: T.typography.fontMono, fontSize: '10px',
                          color: qualityColor(v), minWidth: '28px',
                        }}>{v.toFixed(2)}</span>
                        {dot && (
                          <span title={dot.title} style={{
                            width: '6px', height: '6px', borderRadius: '50%',
                            background: dot.bg, flexShrink: 0,
                          }} />
                        )}
                      </span>
                    );
                  },
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
                <DataTable<Row> C={C} rows={rows} columns={cols}
                  rowKey={(s, /* i */) => `${s.name || s.url || ''}-${s.facts ?? 0}`}
                  sort={{ col: sort.key, dir: sort.dir }}
                  onSortChange={(next) => setSort({ key: next.col as SortKey, dir: next.dir })}
                  virtualize virtualizeHeight='60vh' />
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
