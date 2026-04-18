import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

// c2-356 / task #67 (local): in-browser xterm.js terminal.
//
// First version is client-only: the prompt accepts a small set of local
// commands (help, clear, echo, date, version) so the terminal actually
// echoes something useful without a backend wire. Future work:
//   - pipe input/output to a backend WS endpoint (e.g. /ws/shell) so
//     this becomes a real remote shell
//   - command history (up/down arrow) persisted to localStorage
//   - ANSI color preserved in output (currently plain text only)
//
// AVP-PASS-26 (first-contact test): on open, focus goes to the terminal,
// a welcome banner explains the currently-available commands, and Esc
// closes the modal. No keyboard dead-ends.
//
// SECURITY: no remote execution yet. If a /ws/shell backend lands, the
// websocket must require the existing sovereign-key auth and rate-limit
// per-message -- do NOT let an unauthenticated socket run `bash`.

export interface XTermModalProps {
  C: any;
  onClose: () => void;
}

// Lazy command registry. Keeping this client-local + declarative so the
// initial UI isn't blocked on a backend. When a command isn't found we
// suggest `help` rather than silently failing.
type CmdHandler = (term: Terminal, args: string[]) => void;
const COMMANDS: Record<string, { desc: string; run: CmdHandler }> = {
  help: {
    desc: 'List available commands.',
    run: (term) => {
      term.writeln('');
      term.writeln('\x1b[1mAvailable commands:\x1b[0m');
      for (const [name, c] of Object.entries(COMMANDS)) {
        term.writeln(`  \x1b[36m${name.padEnd(10)}\x1b[0m ${c.desc}`);
      }
    },
  },
  clear: { desc: 'Clear the screen.', run: (term) => term.clear() },
  echo: {
    desc: 'Print arguments.',
    run: (term, args) => term.writeln(args.join(' ')),
  },
  date: {
    desc: 'Print current date/time.',
    run: (term) => term.writeln(new Date().toString()),
  },
  version: {
    desc: 'Print terminal + xterm.js version.',
    run: (term) => term.writeln('PlausiDen xterm v1 (client-local; no backend)'),
  },
};

export const XTermModal: React.FC<XTermModalProps> = ({ C, onClose }) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  const termHostRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  useModalFocus(true, dialogRef);
  const [promptBuffer, setPromptBuffer] = useState('');

  useEffect(() => {
    if (!termHostRef.current) return;
    const term = new Terminal({
      cursorBlink: true,
      fontFamily: "'JetBrains Mono','Fira Code',monospace",
      fontSize: 13,
      theme: {
        background: C.bgCard,
        foreground: C.text,
        cursor: C.accent,
        selectionBackground: C.accentBg,
      },
      convertEol: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(termHostRef.current);
    fit.fit();
    termRef.current = term;

    // Welcome banner. Stick to printable ASCII so users on terminal emulators
    // that don't support the full unicode range still see something legible.
    term.writeln('\x1b[1mPlausiDen terminal\x1b[0m - type `help` for commands');
    term.writeln('');
    term.write('> ');

    // Line editor -- collects keystrokes into `buf` and dispatches on Enter.
    // Exposed via the onData callback rather than DOM listeners so the
    // terminal owns its own input even under focus contention.
    //
    // c2-386 / BIG #182 (frontend slice): command history with up/down
    // arrow recall and localStorage persistence. history is oldest-first;
    // histIdx points into it while navigating. idx === history.length means
    // "current in-progress buf" (so Down from the last recalled entry
    // restores what the user was typing).
    const HIST_KEY = 'lfi_xterm_history';
    const HIST_CAP = 100;
    const loadHistory = (): string[] => {
      try {
        const raw = localStorage.getItem(HIST_KEY);
        return raw ? (JSON.parse(raw) as string[]).slice(-HIST_CAP) : [];
      } catch { return []; }
    };
    const history: string[] = loadHistory();
    let histIdx = history.length;      // cursor into history; length = "editing"
    let draft = '';                    // saved partial buf while browsing history
    let buf = '';
    const clearLine = () => {
      // Replace the current line buffer on the terminal. Emit enough \b to
      // wipe buf chars, then overwrite with spaces, then \b back to start.
      while (buf.length > 0) {
        term.write('\b \b');
        buf = buf.slice(0, -1);
      }
    };
    const writeBuf = (s: string) => { buf = s; term.write(s); };
    const disposer = term.onData((data) => {
      // Arrow keys arrive as CSI escape sequences: \x1b [ A (up) / B (down).
      // Detect them as whole tokens rather than iterating char-by-char.
      if (data === '\x1b[A') {
        if (history.length === 0) return;
        if (histIdx === history.length) draft = buf;
        if (histIdx > 0) histIdx -= 1;
        clearLine();
        writeBuf(history[histIdx]);
        return;
      }
      if (data === '\x1b[B') {
        if (histIdx >= history.length) return;
        histIdx += 1;
        clearLine();
        writeBuf(histIdx === history.length ? draft : history[histIdx]);
        return;
      }
      for (const ch of data) {
        const code = ch.charCodeAt(0);
        if (code === 13) {               // Enter
          term.writeln('');
          const parts = buf.trim().split(/\s+/).filter(Boolean);
          if (parts.length > 0) {
            const trimmed = buf.trim();
            if (trimmed && (history.length === 0 || history[history.length - 1] !== trimmed)) {
              history.push(trimmed);
              if (history.length > HIST_CAP) history.shift();
              try { localStorage.setItem(HIST_KEY, JSON.stringify(history)); } catch { /* quota */ }
            }
            const [name, ...args] = parts;
            const c = COMMANDS[name];
            if (c) {
              try { c.run(term, args); }
              catch (err) { term.writeln(`\x1b[31merror: ${String(err)}\x1b[0m`); }
            } else {
              term.writeln(`\x1b[31mcommand not found:\x1b[0m ${name} (try \`help\`)`);
            }
          }
          buf = '';
          draft = '';
          histIdx = history.length;
          term.write('> ');
        } else if (code === 127) {       // Backspace
          if (buf.length > 0) {
            buf = buf.slice(0, -1);
            term.write('\b \b');
          }
        } else if (code >= 32 && code < 127) {
          buf += ch;
          term.write(ch);
        }
        // Control chars + arrows currently ignored; history is future work.
      }
      setPromptBuffer(buf);
    });

    // Refit on window resize so the terminal grid matches the host element.
    const onResize = () => { try { fit.fit(); } catch { /* safe */ } };
    window.addEventListener('resize', onResize);

    return () => {
      window.removeEventListener('resize', onResize);
      disposer.dispose();
      term.dispose();
    };
  }, [C.accent, C.accentBg, C.bgCard, C.text]);

  return (
    <div onClick={onClose}
      style={{
        position: 'fixed', inset: 0, zIndex: T.z.modal,
        background: C.overlayBg,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        padding: T.spacing.lg,
      }}>
      <div ref={dialogRef} role='dialog' aria-modal='true' aria-label='In-browser terminal'
        onClick={(e) => e.stopPropagation()}
        style={{
          width: '100%', maxWidth: '900px', height: '560px',
          background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
          boxShadow: T.shadows.modal,
          display: 'flex', flexDirection: 'column', overflow: 'hidden',
        }}>
        <div style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          padding: `${T.spacing.md} ${T.spacing.lg}`,
          borderBottom: `1px solid ${C.borderSubtle}`,
        }}>
          <h2 style={{
            margin: 0, fontSize: T.typography.sizeMd, fontWeight: T.typography.weightBlack,
            letterSpacing: T.typography.trackingCap, textTransform: 'uppercase', color: C.text,
          }}>Terminal</h2>
          <button onClick={onClose} aria-label='Close terminal'
            style={{
              background: 'transparent', border: 'none', color: C.textMuted,
              fontSize: T.typography.size2xl, cursor: 'pointer',
            }}>{'\u2715'}</button>
        </div>
        <div ref={termHostRef} style={{
          flex: 1, background: C.bgCard,
          padding: T.spacing.sm,
        }} />
      </div>
    </div>
  );
};
