import React from 'react';

// Shown in the chat area when there are no messages yet. Four quick-start
// prompts and a logo. Parent owns the input textarea + ref; we just pre-fill.
export interface WelcomeScreenProps {
  C: any;
  isDesktop: boolean;
  onPickPrompt: (text: string) => void;
}

const QUICK_STARTS: { t: string; p: string }[] = [
  { t: 'Say hi', p: "hey, what's up?" },
  { t: 'Ask for help', p: 'help me plan my day' },
  { t: 'Write code', p: 'write a Rust function to reverse a string' },
  { t: 'Explain something', p: 'explain how WireGuard works' },
];

export const WelcomeScreen: React.FC<WelcomeScreenProps> = ({ C, isDesktop, onPickPrompt }) => (
  <div style={{ textAlign: 'center', padding: isDesktop ? '80px 24px 40px' : '48px 24px 24px' }}>
    <pre style={{
      margin: '0 auto 18px',
      color: C.accent,
      fontFamily: C.font,
      fontSize: isDesktop ? '11px' : '9px',
      lineHeight: 1.15,
      letterSpacing: '0.5px',
      textShadow: `0 0 12px ${C.accentGlow}`,
      animation: 'lfi-fadein 0.5s ease-out',
    }}>
    PlausiDen <span style={{ opacity: 0.7 }}>AI</span>
    </pre>
    <p style={{ fontSize: '22px', fontWeight: 700, color: C.text, margin: '0 0 6px' }}>
      PlausiDen <span style={{ color: C.accent }}>AI</span>
    </p>
    <p style={{ fontSize: '14px', color: C.textMuted, margin: '0 0 8px', fontWeight: 500 }}>How can I help?</p>
    <p style={{ fontSize: '14px', color: C.textMuted, marginTop: 0, marginBottom: '20px', marginLeft: 'auto', marginRight: 'auto', maxWidth: '420px' }}>
      Code, research, planning, analysis &mdash; or just chat. I remember what we talk about across sessions.
    </p>
    <div style={{
      display: 'grid',
      gridTemplateColumns: isDesktop ? 'repeat(2, 1fr)' : '1fr',
      gap: '10px', maxWidth: '600px', margin: '0 auto',
    }}>
      {QUICK_STARTS.map(s => (
        <button key={s.t}
          onClick={() => onPickPrompt(s.p)}
          aria-label={`${s.t}: ${s.p}`}
          style={{
            textAlign: 'left', padding: '12px 14px', borderRadius: '10px',
            background: C.bgCard, border: `1px solid ${C.border}`, cursor: 'pointer',
            fontFamily: 'inherit', color: C.text,
            transition: 'border-color 0.15s',
          }}
          onMouseEnter={(e) => (e.currentTarget.style.borderColor = C.accentBorder)}
          onMouseLeave={(e) => (e.currentTarget.style.borderColor = C.border)}
        >
          <div style={{ fontSize: '12px', color: C.accent, fontWeight: 700, marginBottom: '4px' }}>{s.t}</div>
          <div style={{ fontSize: '13px', color: C.textSecondary }}>{s.p}</div>
        </button>
      ))}
    </div>
  </div>
);
