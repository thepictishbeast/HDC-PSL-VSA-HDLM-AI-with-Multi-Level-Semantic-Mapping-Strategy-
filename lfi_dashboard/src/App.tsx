// ============================================================
// Sovereign Command Console (SCC) v2.0 — Mobile-First Dashboard
//
// PROTOCOL: Real-time WebSocket integration with LFI Cognitive Core
// SUBSTRATE: React + Tailwind CSS, responsive single-col (mobile)
//            to dual-pane (desktop) layout
//
// ENDPOINTS:
//   ws://<host>:3000/ws/chat       — Bidirectional cognitive chat
//   ws://<host>:3000/ws/telemetry  — Real-time substrate telemetry
//   POST /api/auth                 — Sovereign key verification
//   GET  /api/status               — Substrate status
// ============================================================

import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  ShieldAlert, Cpu, Activity, Database,
  Terminal, Send, X, Zap, Brain, Search,
  BookOpen, Settings, ChevronDown, ChevronUp
} from 'lucide-react';

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
  console.debug("// SUBSTRATE: SCC v2.0 component mount.");

  // ---- State ----
  const [isAuthenticated, setIsAuthenticated] = useState(() => {
    return localStorage.getItem('lfi_auth') === 'true';
  });
  const [password, setPassword] = useState('');
  const [authError, setAuthError] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const [showReasoning, setShowReasoning] = useState<number | null>(null);
  const [stats, setStats] = useState<SubstrateStats>({
    ram_available_mb: 0, cpu_temp_c: 0, vsa_orthogonality: 0.02,
    axiom_pass_rate: 1.0, is_throttled: false, logic_density: 0
  });

  const chatWsRef = useRef<WebSocket | null>(null);
  const telemetryWsRef = useRef<WebSocket | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // ---- Helpers ----
  const getHost = () => window.location.hostname || '127.0.0.1';

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => { scrollToBottom(); }, [messages, scrollToBottom]);

  // Persist auth
  useEffect(() => {
    localStorage.setItem('lfi_auth', isAuthenticated.toString());
  }, [isAuthenticated]);

  // ---- WebSocket: Chat ----
  useEffect(() => {
    if (!isAuthenticated) return;

    const wsUrl = `ws://${getHost()}:3000/ws/chat`;
    console.info(`// NETWORK: Connecting chat WebSocket to ${wsUrl}`);

    const connect = () => {
      const ws = new WebSocket(wsUrl);
      chatWsRef.current = ws;

      ws.onopen = () => {
        console.warn("// NETWORK: Chat WebSocket synchronized.");
        setIsConnected(true);
        setMessages(prev => [...prev, {
          id: Date.now(), role: 'system', content: 'Sovereign Cognitive Link established.',
          timestamp: Date.now()
        }]);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          console.debug("// AUDIT: Chat response received:", msg.type);

          if (msg.type === 'chat_response') {
            setIsThinking(false);
            setMessages(prev => [...prev, {
              id: Date.now(),
              role: 'assistant',
              content: msg.content || '',
              mode: msg.mode,
              confidence: msg.confidence,
              tier: msg.tier,
              intent: msg.intent,
              reasoning: msg.reasoning,
              plan: msg.plan,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'web_result') {
            setMessages(prev => [...prev, {
              id: Date.now(),
              role: 'web',
              content: `[WEB: ${msg.source_count} sources, trust ${(msg.trust * 100).toFixed(0)}%]\n${msg.summary}`,
              timestamp: Date.now(),
            }]);
          } else if (msg.type === 'chat_error') {
            setIsThinking(false);
            setMessages(prev => [...prev, {
              id: Date.now(), role: 'system',
              content: `Cognitive Fault: ${msg.error}`,
              timestamp: Date.now(),
            }]);
          }
        } catch (e) {
          console.error("// TELEMETRY: Chat parse fault.", e);
        }
      };

      ws.onclose = () => {
        console.warn("// NETWORK: Chat WebSocket disconnected. Reconnecting...");
        setIsConnected(false);
        setTimeout(connect, 3000);
      };

      ws.onerror = () => {
        setIsConnected(false);
      };
    };

    connect();
    return () => { chatWsRef.current?.close(); };
  }, [isAuthenticated]);

  // ---- WebSocket: Telemetry ----
  useEffect(() => {
    if (!isAuthenticated) return;

    const wsUrl = `ws://${getHost()}:3000/ws/telemetry`;
    console.info(`// NETWORK: Connecting telemetry WebSocket to ${wsUrl}`);

    const connect = () => {
      const ws = new WebSocket(wsUrl);
      telemetryWsRef.current = ws;

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          if (msg.type === 'telemetry' && msg.data) {
            setStats(prev => ({ ...prev, ...msg.data }));
          }
        } catch (e) {
          console.error("// TELEMETRY: Parse fault.", e);
        }
      };

      ws.onclose = () => setTimeout(connect, 5000);
    };

    connect();
    return () => { telemetryWsRef.current?.close(); };
  }, [isAuthenticated]);

  // ---- Authentication ----
  const handleLogin = async () => {
    setAuthError('');
    try {
      const res = await fetch(`http://${getHost()}:3000/api/auth`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: password }),
      });
      const data = await res.json();
      if (data.status === 'authenticated') {
        setIsAuthenticated(true);
      } else {
        setAuthError('Sovereign key rejected.');
      }
    } catch {
      setAuthError('Backend unreachable. Verify SCC server is running on port 3000.');
    }
  };

  const handleLogout = () => {
    localStorage.removeItem('lfi_auth');
    chatWsRef.current?.close();
    telemetryWsRef.current?.close();
    setIsAuthenticated(false);
    setMessages([]);
  };

  // ---- Chat Send ----
  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || !chatWsRef.current || chatWsRef.current.readyState !== WebSocket.OPEN) return;

    // Add user message to UI
    setMessages(prev => [...prev, {
      id: Date.now(), role: 'user', content: trimmed, timestamp: Date.now()
    }]);

    // Send to backend
    chatWsRef.current.send(JSON.stringify({ content: trimmed }));
    setIsThinking(true);
    setInput('');
    inputRef.current?.focus();
  };

  // ============================================================
  // RENDER: Login Screen
  // ============================================================
  if (!isAuthenticated) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-[#030303] text-blue-500 font-mono p-4">
        <div className="w-full max-w-sm p-8 border border-blue-900/20 bg-[#0a0a0f] rounded-lg">
          <div className="flex justify-center mb-6">
            <ShieldAlert size={36} className="text-blue-500 animate-pulse" />
          </div>
          <h1 className="text-[10px] font-bold text-center mb-8 tracking-[0.4em] uppercase text-blue-400">
            Sovereign Identification
          </h1>
          <input
            type="password"
            autoFocus
            className="w-full p-4 bg-black/60 border border-blue-900/30 rounded-md outline-none text-center text-blue-300 placeholder:text-blue-900/50 focus:border-blue-500/50 transition-all mb-4 text-sm"
            placeholder="AUTH_KEY"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleLogin()}
          />
          {authError && (
            <p className="text-red-500 text-[9px] text-center mb-4 uppercase">{authError}</p>
          )}
          <button
            onClick={handleLogin}
            className="w-full py-3 bg-blue-900/20 border border-blue-500/30 hover:bg-blue-500/20 transition-all text-[10px] font-bold uppercase tracking-widest rounded-md active:scale-95"
          >
            Initiate Link
          </button>
        </div>
      </div>
    );
  }

  // ============================================================
  // RENDER: Main Console
  // ============================================================
  return (
    <div className="flex flex-col h-screen w-full bg-[#050505] text-[#e0e0e0] font-mono overflow-hidden">

      {/* ---- HEADER: Telemetry Bar ---- */}
      <header className="h-11 bg-black/90 border-b border-white/5 flex items-center justify-between px-3 z-50 shrink-0">
        <div className="flex items-center gap-3">
          <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.6)]' : 'bg-red-500 animate-pulse'}`} />
          <span className="text-[9px] font-black uppercase tracking-wider hidden sm:inline">
            {isConnected ? 'SOVEREIGN_CORE' : 'DISCONNECTED'}
          </span>
        </div>

        <div className="flex items-center gap-4 text-[8px] text-gray-500 uppercase font-bold">
          <span className="flex items-center gap-1">
            <Cpu size={10} /> {stats.ram_available_mb}MB
          </span>
          <span className="flex items-center gap-1 hidden sm:flex">
            <Database size={10} /> VSA:{(100 - stats.vsa_orthogonality * 100).toFixed(1)}%
          </span>
          <span className={`flex items-center gap-1 ${stats.cpu_temp_c > 65 ? 'text-red-500' : stats.cpu_temp_c > 50 ? 'text-yellow-500' : 'text-blue-400'}`}>
            <Activity size={10} /> {stats.cpu_temp_c.toFixed(0)}C
          </span>
          <span className="flex items-center gap-1 hidden sm:flex">
            <Zap size={10} /> PSL:{(stats.axiom_pass_rate * 100).toFixed(0)}%
          </span>
        </div>

        <button
          onClick={handleLogout}
          className="text-[8px] text-gray-600 hover:text-red-500 transition-colors uppercase font-bold"
        >
          Logout
        </button>
      </header>

      {/* ---- MAIN: Chat Area ---- */}
      <main className="flex-1 overflow-y-auto px-3 py-4 space-y-4">
        <div className="max-w-3xl mx-auto space-y-4">
          {messages.map((msg) => (
            <div key={msg.id} className={`flex flex-col ${msg.role === 'user' ? 'items-end' : 'items-start'}`}>

              {/* System messages */}
              {msg.role === 'system' && (
                <div className="text-[9px] text-gray-600 italic px-2 py-1 w-full text-center">
                  {msg.content}
                </div>
              )}

              {/* Web search results */}
              {msg.role === 'web' && (
                <div className="w-full p-3 bg-emerald-900/10 border border-emerald-500/20 rounded-md text-[10px] text-emerald-300/80">
                  <div className="flex items-center gap-1 mb-1 text-[8px] uppercase font-bold text-emerald-500">
                    <Search size={10} /> Web Intelligence
                  </div>
                  <pre className="whitespace-pre-wrap">{msg.content}</pre>
                </div>
              )}

              {/* User messages */}
              {msg.role === 'user' && (
                <div className="max-w-[85%] p-3 bg-blue-600/10 border border-blue-500/20 rounded-lg text-[11px] leading-relaxed">
                  {msg.content}
                </div>
              )}

              {/* Assistant messages */}
              {msg.role === 'assistant' && (
                <div className="w-full max-w-[95%] space-y-2">
                  {/* Metadata badge */}
                  <div className="flex items-center gap-2 text-[7px] uppercase font-bold text-gray-600">
                    {msg.tier && <span className="px-1.5 py-0.5 border border-white/10 rounded">{msg.tier}</span>}
                    {msg.mode && <span className="flex items-center gap-0.5"><Brain size={8} /> {msg.mode}</span>}
                    {msg.confidence !== undefined && <span>conf:{(msg.confidence * 100).toFixed(0)}%</span>}
                    {msg.intent && <span className="text-blue-500 truncate max-w-32">{msg.intent.split('{')[0]}</span>}
                  </div>

                  {/* Main response */}
                  <div className="p-3 bg-[#0a0a0f] border border-white/5 rounded-lg text-[11px] leading-relaxed whitespace-pre-wrap">
                    {msg.content}
                  </div>

                  {/* Reasoning scratchpad (collapsible) */}
                  {msg.reasoning && msg.reasoning.length > 0 && (
                    <div>
                      <button
                        onClick={() => setShowReasoning(showReasoning === msg.id ? null : msg.id)}
                        className="flex items-center gap-1 text-[8px] text-gray-600 hover:text-gray-400 uppercase font-bold transition-colors"
                      >
                        <BookOpen size={10} />
                        Reasoning Scratchpad ({msg.reasoning.length} steps)
                        {showReasoning === msg.id ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                      </button>
                      {showReasoning === msg.id && (
                        <div className="mt-1 p-2 bg-black/40 border-l-2 border-blue-900/30 text-[9px] text-gray-500 space-y-0.5">
                          {msg.reasoning.map((step, j) => (
                            <p key={j} className="font-mono">[{j}] {step}</p>
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* Plan display */}
                  {msg.plan && (
                    <div className="p-2 bg-black/30 border border-white/5 rounded text-[8px] text-gray-500">
                      <span className="font-bold text-blue-500">PLAN:</span> {msg.plan.steps} steps | complexity: {msg.plan.complexity.toFixed(2)} | {msg.plan.goal.slice(0, 80)}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}

          {/* Thinking indicator */}
          {isThinking && (
            <div className="flex items-center gap-2 text-[9px] text-blue-500">
              <div className="flex gap-1">
                <div className="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                <div className="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                <div className="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
              Cognitive processing...
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </main>

      {/* ---- INPUT BAR ---- */}
      <div className="p-3 bg-black/80 border-t border-white/5 shrink-0">
        <div className="max-w-3xl mx-auto relative bg-[#0a0a0f] border border-white/10 rounded-lg flex items-end focus-within:border-blue-500/40 transition-all">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Enter directive..."
            className="w-full bg-transparent border-none focus:ring-0 outline-none text-[11px] py-3 px-4 resize-none max-h-32 min-h-[44px] placeholder:text-gray-700"
            rows={1}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || !isConnected}
            className="p-3 text-blue-500 disabled:text-gray-800 hover:text-blue-400 transition-colors shrink-0"
          >
            <Send size={16} />
          </button>
        </div>
        <div className="max-w-3xl mx-auto flex justify-between mt-1 px-1">
          <span className="text-[7px] text-gray-700 uppercase">
            {isConnected ? 'link:active' : 'link:disconnected'}
          </span>
          <span className="text-[7px] text-gray-700 uppercase">
            shift+enter for newline
          </span>
        </div>
      </div>
    </div>
  );
};

export default SovereignCommandConsole;
