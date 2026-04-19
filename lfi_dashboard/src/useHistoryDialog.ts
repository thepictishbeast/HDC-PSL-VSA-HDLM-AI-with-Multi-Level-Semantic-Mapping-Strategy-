/**
 * useHistoryDialog — wire a boolean-controlled modal/dialog into the
 * browser history API so the Back button closes it instead of leaving
 * the SPA.
 *
 * Behavior:
 *   - When `isOpen` flips true: push a history state entry tagged with
 *     `key`. Install a popstate listener that calls `close()`.
 *   - When `isOpen` flips false while we own the top history entry:
 *     pop it with history.back() so forward/back stays balanced.
 *   - Multiple concurrent dialogs stack naturally: each pushes its own
 *     entry; Back closes them LIFO.
 *
 * Keep `close` stable (wrap in useCallback). Unstable `close` triggers a
 * re-install of the popstate listener on every render, which is harmless
 * but wasteful.
 */

import { useEffect, useRef } from 'react';

type Options = {
  enabled?: boolean;
};

export function useHistoryDialog(
  isOpen: boolean,
  close: () => void,
  key: string,
  opts: Options = {},
) {
  const pushedRef = useRef(false);
  const closeRef = useRef(close);
  closeRef.current = close;

  useEffect(() => {
    if (opts.enabled === false) return;
    if (typeof window === 'undefined') return;

    if (isOpen && !pushedRef.current) {
      try {
        window.history.pushState({ dialog: key, ts: Date.now() }, '');
        pushedRef.current = true;
      } catch { /* some iframes / sandboxes disallow — fall through silently */ }

      const onPopState = () => {
        if (!pushedRef.current) return;
        pushedRef.current = false;
        try { closeRef.current(); } catch { /* close handler should never throw */ }
      };
      window.addEventListener('popstate', onPopState);
      return () => window.removeEventListener('popstate', onPopState);
    }

    if (!isOpen && pushedRef.current) {
      pushedRef.current = false;
      try {
        const st: any = window.history.state;
        if (st && st.dialog === key) {
          window.history.back();
        }
      } catch { /* silent */ }
    }
    return;
  }, [isOpen, key, opts.enabled]);
}
