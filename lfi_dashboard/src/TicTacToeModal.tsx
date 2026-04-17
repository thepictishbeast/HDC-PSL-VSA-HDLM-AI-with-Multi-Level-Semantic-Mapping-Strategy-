import React, { useRef } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

// Tic-Tac-Toe easter egg. Purely presentational — board/turn/winner state and
// the play/reset handlers live in the parent so the game logic stays alongside
// the rest of the app state.
export interface TicTacToeModalProps {
  C: any;
  board: Array<'X' | 'O' | null>;
  winner: string | null;
  onPlay: (i: number) => void;
  onReset: () => void;
  onClose: () => void;
}

export const TicTacToeModal: React.FC<TicTacToeModalProps> = ({ C, board, winner, onPlay, onReset, onClose }) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  return (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: T.z.modal + 30,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: T.spacing.lg,
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-ttt-title'
      onClick={(e) => e.stopPropagation()}
      style={{
        background: C.bgCard, border: `1px solid ${C.border}`,
        borderRadius: T.radii.xxl, padding: '28px',
        boxShadow: T.shadows.modal,
        textAlign: 'center', minWidth: '300px',
      }}>
      <h2 id='scc-ttt-title' style={{ margin: '0 0 4px', fontSize: T.typography.size2xl, fontWeight: T.typography.weightBold, color: C.text }}>Tic-Tac-Toe</h2>
      <p style={{ margin: '0 0 18px', fontSize: T.typography.sizeMd, color: C.textMuted }}>
        {winner || 'You are X. Click a cell to play.'}
      </p>
      <div style={{
        display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '6px',
        width: '200px', margin: '0 auto 18px',
      }}>
        {board.map((cell, i) => (
          <button key={i} onClick={() => onPlay(i)}
            style={{
              width: '60px', height: '60px',
              fontSize: '24px', fontWeight: T.typography.weightBlack,
              background: cell === 'X' ? C.accentBg : cell === 'O' ? C.redBg : C.bgInput,
              border: `1px solid ${C.border}`,
              borderRadius: T.radii.md, cursor: cell || winner ? 'default' : 'pointer',
              color: cell === 'X' ? C.accent : cell === 'O' ? C.red : 'transparent',
              fontFamily: 'inherit',
              transition: `background ${T.motion.fast}`,
            }}>
            {cell || '\u00B7'}
          </button>
        ))}
      </div>
      <div style={{ display: 'flex', gap: T.spacing.md, justifyContent: 'center' }}>
        <button onClick={onReset}
          style={{
            padding: '8px 18px', background: C.accentBg, border: `1px solid ${C.accentBorder}`,
            color: C.accent, borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
            fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
          }}>New game</button>
        <button onClick={onClose}
          style={{
            padding: '8px 18px', background: 'transparent', border: `1px solid ${C.border}`,
            color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
            fontSize: T.typography.sizeMd, fontWeight: T.typography.weightSemibold,
          }}>Close</button>
      </div>
    </div>
  </div>
  );
};
