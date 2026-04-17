import React from 'react';
import { T } from './tokens';
// c2-352 / task 48: shared Label + design-system for the 20px value size.
import { Label } from './components';
import { typography as dsType } from './design-system';

// Small colored card used in the sidebar telemetry grid and the mobile
// telemetry row. Kept here so the sidebar's `renderTelemetryCard` helper
// doesn't need to exist inside the component — callers just pass the card
// shape and a `compact` flag.
//
// c2-237 / #20: migrated hardcoded spacing/radii/typography to tokens.ts.

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
    padding: compact ? `${T.spacing.sm} ${T.spacing.md}` : `${T.spacing.md} ${T.spacing.lg}`,
    borderRadius: T.radii.xl,
    background: card.bg, border: `1px solid ${card.border}`,
  }}>
    <Label color={C.textMuted} mb={compact ? '3px' : '5px'}>{card.label}</Label>
    <div style={{
      fontSize: compact ? T.typography.size2xl : dsType.sizes.xl,
      fontWeight: T.typography.weightBlack, color: card.color,
    }}>
      {card.value}<span style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginLeft: '2px' }}>{card.unit}</span>
    </div>
  </div>
);
