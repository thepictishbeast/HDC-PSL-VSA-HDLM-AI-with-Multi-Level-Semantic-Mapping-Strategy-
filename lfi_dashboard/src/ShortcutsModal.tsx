import React, { useRef } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';
// c2-341: 20px close button sourced from design-system (T.typography caps at 22).
import { typography as dsType } from './design-system';
import { IS_MAC } from './util';

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
      { keys: ['⌘', '1'], label: 'Switch to Agora (Chat)' },
      { keys: ['⌘', '2'], label: 'Switch to Classroom' },
      { keys: ['⌘', '3'], label: 'Open Admin console' },
      { keys: ['⌘', '4'], label: 'Switch to Fleet (orchestrator)' },
      { keys: ['⌘', '5'], label: 'Switch to Library (sources)' },
      { keys: ['⌘', '6'], label: 'Switch to Auditorium (AVP-2 state)' },
      { keys: ['⌘', 'Shift', 'K'], label: 'Open knowledge browser' },
      { keys: ['⌘', 'F'], label: 'Search this conversation (in Chat view)' },
      { keys: ['⌘', 'Shift', 'F'], label: 'Search anywhere (overrides browser find)' },
      { keys: ['⌘', 'Shift', 'L'], label: 'Jump to Admin → Logs' },
    ],
  },
  {
    group: 'Chat',
    items: [
      { keys: ['⌘', 'N'], label: 'New conversation' },
      { keys: ['⌘', 'E'], label: 'Focus the message input' },
      { keys: ['⌘', '/'], label: 'Focus the message input' },
      { keys: ['any letter'], label: 'Auto-focuses input + types it' },
      { keys: ['Shift', '↑'], label: 'Walk back through last 10 sent prompts' },
      { keys: ['Shift', '↓'], label: 'Walk forward through prompt history' },
      { keys: ['⌘', 'Shift', 'R'], label: 'Regenerate last assistant response' },
      { keys: ['⌘', 'Z'], label: 'Undo last conversation delete (within 5s)' },
      { keys: ['⌘', 'Home'], label: 'Scroll chat to top' },
      { keys: ['⌘', 'End'], label: 'Scroll chat to bottom' },
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
      { keys: ['⌘', 'Shift', 'A'], label: 'Toggle auto theme (follow OS)' },
    ],
  },
  // c2-433 / #178: chat search shortcuts. Cmd+F opens it (Navigation group),
  // these keys are active inside the search input.
  {
    group: 'Chat search (search bar focused)',
    items: [
      { keys: ['Enter'], label: 'Jump to next match' },
      { keys: ['Shift', 'Enter'], label: 'Jump to previous match' },
      { keys: ['Esc'], label: 'Close search + clear query' },
    ],
  },
  // c2-250 / #112: document the sidebar row shortcuts added in c2-247+249
  // so the cheatsheet matches the behaviour. All modifier-free — works on
  // whichever row has focus.
  {
    group: 'Conversations (on focused sidebar row)',
    items: [
      { keys: ['↑'], label: 'Focus previous conversation' },
      { keys: ['↓'], label: 'Focus next conversation' },
      { keys: ['Home'], label: 'Focus first conversation' },
      { keys: ['End'], label: 'Focus last conversation' },
      { keys: ['Enter'], label: 'Open the focused conversation' },
      { keys: ['P'], label: 'Pin / unpin' },
      { keys: ['S'], label: 'Star / unstar' },
      { keys: ['F2'], label: 'Rename inline' },
      { keys: ['Del'], label: 'Delete (with undo toast)' },
      { keys: ['drag'], label: 'Drag a pinned row to reorder' },
    ],
  },
];

export interface ShortcutsModalProps {
  C: any;
  onClose: () => void;
  // #351 polish: link the full user guide from the cheatsheet footer so
  // users can jump from "what's the shortcut?" to "what does this do?" in
  // one click. Optional — shipping new calls with it, existing ones still
  // work without.
  onOpenUserGuide?: () => void;
}

export const ShortcutsModal: React.FC<ShortcutsModalProps> = ({ C, onClose, onOpenUserGuide }) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  // c2-265: IS_MAC sourced from util.ts so the substitution rule matches the
  // one used by the command palette chip.
  const renderKey = (k: string) => (k === '\u2318' && !IS_MAC ? 'Ctrl' : k);
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
          width: '100%', maxWidth: '520px', maxHeight: '85dvh', overflowY: 'auto',
          background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
          padding: T.spacing.xl, boxShadow: T.shadows.modal,
        }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: T.spacing.lg }}>
          <h2 id='scc-shortcuts-title' style={{ margin: 0, fontSize: T.typography.sizeLg, fontWeight: T.typography.weightBlack, letterSpacing: '0.12em', textTransform: 'uppercase', color: C.text }}>
            Keyboard Shortcuts
          </h2>
          <button onClick={onClose} aria-label='Close shortcuts'
            style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: dsType.sizes.xl, cursor: 'pointer' }}>
            {'\u2715'}
          </button>
        </div>
        {SHORTCUTS.map(g => (
          <div key={g.group} style={{ marginBottom: '18px' }}>
            <div style={{
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold, color: C.textMuted,
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
                          fontFamily: T.typography.fontMono,
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
            padding: '1px 6px', fontSize: T.typography.sizeXs,
            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            borderRadius: T.radii.xs, fontFamily: T.typography.fontMono,
          }}>?</kbd> any time to reopen this.
          {onOpenUserGuide && (
            <>
              <span style={{ margin: '0 8px', color: C.borderSubtle }}>·</span>
              <button onClick={onOpenUserGuide}
                style={{
                  background: 'transparent', border: 'none', color: C.accent,
                  cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeXs, textDecoration: 'underline',
                  padding: 0,
                }}>Open user guide</button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
