import React, { useRef, useImperativeHandle, forwardRef } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';

// Thin virtualized message list. Parent keeps its handler/state soup —
// ChatView just wires Virtuoso and asks the parent to render each message
// via the `renderMessage` prop. When messages.length===0, renderEmpty runs
// in a regular scroll container (Virtuoso skipped).
//
// Scales to 10k+ messages without re-rendering the whole list on state
// changes elsewhere in App.tsx.
export interface ChatViewHandle {
  scrollToBottom: () => void;
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
  WebkitOverflowScrolling?: 'touch' | 'auto';
}

function ChatViewInner<T extends { id: number | string }>(
  { messages, renderMessage, renderEmpty, renderFooter, chatMaxWidth, chatPadding, isDesktop, onAtBottomChange }: ChatViewProps<T>,
  ref: React.ForwardedRef<ChatViewHandle>,
) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  useImperativeHandle(ref, () => ({
    scrollToBottom: () => {
      if (messages.length > 0) {
        virtuosoRef.current?.scrollToIndex({ index: messages.length - 1, align: 'end', behavior: 'smooth' });
      }
    },
  }), [messages.length]);
  if (messages.length === 0) {
    return (
      <div style={{ flex: 1, overflowY: 'auto', padding: chatPadding, WebkitOverflowScrolling: 'touch' as any }}>
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
      style={{ flex: 1 }}
      data={messages}
      // Follow only when at-bottom — but be generous about what counts as
      // at-bottom (50px instead of the 4px default), so streaming chunks
      // don't fight the user when they nudge the scroll a few pixels up.
      // BUG-FIX 2026-04-17 c0-008: user reported "scroll is jumpy/wonky".
      // Function form returns 'smooth' only when at bottom; otherwise false
      // so Virtuoso stops trying to follow. This also fixes the case where
      // streaming chat_chunk updates the last message in place — content
      // grows but message count doesn't, and followOutput now fires anyway.
      followOutput={(isAtBottom) => isAtBottom ? 'smooth' : false}
      atBottomThreshold={50}
      // On initial mount + conversation switch, render the last message.
      initialTopMostItemIndex={messages.length > 0 ? messages.length - 1 : 0}
      atBottomStateChange={onAtBottomChange}
      computeItemKey={(_i, m) => String(m.id)}
      // Increase overscan so heights are pre-computed off-screen — reduces
      // mid-scroll height-correction jumps when the user scrolls fast.
      increaseViewportBy={{ top: 400, bottom: 400 }}
      components={{
        Header: () => <div style={{ height: isDesktop ? '24px' : '12px' }} />,
        Footer: () => (
          <div style={{ maxWidth: chatMaxWidth, margin: '0 auto', padding: `0 ${chatPadding.split(' ')[1] || '16px'}` }}>
            {renderFooter?.()}
            <div style={{ height: isDesktop ? '24px' : '12px' }} />
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
