import React from 'react';

// Sub-components of the chat message list. Extracted from App.tsx in stages —
// system + web first (zero-closure, trivial) so the pattern is proven before
// tackling the heavier tool / user / assistant branches.

export const SystemMessage: React.FC<{ content: string; C: any }> = ({ content, C }) => (
  <div style={{
    textAlign: 'center', padding: '8px 16px', fontSize: '12px',
    color: C.textMuted, fontStyle: 'italic',
  }}>
    {content}
  </div>
);

export interface ToolMessageProps {
  msg: {
    content: string;
    toolName?: string;
    toolStatus?: 'running' | 'ok' | 'error';
    toolInput?: string;
    toolOutput?: string;
    toolDuration?: number;
  };
  C: any;
  isDesktop: boolean;
  expanded: boolean;
  onToggle: () => void;
}

// Tool-call message: Claude Code-style expandable block showing what the AI
// invoked, its status, and (when expanded) the input/output. Parent owns the
// expanded Set — we take the flag + toggle handler via props.
export const ToolMessage: React.FC<ToolMessageProps> = ({ msg, C, isDesktop, expanded, onToggle }) => {
  const statusColor = msg.toolStatus === 'ok' ? C.green
    : msg.toolStatus === 'error' ? C.red : C.accent;
  const statusBg = msg.toolStatus === 'ok' ? C.greenBg
    : msg.toolStatus === 'error' ? C.redBg : C.accentBg;
  return (
    <div style={{
      maxWidth: isDesktop ? '80%' : '96%',
      border: `1px solid ${C.borderSubtle}`,
      borderRadius: '10px', overflow: 'hidden',
      background: C.bgCard,
    }}>
      <button onClick={onToggle}
        style={{
          width: '100%', display: 'flex', alignItems: 'center', gap: '10px',
          padding: '10px 14px', background: 'transparent',
          border: 'none', cursor: 'pointer', fontFamily: 'inherit',
          color: C.text, textAlign: 'left',
        }}>
        {/* Status dot */}
        <div style={{
          width: '8px', height: '8px', borderRadius: '50%',
          background: statusColor, flexShrink: 0,
          boxShadow: msg.toolStatus === 'running' ? `0 0 6px ${statusColor}` : 'none',
          animation: msg.toolStatus === 'running' ? 'scc-bounce 1.4s infinite ease-in-out' : 'none',
        }} />
        {/* Tool name */}
        <span style={{
          fontSize: '13px', fontWeight: 600,
          fontFamily: "'JetBrains Mono','Fira Code',monospace",
        }}>{msg.toolName || 'tool'}</span>
        {/* Summary */}
        <span style={{
          flex: 1, fontSize: '12px', color: C.textMuted,
          whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
        }}>{msg.content}</span>
        {/* Status badge */}
        <span style={{
          padding: '2px 8px', fontSize: '10px', fontWeight: 700,
          background: statusBg, color: statusColor,
          borderRadius: '4px', textTransform: 'uppercase',
          flexShrink: 0,
        }}>{msg.toolStatus || 'done'}</span>
        {msg.toolDuration != null && (
          <span style={{
            fontSize: '10px', color: C.textDim, flexShrink: 0,
            fontFamily: "'JetBrains Mono','Fira Code',monospace",
          }}>{msg.toolDuration}ms</span>
        )}
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke={C.textMuted}
          strokeWidth="2" style={{ flexShrink: 0, transform: expanded ? 'rotate(180deg)' : 'rotate(0)', transition: 'transform 0.15s' }}>
          <polyline points="6 9 12 15 18 9"/>
        </svg>
      </button>
      {expanded && (
        <div style={{ padding: '0 14px 12px', fontSize: '12px' }}>
          {msg.toolInput && (
            <div style={{ marginBottom: '8px' }}>
              <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Input</div>
              <pre style={{
                padding: '8px 10px', background: C.bgInput, borderRadius: '6px',
                fontFamily: "'JetBrains Mono','Fira Code',monospace",
                fontSize: '11.5px', color: C.textSecondary,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0,
              }}>{msg.toolInput}</pre>
            </div>
          )}
          {msg.toolOutput && (
            <div>
              <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Output</div>
              <pre style={{
                padding: '8px 10px', background: C.bgInput, borderRadius: '6px',
                fontFamily: "'JetBrains Mono','Fira Code',monospace",
                fontSize: '11.5px', color: C.textSecondary,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0,
                maxHeight: '300px', overflowY: 'auto',
              }}>{msg.toolOutput}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export interface UserMessageProps {
  msg: { id: number; content: string; timestamp: number };
  C: any;
  isMobile: boolean;
  maxWidth: string;
  editing: boolean;
  editText: string;
  setEditText: (s: string) => void;
  onBeginEdit: () => void;
  onCancelEdit: () => void;
  onCommitEdit: (trimmed: string) => void;
  formatTime: (ts: number) => string;
}

// User message bubble with inline edit flow. Parent owns the editing state
// so cross-message coordination (only one editor open at a time) stays simple.
export const UserMessage: React.FC<UserMessageProps> = ({
  msg, C, isMobile, maxWidth, editing, editText, setEditText,
  onBeginEdit, onCancelEdit, onCommitEdit, formatTime,
}) => (
  <div
    onMouseEnter={(e) => { const btn = e.currentTarget.querySelector('.user-edit-btn') as HTMLElement; if (btn) btn.style.opacity = '1'; }}
    onMouseLeave={(e) => { const btn = e.currentTarget.querySelector('.user-edit-btn') as HTMLElement; if (btn) btn.style.opacity = '0'; }}
    style={{ display: 'flex', justifyContent: 'flex-end', gap: '6px', alignItems: 'flex-end' }}>
    {!editing && (
      <button className='user-edit-btn'
        onClick={onBeginEdit}
        title='Edit and resend'
        style={{
          width: '28px', height: '28px', flexShrink: 0,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: 'transparent', border: 'none',
          color: C.textMuted, cursor: 'pointer', borderRadius: '6px',
          opacity: isMobile ? 1 : 0, transition: 'opacity 0.12s',
        }}
        onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; }}
        onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </svg>
      </button>
    )}
    {editing ? (
      <div style={{
        maxWidth, width: '100%',
        background: C.bgCard, border: `1px solid ${C.accent}`,
        borderRadius: '12px', padding: '10px',
      }}>
        <textarea value={editText}
          onChange={(e) => setEditText(e.target.value)}
          style={{
            width: '100%', background: 'transparent', border: 'none', outline: 'none',
            color: C.text, fontFamily: 'inherit', fontSize: '14px', lineHeight: '1.6',
            resize: 'vertical', minHeight: '60px',
          }} />
        <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end', marginTop: '8px' }}>
          <button onClick={onCancelEdit}
            style={{ padding: '6px 14px', fontSize: '12px', background: 'transparent',
              border: `1px solid ${C.border}`, color: C.textMuted, borderRadius: '6px',
              cursor: 'pointer', fontFamily: 'inherit' }}>Cancel</button>
          <button onClick={() => {
            const trimmed = editText.trim();
            if (!trimmed) return;
            onCommitEdit(trimmed);
          }}
            style={{ padding: '6px 14px', fontSize: '12px',
              background: C.accent, border: 'none', color: '#fff',
              borderRadius: '6px', cursor: 'pointer', fontFamily: 'inherit', fontWeight: 600 }}>
            Send
          </button>
        </div>
      </div>
    ) : (
      <div style={{
        maxWidth, padding: '12px 16px',
        background: C.accent,
        borderRadius: '16px 16px 4px 16px', fontSize: '14px', lineHeight: '1.6',
        color: '#fff',
        wordBreak: 'break-word',
        boxShadow: `0 1px 4px rgba(0,0,0,0.10)`,
      }}>
        {msg.content}
        <div style={{ fontSize: '10px', color: 'rgba(255,255,255,0.55)', marginTop: '6px', textAlign: 'right' }}>
          {formatTime(msg.timestamp)}
        </div>
      </div>
    )}
  </div>
);

export const WebMessage: React.FC<{ content: string; C: any; isDesktop: boolean }> = ({ content, C, isDesktop }) => (
  <div style={{
    padding: '14px 16px', borderRadius: '12px',
    background: C.greenBg, border: `1px solid ${C.greenBorder}`,
    maxWidth: isDesktop ? '75%' : '100%',
  }}>
    <div style={{ fontSize: '11px', fontWeight: 800, color: C.green, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '8px' }}>
      Web Intelligence
    </div>
    <pre style={{
      whiteSpace: 'pre-wrap', wordBreak: 'break-word',
      fontSize: '13px', lineHeight: '1.6', color: C.text, margin: 0,
    }}>{content}</pre>
  </div>
);
