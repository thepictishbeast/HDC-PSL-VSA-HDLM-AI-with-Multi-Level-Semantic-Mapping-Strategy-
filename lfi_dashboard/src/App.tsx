// ============================================================
// Sovereign Command Console (SCC) v3.0 — Mobile-First Dashboard
//
// PROTOCOL: Real-time WebSocket integration with LFI Cognitive Core
// SUBSTRATE: React, inline styles (no CSS framework dependency)
// LAYOUT: Mobile-first single column, scales to desktop
//
// ENDPOINTS:
//   ws://<host>:3000/ws/chat       — Bidirectional cognitive chat
//   ws://<host>:3000/ws/telemetry  — Real-time substrate telemetry
//   POST /api/auth                 — Sovereign key verification
//   GET  /api/status               — Substrate status
//   GET  /api/qos                  — QoS compliance report
//
// DEBUG: console.debug() on every state change for Eruda inspector
// ============================================================

import React, { useState, useEffect, useRef, useCallback } from 'react';

// ---- Types ----
interface ChatMessage {
  id: number;
  role: 'user' | 'assistant' | 'system' | 'web';
  content: string;
  mode?: string;
  confidence?: number;
  tier?: string;
  intent?: string;
  reasoning?: string[];
  plan?: { steps: number; complexity: number; goal: string };
  timestamp: number;
}

interface SubstrateStats {
  ram_available_mb: number;
  cpu_temp_c: number;
  vsa_orthogonality: number;
  axiom_pass_rate: number;
  is_throttled: boolean;
  logic_density: number;
}

// ---- Main Component ----
const SovereignCommandConsole: React.FC = () => {
  console.debug("// SCC v3.0: Component mounting...");

  // ---- State ----
  const [isAuthenticated, setIsAuthenticated] = useState(() => {
    const stored = localStorage.getItem('lfi_auth') === 'true';
    console.debug("// SCC: Auth from localStorage:", stored);
    return stored;
  });
  const [password, setPassword] = useState('');
  const [authError, setAuthError] = useState('');
  const [authLoading, setAuthLoading] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const [expandedReasoning, setExpandedReasoning] = useState<number | null>(null);
  const [showTelemetry, setShowTelemetry] = useState(false);
  const [stats, setStats] = useState<SubstrateStats>({
    ram_available_mb: 0, cpu_temp_c: 0, vsa_orthogonality: 0.02,
    axiom_pass_rate: 1.0, is_throttled: false, logic_density: 0
  });

  const chatWsRef = useRef<WebSocket | null>(null);
  const telemetryWsRef = useRef<WebSocket | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // ---- Helpers ----
  const getHost = () => {
    const h = window.location.hostname || '127.0.0.1';
    console.debug("// SCC: Resolved host:", h);
    return h;
  };

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => { scrollToBottom(); }, [messages, scrollToBottom]);

  useEffect(() => {
    console.debug("// SCC: Persisting auth:", isAuthenticated);
    localStorage.setItem('lfi_auth', isAuthenticated.toString());
  }, [isAuthenticated]);

  // ---- WebSocket: Chat ----
  useEffect(() => {
    if (!isAuthenticated) {
      console.debug("// SCC: Skipping chat WS — not authenticated");
      return;
    }
    const wsUrl = `ws://${getHost()}:3000/ws/chat`;
    console.debug("// SCC: Connecting chat WS:", wsUrl);
    let reconnectTimer: ReturnType<typeof setTimeout>;

    const connect = () => {
      console.debug("// SCC: chat WS connect()");
      const ws = new WebSocket(wsUrl);
      chatWsRef.current = ws;

      ws.onopen = () => {
        console.debug("// SCC: Chat WS OPEN");
        setIsConnected(true);
        setMessages(prev => [...prev, {
          id: Date.now(), role: 'system', content: 'Cognitive link established.',
          timestamp: Date.now()
        }]);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          console.debug("// SCC: Chat msg:", msg.type);

          if (msg.type === 'chat_response') {
            setIsThinking(false);
            setMessages(prev => [...prev, {
              id: Date.now(), role: 'assistant',
              content: msg.content || '',
              mode: msg.mode, confidence: msg.confidence,
              tier: msg.tier, intent: msg.intent,
              reasoning: msg.reasoning, plan: msg.plan,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'web_result') {
            console.debug("// SCC: Web result, sources:", msg.source_count);
            setMessages(prev => [...prev, {
              id: Date.now(), role: 'web',
              content: `${msg.source_count} sources | trust: ${(msg.trust * 100).toFixed(0)}%\n\n${msg.summary}`,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'chat_error') {
            console.debug("// SCC: Chat error:", msg.error);
            setIsThinking(false);
            setMessages(prev => [...prev, {
              id: Date.now(), role: 'system',
              content: `Error: ${msg.error}`, timestamp: Date.now(),
            }]);
          }
        } catch (e) {
          console.error("// SCC: Chat parse error:", e);
        }
      };

      ws.onclose = (ev) => {
        console.debug("// SCC: Chat WS CLOSED:", ev.code);
        setIsConnected(false);
        reconnectTimer = setTimeout(connect, 3000);
      };

      ws.onerror = (ev) => {
        console.error("// SCC: Chat WS ERROR:", ev);
        setIsConnected(false);
      };
    };

    connect();
    return () => { clearTimeout(reconnectTimer); chatWsRef.current?.close(); };
  }, [isAuthenticated]);

  // ---- WebSocket: Telemetry ----
  useEffect(() => {
    if (!isAuthenticated) return;
    const wsUrl = `ws://${getHost()}:3000/ws/telemetry`;
    console.debug("// SCC: Connecting telemetry WS:", wsUrl);
    let reconnectTimer: ReturnType<typeof setTimeout>;

    const connect = () => {
      const ws = new WebSocket(wsUrl);
      telemetryWsRef.current = ws;
      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          if (msg.type === 'telemetry' && msg.data) {
            setStats(prev => ({ ...prev, ...msg.data }));
          }
        } catch (e) { console.error("// SCC: Telemetry parse error:", e); }
      };
      ws.onclose = () => { reconnectTimer = setTimeout(connect, 5000); };
      ws.onerror = (ev) => console.error("// SCC: Telemetry WS ERROR:", ev);
    };

    connect();
    return () => { clearTimeout(reconnectTimer); telemetryWsRef.current?.close(); };
  }, [isAuthenticated]);

  // ---- Auth ----
  const handleLogin = async () => {
    console.debug("// SCC: handleLogin");
    setAuthError('');
    setAuthLoading(true);
    try {
      const url = `http://${getHost()}:3000/api/auth`;
      console.debug("// SCC: POST", url);
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: password }),
      });
      const data = await res.json();
      console.debug("// SCC: Auth response:", data);
      if (data.status === 'authenticated') setIsAuthenticated(true);
      else setAuthError('Sovereign key rejected.');
    } catch (e) {
      console.error("// SCC: Auth error:", e);
      setAuthError('Backend unreachable. Is the server running on port 3000?');
    } finally { setAuthLoading(false); }
  };

  const handleLogout = () => {
    console.debug("// SCC: Logout");
    localStorage.removeItem('lfi_auth');
    chatWsRef.current?.close();
    telemetryWsRef.current?.close();
    setIsAuthenticated(false);
    setMessages([]);
  };

  // ---- Send ----
  const handleSend = () => {
    const trimmed = input.trim();
    console.debug("// SCC: handleSend, len:", trimmed.length, "wsState:", chatWsRef.current?.readyState);
    if (!trimmed || !chatWsRef.current || chatWsRef.current.readyState !== WebSocket.OPEN) return;

    setMessages(prev => [...prev, {
      id: Date.now(), role: 'user', content: trimmed, timestamp: Date.now()
    }]);
    chatWsRef.current.send(JSON.stringify({ content: trimmed }));
    console.debug("// SCC: Sent to WS");
    setIsThinking(true);
    setInput('');
    inputRef.current?.focus();
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 160) + 'px';
  };

  const formatTime = (ts: number) => new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  // ============================================================
  // RENDER: Login
  // ============================================================
  if (!isAuthenticated) {
    console.debug("// SCC: Rendering login");
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        minHeight: '100vh', width: '100%',
        background: '#050508', padding: '24px',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      }}>
        <div style={{
          width: '100%', maxWidth: '400px', padding: '32px',
          background: '#0c0c14', border: '1px solid rgba(59,130,246,0.15)',
          borderRadius: '16px',
        }}>
          <div style={{ textAlign: 'center', marginBottom: '24px' }}>
            <div style={{
              display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
              width: '64px', height: '64px', borderRadius: '50%',
              background: 'rgba(59,130,246,0.08)', border: '1px solid rgba(59,130,246,0.2)',
            }}>
              <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="1.5">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                <path d="M12 8v4M12 16h.01"/>
              </svg>
            </div>
          </div>
          <h1 style={{
            fontSize: '13px', fontWeight: 700, textAlign: 'center',
            letterSpacing: '0.25em', textTransform: 'uppercase',
            color: '#93c5fd', marginBottom: '8px',
          }}>Sovereign Command Console</h1>
          <p style={{ fontSize: '12px', textAlign: 'center', color: '#475569', marginBottom: '28px' }}>
            Enter your sovereign key to authenticate
          </p>
          <input
            type="password" autoFocus
            style={{
              width: '100%', padding: '14px 16px',
              background: 'rgba(0,0,0,0.4)', border: '1px solid rgba(59,130,246,0.2)',
              borderRadius: '10px', outline: 'none', color: '#93c5fd',
              fontSize: '16px', fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: '12px',
            }}
            placeholder="AUTH_KEY"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleLogin()}
          />
          {authError && (
            <p style={{
              color: '#ef4444', fontSize: '13px', textAlign: 'center', marginBottom: '12px',
              padding: '8px', background: 'rgba(239,68,68,0.08)', borderRadius: '8px',
            }}>{authError}</p>
          )}
          <button onClick={handleLogin} disabled={authLoading || !password}
            style={{
              width: '100%', padding: '14px',
              background: 'rgba(59,130,246,0.15)', border: '1px solid rgba(59,130,246,0.3)',
              borderRadius: '10px', color: '#93c5fd', fontSize: '14px', fontWeight: 700,
              textTransform: 'uppercase', letterSpacing: '0.15em',
              cursor: authLoading ? 'wait' : 'pointer', fontFamily: 'inherit',
              opacity: !password ? 0.4 : 1,
            }}>
            {authLoading ? 'Authenticating...' : 'Initiate Link'}
          </button>
        </div>
      </div>
    );
  }

  // ============================================================
  // RENDER: Main Console
  // ============================================================
  console.debug("// SCC: Rendering console, msgs:", messages.length);

  return (
    <div style={{
      display: 'flex', flexDirection: 'column', height: '100vh', width: '100%',
      background: '#050508', color: '#e0e0e0',
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      overflow: 'hidden',
    }}>
      {/* HEADER */}
      <header style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '10px 16px', background: '#0a0a10',
        borderBottom: '1px solid rgba(255,255,255,0.06)',
        flexShrink: 0, zIndex: 50, minHeight: '48px',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <div style={{
            width: '10px', height: '10px', borderRadius: '50%',
            background: isConnected ? '#3b82f6' : '#ef4444',
            boxShadow: isConnected ? '0 0 8px rgba(59,130,246,0.5)' : '0 0 8px rgba(239,68,68,0.5)',
          }} />
          <span style={{
            fontSize: '12px', fontWeight: 800, letterSpacing: '0.1em', textTransform: 'uppercase',
            color: isConnected ? '#93c5fd' : '#f87171',
          }}>{isConnected ? 'SCC Online' : 'Disconnected'}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <button onClick={() => setShowTelemetry(!showTelemetry)} style={{
            padding: '6px 10px', fontSize: '11px', fontWeight: 700,
            background: showTelemetry ? 'rgba(59,130,246,0.15)' : 'transparent',
            border: '1px solid rgba(255,255,255,0.1)', borderRadius: '6px',
            color: '#94a3b8', cursor: 'pointer', fontFamily: 'inherit',
            textTransform: 'uppercase',
          }}>Stats</button>
          <button onClick={handleLogout} style={{
            padding: '6px 10px', fontSize: '11px', fontWeight: 700,
            background: 'transparent', border: '1px solid rgba(255,255,255,0.08)',
            borderRadius: '6px', color: '#64748b', cursor: 'pointer', fontFamily: 'inherit',
            textTransform: 'uppercase',
          }}>Logout</button>
        </div>
      </header>

      {/* TELEMETRY PANEL */}
      {showTelemetry && (
        <div style={{
          display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)',
          gap: '8px', padding: '12px 16px', background: '#08080d',
          borderBottom: '1px solid rgba(255,255,255,0.04)', flexShrink: 0,
        }}>
          {[
            { label: 'RAM', value: `${stats.ram_available_mb}`, unit: 'MB', color: '#93c5fd', bg: 'rgba(59,130,246,0.06)', border: 'rgba(59,130,246,0.1)' },
            { label: 'CPU', value: `${stats.cpu_temp_c.toFixed(0)}`, unit: '°C', color: stats.cpu_temp_c > 65 ? '#f87171' : '#4ade80', bg: stats.cpu_temp_c > 65 ? 'rgba(239,68,68,0.06)' : 'rgba(34,197,94,0.06)', border: stats.cpu_temp_c > 65 ? 'rgba(239,68,68,0.1)' : 'rgba(34,197,94,0.1)' },
            { label: 'VSA Health', value: `${(100 - stats.vsa_orthogonality * 100).toFixed(1)}`, unit: '%', color: '#c084fc', bg: 'rgba(168,85,247,0.06)', border: 'rgba(168,85,247,0.1)' },
            { label: 'PSL Axioms', value: `${(stats.axiom_pass_rate * 100).toFixed(0)}`, unit: '%', color: '#4ade80', bg: 'rgba(34,197,94,0.06)', border: 'rgba(34,197,94,0.1)' },
          ].map(s => (
            <div key={s.label} style={{ padding: '10px 12px', borderRadius: '8px', background: s.bg, border: `1px solid ${s.border}` }}>
              <div style={{ fontSize: '10px', color: '#64748b', fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>{s.label}</div>
              <div style={{ fontSize: '18px', fontWeight: 800, color: s.color }}>{s.value}<span style={{ fontSize: '11px', color: '#475569' }}>{s.unit}</span></div>
            </div>
          ))}
          {stats.is_throttled && (
            <div style={{
              gridColumn: '1 / -1', padding: '10px', background: 'rgba(239,68,68,0.1)',
              border: '1px solid rgba(239,68,68,0.2)', borderRadius: '8px',
              textAlign: 'center', fontSize: '12px', fontWeight: 700, color: '#f87171', textTransform: 'uppercase',
            }}>Thermal Throttle Active</div>
          )}
        </div>
      )}

      {/* CHAT AREA */}
      <main style={{ flex: 1, overflowY: 'auto', padding: '16px', WebkitOverflowScrolling: 'touch' as any }}>
        <div style={{ maxWidth: '720px', margin: '0 auto' }}>
          {messages.length === 0 && (
            <div style={{ textAlign: 'center', padding: '48px 24px', color: '#334155' }}>
              <div style={{ fontSize: '32px', marginBottom: '16px', opacity: 0.3 }}>&#9670;</div>
              <p style={{ fontSize: '14px', fontWeight: 600 }}>Sovereign Command Console</p>
              <p style={{ fontSize: '12px', marginTop: '8px', color: '#1e293b' }}>Type a message to begin</p>
            </div>
          )}

          {messages.map((msg) => (
            <div key={msg.id} style={{ marginBottom: '16px' }}>
              {msg.role === 'system' && (
                <div style={{ textAlign: 'center', padding: '6px 12px', fontSize: '12px', color: '#475569', fontStyle: 'italic' }}>
                  {msg.content}
                </div>
              )}
              {msg.role === 'web' && (
                <div style={{
                  padding: '14px', borderRadius: '12px',
                  background: 'rgba(16,185,129,0.06)', border: '1px solid rgba(16,185,129,0.15)',
                }}>
                  <div style={{ fontSize: '11px', fontWeight: 700, color: '#10b981', textTransform: 'uppercase', marginBottom: '8px' }}>Web Intelligence</div>
                  <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', fontSize: '13px', lineHeight: '1.6', color: '#a7f3d0', margin: 0 }}>{msg.content}</pre>
                </div>
              )}
              {msg.role === 'user' && (
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <div style={{
                    maxWidth: '85%', padding: '12px 16px',
                    background: 'rgba(59,130,246,0.12)', border: '1px solid rgba(59,130,246,0.2)',
                    borderRadius: '16px 16px 4px 16px', fontSize: '14px', lineHeight: '1.5',
                    color: '#bfdbfe', wordBreak: 'break-word',
                  }}>
                    {msg.content}
                    <div style={{ fontSize: '10px', color: '#334155', marginTop: '6px', textAlign: 'right' }}>{formatTime(msg.timestamp)}</div>
                  </div>
                </div>
              )}
              {msg.role === 'assistant' && (
                <div style={{ display: 'flex', justifyContent: 'flex-start' }}>
                  <div style={{ maxWidth: '95%', width: '100%' }}>
                    {/* Badges */}
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginBottom: '6px' }}>
                      {msg.tier && <span style={{ padding: '3px 8px', fontSize: '10px', fontWeight: 700, background: 'rgba(59,130,246,0.1)', border: '1px solid rgba(59,130,246,0.15)', borderRadius: '4px', color: '#60a5fa', textTransform: 'uppercase' }}>{msg.tier}</span>}
                      {msg.mode && <span style={{ padding: '3px 8px', fontSize: '10px', fontWeight: 700, background: 'rgba(168,85,247,0.1)', border: '1px solid rgba(168,85,247,0.15)', borderRadius: '4px', color: '#a78bfa', textTransform: 'uppercase' }}>{msg.mode}</span>}
                      {msg.confidence !== undefined && <span style={{ padding: '3px 8px', fontSize: '10px', fontWeight: 700, background: 'rgba(34,197,94,0.1)', border: '1px solid rgba(34,197,94,0.15)', borderRadius: '4px', color: msg.confidence > 0.7 ? '#4ade80' : '#fbbf24' }}>{(msg.confidence * 100).toFixed(0)}%</span>}
                    </div>
                    {/* Response body */}
                    <div style={{
                      padding: '14px 16px', background: '#0c0c14',
                      border: '1px solid rgba(255,255,255,0.06)',
                      borderRadius: '4px 16px 16px 16px', fontSize: '14px', lineHeight: '1.6',
                      color: '#d1d5db', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                    }}>
                      {msg.content}
                      <div style={{ fontSize: '10px', color: '#334155', marginTop: '8px' }}>
                        {formatTime(msg.timestamp)}
                        {msg.intent && <span style={{ marginLeft: '8px', color: '#475569' }}>{msg.intent.split('{')[0]}</span>}
                      </div>
                    </div>
                    {/* Reasoning */}
                    {msg.reasoning && msg.reasoning.length > 0 && (
                      <div style={{ marginTop: '6px' }}>
                        <button onClick={() => setExpandedReasoning(expandedReasoning === msg.id ? null : msg.id)} style={{
                          display: 'flex', alignItems: 'center', gap: '6px',
                          padding: '6px 10px', fontSize: '11px', fontWeight: 700,
                          color: '#64748b', background: 'transparent',
                          border: '1px solid rgba(255,255,255,0.06)', borderRadius: '6px',
                          cursor: 'pointer', fontFamily: 'inherit', textTransform: 'uppercase',
                        }}>
                          Reasoning ({msg.reasoning.length}) {expandedReasoning === msg.id ? '\u25B2' : '\u25BC'}
                        </button>
                        {expandedReasoning === msg.id && (
                          <div style={{ marginTop: '6px', padding: '12px', background: 'rgba(0,0,0,0.3)', borderLeft: '3px solid rgba(59,130,246,0.2)', borderRadius: '0 8px 8px 0' }}>
                            {msg.reasoning.map((step, j) => (
                              <p key={j} style={{ fontSize: '12px', color: '#64748b', lineHeight: '1.6', margin: '4px 0' }}>
                                <span style={{ color: '#3b82f6', fontWeight: 700 }}>[{j}]</span> {step}
                              </p>
                            ))}
                          </div>
                        )}
                      </div>
                    )}
                    {/* Plan */}
                    {msg.plan && (
                      <div style={{ marginTop: '6px', padding: '10px 12px', background: 'rgba(59,130,246,0.05)', border: '1px solid rgba(59,130,246,0.1)', borderRadius: '8px', fontSize: '12px', color: '#64748b' }}>
                        <span style={{ fontWeight: 700, color: '#60a5fa' }}>PLAN: </span>
                        {msg.plan.steps} steps | complexity: {msg.plan.complexity.toFixed(2)} | {msg.plan.goal.slice(0, 100)}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          ))}

          {isThinking && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px', padding: '12px 16px', fontSize: '13px', color: '#60a5fa' }}>
              <div style={{ display: 'flex', gap: '4px' }}>
                {[0,1,2].map(i => <div key={i} style={{ width: '6px', height: '6px', background: '#3b82f6', borderRadius: '50%', animation: 'scc-bounce 1.4s infinite ease-in-out', animationDelay: `${i*0.16}s` }} />)}
              </div>
              Processing...
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>
      </main>

      {/* INPUT BAR */}
      <div style={{
        padding: '12px 16px', paddingBottom: 'max(12px, env(safe-area-inset-bottom))',
        background: '#0a0a10', borderTop: '1px solid rgba(255,255,255,0.06)', flexShrink: 0,
      }}>
        <div style={{
          maxWidth: '720px', margin: '0 auto', display: 'flex', alignItems: 'flex-end', gap: '8px',
          background: '#0c0c14', border: `1px solid ${input ? 'rgba(59,130,246,0.3)' : 'rgba(255,255,255,0.08)'}`,
          borderRadius: '12px', padding: '4px', transition: 'border-color 0.2s',
        }}>
          <textarea ref={inputRef} value={input} onChange={handleInputChange}
            onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }}}
            placeholder="Enter directive..."
            style={{
              flex: 1, background: 'transparent', border: 'none', outline: 'none',
              resize: 'none', fontSize: '15px', lineHeight: '1.5', padding: '10px 12px',
              color: '#e0e0e0', fontFamily: 'inherit', minHeight: '44px', maxHeight: '160px',
            }} rows={1}
          />
          <button onClick={handleSend} disabled={!input.trim() || !isConnected} style={{
            width: '44px', height: '44px', display: 'flex', alignItems: 'center', justifyContent: 'center',
            background: input.trim() && isConnected ? 'rgba(59,130,246,0.2)' : 'transparent',
            border: 'none', borderRadius: '10px',
            color: input.trim() && isConnected ? '#60a5fa' : '#1e293b',
            cursor: input.trim() && isConnected ? 'pointer' : 'default',
            flexShrink: 0,
          }}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="m22 2-7 20-4-9-9-4z"/><path d="M22 2 11 13"/>
            </svg>
          </button>
        </div>
        <div style={{ maxWidth: '720px', margin: '6px auto 0', display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: '#1e293b', padding: '0 8px' }}>
          <span>{isConnected ? 'Link active' : 'Reconnecting...'}</span>
          <span>Shift+Enter for newline</span>
        </div>
      </div>

      <style>{`
        @keyframes scc-bounce { 0%,80%,100%{transform:scale(0);opacity:.5} 40%{transform:scale(1);opacity:1} }
        * { box-sizing: border-box; }
        body { margin: 0; padding: 0; overflow: hidden; }
        input::placeholder, textarea::placeholder { color: #334155; }
        ::-webkit-scrollbar { width: 4px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.08); border-radius: 4px; }
      `}</style>
    </div>
  );
};

export default SovereignCommandConsole;
