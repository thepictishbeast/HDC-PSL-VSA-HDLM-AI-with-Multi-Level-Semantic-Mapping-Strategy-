# Mobile-Friendly Audit

This doc captures the invariants the PlausiDen dashboard MUST hold on mobile
viewports (320 × 568 through 414 × 896) and how to verify them. Three layers:

1. **Static lint** (`scripts/mobile-lint.sh`) — grep-based checks on source.
2. **Browser smoke harness** (`scripts/mobile-smoke.js`) — paste into DevTools
   console at mobile-emulation sizes; prints PASS / FAIL per invariant.
3. **Manual checklist** (below) — tap-through gates a human confirms
   before shipping a visible UI change.

Run the two automated layers every time you edit a layout-heavy file.

---

## Design invariants

All hold at 320 × 568 (iPhone SE portrait) and 375 × 667 (iPhone 8 portrait).

### Layout
- **No horizontal overflow**: `document.documentElement.scrollWidth <= window.innerWidth + 1`.
- **No fixed `width: Npx`** ≥ 200px on flex children inside a `.lfi-app-root`
  or modal without a paired `minWidth: 0` / `flex: 1 1 N` / `maxWidth: Npx`
  fallback. The one exception is images (`<img>` / `<canvas>` / SVG).
- **`dvh`-based heights** on fullscreen surfaces (`.lfi-app-root`, modal
  dialogs). `vh` is tolerated as a fallback beneath `dvh` for older
  browsers but never used alone for primary layout containers.
- **`flexWrap: 'wrap'`** on every horizontal flex row whose children sum to
  > 200px of intrinsic width (header rows, toolbars, chip strips).

### Targets
- **Tap target ≥ 44 × 44 px** for any `role=button`, `<button>`, or
  `<a onClick>`. Small (< 44px) icon-only buttons must have
  `padding` bumping the effective hit area above 44px.
- **Label text ≥ 13px** on primary actions; 10px is allowed for secondary
  metadata (chips, timestamps, counts) but never for tappable labels.

### Text
- **`wordBreak: 'break-word'`** on any span with user-supplied content
  inside a `flex: 1` parent (fact values, error messages, tooltips).
- **`minWidth: 0`** on flex children whose content could exceed the row;
  without it, browsers keep flex children at their intrinsic width and
  push siblings off-screen.
- **Tooltips available via keyboard**: `title` attributes on desktop map
  to pointer-hover only; mobile fallback is either a visible hint or a
  tap-expand state (pattern: see Drift cards where thresholds read via
  title + jumpTo arrow as visible cue).

### Interaction
- **Touch-friendly action buttons**: no `:hover`-only affordances.
  Active/inactive/pressed states must be visible at touch-only too
  (cursor pointer, underline, border).
- **Scroll containers preserve page scroll**: inner `overflow-y: auto`
  panels (Drift sparkline row, audit-chain expand) use bounded heights
  (`maxHeight: 220px`) so the outer page can still scroll.

### Performance
- **First contentful paint < 2s** on a 3G throttle against the 493KB
  production bundle. Monitor via DevTools Lighthouse.
- **No layout shift > 0.1 CLS** during initial load. The sidebar + header
  should size themselves before content arrives.

---

## Manual pre-ship checklist

For every UI PR that touches layout, tap through the following in DevTools
device-mode at **iPhone SE (375 × 667)** and **Galaxy S5 (360 × 640)**:

- [ ] Chat: type a message, verify Send button tappable without zoom.
- [ ] Chat: topic chip + modules pill + predicted-chip row wraps rather than clipping.
- [ ] Sidebar: conversation list opens/closes with the hamburger; items tappable.
- [ ] Fact popover: open a `[fact:KEY]` chip; popover fits viewport width, all
      sections (Versions/Contradictions/Inbound/Outbound/Translations/Verdict)
      render without horizontal scroll.
- [ ] Fact popover footer: FSRS rating row + Verify-now + add-translation
      form all tappable; Save is reachable without layout reflow hiding it.
- [ ] Admin Modal: Integrity banner wraps its long alert message; chevron
      remains visible on the right.
- [ ] Admin → Tokens: issue form fields reflow to stack; preset chips
      wrap; capability filter chips wrap.
- [ ] Admin → Audit chain expand: 5-column grid renders without sideways scroll.
- [ ] Admin → Proof: 4 buckets wrap to 2×2 on narrow.
- [ ] Classroom → Drift: 8 cards reflow to single column; health chip fits.
- [ ] Classroom → Ledger: row `side_a ↔ side_b` stacks when narrower than ~400px.
- [ ] Classroom → Runs: progress bar + readout fit on one line or wrap cleanly.
- [ ] Cmd+K palette: prompt dialogs (window.prompt native) keep full keyboard working.
- [ ] Long-press / paste / select-text all work on mobile without intercepting.

---

## Known-good patterns (copy-paste)

```tsx
// Horizontal flex row with chips that might overflow
<div style={{ display: 'flex', gap: T.spacing.sm, flexWrap: 'wrap', alignItems: 'center' }}>

// Flex child with text that should shrink + ellipsize
<span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>

// Flex child with text that should shrink + wrap
<span style={{ flex: '1 1 160px', minWidth: 0, wordBreak: 'break-word' }}>

// Sidebar / modal width with mobile cap
const W = Math.min(320, window.innerWidth - 16);  // 8px gutter each side

// Responsive metric grid
display: 'grid',
gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
gap: T.spacing.md,

// Input field that fits mobile but caps on desktop
padding: '6px 10px', flex: '1 1 110px', maxWidth: '150px', minWidth: 0,

// Button icon that must not get crushed
style={{ flexShrink: 0 }}
```

---

## Anti-patterns (always caught by `mobile-lint.sh`)

```tsx
// ❌ Fixed pixel width on a flex child without fallback
<input style={{ width: '180px' }}>

// ❌ `vh` for primary layout (mobile keyboard hides bottom)
<div style={{ height: '100vh' }}>

// ❌ Horizontal flex row without wrap + children exceeding container
<div style={{ display: 'flex', gap: '8px' }}>{manyChips}</div>

// ❌ Icon button without padding bumping to 44px
<button><svg width='12' height='12'/></button>

// ❌ Truncated text without title fallback
<span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{userText}</span>
// (title={userText} missing — user can't see the full content on touch)
```
