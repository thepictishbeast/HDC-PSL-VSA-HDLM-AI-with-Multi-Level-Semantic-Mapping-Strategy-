import React from 'react';

// Sidebar "Administration" button cluster. Parent owns loading state + handlers.
export interface AdminActionsProps {
  C: any;
  adminLoading: string;
  onFetchFacts: () => void;
  onFetchQos: () => void;
  onClearChat: () => void;
  onOpenSettings: () => void;
  children?: React.ReactNode; // lets the parent slot FactsPanel + QosPanel below the buttons
}

export const AdminActions: React.FC<AdminActionsProps> = ({
  C, adminLoading, onFetchFacts, onFetchQos, onClearChat, onOpenSettings, children,
}) => (
  <div style={{ padding: '20px' }}>
    <div style={{ fontSize: '11px', fontWeight: 800, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.12em', marginBottom: '14px' }}>
      Administration
    </div>
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
      <button onClick={onFetchFacts} disabled={adminLoading === 'facts'} style={{
        padding: '10px', fontSize: '12px', fontWeight: 700, color: C.accent,
        background: C.accentBg, border: `1px solid ${C.accentBorder}`, borderRadius: '8px',
        cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: '0.05em',
      }}>{adminLoading === 'facts' ? 'Loading...' : 'View Facts'}</button>
      <button onClick={onFetchQos} disabled={adminLoading === 'qos'} style={{
        padding: '10px', fontSize: '12px', fontWeight: 700, color: C.purple,
        background: C.purpleBg, border: `1px solid ${C.purpleBorder}`, borderRadius: '8px',
        cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: '0.05em',
      }}>{adminLoading === 'qos' ? 'Loading...' : 'QoS Report'}</button>
      <button onClick={onClearChat} style={{
        padding: '10px', fontSize: '12px', fontWeight: 700, color: C.textMuted,
        background: 'transparent', border: `1px solid ${C.border}`, borderRadius: '8px',
        cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: '0.05em',
      }}>Clear Chat</button>
      <button onClick={onOpenSettings} style={{
        padding: '10px', fontSize: '12px', fontWeight: 700, color: C.accent,
        background: 'transparent', border: `1px solid ${C.accentBorder}`, borderRadius: '8px',
        cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase', letterSpacing: '0.05em',
      }}>Settings</button>
    </div>
    {children}
  </div>
);
