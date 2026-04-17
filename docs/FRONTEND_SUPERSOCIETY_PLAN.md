# PlausiDen AI — Supersociety Frontend Architecture Plan

## Current State
- React 18 + TypeScript + Vite
- Single-file App.tsx (4600 lines)
- Inline styles (no CSS framework)
- 7 theme palettes, responsive design
- WebSocket chat + REST API

## Phase 1: Component Decomposition (No Framework Change)
Priority: HIGH — immediate quality improvement without migration risk.

1. **Split App.tsx into components:**
   - `ChatView.tsx` (message list, input, slash menu)
   - `Sidebar.tsx` (conversation list, search, controls)
   - `Settings.tsx` (tabbed modal)
   - `CommandPalette.tsx` (Cmd+K)
   - `ThemeProvider.tsx` (theme context + palettes)
   - `TelemetryCards.tsx` (system stats)
   - `MessageBubble.tsx` (individual message rendering)

2. **Extract shared styles to design tokens:**
   - `tokens.ts` (colors, spacing, typography, shadows, motion)
   - Use CSS-in-JS (styled-components or Emotion) or Tailwind CSS

3. **Add state management:**
   - Zustand or Jotai for global state (conversations, settings, theme)
   - React Query for server state (API caching, revalidation)

## Phase 2: Performance (Medium Priority)
1. **Virtualized message list** (react-virtuoso) for long conversations
2. **Code splitting** — lazy load Settings, CommandPalette, KnowledgeBrowser
3. **Service Worker** for offline support + push notifications
4. **WebSocket reconnection** with exponential backoff

## Phase 3: WASM Integration (Long-Term)
1. **Rust→WASM bridge** for client-side HDC operations:
   - Vector similarity search in browser
   - Local encryption/decryption
   - Offline knowledge queries against cached facts
2. **wasm-bindgen** + **wasm-pack** toolchain
3. Shared types between Rust backend and WASM frontend

## Phase 4: Desktop Shell (Tauri v2)
- Already scaffolded at `/root/LFI/plausiden-desktop/`
- Tauri v2 for native window, system tray, file system access
- WebView renders the React app
- Rust backend runs in-process (no HTTP needed)

## Decision: Why Not Full WASM Framework?
- Leptos/Yew/Dioxus are less mature than React ecosystem
- Hot-reload, DevTools, component libraries all weaker
- React + WASM bridge gives best of both worlds
- Can migrate individual components to WASM incrementally
