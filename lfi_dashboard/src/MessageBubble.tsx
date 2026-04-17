import React from 'react';
import { T } from './tokens';

// Sub-components of the chat message list. Extracted from App.tsx in stages —
// system + web first (zero-closure, trivial) so the pattern is proven before
// tackling the heavier tool / user / assistant branches.

export const SystemMessage: React.FC<{ content: string; C: any }> = React.memo(({ content, C }) => (
  <div style={{
    textAlign: 'center', padding: `${T.spacing.sm} ${T.spacing.lg}`, fontSize: T.typography.sizeSm,
    color: C.textMuted, fontStyle: 'italic',
  }}>
    {content}
  </div>
));

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
      borderRadius: T.radii.lg, overflow: 'hidden',
      background: C.bgCard,
    }}>
      <button onClick={onToggle}
        aria-expanded={expanded}
        aria-label={`${msg.toolName || 'tool'} — ${expanded ? 'collapse' : 'expand'}`}
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
        aria-label='Edit message and resend'
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
        borderRadius: T.radii.xl, padding: '10px',
      }}>
        <textarea value={editText}
          onChange={(e) => setEditText(e.target.value)}
          aria-label='Edit message'
          autoComplete='off'
          spellCheck={true}
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
        maxWidth, padding: `${T.spacing.md} ${T.spacing.lg}`,
        // c0-019/020: user bubble uses muted accent-bg (not saturated accent),
        // with body-color text for readability. Professional feel over
        // "branded message balloon."
        background: C.accentBg, color: C.text,
        borderRadius: `${T.radii.lg} ${T.radii.lg} ${T.radii.xs} ${T.radii.lg}`,
        fontSize: T.typography.sizeBody, lineHeight: T.typography.lineLoose,
        wordBreak: 'break-word',
        border: `1px solid ${C.accentBorder}`,
      }}>
        {msg.content}
        <div style={{ fontSize: '10px', color: C.textMuted, marginTop: '6px', textAlign: 'right' }}>
          {formatTime(msg.timestamp)}
        </div>
      </div>
    )}
  </div>
);

export interface AssistantMessageProps {
  msg: {
    id: number;
    content: string;
    timestamp: number;
    conclusion_id?: number;
    reasoning?: string[];
    confidence?: number;
  };
  C: any;
  isMobile: boolean;
  isDesktop: boolean;
  isLast: boolean;
  isThinking: boolean;
  showReasoning: boolean;
  developerMode: boolean;
  reasoningExpanded: boolean;
  renderBody: (text: string) => React.ReactNode;
  onToggleReasoning: () => void;
  onRegenerate: () => void;
  onCopy: (text: string) => void;
  onOpenProvenance: (conclusion_id: number) => void;
  onFollowUpChip: (prompt: string) => void;
  onFeedbackPositive: () => void;
  onFeedbackNegative: () => void;
  formatTime: (ts: number) => string;
}

// Assistant message — the heaviest bubble. Hover-revealed action bar with copy,
// regenerate (last turn only), thumbs up/down, timestamp. Follow-up suggestion
// chips and the reasoning toggle are rendered below when applicable.
export const AssistantMessage: React.FC<AssistantMessageProps> = ({
  msg, C, isMobile, isDesktop, isLast, isThinking,
  showReasoning, developerMode, reasoningExpanded,
  renderBody, onToggleReasoning, onRegenerate, onCopy,
  onOpenProvenance, onFollowUpChip, onFeedbackPositive, onFeedbackNegative, formatTime,
}) => {
  // Follow-up chips — simple keyword extraction, only on last assistant message
  // when the body is long enough to have meaningful topics.
  const chips: string[] = (() => {
    if (!isLast || msg.content.length <= 40) return [];
    const words = msg.content.toLowerCase().split(/\s+/).filter(w => w.length > 5);
    const unique = [...new Set(words)].slice(0, 20);
    const topics = unique.filter(w => !['about','which','would','could','should','these','those','there','their','really','actually','because','through'].includes(w)).slice(0, 3);
    if (topics.length === 0) return [];
    return [
      topics[0] ? `Tell me more about ${topics[0]}` : null,
      topics[1] ? `How does ${topics[1]} work?` : null,
      topics[2] ? `What's the connection to ${topics[2]}?` : null,
    ].filter(Boolean) as string[];
  })();

  return (
    <div
      onMouseEnter={(e) => { (e.currentTarget.querySelector('.lfi-msg-actions') as HTMLElement)?.style.setProperty('opacity', '1'); }}
      onMouseLeave={(e) => { (e.currentTarget.querySelector('.lfi-msg-actions') as HTMLElement)?.style.setProperty('opacity', '0'); }}
      style={{ display: 'flex', justifyContent: 'flex-start' }}>
      <div style={{ maxWidth: isDesktop ? '80%' : '96%', width: '100%' }}>
        {/* Response body */}
        <div style={{
          padding: '14px 18px',
          background: C.bgCard,
          border: `1px solid ${C.border}`,
          borderRadius: `${T.radii.xs} ${T.radii.lg} ${T.radii.lg} ${T.radii.lg}`,
          fontSize: '14px', lineHeight: '1.7',
          color: C.text,
          whiteSpace: 'pre-wrap', wordBreak: 'break-word',
        }}>
          {renderBody(msg.content)}
          {developerMode && msg.conclusion_id != null && (
            <span title={`Provenance: conclusion #${msg.conclusion_id}`}
              role='button' tabIndex={0}
              aria-label={`Open provenance for conclusion ${msg.conclusion_id}`}
              onClick={() => onOpenProvenance(msg.conclusion_id!)}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onOpenProvenance(msg.conclusion_id!); } }}
              style={{
                display: 'inline-block', marginLeft: '8px',
                padding: '1px 6px', fontSize: '10px',
                background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                borderRadius: '4px', color: C.textDim,
                cursor: 'pointer', fontFamily: "'JetBrains Mono',monospace",
              }}>
              #{msg.conclusion_id}
            </span>
          )}
        </div>

        {/* Hover-revealed action bar */}
        <div className='lfi-msg-actions'
          style={{
            display: 'flex', gap: '4px', marginTop: '4px',
            justifyContent: 'flex-end',
            opacity: isMobile ? 1 : 0,
            transition: 'opacity 0.15s',
          }}>
          <button onClick={() => onCopy(msg.content)} title='Copy message' aria-label='Copy message'
            style={{
              width: '30px', height: '30px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'transparent', border: 'none',
              color: C.textMuted, borderRadius: '6px', cursor: 'pointer',
              fontFamily: 'inherit',
            }}
            onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; e.currentTarget.style.color = C.text; }}
            onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = C.textMuted; }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2"/>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
          </button>
          {isLast && (
            <button onClick={onRegenerate} title='Regenerate' aria-label='Regenerate last response'
              disabled={isThinking}
              style={{
                width: '30px', height: '30px',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                background: 'transparent', border: 'none',
                color: C.textMuted, borderRadius: '6px',
                cursor: isThinking ? 'wait' : 'pointer', fontFamily: 'inherit',
              }}
              onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; e.currentTarget.style.color = C.text; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = C.textMuted; }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="1 4 1 10 7 10"/>
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
              </svg>
            </button>
          )}
          <button onClick={onFeedbackPositive} title='Good response' aria-label='Mark as good response'
            style={{
              width: '30px', height: '30px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'transparent', border: 'none',
              color: C.textMuted, borderRadius: '6px', cursor: 'pointer',
            }}
            onMouseEnter={(e) => { e.currentTarget.style.color = C.green; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = C.textMuted; }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3"/>
            </svg>
          </button>
          <button onClick={onFeedbackNegative}
            title='Bad response — tell us what it should have said'
            aria-label='Mark as bad response'
            style={{
              width: '30px', height: '30px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'transparent', border: 'none',
              color: C.textMuted, borderRadius: '6px', cursor: 'pointer',
            }}
            onMouseEnter={(e) => { e.currentTarget.style.color = C.red; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = C.textMuted; }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17"/>
            </svg>
          </button>
          {typeof msg.confidence === 'number' && (() => {
            // Confidence badge next to the action bar. Green ≥0.75, yellow 0.5-0.75,
            // amber-ish below. Gives users a cue when the AI is low-confidence so
            // they calibrate trust (addresses c0-008 bug #5 — quality indicator).
            const pct = Math.round(msg.confidence * 100);
            const color = msg.confidence >= 0.75 ? C.green : msg.confidence >= 0.5 ? C.yellow : C.red;
            const bg = msg.confidence >= 0.75 ? C.greenBg : msg.confidence >= 0.5 ? C.accentBg : C.redBg;
            return (
              <span
                title={`Model confidence on this response: ${pct}%`}
                style={{
                  fontSize: '10px', fontWeight: 700, color, background: bg,
                  padding: '2px 8px', borderRadius: '4px', alignSelf: 'center',
                  fontFamily: "'JetBrains Mono', monospace",
                }}>
                {pct}% confident
              </span>
            );
          })()}
          <span style={{
            fontSize: '10px', color: C.textDim, alignSelf: 'center',
            padding: '0 8px',
          }}>{formatTime(msg.timestamp)}</span>
        </div>
        {/* Last-message helpfulness nudge — only on the latest assistant reply,
            fades to invisibility once user votes (tracked via onFeedbackPositive/
            onFeedbackNegative side effects at the parent). Addresses c0-008 #5. */}
        {isLast && !isThinking && (
          <div style={{
            fontSize: '11px', color: C.textDim, textAlign: 'right',
            marginTop: '4px', paddingRight: '4px',
            fontStyle: 'italic',
          }}>Was this helpful? Use 👍 / 👎 above.</div>
        )}

        {/* Follow-up suggestion chips — only on last assistant message with enough content. */}
        {chips.length > 0 && (
          <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', marginTop: '8px' }}>
            {chips.map((chip, ci) => (
              <button key={ci}
                onClick={() => onFollowUpChip(chip)}
                style={{
                  padding: '6px 12px', fontSize: '12px',
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                  color: C.textSecondary, borderRadius: '999px',
                  cursor: 'pointer', fontFamily: 'inherit',
                  transition: 'border-color 0.15s',
                }}
                onMouseEnter={(e) => e.currentTarget.style.borderColor = C.accent}
                onMouseLeave={(e) => e.currentTarget.style.borderColor = C.borderSubtle}>
                {chip}
              </button>
            ))}
          </div>
        )}

        {/* Reasoning toggle — gated on user preference + presence of reasoning. */}
        {showReasoning && msg.reasoning && msg.reasoning.length > 0 && (
          <div style={{ marginTop: '8px' }}>
            <button
              onClick={onToggleReasoning}
              aria-expanded={reasoningExpanded}
              aria-controls={`lfi-reasoning-${msg.id}`}
              style={{
                display: 'flex', alignItems: 'center', gap: '6px',
                padding: '5px 10px', fontSize: '11px', fontWeight: 600,
                color: C.textMuted, background: 'transparent',
                border: `1px solid ${C.borderSubtle}`, borderRadius: '6px',
                cursor: 'pointer', fontFamily: 'inherit',
              }}>
              Show reasoning ({msg.reasoning.length}) {reasoningExpanded ? '\u25B2' : '\u25BC'}
            </button>
            {reasoningExpanded && (
              <div id={`lfi-reasoning-${msg.id}`} role='region' aria-label='Reasoning trace'
                style={{
                  marginTop: '8px', padding: '12px 14px',
                  background: C.bgInput,
                  borderLeft: `3px solid ${C.accent}`,
                  borderRadius: '0 8px 8px 0',
                }}>
                {msg.reasoning.map((step, j) => (
                  <p key={j} style={{ fontSize: '12px', color: C.textSecondary, lineHeight: '1.6', margin: '4px 0' }}>
                    <span style={{ color: C.accent, fontWeight: 700 }}>[{j}]</span> {step}
                  </p>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export const WebMessage: React.FC<{ content: string; C: any; isDesktop: boolean }> = React.memo(({ content, C, isDesktop }) => (
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
));
