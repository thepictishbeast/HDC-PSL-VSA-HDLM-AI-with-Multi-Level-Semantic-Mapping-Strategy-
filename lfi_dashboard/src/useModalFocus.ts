import { useEffect, useRef } from 'react';

// Minimal modal focus management: when `open` becomes true, move focus into the
// dialog container (first focusable descendant), then restore to the previous
// element on close. Not a full focus-trap (Tab cycling) — that's a bigger hook
// and harder to get right without testing; this is the 80/20 win.
//
// Usage:
//   const dialogRef = useRef<HTMLDivElement>(null);
//   useModalFocus(open, dialogRef);
//   <div ref={dialogRef} role='dialog' aria-modal>...</div>
export const useModalFocus = (open: boolean, ref: React.RefObject<HTMLElement>) => {
  const previousFocused = useRef<Element | null>(null);
  useEffect(() => {
    if (!open) return;
    previousFocused.current = document.activeElement;
    // Next microtask so the modal's inputs have mounted.
    queueMicrotask(() => {
      const node = ref.current;
      if (!node) return;
      const first = node.querySelector<HTMLElement>(
        'button, [href], input:not([type="hidden"]), textarea, select, [tabindex]:not([tabindex="-1"])'
      );
      (first ?? node).focus?.();
    });
    return () => {
      const prev = previousFocused.current as HTMLElement | null;
      prev?.focus?.();
    };
  }, [open, ref]);
};
