import React from 'react';

// Login gate shown when isAuthenticated is false. Kept as a pure render — all
// state + handlers come in via props. Theme/palette passed as `C` opaquely so
// this stays unaware of the palette definitions.
export interface LoginScreenProps {
  C: any;
  isMobile: boolean;
  isDesktop: boolean;
  password: string;
  setPassword: (v: string) => void;
  authError: string;
  authLoading: boolean;
  onLogin: () => void;
}

export const LoginScreen: React.FC<LoginScreenProps> = ({
  C, isMobile, isDesktop, password, setPassword, authError, authLoading, onLogin,
}) => (
  <div style={{
    display: 'flex', alignItems: 'center', justifyContent: 'center',
    minHeight: '100vh', width: '100%',
    background: C.bg, padding: isMobile ? '24px' : '48px',
    fontFamily: C.font,
  }}>
    <div style={{
      width: '100%', maxWidth: isDesktop ? '440px' : '400px',
      padding: isDesktop ? '48px' : '32px',
      background: C.bgCard, border: `1px solid ${C.accentBorder}`,
      borderRadius: '16px',
      boxShadow: '0 12px 48px rgba(0,0,0,0.6)',
    }}>
      <div style={{ textAlign: 'center', marginBottom: '28px' }}>
        <div style={{
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          width: '72px', height: '72px', borderRadius: '50%',
          background: C.accentBg, border: `2px solid ${C.accentBorder}`,
          boxShadow: `0 0 24px ${C.accentGlow}`,
        }}>
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke={C.accent} strokeWidth="1.5">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
            <path d="M12 8v4M12 16h.01"/>
          </svg>
        </div>
      </div>
      <h1 style={{
        fontSize: '16px', fontWeight: 800, textAlign: 'center',
        letterSpacing: '0.2em', textTransform: 'uppercase',
        color: C.text, marginBottom: '6px',
      }}>Sovereign Command Console</h1>
      <p style={{ fontSize: '13px', textAlign: 'center', color: C.textMuted, marginBottom: '32px' }}>
        Enter your sovereign key to authenticate
      </p>
      <input
        type="password" autoFocus
        style={{
          width: '100%', padding: '14px 16px',
          background: 'rgba(0,0,0,0.3)', border: `1px solid ${C.accentBorder}`,
          borderRadius: '10px', outline: 'none', color: C.text,
          fontSize: '16px', fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: '12px',
        }}
        placeholder="AUTH_KEY"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && onLogin()}
      />
      {authError && (
        <p style={{
          color: C.red, fontSize: '13px', textAlign: 'center', marginBottom: '12px',
          padding: '10px', background: C.redBg, borderRadius: '8px',
          border: `1px solid ${C.redBorder}`,
        }}>{authError}</p>
      )}
      <button onClick={onLogin} disabled={authLoading || !password}
        style={{
          width: '100%', padding: '14px',
          background: C.accentBg, border: `1px solid ${C.accentBorder}`,
          borderRadius: '10px', color: C.accent, fontSize: '14px', fontWeight: 800,
          textTransform: 'uppercase', letterSpacing: '0.15em',
          cursor: authLoading ? 'wait' : 'pointer', fontFamily: 'inherit',
          opacity: !password ? 0.4 : 1,
          transition: 'all 0.2s',
        }}>
        {authLoading ? 'Authenticating...' : 'Initiate Link'}
      </button>
    </div>
    <style>{`
      * { box-sizing: border-box; }
      body { margin: 0; padding: 0; }
      input::placeholder { color: ${C.textDim}; }
    `}</style>
  </div>
);
