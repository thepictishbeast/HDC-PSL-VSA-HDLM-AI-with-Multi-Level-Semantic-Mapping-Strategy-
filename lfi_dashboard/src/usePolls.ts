import { useEffect, useState } from 'react';

// Three polling hooks for the sidebar dashboard. Each manages its own interval
// + AbortController; parent just flips `active` to pause (unauthenticated).

// -------- /api/status (15s) --------
// Cadence bumped 5s → 15s per claude-0 #403: backend stats are cached
// server-side on a 60s background refresh, so polling faster than ~15s
// just re-reads the same Arc<AtomicI64>. 3× fewer requests, same data
// freshness. Latency sample still captures user-visible RTT.
export interface KgCounters { facts: number; concepts: number; sources: number; entropy: number }

export const useStatusPoll = (host: string, active: boolean) => {
  const [kg, setKg] = useState<KgCounters>({ facts: 0, concepts: 0, sources: 0, entropy: 0 });
  const [lastOk, setLastOk] = useState<number | null>(null);
  // Surfaces the most recent fetch error so the UI can distinguish "loading"
  // from "server returned 0" — previously both rendered as 0 with no signal.
  const [lastError, setLastError] = useState<string | null>(null);
  // Rolling 5-sample latency. Smoothed so a single slow request doesn't make
  // the badge flap; users see a stable "this is the typical RTT" number.
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  useEffect(() => {
    if (!active) return;
    const samples: number[] = [];
    const fetchStatus = async () => {
      const t0 = performance.now();
      try {
        // 8s timeout: backend DB can block on write-lock windows; don't freeze the UI.
        const ctrl = new AbortController();
        const to = setTimeout(() => ctrl.abort(), 8000);
        const res = await fetch(`http://${host}:3000/api/status`, { signal: ctrl.signal });
        clearTimeout(to);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();
        const dur = performance.now() - t0;
        samples.push(dur);
        if (samples.length > 5) samples.shift();
        setLatencyMs(samples.reduce((a, b) => a + b, 0) / samples.length);
        setKg(k => ({
          facts: typeof data.facts_count === 'number' ? data.facts_count : k.facts,
          concepts: typeof data.concepts_count === 'number' ? data.concepts_count : k.concepts,
          sources: typeof data.sources_count === 'number' ? data.sources_count : k.sources,
          entropy: typeof data.entropy === 'number' ? data.entropy : k.entropy,
        }));
        setLastOk(Date.now());
        setLastError(null);
      } catch (e: any) {
        setLastError(String(e?.message || e || 'fetch failed'));
        setLatencyMs(null);
      }
    };
    fetchStatus();
    const id = setInterval(fetchStatus, 15000);
    return () => clearInterval(id);
  }, [host, active]);
  return { kg, lastOk, lastError, latencyMs };
};

// -------- /api/quality/report (30s) --------
export interface QualityData {
  adversarial: number;
  psl_pass_rate: number | null;
  psl_status: string | null;
  psl_last_run: string | null;
  fts5_enabled: boolean;
  distinct_sources: number;
  stale: boolean;
}

export const useQualityPoll = (host: string, active: boolean) => {
  const [quality, setQuality] = useState<QualityData | null>(null);
  useEffect(() => {
    if (!active) return;
    let lastOk = 0;
    const fetchQuality = async () => {
      try {
        const ctrl = new AbortController();
        const to = setTimeout(() => ctrl.abort(), 12000);
        const res = await fetch(`http://${host}:3000/api/quality/report`, { signal: ctrl.signal });
        clearTimeout(to);
        const d = await res.json();
        lastOk = Date.now();
        setQuality({
          adversarial: typeof d.adversarial_count === 'number' ? d.adversarial_count : 0,
          psl_pass_rate: typeof d?.psl_calibration?.pass_rate === 'number' ? d.psl_calibration.pass_rate : null,
          psl_status: d?.psl_calibration?.status ?? null,
          psl_last_run: d?.psl_calibration?.last_run ?? null,
          fts5_enabled: !!d.fts5_enabled,
          distinct_sources: typeof d.distinct_sources === 'number' ? d.distinct_sources : 0,
          stale: false,
        });
      } catch (_) {
        setQuality(q => q ? { ...q, stale: true } : q);
      }
    };
    fetchQuality();
    const id = setInterval(() => {
      fetchQuality();
      if (lastOk && Date.now() - lastOk > 90000) {
        setQuality(q => q ? { ...q, stale: true } : q);
      }
    }, 30000);
    return () => clearInterval(id);
  }, [host, active]);
  return quality;
};

// -------- /api/system/info (60s) --------
export interface SysInfo {
  hostname?: string;
  os?: string;
  cpu_count?: number;
  cpu_model?: string;
  disk_free?: number;
  disk_total?: number;
}

export const useSysInfoPoll = (host: string, active: boolean) => {
  const [sysInfo, setSysInfo] = useState<SysInfo>({});
  useEffect(() => {
    if (!active) return;
    const fetchSys = () => {
      fetch(`http://${host}:3000/api/system/info`)
        .then(r => r.json())
        .then(d => setSysInfo({
          hostname: d.hostname,
          os: d.os,
          cpu_count: d.cpu_count,
          cpu_model: d.cpu_model,
          disk_free: typeof d.disk_root_free_bytes === 'number' ? d.disk_root_free_bytes : undefined,
          disk_total: typeof d.disk_root_total_bytes === 'number' ? d.disk_root_total_bytes : undefined,
        }))
        .catch(() => {});
    };
    fetchSys();
    const id = setInterval(fetchSys, 60000);
    return () => clearInterval(id);
  }, [host, active]);
  return sysInfo;
};
