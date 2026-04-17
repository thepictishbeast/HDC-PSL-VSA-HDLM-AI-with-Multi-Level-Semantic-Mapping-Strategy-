import { useCallback, useEffect, useRef } from 'react';

// Auto-scroll a container to the bottom when the tracked count grows.
// Only scrolls on count-increase — never on re-renders caused by unrelated
// state changes. REGRESSION-GUARD: user complaint 2026-04-15 "conversation
// keeps scrolling randomly" was caused by scrolling on every render.
//
// Usage:
//   const endRef = useRef<HTMLDivElement>(null);
//   useAutoScroll(endRef, messages.length);
//   ...
//   <div ref={endRef} />
export const useAutoScroll = (endRef: React.RefObject<HTMLElement>, count: number) => {
  const prev = useRef(0);
  const scroll = useCallback(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [endRef]);
  useEffect(() => {
    if (count > prev.current) scroll();
    prev.current = count;
  }, [count, scroll]);
  return scroll;
};
