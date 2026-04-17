import React from 'react';
import { T } from './tokens';

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
    background: C.bg, padding: isMobile ? T.spacing.xl : T.spacing.xxxl,
    fontFamily: C.font,
  }}>
    <div style={{
      width: '100%', maxWidth: isDesktop ? '440px' : '400px',
      padding: isDesktop ? T.spacing.xxxl : T.spacing.xxl,
      background: C.bgCard, border: `1px solid ${C.accentBorder}`,
      borderRadius: T.radii.xxl,
      boxShadow: T.shadows.modalDeep,
    }}>
      <div style={{ textAlign: 'center', marginBottom: '28px' }}>
        <div style={{
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          width: '72px', height: '72px', borderRadius: T.radii.round,
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
        fontSize: T.typography.sizeXl, fontWeight: T.typography.weightBlack, textAlign: 'center',
        letterSpacing: T.typography.trackingCapWide, textTransform: 'uppercase',
        color: C.text, marginBottom: '6px',
      }}>Sovereign Command Console</h1>
      <p style={{ fontSize: T.typography.sizeMd, textAlign: 'center', color: C.textMuted, marginBottom: T.spacing.xxl }}>
        Enter your sovereign key to authenticate
      </p>
      <input
        type="password" autoFocus
        autoComplete="current-password" spellCheck={false} aria-label="Sovereign key"
        style={{
          width: '100%', padding: '14px 16px',
          background: 'rgba(0,0,0,0.3)', border: `1px solid ${C.accentBorder}`,
          borderRadius: T.radii.lg, outline: 'none', color: C.text,
          fontSize: T.typography.sizeXl, fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: T.spacing.md,
        }}
        placeholder="AUTH_KEY"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && onLogin()}
      />
      {authError && (
        <p style={{
          color: C.red, fontSize: T.typography.sizeMd, textAlign: 'center', marginBottom: T.spacing.md,
          padding: T.spacing.sm, background: C.redBg, borderRadius: T.radii.md,
          border: `1px solid ${C.redBorder}`,
        }}>{authError}</p>
      )}
      <button onClick={onLogin} disabled={authLoading || !password}
        style={{
          width: '100%', padding: '14px',
          background: C.accentBg, border: `1px solid ${C.accentBorder}`,
          borderRadius: T.radii.lg, color: C.accent, fontSize: T.typography.sizeBody, fontWeight: T.typography.weightBlack,
          textTransform: 'uppercase', letterSpacing: T.typography.trackingCap,
          cursor: authLoading ? 'wait' : 'pointer', fontFamily: 'inherit',
          opacity: !password ? 0.4 : 1,
          transition: `all ${T.motion.base}`,
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
