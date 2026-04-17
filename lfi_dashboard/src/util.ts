// Shared formatting helpers. Keep this file dependency-free so any component
// can import from it without pulling in React, theme, or DOM specifics.

// 56_750_622 → "56.7M", 168 → "168", 3945 → "3.9K", null/NaN → "—".
export const compactNum = (n: number | null | undefined): string => {
  if (n == null || Number.isNaN(n)) return '—';
  const abs = Math.abs(n);
  if (abs >= 1e9) return (n / 1e9).toFixed(1).replace(/\.0$/, '') + 'B';
  if (abs >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, '') + 'M';
  if (abs >= 1e3) return (n / 1e3).toFixed(1).replace(/\.0$/, '') + 'K';
  return String(n);
};
