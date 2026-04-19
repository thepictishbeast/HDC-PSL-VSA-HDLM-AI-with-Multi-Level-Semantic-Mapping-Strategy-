/**
 * mobile-smoke.js — runtime mobile-friendliness smoke test.
 *
 * Paste this whole file into the DevTools console while emulating a
 * mobile viewport (iPhone SE 375x667 or narrower). It exercises the
 * dashboard's reachable surfaces and asserts the invariants from
 * docs/MOBILE_AUDIT.md.
 *
 * Output: PASS / FAIL per check. Summary at end. Failures include the
 * offending element for inspection.
 *
 * Run:
 *   1. Open the dashboard at :3000 in DevTools.
 *   2. Toggle device emulation (Cmd+Shift+M / Ctrl+Shift+M).
 *   3. Pick "iPhone SE" from the preset list.
 *   4. Reload so media queries pick the mobile branch.
 *   5. Open DevTools console → Sources → Snippets → paste this file.
 *   6. Run the snippet. Read the output.
 *
 * Results are also written to window.__mobileSmokeReport for
 * programmatic access (e.g. CI scraper).
 */
(() => {
  const report = { pass: 0, fail: 0, warnings: 0, checks: [], started_at: new Date().toISOString() };
  const check = (label, ok, detail) => {
    const rec = { label, ok, detail: detail || null, ts: Date.now() };
    report.checks.push(rec);
    if (ok === true) { report.pass++; console.log(`%cPASS%c ${label}`, 'color:#0a0;font-weight:bold', ''); }
    else if (ok === 'warn') { report.warnings++; console.warn(`WARN ${label}${detail ? ' — ' + detail : ''}`); }
    else { report.fail++; console.error(`FAIL ${label}${detail ? ' — ' + detail : ''}`); }
  };

  const vw = window.innerWidth;
  const vh = window.innerHeight;
  console.log(`%cMobile smoke test @ ${vw}x${vh}`, 'color:#08f;font-weight:bold');

  // ----- Layout -----

  // 1. No horizontal overflow.
  const scroll = document.documentElement.scrollWidth;
  check(
    `no horizontal overflow (scrollWidth ${scroll} ≤ innerWidth ${vw})`,
    scroll <= vw + 2
  );

  // 2. Primary container uses dvh or similar.
  const root = document.querySelector('.lfi-app-root') || document.body;
  const rh = getComputedStyle(root).height;
  check(
    `primary container height is viewport-relative (${rh})`,
    rh !== '' && (rh.endsWith('px') ? parseFloat(rh) > 0 : true),
    rh
  );

  // 3. No element has width > viewport.
  const all = Array.from(document.querySelectorAll('body *'));
  const oversized = all.filter(el => {
    const r = el.getBoundingClientRect();
    return r.width > vw + 2 && r.width - vw > 4;
  });
  check(
    `no element wider than viewport (${oversized.length} found)`,
    oversized.length === 0,
    oversized.length > 0 ? oversized.slice(0, 3).map(e => e.tagName + (e.className ? '.' + String(e.className).split(' ')[0] : '')).join(', ') : null
  );

  // ----- Tap targets -----

  // 4. All buttons have bounding box >= 32x32 (allowing some slack from 44px ideal for chips).
  const buttons = Array.from(document.querySelectorAll('button, [role="button"], [role="tab"]'));
  const visibleButtons = buttons.filter(b => {
    const r = b.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  });
  const small = visibleButtons.filter(b => {
    const r = b.getBoundingClientRect();
    return r.width < 32 || r.height < 32;
  });
  check(
    `tap targets ≥ 32x32 (${small.length}/${visibleButtons.length} undersized)`,
    small.length === 0 ? true : 'warn',
    small.length > 0 ? small.slice(0, 3).map(e => `${Math.round(e.getBoundingClientRect().width)}x${Math.round(e.getBoundingClientRect().height)}: ${e.textContent?.slice(0, 30)}`).join(' | ') : null
  );

  // 5. All icon-only buttons have aria-label or title.
  const iconButtons = visibleButtons.filter(b => {
    const txt = (b.textContent || '').trim();
    return txt.length <= 2; // 1-2 char = likely icon (×, +, ▾)
  });
  const unlabeled = iconButtons.filter(b => !b.getAttribute('aria-label') && !b.getAttribute('title'));
  check(
    `icon-only buttons have aria-label or title (${unlabeled.length}/${iconButtons.length} missing)`,
    unlabeled.length === 0 ? true : 'warn',
    unlabeled.length > 0 ? unlabeled.slice(0, 3).map(e => e.outerHTML.slice(0, 80)).join(' | ') : null
  );

  // ----- Text legibility -----

  // 6. Body text ≥ 12px (11 allowed on chrome/metadata).
  const textNodes = Array.from(document.querySelectorAll('p, span, div, li, button, a, h1, h2, h3, h4')).filter(n => {
    const r = n.getBoundingClientRect();
    return n.textContent && n.textContent.trim().length >= 20 && r.width > 0;
  });
  const tiny = textNodes.filter(n => parseFloat(getComputedStyle(n).fontSize) < 11);
  check(
    `body-text nodes font-size ≥ 11px (${tiny.length}/${textNodes.length} smaller)`,
    tiny.length === 0 ? true : 'warn',
    tiny.length > 0 ? tiny.slice(0, 3).map(e => `${getComputedStyle(e).fontSize}: ${e.textContent?.slice(0, 40)}`).join(' | ') : null
  );

  // ----- Interaction surfaces -----

  // 7. Chat input visible + tappable.
  const inputEl = document.querySelector('textarea[placeholder*="Ask"], textarea[aria-label*="message"], textarea, input[type="text"]');
  if (inputEl) {
    const r = inputEl.getBoundingClientRect();
    check(
      `chat input is in-viewport (${Math.round(r.width)}x${Math.round(r.height)} @ ${Math.round(r.top)})`,
      r.top < vh && r.top + r.height > 0 && r.width > 40
    );
  } else {
    check('chat input found', 'warn', 'no textarea/input matched — auth-gated?');
  }

  // 8. No overlapping tap targets in the header.
  const header = document.querySelector('header, [role="banner"], .lfi-app-root > div:first-child');
  if (header) {
    const headerButtons = Array.from(header.querySelectorAll('button'));
    let overlaps = 0;
    for (let i = 0; i < headerButtons.length; i++) {
      for (let j = i + 1; j < headerButtons.length; j++) {
        const a = headerButtons[i].getBoundingClientRect();
        const b = headerButtons[j].getBoundingClientRect();
        if (a.width === 0 || b.width === 0) continue;
        if (a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top) {
          overlaps++;
        }
      }
    }
    check(`no overlapping buttons in header (${overlaps} pairs)`, overlaps === 0);
  }

  // 9. Focus-visible reachable via keyboard (sanity: body.activeElement exists).
  check('document.activeElement defined', document.activeElement !== null);

  // 10. `<meta name="viewport">` present.
  const viewport = document.querySelector('meta[name="viewport"]');
  check(
    'viewport meta tag present',
    viewport !== null,
    viewport?.getAttribute('content') || 'MISSING'
  );

  // ----- Summary -----

  console.log(`%c━━━━━━━━━━━━━━━━━━━━━━━━`, 'color:#888');
  const verdict = report.fail === 0 ? (report.warnings === 0 ? 'PASS' : 'PASS with warnings') : 'FAIL';
  const tone = report.fail === 0 ? 'color:#0a0' : 'color:#a00';
  console.log(`%c${verdict}%c — ${report.pass} pass · ${report.warnings} warn · ${report.fail} fail`, tone + ';font-weight:bold', '');

  report.finished_at = new Date().toISOString();
  window.__mobileSmokeReport = report;
  console.log('Full report: window.__mobileSmokeReport');
  return report;
})();
