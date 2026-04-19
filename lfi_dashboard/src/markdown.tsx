import React from 'react';
import hljs from 'highlight.js/lib/core';
import { canonicalLang, ensureLanguage } from './hljsLazy';
import { T } from './tokens';

// c2-240 / #20: padding/radii/font weights on tokens where they straightforwardly
// map. Font-size literals inside renderMessageBody (0.92em on inline code,
// 12.5px on code block) stay literal — they're tuned relative to the caller's
// font size, not part of the generic scale.

// Lightweight markdown renderer — bold/italic/code/links/lists/fenced code.
// Previously inlined in App.tsx; extracted so the rest of the tree doesn't
// close over the theme key + copyToClipboard just to format a message body.

// Minimal HTML-escape for when a code block is shown before its grammar
// loads (or when no grammar exists for the tag). Keeps angle brackets +
// ampersands from breaking out of <pre>.
const escapeHtml = (s: string): string =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

// Code block component. Renders escaped plaintext synchronously, kicks off
// the dynamic grammar import, then swaps in highlighted HTML when ready.
// Extracted into its own component because the async highlight needs
// React state to re-render; the previous inline dangerouslySetInnerHTML
// version assumed all grammars were already registered at module load.
interface CodeBlockProps {
  lang: string;
  code: string;
  C: any;
  themeKey: string;
  onCopy?: (text: string) => void;
  onCopyEvent?: (lang: string, length: number) => void;
}
const CodeBlock: React.FC<CodeBlockProps> = ({ lang, code, C, themeKey, onCopy, onCopyEvent }) => {
  const canon = canonicalLang(lang);
  const initial = React.useMemo(() => {
    if (canon && hljs.getLanguage(canon)) {
      try { return hljs.highlight(code, { language: canon }).value; }
      catch { return escapeHtml(code); }
    }
    return escapeHtml(code);
  }, [canon, code]);
  const [html, setHtml] = React.useState<string>(initial);
  React.useEffect(() => {
    if (!canon) return;
    if (hljs.getLanguage(canon)) return; // already applied synchronously
    let cancelled = false;
    ensureLanguage(lang).then(ok => {
      if (cancelled || !ok) return;
      try {
        const next = hljs.highlight(code, { language: canon }).value;
        setHtml(next);
      } catch { /* keep plain */ }
    });
    return () => { cancelled = true; };
  }, [lang, canon, code]);
  return (
    <div style={{
      margin: `${T.spacing.sm} 0`, border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
      background: themeKey === 'light' ? '#f8fafd' : '#0a0b13', overflow: 'hidden',
    }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: `${T.spacing.xs} ${T.spacing.sm}`, borderBottom: `1px solid ${C.borderSubtle}`,
        fontSize: '10px', color: C.textDim,
        textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
      }}>
        <span>{lang}</span>
        <button onClick={() => { onCopy?.(code); onCopyEvent?.(lang, code.length); }}
          style={{
            background: 'transparent', border: 'none', color: C.textMuted,
            cursor: 'pointer', fontSize: '10px',
            textTransform: 'uppercase', letterSpacing: T.typography.trackingLoose,
          }}>Copy</button>
      </div>
      {/* c2-355 / task 74: line-number gutter. Two-column table layout so the
          gutter scrolls with the content horizontally (no visual drift on
          wide lines) but stays fixed-width (36px + 12px padding) on the
          left. userSelect: none so Copy doesn't include the numbers. We
          don't re-highlight here; the hljs HTML remains untouched. Line
          count is derived from the raw code so non-printable trailing
          content is counted correctly. */}
      <div style={{
        display: 'flex',
        fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '12.5px',
        lineHeight: T.typography.lineNormal,
      }}>
        <div aria-hidden='true' style={{
          width: '36px', padding: `${T.spacing.md} 12px ${T.spacing.md} 0`,
          color: C.textDim, textAlign: 'right', userSelect: 'none',
          whiteSpace: 'pre', flexShrink: 0,
          borderRight: `1px solid ${C.borderSubtle}`,
        }}>
          {Array.from({ length: Math.max(1, code.split('\n').length - (code.endsWith('\n') ? 1 : 0)) },
            (_, n) => String(n + 1)).join('\n')}
        </div>
        <pre style={{
          margin: 0, padding: `${T.spacing.md} ${T.spacing.lg}`,
          color: C.text, whiteSpace: 'pre', overflowX: 'auto', flex: 1,
        }} dangerouslySetInnerHTML={{ __html: html }} />
      </div>
    </div>
  );
};

export interface MarkdownCtx {
  C: any;
  themeKey: string;     // settings.theme — used to pick light/dark fenced-code background
  onCopy?: (text: string) => void;
  onCopyEvent?: (lang: string, length: number) => void;
  // Optional case-insensitive substring to wrap with <mark> in plain-text
  // segments. Driven by Cmd+Shift+F in-conversation search. Skip code blocks
  // and links to avoid mangling formatting.
  highlight?: string;
  // c2-433 / #317: fact-key popover. When provided, the parser turns
  // `[fact:abc123]` (or `[k:abc123]`) tokens into clickable chips that fire
  // this callback with the key + the chip's bounding rect so the host can
  // anchor a popover next to the click. Without the callback, the syntax
  // renders as plain text — keeps the parser future-proof.
  onFactKey?: (key: string, anchorRect: DOMRect) => void;
}

// Wrap occurrences of `query` (case-insensitive) inside a plain-text string
// with <mark>. Returns React node. No-op if query is empty. Uses split with
// a capturing group, so odd indices are matches and even indices are
// surrounding text — no stateful regex needed.
const wrapHighlight = (text: string, query: string | undefined, baseKey: string): React.ReactNode => {
  if (!query) return text;
  const q = query.trim();
  if (!q) return text;
  const re = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'i');
  const parts = text.split(re);
  return parts.map((p, i) => i % 2 === 1
    ? <mark key={`${baseKey}-h${i}`} style={{ background: 'rgba(255,211,107,0.45)', color: 'inherit', padding: '0 1px', borderRadius: '2px' }}>{p}</mark>
    : p);
};

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
    // c2-357 / task 76: $$display$$ and $inline$ math. Matched before
    // emphasis so the dollars don't get eaten by a stray bold/italic. We
    // don't render actual math (MathJax/KaTeX is ~200 kB) -- instead we
    // give the content a monospace font + subtle background chip so LaTeX
    // stands out from prose. Downstream MathJax adoption can drop into
    // the same spans without changing the detection pass.
    //
    // c2-354 / task 70: ~~strikethrough~~ handled before bold/italic so the
    // tilde pair doesn't get eaten by a stray emphasis run. Matches
    // GitHub/CommonMark extended syntax exactly: 2+ tildes on both sides.
    const tokens = seg.split(/(\$\$[^$\n]+\$\$|\$[^$\n]+\$|~~[^~\n]+~~|\*\*[^*\n]+\*\*|\*[^*\n]+\*)/g);
    tokens.forEach((tok, j) => {
      if (tok.startsWith('$$') && tok.endsWith('$$') && tok.length >= 4) {
        out.push(
          <div key={`${baseKey}-mdisp${i}-${j}`} style={{
            fontFamily: C.font, fontSize: '0.95em',
            padding: '6px 10px', margin: '6px 0',
            background: themeKey === 'light' ? 'rgba(20,30,60,0.04)' : 'rgba(255,255,255,0.05)',
            borderRadius: '4px', color: C.textSecondary,
            overflowX: 'auto', whiteSpace: 'pre',
          }}>{tok.slice(2, -2)}</div>
        );
      } else if (tok.startsWith('$') && tok.endsWith('$') && tok.length >= 2) {
        out.push(
          <span key={`${baseKey}-minl${i}-${j}`} style={{
            fontFamily: C.font, fontSize: '0.92em',
            padding: '0 4px', borderRadius: '3px',
            background: themeKey === 'light' ? 'rgba(20,30,60,0.04)' : 'rgba(255,255,255,0.05)',
            color: C.textSecondary,
          }}>{tok.slice(1, -1)}</span>
        );
      } else if (tok.startsWith('~~') && tok.endsWith('~~') && tok.length >= 4) {
        out.push(
          <span key={`${baseKey}-s${i}-${j}`}
            style={{ textDecoration: 'line-through', color: C.textMuted }}>
            {tok.slice(2, -2)}
          </span>
        );
      } else if (tok.startsWith('**') && tok.endsWith('**') && tok.length >= 4) {
        out.push(<strong key={`${baseKey}-b${i}-${j}`}>{tok.slice(2, -2)}</strong>);
      } else if (tok.startsWith('*') && tok.endsWith('*') && tok.length >= 2) {
        out.push(<em key={`${baseKey}-i${i}-${j}`}>{tok.slice(1, -1)}</em>);
      } else if (tok) {
        // c2-433 / #317: split first on link syntax, then on fact-key syntax
        // (`[fact:abc]` or `[k:abc]`). Order matters — links can contain
        // colons inside their text or url, so we strip them first.
        const linkParts = tok.split(/(\[[^\]]+\]\([^)]+\))/g);
        linkParts.forEach((lp, k) => {
          const linkMatch = lp.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
          if (linkMatch) {
            out.push(<a key={`${baseKey}-l${i}-${j}-${k}`} href={linkMatch[2]}
              target="_blank" rel="noopener noreferrer"
              style={{ color: C.accent, textDecoration: 'underline' }}
            >{linkMatch[1]}</a>);
          } else if (lp) {
            // Fact-key chips: split on `[fact:KEY]` / `[k:KEY]` runs.
            const factParts = lp.split(/(\[(?:fact|k):[A-Za-z0-9_-]{1,64}\])/g);
            // c2-433 / #357: inline citations ship as `[fact:KEY] (source:
            // X, similarity N%)`. After a chip, sniff the next text segment
            // and strip+render the metadata as a muted annotation. The
            // regex tolerates a leading space and `similarity N%` as an
            // integer percent. Backend may emit both `(source: ..., ...)`
            // and `[source ..., ...]` variants — we match either bracket.
            const citationMeta = /^\s*[\(\[]source:\s*([^,\)\]]+),\s*similarity\s+(\d+)%[\)\]]\s*/;
            let skipNext = false;
            factParts.forEach((fp, m) => {
              if (skipNext) { skipNext = false; return; }
              const factMatch = fp.match(/^\[(?:fact|k):([A-Za-z0-9_-]{1,64})\]$/);
              if (factMatch && ctx.onFactKey) {
                const key = factMatch[1];
                // Peek ahead for citation metadata.
                const nextFp = factParts[m + 1] || '';
                const metaMatch = nextFp.match(citationMeta);
                const metaSource = metaMatch ? metaMatch[1].trim() : null;
                const metaSimilarity = metaMatch ? Number(metaMatch[2]) : null;
                out.push(
                  <button key={`${baseKey}-f${i}-${j}-${k}-${m}`}
                    onClick={(e) => {
                      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                      ctx.onFactKey!(key, rect);
                    }}
                    title={metaMatch ? `Inspect fact ${key} · source: ${metaSource} · similarity ${metaSimilarity}%` : `Inspect fact ${key}`}
                    style={{
                      display: 'inline-block', padding: '0 5px',
                      margin: '0 2px',
                      background: 'transparent',
                      border: `1px solid ${C.borderSubtle}`,
                      borderRadius: '3px',
                      color: C.accent, fontFamily: C.font || 'monospace',
                      fontSize: '0.85em', fontWeight: 600,
                      cursor: 'pointer', verticalAlign: 'baseline',
                      lineHeight: '1.2',
                    }}>{key}</button>
                );
                if (metaMatch) {
                  // Tint similarity: 80+ accent, 50+ muted, below textDim.
                  const simColor = metaSimilarity != null
                    ? (metaSimilarity >= 80 ? C.accent : metaSimilarity >= 50 ? (C.textMuted || C.textSecondary) : C.textDim)
                    : C.textDim;
                  out.push(
                    <span key={`${baseKey}-fm${i}-${j}-${k}-${m}`} style={{
                      color: C.textDim || '#888', fontSize: '0.78em',
                      fontFamily: C.font || 'monospace', marginRight: '2px',
                      opacity: 0.85,
                    }} title={`similarity ${metaSimilarity}%`}>
                      <span style={{ color: simColor }}>{metaSource}</span>
                      {metaSimilarity != null && <span style={{ marginLeft: '4px' }}>{metaSimilarity}%</span>}
                    </span>
                  );
                  // Render any trailing text AFTER the metadata.
                  const remainder = nextFp.replace(citationMeta, '');
                  if (remainder) {
                    out.push(<span key={`${baseKey}-fr${i}-${j}-${k}-${m}`}>{wrapHighlight(remainder, ctx.highlight, `${baseKey}-fr${i}-${j}-${k}-${m}`)}</span>);
                  }
                  skipNext = true; // already handled nextFp
                }
              } else if (factMatch) {
                // No callback — render the literal token so the syntax
                // is still readable.
                out.push(<span key={`${baseKey}-f${i}-${j}-${k}-${m}`}>{fp}</span>);
              } else if (fp) {
                out.push(<span key={`${baseKey}-t${i}-${j}-${k}-${m}`}>{wrapHighlight(fp, ctx.highlight, `${baseKey}-t${i}-${j}-${k}-${m}`)}</span>);
              }
            });
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
      <CodeBlock key={`c${key++}`} lang={lang} code={code}
        C={C} themeKey={themeKey}
        onCopy={onCopy} onCopyEvent={onCopyEvent} />
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
    // Pipe-table detection. A table block is:
    //   | header1 | header2 |
    //   |---------|---------|
    //   | data    | data    |
    // We scan ahead from any line starting with '|' to detect the separator
    // row; if found, consume the contiguous block and render as <table>.
    const splitRow = (line: string) => line.replace(/^\s*\|/, '').replace(/\|\s*$/, '').split('|').map(c => c.trim());
    const isSeparator = (line: string) => /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?\s*$/.test(line);
    let i = 0;
    while (i < listLines.length) {
      const line = listLines[i];
      const next = listLines[i + 1];
      if (line.trim().startsWith('|') && next && isSeparator(next)) {
        flushList();
        const header = splitRow(line);
        const rows: string[][] = [];
        let j = i + 2;
        while (j < listLines.length && listLines[j].trim().startsWith('|')) {
          rows.push(splitRow(listLines[j]));
          j++;
        }
        parts.push(
          <div key={`tbl${key++}`} style={{ overflowX: 'auto', margin: '10px 0' }}>
            <table style={{
              borderCollapse: 'collapse', width: '100%',
              fontSize: '13px', color: C.text,
            }}>
              <thead>
                <tr>{header.map((h, hi) => (
                  <th key={hi} style={{
                    textAlign: 'left', padding: '8px 10px', fontWeight: 700,
                    background: C.bgInput, borderBottom: `2px solid ${C.borderSubtle}`,
                    borderRight: hi < header.length - 1 ? `1px solid ${C.borderSubtle}` : 'none',
                  }}>{renderInlineMd(h, `th${key}-${hi}`, ctx)}</th>
                ))}</tr>
              </thead>
              <tbody>
                {rows.map((row, ri) => (
                  <tr key={ri} style={{ background: ri % 2 === 0 ? 'transparent' : (themeKey === 'light' ? 'rgba(0,0,0,0.02)' : 'rgba(255,255,255,0.02)') }}>
                    {row.map((cell, ci) => (
                      <td key={ci} style={{
                        padding: '8px 10px',
                        borderBottom: `1px solid ${C.borderSubtle}`,
                        borderRight: ci < row.length - 1 ? `1px solid ${C.borderSubtle}` : 'none',
                      }}>{renderInlineMd(cell, `td${key}-${ri}-${ci}`, ctx)}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
        i = j;
        continue;
      }
      // c2-354 / task 68: horizontal rule. Three or more dashes or asterisks
      // on an otherwise-blank line. Must appear on its own line; inline '---'
      // inside a paragraph is still emphasis text.
      if (/^\s*(-{3,}|\*{3,})\s*$/.test(line)) {
        flushList();
        parts.push(
          <hr key={`hr${key++}`} style={{
            border: 'none', borderTop: `1px solid ${C.borderSubtle}`,
            margin: `${T.spacing.lg} 0`,
          }} />
        );
        i++;
        continue;
      }
      // c2-354 / task 69: blockquote. Group consecutive lines starting with
      // '> ' into a single <blockquote> so multi-line quotes render as one
      // visual block. Nested quotes (>>) render as plain prefix text -- not
      // supporting CommonMark's full nesting yet.
      const quoteMatch = line.match(/^\s*>\s?(.*)$/);
      if (quoteMatch) {
        flushList();
        const quoteLines: string[] = [quoteMatch[1]];
        let qj = i + 1;
        while (qj < listLines.length) {
          const qm = listLines[qj].match(/^\s*>\s?(.*)$/);
          if (!qm) break;
          quoteLines.push(qm[1]);
          qj++;
        }
        parts.push(
          <blockquote key={`bq${key++}`} style={{
            borderLeft: `3px solid ${C.accent}`,
            paddingLeft: T.spacing.lg,
            margin: `${T.spacing.sm} 0`,
            color: C.textSecondary, fontStyle: 'italic',
          }}>
            {quoteLines.map((ql, qi) => (
              <div key={qi}>{renderInlineMd(ql, `bq${key}-${qi}`, ctx)}</div>
            ))}
          </blockquote>
        );
        i = qj;
        continue;
      }
      // c2-354 / task 71: task list items. Detected before the generic
      // bullet match so the checkbox replaces the bullet marker. Rendered
      // disabled -- these are readonly status indicators, not interactive.
      const taskMatch = line.match(/^\s*[-*]\s+\[([ xX])\]\s+(.+)/);
      if (taskMatch) {
        flushList();  // task list items are standalone, not part of a ul group
        const checked = taskMatch[1].toLowerCase() === 'x';
        parts.push(
          <div key={`task${key++}`} style={{
            display: 'flex', alignItems: 'flex-start',
            gap: T.spacing.sm, marginBottom: '4px',
          }}>
            <input type='checkbox' checked={checked} disabled readOnly
              aria-label={checked ? 'completed task' : 'open task'}
              style={{ marginTop: '3px', accentColor: C.accent, flexShrink: 0 }} />
            <span style={{
              color: checked ? C.textMuted : C.text,
              textDecoration: checked ? 'line-through' : 'none',
            }}>{renderInlineMd(taskMatch[2], `task${key}`, ctx)}</span>
          </div>
        );
        i++;
        continue;
      }
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
      i++;
    }
    flushList();
  }
  return parts.length > 0 ? parts : [<span key='empty'>{renderInlineMd(text, 'solo', ctx)}</span>];
};
