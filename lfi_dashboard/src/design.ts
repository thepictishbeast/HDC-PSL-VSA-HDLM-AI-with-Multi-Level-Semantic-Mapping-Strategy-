// ============================================================
// LFI Design System — Premium Claude Code-inspired Dark Mode
//
// Design tokens, color system, typography, spacing, motion.
// Import these constants into components instead of hard-coding values.
// ============================================================

// ---- COLOR SYSTEM ----
// Claude Code uses sophisticated near-blacks with warm undertones
// and accent colors that pop against deep backgrounds.
export const colors = {
  // Backgrounds (dark-mode-first)
  bg: {
    base: '#0a0a0b',        // Deep near-black with slight warmth
    elevated: '#111113',     // Panels, cards
    surface: '#1a1a1d',      // Input fields, buttons
    overlay: '#22222680',    // Modal backdrops (with alpha)
    hover: '#2a2a2e',        // Hover states
    active: '#333338',       // Active/pressed states
  },

  // Text
  text: {
    primary: '#ededef',      // Headlines, body
    secondary: '#a1a1a6',    // Muted text, metadata
    tertiary: '#6b6b75',     // Very faint, placeholders
    inverse: '#0a0a0b',      // On light backgrounds
  },

  // Borders
  border: {
    subtle: '#1f1f22',       // Barely visible
    default: '#2d2d32',      // Card borders
    strong: '#3f3f46',       // Emphasis
    focus: '#8b5cf6',        // Keyboard focus (purple accent)
  },

  // Brand / Accent colors
  accent: {
    purple: '#8b5cf6',       // Primary brand (PlausiDen purple)
    purpleLight: '#a78bfa',  // Hover/active
    purpleDim: '#6d28d9',    // Pressed

    teal: '#06b6d4',         // Info, links
    tealLight: '#22d3ee',    // Hover

    green: '#10b981',        // Success, online, correct
    greenLight: '#34d399',

    amber: '#f59e0b',        // Warning
    amberLight: '#fbbf24',

    red: '#ef4444',          // Error, destructive
    redLight: '#f87171',
  },

  // Semantic colors (map to accent)
  semantic: {
    success: '#10b981',
    warning: '#f59e0b',
    danger: '#ef4444',
    info: '#06b6d4',
  },

  // Gradients for heroes and special surfaces
  gradients: {
    hero: 'linear-gradient(135deg, #8b5cf6 0%, #06b6d4 100%)',
    heroDim: 'linear-gradient(135deg, #8b5cf620 0%, #06b6d420 100%)',
    brand: 'linear-gradient(135deg, #a78bfa 0%, #8b5cf6 50%, #6d28d9 100%)',
    surface: 'linear-gradient(180deg, #111113 0%, #0a0a0b 100%)',
    card: 'linear-gradient(180deg, #1a1a1d 0%, #111113 100%)',
    danger: 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
    success: 'linear-gradient(135deg, #10b981 0%, #059669 100%)',
  },
};

// ---- TYPOGRAPHY ----
// Premium typography uses a distinctive display font with excellent
// readability at body sizes. SF Mono / JetBrains Mono for code.
export const typography = {
  fonts: {
    sans: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif',
    display: '"Inter Display", "Inter", -apple-system, sans-serif',
    mono: '"JetBrains Mono", "SF Mono", Consolas, "Liberation Mono", Menlo, monospace',
  },

  // Size scale (modular scale × 1.125 for tight spacing)
  sizes: {
    xs: '0.75rem',     // 12px
    sm: '0.875rem',    // 14px
    base: '1rem',      // 16px
    lg: '1.125rem',    // 18px
    xl: '1.25rem',     // 20px
    '2xl': '1.5rem',   // 24px
    '3xl': '1.875rem', // 30px
    '4xl': '2.25rem',  // 36px
    '5xl': '3rem',     // 48px
    '6xl': '3.75rem',  // 60px
  },

  weights: {
    regular: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },

  // Line heights (tight for display, relaxed for body)
  lineHeights: {
    tight: 1.1,
    snug: 1.25,
    normal: 1.5,
    relaxed: 1.65,
    loose: 1.85,
  },

  // Letter spacing (tracking)
  tracking: {
    tight: '-0.02em',      // Display text
    normal: '0',
    wide: '0.02em',        // All-caps labels
    wider: '0.08em',       // Tiny labels
  },
};

// ---- SPACING ----
// 4px base unit × 8px grid. Use multiples.
export const spacing = {
  '0': '0',
  '1': '0.25rem',    // 4px
  '2': '0.5rem',     // 8px
  '3': '0.75rem',    // 12px
  '4': '1rem',       // 16px
  '5': '1.25rem',    // 20px
  '6': '1.5rem',     // 24px
  '8': '2rem',       // 32px
  '10': '2.5rem',    // 40px
  '12': '3rem',      // 48px
  '16': '4rem',      // 64px
  '20': '5rem',      // 80px
  '24': '6rem',      // 96px
  '32': '8rem',      // 128px
};

// ---- RADIUS ----
// Claude Code uses softer radii for a modern premium feel.
export const radius = {
  sm: '0.375rem',    // 6px — inputs, tags
  md: '0.5rem',      // 8px — buttons, small cards
  lg: '0.75rem',     // 12px — cards, panels
  xl: '1rem',        // 16px — hero surfaces
  '2xl': '1.5rem',   // 24px — modals
  full: '9999px',    // Pills, avatars
};

// ---- SHADOWS ----
// Soft, elevated shadows for depth. Dark mode uses subtle light shadows
// for elevation, colored shadows for interactive/brand elements.
export const shadows = {
  // Neutral elevation
  xs: '0 1px 2px rgba(0, 0, 0, 0.4)',
  sm: '0 2px 4px rgba(0, 0, 0, 0.5)',
  md: '0 4px 12px rgba(0, 0, 0, 0.6)',
  lg: '0 8px 24px rgba(0, 0, 0, 0.7)',
  xl: '0 16px 48px rgba(0, 0, 0, 0.8)',

  // Inner shadows (for pressed states, insets)
  inner: 'inset 0 1px 2px rgba(0, 0, 0, 0.5)',

  // Colored shadows (brand glow)
  brandGlow: '0 0 32px rgba(139, 92, 246, 0.35)',
  tealGlow: '0 0 32px rgba(6, 182, 212, 0.35)',
  successGlow: '0 0 24px rgba(16, 185, 129, 0.3)',
  dangerGlow: '0 0 24px rgba(239, 68, 68, 0.3)',

  // Focus ring
  focus: '0 0 0 3px rgba(139, 92, 246, 0.35)',
};

// ---- MOTION ----
// Use spring-like easings for a premium feel. No linear timings.
export const motion = {
  durations: {
    instant: '50ms',
    fast: '150ms',
    normal: '250ms',
    slow: '400ms',
    slower: '600ms',
  },

  easings: {
    // Standard ease-out for exits
    out: 'cubic-bezier(0.16, 1, 0.3, 1)',
    // Smooth ease-in-out for transitions
    inOut: 'cubic-bezier(0.65, 0, 0.35, 1)',
    // Bounce for entrances
    spring: 'cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    // Sharp deceleration
    sharp: 'cubic-bezier(0.4, 0, 0.6, 1)',
  },

  // Pre-composed transitions
  transitions: {
    all: 'all 250ms cubic-bezier(0.16, 1, 0.3, 1)',
    colors: 'color 150ms, background-color 150ms, border-color 150ms',
    transform: 'transform 250ms cubic-bezier(0.16, 1, 0.3, 1)',
    opacity: 'opacity 200ms ease-out',
  },
};

// ---- Z-INDEX LAYERS ----
export const z = {
  base: 0,
  raised: 10,
  dropdown: 100,
  sticky: 200,
  overlay: 500,
  modal: 1000,
  popover: 1500,
  tooltip: 2000,
  notification: 5000,
};

// ---- COMPONENT PRESETS ----
// Common styles you can spread into inline style objects.
export const presets = {
  // Card: elevated surface for content blocks
  card: {
    backgroundColor: colors.bg.elevated,
    borderRadius: radius.lg,
    border: `1px solid ${colors.border.subtle}`,
    boxShadow: shadows.md,
    padding: spacing['6'],
  },

  // Hero card: gradient-accented card for primary surfaces
  heroCard: {
    background: colors.gradients.card,
    borderRadius: radius.xl,
    border: `1px solid ${colors.border.default}`,
    boxShadow: shadows.lg,
    padding: spacing['8'],
  },

  // Input: form fields
  input: {
    backgroundColor: colors.bg.surface,
    border: `1px solid ${colors.border.default}`,
    borderRadius: radius.md,
    color: colors.text.primary,
    fontSize: typography.sizes.base,
    fontFamily: typography.fonts.sans,
    padding: `${spacing['2']} ${spacing['4']}`,
    outline: 'none',
    transition: motion.transitions.colors,
  },

  // Button primary
  buttonPrimary: {
    background: colors.gradients.brand,
    color: '#fff',
    border: 'none',
    borderRadius: radius.md,
    fontSize: typography.sizes.sm,
    fontWeight: typography.weights.semibold,
    padding: `${spacing['2']} ${spacing['5']}`,
    cursor: 'pointer',
    transition: motion.transitions.all,
    boxShadow: shadows.brandGlow,
  },

  // Button ghost (secondary)
  buttonGhost: {
    backgroundColor: 'transparent',
    color: colors.text.primary,
    border: `1px solid ${colors.border.default}`,
    borderRadius: radius.md,
    fontSize: typography.sizes.sm,
    fontWeight: typography.weights.medium,
    padding: `${spacing['2']} ${spacing['4']}`,
    cursor: 'pointer',
    transition: motion.transitions.all,
  },

  // Tag / Pill
  tag: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: spacing['1'],
    backgroundColor: colors.bg.surface,
    color: colors.text.secondary,
    fontSize: typography.sizes.xs,
    fontWeight: typography.weights.medium,
    padding: `${spacing['1']} ${spacing['2']}`,
    borderRadius: radius.full,
    border: `1px solid ${colors.border.subtle}`,
  },

  // Label (small uppercase)
  label: {
    fontSize: typography.sizes.xs,
    fontWeight: typography.weights.semibold,
    letterSpacing: typography.tracking.wider,
    textTransform: 'uppercase' as const,
    color: colors.text.tertiary,
  },

  // Monospace code
  code: {
    fontFamily: typography.fonts.mono,
    fontSize: typography.sizes.sm,
    backgroundColor: colors.bg.surface,
    padding: `${spacing['1']} ${spacing['2']}`,
    borderRadius: radius.sm,
    color: colors.accent.tealLight,
  },

  // Scrollbar styling (apply to container)
  scrollableContainer: {
    scrollbarWidth: 'thin' as const,
    scrollbarColor: `${colors.border.strong} transparent`,
  },
};

// ---- ACCESSIBILITY ----
export const a11y = {
  // Focus ring for keyboard navigation
  focusRing: {
    outline: 'none',
    boxShadow: shadows.focus,
  },

  // Screen-reader-only text
  srOnly: {
    position: 'absolute' as const,
    width: '1px',
    height: '1px',
    padding: '0',
    margin: '-1px',
    overflow: 'hidden',
    clip: 'rect(0, 0, 0, 0)',
    whiteSpace: 'nowrap' as const,
    borderWidth: '0',
  },

  // Reduced motion preference check (use in useEffect)
  prefersReducedMotion: () => {
    if (typeof window === 'undefined') return false;
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  },
};

// ---- UTILITIES ----

/// Combine inline styles. More readable than nested spreads.
export function sx(...styles: (React.CSSProperties | false | null | undefined)[]): React.CSSProperties {
  return Object.assign({}, ...styles.filter(Boolean));
}

/// Format a number as a percentage with specified decimals.
export function pct(value: number, decimals = 1): string {
  return `${(value * 100).toFixed(decimals)}%`;
}

/// Format a large number with commas.
export function fmt(value: number): string {
  return value.toLocaleString('en-US');
}

/// Format milliseconds to human-readable duration.
export function ms(value: number): string {
  if (value < 1000) return `${value}ms`;
  if (value < 60_000) return `${(value / 1000).toFixed(1)}s`;
  return `${(value / 60_000).toFixed(1)}m`;
}

/// Map a domain name to a consistent color.
export function domainColor(domain: string): string {
  const palette = [
    colors.accent.purple, colors.accent.teal, colors.accent.green,
    colors.accent.amber, colors.accent.red, colors.accent.purpleLight,
    colors.accent.tealLight, colors.accent.greenLight,
  ];
  let hash = 0;
  for (let i = 0; i < domain.length; i++) {
    hash = domain.charCodeAt(i) + ((hash << 5) - hash);
  }
  return palette[Math.abs(hash) % palette.length];
}

export default {
  colors,
  typography,
  spacing,
  radius,
  shadows,
  motion,
  z,
  presets,
  a11y,
  sx,
  pct,
  fmt,
  ms,
  domainColor,
};
