import React, { useRef } from 'react';
import { THEMES } from './themes';
import { AVATAR_PRESETS } from './catalogs';
import { useModalFocus } from './useModalFocus';

export type SettingsTab = 'profile' | 'appearance' | 'behavior' | 'data';

export interface SettingsShape {
  theme: string;
  displayName: string;
  avatarDataUrl?: string;
  avatarGradient?: string;
  fontSize: 'small' | 'medium' | 'large';
  erudaMode: 'auto' | 'on' | 'off';
  sendOnEnter: boolean;
  persistConversations: boolean;
  showReasoning: boolean;
  compactMode: boolean;
  developerMode: boolean;
  defaultTier: string;
  apiHost: string;
  customTheme: { bg: string; accent: string; text: string; green: string; red: string } | null;
  // Callers may carry additional settings keys; we read them by string index.
  [k: string]: any;
}

export interface SettingsModalProps {
  C: any;
  isMobile: boolean;
  settings: SettingsShape;
  setSettings: React.Dispatch<React.SetStateAction<SettingsShape>>;
  tab: SettingsTab;
  onTabChange: (t: SettingsTab) => void;
  onClose: () => void;
  currentTier: string;
  onTierSelect: (tier: string) => void;
  onExportEvents: () => void;
  onExportConversations: () => void;
  onClearHistory: () => void;
  onResetSettings: () => void;
  onDeleteAccount: () => void;
  conversationCount: number;
  messageCount: number;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  C, isMobile, settings, setSettings, tab, onTabChange, onClose,
  currentTier, onTierSelect,
  onExportEvents, onExportConversations, onClearHistory, onResetSettings, onDeleteAccount,
  conversationCount, messageCount,
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  return (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: 200,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: '16px',
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-label='Settings'
      onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '520px',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: '14px',
        padding: isMobile ? '20px' : '28px', color: C.text,
        boxShadow: '0 24px 60px rgba(0,0,0,0.45)',
        maxHeight: '90vh', overflowY: 'auto',
      }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
        <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 800, letterSpacing: '0.12em', textTransform: 'uppercase' }}>Settings</h2>
        <button onClick={onClose} aria-label='Close settings'
          style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
          {'\u2715'}
        </button>
      </div>

      {/* Tabs */}
      <div role='tablist' aria-label='Settings sections'
        style={{ display: 'flex', gap: '4px', borderBottom: `1px solid ${C.borderSubtle}`, marginBottom: '18px' }}>
        {([
          { id: 'profile', label: 'Profile' },
          { id: 'appearance', label: 'Appearance' },
          { id: 'behavior', label: 'Behavior' },
          { id: 'data', label: 'Data' },
        ] as const).map(t => (
          <button key={t.id} onClick={() => onTabChange(t.id)}
            role='tab' aria-selected={tab === t.id}
            style={{
              padding: '8px 12px', fontSize: '12px', fontWeight: 700,
              background: 'transparent', border: 'none', cursor: 'pointer',
              color: tab === t.id ? C.accent : C.textMuted,
              borderBottom: `2px solid ${tab === t.id ? C.accent : 'transparent'}`,
              marginBottom: '-1px', fontFamily: 'inherit',
            }}>{t.label}</button>
        ))}
      </div>

      {/* ===== Profile tab ===== */}
      {tab === 'profile' && (
        <div role='tabpanel' aria-label='Profile'>
          <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Display Name</label>
          <input type='text' value={settings.displayName}
            onChange={(e) => setSettings(s => ({ ...s, displayName: e.target.value.slice(0, 40) }))}
            placeholder='Your name'
            autoComplete='name' aria-label='Display name' maxLength={40}
            style={{
              width: '100%', marginTop: '6px', padding: '10px 12px',
              background: C.bgInput, border: `1px solid ${C.border}`, borderRadius: '8px',
              color: C.text, fontFamily: 'inherit', fontSize: '14px', boxSizing: 'border-box',
            }} />

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Avatar</label>
            <div style={{ display: 'flex', gap: '14px', alignItems: 'center', marginTop: '10px' }}>
              <div style={{
                width: '72px', height: '72px', borderRadius: '50%',
                background: settings.avatarDataUrl ? `url(${settings.avatarDataUrl}) center/cover` : (settings.avatarGradient || AVATAR_PRESETS[0]),
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontSize: '26px', fontWeight: 800, color: '#fff',
                boxShadow: `0 0 0 1px ${C.border}, 0 4px 14px rgba(0,0,0,0.2)`,
                flexShrink: 0,
              }}>
                {!settings.avatarDataUrl && (settings.displayName.trim().charAt(0).toUpperCase() || 'U')}
              </div>
              <div style={{ flex: 1 }}>
                <label style={{
                  display: 'inline-block', padding: '8px 14px', fontSize: '12px', fontWeight: 700,
                  background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                  color: C.accent, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                  textTransform: 'uppercase', letterSpacing: '0.05em',
                }}>
                  Upload photo
                  <input type='file' accept='image/*' style={{ display: 'none' }}
                    onChange={(e) => {
                      const f = e.target.files?.[0];
                      if (!f) return;
                      if (f.size > 500 * 1024) { alert('Please pick an image under 500 KB.'); return; }
                      const reader = new FileReader();
                      reader.onload = () => {
                        setSettings(s => ({ ...s, avatarDataUrl: String(reader.result) }));
                      };
                      reader.readAsDataURL(f);
                      e.target.value = '';
                    }} />
                </label>
                {settings.avatarDataUrl && (
                  <button onClick={() => setSettings(s => ({ ...s, avatarDataUrl: '' }))}
                    style={{
                      marginLeft: '8px', padding: '8px 14px', fontSize: '12px', fontWeight: 700,
                      background: 'transparent', border: `1px solid ${C.border}`,
                      color: C.textMuted, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                      textTransform: 'uppercase', letterSpacing: '0.05em',
                    }}>Remove</button>
                )}
                <div style={{ fontSize: '11px', color: C.textDim, marginTop: '6px' }}>
                  PNG / JPG / SVG up to 500 KB. Or pick a preset below.
                </div>
              </div>
            </div>

            <div style={{
              display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '10px',
              marginTop: '14px',
            }}>
              {AVATAR_PRESETS.map((g, i) => {
                const isPicked = !settings.avatarDataUrl && (settings.avatarGradient || AVATAR_PRESETS[0]) === g;
                return (
                  <button key={i}
                    onClick={() => setSettings(s => ({ ...s, avatarDataUrl: '', avatarGradient: g }))}
                    title={`Preset ${i + 1}`}
                    style={{
                      width: '100%', aspectRatio: '1 / 1',
                      borderRadius: '50%', background: g,
                      border: `2px solid ${isPicked ? C.accent : 'transparent'}`,
                      boxShadow: isPicked ? `0 0 0 3px ${C.accentGlow}` : 'none',
                      cursor: 'pointer', padding: 0,
                      transition: 'transform 0.1s',
                    }} />
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* ===== Appearance tab ===== */}
      {tab === 'appearance' && (
        <div role='tabpanel' aria-label='Appearance'>
          <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Theme</label>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '10px', marginTop: '10px' }}>
            {([
              { id: 'dark' as const, name: 'Onyx', tagline: 'Deep black, violet accent' },
              { id: 'light' as const, name: 'Light', tagline: 'Clean white, violet accent' },
              { id: 'midnight' as const, name: 'Midnight', tagline: 'Deep blue, airy' },
              { id: 'forest' as const, name: 'Forest', tagline: 'Green-ink, calm' },
              { id: 'sunset' as const, name: 'Sunset', tagline: 'Warm plum, peach' },
              { id: 'rose' as const, name: 'Rose', tagline: 'Blush paper, cranberry' },
              { id: 'contrast' as const, name: 'High Contrast', tagline: 'A11y: max readability' },
            ] as Array<{ id: string; name: string; tagline: string }>).map(t => {
              const preview = THEMES[t.id];
              const picked = settings.theme === t.id;
              return (
                <button key={t.id} onClick={() => setSettings(s => ({ ...s, theme: t.id }))}
                  style={{
                    padding: '14px', background: preview.bgCard,
                    border: `2px solid ${picked ? C.accent : C.border}`,
                    borderRadius: '12px', cursor: 'pointer', fontFamily: 'inherit',
                    textAlign: 'left',
                    boxShadow: picked ? `0 0 0 3px ${C.accentGlow}` : 'none',
                  }}>
                  <div style={{ fontSize: '13px', fontWeight: 700, color: preview.text }}>{t.name}</div>
                  <div style={{ display: 'flex', gap: '4px', marginTop: '8px' }}>
                    <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: preview.accent }} />
                    <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: preview.green }} />
                    <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: preview.purple }} />
                    <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: preview.bg, border: `1px solid ${preview.border}` }} />
                  </div>
                  <div style={{ fontSize: '10.5px', color: preview.textMuted, marginTop: '8px' }}>
                    {t.tagline}
                  </div>
                </button>
              );
            })}
          </div>

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Font Size</label>
            <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
              {(['small', 'medium', 'large'] as const).map(sz => (
                <button key={sz} onClick={() => setSettings(s => ({ ...s, fontSize: sz }))}
                  style={{
                    flex: 1, padding: '10px',
                    background: settings.fontSize === sz ? C.accentBg : 'transparent',
                    border: `1px solid ${settings.fontSize === sz ? C.accentBorder : C.border}`,
                    color: settings.fontSize === sz ? C.accent : C.textMuted,
                    borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                    textTransform: 'uppercase', fontSize: '12px', fontWeight: 700,
                  }}>{sz}</button>
              ))}
            </div>
          </div>

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Custom Colors</label>
            <div style={{ fontSize: '11px', color: C.textDim, marginTop: '4px', marginBottom: '10px' }}>
              Override the active theme's key colors. Set any to change the look instantly.
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '10px' }}>
              {([
                { key: 'bg', label: 'Background' },
                { key: 'accent', label: 'Accent' },
                { key: 'text', label: 'Text' },
                { key: 'green', label: 'Success' },
                { key: 'red', label: 'Error' },
              ] as const).map(({ key, label }) => (
                <label key={key} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input type='color'
                    value={settings.customTheme?.[key] || (C as any)[key] || '#000000'}
                    onChange={(e) => setSettings(s => ({
                      ...s,
                      customTheme: { ...(s.customTheme || { bg: C.bg, accent: C.accent, text: C.text, green: C.green, red: C.red }), [key]: e.target.value },
                    }))}
                    style={{ width: '32px', height: '32px', border: 'none', borderRadius: '6px', cursor: 'pointer', background: 'transparent' }} />
                  <span style={{ fontSize: '12px', color: C.textSecondary }}>{label}</span>
                </label>
              ))}
            </div>
            {settings.customTheme && (
              <button onClick={() => setSettings(s => ({ ...s, customTheme: null }))}
                style={{
                  marginTop: '10px', padding: '6px 14px', fontSize: '11px',
                  background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: '6px', cursor: 'pointer', fontFamily: 'inherit',
                }}>Reset custom colors</button>
            )}
          </div>

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Dev Tools (Eruda)</label>
            <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
              {(['auto', 'on', 'off'] as const).map(m => (
                <button key={m} onClick={() => setSettings(s => ({ ...s, erudaMode: m }))}
                  style={{
                    flex: 1, padding: '10px',
                    background: settings.erudaMode === m ? C.purpleBg : 'transparent',
                    border: `1px solid ${settings.erudaMode === m ? C.purpleBorder : C.border}`,
                    color: settings.erudaMode === m ? C.purple : C.textMuted,
                    borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                    textTransform: 'uppercase', fontSize: '12px', fontWeight: 700,
                  }}>{m === 'auto' ? 'Auto (mobile)' : m === 'on' ? 'Always on' : 'Off'}</button>
              ))}
            </div>
            <div style={{ fontSize: '11px', color: C.textDim, marginTop: '6px' }}>
              Floating devtools overlay. Auto only shows on phones/tablets.
            </div>
          </div>
        </div>
      )}

      {/* ===== Behavior tab ===== */}
      {tab === 'behavior' && (
        <div role='tabpanel' aria-label='Behavior'>
          {([
            { key: 'sendOnEnter', label: 'Send on Enter', sub: 'Shift+Enter inserts a newline.' },
            { key: 'persistConversations', label: 'Save conversations locally', sub: 'Stored in this browser only; never uploaded. Per PSA policy.' },
            { key: 'showReasoning', label: 'Show reasoning trace on replies', sub: 'Expandable per-message. Shows DerivationTrace steps.' },
            { key: 'compactMode', label: 'Compact mode', sub: 'Dense TUI-style layout: smaller fonts, tighter spacing.' },
            { key: 'developerMode', label: 'Developer mode', sub: 'Telemetry, system info, plan panel, provenance badges, diagnostic panels.' },
          ] as const).map((row, i) => (
            <label key={row.key} style={{
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              padding: '12px 0', borderTop: i === 0 ? 'none' : `1px solid ${C.borderSubtle}`,
              cursor: 'pointer', gap: '12px',
            }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: '13px', fontWeight: 600, color: C.text }}>{row.label}</div>
                <div style={{ fontSize: '11px', color: C.textDim, marginTop: '2px' }}>{row.sub}</div>
              </div>
              <input type='checkbox' checked={!!settings[row.key]}
                onChange={(e) => setSettings(s => ({ ...s, [row.key]: e.target.checked }))}
                style={{ width: '18px', height: '18px', accentColor: C.accent, flexShrink: 0 }} />
            </label>
          ))}

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Default Model</label>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '8px', marginTop: '8px' }}>
              {(['Pulse','Bridge','BigBrain'] as const).map(tier => {
                const picked = settings.defaultTier === tier;
                return (
                  <button key={tier}
                    onClick={() => {
                      setSettings(s => ({ ...s, defaultTier: tier }));
                      onTierSelect(tier);
                    }}
                    style={{
                      padding: '12px 10px', borderRadius: '8px',
                      background: picked ? C.accentBg : 'transparent',
                      border: `1px solid ${picked ? C.accentBorder : C.border}`,
                      color: picked ? C.accent : C.textMuted,
                      cursor: 'pointer', fontFamily: 'inherit',
                      textAlign: 'center',
                    }}>
                    <div style={{ fontSize: '13px', fontWeight: 700 }}>{tier}</div>
                    <div style={{ fontSize: '10.5px', color: C.textDim, marginTop: '4px' }}>
                      {tier === 'Pulse' ? 'Fast' : tier === 'Bridge' ? 'Balanced' : 'Deepest'}
                    </div>
                  </button>
                );
              })}
            </div>
            <div style={{ fontSize: '11px', color: C.textDim, marginTop: '6px' }}>
              Locks your chosen model across sessions and server restarts. Currently active: <strong style={{ color: C.text }}>{currentTier}</strong>.
            </div>
          </div>

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Backend Host</label>
            <input type='text' value={settings.apiHost}
              onChange={(e) => setSettings(s => ({ ...s, apiHost: e.target.value }))}
              autoComplete='off' spellCheck={false} aria-label='Backend host'
              placeholder='Empty = auto-detect'
              style={{
                width: '100%', marginTop: '6px', padding: '10px 12px',
                background: C.bgInput, border: `1px solid ${C.border}`, borderRadius: '8px',
                color: C.text, fontFamily: 'inherit', fontSize: '13px', boxSizing: 'border-box',
              }} />
            <div style={{ fontSize: '11px', color: C.textDim, marginTop: '4px' }}>
              Override when serving the UI from a different host than the API.
            </div>
          </div>
        </div>
      )}

      {/* ===== Data tab ===== */}
      {tab === 'data' && (
        <div role='tabpanel' aria-label='Data'>
          <div style={{ fontSize: '11px', fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '10px' }}>Export</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
            <button onClick={onExportEvents}
              style={{
                padding: '10px', background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                color: C.accent, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                fontSize: '11px', fontWeight: 700, textTransform: 'uppercase',
              }}>Event log</button>
            <button onClick={onExportConversations}
              style={{
                padding: '10px', background: C.purpleBg, border: `1px solid ${C.purpleBorder}`,
                color: C.purple, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                fontSize: '11px', fontWeight: 700, textTransform: 'uppercase',
              }}>Conversations</button>
          </div>
          <div style={{ marginTop: '8px', fontSize: '10px', color: C.textDim }}>
            {messageCount} messages across {conversationCount} conversation{conversationCount === 1 ? '' : 's'}.
          </div>

          <div style={{ marginTop: '22px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <div style={{ fontSize: '11px', fontWeight: 700, color: C.red, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '10px' }}>Danger zone</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
              <button onClick={onClearHistory}
                style={{
                  padding: '10px', background: C.redBg, border: `1px solid ${C.redBorder}`,
                  color: C.red, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: '11px', fontWeight: 700, textTransform: 'uppercase',
                }}>Clear history</button>
              <button onClick={onResetSettings}
                style={{
                  padding: '10px', background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: '11px', fontWeight: 700, textTransform: 'uppercase',
                }}>Reset settings</button>
            </div>
            <button onClick={onDeleteAccount}
              style={{
                width: '100%', marginTop: '10px', padding: '12px',
                background: 'transparent', border: `1px solid ${C.redBorder}`,
                color: C.red, borderRadius: '8px', cursor: 'pointer', fontFamily: 'inherit',
                fontSize: '11px', fontWeight: 700, textTransform: 'uppercase',
              }}>Delete account &amp; all data</button>
          </div>
        </div>
      )}

      <div style={{ marginTop: '16px', fontSize: '11px', color: C.textDim, textAlign: 'center' }}>
        Settings save automatically to this browser.
      </div>
    </div>
  </div>
  );
};
