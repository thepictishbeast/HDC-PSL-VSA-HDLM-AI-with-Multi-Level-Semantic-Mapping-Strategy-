// Theme palettes. Each export shares the shape of DARK; the THEMES record
// maps the string key used in localStorage / Settings to its palette.
//
// c0-024 2026-04-17: DARK + LIGHT now pull from ./design-system.ts (the
// cross-platform single source of truth shared with Tauri desktop + Android
// WebView). Field names stay the same so existing components don't need
// refactoring.
import { darkTheme as ds_dark, lightTheme as ds_light, typography as ds_type } from './design-system';

// Derive a legacy-shaped palette from the cross-platform theme tokens. The
// helper preserves the existing field names the rest of the app expects.
const fromTokens = (t: typeof ds_dark) => ({
  bg: t.bgDeep,
  bgCard: t.bgSurface,
  bgInput: t.bgInput,
  bgHover: t.bgHover,
  border: t.border,
  borderFocus: t.accent,
  borderSubtle: t.border,
  text: t.textPrimary,
  textSecondary: t.textSecondary,
  textMuted: t.textTertiary,
  textDim: t.textTertiary,
  accent: t.accent,
  accentGlow: 'transparent',   // c0-019: no glow
  accentBg: t.accentMuted,
  accentBorder: t.borderHover,
  green: t.success,
  greenBg: hexToRgba(t.success, 0.10),
  greenBorder: hexToRgba(t.success, 0.28),
  red: t.error,
  redBg: hexToRgba(t.error, 0.10),
  redBorder: hexToRgba(t.error, 0.28),
  // Purple kept as a secondary-status color (provenance badges etc.). Not
  // part of the core design-system palette, but we need it for semantic
  // distinctions the system uses in the chat. Stay in a neutral indigo.
  purple: '#8b5cf6',
  purpleBg: hexToRgba('#8b5cf6', 0.10),
  purpleBorder: hexToRgba('#8b5cf6', 0.25),
  yellow: t.warning,
  yellowBg: hexToRgba(t.warning, 0.10),
  font: ds_type.fontFamily,
});

function hexToRgba(hex: string, alpha: number): string {
  // Accepts #rrggbb. Falls back to the input unchanged if it doesn't match
  // (design-system.ts only defines hex values, so this is safe in practice).
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) return hex;
  return `rgba(${parseInt(m[1], 16)},${parseInt(m[2], 16)},${parseInt(m[3], 16)},${alpha})`;
}

export const DARK = fromTokens(ds_dark);

export const LIGHT: typeof DARK = fromTokens(ds_light);

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
  font: ds_type.fontFamily,
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
  font: ds_type.fontFamily,
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
  font: ds_type.fontFamily,
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
  font: ds_type.fontFamily,
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
  font: ds_type.fontFamily,
};

export const THEMES: Record<string, typeof DARK> = {
  dark: DARK, light: LIGHT,
  midnight: MIDNIGHT, forest: FOREST, sunset: SUNSET, rose: ROSE, contrast: CONTRAST,
};
