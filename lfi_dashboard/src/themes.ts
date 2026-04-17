// Theme palettes. Each export shares the shape of DARK; the THEMES record
// maps the string key used in localStorage / Settings to its palette.

export const DARK = {
  // c0-018 THEME OVERHAUL 2026-04-17: Linear/Vercel/Raycast aesthetic.
  // Layered depth via subtle bg variation (surface → elevated), warm-white
  // text, indigo #6366f1 primary accent, emerald / amber / rose accents.
  bg: '#0a0a0f',              // --bg-deep
  bgCard: '#12121a',          // --bg-surface
  bgInput: '#1a1a2e',         // --bg-elevated
  bgHover: '#22223a',         // --bg-hover
  border: '#2a2a40',          // --border-default
  borderFocus: 'rgba(99,102,241,0.60)',    // indigo 60%
  borderSubtle: '#1e1e30',    // --border-subtle
  text: '#e8e8f0',            // warm white
  textSecondary: '#8888a0',   // --text-secondary
  textMuted: '#555570',       // --text-tertiary
  textDim: '#3a3a55',         // --border-strong as dim text reference
  accent: '#6366f1',          // indigo — primary actions
  accentGlow: 'rgba(99,102,241,0.35)',
  accentBg: 'rgba(99,102,241,0.12)',
  accentBorder: 'rgba(99,102,241,0.30)',
  green: '#34d399',           // emerald — positive
  greenBg: 'rgba(52,211,153,0.12)',
  greenBorder: 'rgba(52,211,153,0.30)',
  red: '#f87171',             // rose red — errors
  redBg: 'rgba(248,113,113,0.12)',
  redBorder: 'rgba(248,113,113,0.30)',
  purple: '#a78bfa',          // softer violet — secondary accent
  purpleBg: 'rgba(167,139,250,0.12)',
  purpleBorder: 'rgba(167,139,250,0.30)',
  yellow: '#fbbf24',          // amber — caution
  yellowBg: 'rgba(251,191,36,0.12)',
  // Per Architectural Bible §6.3: DM Sans (distinctive, not generic).
  // Mono reserved for code blocks + reasoning/PLAN.
  font: "'DM Sans', 'Outfit', 'Satoshi', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
};

export const LIGHT: typeof DARK = {
  // c0-018 THEME OVERHAUL: warm-gray page, pure-white surfaces, deeper
  // indigo accent for AA contrast on white, deeper success/warn/danger.
  bg: '#f8f9fc',              // --bg-deep, warm gray
  bgCard: '#ffffff',          // --bg-surface
  bgInput: '#f0f1f5',         // --bg-hover (used as elevated input bg)
  bgHover: '#e8e9f0',         // --bg-active
  border: '#d1d5e0',          // --border-default
  borderFocus: 'rgba(79,70,229,0.55)',
  borderSubtle: '#e8e9f0',    // --border-subtle
  text: '#1a1a2e',            // near-black with blue tint
  textSecondary: '#6b6b80',
  textMuted: '#9b9bb0',
  textDim: '#b0b5c5',
  accent: '#4f46e5',          // deeper indigo for AA on white
  accentGlow: 'rgba(79,70,229,0.25)',
  accentBg: 'rgba(79,70,229,0.08)',
  accentBorder: 'rgba(79,70,229,0.28)',
  green: '#059669',           // deeper emerald
  greenBg: 'rgba(5,150,105,0.10)',
  greenBorder: 'rgba(5,150,105,0.30)',
  red: '#dc2626',             // deeper red
  redBg: 'rgba(220,38,38,0.08)',
  redBorder: 'rgba(220,38,38,0.28)',
  purple: '#7c3aed',          // deeper violet
  purpleBg: 'rgba(124,58,237,0.08)',
  purpleBorder: 'rgba(124,58,237,0.28)',
  yellow: '#d97706',          // deeper amber
  yellowBg: 'rgba(217,119,6,0.10)',
  font: DARK.font,
};

export const MIDNIGHT: typeof DARK = {
  bg: '#0a0f1f',
  bgCard: '#111831',
  bgInput: '#161d3a',
  bgHover: '#1d2544',
  border: 'rgba(200,220,255,0.12)',
  borderFocus: 'rgba(120,165,255,0.6)',
  borderSubtle: 'rgba(200,220,255,0.06)',
  text: '#e4ecff', textSecondary: '#a8b5d9', textMuted: '#7a86a8', textDim: '#4e5670',
  accent: '#78a5ff', accentGlow: 'rgba(120,165,255,0.42)',
  accentBg: 'rgba(120,165,255,0.12)', accentBorder: 'rgba(120,165,255,0.30)',
  green: '#3dd68c', greenBg: 'rgba(61,214,140,0.10)', greenBorder: 'rgba(61,214,140,0.24)',
  red: '#ff6b84', redBg: 'rgba(255,107,132,0.10)', redBorder: 'rgba(255,107,132,0.24)',
  purple: '#c79dff', purpleBg: 'rgba(199,157,255,0.10)', purpleBorder: 'rgba(199,157,255,0.24)',
  yellow: '#ffd36b', yellowBg: 'rgba(255,211,107,0.10)',
  font: DARK.font,
};

export const FOREST: typeof DARK = {
  bg: '#0a130d',
  bgCard: '#111d16',
  bgInput: '#17251c',
  bgHover: '#1e3025',
  border: 'rgba(200,255,220,0.10)',
  borderFocus: 'rgba(120,200,145,0.6)',
  borderSubtle: 'rgba(200,255,220,0.05)',
  text: '#e8f3ec', textSecondary: '#b0c8b6', textMuted: '#7f9a86', textDim: '#536657',
  accent: '#7cd49c', accentGlow: 'rgba(124,212,156,0.40)',
  accentBg: 'rgba(124,212,156,0.12)', accentBorder: 'rgba(124,212,156,0.30)',
  green: '#22c55e', greenBg: 'rgba(34,197,94,0.12)', greenBorder: 'rgba(34,197,94,0.28)',
  red: '#ff7388', redBg: 'rgba(255,115,136,0.10)', redBorder: 'rgba(255,115,136,0.24)',
  purple: '#d58bff', purpleBg: 'rgba(213,139,255,0.10)', purpleBorder: 'rgba(213,139,255,0.24)',
  yellow: '#ffd96b', yellowBg: 'rgba(255,217,107,0.10)',
  font: DARK.font,
};

export const SUNSET: typeof DARK = {
  bg: '#1a0f14',
  bgCard: '#241620',
  bgInput: '#2b1a26',
  bgHover: '#35212e',
  border: 'rgba(255,220,210,0.12)',
  borderFocus: 'rgba(255,150,120,0.6)',
  borderSubtle: 'rgba(255,220,210,0.06)',
  text: '#fdeee5', textSecondary: '#d6b8ad', textMuted: '#a08a82', textDim: '#695650',
  accent: '#ff9678', accentGlow: 'rgba(255,150,120,0.42)',
  accentBg: 'rgba(255,150,120,0.12)', accentBorder: 'rgba(255,150,120,0.30)',
  green: '#5fd6a0', greenBg: 'rgba(95,214,160,0.10)', greenBorder: 'rgba(95,214,160,0.24)',
  red: '#ff677e', redBg: 'rgba(255,103,126,0.10)', redBorder: 'rgba(255,103,126,0.24)',
  purple: '#e18bff', purpleBg: 'rgba(225,139,255,0.10)', purpleBorder: 'rgba(225,139,255,0.24)',
  yellow: '#ffcf5e', yellowBg: 'rgba(255,207,94,0.10)',
  font: DARK.font,
};

export const ROSE: typeof DARK = {
  bg: '#fff6f8',
  bgCard: '#ffffff',
  bgInput: '#fbeaee',
  bgHover: '#f5dae0',
  border: 'rgba(120,30,60,0.12)',
  borderFocus: 'rgba(205,70,100,0.55)',
  borderSubtle: 'rgba(120,30,60,0.06)',
  text: '#2a1420', textSecondary: '#5a3545', textMuted: '#7e5767', textDim: '#a9879a',
  accent: '#cd4664', accentGlow: 'rgba(205,70,100,0.32)',
  accentBg: 'rgba(205,70,100,0.08)', accentBorder: 'rgba(205,70,100,0.28)',
  green: '#0f8f5a', greenBg: 'rgba(15,143,90,0.10)', greenBorder: 'rgba(15,143,90,0.30)',
  red: '#b82040', redBg: 'rgba(184,32,64,0.08)', redBorder: 'rgba(184,32,64,0.28)',
  purple: '#7a3abf', purpleBg: 'rgba(122,58,191,0.08)', purpleBorder: 'rgba(122,58,191,0.28)',
  yellow: '#966612', yellowBg: 'rgba(150,102,18,0.10)',
  font: DARK.font,
};

export const CONTRAST: typeof DARK = {
  bg: '#000000',
  bgCard: '#000000',
  bgInput: '#000000',
  bgHover: '#1a1a1a',
  border: '#ffffff',
  borderFocus: '#ffff00',
  borderSubtle: 'rgba(255,255,255,0.35)',
  text: '#ffffff', textSecondary: '#ffffff', textMuted: '#e0e0e0', textDim: '#a0a0a0',
  accent: '#ffff00', accentGlow: 'rgba(255,255,0,0.5)',
  accentBg: 'rgba(255,255,0,0.14)', accentBorder: '#ffff00',
  green: '#00ff66', greenBg: 'rgba(0,255,102,0.15)', greenBorder: '#00ff66',
  red: '#ff5577', redBg: 'rgba(255,85,119,0.15)', redBorder: '#ff5577',
  purple: '#ff77ff', purpleBg: 'rgba(255,119,255,0.15)', purpleBorder: '#ff77ff',
  yellow: '#ffff00', yellowBg: 'rgba(255,255,0,0.15)',
  font: DARK.font,
};

export const THEMES: Record<string, typeof DARK> = {
  dark: DARK, light: LIGHT,
  midnight: MIDNIGHT, forest: FOREST, sunset: SUNSET, rose: ROSE, contrast: CONTRAST,
};
