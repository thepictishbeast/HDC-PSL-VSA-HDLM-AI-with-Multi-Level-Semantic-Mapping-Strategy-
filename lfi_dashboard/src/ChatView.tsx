import React, { useRef, useImperativeHandle, forwardRef } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';
import { T } from './tokens';

// c2-240 / #20: header/footer spacer + fallback horizontal padding now via
// tokens. The 28/18 inter-message margin stays literal — those values were
// design-reviewed for "bubble breathing room" and don't map cleanly to the
// 4/8 grid.

// Thin virtualized message list. Parent keeps its handler/state soup —
// ChatView just wires Virtuoso and asks the parent to render each message
// via the `renderMessage` prop. When messages.length===0, renderEmpty runs
// in a regular scroll container (Virtuoso skipped).
//
// Scales to 10k+ messages without re-rendering the whole list on state
// changes elsewhere in App.tsx.
export interface ChatViewHandle {
  scrollToBottom: () => void;
  // c2-256 / #118: needed for chat-search match navigation. align='center'
  // puts the match in the middle of the viewport rather than clipping at
  // the top edge.
  scrollToIndex: (index: number) => void;
}

export interface ChatViewProps<T extends { id: number | string }> {
  messages: T[];
  renderMessage: (msg: T, index: number) => React.ReactNode;
  renderEmpty?: () => React.ReactNode;
  renderFooter?: () => React.ReactNode;
  chatMaxWidth: string;
  chatPadding: string;
  isDesktop: boolean;
  // Notified whenever the at-bottom state changes. Parent uses this to render
  // a floating "scroll to bottom" affordance when the user has scrolled up.
  onAtBottomChange?: (atBottom: boolean) => void;
  // Reports the current topmost-visible item index so a parent can render
  // a sticky "day header" outside Virtuoso's absolutely-positioned list.
  onVisibleRangeChange?: (startIdx: number, endIdx: number) => void;
  WebkitOverflowScrolling?: 'touch' | 'auto';
  // c2-353 / task 57: scroll animation for programmatic scrolls. 'smooth'
  // is the visible default; pass 'auto' when switching conversations to
  // land instantly on the last message (100+ message histories made the
  // smooth scroll visibly lag).
  scrollBehavior?: 'smooth' | 'auto';
  // c2-353 / task 58: theme palette needed for the thin scrollbar colors.
  // Optional so non-theme-aware callers still render.
  C?: any;
}

function ChatViewInner<T extends { id: number | string }>(
  { messages, renderMessage, renderEmpty, renderFooter, chatMaxWidth, chatPadding, isDesktop, onAtBottomChange, onVisibleRangeChange, scrollBehavior = 'smooth', C }: ChatViewProps<T>,
  ref: React.ForwardedRef<ChatViewHandle>,
) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  useImperativeHandle(ref, () => ({
    scrollToBottom: () => {
      if (messages.length > 0) {
        virtuosoRef.current?.scrollToIndex({ index: messages.length - 1, align: 'end', behavior: scrollBehavior });
      }
    },
    scrollToIndex: (index: number) => {
      if (messages.length === 0) return;
      const clamped = Math.max(0, Math.min(index, messages.length - 1));
      virtuosoRef.current?.scrollToIndex({ index: clamped, align: 'center', behavior: scrollBehavior });
    },
  }), [messages.length, scrollBehavior]);
  // c2-353 / task 58: thin styled scrollbar -- only renders when theme is
  // available; Firefox honors scrollbar-* directly, WebKit needs the CSS
  // vendor rules which we can't inline via the React style object, so
  // WebKit users keep the browser default (acceptable).
  const scrollbarStyle: React.CSSProperties = C ? {
    scrollbarWidth: 'thin' as any,
    scrollbarColor: `${C.borderSubtle} transparent`,
  } : {};
  if (messages.length === 0) {
    return (
      <div
        // c2-353 / task 60: aria-live so SR announces new messages arriving
        // while user is on the empty state (welcome screen).
        aria-live='polite'
        style={{ flex: 1, overflowY: 'auto', padding: chatPadding, WebkitOverflowScrolling: 'touch' as any, ...scrollbarStyle }}>
        <div style={{ maxWidth: chatMaxWidth, margin: '0 auto' }}>
          {renderEmpty?.()}
          {renderFooter?.()}
        </div>
      </div>
    );
  }
  return (
    <Virtuoso
      ref={virtuosoRef}
      // c2-353 / task 58+60: inline style merges theme-driven thin scrollbar
      // and aria-live on the outer Virtuoso wrapper so SR users get
      // announcements on incoming messages.
      style={{ flex: 1, ...scrollbarStyle }}
      // Virtuoso forwards its own role. aria-live must be applied to the
      // internal scroll container via scrollerProps rather than the root.
      scrollerProps={{ 'aria-live': 'polite', 'aria-relevant': 'additions' } as any}
      data={messages}
      // Follow only when at-bottom — but be generous about what counts as
      // at-bottom (80px instead of the 4px default), so streaming chunks
      // don't fight the user when they nudge the scroll a few pixels up.
      // BUG-FIX 2026-04-17 c0-008: user reported "scroll is jumpy/wonky".
      // Function form returns scrollBehavior only when at bottom; otherwise
      // false so Virtuoso stops trying to follow. This also fixes the case
      // where streaming chat_chunk updates the last message in place --
      // content grows but message count doesn't, and followOutput now fires.
      // c2-353 / task 56: bumped threshold from 50 -> 80 for even less
      // jitter when the user nudges scroll 50-80px up during streaming.
      followOutput={(isAtBottom) => isAtBottom ? scrollBehavior : false}
      atBottomThreshold={80}
      // On initial mount + conversation switch, render the last message.
      initialTopMostItemIndex={messages.length > 0 ? messages.length - 1 : 0}
      atBottomStateChange={onAtBottomChange}
      rangeChanged={onVisibleRangeChange ? (r) => onVisibleRangeChange(r.startIndex, r.endIndex) : undefined}
      computeItemKey={(_i, m) => String(m.id)}
      // Increase overscan so heights are pre-computed off-screen — reduces
      // mid-scroll height-correction jumps when the user scrolls fast.
      increaseViewportBy={{ top: 400, bottom: 400 }}
      components={{
        Header: () => <div style={{ height: isDesktop ? T.spacing.xl : T.spacing.md }} />,
        Footer: () => (
          <div style={{ maxWidth: chatMaxWidth, margin: '0 auto', padding: `0 ${chatPadding.split(' ')[1] || '16px'}` }}>
            {renderFooter?.()}
            <div style={{ height: isDesktop ? T.spacing.xl : T.spacing.md }} />
          </div>
        ),
      }}
      itemContent={(index, msg) => (
        <div style={{
          maxWidth: chatMaxWidth, margin: '0 auto',
          padding: `0 ${chatPadding.split(' ')[1] || '16px'}`,
          // c0-020: generous whitespace between messages. Bumped from 20/14
          // to 28/18 so bubbles breathe instead of stacking.
          marginBottom: isDesktop ? '28px' : '18px',
        }}>
          {renderMessage(msg, index)}
        </div>
      )}
    />
  );
}

// forwardRef wrapper preserves the generic. Cast to keep the inferred type.
export const ChatView = forwardRef(ChatViewInner) as <T extends { id: number | string }>(
  props: ChatViewProps<T> & { ref?: React.ForwardedRef<ChatViewHandle> },
) => ReturnType<typeof ChatViewInner>;
