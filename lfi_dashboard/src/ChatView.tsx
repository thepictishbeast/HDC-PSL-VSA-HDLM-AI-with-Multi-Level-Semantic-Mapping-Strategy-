import React from 'react';
import { Virtuoso } from 'react-virtuoso';

// Thin virtualized message list. Parent keeps its handler/state soup —
// ChatView just wires Virtuoso and asks the parent to render each message
// via the `renderMessage` prop. When messages.length===0, renderEmpty runs
// in a regular scroll container (Virtuoso skipped).
//
// Scales to 10k+ messages without re-rendering the whole list on state
// changes elsewhere in App.tsx.
export interface ChatViewProps<T extends { id: number | string }> {
  messages: T[];
  renderMessage: (msg: T, index: number) => React.ReactNode;
  renderEmpty?: () => React.ReactNode;
  renderFooter?: () => React.ReactNode;
  chatMaxWidth: string;
  chatPadding: string;
  isDesktop: boolean;
  WebkitOverflowScrolling?: 'touch' | 'auto';
}

export function ChatView<T extends { id: number | string }>({
  messages, renderMessage, renderEmpty, renderFooter,
  chatMaxWidth, chatPadding, isDesktop,
}: ChatViewProps<T>) {
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
      style={{ flex: 1 }}
      data={messages}
      followOutput='smooth'
      computeItemKey={(_i, m) => String(m.id)}
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
          marginBottom: isDesktop ? '20px' : '14px',
        }}>
          {renderMessage(msg, index)}
        </div>
      )}
    />
  );
}
