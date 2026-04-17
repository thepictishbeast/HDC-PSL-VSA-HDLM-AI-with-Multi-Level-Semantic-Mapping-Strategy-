import React from 'react';
import { T } from '../tokens';

// c0-auto-2 task 24 (CLAUDE2_500_TASKS.md): shared uppercase label component.
//
// Prior to this there were 30+ inline copies of the same style object across
// AdminModal, ClassroomView, TrainingDashboard and smaller panels:
//
//   <div style={{
//     fontSize: T.typography.sizeXs, color: C.textMuted,
//     fontWeight: T.typography.weightBold,
//     textTransform: 'uppercase',
//     letterSpacing: T.typography.trackingLoose,
//   }}>LABEL</div>
//
// This component captures the canonical form so upstream pages just write
// <Label color={C.textMuted}>LABEL</Label> and the typography tokens stay in
// one place. Keep the surface deliberately small — if a caller needs a
// fundamentally different look, they should build their own heading instead
// of extending this.
//
// AVP-PASS-34 (design-system consistency): every uppercase meta-label in the
// app now flows through one definition; a future token change propagates
// automatically to all call sites.

export interface LabelProps {
  children: React.ReactNode;
  /** Text color. Typically C.textMuted / C.textSecondary from the active theme. */
  color?: string;
  /** Optional bottom margin (pass a token, e.g. T.spacing.md). */
  mb?: string;
  /** Escape hatch for the rare call site that needs a one-off tweak.
   *  Merged last so it wins over the defaults — use sparingly. */
  style?: React.CSSProperties;
  /** Optional stable id for aria-labelledby hooks. */
  id?: string;
}

export const Label: React.FC<LabelProps> = ({ children, color, mb, style, id }) => (
  <div id={id} style={{
    fontSize: T.typography.sizeXs,
    fontWeight: T.typography.weightBold,
    textTransform: 'uppercase',
    letterSpacing: T.typography.trackingLoose,
    ...(color !== undefined ? { color } : null),
    ...(mb !== undefined ? { marginBottom: mb } : null),
    ...style,
  }}>{children}</div>
);
