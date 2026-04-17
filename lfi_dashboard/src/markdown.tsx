import React from 'react';
import hljs from 'highlight.js/lib/core';

// Lightweight markdown renderer — bold/italic/code/links/lists/fenced code.
// Previously inlined in App.tsx; extracted so the rest of the tree doesn't
// close over the theme key + copyToClipboard just to format a message body.

export interface MarkdownCtx {
  C: any;
  themeKey: string;     // settings.theme — used to pick light/dark fenced-code background
  onCopy?: (text: string) => void;
  onCopyEvent?: (lang: string, length: number) => void;
}

export const renderInlineMd = (raw: string, baseKey: string, ctx: MarkdownCtx): React.ReactNode[] => {
  const { C, themeKey } = ctx;
  const out: React.ReactNode[] = [];
  // Priority 1: inline code. Split on backticks first so its contents render verbatim.
  const codeParts = raw.split(/(`[^`\n]+`)/g);
  codeParts.forEach((seg, i) => {
    if (seg.startsWith('`') && seg.endsWith('`') && seg.length >= 2) {
      out.push(
        <code key={`${baseKey}-c${i}`} style={{
          padding: '1px 6px', borderRadius: '4px',
          background: themeKey === 'light' ? 'rgba(20,30,60,0.06)' : 'rgba(255,255,255,0.08)',
          fontFamily: C.font, fontSize: '0.92em',
        }}>{seg.slice(1, -1)}</code>
      );
      return;
    }
    const tokens = seg.split(/(\*\*[^*\n]+\*\*|\*[^*\n]+\*)/g);
    tokens.forEach((tok, j) => {
      if (tok.startsWith('**') && tok.endsWith('**') && tok.length >= 4) {
        out.push(<strong key={`${baseKey}-b${i}-${j}`}>{tok.slice(2, -2)}</strong>);
      } else if (tok.startsWith('*') && tok.endsWith('*') && tok.length >= 2) {
        out.push(<em key={`${baseKey}-i${i}-${j}`}>{tok.slice(1, -1)}</em>);
      } else if (tok) {
        const linkParts = tok.split(/(\[[^\]]+\]\([^)]+\))/g);
        linkParts.forEach((lp, k) => {
          const linkMatch = lp.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
          if (linkMatch) {
            out.push(<a key={`${baseKey}-l${i}-${j}-${k}`} href={linkMatch[2]}
              target="_blank" rel="noopener noreferrer"
              style={{ color: C.accent, textDecoration: 'underline' }}
            >{linkMatch[1]}</a>);
          } else if (lp) {
            out.push(<span key={`${baseKey}-t${i}-${j}-${k}`}>{lp}</span>);
          }
        });
      }
    });
  });
  return out;
};

export const renderMessageBody = (text: string, ctx: MarkdownCtx): React.ReactNode[] => {
  const { C, themeKey, onCopy, onCopyEvent } = ctx;
  const parts: React.ReactNode[] = [];
  const fence = /```([a-zA-Z0-9_+-]*)\n([\s\S]*?)```/g;
  let lastIndex = 0; let match: RegExpExecArray | null; let key = 0;
  while ((match = fence.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const before = text.slice(lastIndex, match.index);
      parts.push(<span key={`t${key++}`}>{renderInlineMd(before, `pre${key}`, ctx)}</span>);
    }
    const lang = match[1] || 'text';
    const code = match[2];
    parts.push(
      <div key={`c${key++}`} style={{
        margin: '10px 0', border: `1px solid ${C.borderSubtle}`, borderRadius: '8px',
        background: themeKey === 'light' ? '#f8fafd' : '#0a0b13', overflow: 'hidden',
      }}>
        <div style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          padding: '6px 10px', borderBottom: `1px solid ${C.borderSubtle}`,
          fontSize: '10px', color: C.textDim, textTransform: 'uppercase', letterSpacing: '0.08em',
        }}>
          <span>{lang}</span>
          <button onClick={() => { onCopy?.(code); onCopyEvent?.(lang, code.length); }}
            style={{
              background: 'transparent', border: 'none', color: C.textMuted,
              cursor: 'pointer', fontSize: '10px', textTransform: 'uppercase', letterSpacing: '0.08em',
            }}>Copy</button>
        </div>
        <pre style={{
          margin: 0, padding: '12px 14px',
          fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '12.5px', lineHeight: '1.55',
          color: C.text, whiteSpace: 'pre', overflowX: 'auto',
        }} dangerouslySetInnerHTML={{
          __html: (() => {
            try {
              if (lang && hljs.getLanguage(lang)) {
                return hljs.highlight(code, { language: lang }).value;
              }
              return hljs.highlightAuto(code).value;
            } catch { return code.replace(/</g, '&lt;').replace(/>/g, '&gt;'); }
          })()
        }} />
      </div>
    );
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    const tail = text.slice(lastIndex);
    const listLines = tail.split('\n');
    let currentList: string[] = [];
    let listType: 'ul' | 'ol' | null = null;
    const flushList = () => {
      if (currentList.length > 0 && listType) {
        const Tag = listType;
        parts.push(
          <Tag key={`list${key++}`} style={{ margin: '8px 0', paddingLeft: '24px' }}>
            {currentList.map((item, li) => (
              <li key={li} style={{ marginBottom: '4px' }}>{renderInlineMd(item, `li${key}-${li}`, ctx)}</li>
            ))}
          </Tag>
        );
        currentList = [];
        listType = null;
      }
    };
    listLines.forEach((line) => {
      const bulletMatch = line.match(/^\s*[-*]\s+(.+)/);
      const numMatch = line.match(/^\s*\d+\.\s+(.+)/);
      if (bulletMatch) {
        if (listType === 'ol') flushList();
        listType = 'ul';
        currentList.push(bulletMatch[1]);
      } else if (numMatch) {
        if (listType === 'ul') flushList();
        listType = 'ol';
        currentList.push(numMatch[1]);
      } else {
        flushList();
        if (line.trim()) {
          parts.push(<span key={`t${key++}`}>{renderInlineMd(line, `post${key}`, ctx)}{'\n'}</span>);
        } else {
          parts.push(<br key={`br${key++}`} />);
        }
      }
    });
    flushList();
  }
  return parts.length > 0 ? parts : [<span key='empty'>{renderInlineMd(text, 'solo', ctx)}</span>];
};
