import React from 'react';

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
  onRefreshFacts: () => void;
}

export const ActivityModal: React.FC<ActivityModalProps> = ({
  C, tab, onTabChange, onClose,
  serverChatLog, chatLogError, chatLogFetchedAt,
  localEvents,
  isConnected, currentTier, thermalThrottled, ramLabel, cpuTempC, factsLabel, conceptsLabel, logicDensity,
  qosReport, onRefreshQos, onRefreshFacts,
}) => (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: 220,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: '16px',
    }}>
    <div onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '900px', height: '82vh',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
        display: 'flex', flexDirection: 'column', overflow: 'hidden',
        boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
      }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 20px', borderBottom: `1px solid ${C.borderSubtle}`,
      }}>
        <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 800, letterSpacing: '0.12em', textTransform: 'uppercase', color: C.text }}>
          Activity &amp; Logs
        </h2>
        <button onClick={onClose}
          style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
          {'\u2715'}
        </button>
      </div>
      <div style={{ display: 'flex', gap: '4px', padding: '8px 12px', borderBottom: `1px solid ${C.borderSubtle}` }}>
        {([
          { id: 'chat', label: `Conversations (${serverChatLog.length})` },
          { id: 'events', label: `UI events (${localEvents.length})` },
          { id: 'system', label: 'System' },
        ] as const).map(t => (
          <button key={t.id} onClick={() => onTabChange(t.id)}
            style={{
              padding: '8px 14px', fontSize: '12px',
              background: tab === t.id ? C.accentBg : 'transparent',
              border: `1px solid ${tab === t.id ? C.accentBorder : 'transparent'}`,
              color: tab === t.id ? C.accent : C.textMuted,
              borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
            }}>{t.label}</button>
        ))}
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
        {tab === 'chat' && (
          <>
            {serverChatLog.length === 0 && (
              chatLogError ? (
                <div style={{ fontSize: '13px', padding: '16px', background: C.redBg, border: `1px solid ${C.redBorder}`, borderRadius: '8px', color: C.red, lineHeight: 1.5 }}>
                  <div style={{ fontWeight: 700, marginBottom: '4px' }}>Could not load chat log</div>
                  <div style={{ fontSize: '12px', opacity: 0.9 }}>{chatLogError}</div>
                  <div style={{ fontSize: '11px', marginTop: '8px', color: C.textMuted }}>
                    {chatLogError.toLowerCase().includes('auth') ? 'Server is gating this endpoint; the passwordless-mode flag may be off-sync after a restart.' : null}
                  </div>
                </div>
              ) : chatLogFetchedAt ? (
                <div style={{ color: C.textMuted, fontSize: '13px', padding: '20px', textAlign: 'center' }}>
                  No logged turns yet. Send a message to populate.
                  <div style={{ fontSize: '11px', color: C.textDim, marginTop: '6px' }}>
                    Last checked {new Date(chatLogFetchedAt).toLocaleTimeString()}
                  </div>
                </div>
              ) : (
                <div style={{ color: C.textMuted, fontSize: '13px', padding: '20px', textAlign: 'center' }}>
                  Loading chat log…
                </div>
              )
            )}
            {serverChatLog.slice().reverse().map((e, i) => (
              <div key={i} style={{
                padding: '12px 14px', marginBottom: '8px',
                background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: '10px',
              }}>
                <div style={{ fontSize: '10px', color: C.textDim, marginBottom: '6px', display: 'flex', gap: '10px' }}>
                  <span>{new Date((e.ts || 0) * 1000).toLocaleString()}</span>
                  <span style={{ color: C.accent }}>{e.tier}</span>
                  <span style={{ color: C.purple }}>{(e.intent || '').split('{')[0]}</span>
                  <span style={{ color: C.green }}>{e.confidence ? `${(e.confidence * 100).toFixed(0)}%` : ''}</span>
                </div>
                <div style={{ fontSize: '13px', color: C.accent, marginBottom: '4px' }}>
                  <strong>User:</strong> {e.user}
                </div>
                <div style={{ fontSize: '13px', color: C.text, whiteSpace: 'pre-wrap' }}>
                  <strong style={{ color: C.green }}>AI:</strong> {e.reply}
                </div>
              </div>
            ))}
          </>
        )}
        {tab === 'events' && (
          <>
            {localEvents.length === 0 && (
              <div style={{ color: C.textMuted, fontSize: '13px', padding: '20px', textAlign: 'center' }}>
                No UI events captured yet.
              </div>
            )}
            {localEvents.slice().reverse().map((e, i) => (
              <div key={i} style={{
                display: 'flex', gap: '12px', padding: '6px 10px',
                borderBottom: `1px solid ${C.borderSubtle}`, fontSize: '12px',
              }}>
                <span style={{ color: C.textDim, minWidth: '120px' }}>
                  {new Date(e.t).toLocaleTimeString()}
                </span>
                <span style={{ color: C.accent, minWidth: '140px', fontWeight: 700 }}>{e.kind}</span>
                <span style={{ color: C.textSecondary, flex: 1 }}>
                  {e.data ? JSON.stringify(e.data) : ''}
                </span>
              </div>
            ))}
          </>
        )}
        {tab === 'system' && (
          <div style={{ fontSize: '12px', color: C.textSecondary }}>
            <div style={{ marginBottom: '12px', display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '10px' }}>
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
                  background: C.bgInput, border: `1px solid ${C.borderSubtle}`, borderRadius: '8px',
                }}>
                  <span style={{ color: C.textMuted }}>{k}</span>
                  <span style={{ color: C.text, fontWeight: 700 }}>{v}</span>
                </div>
              ))}
            </div>
            <div style={{ display: 'flex', gap: '8px' }}>
              <button onClick={onRefreshQos}
                style={{
                  padding: '8px 14px', fontSize: '12px', background: C.accentBg,
                  border: `1px solid ${C.accentBorder}`, color: C.accent,
                  borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                }}>Refresh QoS</button>
              <button onClick={onRefreshFacts}
                style={{
                  padding: '8px 14px', fontSize: '12px', background: C.purpleBg,
                  border: `1px solid ${C.purpleBorder}`, color: C.purple,
                  borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit', fontWeight: 700,
                }}>Refresh facts</button>
            </div>
            {qosReport && (
              <div style={{ marginTop: '14px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {qosReport.checks?.map((c, i) => (
                  <div key={i} style={{
                    display: 'flex', justifyContent: 'space-between',
                    padding: '6px 10px', borderRadius: '6px',
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
