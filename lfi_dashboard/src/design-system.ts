/**
 * PlausiDen Design System — Single source of truth for all frontends.
 *
 * Used by: Web dashboard, Tauri desktop app, Android WebView
 *
 * Every color, spacing, typography, and component style is defined here.
 * NO hardcoded values anywhere else in the codebase.
 *
 * Design reference: Stripe, Linear, Notion — professional tools for technical users.
 */

// ============================================================
// COLOR PALETTE
// ============================================================

export const palette = {
  // Neutral slate — professional base
  slate: {
    950: '#0f1117',
    900: '#161922',
    850: '#1c1f2b',
    800: '#252836',
    750: '#2d3142',
    700: '#363a50',
    600: '#4a4e68',
    500: '#6b7084',
    400: '#8b8fa3',
    300: '#a8abbe',
    200: '#c8cad6',
    100: '#e5e7ed',
    50:  '#f5f6f8',
  },
  // Primary blue — trust, intelligence
  blue: {
    900: '#1e3a5f',
    800: '#1e40af',
    700: '#1d4ed8',
    600: '#2563eb',
    500: '#3b82f6',
    400: '#60a5fa',
    300: '#93bbfd',
    200: '#bfdbfe',
    100: '#dbeafe',
    50:  '#eff6ff',
  },
  // Semantic
  green:  { 600: '#16a34a', 500: '#22c55e', 400: '#4ade80', 100: '#dcfce7' },
  amber:  { 600: '#ca8a04', 500: '#eab308', 400: '#facc15', 100: '#fef9c3' },
  red:    { 600: '#dc2626', 500: '#ef4444', 400: '#f87171', 100: '#fee2e2' },
} as const;

// ============================================================
// THEME DEFINITIONS
// ============================================================

export interface ThemeTokens {
  name: string;
  // Backgrounds
  bgDeep: string;
  bgSurface: string;
  bgElevated: string;
  bgHover: string;
  bgActive: string;
  bgInput: string;
  // Text
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  // Accent
  accent: string;
  accentHover: string;
  accentMuted: string;
  // Semantic
  success: string;
  warning: string;
  error: string;
  info: string;
  // Borders
  border: string;
  borderHover: string;
  // Shadows
  shadowSm: string;
  shadowMd: string;
  shadowLg: string;
  // Message bubbles
  userBubbleBg: string;
  userBubbleText: string;
  aiBubbleBg: string;
  aiBubbleText: string;
  // c2-342 / c0-auto-2 tasks 53+54: keyboard focus ring and modal overlay
  // backdrop pulled into the cross-platform design-system so desktop and
  // Android builds get the same accessibility affordance and dim strength.
  focusRing: string;
  overlayBg: string;
}

export const darkTheme: ThemeTokens = {
  name: 'dark',
  bgDeep:      palette.slate[950],
  bgSurface:   palette.slate[900],
  bgElevated:  palette.slate[850],
  bgHover:     palette.slate[800],
  bgActive:    palette.slate[750],
  bgInput:     '#1a1d28',
  textPrimary:   '#ecedF0',
  textSecondary: palette.slate[400],
  textTertiary:  palette.slate[600],
  accent:       palette.blue[500],
  accentHover:  palette.blue[400],
  accentMuted:  palette.blue[900],
  success:  palette.green[500],
  warning:  palette.amber[500],
  error:    palette.red[500],
  info:     palette.blue[500],
  border:      '#1f2233',
  borderHover: '#2a2d40',
  shadowSm: '0 1px 2px rgba(0,0,20,0.3)',
  shadowMd: '0 4px 12px rgba(0,0,20,0.4)',
  shadowLg: '0 8px 24px rgba(0,0,20,0.5)',
  userBubbleBg:   palette.blue[900],
  userBubbleText: palette.blue[200],
  aiBubbleBg:     palette.slate[850],
  aiBubbleText:   '#ecedF0',
  focusRing: `0 0 0 2px ${palette.blue[500]}`,
  overlayBg: 'rgba(0,0,0,0.65)',
};

export const lightTheme: ThemeTokens = {
  name: 'light',
  bgDeep:      palette.slate[50],
  bgSurface:   '#ffffff',
  bgElevated:  '#ffffff',
  bgHover:     '#f0f1f4',
  bgActive:    '#e5e7ed',
  bgInput:     palette.slate[50],
  textPrimary:   '#111827',
  textSecondary: '#6b7280',
  textTertiary:  '#9ca3af',
  accent:       palette.blue[600],
  accentHover:  palette.blue[500],
  accentMuted:  palette.blue[50],
  success:  palette.green[600],
  warning:  palette.amber[600],
  error:    palette.red[600],
  info:     palette.blue[600],
  border:      '#e5e7eb',
  borderHover: '#d1d5db',
  shadowSm: '0 1px 2px rgba(0,0,30,0.06)',
  shadowMd: '0 4px 12px rgba(0,0,30,0.08)',
  shadowLg: '0 8px 24px rgba(0,0,30,0.12)',
  userBubbleBg:   palette.blue[50],
  userBubbleText: palette.blue[800],
  aiBubbleBg:     '#ffffff',
  aiBubbleText:   '#111827',
  focusRing: `0 0 0 2px ${palette.blue[600]}`,
  overlayBg: 'rgba(0,0,0,0.4)',
};

// ============================================================
// TYPOGRAPHY
// ============================================================

export const typography = {
  fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  fontMono: '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace',
  sizes: {
    xs: '12px',
    sm: '13px',
    base: '14px',
    md: '16px',
    lg: '18px',
    xl: '20px',
    '2xl': '24px',
    '3xl': '32px',
  },
  weights: {
    normal: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },
  lineHeight: {
    tight: 1.3,
    normal: 1.5,
    relaxed: 1.7,
  },
} as const;

// ============================================================
// SPACING (8px grid)
// ============================================================

export const spacing = {
  0: '0',
  1: '4px',
  2: '8px',
  3: '12px',
  4: '16px',
  5: '20px',
  6: '24px',
  8: '32px',
  10: '40px',
  12: '48px',
  16: '64px',
} as const;

// ============================================================
// BORDER RADIUS
// ============================================================

export const radius = {
  sm: '4px',
  md: '6px',
  lg: '8px',
  xl: '12px',
  '2xl': '16px',
  full: '9999px',
} as const;

// ============================================================
// BREAKPOINTS
// ============================================================

export const breakpoints = {
  mobile: 375,
  tablet: 768,
  desktop: 1024,
  wide: 1280,
} as const;

// ============================================================
// Z-INDEX SCALE
// ============================================================

export const zIndex = {
  base: 0,
  dropdown: 100,
  sticky: 200,
  overlay: 300,
  modal: 400,
  toast: 500,
  tooltip: 600,
} as const;

// ============================================================
// COMPONENT PRESETS
// ============================================================

export const components = {
  card: {
    borderRadius: radius.xl,
    padding: spacing[6],
    border: '1px solid',
  },
  button: {
    borderRadius: radius.md,
    paddingX: spacing[4],
    paddingY: spacing[2],
    fontSize: typography.sizes.base,
    fontWeight: typography.weights.medium,
    minHeight: '36px',
    minTouchTarget: '44px', // Mobile accessibility
  },
  input: {
    borderRadius: radius.md,
    paddingX: spacing[3],
    paddingY: spacing[2],
    fontSize: typography.sizes.base,
    minHeight: '36px',
  },
  chatBubble: {
    borderRadius: radius['2xl'],
    padding: `${spacing[3]} ${spacing[4]}`,
    maxWidth: '80%',
  },
  sidebar: {
    width: '280px',
    collapsedWidth: '60px',
  },
  modal: {
    borderRadius: radius.xl,
    maxWidth: '900px',
    maxHeight: '85vh',
  },
} as const;

// ============================================================
// ANIMATION
// ============================================================

export const animation = {
  fast: '0.1s ease',
  normal: '0.2s ease',
  slow: '0.3s ease',
  // Theme transition — slightly slower for background changes
  themeTransition: 'background-color 0.2s ease, color 0.15s ease, border-color 0.2s ease',
} as const;

// ============================================================
// HELPERS
// ============================================================

/** Get current theme based on preference */
export function getTheme(name: string): ThemeTokens {
  switch (name) {
    case 'light': return lightTheme;
    case 'dark':
    default: return darkTheme;
  }
}

/** Apply theme tokens as CSS custom properties */
export function applyTheme(theme: ThemeTokens): void {
  const root = document.documentElement;
  Object.entries(theme).forEach(([key, value]) => {
    if (typeof value === 'string') {
      root.style.setProperty(`--${camelToKebab(key)}`, value);
    }
  });
}

function camelToKebab(str: string): string {
  return str.replace(/([A-Z])/g, '-$1').toLowerCase();
}
