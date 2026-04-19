// PlausiDen dashboard service worker — static-asset cache-first + HTML shell
// precache for offline navigation. Scope is the registering origin.
//
// c2-233 / #90: precache the HTML entry at install time so the SPA shell is
// available when the user visits offline (returning user → refresh on the
// train). Hashed JS/CSS/fonts still populate the cache on first fetch;
// they're immutable per Vite's filename contract so reading a stale one is
// harmless.
//
// Cache strategy:
//   - Navigation request: network-first, fall back to cached "/" HTML.
//   - Same-origin GET for fonts/images/css/js: cache-first, populate-on-miss.
//   - Everything else (API calls, POSTs, cross-origin): network only.
// API calls MUST NEVER be cached — stale facts would be worse than offline.

// c2-431 bump to force old-bundle eviction after the post-LLM pivot
// mid-flight refactors. The activate handler already drops any key that
// isn't the current CACHE_VERSION so this single string change wipes
// whatever stale build was serving.
const CACHE_VERSION = 'plausiden-v6';
const PRECACHE_URLS = ['/'];
const SAME_ORIGIN_STATIC = /\.(?:js|mjs|css|woff2?|ttf|otf|png|jpg|jpeg|svg|webp|ico)(?:\?.*)?$/i;

self.addEventListener('install', (event) => {
  self.skipWaiting();
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE_VERSION);
    // addAll is atomic — if any URL fails we skip the precache rather than
    // leaving a half-populated cache that would serve a broken shell.
    try { await cache.addAll(PRECACHE_URLS); }
    catch { /* offline install or blocked — on-demand fetch will still work */ }
  })());
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys();
    await Promise.all(keys.filter(k => k !== CACHE_VERSION).map(k => caches.delete(k)));
    await self.clients.claim();
  })());
});

// Return true for top-level HTML navigation requests. Covers both the
// explicit request.mode === 'navigate' path and the Accept-header fallback
// (older Safari without mode=navigate on document loads).
const isNavigationRequest = (req) => {
  if (req.mode === 'navigate') return true;
  const accept = req.headers.get('accept') || '';
  return accept.includes('text/html');
};

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;            // skip cross-origin (API on :3000)
  if (url.pathname.startsWith('/api/')) return;               // explicit safety: never cache API
  if (url.pathname.startsWith('/ws/')) return;                // websockets shouldn't reach here but be safe

  // Navigation: network-first so the user sees fresh HTML whenever online,
  // but fall back to the precached shell offline. The SPA then hydrates
  // from the shell and the previously-cached static chunks.
  if (isNavigationRequest(req)) {
    event.respondWith((async () => {
      const cache = await caches.open(CACHE_VERSION);
      try {
        const net = await fetch(req);
        if (net.ok) {
          // Keep the cached "/" fresh by snapshotting whichever URL the user
          // is on — SPA routes all resolve to the same HTML.
          cache.put('/', net.clone()).catch(() => { /* quota — drop */ });
        }
        return net;
      } catch {
        const hit = await cache.match('/');
        if (hit) return hit;
        return new Response('offline', { status: 503, statusText: 'Offline' });
      }
    })());
    return;
  }

  if (!SAME_ORIGIN_STATIC.test(url.pathname)) return;         // only cache static assets

  event.respondWith((async () => {
    const cache = await caches.open(CACHE_VERSION);
    const hit = await cache.match(req);
    if (hit) return hit;
    try {
      const res = await fetch(req);
      // Only cache successful opaque-safe responses.
      if (res.ok && res.status === 200) {
        cache.put(req, res.clone()).catch(() => { /* quota — drop */ });
      }
      return res;
    } catch (e) {
      // Offline + no cache — return a minimal synthesized response so the browser
      // surfaces a reasonable error rather than a generic network failure.
      return new Response('offline', { status: 503, statusText: 'Offline' });
    }
  })());
});
