import React, { useRef, useState } from 'react';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

// Activity & Logs modal: 3 tabs (server chat log, local UI events, system snapshot).
// Kept as a pure render — parent owns all of the data + fetch triggers.

export interface ActivityChatEntry {
  ts?: number;
  tier?: string;
  intent?: string;
  confidence?: number;
  user?: string;
  reply?: string;
}

export interface ActivityLocalEvent {
  t: number;
  kind: string;
  data?: any;
}

export interface ActivityQosCheck {
  name: string;
  passed: boolean;
  value: string;
}

export type ActivityTab = 'chat' | 'events' | 'system';

export interface ActivityModalProps {
  C: any;
  tab: ActivityTab;
  onTabChange: (t: ActivityTab) => void;
  onClose: () => void;
  serverChatLog: ActivityChatEntry[];
  chatLogError: string | null;
  chatLogFetchedAt: number | null;
  onRefreshChatLog?: () => void;
  localEvents: ActivityLocalEvent[];
  isConnected: boolean;
  currentTier: string;
  thermalThrottled: boolean;
  ramLabel: string;
  cpuTempC: number;
  factsLabel: string;
  conceptsLabel: string;
  logicDensity: number;
  qosReport: { checks?: ActivityQosCheck[] } | null;
  onRefreshQos: () => void;
  // c2-433 / task 253: clear all session-local UI events. Wired from
  // App-level localEvents state. Confirm path lives here so the modal
  // owns the destructive flow.
  onClearLocalEvents?: () => void;
  onRefreshFacts: () => void;
}

export const ActivityModal: React.FC<ActivityModalProps> = ({
  C, tab, onTabChange, onClose,
  serverChatLog, chatLogError, chatLogFetchedAt, onRefreshChatLog,
  localEvents,
  isConnected, currentTier, thermalThrottled, ramLabel, cpuTempC, factsLabel, conceptsLabel, logicDensity,
  qosReport, onRefreshQos, onRefreshFacts, onClearLocalEvents,
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  // c2-433 / task 252: events tab filtering. eventKindFilter narrows to a
  // single chip-selected kind; eventQuery is free-text matched against
  // both kind + JSON-stringified data. Local state — resets on modal
  // close so each open starts fresh.
  const [eventKindFilter, setEventKindFilter] = useState<string | null>(null);
  const [eventQuery, setEventQuery] = useState<string>('');
  // c2-433: Copy-JSON export feedback for the activity snapshot. 2s
  // Copy → Copied ✓ flip matching the other export surfaces.
  const [copiedAt, setCopiedAt] = useState<number>(0);
  return (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: T.z.palette,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: T.spacing.lg,
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-activity-title'
      onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '900px', height: '82dvh',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
        display: 'flex', flexDirection: 'column', overflow: 'hidden',
        boxShadow: T.shadows.modal,
      }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
      }}>
        <h2 id='scc-activity-title' style={{ margin: 0, fontSize: '15px', fontWeight: T.typography.weightBlack, letterSpacing: '0.12em', textTransform: 'uppercase', color: C.text }}>
          Activity &amp; Logs
        </h2>
        <div style={{ display: 'flex', gap: T.spacing.sm, alignItems: 'center' }}>
          {/* c2-433: Activity snapshot Copy-JSON export. Bundles local UI
              events + server chat log into one clipboard paste. Matches
              the Drift/Ledger/Runs/KB/Library pattern. */}
          <button
            disabled={localEvents.length === 0 && serverChatLog.length === 0}
            onClick={async () => {
              const payload = {
                exported_at: new Date().toISOString(),
                events: localEvents,
                chat_log: serverChatLog,
              };
              try {
                await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
                setCopiedAt(Date.now());
                window.setTimeout(() => setCopiedAt(0), 2000);
              } catch { /* clipboard blocked */ }
            }}
            title={copiedAt > 0 ? 'Copied to clipboard' : `Copy ${localEvents.length} UI events + ${serverChatLog.length} log entries as JSON`}
            style={{
              background: copiedAt > 0 ? `${C.green}18` : 'transparent',
              border: `1px solid ${copiedAt > 0 ? C.green : C.borderSubtle}`,
              color: copiedAt > 0 ? C.green : C.textMuted,
              borderRadius: T.radii.sm,
              cursor: (localEvents.length === 0 && serverChatLog.length === 0) ? 'not-allowed' : 'pointer',
              padding: '4px 10px', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: T.typography.weightSemibold,
              opacity: (localEvents.length === 0 && serverChatLog.length === 0) ? 0.5 : 1,
              whiteSpace: 'nowrap',
            }}>{copiedAt > 0 ? 'Copied \u2713' : 'Copy'}</button>
          <button onClick={onClose} aria-label='Close activity'
            style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
            {'\u2715'}
          </button>
        </div>
      </div>
      {(() => {
        // c2-433 / task 271: WAI-ARIA tablist kbd nav (Arrow/Home/End +
        // roving tabindex) per #178. Same pattern as SettingsModal.
        const TABS: ReadonlyArray<{ id: ActivityTab; label: string }> = [
          { id: 'chat', label: `Conversations (${serverChatLog.length})` },
          { id: 'events', label: `UI events (${localEvents.length})` },
          { id: 'system', label: 'System' },
        ];
        const onTabKey = (e: React.KeyboardEvent) => {
          if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight' && e.key !== 'Home' && e.key !== 'End') return;
          e.preventDefault();
          const idx = TABS.findIndex(t => t.id === tab);
          let next = idx;
          if (e.key === 'ArrowLeft') next = (idx - 1 + TABS.length) % TABS.length;
          else if (e.key === 'ArrowRight') next = (idx + 1) % TABS.length;
          else if (e.key === 'Home') next = 0;
          else if (e.key === 'End') next = TABS.length - 1;
          onTabChange(TABS[next].id);
        };
        return (
          <div role='tablist' aria-label='Activity sections' onKeyDown={onTabKey}
            style={{ display: 'flex', gap: T.spacing.xs, padding: '8px 12px', borderBottom: `1px solid ${C.borderSubtle}` }}>
            {TABS.map(t => {
              const active = tab === t.id;
              return (
                <button key={t.id} onClick={() => onTabChange(t.id)}
                  role='tab' aria-selected={active}
                  tabIndex={active ? 0 : -1}
                  style={{
                    padding: '8px 14px', fontSize: T.typography.sizeSm,
                    background: active ? C.accentBg : 'transparent',
                    border: `1px solid ${active ? C.accentBorder : 'transparent'}`,
                    color: active ? C.accent : C.textMuted,
                    borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                  }}>{t.label}</button>
              );
            })}
          </div>
        );
      })()}
      <div role='tabpanel' aria-label={tab} style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
        {tab === 'chat' && (
          <>
            {serverChatLog.length === 0 && (
              chatLogError ? (
                <div role='alert' style={{ fontSize: T.typography.sizeMd, padding: T.spacing.lg, background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: T.radii.lg, color: C.red, lineHeight: 1.5 }}>
                  <div style={{ fontWeight: 700, marginBottom: T.spacing.xs }}>Could not load chat log</div>
                  <div style={{ fontSize: T.typography.sizeSm, opacity: 0.9 }}>{chatLogError}</div>
                  <div style={{ fontSize: T.typography.sizeXs, marginTop: T.spacing.sm, color: C.textMuted }}>
                    {chatLogError.toLowerCase().includes('auth') ? 'Server is gating this endpoint; the passwordless-mode flag may be off-sync after a restart.' : 'Check that the Rust backend is running on port 3000 and try again.'}
                  </div>
                  {onRefreshChatLog && (
                    <button onClick={onRefreshChatLog}
                      style={{
                        marginTop: '10px', padding: '6px 14px', fontSize: T.typography.sizeSm, fontWeight: 700,
                        background: C.accentBg, border: `1px solid ${C.accentBorder}`, color: C.accent,
                        borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                      }}>Retry</button>
                  )}
                </div>
              ) : chatLogFetchedAt ? (
                <div style={{ color: C.textMuted, fontSize: T.typography.sizeMd, padding: '20px', textAlign: 'center' }}>
                  No logged turns yet. Send a message to populate.
                  <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '6px' }}>
                    Last checked {new Date(chatLogFetchedAt).toLocaleTimeString()}
                  </div>
                </div>
              ) : (
                <div style={{ color: C.textMuted, fontSize: T.typography.sizeMd, padding: '20px', textAlign: 'center' }}>
                  Loading chat log…
                </div>
              )
            )}
            {serverChatLog.slice().reverse().map((e, i) => (
              <div key={i} style={{
                padding: '12px 14px', marginBottom: '8px',
                background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: '10px',
              }}>
                <div style={{ fontSize: '10px', color: C.textDim, marginBottom: '6px', display: 'flex', gap: T.spacing.md }}>
                  <span>{new Date((e.ts || 0) * 1000).toLocaleString()}</span>
                  <span style={{ color: C.accent }}>{e.tier}</span>
                  <span style={{ color: C.purple }}>{(e.intent || '').split('{')[0]}</span>
                  <span style={{ color: C.green }}>{e.confidence ? `${(e.confidence * 100).toFixed(0)}%` : ''}</span>
                </div>
                <div style={{ fontSize: T.typography.sizeMd, color: C.accent, marginBottom: T.spacing.xs }}>
                  <strong>User:</strong> {e.user}
                </div>
                <div style={{ fontSize: T.typography.sizeMd, color: C.text, whiteSpace: 'pre-wrap' }}>
                  <strong style={{ color: C.green }}>AI:</strong> {e.reply}
                </div>
              </div>
            ))}
          </>
        )}
        {tab === 'events' && (() => {
          // c2-433 / task 252: kind-filter chip group + search input. Long
          // event logs (1000s of entries) are unreadable without filters;
          // chips list every distinct kind seen + counts so users can
          // narrow to just "feedback_correct" or "chat_modules_used"
          // without manually scanning. Free-text search matches against
          // both kind and the JSON-stringified data.
          const kindCounts = new Map<string, number>();
          for (const e of localEvents) kindCounts.set(e.kind, (kindCounts.get(e.kind) || 0) + 1);
          const allKinds = Array.from(kindCounts.entries()).sort((a, b) => b[1] - a[1]);
          const filtered = localEvents.filter(e => {
            if (eventKindFilter && e.kind !== eventKindFilter) return false;
            if (!eventQuery.trim()) return true;
            const q = eventQuery.toLowerCase();
            return e.kind.toLowerCase().includes(q) || (e.data ? JSON.stringify(e.data).toLowerCase().includes(q) : false);
          });
          return (
            <>
              {localEvents.length === 0 ? (
                <div style={{ color: C.textMuted, fontSize: T.typography.sizeMd, padding: '20px', textAlign: 'center' }}>
                  No UI events captured yet.
                </div>
              ) : (
                <>
                  <div style={{ display: 'flex', gap: T.spacing.sm, marginBottom: T.spacing.sm, flexWrap: 'wrap' }}>
                    <input type='search' value={eventQuery}
                      onChange={(ev) => setEventQuery(ev.target.value)}
                      // c2-433 / task 282b: same Esc-clears-filter step-down
                      // pattern as KnowledgeBrowser. First Esc clears the
                      // free-text query (or the kind chip if no query);
                      // second Esc propagates to global to close the modal.
                      onKeyDown={(ev) => {
                        if (ev.key !== 'Escape') return;
                        if (eventQuery) {
                          ev.preventDefault();
                          ev.stopPropagation();
                          setEventQuery('');
                        } else if (eventKindFilter) {
                          ev.preventDefault();
                          ev.stopPropagation();
                          setEventKindFilter(null);
                        }
                      }}
                      placeholder='Filter events…' aria-label='Filter events'
                      style={{
                        flex: 1, minWidth: '160px', padding: '6px 10px',
                        background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
                        borderRadius: T.radii.md, color: C.text, fontFamily: 'inherit',
                        fontSize: T.typography.sizeSm, outline: 'none', boxSizing: 'border-box',
                      }} />
                    {eventKindFilter && (
                      <button onClick={() => setEventKindFilter(null)}
                        style={{
                          padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                          background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                          color: C.textMuted, borderRadius: T.radii.pill, cursor: 'pointer',
                          fontFamily: 'inherit',
                        }}>{'\u2715'} {eventKindFilter}</button>
                    )}
                    {/* c2-433 / task 253: destructive clear with native confirm
                        — lives in the modal's filter row so it's grouped with
                        the other event-management controls. Hidden when no
                        events to clear. */}
                    {onClearLocalEvents && localEvents.length > 0 && (
                      <button
                        onClick={() => {
                          if (window.confirm(`Clear ${localEvents.length} session-local UI event${localEvents.length === 1 ? '' : 's'}? Server-side chat log is unaffected.`)) {
                            onClearLocalEvents();
                            setEventKindFilter(null);
                            setEventQuery('');
                          }
                        }}
                        title='Clear all session-local UI events'
                        style={{
                          padding: '4px 10px', fontSize: T.typography.sizeXs, fontWeight: T.typography.weightBold,
                          background: 'transparent', border: `1px solid ${C.borderSubtle}`,
                          color: C.red, borderRadius: T.radii.md, cursor: 'pointer',
                          fontFamily: 'inherit',
                        }}>Clear log</button>
                    )}
                  </div>
                  <div style={{ display: 'flex', gap: '4px', marginBottom: T.spacing.md, flexWrap: 'wrap' }}>
                    {allKinds.slice(0, 12).map(([k, n]) => {
                      const active = eventKindFilter === k;
                      return (
                        <button key={k} onClick={() => setEventKindFilter(active ? null : k)}
                          aria-pressed={active}
                          style={{
                            padding: '2px 8px', fontSize: '10px', fontWeight: T.typography.weightBold,
                            background: active ? C.accentBg : 'transparent',
                            border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                            color: active ? C.accent : C.textMuted,
                            borderRadius: T.radii.pill, cursor: 'pointer',
                            fontFamily: T.typography.fontMono,
                          }}>{k} <span style={{ opacity: 0.6 }}>({n})</span></button>
                      );
                    })}
                  </div>
                  {filtered.length === 0 ? (
                    <div style={{ color: C.textMuted, fontSize: T.typography.sizeSm, padding: '20px', textAlign: 'center', fontStyle: 'italic' }}>
                      No events match the current filter.
                    </div>
                  ) : (
                    filtered.slice().reverse().map((e, i) => (
                      <div key={i} style={{
                        display: 'flex', gap: '12px', padding: '6px 10px',
                        borderBottom: `1px solid ${C.borderSubtle}`, fontSize: T.typography.sizeSm,
                      }}>
                        <span style={{ color: C.textDim, minWidth: '90px' }}>
                          {new Date(e.t).toLocaleTimeString()}
                        </span>
                        <span style={{ color: C.accent, minWidth: '140px', fontWeight: 700, fontFamily: T.typography.fontMono }}>{e.kind}</span>
                        <span style={{ color: C.textSecondary, flex: 1, fontFamily: T.typography.fontMono, fontSize: '11px', wordBreak: 'break-word' }}>
                          {(() => {
                            // c2-433 / task 275 + 276: friendlier rendering
                            // for common event-kind shapes. Falls back to
                            // JSON for everything else.
                            if (!e.data) return '';
                            const d = e.data as any;
                            if (e.kind === 'chat_modules_used' && Array.isArray(d.modules)) {
                              return d.modules.join(' · ');
                            }
                            if (e.kind === 'feedback_positive' && d.msgId != null) return `msg ${d.msgId}`;
                            if (e.kind === 'feedback_negative' && d.msgId != null) return `msg ${d.msgId}${d.category ? ` · ${d.category}` : ''}`;
                            if (e.kind === 'feedback_correct' && d.msgId != null) return `msg ${d.msgId}${typeof d.len === 'number' ? ` · ${d.len} chars` : ''}`;
                            if (e.kind === 'fact_key_opened' && d.key) return d.key;
                            if (e.kind === 'message_copied' && d.role) return `${d.role}${typeof d.length === 'number' ? ` · ${d.length} chars` : ''}${d.via ? ` · via ${d.via}` : ''}`;
                            if (e.kind === 'chat_retry' && typeof d.len === 'number') return `${d.len} chars`;
                            if (e.kind === 'chat_stop' && typeof d.elapsed === 'number') return `after ${d.elapsed}s`;
                            if (e.kind === 'theme_cycled' && d.theme) return `→ ${d.theme}`;
                            if (e.kind === 'slash_cmd' && d.cmd) return d.cmd;
                            if (e.kind === 'tier_switched' && d.tier) return `→ ${d.tier}`;
                            if (e.kind === 'message_sent' && typeof d.length === 'number') return `${d.length} chars${d.tier ? ` · ${d.tier}` : ''}${d.skill && d.skill !== 'chat' ? ` · ${d.skill}` : ''}`;
                            if (e.kind === 'message_edited' && typeof d.originalLen === 'number' && typeof d.newLen === 'number') return `${d.originalLen} → ${d.newLen} chars`;
                            if (e.kind === 'code_copied' && d.lang) return `${d.lang}${typeof d.length === 'number' ? ` · ${d.length} chars` : ''}`;
                            if (e.kind === 'new_conversation') return d.incognito ? 'incognito' : '';
                            if (e.kind === 'delete_conversation' && typeof d.messages === 'number') return `${d.messages} msgs`;
                            if ((e.kind === 'toggle_pinned' || e.kind === 'toggle_starred' || e.kind === 'toggle_archived') && typeof d['nowPinned'] !== 'undefined') return d.nowPinned ? 'on' : 'off';
                            if (e.kind === 'toggle_starred' && typeof d.nowStarred !== 'undefined') return d.nowStarred ? 'on' : 'off';
                            if (e.kind === 'toggle_archived' && typeof d.nowArchived !== 'undefined') return d.nowArchived ? 'on' : 'off';
                            if (e.kind === 'msg_queue_drained' && typeof d.count === 'number') return `${d.count} msg${d.count === 1 ? '' : 's'}`;
                            if (e.kind === 'cmd_palette_run' && d.id) return d.id;
                            if ((e.kind === 'bulk_delete_archived' || e.kind === 'bulk_unarchive') && typeof d.count === 'number') return `${d.count} convo${d.count === 1 ? '' : 's'}`;
                            return JSON.stringify(d);
                          })()}
                        </span>
                      </div>
                    ))
                  )}
                </>
              )}
            </>
          );
        })()}
        {tab === 'system' && (
          <div style={{ fontSize: T.typography.sizeSm, color: C.textSecondary }}>
            <div style={{ marginBottom: '12px', display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: T.spacing.md }}>
              {[
                ['Connection', isConnected ? 'LIVE' : 'DOWN'],
                ['Tier', currentTier],
                ['Throttled', thermalThrottled ? 'YES' : 'NO'],
                ['RAM', ramLabel],
                ['CPU temp', `${cpuTempC.toFixed(0)}\u00B0C`],
                ['Facts', factsLabel],
                ['Concepts', conceptsLabel],
                ['Logic density', logicDensity.toFixed(3)],
              ].map(([k, v]) => (
                <div key={k} style={{
                  display: 'flex', justifyContent: 'space-between',
                  padding: '8px 12px',
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.lg,
                }}>
                  <span style={{ color: C.textMuted }}>{k}</span>
                  <span style={{ color: C.text, fontWeight: 700 }}>{v}</span>
                </div>
              ))}
            </div>
            <div style={{ display: 'flex', gap: '8px' }}>
              <button onClick={onRefreshQos}
                style={{
                  padding: '8px 14px', fontSize: T.typography.sizeSm, background: C.accentBg,
                  border: `1px solid ${C.accentBorder}`, color: C.accent,
                  borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                }}>Refresh QoS</button>
              <button onClick={onRefreshFacts}
                style={{
                  padding: '8px 14px', fontSize: T.typography.sizeSm, background: C.purpleBg,
                  border: `1px solid ${C.purpleBorder}`, color: C.purple,
                  borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                }}>Refresh facts</button>
              {/* c2-433 / task 266: copy-to-clipboard the system snapshot
                  for paste-into-bug-report. Includes the same fields shown
                  in the grid above + UA + viewport + timestamp. Useful when
                  filing issues without manually transcribing each row. */}
              <button onClick={(e) => {
                const lines = [
                  `PlausiDen AI debug snapshot — ${new Date().toISOString()}`,
                  `Connection: ${isConnected ? 'LIVE' : 'DOWN'}`,
                  `Tier: ${currentTier}`,
                  `Throttled: ${thermalThrottled ? 'YES' : 'NO'}`,
                  `RAM: ${ramLabel}`,
                  `CPU temp: ${cpuTempC.toFixed(0)}\u00B0C`,
                  `Facts: ${factsLabel}`,
                  `Concepts: ${conceptsLabel}`,
                  `Logic density: ${logicDensity.toFixed(3)}`,
                  `User-agent: ${typeof navigator !== 'undefined' ? navigator.userAgent : 'n/a'}`,
                  `Viewport: ${typeof window !== 'undefined' ? `${window.innerWidth}x${window.innerHeight}` : 'n/a'}`,
                ];
                try { navigator.clipboard.writeText(lines.join('\n')); } catch { /* clipboard blocked */ }
                const btn = e.currentTarget;
                const orig = btn.textContent;
                btn.textContent = 'Copied';
                btn.style.color = C.green;
                window.setTimeout(() => { btn.textContent = orig; btn.style.color = C.textMuted; }, 1200);
              }}
                title='Copy system snapshot to clipboard for bug reports'
                style={{
                  padding: '8px 14px', fontSize: T.typography.sizeSm, background: 'transparent',
                  border: `1px solid ${C.borderSubtle}`, color: C.textMuted,
                  borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                }}>Copy debug</button>
            </div>
            {qosReport && (
              <div style={{ marginTop: '14px', display: 'flex', flexDirection: 'column', gap: T.spacing.xs }}>
                {qosReport.checks?.map((c, i) => (
                  <div key={i} style={{
                    display: 'flex', justifyContent: 'space-between',
                    padding: '6px 10px', borderRadius: T.radii.md,
                    background: c.passed ? C.greenBg : C.redBg,
                    border: `1px solid ${c.passed ? C.greenBorder : C.redBorder}`,
                  }}>
                    <span>{c.name}</span>
                    <span style={{ color: c.passed ? C.green : C.red, fontWeight: 700 }}>{c.value}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  </div>
  );
};
