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
