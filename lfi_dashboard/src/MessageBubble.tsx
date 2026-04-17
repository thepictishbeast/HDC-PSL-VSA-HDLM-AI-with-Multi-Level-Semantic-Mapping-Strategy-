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
