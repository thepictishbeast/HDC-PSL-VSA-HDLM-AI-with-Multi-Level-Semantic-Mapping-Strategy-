import React, { useState, useRef, useCallback } from 'react';
import { T } from './tokens';
import { stripMarkdown, formatRelative, formatDuration } from './util';
// c2-359 / task 65: shared copy button with 2s checkmark flash.
import { CopyButton } from './components';

// c2-433 / task 223: long-press → context-menu shim. Touch users can't right-
// click; this synthesises the same `onContextMenu(MouseEvent)` callback after
// a 500 ms hold without movement. 10 px movement budget cancels (so a scroll
// gesture doesn't double as a long-press). navigator.vibrate(20) gives a
// confirmation tick on devices that support it (Android Chrome). The synthetic
// event provides preventDefault + clientX/clientY since that's what callers
// actually read from MouseEvent — the rest of the surface is unused.
const LONG_PRESS_MS = 500;
const LONG_PRESS_MOVE_PX = 10;
function useLongPress(onLongPress?: (e: React.MouseEvent) => void) {
  const timerRef = useRef<number | null>(null);
  const startRef = useRef<{ x: number; y: number } | null>(null);
  const firedRef = useRef<boolean>(false);
  const cancel = useCallback(() => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    startRef.current = null;
  }, []);
  const onTouchStart = useCallback((e: React.TouchEvent) => {
    if (!onLongPress || e.touches.length !== 1) return;
    const t = e.touches[0];
    startRef.current = { x: t.clientX, y: t.clientY };
    firedRef.current = false;
    timerRef.current = window.setTimeout(() => {
      firedRef.current = true;
      try { (navigator as any).vibrate?.(20); } catch { /* unsupported / blocked */ }
      const synth = {
        preventDefault: () => {},
        stopPropagation: () => {},
        clientX: t.clientX,
        clientY: t.clientY,
      } as unknown as React.MouseEvent;
      onLongPress(synth);
    }, LONG_PRESS_MS);
  }, [onLongPress]);
  const onTouchMove = useCallback((e: React.TouchEvent) => {
    if (!startRef.current || e.touches.length !== 1) return;
    const t = e.touches[0];
    const dx = t.clientX - startRef.current.x;
    const dy = t.clientY - startRef.current.y;
    if (dx * dx + dy * dy > LONG_PRESS_MOVE_PX * LONG_PRESS_MOVE_PX) cancel();
  }, [cancel]);
  const onTouchEnd = useCallback(() => { cancel(); }, [cancel]);
  const onTouchCancel = useCallback(() => { cancel(); }, [cancel]);
  return { onTouchStart, onTouchMove, onTouchEnd, onTouchCancel };
}

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
        {/* Status dot. c2-358 / task 67: when running, render a spinner
            ring (borderLeft accent + borderRight transparent + spin) so the
            motion reads as "working" rather than the earlier bounce-dot
            which was read as "alert". Solid dots stay for ok / error. */}
        {msg.toolStatus === 'running' ? (
          <div aria-label='tool running' style={{
            width: '10px', height: '10px', borderRadius: '50%',
            borderLeft: `2px solid ${C.accent}`,
            borderRight: `2px solid transparent`,
            borderTop: `2px solid ${C.accent}`,
            borderBottom: `2px solid transparent`,
            animation: 'scc-spin 0.8s linear infinite',
            boxSizing: 'border-box', flexShrink: 0,
          }} />
        ) : (
          <div style={{
            width: '8px', height: '8px', borderRadius: '50%',
            background: statusColor, flexShrink: 0,
          }} />
        )}
        {/* Tool name */}
        <span style={{
          fontSize: '13px', fontWeight: 600,
          fontFamily: T.typography.fontMono,
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
          borderRadius: T.radii.sm, textTransform: 'uppercase',
          flexShrink: 0,
        }}>{msg.toolStatus || 'done'}</span>
        {msg.toolDuration != null && (
          <span style={{
            fontSize: '10px', color: C.textDim, flexShrink: 0,
            fontFamily: T.typography.fontMono,
          }}
            // c2-433 / task 283 + 284: formatDuration helper from util.ts.
            // Tooltip carries exact ms for precision.
            title={`${msg.toolDuration} ms`}>{formatDuration(msg.toolDuration)}</span>
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
                padding: '8px 10px', background: C.bgInput, borderRadius: T.radii.md,
                fontFamily: T.typography.fontMono,
                fontSize: '11.5px', color: C.textSecondary,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0,
              }}>{msg.toolInput}</pre>
            </div>
          )}
          {msg.toolOutput && (
            <div style={{ position: 'relative' }}>
              <div style={{ fontSize: '10px', color: C.textMuted, fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Output</div>
              <pre style={{
                padding: '8px 10px', background: C.bgInput, borderRadius: T.radii.md,
                fontFamily: T.typography.fontMono,
                fontSize: '11.5px', color: C.textSecondary,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word', margin: 0,
                maxHeight: '300px', overflowY: 'auto',
              }}>{msg.toolOutput}</pre>
              {/* c2-359 / task 66: copy button over the output pane.
                  position:absolute so it floats in the pre's top-right
                  without displacing content. Uses the shared CopyButton
                  so the 2s checkmark flash is consistent with the other
                  copy affordances in the app. */}
              <div style={{ position: 'absolute', top: '20px', right: '8px', zIndex: 1 }}>
                <CopyButton C={C} size={26} iconSize={14}
                  title='Copy output'
                  onCopy={() => navigator.clipboard?.writeText(msg.toolOutput || '')} />
              </div>
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
  // c2-299: optional copy-to-clipboard hook. When provided, a copy button
  // appears next to the edit button on hover / always on mobile. Parent
  // owns the clipboard path so toast + permission handling stays in App.
  onCopy?: (text: string) => void;
  // c2-400 / task 185: right-click hook. Parent opens a floating menu at
  // the event coordinates with role-appropriate actions (copy / edit /
  // fork). Absent → browser's native context menu fires as normal.
  onContextMenu?: (e: React.MouseEvent) => void;
}

// c2-406 / task 216: long-content collapse. Messages over COLLAPSE_AT
// chars render a preview slice with a Show more toggle so a pasted log
// doesn't blow the scroll position. State is local — re-mount (Virtuoso
// recycle) resets to collapsed, which matches user intent.
const COLLAPSE_AT = 4000;
const COLLAPSE_PREVIEW = 1500;

// User message bubble with inline edit flow. Parent owns the editing state
// so cross-message coordination (only one editor open at a time) stays simple.
export const UserMessage: React.FC<UserMessageProps> = ({
  msg, C, isMobile, maxWidth, editing, editText, setEditText,
  onBeginEdit, onCancelEdit, onCommitEdit, formatTime, onCopy, onContextMenu,
}) => {
  const [expanded, setExpanded] = React.useState(false);
  const needsCollapse = !editing && msg.content.length > COLLAPSE_AT && !expanded;
  const shown = needsCollapse ? msg.content.slice(0, COLLAPSE_PREVIEW) : msg.content;
  // c2-433 / task 223: long-press shim — only attach handlers when not editing
  // (the editor uses its own touch behavior) and when the parent provided a
  // context-menu callback.
  const longPress = useLongPress(!editing ? onContextMenu : undefined);
  return (
  <div
    onMouseEnter={(e) => { const bar = e.currentTarget.querySelector('.user-msg-actions') as HTMLElement; if (bar) bar.style.opacity = '1'; }}
    onMouseLeave={(e) => { const bar = e.currentTarget.querySelector('.user-msg-actions') as HTMLElement; if (bar) bar.style.opacity = '0'; }}
    onContextMenu={onContextMenu}
    {...longPress}
    style={{ display: 'flex', justifyContent: 'flex-end', gap: '6px', alignItems: 'flex-end' }}>
    {!editing && (
      <div className='user-msg-actions'
        style={{
          display: 'flex', gap: '2px', flexShrink: 0,
          opacity: isMobile ? 1 : 0, transition: 'opacity 0.12s',
        }}>
        {onCopy && (
          <CopyButton C={C} size={28}
            title='Copy (Shift-click: plain text)'
            onCopy={(e) => onCopy(e.shiftKey ? stripMarkdown(msg.content) : msg.content)} />
        )}
        <button onClick={onBeginEdit}
          title='Edit and resend'
          aria-label='Edit message and resend'
          style={{
            width: '28px', height: '28px',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            background: 'transparent', border: 'none',
            color: C.textMuted, cursor: 'pointer', borderRadius: T.radii.md,
          }}
          onMouseEnter={(e) => { e.currentTarget.style.background = C.bgHover; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
          </svg>
        </button>
      </div>
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
              border: `1px solid ${C.border}`, color: C.textMuted, borderRadius: T.radii.md,
              cursor: 'pointer', fontFamily: 'inherit' }}>Cancel</button>
          <button onClick={() => {
            const trimmed = editText.trim();
            if (!trimmed) return;
            onCommitEdit(trimmed);
          }}
            style={{ padding: '6px 14px', fontSize: '12px',
              background: C.accent, border: 'none', color: '#fff',
              borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit', fontWeight: 600 }}>
            Send
          </button>
        </div>
      </div>
    ) : (
      <div dir='auto' style={{
        maxWidth, padding: `${T.spacing.md} ${T.spacing.lg}`,
        // c0-019/020: user bubble uses muted accent-bg (not saturated accent),
        // with body-color text for readability. Professional feel over
        // "branded message balloon."
        // c2-270: dir=auto so RTL content (Arabic/Hebrew) flips text direction
        // per-bubble without affecting siblings.
        background: C.accentBg, color: C.text,
        borderRadius: `${T.radii.lg} ${T.radii.lg} ${T.radii.xs} ${T.radii.lg}`,
        fontSize: T.typography.sizeBody, lineHeight: T.typography.lineLoose,
        wordBreak: 'break-word',
        border: `1px solid ${C.accentBorder}`,
      }}>
        {shown}
        {needsCollapse && <span style={{ color: C.textMuted }}>{'\u2026'}</span>}
        {msg.content.length > COLLAPSE_AT && (
          <button onClick={() => setExpanded(v => !v)}
            style={{
              display: 'block', marginTop: T.spacing.xs,
              background: 'transparent', border: 'none',
              color: C.accent, cursor: 'pointer', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
              padding: 0, textAlign: 'left',
            }}>
            {expanded ? 'Show less' : `Show more (${msg.content.length.toLocaleString()} chars)`}
          </button>
        )}
        <div title={new Date(msg.timestamp).toLocaleString()}
          style={{ fontSize: '10px', color: C.textMuted, marginTop: '6px', textAlign: 'right' }}>
          {formatTime(msg.timestamp)}
        </div>
      </div>
    )}
  </div>
  );
};

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
  // c2-393 / task 205: timestamp of the user turn that triggered this
  // response. When present, renders a "3.4s" duration chip next to the
  // relative age so users see response latency at a glance.
  respondToTs?: number;
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
  // c2-433 / #350: third feedback affordance — "correct this." Opens a
  // textarea modal so the user can paste what the AI *should* have said.
  // Optional so existing call sites that haven't wired it yet still
  // typecheck; when omitted the button is hidden.
  onFeedbackCorrect?: () => void;
  formatTime: (ts: number) => string;
  // c2-400 / task 185: right-click hook. Same shape as UserMessage's.
  onContextMenu?: (e: React.MouseEvent) => void;
}

// Assistant message — the heaviest bubble. Hover-revealed action bar with copy,
// regenerate (last turn only), thumbs up/down, timestamp. Follow-up suggestion
// chips and the reasoning toggle are rendered below when applicable.
export const AssistantMessage: React.FC<AssistantMessageProps> = ({
  msg, C, isMobile, isDesktop, isLast, isThinking,
  showReasoning, developerMode, reasoningExpanded,
  renderBody, onToggleReasoning, onRegenerate, onCopy,
  onOpenProvenance, onFollowUpChip, onFeedbackPositive, onFeedbackNegative, onFeedbackCorrect, formatTime, respondToTs,
  onContextMenu,
}) => {
  // c2-406 / task 216: same collapse logic as UserMessage. Slicing
  // markdown mid-fence is safe because the renderer treats unclosed
  // fences as literal text — worst case the preview ends in an
  // unclosed ``` which renders as a flat fence until the user expands.
  const [expanded, setExpanded] = React.useState(false);
  const needsCollapse = msg.content.length > COLLAPSE_AT && !expanded;
  const bodyText = needsCollapse ? msg.content.slice(0, COLLAPSE_PREVIEW) : msg.content;
  // c2-433 / #359 + claude-0 #400 ship: refusal detection. After backend
  // de-hardcoded the conversational response pools, Pulse responses now
  // emit an explicit refusal string when retrieval doesn't clear
  // threshold: "No HDC match in knowledge base for X — I won't fabricate".
  // That + the legacy heuristics (I don't know / I can't answer) all flip
  // the yellow left-border + REFUSAL pill. Conservative: single-paragraph
  // only so a long answer that happens to contain "cannot" isn't flagged.
  const refusalMatch = (() => {
    const first = msg.content.trim().slice(0, 400);
    const multiline = first.split('\n').filter(l => l.trim()).length > 3;
    if (multiline) return null;
    // Claude-0's explicit refusal from #400: "No HDC match in knowledge base
    // for <subject> — I won't fabricate". Capture the subject as the reason.
    const hdc = first.match(/^no\s+hdc\s+match(?:\s+in\s+(?:knowledge\s+base|kb))?(?:\s+for\s+(.+?))?\s*[-—–:]?\s*(?:i\s+won'?t\s+fabricate)?\s*[.!?]?$/i);
    if (hdc) {
      const subject = hdc[1]?.trim().replace(/[.!?]+$/, '');
      return { reason: subject ? `no substrate match for "${subject}"` : "won't fabricate" };
    }
    // Legacy heuristics: explicit I-don't-know openers.
    const generic = first.match(/^(?:i\s+don'?t\s+know|i\s+(?:can'?t|cannot|am\s+unable\s+to)\s+(?:answer|say|confirm|verify)|refusing\s+to\s+answer)(?:\s+because\s+(.+?)[.!?]?$|[.!?]?$)/i);
    if (generic) return { reason: generic[1]?.trim() || null };
    return null;
  })();
  // Follow-up chips — simple keyword extraction, only on last assistant message
  // when the body is long enough to have meaningful topics.
  // c2-433 / task 223: same long-press shim as UserMessage so phone users can
  // reach the context menu (copy / edit / branch / re-ask). Vibration tick on
  // open feels native vs the silent appearance of the menu.
  const longPress = useLongPress(onContextMenu);
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
      onContextMenu={onContextMenu}
      {...longPress}
      style={{ display: 'flex', justifyContent: 'flex-start' }}>
      <div style={{ maxWidth: isDesktop ? '80%' : '96%', width: '100%' }}>
        {/* Response body */}
        <div dir='auto' style={{
          padding: '14px 18px',
          background: C.bgCard,
          border: `1px solid ${C.border}`,
          borderLeft: refusalMatch ? `3px solid ${C.yellow}` : `1px solid ${C.border}`,
          borderRadius: `${T.radii.xs} ${T.radii.lg} ${T.radii.lg} ${T.radii.lg}`,
          fontSize: '14px', lineHeight: '1.7',
          color: C.text,
          whiteSpace: 'pre-wrap', wordBreak: 'break-word',
        }}>
          {refusalMatch && (
            <div style={{
              display: 'inline-flex', alignItems: 'center', gap: '6px',
              marginRight: '8px', marginBottom: '4px',
              padding: '2px 8px', borderRadius: T.radii.sm,
              background: `${C.yellow}18`, color: C.yellow,
              fontSize: '10px', fontWeight: 800,
              fontFamily: T.typography.fontMono,
              letterSpacing: '0.04em', textTransform: 'uppercase',
              verticalAlign: 'middle',
            }}
              title={refusalMatch.reason
                ? `Refusal — reason: ${refusalMatch.reason}`
                : 'Refusal — the system declined to answer'}>
              <svg width='10' height='10' viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='2.4' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true'>
                <path d='M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z' />
                <line x1='12' y1='9' x2='12' y2='13' />
                <line x1='12' y1='17' x2='12.01' y2='17' />
              </svg>
              <span>refusal</span>
            </div>
          )}
          {renderBody(bodyText)}
          {refusalMatch && isLast && onFeedbackCorrect && (
            // claude-0 #400 ship: refusals now mean "no substrate match" —
            // so the right next action for the user is TEACHING LFI the
            // answer. Inline CTA chip makes the improvement loop
            // one click away instead of hidden in the hover action bar.
            <div style={{
              marginTop: T.spacing.md,
              padding: '10px 12px',
              background: `${C.yellow}10`,
              border: `1px dashed ${C.yellow}60`,
              borderRadius: T.radii.md,
              fontSize: T.typography.sizeSm,
              color: C.text,
              display: 'flex', alignItems: 'center', gap: T.spacing.sm, flexWrap: 'wrap',
            }}>
              <span style={{ flex: 1, minWidth: '180px', lineHeight: 1.5, color: C.textMuted }}>
                LFI doesn't know the answer. Teach it so it does next time.
              </span>
              <button onClick={onFeedbackCorrect}
                style={{
                  padding: '6px 14px',
                  background: C.yellow, color: C.bg,
                  border: 'none', borderRadius: T.radii.sm,
                  fontSize: T.typography.sizeSm,
                  fontWeight: T.typography.weightBold,
                  cursor: 'pointer', fontFamily: 'inherit',
                  whiteSpace: 'nowrap',
                }}>
                Teach LFI
              </button>
            </div>
          )}
          {msg.content.length > COLLAPSE_AT && (
            <button onClick={() => setExpanded(v => !v)}
              style={{
                display: 'block', marginTop: T.spacing.sm,
                background: 'transparent', border: 'none',
                color: C.accent, cursor: 'pointer', fontFamily: 'inherit',
                fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                padding: 0, textAlign: 'left',
              }}>
              {expanded ? 'Show less' : `Show more (${msg.content.length.toLocaleString()} chars)`}
            </button>
          )}
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
                borderRadius: T.radii.sm, color: C.textDim,
                cursor: 'pointer', fontFamily: T.typography.fontMono,
              }}>
              #{msg.conclusion_id}
            </span>
          )}
        </div>

        {/* Hover-revealed action bar.
            c2-433 mobile compaction: button shrink 30→28, gap 4→3, no-wrap
            with overflowX scroll so the new "correct" button + confidence
            chip + duration chip don't push each other onto a 2nd line.
            Confidence becomes icon-only on mobile (just %, not "% confident")
            and the words/tokens line below is hidden — that detail lives
            in the long-press context menu now. */}
        <div className='lfi-msg-actions'
          style={{
            display: 'flex', gap: isMobile ? '3px' : '4px', marginTop: '4px',
            justifyContent: 'flex-end', alignItems: 'center',
            flexWrap: 'nowrap', overflowX: 'auto', scrollbarWidth: 'none',
            opacity: isMobile ? 1 : 0,
            transition: 'opacity 0.15s',
          }}>
          <CopyButton C={C} size={isMobile ? 28 : 30}
            title='Copy markdown (Shift-click: plain text)'
            onCopy={(e) => {
              // Shift-click -> copy as plain text (strips markdown syntax).
              // Default click -> copy raw markdown source.
              const text = e.shiftKey ? stripMarkdown(msg.content) : msg.content;
              onCopy(text);
            }} />
          {isLast && (
            <button onClick={onRegenerate} title='Regenerate' aria-label='Regenerate last response'
              disabled={isThinking}
              style={{
                width: isMobile ? '28px' : '30px', height: isMobile ? '28px' : '30px',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                background: 'transparent', border: 'none',
                color: C.textMuted, borderRadius: T.radii.md,
                cursor: isThinking ? 'wait' : 'pointer', fontFamily: 'inherit',
                flexShrink: 0,
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
              width: isMobile ? '28px' : '30px', height: isMobile ? '28px' : '30px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'transparent', border: 'none',
              color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer',
              flexShrink: 0,
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
              width: isMobile ? '28px' : '30px', height: isMobile ? '28px' : '30px',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'transparent', border: 'none',
              color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer',
              flexShrink: 0,
            }}
            onMouseEnter={(e) => { e.currentTarget.style.color = C.red; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = C.textMuted; }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17"/>
            </svg>
          </button>
          {/* c2-433 / #350: "Correct this" — opens a textarea so the user can
              teach the system the right answer. Distinct from thumbs-down
              (which is "this is wrong") because it carries the *correction*,
              not just the rating. Backend stores rating='correct' + correction
              text; Classroom feedback queue surfaces these for ingestion. */}
          {onFeedbackCorrect != null && (
            <button onClick={onFeedbackCorrect}
              title='Correct this — teach the system the right answer'
              aria-label='Submit a correction'
              style={{
                width: isMobile ? '28px' : '30px', height: isMobile ? '28px' : '30px',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                background: 'transparent', border: 'none',
                color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer',
                flexShrink: 0,
              }}
              onMouseEnter={(e) => { e.currentTarget.style.color = C.accent; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = C.textMuted; }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 20h9"/>
                <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
              </svg>
            </button>
          )}
          {typeof msg.confidence === 'number' && (() => {
            // Confidence badge next to the action bar. Green ≥0.75, yellow 0.5-0.75,
            // amber-ish below. Gives users a cue when the AI is low-confidence so
            // they calibrate trust (addresses c0-008 bug #5 — quality indicator).
            // c2-433 mobile: shorten label "82% confident" → "82%" to save room.
            const pct = Math.round(msg.confidence * 100);
            const color = msg.confidence >= 0.75 ? C.green : msg.confidence >= 0.5 ? C.yellow : C.red;
            const bg = msg.confidence >= 0.75 ? C.greenBg : msg.confidence >= 0.5 ? C.accentBg : C.redBg;
            return (
              <span
                title={`Model confidence on this response: ${pct}%`}
                style={{
                  fontSize: '10px', fontWeight: 700, color, background: bg,
                  padding: isMobile ? '2px 6px' : '2px 8px', borderRadius: T.radii.sm, alignSelf: 'center',
                  fontFamily: T.typography.fontMono, flexShrink: 0,
                }}>
                {isMobile ? `${pct}%` : `${pct}% confident`}
              </span>
            );
          })()}
          {/* c2-358 / task 63: show relative age (2m ago) for at-a-glance recency;
              keep the absolute timestamp in the title for precise inspection. */}
          <span title={new Date(msg.timestamp).toLocaleString()}
            style={{
              fontSize: '10px', color: C.textDim, alignSelf: 'center',
              padding: isMobile ? '0 4px' : '0 8px', flexShrink: 0,
            }}>{formatRelative(msg.timestamp)}</span>
          {/* c2-393 / task 205: response-duration chip. Only when we have
              a user-turn timestamp to diff against and the gap is sane
              (< 1 hour — larger means session boundaries, not latency).
              c2-433 mobile: hidden on mobile to free row space — duration
              still inspectable via long-press / context menu. */}
          {!isMobile && typeof respondToTs === 'number' && msg.timestamp > respondToTs && (msg.timestamp - respondToTs) < 3_600_000 && (() => {
            const ms = msg.timestamp - respondToTs;
            const label = formatDuration(ms);
            return (
              <span title={`Response took ${label}`}
                style={{
                  fontSize: '10px', color: C.textDim, alignSelf: 'center',
                  padding: '0 8px 0 0', fontFamily: T.typography.fontMono, flexShrink: 0,
                }}>{label}</span>
            );
          })()}
          {/* c2-433 / #357 + Tier 5 #38 provenance-lite: count distinct
              facts + distinct sources surfaced in the response text.
              Desktop-only — mobile users see the same info via each
              individual [fact:KEY] chip. Provides an at-a-glance trust
              signal (5 facts · 3 sources = well-grounded vs 1 fact · 1
              source = leaning hard on one claim). Hidden when no
              citations present. */}
          {!isMobile && !isThinking && msg.content && (() => {
            const factRegex = /\[(?:fact|k):([A-Za-z0-9_\-:]{1,80})\]/g;
            const factKeys = new Set<string>();
            let m: RegExpExecArray | null;
            while ((m = factRegex.exec(msg.content)) !== null) factKeys.add(m[1]);
            if (factKeys.size === 0) return null;
            const sourceRegex = /[\(\[]source:\s*([^,\)\]]+),\s*similarity/gi;
            const sources = new Set<string>();
            while ((m = sourceRegex.exec(msg.content)) !== null) sources.add(m[1].trim().toLowerCase());
            return (
              <span title={sources.size > 0
                ? `${factKeys.size} cited fact${factKeys.size === 1 ? '' : 's'} from ${sources.size} distinct source${sources.size === 1 ? '' : 's'} — click any chip inline for ancestry`
                : `${factKeys.size} cited fact${factKeys.size === 1 ? '' : 's'} — click any chip inline for ancestry`}
                style={{
                  fontSize: '10px', color: C.textDim, alignSelf: 'center',
                  padding: '0 8px 0 0', fontFamily: T.typography.fontMono, flexShrink: 0,
                }}>
                {factKeys.size}f{sources.size > 0 ? ` · ${sources.size}s` : ''}
              </span>
            );
          })()}
        </div>
        {/* c2-360 / task 88 / c2-433 #339 vocab sweep: word + char counts on
            assistant messages. Hidden on empty / thinking messages so it
            doesn't flash during streaming. The "tokens" estimate that lived
            here was an LLM concept — replaced with raw char count which is
            the post-LLM-honest unit. Hidden on mobile — accessible via long-
            press menu. */}
        {!isMobile && !isThinking && msg.content && msg.content.trim().length > 0 && (() => {
          const words = msg.content.trim().split(/\s+/).length;
          return (
            <div style={{
              fontSize: T.typography.sizeXs, color: C.textDim,
              padding: '0 8px', marginTop: '2px',
              fontFamily: T.typography.fontMono,
            }}>
              {words.toLocaleString()} {words === 1 ? 'word' : 'words'} · {msg.content.length.toLocaleString()} chars
            </div>
          );
        })()}
        {/* Last-message helpfulness nudge — only on the latest assistant reply,
            fades to invisibility once user votes (tracked via onFeedbackPositive/
            onFeedbackNegative side effects at the parent). Addresses c0-008 #5.
            c2-433 mobile: hidden on mobile — the action bar above with three
            buttons is self-explanatory and the nudge text is clutter on phones. */}
        {!isMobile && isLast && !isThinking && (
          <div style={{
            fontSize: '11px', color: C.textDim, textAlign: 'right',
            marginTop: '4px', paddingRight: '4px',
            fontStyle: 'italic',
          }}>Was this helpful? Use 👍 / 👎 / ✏️ above.</div>
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
                  color: C.textSecondary, borderRadius: T.radii.pill,
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
                border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md,
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
                <ReasoningSteps C={C} steps={msg.reasoning} />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

// Collapsible per-step reasoning list. Each step is a disclosure that
// shows the first 80 chars by default; click to expand the full text.
const ReasoningSteps: React.FC<{ C: any; steps: string[] }> = ({ C, steps }) => {
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const toggle = (i: number) => setExpanded(prev => {
    const next = new Set(prev);
    if (next.has(i)) next.delete(i); else next.add(i);
    return next;
  });
  const shortOf = (s: string) => s.length > 80 ? s.slice(0, 80).trimEnd() + '…' : s;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
      {steps.map((step, j) => {
        const isOpen = expanded.has(j);
        const needsToggle = step.length > 80;
        return (
          <div key={j} style={{
            borderBottom: j < steps.length - 1 ? `1px solid ${C.borderSubtle}` : 'none',
            padding: '4px 0',
          }}>
            <button
              onClick={() => needsToggle && toggle(j)}
              aria-expanded={needsToggle ? isOpen : undefined}
              aria-label={needsToggle ? `Step ${j} ${isOpen ? 'collapse' : 'expand'}` : undefined}
              style={{
                display: 'flex', alignItems: 'flex-start', gap: '8px', width: '100%',
                background: 'transparent', border: 'none', padding: 0,
                cursor: needsToggle ? 'pointer' : 'default', textAlign: 'left',
                fontFamily: 'inherit', color: C.textSecondary,
              }}>
              <span style={{ color: C.accent, fontWeight: 700, fontSize: '11px', minWidth: '22px', lineHeight: '1.6' }}>[{j}]</span>
              <span style={{ fontSize: '12px', lineHeight: '1.6', flex: 1, whiteSpace: isOpen ? 'pre-wrap' : 'normal' }}>
                {isOpen ? step : shortOf(step)}
              </span>
              {needsToggle && (
                <span style={{ color: C.textDim, fontSize: '10px', flexShrink: 0, lineHeight: '1.6' }}>
                  {isOpen ? '▴' : '▾'}
                </span>
              )}
            </button>
          </div>
        );
      })}
    </div>
  );
};

export const WebMessage: React.FC<{ content: string; C: any; isDesktop: boolean }> = React.memo(({ content, C, isDesktop }) => (
  <div style={{
    padding: '14px 16px', borderRadius: T.radii.xxl,
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
