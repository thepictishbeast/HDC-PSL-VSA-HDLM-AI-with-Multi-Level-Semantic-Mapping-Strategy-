import { useCallback, useState } from 'react';

// Simple 3x3 Tic-Tac-Toe hook: you play X, a deterministic AI plays O
// (center → corners → sides). Returns board/winner + play/reset handlers,
// which map directly onto TicTacToeModal's props.
type Cell = 'X' | 'O' | null;

const WIN_LINES = [
  [0, 1, 2], [3, 4, 5], [6, 7, 8],
  [0, 3, 6], [1, 4, 7], [2, 5, 8],
  [0, 4, 8], [2, 4, 6],
];

const checkWinner = (b: Cell[]): string | null => {
  for (const [a, bb, c] of WIN_LINES) {
    if (b[a] && b[a] === b[bb] && b[a] === b[c]) return b[a] as string;
  }
  return b.every(Boolean) ? 'Draw' : null;
};

const formatOutcome = (w: string): string => {
  if (w === 'X') return 'You win!';
  if (w === 'O') return 'AI wins!';
  return 'Draw!';
};

export interface TicTacToeState {
  board: Cell[];
  winner: string | null;
  play: (idx: number) => void;
  reset: () => void;
}

export const useTicTacToe = (): TicTacToeState => {
  const [board, setBoard] = useState<Cell[]>(() => Array(9).fill(null));
  const [turn, setTurn] = useState<'X' | 'O'>('X');
  const [winner, setWinner] = useState<string | null>(null);

  const play = useCallback((idx: number) => {
    if (board[idx] || winner || turn !== 'X') return;
    const next = [...board];
    next[idx] = 'X';
    const w = checkWinner(next);
    if (w) { setBoard(next); setWinner(formatOutcome(w)); return; }
    // AI move: prefer center, then corners, then sides.
    const empty = next.map((v, i) => (v === null ? i : -1)).filter(i => i >= 0);
    const pref = [4, 0, 2, 6, 8, 1, 3, 5, 7];
    const aiMove = pref.find(p => empty.includes(p)) ?? empty[0];
    if (aiMove != null) next[aiMove] = 'O';
    const w2 = checkWinner(next);
    setBoard(next);
    setTurn('X');
    if (w2) setWinner(formatOutcome(w2));
  }, [board, winner, turn]);

  const reset = useCallback(() => {
    setBoard(Array(9).fill(null));
    setTurn('X');
    setWinner(null);
  }, []);

  return { board, winner, play, reset };
};
