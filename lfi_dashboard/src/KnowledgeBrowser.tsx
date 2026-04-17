import React from 'react';

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
  onClose: () => void;
}

export const KnowledgeBrowser: React.FC<KnowledgeBrowserProps> = ({ C, facts, concepts, due, onClose }) => (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: 230,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: '16px',
    }}>
    <div role='dialog' aria-modal='true' aria-label='Knowledge browser'
      onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '700px', height: '80vh',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
        display: 'flex', flexDirection: 'column', overflow: 'hidden',
        boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
      }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
      }}>
        <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 700, color: C.text }}>Knowledge Browser</h2>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <span style={{ fontSize: '12px', color: C.textMuted }}>
            {facts.length} facts &middot; {concepts.length} concepts &middot; {due.length} due
          </span>
          <button onClick={onClose} aria-label='Close knowledge browser'
            style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
            {'\u2715'}
          </button>
        </div>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
        {/* Due for review */}
        {due.length > 0 && (
          <div style={{ marginBottom: '20px' }}>
            <div style={{ fontSize: '11px', fontWeight: 700, color: C.accent, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: '10px' }}>
              Due for review ({due.length})
            </div>
            {due.map((d, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 12px',
                background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '8px',
                marginBottom: '6px',
              }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: '13px', fontWeight: 600, color: C.text }}>{d.name}</div>
                  <div style={{ fontSize: '11px', color: C.textMuted }}>
                    Mastery {(d.mastery * 100).toFixed(0)}% &middot; {d.days_overdue.toFixed(1)} days overdue
                  </div>
                </div>
                <div style={{
                  width: '60px', height: '6px', background: C.bgInput, borderRadius: '3px', overflow: 'hidden',
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
        <div style={{ marginBottom: '20px' }}>
          <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: '10px' }}>
            Facts ({facts.length})
          </div>
          {facts.length === 0 ? (
            <div style={{ fontSize: '13px', color: C.textDim, padding: '12px', textAlign: 'center' }}>
              No facts learned yet. Chat with the AI — it picks up facts from conversation.
            </div>
          ) : (
            facts.map((f, i) => (
              <div key={i} style={{
                display: 'flex', gap: '8px', padding: '8px 12px',
                borderBottom: `1px solid ${C.borderSubtle}`, fontSize: '13px',
              }}>
                <span style={{ color: C.accent, fontWeight: 600, minWidth: '120px' }}>{f.key}</span>
                <span style={{ color: C.text, flex: 1 }}>{f.value}</span>
              </div>
            ))
          )}
        </div>

        {/* Concepts */}
        <div>
          <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: '10px' }}>
            Concepts ({concepts.length})
          </div>
          {concepts.length === 0 ? (
            <div style={{ fontSize: '13px', color: C.textDim, padding: '12px', textAlign: 'center' }}>
              No concepts yet. Teach the AI with /knowledge or via Settings.
            </div>
          ) : (
            concepts.map((c, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: '12px',
                padding: '10px 12px', borderBottom: `1px solid ${C.borderSubtle}`,
              }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: '13px', fontWeight: 600, color: C.text }}>{c.name}</div>
                  <div style={{ fontSize: '11px', color: C.textDim }}>
                    {c.review_count} reviews &middot; mastery {(c.mastery * 100).toFixed(0)}%
                  </div>
                </div>
                <div style={{
                  width: '80px', height: '6px', background: C.bgInput, borderRadius: '3px', overflow: 'hidden',
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
      </div>
    </div>
  </div>
);
