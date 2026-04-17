import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import { AppErrorBoundary } from './AppErrorBoundary'

// Register the service worker in production only. Dev builds skip it — Vite's
// HMR fights with any SW caching of /src/*.tsx, and we don't want half-stale
// modules while editing.
if (import.meta.env.PROD && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => { /* non-fatal */ });
  });
}

// c2-317: wrap the root so a throw during App's initial render (before App's
// own scoped boundaries mount) doesn't fall off the world with a blank page.
// AppErrorBoundary uses hardcoded dark-palette fallbacks because it may be
// shown before any theme loads.
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <AppErrorBoundary>
      <App />
    </AppErrorBoundary>
  </React.StrictMode>,
)
