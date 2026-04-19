import React, { useRef, useState } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';
// c2-349 / task 29: shared shimmer skeleton.
import { SkeletonLoader } from './components/SkeletonLoader';

// Knowledge browser modal: facts, concepts, and the "due for review" list.
// Pure presentational — parent owns the data and the fetchKnowledge trigger.

export interface KnowledgeFact { key: string; value: string }
export interface KnowledgeConcept { name: string; review_count: number; mastery: number }
// c2-433 / #337: fact_key added so the FSRS rating buttons can POST a review
// keyed to the specific card. Legacy SM-2 /api/knowledge/due payloads lack
// fact_key, in which case the rating buttons are hidden (graceful degradation).
export interface KnowledgeDue { name: string; mastery: number; days_overdue: number; fact_key?: string }

export interface KnowledgeBrowserProps {
  C: any;
  facts: KnowledgeFact[];
  concepts: KnowledgeConcept[];
  due: KnowledgeDue[];
  // c2-433 / #337 followup: FSRS envelope meta from /api/fsrs/due. Null
  // when response came from the legacy SM-2 path — header line hides.
  fsrsMeta?: { due_cards?: number; target_retention?: number } | null;
  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;
  onClose: () => void;
  // c2-405 / task 191: optional jump-link to Admin → Training. When provided,
  // the zero state surfaces a button that opens it (instead of just telling
  // users to chat more).
  onOpenTraining?: () => void;
  // c2-433 / #337: FSRS review handler. When provided AND the due row has a
  // fact_key, four rating buttons (Again / Hard / Good / Easy = 1..4) appear
  // on the card. Parent POSTs /api/fsrs/review and refetches.
  onReview?: (factKey: string, rating: 1 | 2 | 3 | 4) => Promise<void> | void;
}

export const KnowledgeBrowser: React.FC<KnowledgeBrowserProps> = ({ C, facts, concepts, due, fsrsMeta, loading, error, onRetry, onClose, onOpenTraining, onReview }) => {
  // c2-433 / #337: per-row review state — which fact_key is currently being
  // graded (disables its buttons + shows inline spinner). Map so multiple
  // reviews can technically be in flight, though the UX expects serial.
  const [reviewing, setReviewing] = useState<Record<string, boolean>>({});
  // c2-433 / #313-ish: Copy-JSON export feedback. 2s Copy → Copied ✓ flip,
  // matching the Drift/Ledger/Runs export pattern.
  const [copiedAt, setCopiedAt] = useState<number>(0);
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  const isEmpty = !loading && !error && facts.length === 0 && concepts.length === 0 && due.length === 0;
  // c2-433 / task 274: filter input — narrows facts (key+value), concepts
  // (name), and due (name) by case-insensitive substring. Useful when the
  // KB grows past ~50 entries.
  const [filter, setFilter] = useState<string>('');
  const fLower = filter.trim().toLowerCase();
  const filteredFacts = fLower
    ? facts.filter(f => f.key.toLowerCase().includes(fLower) || f.value.toLowerCase().includes(fLower))
    : facts;
  const filteredConcepts = fLower
    ? concepts.filter(c => c.name.toLowerCase().includes(fLower))
    : concepts;
  const filteredDue = fLower
    ? due.filter(d => d.name.toLowerCase().includes(fLower))
    : due;
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
        width: '100%', maxWidth: '700px', height: '80dvh',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
        display: 'flex', flexDirection: 'column', overflow: 'hidden',
        boxShadow: T.shadows.modal,
      }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
      }}>
        <h2 id='scc-knowledge-title' style={{ margin: 0, fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBold, color: C.text }}>Knowledge Browser</h2>
        <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center', flexWrap: 'wrap', justifyContent: 'flex-end', minWidth: 0 }}>
          {/* c2-433 / task 274: filter input. Narrows facts/concepts/due
              by substring. Counts shown reflect filter when active. */}
          <input type='search' value={filter}
            onChange={(e) => setFilter(e.target.value)}
            // c2-433 / task 282: Esc-to-clear-filter (then Esc again closes
            // the modal via the global Esc handler). Standard step-down
            // pattern — gives users a graceful way to back out of a filter
            // without losing the modal.
            onKeyDown={(e) => {
              if (e.key === 'Escape' && filter) {
                e.preventDefault();
                e.stopPropagation();
                setFilter('');
              }
            }}
            placeholder='Filter…' aria-label='Filter knowledge'
            style={{
              padding: '4px 10px', fontSize: T.typography.sizeXs,
              background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
              borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
              // c2-433 / mobile fix: shrink below 140px on narrow viewports
              // so the header count + close button stay visible.
              outline: 'none', flex: '1 1 100px', maxWidth: '180px', minWidth: 0,
            }} />
          <span style={{ fontSize: T.typography.sizeSm, color: C.textMuted, fontFamily: T.typography.fontMono }}>
            {fLower
              ? `${filteredFacts.length}/${facts.length} f · ${filteredConcepts.length}/${concepts.length} c · ${filteredDue.length}/${due.length} d`
              : `${facts.length} facts · ${concepts.length} concepts · ${due.length} due`}
          </span>
          {/* c2-433: Copy-JSON export for the KB snapshot. Disabled when
              everything is empty (nothing to export). 2s Copied ✓ feedback
              matches the Drift/Ledger/Runs pattern. */}
          <button
            disabled={isEmpty}
            onClick={async () => {
              const payload = {
                exported_at: new Date().toISOString(),
                facts, concepts, due, fsrsMeta,
              };
              try {
                await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
                setCopiedAt(Date.now());
                window.setTimeout(() => setCopiedAt(0), 2000);
              } catch { /* clipboard blocked */ }
            }}
            title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${facts.length} facts + ${concepts.length} concepts + ${due.length} due as JSON`}
            style={{
              background: copiedAt > 0 ? `${C.green}18` : 'transparent',
              border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
              color: copiedAt > 0 ? C.green : isEmpty ? C.textDim : C.textMuted,
              borderRadius: T.radii.sm,
              cursor: isEmpty ? 'not-allowed' : 'pointer',
              padding: '4px 10px', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
              opacity: isEmpty ? 0.5 : 1,
              flexShrink: 0,
            }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
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
              <SkeletonLoader key={i} C={C} base='input' height='42px'
                style={{ marginBottom: T.spacing.md }} />
            ))}
          </div>
        )}
        {/* Error — tell the user what/why/what next (Tier 4 §26). */}
        {error && !loading && (
          <div role='alert' style={{
            padding: '16px 18px', marginBottom: T.spacing.lg,
            background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: T.radii.xl,
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
                  borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                }}>Retry</button>
            )}
          </div>
        )}
        {/* c2-405 / task 191: full zero state. Explains WHY it's empty (KB
            hydrates from ingestion / chat) and gives two paths forward —
            chat more, or open Admin → Training to kick off ingestion. */}
        {isEmpty && (
          <div style={{
            padding: '40px 20px', textAlign: 'center',
            display: 'flex', flexDirection: 'column', alignItems: 'center', gap: T.spacing.sm,
          }}>
            <svg width='48' height='48' viewBox='0 0 24 24' fill='none' stroke={C.textMuted} strokeWidth='1.5' aria-hidden='true'>
              <path d='M12 3v18M3 12h18' strokeLinecap='round' />
              <circle cx='12' cy='12' r='9' />
            </svg>
            <div style={{ fontSize: T.typography.sizeBody, fontWeight: 700, color: C.text }}>Nothing learned yet</div>
            <div style={{ fontSize: T.typography.sizeSm, color: C.textMuted, maxWidth: '420px', lineHeight: 1.6 }}>
              The knowledge base hydrates from two sources: ongoing chat (facts + concepts get picked up automatically), and the background training pipeline (ingests curated sources into the KB). If both are idle, it stays empty.
            </div>
            <div style={{ display: 'flex', gap: T.spacing.sm, marginTop: T.spacing.sm, flexWrap: 'wrap', justifyContent: 'center' }}>
              {onOpenTraining && (
                <button onClick={onOpenTraining}
                  style={{
                    padding: '8px 16px', fontSize: T.typography.sizeSm, fontWeight: T.typography.weightBold,
                    background: C.accentBg, border: `1px solid ${C.accentBorder}`, color: C.accent,
                    borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                  }}>Open Training →</button>
              )}
              <button onClick={onClose}
                style={{
                  padding: '8px 16px', fontSize: T.typography.sizeSm, fontWeight: T.typography.weightBold,
                  background: 'transparent', border: `1px solid ${C.border}`, color: C.textSecondary,
                  borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                }}>Start chatting</button>
            </div>
            <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: T.spacing.xs }}>
              Or run <code style={{ fontFamily: 'monospace', color: C.accent }}>/knowledge</code> at the chat prompt to reopen this panel.
            </div>
          </div>
        )}
        {/* Due for review */}
        {!loading && !error && due.length > 0 && (
          <div style={{ marginBottom: '20px' }}>
            <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.accent, textTransform: 'uppercase', letterSpacing: '0.10em', marginBottom: T.spacing.md, display: 'flex', alignItems: 'baseline', gap: T.spacing.sm, flexWrap: 'wrap' }}>
              <span>Due for review ({due.length}{fsrsMeta?.due_cards != null && fsrsMeta.due_cards > due.length ? ` of ${fsrsMeta.due_cards}` : ''})</span>
              {/* c2-433 / #337 followup: FSRS envelope meta line. Shows the
                  target retention the scheduler is grading against so the
                  user understands WHY a card is due (the scheduler aims to
                  re-show right before the retrievability drops below this
                  target). Hidden when not on the FSRS branch. */}
              {fsrsMeta?.target_retention != null && (
                <span title={`FSRS target retention: ${(fsrsMeta.target_retention * 100).toFixed(0)}% — cards reappear just before retrievability falls below this`}
                  style={{
                    fontSize: '10px', fontWeight: 600,
                    color: C.textMuted, fontFamily: T.typography.fontMono,
                    textTransform: 'none', letterSpacing: 0,
                  }}>
                  · target retention {(fsrsMeta.target_retention * 100).toFixed(0)}%
                </span>
              )}
            </div>
            {filteredDue.map((d, i) => {
              const canReview = !!(onReview && d.fact_key);
              const isReviewing = canReview && reviewing[d.fact_key!] === true;
              return (
              <div key={i} style={{
                display: 'flex', flexDirection: 'column', gap: T.spacing.sm, padding: '10px 12px',
                background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: T.radii.lg,
                marginBottom: '6px',
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: T.spacing.md }}>
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
                {/* c2-433 / #337: FSRS 4-rating row. Mapping follows FSRS
                    convention: 1=Again (failed recall), 2=Hard (recalled but
                    slowly), 3=Good (normal), 4=Easy (trivial). Buttons are
                    color-coded along the same Again→Easy spectrum: red →
                    amber → accent → green. Hidden when no fact_key or no
                    review handler (legacy SM-2 payloads). */}
                {canReview && (
                  <div style={{ display: 'flex', gap: '4px' }}>
                    {([
                      { r: 1 as const, label: 'Again', hint: 'Forgot — schedule soon', bg: C.redBg, border: C.redBorder, fg: C.red },
                      { r: 2 as const, label: 'Hard',  hint: 'Recalled with effort', bg: C.yellowBg || C.bgInput, border: C.yellow, fg: C.yellow },
                      { r: 3 as const, label: 'Good',  hint: 'Recalled normally', bg: C.accentBg, border: C.accentBorder, fg: C.accent },
                      { r: 4 as const, label: 'Easy',  hint: 'Trivial — longer interval', bg: C.greenBg || C.bgInput, border: C.green, fg: C.green },
                    ]).map(b => (
                      <button key={b.r} disabled={isReviewing}
                        onClick={async () => {
                          if (!onReview || !d.fact_key) return;
                          setReviewing(prev => ({ ...prev, [d.fact_key!]: true }));
                          try { await onReview(d.fact_key, b.r); }
                          finally { setReviewing(prev => { const n = { ...prev }; delete n[d.fact_key!]; return n; }); }
                        }}
                        title={`${b.hint} (${b.r})`}
                        aria-label={`Rate ${d.name}: ${b.label}`}
                        style={{
                          flex: 1, padding: '5px 8px', fontSize: T.typography.sizeXs,
                          fontWeight: T.typography.weightBold,
                          background: b.bg, color: b.fg,
                          border: `1px solid ${b.border}`,
                          borderRadius: T.radii.sm,
                          cursor: isReviewing ? 'wait' : 'pointer',
                          opacity: isReviewing ? 0.5 : 1,
                          fontFamily: 'inherit', letterSpacing: '0.02em',
                        }}>{b.label}</button>
                    ))}
                  </div>
                )}
              </div>
              );
            })}
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
          ) : filteredFacts.length === 0 ? (
            // c2-433 / task 274 followup: filter zero-state. Differentiates
            // "no facts at all" from "no facts match the filter."
            <div style={{ fontSize: T.typography.sizeSm, color: C.textMuted, padding: T.spacing.md, textAlign: 'center', fontStyle: 'italic' }}>
              No facts match "{filter}". <button onClick={() => setFilter('')} style={{ background: 'transparent', border: 'none', color: C.accent, cursor: 'pointer', fontFamily: 'inherit', textDecoration: 'underline', padding: 0 }}>Clear filter.</button>
            </div>
          ) : (
            filteredFacts.map((f, i) => (
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
              No concepts yet. Concepts emerge as the substrate ingests facts and detects clusters — chat more or run an ingestion batch from Classroom.
            </div>
          ) : (
            filteredConcepts.map((c, i) => (
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
