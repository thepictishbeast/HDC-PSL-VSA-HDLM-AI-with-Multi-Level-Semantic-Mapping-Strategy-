import React from 'react';

// Small colored card used in the sidebar telemetry grid and the mobile
// telemetry row. Kept here so the sidebar's `renderTelemetryCard` helper
// doesn't need to exist inside the component — callers just pass the card
// shape and a `compact` flag.

export interface TelemetryCardData {
  label: string;
  value: string;
  unit: string;
  color: string;
  bg: string;
  border: string;
}

export interface TelemetryCardProps {
  C: any;
  card: TelemetryCardData;
  compact?: boolean;
}

export const TelemetryCard: React.FC<TelemetryCardProps> = ({ C, card, compact = false }) => (
  <div style={{
    padding: compact ? '10px 12px' : '12px 14px', borderRadius: '10px',
    background: card.bg, border: `1px solid ${card.border}`,
  }}>
    <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: compact ? '3px' : '5px' }}>{card.label}</div>
    <div style={{ fontSize: compact ? '18px' : '20px', fontWeight: 800, color: card.color }}>
      {card.value}<span style={{ fontSize: '11px', color: C.textDim, marginLeft: '2px' }}>{card.unit}</span>
    </div>
  </div>
);
