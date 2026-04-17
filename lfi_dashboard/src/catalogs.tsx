import React from 'react';

// Skill IDs mirrored from App.tsx. Kept in sync by convention — any new skill
// added here must also be added to the Skill union type in App.tsx.
export type Skill = 'chat' | 'research' | 'web' | 'image' | 'code' | 'analyze' | 'opsec';

export interface SkillMeta {
  id: Skill;
  label: string;
  hint: string;
  available: boolean;
  icon: React.ReactNode;
}

export const SKILLS: SkillMeta[] = [
  {
    id: 'chat', label: 'Chat', hint: 'Regular conversation', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>,
  },
  {
    id: 'research', label: 'Deep Research', hint: 'Multi-source investigation with citations', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>,
  },
  {
    id: 'web', label: 'Web Search', hint: 'Live web results with trust scoring', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>,
  },
  {
    id: 'image', label: 'Image', hint: 'Describe an image to generate (requires local SD/ComfyUI)', available: false,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>,
  },
  {
    id: 'code', label: 'Code', hint: 'BigBrain tier, code-first answers', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>,
  },
  {
    id: 'analyze', label: 'Analyze', hint: 'PSL-supervised structured audit', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 3v18h18"/><path d="M7 14l4-4 4 4 5-5"/></svg>,
  },
  {
    id: 'opsec', label: 'OPSEC Scan', hint: 'Scan text for PII, secrets, credentials', available: true,
    icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>,
  },
];

// Avatar gradients — picked so they look good on both dark and light themes.
export const AVATAR_PRESETS: string[] = [
  'linear-gradient(135deg, #8b7cff, #e879f9)',   // violet → magenta
  'linear-gradient(135deg, #22c55e, #0ea5e9)',   // emerald → sky
  'linear-gradient(135deg, #f97316, #eab308)',   // orange → amber
  'linear-gradient(135deg, #ec4899, #ff6b81)',   // pink → coral
  'linear-gradient(135deg, #0ea5e9, #8b5cf6)',   // sky → violet
  'linear-gradient(135deg, #64748b, #0f172a)',   // slate → ink (muted)
  'linear-gradient(135deg, #facc15, #fb923c)',   // gold
  'linear-gradient(135deg, #14b8a6, #22d3ee)',   // teal → cyan
];
