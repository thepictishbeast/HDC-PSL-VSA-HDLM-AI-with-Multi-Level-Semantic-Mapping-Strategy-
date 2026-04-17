import React from 'react';

// Shown in the chat area when there are no messages yet. Six quick-start
// prompts, minimal copy. Parent owns the input textarea + ref; we pre-fill.
// c0-020: onboarding state — welcoming, professional, prompt cards as
// clickable launchpads.
export interface WelcomeScreenProps {
  C: any;
  isDesktop: boolean;
  onPickPrompt: (text: string) => void;
}

// c0-023 fix: prior preset prompts produced poor responses from the
// backend. Claude 0 recommended simpler, more direct starters that let the
// AI lean on its RAG/knowledge context. Keep every starter answerable with
// the facts we've ingested.
const QUICK_STARTS: { t: string; p: string }[] = [
  { t: 'Capabilities', p: 'What can you do? List your skills and how you work.' },
  { t: 'Security check', p: 'Help me think through the security of my Linux setup.' },
  { t: 'Analyse my system', p: 'Walk me through interpreting my CPU, RAM, and disk usage.' },
  { t: 'Explain a topic', p: 'What do you know about sovereign AI and local-first systems?' },
  { t: 'Code help', p: 'Help me debug a Rust program. I will paste the error next.' },
  { t: 'Learn something', p: 'Teach me something useful about networking I probably do not know.' },
];

export const WelcomeScreen: React.FC<WelcomeScreenProps> = ({ C, isDesktop, onPickPrompt }) => (
  <div style={{ textAlign: 'center', padding: isDesktop ? '72px 24px 40px' : '40px 20px 24px' }}>
    <h1 style={{ fontSize: isDesktop ? '28px' : '22px', fontWeight: 600, color: C.text, margin: '0 0 8px', letterSpacing: '-0.01em' }}>
      PlausiDen <span style={{ color: C.accent }}>AI</span>
    </h1>
    <p style={{ fontSize: '14px', color: C.textSecondary, margin: '0 0 28px', maxWidth: '440px', marginLeft: 'auto', marginRight: 'auto', lineHeight: 1.55 }}>
      Sovereign AI that runs on your hardware. Private by default, remembers across sessions.
    </p>
    <div style={{
      display: 'grid',
      gridTemplateColumns: isDesktop ? 'repeat(3, 1fr)' : 'repeat(2, 1fr)',
      gap: '10px', maxWidth: '720px', margin: '0 auto',
    }}>
      {QUICK_STARTS.map(s => (
        <button key={s.t}
          onClick={() => onPickPrompt(s.p)}
          aria-label={`${s.t}: ${s.p}`}
          style={{
            textAlign: 'left', padding: '14px 16px', borderRadius: '6px',
            background: C.bgCard, border: `1px solid ${C.border}`, cursor: 'pointer',
            fontFamily: 'inherit', color: C.text,
            transition: 'border-color 0.12s, background 0.12s',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.borderColor = C.accent; e.currentTarget.style.background = C.bgHover; }}
          onMouseLeave={(e) => { e.currentTarget.style.borderColor = C.border; e.currentTarget.style.background = C.bgCard; }}
        >
          <div style={{ fontSize: '11px', color: C.accent, fontWeight: 600, marginBottom: '6px', textTransform: 'uppercase', letterSpacing: '0.04em' }}>{s.t}</div>
          <div style={{ fontSize: '13px', color: C.textSecondary, lineHeight: 1.5 }}>{s.p}</div>
        </button>
      ))}
    </div>
  </div>
);
