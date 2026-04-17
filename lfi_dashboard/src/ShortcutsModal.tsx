import React, { useRef } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

// Keyboard-shortcut cheatsheet. Opened with "?" (standard pattern from
// GitHub/Gmail/etc). Content is static — if shortcuts change in App.tsx,
// update the SHORTCUTS list below in the same commit.

const SHORTCUTS: Array<{ group: string; items: Array<{ keys: string[]; label: string }> }> = [
  {
    group: 'Navigation',
    items: [
      { keys: ['?'], label: 'Show this cheatsheet' },
      { keys: ['Esc'], label: 'Close the active modal' },
      { keys: ['⌘', 'K'], label: 'Open command palette' },
      { keys: ['⌘', 'B'], label: 'Toggle conversation sidebar' },
      { keys: ['⌘', '1'], label: 'Switch to Chat' },
      { keys: ['⌘', '2'], label: 'Switch to Classroom' },
      { keys: ['⌘', '3'], label: 'Open Admin console' },
      { keys: ['⌘', 'Shift', 'K'], label: 'Open knowledge browser' },
      { keys: ['⌘', 'Shift', 'F'], label: 'Search this conversation' },
    ],
  },
  {
    group: 'Chat',
    items: [
      { keys: ['⌘', 'N'], label: 'New conversation' },
      { keys: ['⌘', 'E'], label: 'Focus the message input' },
      { keys: ['⌘', '/'], label: 'Focus the message input' },
      { keys: ['any letter'], label: 'Auto-focuses input + types it' },
      { keys: ['Shift', '↑'], label: 'Recall your last sent message' },
      { keys: ['⌘', 'Shift', 'R'], label: 'Regenerate last assistant response' },
      { keys: ['Enter'], label: 'Send (when sendOnEnter is on)' },
      { keys: ['Shift', 'Enter'], label: 'New line in the input' },
      { keys: ['Esc'], label: 'Stop in-flight request (or close modal)' },
    ],
  },
  {
    group: 'Preferences',
    items: [
      { keys: ['⌘', ','], label: 'Open settings' },
      { keys: ['⌘', 'D'], label: 'Toggle developer mode' },
      { keys: ['⌘', 'Shift', 'D'], label: 'Cycle through themes' },
    ],
  },
];

export interface ShortcutsModalProps {
  C: any;
  onClose: () => void;
}

export const ShortcutsModal: React.FC<ShortcutsModalProps> = ({ C, onClose }) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  const isMac = typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('mac');
  const renderKey = (k: string) => (k === '⌘' && !isMac ? 'Ctrl' : k);
  return (
    <div onClick={onClose}
      style={{
        position: 'fixed', inset: 0, zIndex: 240,
        background: 'rgba(0,0,0,0.55)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        padding: T.spacing.lg,
      }}>
      <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-shortcuts-title'
        onClick={(e) => e.stopPropagation()}
        style={{
          width: '100%', maxWidth: '520px', maxHeight: '85vh', overflowY: 'auto',
          background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
          padding: T.spacing.xl, boxShadow: T.shadows.modal,
        }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: T.spacing.lg }}>
          <h2 id='scc-shortcuts-title' style={{ margin: 0, fontSize: '15px', fontWeight: T.typography.weightBlack, letterSpacing: '0.12em', textTransform: 'uppercase', color: C.text }}>
            Keyboard Shortcuts
          </h2>
          <button onClick={onClose} aria-label='Close shortcuts'
            style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
            {'\u2715'}
          </button>
        </div>
        {SHORTCUTS.map(g => (
          <div key={g.group} style={{ marginBottom: '18px' }}>
            <div style={{
              fontSize: '10px', fontWeight: T.typography.weightBold, color: C.textMuted,
              textTransform: 'uppercase', letterSpacing: '0.12em', marginBottom: T.spacing.sm,
            }}>{g.group}</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              {g.items.map((sc, i) => (
                <div key={i} style={{
                  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                  fontSize: T.typography.sizeMd, padding: '4px 0',
                }}>
                  <span style={{ color: C.textSecondary }}>{sc.label}</span>
                  <span style={{ display: 'flex', gap: T.spacing.xs, alignItems: 'center' }}>
                    {sc.keys.map((k, j) => (
                      <React.Fragment key={j}>
                        {j > 0 && <span style={{ color: C.textDim, fontSize: T.typography.sizeXs }}>+</span>}
                        <kbd style={{
                          padding: '2px 8px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                          background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                          borderRadius: T.radii.xs, color: C.text,
                          fontFamily: "'JetBrains Mono', monospace",
                          minWidth: '22px', textAlign: 'center',
                        }}>{renderKey(k)}</kbd>
                      </React.Fragment>
                    ))}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
        <div style={{
          fontSize: T.typography.sizeXs, color: C.textDim, textAlign: 'center',
          paddingTop: T.spacing.sm, borderTop: `1px solid ${C.borderSubtle}`,
        }}>
          Press <kbd style={{
            padding: '1px 6px', fontSize: '10px',
            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            borderRadius: '3px', fontFamily: "'JetBrains Mono', monospace",
          }}>?</kbd> any time to reopen this.
        </div>
      </div>
    </div>
  );
};
