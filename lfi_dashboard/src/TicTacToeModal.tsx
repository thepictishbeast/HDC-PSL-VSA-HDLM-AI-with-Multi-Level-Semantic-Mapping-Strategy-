import React, { useRef } from 'react';
import { useModalFocus } from './useModalFocus';

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
      position: 'fixed', inset: 0, zIndex: 230,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: '16px',
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-label='Tic-Tac-Toe'
      onClick={(e) => e.stopPropagation()}
      style={{
        background: C.bgCard, border: `1px solid ${C.border}`,
        borderRadius: '16px', padding: '28px',
        boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
        textAlign: 'center', minWidth: '300px',
      }}>
      <h2 style={{ margin: '0 0 4px', fontSize: '18px', fontWeight: 700, color: C.text }}>Tic-Tac-Toe</h2>
      <p style={{ margin: '0 0 18px', fontSize: '13px', color: C.textMuted }}>
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
              fontSize: '24px', fontWeight: 800,
              background: cell === 'X' ? C.accentBg : cell === 'O' ? C.redBg : C.bgInput,
              border: `1px solid ${C.border}`,
              borderRadius: '8px', cursor: cell || winner ? 'default' : 'pointer',
              color: cell === 'X' ? C.accent : cell === 'O' ? C.red : 'transparent',
              fontFamily: 'inherit',
              transition: 'background 0.1s',
            }}>
            {cell || '\u00B7'}
          </button>
        ))}
      </div>
      <div style={{ display: 'flex', gap: '10px', justifyContent: 'center' }}>
        <button onClick={onReset}
          style={{
            padding: '8px 18px', background: C.accentBg, border: `1px solid ${C.accentBorder}`,
            color: C.accent, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
            fontSize: '13px', fontWeight: 600,
          }}>New game</button>
        <button onClick={onClose}
          style={{
            padding: '8px 18px', background: 'transparent', border: `1px solid ${C.border}`,
            color: C.textMuted, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
            fontSize: '13px', fontWeight: 600,
          }}>Close</button>
      </div>
    </div>
  </div>
  );
};
