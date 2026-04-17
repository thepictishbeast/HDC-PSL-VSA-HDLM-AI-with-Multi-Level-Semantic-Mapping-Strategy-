import React from 'react';
import { T } from '../tokens';

// c0-auto-2 task 30 (CLAUDE2_500_TASKS.md): shared tab-bar component.
//
// Consolidates the AdminModal + ClassroomView tablists which shared
// the same shape: role='tablist', arrow-key + Home/End navigation,
// 2px accent-colored underline indicator, horizontal scroll, and the
// `marginBottom: -1px` trick so the active tab's border overlays the
// container's bottom border cleanly.
//
// AVP-PASS-27 (a11y):
//  - wrapper has role='tablist' and a required `label` prop for
//    aria-label; buttons have role='tab' and aria-selected state;
//    tabIndex is roving (0 on active, -1 on rest) so focus lands on
//    the current tab when the user tabs into the bar.
//  - ArrowRight / ArrowLeft / Home / End all implemented per WAI
//    ARIA APG tabs pattern; wraps around on Left/Right.

export interface TabDef<T extends string> {
  id: T;
  label: string;
  /** Optional browser tooltip hint (title attribute). */
  title?: string;
}

export interface TabBarProps<T extends string> {
  C: any;
  /** aria-label for the tablist wrapper; required for screen readers. */
  label: string;
  tabs: ReadonlyArray<TabDef<T>>;
  active: T;
  onChange: (id: T) => void;
  /** Horizontal padding on the tablist. Default 0 24px. */
  padding?: string;
  /** Tablist background (e.g. C.bgCard). Defaults to transparent so
   *  the page background shows through. */
  background?: string;
  /** Font weight for tab buttons. Default 600 (semibold); set 700
   *  (bold) to match the AdminModal convention. */
  weight?: number;
  /** Content appended after the tabs, pushed to the right via a
   *  flex:1 spacer. Used for refresh buttons, stale indicators,
   *  etc. */
  rightContent?: React.ReactNode;
}

export function TabBar<T extends string>({
  C, label, tabs, active, onChange,
  padding = '0 24px', background, weight = T.typography.weightSemibold,
  rightContent,
}: TabBarProps<T>): JSX.Element {
  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const idx = tabs.findIndex(t => t.id === active);
    if (idx < 0) return;
    const n = tabs.length;
    if (e.key === 'ArrowRight') { e.preventDefault(); onChange(tabs[(idx + 1) % n].id); }
    else if (e.key === 'ArrowLeft') { e.preventDefault(); onChange(tabs[(idx - 1 + n) % n].id); }
    else if (e.key === 'Home') { e.preventDefault(); onChange(tabs[0].id); }
    else if (e.key === 'End') { e.preventDefault(); onChange(tabs[n - 1].id); }
  };
  return (
    <div role='tablist' aria-label={label}
      onKeyDown={onKeyDown}
      style={{
        display: 'flex', gap: '2px', padding,
        borderBottom: `1px solid ${C.borderSubtle}`, overflowX: 'auto',
        ...(background ? { background } : null),
      }}>
      {tabs.map(t => (
        <button key={t.id} onClick={() => onChange(t.id)}
          role='tab' aria-selected={active === t.id}
          tabIndex={active === t.id ? 0 : -1}
          title={t.title}
          style={{
            padding: '14px 16px', fontSize: T.typography.sizeMd, fontWeight: weight,
            background: 'transparent', border: 'none', cursor: 'pointer',
            color: active === t.id ? C.accent : C.textMuted,
            borderBottom: `2px solid ${active === t.id ? C.accent : 'transparent'}`,
            marginBottom: '-1px', fontFamily: 'inherit', whiteSpace: 'nowrap',
          }}>{t.label}</button>
      ))}
      {rightContent != null && (
        <>
          <div style={{ flex: 1 }} />
          {rightContent}
        </>
      )}
    </div>
  );
}
