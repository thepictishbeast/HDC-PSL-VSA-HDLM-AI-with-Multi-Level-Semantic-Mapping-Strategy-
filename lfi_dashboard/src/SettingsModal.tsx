import React, { useRef } from 'react';
import { THEMES } from './themes';
import { AVATAR_PRESETS } from './catalogs';
import { useModalFocus } from './useModalFocus';
import { T } from './tokens';

export type SettingsTab = 'profile' | 'appearance' | 'behavior' | 'data';

export interface SettingsShape {
  theme: string;
  displayName: string;
  avatarDataUrl?: string;
  avatarGradient?: string;
  fontSize: 'small' | 'medium' | 'large' | 'xlarge';
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
  // c2-433 / #398: backend host for the workspace-capacity slider +
  // any future backend-state controls. Defaults to '127.0.0.1' when
  // not supplied — keeps older call sites backward-compat.
  host?: string;
  onTierSelect: (tier: string) => void;
  onExportEvents: () => void;
  onExportConversations: () => void;
  onExportAllJson: () => void;
  // c2-241 / #102: round-trip for the Full backup export. Receives the raw
  // File; parent validates schema + merges into state.
  onImportBackup: (file: File) => void;
  // Live-preview a theme while hovering its card. null clears the preview.
  onPreviewTheme?: (themeKey: string | null) => void;
  onClearHistory: () => void;
  onResetSettings: () => void;
  onDeleteAccount: () => void;
  conversationCount: number;
  messageCount: number;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  C, isMobile, settings, setSettings, tab, onTabChange, onClose,
  currentTier, onTierSelect, host,
  onExportEvents, onExportConversations, onExportAllJson, onImportBackup, onPreviewTheme, onClearHistory, onResetSettings, onDeleteAccount,
  conversationCount, messageCount,
}) => {
  // c2-433 / #398: workspace capacity state. Fetched from
  // GET /api/settings/workspace when the Behavior tab mounts; PUT with
  // {max_mb} commits a change. Silent-fail when endpoint not live so
  // the rest of the tab still renders.
  const wsHost = host || '127.0.0.1';
  const [wsData, setWsData] = useState<null | { capacity: number; footprint_bytes: number; footprint_mb: number; per_slot_bytes: number; max_mb?: number }>(null);
  const [wsLoading, setWsLoading] = useState<boolean>(false);
  const [wsErr, setWsErr] = useState<string | null>(null);
  const [wsSaving, setWsSaving] = useState<boolean>(false);
  const [wsMaxMb, setWsMaxMb] = useState<number>(512);
  const dialogRef = useRef<HTMLDivElement>(null);
  useModalFocus(true, dialogRef);
  return (
  <div onClick={onClose}
    style={{
      position: 'fixed', inset: 0, zIndex: T.z.modal,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      padding: T.spacing.lg,
    }}>
    <div ref={dialogRef} role='dialog' aria-modal='true' aria-labelledby='scc-settings-title'
      onClick={(e) => e.stopPropagation()}
      style={{
        width: '100%', maxWidth: '520px',
        background: C.bgCard, border: `1px solid ${C.border}`, borderRadius: T.radii.xxl,
        padding: isMobile ? T.spacing.xl : '28px', color: C.text,
        boxShadow: T.shadows.modal,
        maxHeight: '90dvh', overflowY: 'auto',
      }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
        <h2 id='scc-settings-title' style={{ margin: 0, fontSize: '15px', fontWeight: 800, letterSpacing: '0.12em', textTransform: 'uppercase' }}>Settings</h2>
        <button onClick={onClose} aria-label='Close settings'
          style={{ background: 'transparent', border: 'none', color: C.textMuted, fontSize: '20px', cursor: 'pointer' }}>
          {'\u2715'}
        </button>
      </div>

      {/* Tabs — WAI-ARIA tablist with ArrowLeft/Right key nav per #178. */}
      {(() => {
        const TABS = [
          { id: 'profile', label: 'Profile' },
          { id: 'appearance', label: 'Appearance' },
          { id: 'behavior', label: 'Behavior' },
          { id: 'data', label: 'Data' },
        ] as const;
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
          <div role='tablist' aria-label='Settings sections' onKeyDown={onTabKey}
            style={{ display: 'flex', gap: '4px', borderBottom: `1px solid ${C.borderSubtle}`, marginBottom: '18px' }}>
            {TABS.map(t => {
              const active = tab === t.id;
              return (
                <button key={t.id} onClick={() => onTabChange(t.id)}
                  role='tab' aria-selected={active}
                  // c2-433 / task 270: roving tabindex — only the active
                  // tab is tab-stoppable; arrow keys move within. Standard
                  // WAI-ARIA tablist keyboard pattern.
                  tabIndex={active ? 0 : -1}
                  style={{
                    padding: '8px 12px', fontSize: T.typography.sizeSm, fontWeight: 700,
                    background: 'transparent', border: 'none', cursor: 'pointer',
                    color: active ? C.accent : C.textMuted,
                    borderBottom: `2px solid ${active ? C.accent : 'transparent'}`,
                    marginBottom: '-1px', fontFamily: 'inherit',
                  }}>{t.label}</button>
              );
            })}
          </div>
        );
      })()}

      {/* ===== Profile tab ===== */}
      {tab === 'profile' && (
        <div role='tabpanel' aria-label='Profile'>
          <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Display Name</label>
          <input type='text' value={settings.displayName}
            onChange={(e) => setSettings(s => ({ ...s, displayName: e.target.value.slice(0, 40) }))}
            placeholder='Your name'
            autoComplete='name' aria-label='Display name' maxLength={40}
            style={{
              width: '100%', marginTop: '6px', padding: '10px 12px',
              background: C.bgInput, border: `1px solid ${C.border}`, borderRadius: T.radii.lg,
              color: C.text, fontFamily: 'inherit', fontSize: '14px', boxSizing: 'border-box',
            }} />

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Avatar</label>
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
                  display: 'inline-block', padding: '8px 14px', fontSize: T.typography.sizeSm, fontWeight: 700,
                  background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                  color: C.accent, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
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
                      marginLeft: '8px', padding: '8px 14px', fontSize: T.typography.sizeSm, fontWeight: 700,
                      background: 'transparent', border: `1px solid ${C.border}`,
                      color: C.textMuted, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                      textTransform: 'uppercase', letterSpacing: '0.05em',
                    }}>Remove</button>
                )}
                <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '6px' }}>
                  PNG / JPG / SVG up to 500 KB. Or pick a preset below.
                </div>
              </div>
            </div>

            <div style={{
              display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: T.spacing.md,
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
                      // c0-019 disabled glow. Use a solid accentBg ring for
                      // the selected-avatar halo so selection is still clear.
                      boxShadow: isPicked ? `0 0 0 3px ${C.accentBg}` : 'none',
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
          {/* Auto-theme toggle — honors the OS prefers-color-scheme media
              query dynamically, so flipping the OS between dark/light updates
              the dashboard in real time without a setting change. */}
          <label style={{
            display: 'flex', alignItems: 'center', gap: T.spacing.md,
            padding: '10px 12px', marginBottom: '12px',
            background: C.bgInput, border: `1px solid ${C.borderSubtle}`,
            borderRadius: T.radii.lg, cursor: 'pointer',
          }}>
            <input type='checkbox'
              checked={!!settings.autoTheme}
              onChange={(e) => setSettings(s => ({ ...s, autoTheme: e.target.checked }))}
              style={{ accentColor: C.accent, width: '16px', height: '16px' }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: T.typography.sizeMd, fontWeight: 700, color: C.text }}>Auto theme</div>
              <div style={{ fontSize: T.typography.sizeXs, color: C.textMuted, marginTop: '2px' }}>
                Follow the system dark/light preference. Overrides the theme picker below.
              </div>
            </div>
          </label>
          <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: settings.autoTheme ? C.textDim : C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>
            Theme {settings.autoTheme && '(auto mode active)'}
          </label>
          <div style={{
            display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: T.spacing.md, marginTop: '10px',
            opacity: settings.autoTheme ? 0.55 : 1, pointerEvents: settings.autoTheme ? 'none' : 'auto',
          }}>
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
                <button key={t.id}
                  onClick={() => { setSettings(s => ({ ...s, theme: t.id })); onPreviewTheme?.(null); }}
                  onMouseEnter={() => onPreviewTheme?.(t.id)}
                  onMouseLeave={() => onPreviewTheme?.(null)}
                  onFocus={() => onPreviewTheme?.(t.id)}
                  onBlur={() => onPreviewTheme?.(null)}
                  style={{
                    padding: '14px', background: preview.bgCard,
                    border: `2px solid ${picked ? C.accent : C.border}`,
                    borderRadius: T.radii.xxl, cursor: 'pointer', fontFamily: 'inherit',
                    textAlign: 'left',
                    // Solid ring replaces the former glow so selection stays
                    // obvious on the flat-no-neon palette.
                    boxShadow: picked ? `0 0 0 3px ${C.accentBg}` : 'none',
                  }}>
                  <div style={{ fontSize: T.typography.sizeMd, fontWeight: 700, color: preview.text }}>{t.name}</div>
                  {/* c2-433 / task 204: live thumbnail preview. Renders the
                      theme's actual bg + bgCard chrome with two mock chat
                      bubbles (assistant on left in bgCard, user on right in
                      accentBg) so the user sees what the chat will look like,
                      not just abstract color swatches. Static mock — no
                      animation, no real text — keeps the card lightweight. */}
                  <div style={{
                    marginTop: '8px',
                    background: preview.bg,
                    border: `1px solid ${preview.borderSubtle || preview.border}`,
                    borderRadius: T.radii.md,
                    padding: '8px',
                    display: 'flex', flexDirection: 'column', gap: '4px',
                    height: '54px', overflow: 'hidden',
                  }}>
                    <div style={{
                      alignSelf: 'flex-start', maxWidth: '70%',
                      padding: '4px 7px', background: preview.bgCard,
                      border: `1px solid ${preview.border}`,
                      borderRadius: '3px 8px 8px 8px',
                    }}>
                      <div style={{ height: '4px', width: '52px', background: preview.text, opacity: 0.55, borderRadius: '2px', marginBottom: '3px' }} />
                      <div style={{ height: '4px', width: '38px', background: preview.text, opacity: 0.35, borderRadius: '2px' }} />
                    </div>
                    <div style={{
                      alignSelf: 'flex-end', maxWidth: '60%',
                      padding: '4px 7px', background: preview.accentBg,
                      border: `1px solid ${preview.accentBorder}`,
                      borderRadius: '8px 8px 3px 8px',
                    }}>
                      <div style={{ height: '4px', width: '32px', background: preview.accent, opacity: 0.85, borderRadius: '2px' }} />
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '4px', marginTop: '8px' }}>
                    <div title='Accent' style={{ width: '14px', height: '14px', borderRadius: '50%', background: preview.accent }} />
                    <div title='Success' style={{ width: '14px', height: '14px', borderRadius: '50%', background: preview.green }} />
                    <div title='Highlight' style={{ width: '14px', height: '14px', borderRadius: '50%', background: preview.purple }} />
                    <div title='Background' style={{ width: '14px', height: '14px', borderRadius: '50%', background: preview.bg, border: `1px solid ${preview.border}` }} />
                  </div>
                  <div style={{ fontSize: '10.5px', color: preview.textMuted, marginTop: '6px' }}>
                    {t.tagline}
                  </div>
                </button>
              );
            })}
          </div>

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Font Size</label>
            {/* c2-398 / task 198: extended the enum with 'xlarge' for the
                a11y case of users who need the big step without zooming the
                whole app. fontScale in App.tsx maps xlarge → 1.35. */}
            <div style={{ display: 'flex', gap: T.spacing.sm, marginTop: '8px' }}>
              {(['small', 'medium', 'large', 'xlarge'] as const).map(sz => (
                <button key={sz} onClick={() => setSettings(s => ({ ...s, fontSize: sz }))}
                  style={{
                    flex: 1, padding: T.spacing.md,
                    background: settings.fontSize === sz ? C.accentBg : 'transparent',
                    border: `1px solid ${settings.fontSize === sz ? C.accentBorder : C.border}`,
                    color: settings.fontSize === sz ? C.accent : C.textMuted,
                    borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                    textTransform: 'uppercase', fontSize: T.typography.sizeSm, fontWeight: 700,
                  }}>{sz === 'xlarge' ? 'X-Large' : sz}</button>
              ))}
            </div>
          </div>

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Custom Colors</label>
            <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '4px', marginBottom: '10px' }}>
              Override the active theme's key colors. Set any to change the look instantly.
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: T.spacing.md }}>
              {([
                { key: 'bg', label: 'Background' },
                { key: 'accent', label: 'Accent' },
                { key: 'text', label: 'Text' },
                { key: 'green', label: 'Success' },
                { key: 'red', label: 'Error' },
              ] as const).map(({ key, label }) => (
                <label key={key} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                  <input type='color'
                    value={settings.customTheme?.[key] || (C as any)[key] || '#000000'}
                    onChange={(e) => setSettings(s => ({
                      ...s,
                      customTheme: { ...(s.customTheme || { bg: C.bg, accent: C.accent, text: C.text, green: C.green, red: C.red }), [key]: e.target.value },
                    }))}
                    style={{ width: '32px', height: '32px', border: 'none', borderRadius: T.radii.md, cursor: 'pointer', background: 'transparent' }} />
                  <span style={{ fontSize: T.typography.sizeSm, color: C.textSecondary }}>{label}</span>
                </label>
              ))}
            </div>
            {settings.customTheme && (
              <button onClick={() => setSettings(s => ({ ...s, customTheme: null }))}
                style={{
                  marginTop: '10px', padding: '6px 14px', fontSize: T.typography.sizeXs,
                  background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: T.radii.md, cursor: 'pointer', fontFamily: 'inherit',
                }}>Reset custom colors</button>
            )}
          </div>

          <div style={{ marginTop: '18px' }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Dev Tools (Eruda)</label>
            <div style={{ display: 'flex', gap: T.spacing.sm, marginTop: '8px' }}>
              {(['auto', 'on', 'off'] as const).map(m => (
                <button key={m} onClick={() => setSettings(s => ({ ...s, erudaMode: m }))}
                  style={{
                    flex: 1, padding: T.spacing.md,
                    background: settings.erudaMode === m ? C.purpleBg : 'transparent',
                    border: `1px solid ${settings.erudaMode === m ? C.purpleBorder : C.border}`,
                    color: settings.erudaMode === m ? C.purple : C.textMuted,
                    borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                    textTransform: 'uppercase', fontSize: T.typography.sizeSm, fontWeight: 700,
                  }}>{m === 'auto' ? 'Auto (mobile)' : m === 'on' ? 'Always on' : 'Off'}</button>
              ))}
            </div>
            <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '6px' }}>
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
            { key: 'notifyOnReply', label: 'Notify when AI replies', sub: 'OS notification if the tab is hidden when a response finishes.' },
            { key: 'developerMode', label: 'Developer mode', sub: 'Telemetry, system info, plan panel, provenance badges, diagnostic panels.' },
          ] as const).map((row, i) => (
            <label key={row.key} style={{
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              padding: '12px 0', borderTop: i === 0 ? 'none' : `1px solid ${C.borderSubtle}`,
              cursor: 'pointer', gap: '12px',
            }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: T.typography.sizeMd, fontWeight: 600, color: C.text }}>{row.label}</div>
                <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '2px' }}>{row.sub}</div>
              </div>
              <input type='checkbox' checked={!!settings[row.key]}
                onChange={async (e) => {
                  const v = e.target.checked;
                  // Request Notification permission the first time the user
                  // opts into the reply-notification flag — quieter than
                  // prompting on page load.
                  if (row.key === 'notifyOnReply' && v && typeof Notification !== 'undefined' && Notification.permission === 'default') {
                    try { await Notification.requestPermission(); } catch { /* non-fatal */ }
                  }
                  setSettings(s => ({ ...s, [row.key]: v }));
                }}
                style={{ width: '18px', height: '18px', accentColor: C.accent, flexShrink: 0 }} />
            </label>
          ))}

          {/* c2-433 / #398: Workspace capacity. Slider with preset stops
              (64/128/256/512/1024/2048 MB). GET on tab open, PUT on
              slider commit. Hidden when the endpoint is unreachable. */}
          <WorkspaceControl C={C} wsHost={wsHost}
            wsData={wsData} setWsData={setWsData}
            wsLoading={wsLoading} setWsLoading={setWsLoading}
            wsErr={wsErr} setWsErr={setWsErr}
            wsSaving={wsSaving} setWsSaving={setWsSaving}
            wsMaxMb={wsMaxMb} setWsMaxMb={setWsMaxMb} />

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Default Model</label>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: T.spacing.sm, marginTop: '8px' }}>
              {(['Pulse','Bridge','BigBrain'] as const).map(tier => {
                const picked = settings.defaultTier === tier;
                return (
                  <button key={tier}
                    onClick={() => {
                      setSettings(s => ({ ...s, defaultTier: tier }));
                      onTierSelect(tier);
                    }}
                    style={{
                      padding: '12px 10px', borderRadius: T.radii.lg,
                      background: picked ? C.accentBg : 'transparent',
                      border: `1px solid ${picked ? C.accentBorder : C.border}`,
                      color: picked ? C.accent : C.textMuted,
                      cursor: 'pointer', fontFamily: 'inherit',
                      textAlign: 'center',
                    }}>
                    <div style={{ fontSize: T.typography.sizeMd, fontWeight: 700 }}>{tier}</div>
                    <div style={{ fontSize: '10.5px', color: C.textDim, marginTop: '4px' }}>
                      {tier === 'Pulse' ? 'Fast' : tier === 'Bridge' ? 'Balanced' : 'Deepest'}
                    </div>
                  </button>
                );
              })}
            </div>
            <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '6px' }}>
              Locks your chosen model across sessions and server restarts. Currently active: <strong style={{ color: C.text }}>{currentTier}</strong>.
            </div>
          </div>

          <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <label style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>Backend Host</label>
            <input type='text' value={settings.apiHost}
              onChange={(e) => setSettings(s => ({ ...s, apiHost: e.target.value }))}
              autoComplete='off' spellCheck={false} aria-label='Backend host'
              placeholder='Empty = auto-detect'
              style={{
                width: '100%', marginTop: '6px', padding: '10px 12px',
                background: C.bgInput, border: `1px solid ${C.border}`, borderRadius: T.radii.lg,
                color: C.text, fontFamily: 'inherit', fontSize: T.typography.sizeMd, boxSizing: 'border-box',
              }} />
            <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '4px' }}>
              Override when serving the UI from a different host than the API.
            </div>
          </div>
        </div>
      )}

      {/* ===== Data tab ===== */}
      {tab === 'data' && (
        <div role='tabpanel' aria-label='Data'>
          <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '10px' }}>Export</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: T.spacing.sm }}>
            <button onClick={onExportEvents}
              style={{
                padding: T.spacing.md, background: C.accentBg, border: `1px solid ${C.accentBorder}`,
                color: C.accent, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
              }}>Event log</button>
            <button onClick={onExportConversations}
              style={{
                padding: T.spacing.md, background: C.purpleBg, border: `1px solid ${C.purpleBorder}`,
                color: C.purple, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
              }}>Conversations</button>
          </div>
          <div style={{ marginTop: '8px', fontSize: T.typography.sizeXs, color: C.textDim }}>
            {messageCount} messages across {conversationCount} conversation{conversationCount === 1 ? '' : 's'}.
          </div>
          <button onClick={onExportAllJson}
            style={{
              width: '100%', marginTop: '12px', padding: T.spacing.md,
              background: C.greenBg, border: `1px solid ${C.greenBorder}`,
              color: C.green, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
            }}>Full backup (JSON)</button>
          <div style={{ marginTop: '6px', fontSize: T.typography.sizeXs, color: C.textDim }}>
            Includes all conversations + settings. Schema v1.
          </div>
          {/* c2-241 / #102: import inverse of Full backup. Hidden file input
              behind a styled label so the UI matches the export button without
              a visible <input type=file>. Parent handles schema + merge. */}
          <label htmlFor='lfi-import-backup'
            style={{
              display: 'block', width: '100%', marginTop: '8px', padding: T.spacing.md,
              background: C.accentBg, border: `1px solid ${C.accentBorder}`,
              color: C.accent, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
              fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
              textAlign: 'center', boxSizing: 'border-box',
            }}>Import backup…</label>
          <input id='lfi-import-backup' type='file' accept='application/json,.json'
            style={{ display: 'none' }}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) onImportBackup(file);
              e.target.value = ''; // allow re-selecting the same file
            }} />
          <div style={{ marginTop: '6px', fontSize: T.typography.sizeXs, color: C.textDim }}>
            Merges conversations (by id); settings replaced after confirmation.
          </div>

          {/* c2-425 / task 188: storage usage breakdown. Enumerates localStorage
              keys the app owns + queries navigator.storage.estimate() for the
              browser's overall quota/usage. Computed on mount of this tab
              via a local useState+useEffect so it doesn't recompute every
              render. Hidden entries are zero-byte. */}
          <StorageUsage C={C} />

          <div style={{ marginTop: '22px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
            <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.red, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: '10px' }}>Danger zone</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: T.spacing.sm }}>
              <button onClick={onClearHistory}
                style={{
                  padding: T.spacing.md, background: C.redBg, border: `1px solid ${C.redBorder}`,
                  color: C.red, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
                }}>Clear history</button>
              <button onClick={onResetSettings}
                style={{
                  padding: T.spacing.md, background: 'transparent', border: `1px solid ${C.border}`,
                  color: C.textMuted, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                  fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
                }}>Reset settings</button>
            </div>
            <button onClick={onDeleteAccount}
              style={{
                width: '100%', marginTop: '10px', padding: '12px',
                background: 'transparent', border: `1px solid ${C.redBorder}`,
                color: C.red, borderRadius: T.radii.lg, cursor: 'pointer', fontFamily: 'inherit',
                fontSize: T.typography.sizeXs, fontWeight: 700, textTransform: 'uppercase',
              }}>Delete account &amp; all data</button>
          </div>
        </div>
      )}

      <div style={{ marginTop: '16px', fontSize: T.typography.sizeXs, color: C.textDim, textAlign: 'center' }}>
        Settings save automatically to this browser.
        {/* c2-271: app version footer. Hardcoded from package.json — Vite
            define would be better but the extra config isn't worth it for a
            single string. Bump alongside package.json on release. */}
        <div style={{ marginTop: '4px', fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono, opacity: 0.7 }}>
          PlausiDen v1.0.0
        </div>
      </div>
    </div>
  </div>
  );
};

// c2-425 / task 188: storage-usage breakdown panel. Splits localStorage by
// the keys the app writes + browser-level quota via navigator.storage.
// Reads are on mount / when the "Refresh" link is clicked; not live-updated
// because LS writes are rare and the estimate is expensive enough to not
// poll. Sizes computed as key.length + value.length in chars, then bytes
// via `* 2` for UTF-16 storage (LS spec) — same approach everyone uses.
const StorageUsage: React.FC<{ C: any }> = ({ C }) => {
  type Entry = { key: string; label: string; bytes: number };
  const [entries, setEntries] = React.useState<Entry[]>([]);
  const [quota, setQuota] = React.useState<{ used?: number; total?: number }>({});
  const [tick, setTick] = React.useState(0);
  const refresh = () => setTick(t => t + 1);
  React.useEffect(() => {
    const known: Array<{ key: string; label: string }> = [
      { key: 'lfi_conversations_v2', label: 'Conversations' },
      { key: 'lfi_settings',         label: 'Settings' },
      { key: 'lfi_events',           label: 'Event log' },
      { key: 'lfi_conversations_v1', label: 'Legacy conversations' },
      { key: 'lfi_auth',             label: 'Auth state' },
      { key: 'lfi_current_conversation', label: 'Active conversation id' },
      { key: 'lfi_audit_history_v1', label: 'Audit history' },
      { key: 'lfi_gradebook_history_v1', label: 'Gradebook history' },
      { key: 'lfi_quality_history_v1', label: 'Quality history' },
      { key: 'lfi_avp_history_v1',   label: 'AVP history' },
      { key: 'lfi_cmd_recency_v1',   label: 'Command-palette recency' },
      { key: 'lfi_admin_tab',        label: 'Admin active tab' },
      { key: 'lfi_classroom_sub',    label: 'Classroom active sub-tab' },
      { key: 'lfi_convo_date_filter', label: 'Sidebar date filter' },
    ];
    const next: Entry[] = [];
    for (const k of known) {
      try {
        const v = localStorage.getItem(k.key);
        if (v == null) continue;
        next.push({ key: k.key, label: k.label, bytes: (k.key.length + v.length) * 2 });
      } catch { /* storage blocked */ }
    }
    next.sort((a, b) => b.bytes - a.bytes);
    setEntries(next);
    if (typeof navigator !== 'undefined' && typeof navigator.storage?.estimate === 'function') {
      navigator.storage.estimate().then(e => setQuota({ used: e.usage, total: e.quota })).catch(() => {});
    }
  }, [tick]);
  const totalBytes = entries.reduce((s, e) => s + e.bytes, 0);
  const fmtBytes = (n: number): string => {
    if (n >= 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(2)} MB`;
    if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${n} B`;
  };
  const max = Math.max(...entries.map(e => e.bytes), 1);
  return (
    <div style={{ marginTop: '22px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '10px' }}>
        <div style={{ fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.08em' }}>
          Storage usage
        </div>
        <button onClick={refresh} aria-label='Recompute storage usage' style={{
          background: 'transparent', border: 'none', color: C.accent,
          cursor: 'pointer', fontFamily: 'inherit', fontSize: T.typography.sizeXs,
          padding: '2px 4px',
        }}>Refresh</button>
      </div>
      {entries.length === 0 ? (
        <div style={{ fontSize: T.typography.sizeXs, color: C.textDim }}>
          No PlausiDen keys in this browser yet.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          {entries.map(e => {
            const pct = (e.bytes / max) * 100;
            return (
              <div key={e.key} title={e.key} style={{ display: 'flex', alignItems: 'center', gap: T.spacing.sm }}>
                <span style={{ flex: '0 0 150px', fontSize: T.typography.sizeXs, color: C.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{e.label}</span>
                <div style={{ flex: 1, height: '6px', background: C.bgInput, borderRadius: T.radii.xs, overflow: 'hidden' }}>
                  <div style={{ width: `${pct}%`, height: '100%', background: C.accent }} />
                </div>
                <span style={{ flex: '0 0 72px', textAlign: 'right', fontSize: T.typography.sizeXs, fontFamily: T.typography.fontMono, color: C.textMuted }}>
                  {fmtBytes(e.bytes)}
                </span>
              </div>
            );
          })}
        </div>
      )}
      <div style={{ marginTop: '10px', fontSize: T.typography.sizeXs, color: C.textDim, display: 'flex', justifyContent: 'space-between' }}>
        <span>App total: <strong style={{ color: C.text, fontFamily: T.typography.fontMono }}>{fmtBytes(totalBytes)}</strong></span>
        {typeof quota.used === 'number' && typeof quota.total === 'number' && (
          <span>Browser quota: <strong style={{ color: C.text, fontFamily: T.typography.fontMono }}>
            {fmtBytes(quota.used)} / {fmtBytes(quota.total)} ({((quota.used / quota.total) * 100).toFixed(1)}%)
          </strong></span>
        )}
      </div>
    </div>
  );
};

// c2-433 / #398: workspace capacity control. Slider with preset stops
// at 64 / 128 / 256 / 512 / 1024 / 2048 MB. GETs on mount, PUTs on
// commit, surfaces slots + active footprint + cap. Hidden when the
// endpoint never responds.
const WS_STOPS = [64, 128, 256, 512, 1024, 2048];
const WorkspaceControl: React.FC<{
  C: any; wsHost: string;
  wsData: null | { capacity: number; footprint_bytes: number; footprint_mb: number; per_slot_bytes: number; max_mb?: number };
  setWsData: React.Dispatch<React.SetStateAction<any>>;
  wsLoading: boolean; setWsLoading: React.Dispatch<React.SetStateAction<boolean>>;
  wsErr: string | null; setWsErr: React.Dispatch<React.SetStateAction<string | null>>;
  wsSaving: boolean; setWsSaving: React.Dispatch<React.SetStateAction<boolean>>;
  wsMaxMb: number; setWsMaxMb: React.Dispatch<React.SetStateAction<number>>;
}> = ({ C, wsHost, wsData, setWsData, wsLoading, setWsLoading, wsErr, setWsErr, wsSaving, setWsSaving, wsMaxMb, setWsMaxMb }) => {
  useEffect(() => {
    let cancelled = false;
    (async () => {
      setWsLoading(true);
      setWsErr(null);
      try {
        const r = await fetch(`http://${wsHost}:3000/api/settings/workspace`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const data = await r.json();
        if (cancelled) return;
        setWsData(data);
        const mb = typeof data?.max_mb === 'number' ? data.max_mb
          : typeof data?.footprint_mb === 'number' ? data.footprint_mb
          : 512;
        // Snap to the nearest preset stop for the slider's initial position.
        const snapped = WS_STOPS.reduce((best, s) => Math.abs(s - mb) < Math.abs(best - mb) ? s : best, WS_STOPS[0]);
        setWsMaxMb(snapped);
      } catch (e: any) {
        if (!cancelled) setWsErr(String(e?.message || e || 'fetch failed'));
      } finally {
        if (!cancelled) setWsLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [wsHost]);

  // Hide entirely when endpoint is unreachable — keeps Settings tab clean
  // on older deployments without #398.
  if (wsErr && !wsData) return null;

  const commit = async (mb: number) => {
    setWsSaving(true);
    setWsErr(null);
    try {
      const r = await fetch(`http://${wsHost}:3000/api/settings/workspace`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ max_mb: mb }),
      });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const data = await r.json();
      setWsData(data);
    } catch (e: any) {
      setWsErr(String(e?.message || e || 'save failed'));
    } finally {
      setWsSaving(false);
    }
  };

  const fmtSlots = (n: number): string => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}k`;
    return String(n);
  };

  const capacity = wsData?.capacity ?? 0;
  const activeMb = wsData?.footprint_mb ?? 0;
  const perSlot = wsData?.per_slot_bytes ?? 0;

  return (
    <div style={{ marginTop: '18px', paddingTop: '16px', borderTop: `1px solid ${C.borderSubtle}` }}>
      <label style={{
        fontSize: T.typography.sizeXs, fontWeight: 700, color: C.textMuted,
        textTransform: 'uppercase', letterSpacing: '0.08em',
      }}>Workspace capacity</label>
      <div style={{ fontSize: T.typography.sizeXs, color: C.textDim, marginTop: '4px', lineHeight: 1.5 }}>
        Global Workspace holds the active hypervector slots for chat + reasoning.
        Higher caps let longer conversations retain context at the cost of RAM.
      </div>
      {wsLoading && !wsData ? (
        <div style={{ padding: '8px 0', fontSize: T.typography.sizeXs, color: C.textMuted, fontStyle: 'italic' }}>
          Loading workspace state…
        </div>
      ) : (
        <div style={{ marginTop: '10px' }}>
          <div style={{
            display: 'flex', alignItems: 'baseline', gap: T.spacing.sm,
            fontFamily: T.typography.fontMono, fontSize: T.typography.sizeSm,
            color: C.text, marginBottom: '8px',
          }}>
            <span style={{ color: C.accent, fontWeight: 700 }}>{fmtSlots(capacity)} slots</span>
            <span style={{ color: C.textMuted }}>
              · {activeMb.toFixed(activeMb < 10 ? 1 : 0)} MB active · {wsMaxMb} MB cap
            </span>
            {perSlot > 0 && (
              <span style={{ color: C.textDim, fontSize: '10px', marginLeft: 'auto' }}>
                {(perSlot / 1024).toFixed(1)}KB/slot
              </span>
            )}
          </div>
          <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
            {WS_STOPS.map(mb => {
              const active = wsMaxMb === mb;
              return (
                <button key={mb} type='button'
                  onClick={() => { setWsMaxMb(mb); commit(mb); }}
                  disabled={wsSaving}
                  title={`${mb} MB cap (${fmtSlots(Math.floor((mb * 1024 * 1024) / Math.max(perSlot, 1)))} slots at current per-slot size)`}
                  style={{
                    padding: '3px 10px', fontSize: '10px',
                    fontWeight: T.typography.weightBold,
                    background: active ? C.accentBg : 'transparent',
                    border: `1px solid ${active ? C.accentBorder : C.borderSubtle}`,
                    color: active ? C.accent : C.textMuted,
                    borderRadius: T.radii.sm,
                    cursor: wsSaving ? 'wait' : 'pointer',
                    fontFamily: T.typography.fontMono,
                    letterSpacing: '0.04em',
                  }}>
                  {mb < 1024 ? `${mb} MB` : `${mb / 1024} GB`}
                </button>
              );
            })}
          </div>
          {wsSaving && (
            <div style={{ fontSize: '10px', color: C.textMuted, marginTop: '6px', fontStyle: 'italic' }}>
              Saving…
            </div>
          )}
          {wsErr && (
            <div style={{ fontSize: '10px', color: C.red, marginTop: '6px', fontFamily: T.typography.fontMono }}>
              {wsErr}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
