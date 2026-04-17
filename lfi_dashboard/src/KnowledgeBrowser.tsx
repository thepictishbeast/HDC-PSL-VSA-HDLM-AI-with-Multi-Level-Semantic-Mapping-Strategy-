import React, { useRef } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

// Knowledge browser modal: facts, concepts, and the "due for review" list.
// Pure presentational — parent owns the data and the fetchKnowledge trigger.

export interface KnowledgeFact { key: string; value: string }
export interface KnowledgeConcept { name: string; review_count: number; mastery: number }
export interface KnowledgeDue { name: string; mastery: number; days_overdue: number }

export interface KnowledgeBrowserProps {
  C: any;
  facts: KnowledgeFact[];
  concepts: KnowledgeConcept[];
  due: KnowledgeDue[];
  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;
  onClose: () => void;
}

export const KnowledgeBrowser: React.FC<KnowledgeBrowserProps> = ({ C, facts, concepts, due, loading, error, onRetry, onClose }) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  const isEmpty = !loading && !error && facts.length === 0 && concepts.length === 0 && due.length === 0;
  return (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: T.z.modal + 30,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: T.spacing.lg,
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-knowledge-title'
      onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '700px', height: '80vh',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
        display: 'flex', flexDirection: 'column', overflow: 'hidden',
        boxShadow: T.shadows.modal,
      }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
      }}>
        <h2 id='scc-knowledge-title' style={{ margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>Knowledge Browser</h2>
        <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center' }}>
          <span style={{ fontSize: T.typography.sizeSm, color: C.textMuted }}>
            {facts.length} facts &middot; {concepts.length} concepts &middot; {due.length} due
          </span>
          <button onClick={onClose} aria-label='Close knowledge browser'
            style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
            {'\u2715'}
          </button>
        </div>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
        {/* Loading skeleton — shown while the 3-endpoint fetch is inflight. */}
        {loading && (
          <div aria-busy='true' aria-live='polite' style={{ padding: '12px 0' }}>
            {[0, 1, 2].map(i => (
              <div key={i} style={{
                height: '42px', marginBottom: T.spacing.md, borderRadius: T.radii.lg,
                background: `linear-gradient(90deg, ${C.bgInput} 0%, ${C.bgHover} 50%, ${C.bgInput} 100%)`,
                backgroundSize: '200% 100%',
                animation: 'scc-skel 1.4s ease-in-out infinite',
              }} />
            ))}
            <style>{`@keyframes scc-skel { 0% { background-position: 200% 0 } 100% { background-position: -200% 0 } }`}</style>
          </div>
        )}
        {/* Error — tell the user what/why/what next (Tier 4 §26). */}
        {error && !loading && (
          <div role='alert' style={{
            padding: '16px 18px', marginBottom: T.spacing.lg,
            background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: '10px',
            display: 'flex', flexDirection: 'column', gap: T.spacing.sm,
          }}>
            <div style={{ fontSize: T.typography.sizeMd, fontWeight: 700, color: C.red }}>Could not load knowledge</div>
            <div style={{ fontSize: T.typography.sizeSm, color: C.textSecondary, lineHeight: 1.55 }}>
              {error} &middot; The backend at <code style={{ fontFamily: 'monospace' }}>/api/facts</code>, <code style={{ fontFamily: 'monospace' }}>/api/knowledge/concepts</code>, <code style={{ fontFamily: 'monospace' }}>/api/knowledge/due</code> did not respond. Check that the Rust server is running on port 3000.
            </div>
            {onRetry && (
              <button onClick={onRetry}
                style={{
                  alignSelf: 'flex-start', padding: '6px 14px', fontSize: T.typography.sizeSm, fontWeight: 700,
                  background: C.accentBg, border: `1px solid ${C.accentBorder}`, color: C.accent,
                  borderRadius: '6px', cursor: 'pointer', fontFamily: 'inherit',
                }}>Retry</button>
            )}
          </div>
        )}
        {/* Full zero state — no facts, no concepts, nothing due. */}
        {isEmpty && (
          <div style={{
            padding: '40px 20px', textAlign: 'center',
            display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px',
          }}>
            <svg width='48' height='48' viewBox='0 0 24 24' fill='none' stroke={C.textMuted} strokeWidth='1.5' aria-hidden='true'>
              <path d='M12 3v18M3 12h18' strokeLinecap='round' />
              <circle cx='12' cy='12' r='9' />
            </svg>
            <div style={{ fontSize: '14px', fontWeight: 700, color: C.text }}>Nothing learned yet</div>
            <div style={{ fontSize: T.typography.sizeSm, color: C.textMuted, maxWidth: '360px', lineHeight: 1.6 }}>
              PlausiDen's knowledge base is empty. Start chatting — it records facts and reinforces concepts from every exchange. Use <code style={{ fontFamily: 'monospace', color: C.accent }}>/knowledge</code> to seed it manually.
            </div>
          </div>
        )}
        {/* Due for review */}
        {!loading && !error && due.length > 0 && (
          <div style={{ marginBottom: '20px' }}>
            <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.accent, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: T.spacing.md }}>
              Due for review ({due.length})
            </div>
            {due.map((d, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: T.spacing.md, padding: '10px 12px',
                background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.lg,
                marginBottom: '6px',
              }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: T.typography.sizeMd, fontWeight: 600, color: C.text }}>{d.name}</div>
                  <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted }}>
                    Mastery {(d.mastery * 100).toFixed(0)}% &middot; {d.days_overdue.toFixed(1)} days overdue
                  </div>
                </div>
                <div style={{
                  width: '60px', height: '6px', background: C.bgInput, borderRadius: T.radii.xs, overflow: 'hidden',
                }}>
                  <div style={{
                    width: `${d.mastery * 100}%`, height: '100%',
                    background: d.mastery > 0.7 ? C.green : d.mastery > 0.3 ? C.yellow : C.red,
                  }} />
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Facts */}
        {!loading && !error && !isEmpty && (
        <div style={{ marginBottom: '20px' }}>
          <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: T.spacing.md }}>
            Facts ({facts.length})
          </div>
          {facts.length === 0 ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.md, textAlign: 'center' }}>
              No facts learned yet. Chat with the AI — it picks up facts from conversation.
            </div>
          ) : (
            facts.map((f, i) => (
              <div key={i} style={{
                display: 'flex', gap: T.spacing.sm, padding: '8px 12px',
                borderBottom: `1px solid ${C.borderSubtle}`, fontSize: T.typography.sizeMd,
              }}>
                <span style={{ color: C.accent, fontWeight: 600, minWidth: '120px' }}>{f.key}</span>
                <span style={{ color: C.text, flex: 1 }}>{f.value}</span>
              </div>
            ))
          )}
        </div>
        )}

        {/* Concepts */}
        {!loading && !error && !isEmpty && (
        <div>
          <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: T.spacing.md }}>
            Concepts ({concepts.length})
          </div>
          {concepts.length === 0 ? (
            <div style={{ fontSize: T.typography.sizeMd, color: C.textDim, padding: T.spacing.md, textAlign: 'center' }}>
              No concepts yet. Teach the AI with /knowledge or via Settings.
            </div>
          ) : (
            concepts.map((c, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: T.spacing.md,
                padding: '10px 12px', borderBottom: `1px solid ${C.borderSubtle}`,
              }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: T.typography.sizeMd, fontWeight: 600, color: C.text }}>{c.name}</div>
                  <div style={{ fontSize: T.typography.sizeXs, color: C.textDim }}>
                    {c.review_count} reviews &middot; mastery {(c.mastery * 100).toFixed(0)}%
                  </div>
                </div>
                <div style={{
                  width: '80px', height: '6px', background: C.bgInput, borderRadius: T.radii.xs, overflow: 'hidden',
                }}>
                  <div style={{
                    width: `${c.mastery * 100}%`, height: '100%',
                    background: c.mastery > 0.7 ? C.green : c.mastery > 0.3 ? C.yellow : C.red,
                  }} />
                </div>
              </div>
            ))
          )}
        </div>
        )}
      </div>
    </div>
  </div>
  );
};
