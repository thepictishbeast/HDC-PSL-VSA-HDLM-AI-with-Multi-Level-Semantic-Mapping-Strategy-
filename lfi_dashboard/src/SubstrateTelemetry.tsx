import React from 'react';
import { TelemetryCard, type TelemetryCardData } from './TelemetryCards';
import { diskPressure } from './util';

// Top panel of the sidebar: "Substrate Telemetry" header + live/stale/offline
// pill + 2x2 card grid + optional Thermal Throttle and Disk Pressure alerts.

export interface SubstrateTelemetryProps {
  C: any;
  cards: TelemetryCardData[];
  lastOkMs: number | null;  // last successful /api/status timestamp
  thermalThrottled: boolean;
  diskFree?: number;
  diskTotal?: number;
}

export const SubstrateTelemetry: React.FC<SubstrateTelemetryProps> = ({
  C, cards, lastOkMs, thermalThrottled, diskFree, diskTotal,
}) => {
  // Freshness badge values re-computed on every render (parent re-renders
  // on every /api/status poll tick). Green <=20s, amber 20-90s, red >90s.
  const renderBadge = () => {
    if (!lastOkMs) {
      return <span style={{ fontSize: '9px', color: C.red, textTransform: 'none', letterSpacing: 0, fontWeight: 600 }}>offline</span>;
    }
    const ageSec = (Date.now() - lastOkMs) / 1000;
    const color = ageSec <= 20 ? C.green : ageSec <= 90 ? C.yellow : C.red;
    const label = ageSec <= 20 ? 'live' : ageSec <= 90 ? `${Math.round(ageSec)}s stale` : 'offline';
    return (
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: '5px', fontSize: '9px', color, textTransform: 'none', letterSpacing: 0, fontWeight: 600 }}>
        <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: color, boxShadow: ageSec <= 20 ? `0 0 5px ${color}` : 'none' }} />
        {label}
      </span>
    );
  };

  const dp = diskPressure(diskFree, diskTotal);
  const showDiskAlert = dp && dp.usedPct >= 90;
  const diskCritical = showDiskAlert && dp.usedPct >= 95;

  return (
    <div style={{ padding: '20px', borderBottom: `1px solid ${C.borderSubtle}` }}>
      <div style={{
        fontSize: '11px', fontWeight: 800, color: C.textMuted,
        textTransform: 'uppercase', letterSpacing: '0.12em', marginBottom: '14px',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
      }}>
        <span>Substrate Telemetry</span>
        {renderBadge()}
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
        {cards.map(card => (
          <TelemetryCard key={card.label} C={C} card={card} compact />
        ))}
      </div>
      {thermalThrottled && (
        <div style={{
          marginTop: '10px', padding: '10px', background: C.redBg,
          border: `1px solid ${C.redBorder}`, borderRadius: '8px',
          textAlign: 'center', fontSize: '11px', fontWeight: 800, color: C.red, textTransform: 'uppercase',
          letterSpacing: '0.08em',
        }}>Thermal Throttle</div>
      )}
      {showDiskAlert && dp && (
        <div style={{
          marginTop: '10px', padding: '10px',
          background: diskCritical ? C.redBg : C.yellowBg,
          border: `1px solid ${diskCritical ? C.redBorder : C.yellowBorder}`,
          borderRadius: '8px', fontSize: '11px', lineHeight: 1.4,
          color: diskCritical ? C.red : C.yellow,
        }}>
          <div style={{ fontWeight: 800, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '3px' }}>
            Disk Pressure · {Math.round(dp.usedPct)}%
          </div>
          <div style={{ fontSize: '10px', opacity: 0.85 }}>
            {dp.freeGb.toFixed(1)}G free on server root. Writes may start failing.
          </div>
        </div>
      )}
    </div>
  );
};
