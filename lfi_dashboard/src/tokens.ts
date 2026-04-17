// Design tokens — non-color primitives. Palette values live in `themes.ts`;
// this file owns everything else (spacing, radii, shadows, motion, type scale).
//
// Phase 1.2 of FRONTEND_SUPERSOCIETY_PLAN.md. Target is a 4/8 px grid so all
// layouts snap to consistent rhythm (AVP-2 §30 design-system consistency).
//
// c0-024/99: the cross-platform source of truth is `design-system.ts`. We
// re-export its typography so components can pull everything from `T.*`.

import { typography as dsTypography } from './design-system';

// ---- Spacing (4 px base; step 1 = 4 px, step 2 = 8 px, … ) ----
// Named in conventional t-shirt sizes for legibility.
export const spacing = {
  none: '0',
  xs: '4px',   // 1 step
  sm: '8px',   // 2 step
  md: '12px',  // 3 step
  lg: '16px',  // 4 step
  xl: '24px',  // 6 step
  xxl: '32px', // 8 step
  xxxl: '48px',// 12 step
} as const;

// ---- Radii ----
// c0-019 FINAL: buttons/inputs 4px, cards 6px, larger surfaces 8px. No
// rounded-full except on avatars. Stripe/Linear/Notion-style restraint.
export const radii = {
  xs: '3px',
  sm: '4px',    // buttons, inputs
  md: '6px',    // cards, panels
  lg: '8px',    // modal surfaces
  xl: '10px',
  xxl: '12px',
  round: '50%', // only safe on square elements (avatars, perfect circles)
  // c2-344: pill shape for any rectangle — 9999px is the standard trick
  // so callers don't have to compute width/2. Use this instead of 999px
  // inline literals (previous convention).
  pill: '9999px',
} as const;

// ---- Shadows ----
// c0-019 FINAL: subtle, cool-toned, enterprise restraint. No dramatic drops.
export const shadows = {
  none: 'none',
  cardLight: '0 1px 2px rgba(15,17,23,0.24)',
  card: '0 2px 6px rgba(15,17,23,0.32)',
  modal: '0 8px 24px rgba(15,17,23,0.42)',
  modalDeep: '0 16px 48px rgba(15,17,23,0.52)',
} as const;

// ---- Motion ----
// Standardized durations. Prefer `fast` (120 ms) for hovers,
// `base` (200 ms) for most transitions, `slow` (300 ms) for modal transitions.
export const motion = {
  fast: '120ms',
  base: '200ms',
  slow: '300ms',
  easeOut: 'cubic-bezier(0.16, 1, 0.3, 1)',
  easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)',
} as const;

// ---- Typography scale ----
// Ordered so readers can pick a step by visual hierarchy without doing math.
// `body` is the default; step up/down one level at a time for contrast.
// Font families come from design-system.ts (single source of truth shared
// with desktop + Android builds) via fontFamily + fontMono re-exports.
export const typography = {
  // Sizes
  sizeXs: '11px',    // metadata, badges
  sizeSm: '12px',    // secondary text
  sizeMd: '13px',    // body-adjacent
  sizeBody: '14px',  // body default
  sizeLg: '15.5px',  // chat input
  sizeXl: '16px',    // heading-5
  size2xl: '18px',   // heading-4
  size3xl: '22px',   // heading-3
  // Weights
  weightRegular: 400,
  weightMedium: 500,
  weightSemibold: 600,
  weightBold: 700,
  weightBlack: 800,
  // Line heights
  lineTight: 1.3,
  lineNormal: 1.55,
  lineLoose: 1.7,
  // Letter spacing — used on all-caps labels to reduce tightness
  trackingTight: '-0.01em',
  trackingNormal: '0',
  trackingLoose: '0.10em',
  trackingCap: '0.15em',
  trackingCapWide: '0.20em',
  // Font families re-exported from the cross-platform design-system.
  fontFamily: dsTypography.fontFamily,
  fontMono: dsTypography.fontMono,
} as const;

// ---- Z-index scale ----
// Named so the render order is easy to reason about at a glance.
export const z = {
  base: 0,
  sticky: 10,
  overlay: 100,
  modal: 200,
  palette: 220,
  toast: 300,
} as const;

// Convenience re-export for brevity at call sites:
//   import { T } from './tokens';
//   style={{ padding: T.spacing.lg, borderRadius: T.radii.xl }}
export const T = { spacing, radii, shadows, motion, typography, z } as const;
