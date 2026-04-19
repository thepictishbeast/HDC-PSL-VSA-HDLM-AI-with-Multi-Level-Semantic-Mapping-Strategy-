/**
 * TourOverlay — #352 interactive walkthrough.
 *
 * Desktop + mobile friendly. Renders:
 *   1. A spotlight cutout over the target element (rounded rect, 4px
 *      accent-color border, dims everything else).
 *   2. A tooltip card near the target with title/body/Next/Prev/Skip.
 *   3. A progress pill (N/M) + keyboard support (←/→/Esc/Enter).
 *   4. Touch: swipe-left = next, swipe-right = prev, tap overlay to
 *      skip-to-close (confirm once).
 *
 * If a step's target selector isn't in the DOM (e.g. user closed the
 * sidebar), the spotlight degrades to a centered modal-style tooltip
 * so the tour never dead-ends.
 *
 * Call with:
 *   <TourOverlay C={C} isMobile={isMobile} open={showTour}
 *     steps={tourSteps} onClose={() => setShowTour(false)}
 *     onActivate={(stepKey) => { ...navigate to prerequisite surface... }}
 *   />
 */

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { T } from './tokens';

export interface TourStep {
  /** Stable id used by onActivate to trigger the prerequisite state
   *  (e.g. open a modal, switch view) before the step renders. */
  key: string;
  /** CSS selector for the element to spotlight. If missing or not in
   *  the DOM, the card centers on screen. */
  target?: string;
  title: string;
  body: React.ReactNode;
  /** Optional — called on step enter so the parent can open a modal /
   *  switch views before we try to position. */
  onEnter?: () => void;
}

export interface TourOverlayProps {
  C: any;
  isMobile: boolean;
  open: boolean;
  steps: TourStep[];
  onClose: () => void;
}

interface Rect { top: number; left: number; width: number; height: number }

const PAD = 8;

export const TourOverlay: React.FC<TourOverlayProps> = ({ C, isMobile, open, steps, onClose }) => {
  const [stepIdx, setStepIdx] = useState(0);
  const [rect, setRect] = useState<Rect | null>(null);
  const touchStartX = useRef<number | null>(null);
  const touchStartY = useRef<number | null>(null);

  const step = steps[stepIdx];
  const total = steps.length;

  // Reset to step 0 each time the tour opens.
  useEffect(() => {
    if (open) setStepIdx(0);
  }, [open]);

  // Run onEnter for the active step; recompute spotlight rect on DOM or resize.
  useEffect(() => {
    if (!open || !step) return;
    if (step.onEnter) {
      try { step.onEnter(); } catch { /* silent */ }
    }
    const measure = () => {
      if (!step.target) { setRect(null); return; }
      const el = document.querySelector(step.target) as HTMLElement | null;
      if (!el) { setRect(null); return; }
      const box = el.getBoundingClientRect();
      if (box.width === 0 && box.height === 0) { setRect(null); return; }
      // Scroll the target into view if it's offscreen.
      if (box.top < 0 || box.bottom > window.innerHeight) {
        try { el.scrollIntoView({ block: 'center', behavior: 'smooth' }); } catch { /* silent */ }
      }
      setRect({
        top: box.top - PAD,
        left: box.left - PAD,
        width: box.width + PAD * 2,
        height: box.height + PAD * 2,
      });
    };
    // Slight delay so any onEnter-triggered DOM mount has a chance to
    // land before we measure. Two passes (immediate + 200ms) handles
    // lazy-loaded modal chunks.
    measure();
    const t1 = window.setTimeout(measure, 50);
    const t2 = window.setTimeout(measure, 250);
    window.addEventListener('resize', measure);
    window.addEventListener('scroll', measure, true);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.removeEventListener('resize', measure);
      window.removeEventListener('scroll', measure, true);
    };
  }, [open, stepIdx, step]);

  const next = useCallback(() => {
    if (stepIdx >= total - 1) { onClose(); return; }
    setStepIdx(i => Math.min(i + 1, total - 1));
  }, [stepIdx, total, onClose]);

  const prev = useCallback(() => {
    setStepIdx(i => Math.max(i - 1, 0));
  }, []);

  // Keyboard: Esc close, ArrowRight/Enter/Space next, ArrowLeft prev.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.preventDefault(); onClose(); }
      else if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') { e.preventDefault(); next(); }
      else if (e.key === 'ArrowLeft') { e.preventDefault(); prev(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, next, prev, onClose]);

  // Lock body scroll while the tour is open so mobile swipes don't
  // drag the page behind the overlay.
  useEffect(() => {
    if (!open) return;
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => { document.body.style.overflow = prevOverflow; };
  }, [open]);

  if (!open || !step) return null;

  // Tooltip placement: desktop → next to spotlight (above/below based on
  // which half of viewport the target sits in). Mobile → always pinned
  // to bottom of viewport for predictable reach.
  const cardStyle: React.CSSProperties = (() => {
    const base: React.CSSProperties = {
      position: 'fixed',
      zIndex: T.z.modal + 200,
      background: C.bgCard,
      color: C.text,
      border: `1px solid ${C.border}`,
      borderRadius: T.radii.lg,
      boxShadow: T.shadows.modal,
      padding: isMobile ? T.spacing.md : T.spacing.lg,
      maxWidth: isMobile ? 'calc(100vw - 24px)' : '420px',
      width: isMobile ? 'calc(100vw - 24px)' : 'auto',
      fontFamily: "'DM Sans', -apple-system, sans-serif",
      fontSize: T.typography.sizeSm,
      lineHeight: 1.55,
      boxSizing: 'border-box',
    };
    if (isMobile) {
      return {
        ...base,
        left: '12px',
        right: '12px',
        bottom: '12px',
      };
    }
    if (!rect) {
      // No target → centre.
      return {
        ...base,
        left: '50%', top: '50%',
        transform: 'translate(-50%, -50%)',
      };
    }
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const belowHasRoom = rect.top + rect.height + 200 < vh;
    const rightSpace = vw - rect.left - rect.width;
    // Preferred: below the target, left-aligned with it. Fallback: above.
    if (belowHasRoom) {
      return {
        ...base,
        left: Math.max(12, Math.min(rect.left, vw - 440 - 12)),
        top: rect.top + rect.height + 12,
      };
    }
    return {
      ...base,
      left: Math.max(12, Math.min(rect.left, vw - 440 - 12)),
      bottom: vh - rect.top + 12,
    };
  })();

  const btnBase: React.CSSProperties = {
    padding: isMobile ? '10px 16px' : '8px 14px',
    fontSize: T.typography.sizeSm,
    fontWeight: T.typography.weightBold,
    borderRadius: T.radii.md,
    cursor: 'pointer',
    fontFamily: 'inherit',
    minHeight: isMobile ? '44px' : 'auto',
  };

  const onTouchStart = (e: React.TouchEvent) => {
    touchStartX.current = e.touches[0]?.clientX ?? null;
    touchStartY.current = e.touches[0]?.clientY ?? null;
  };
  const onTouchEnd = (e: React.TouchEvent) => {
    if (touchStartX.current == null) return;
    const dx = (e.changedTouches[0]?.clientX ?? 0) - touchStartX.current;
    const dy = (e.changedTouches[0]?.clientY ?? 0) - (touchStartY.current ?? 0);
    touchStartX.current = null;
    touchStartY.current = null;
    if (Math.abs(dx) < 60 || Math.abs(dy) > Math.abs(dx)) return;
    if (dx < 0) next(); else prev();
  };

  // Spotlight uses four dim panels around the rect so the cutout stays
  // crisp and click-through to the target (if a step wants live interaction
  // with it, e.g. highlight a button). Falls back to a single full-screen
  // dim panel when there's no target.
  const dim = 'rgba(0,0,0,0.68)';
  const dimPanels: React.CSSProperties[] = rect
    ? [
        { position: 'fixed', top: 0, left: 0, right: 0, height: rect.top, background: dim, zIndex: T.z.modal + 150 },
        { position: 'fixed', top: rect.top, left: 0, width: rect.left, height: rect.height, background: dim, zIndex: T.z.modal + 150 },
        { position: 'fixed', top: rect.top, left: rect.left + rect.width, right: 0, height: rect.height, background: dim, zIndex: T.z.modal + 150 },
        { position: 'fixed', top: rect.top + rect.height, left: 0, right: 0, bottom: 0, background: dim, zIndex: T.z.modal + 150 },
      ]
    : [{ position: 'fixed', inset: 0, background: dim, zIndex: T.z.modal + 150 }];

  return (
    <>
      {dimPanels.map((s, i) => (
        <div key={i} style={s} onClick={onClose} />
      ))}
      {rect && (
        <div style={{
          position: 'fixed',
          top: rect.top, left: rect.left,
          width: rect.width, height: rect.height,
          border: `3px solid ${C.accent}`,
          borderRadius: T.radii.md,
          boxShadow: `0 0 0 9999px rgba(0,0,0,0) inset`,
          pointerEvents: 'none',
          zIndex: T.z.modal + 160,
          animation: 'scc-tour-pulse 1.4s ease-in-out infinite',
        }} />
      )}
      <style>{`
        @keyframes scc-tour-pulse {
          0%, 100% { box-shadow: 0 0 0 0 ${C.accent}66; }
          50% { box-shadow: 0 0 0 6px ${C.accent}00; }
        }
      `}</style>
      <div role='dialog' aria-modal='true' aria-labelledby='scc-tour-title'
        style={cardStyle}
        onTouchStart={onTouchStart}
        onTouchEnd={onTouchEnd}>
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          marginBottom: T.spacing.sm,
        }}>
          <span style={{
            fontSize: '10px', fontWeight: T.typography.weightBlack,
            color: C.accent, letterSpacing: '0.12em', textTransform: 'uppercase',
          }}>
            Tour · {stepIdx + 1}/{total}
          </span>
          <button onClick={onClose} aria-label='Skip tour'
            style={{
              background: 'transparent', border: 'none', color: C.textMuted,
              fontSize: T.typography.sizeLg, cursor: 'pointer',
              padding: isMobile ? '8px 12px' : '2px 8px',
              minHeight: isMobile ? '44px' : 'auto',
              minWidth: isMobile ? '44px' : 'auto',
            }}>{'\u2715'}</button>
        </div>
        <h2 id='scc-tour-title' style={{
          margin: 0, fontSize: isMobile ? T.typography.sizeLg : T.typography.sizeXl,
          fontWeight: T.typography.weightBold, color: C.text,
          marginBottom: T.spacing.sm, lineHeight: 1.3,
        }}>{step.title}</h2>
        <div style={{ color: C.textSecondary, marginBottom: T.spacing.md }}>
          {step.body}
        </div>
        {/* Step progress dots (desktop only — mobile has the pill chip). */}
        {!isMobile && (
          <div style={{
            display: 'flex', gap: '6px', marginBottom: T.spacing.md, flexWrap: 'wrap',
          }}>
            {steps.map((_, i) => (
              <span key={i}
                aria-hidden='true'
                onClick={() => setStepIdx(i)}
                style={{
                  width: '8px', height: '8px', borderRadius: '50%',
                  background: i === stepIdx ? C.accent : (i < stepIdx ? C.accentBorder : C.borderSubtle),
                  cursor: 'pointer',
                  transition: 'background-color 0.15s',
                }} />
            ))}
          </div>
        )}
        <div style={{
          display: 'flex', justifyContent: 'space-between', gap: T.spacing.sm,
          flexWrap: 'wrap',
        }}>
          <button onClick={prev} disabled={stepIdx === 0}
            style={{
              ...btnBase, color: stepIdx === 0 ? C.textDim : C.text,
              background: 'transparent', border: `1px solid ${C.border}`,
              cursor: stepIdx === 0 ? 'default' : 'pointer',
              opacity: stepIdx === 0 ? 0.5 : 1,
            }}>Back</button>
          <button onClick={next} autoFocus
            style={{
              ...btnBase, color: '#fff', background: C.accent, border: 'none',
              flex: isMobile ? '1 0 auto' : undefined,
            }}>{stepIdx >= total - 1 ? 'Finish' : 'Next →'}</button>
        </div>
        {isMobile && (
          <div style={{
            marginTop: T.spacing.sm, fontSize: '10px', color: C.textDim,
            textAlign: 'center',
          }}>
            Swipe left/right to navigate
          </div>
        )}
      </div>
    </>
  );
};
