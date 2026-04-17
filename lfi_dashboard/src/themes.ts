// Theme palettes. Each export shares the shape of DARK; the THEMES record
// maps the string key used in localStorage / Settings to its palette.

export const DARK = {
  // Ported from reference: Ideas-for-improvement.jsx "onyx" tokens
  bg: '#08080D',
  bgCard: '#0C0C14',
  bgInput: '#13131E',
  bgHover: '#1A1A28',
  border: 'rgba(255,255,255,0.10)',
  borderFocus: 'rgba(139,123,247,0.60)',
  borderSubtle: 'rgba(255,255,255,0.06)',
  text: '#E8E6F0',
  textSecondary: '#9A96AD',
  textMuted: '#6B6780',
  textDim: '#4A4660',
  accent: '#8b7bf7',           // desaturated indigo-violet — enterprise calm
  accentGlow: 'rgba(139,123,247,0.35)',
  accentBg: 'rgba(139,123,247,0.10)',
  accentBorder: 'rgba(139,123,247,0.28)',
  green: '#22c55e',            // vivid emerald — reads as success, clearly green
  greenBg: 'rgba(34,197,94,0.12)',
  greenBorder: 'rgba(34,197,94,0.28)',
  red: '#ff6f81',
  redBg: 'rgba(255,111,129,0.10)',
  redBorder: 'rgba(255,111,129,0.24)',
  purple: '#e879f9',           // magenta-leaning — distinct from the indigo accent
  purpleBg: 'rgba(232,121,249,0.10)',
  purpleBorder: 'rgba(232,121,249,0.28)',
  yellow: '#ffd36b',
  yellowBg: 'rgba(255,211,107,0.10)',
  // Per Architectural Bible §6.3: DM Sans (distinctive, not generic).
  // Mono reserved for code blocks + reasoning/PLAN.
  font: "'DM Sans', 'Outfit', 'Satoshi', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
};

export const LIGHT: typeof DARK = {
  bg: '#FAFAFA',
  bgCard: '#FFFFFF',
  bgInput: '#F5F3F7',
  bgHover: '#F0ECF4',
  border: 'rgba(0,0,0,0.10)',
  borderFocus: 'rgba(124,107,240,0.55)',
  borderSubtle: 'rgba(0,0,0,0.06)',
  text: '#1A1525',
  textSecondary: '#6B6280',
  textMuted: '#9A93AD',
  textDim: '#B5B0C5',
  accent: '#7C6BF0',
  accentGlow: 'rgba(124,107,240,0.28)',
  accentBg: 'rgba(124,107,240,0.08)',
  accentBorder: 'rgba(92,74,220,0.28)',
  green: '#108a4e',
  greenBg: 'rgba(16,138,78,0.10)',
  greenBorder: 'rgba(16,138,78,0.30)',
  red: '#b83040',
  redBg: 'rgba(184,48,64,0.08)',
  redBorder: 'rgba(184,48,64,0.28)',
  purple: '#b520c8',
  purpleBg: 'rgba(181,32,200,0.08)',
  purpleBorder: 'rgba(181,32,200,0.30)',
  yellow: '#966612',
  yellowBg: 'rgba(150,102,18,0.10)',
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
