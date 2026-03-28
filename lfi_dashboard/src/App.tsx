import React, { useState, useRef, useEffect } from 'react';
import { 
  Send, Paperclip, Mic, Image as ImageIcon, FileText, 
  Search, ShieldAlert, Zap, Settings, Menu, X, 
  Globe, CheckCircle2, AlertCircle, Cpu
} from 'lucide-react';

const ModernDashboard = () => {
  const [input, setInput] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [isCreative, setIsCreative] = useState(false);
  const [sensoryStatus, setSensoryStatus] = useState('Idle');
  const [messages, setMessages] = useState([
    { role: 'assistant', content: "Sovereign LFI v5.6.8 Online. Multimodal transducers active. Web Search skepticism protocol engaged. How shall we proceed?", trust: 'Sovereign' }
  ]);
  const [isSidebarOpen, setSidebarOpen] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Multimodal State
  const [attachments, setAttachments] = useState<File[]>([]);

  const handleSend = () => {
    if (!input.trim() && attachments.length === 0) return;
    
    const newMsg = { role: 'user', content: input, trust: 'Pending' };
    setMessages(prev => [...prev, newMsg]);
    setInput('');
    setAttachments([]);

    // Simulate Agent Thinking & PSL Auditing
    setTimeout(() => {
      let content = "Forensic analysis complete. All constructs encoded into 10,000-bit VSA space. No anomalies detected.";
      let auditLog = "PSL: Local compute verified.";
      let trust = 'Sovereign';

      if (isSearching) {
        content = "I've analyzed the web results for your query. Note: Source 'WikiCompute' failed the Statistical Equilibrium audit. I have discarded that data and synthesized this verified response.";
        auditLog = "PSL: Discarded 3 hostile payloads from 'untrusted_dns'.";
        trust = 'Verified';
      } else if (isCreative) {
        content = "Creative Synthesis Active. I have mapped the structural similarities between your problem and the biological 'Vascular Cooling' domain. Proposed solution: Fractal-based micro-channel distribution.";
        auditLog = "VSA: Analogy mapped via binding operator (\u2297). Sim=0.8421";
        trust = 'Sovereign';
      }

      const response = { role: 'assistant', content, trust, auditLog };
      setMessages(prev => [...prev, response]);
      
      if (isCreative) {
        setSensoryStatus('Ingesting IMU...');
        setTimeout(() => setSensoryStatus('Idle'), 2000);
      }
    }, 1500);
  };

  return (
    <div className="flex h-screen bg-[#0a0a0a] text-gray-100 selection:bg-blue-500/30 font-sans overflow-hidden">
      
      {/* Sidebar */}
      <div className={`${isSidebarOpen ? 'w-64' : 'w-0'} transition-all duration-300 border-r border-white/5 bg-[#0d0d0d] flex flex-col`}>
        <div className="p-4 border-b border-white/5 flex items-center justify-between">
          <div className="flex items-center gap-2 font-bold text-sm tracking-widest text-blue-400">
            <Cpu size={18} /> LFI PROJECT
          </div>
          <button onClick={() => setSidebarOpen(false)}><X size={18} className="text-gray-500 hover:text-white" /></button>
        </div>
        
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          <div className="px-3 py-2 text-[10px] uppercase text-gray-500 font-bold tracking-widest">Recent Sessions</div>
          {['Polyglot Core Audit', 'HDC Topology Map', 'Android Kernel Probe'].map((item, i) => (
            <div key={i} className="px-3 py-2 rounded-lg hover:bg-white/5 text-sm text-gray-400 cursor-pointer flex items-center gap-2 group">
              <FileText size={14} className="group-hover:text-blue-400" /> {item}
            </div>
          ))}
        </div>

        <div className="p-4 border-t border-white/5 space-y-4">
          <div className="flex items-center justify-between text-[10px] text-gray-500 uppercase font-bold tracking-widest">
            <span>Skepticism Level</span>
            <span className="text-orange-500">MAXIMUM</span>
          </div>
          <div className="h-1 bg-white/5 rounded-full overflow-hidden">
            <div className="h-full w-full bg-orange-500" />
          </div>
        </div>
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col relative bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-blue-900/5 via-transparent to-transparent">
        {!isSidebarOpen && (
          <button onClick={() => setSidebarOpen(true)} className="absolute top-4 left-4 z-10 p-2 hover:bg-white/5 rounded-lg">
            <Menu size={20} />
          </button>
        )}

        {/* Chat Messages */}
        <div className="flex-1 overflow-y-auto p-4 md:p-8 space-y-8" ref={scrollRef}>
          <div className="max-w-3xl mx-auto space-y-8">
            {messages.map((msg, i) => (
              <div key={i} className={`flex gap-4 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                <div className={`max-w-[85%] rounded-2xl p-4 ${
                  msg.role === 'user' 
                  ? 'bg-blue-600/10 border border-blue-500/20 text-gray-200' 
                  : 'bg-white/5 border border-white/10 text-gray-300'
                }`}>
                  <div className="flex items-center gap-2 mb-2">
                    {msg.role === 'assistant' && <div className="p-1 bg-blue-500 rounded-md"><Cpu size={12} className="text-white" /></div>}
                    <span className="text-[10px] font-bold uppercase tracking-widest text-gray-500">
                      {msg.role === 'user' ? 'Direct Directive' : 'LFI Alpha'}
                    </span>
                    {msg.trust === 'Sovereign' && <ShieldAlert size={12} className="text-blue-400" />}
                    {msg.trust === 'Verified' && <CheckCircle2 size={12} className="text-green-500" />}
                  </div>
                  <div className="text-sm leading-relaxed whitespace-pre-wrap">{msg.content}</div>
                  {msg.auditLog && (
                    <div className="mt-4 pt-4 border-t border-white/5 text-[10px] font-mono text-orange-500/80 italic">
                      {msg.auditLog}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Multimodal Input Area */}
        <div className="p-4 md:p-8 border-t border-white/5 bg-[#0d0d0d]/80 backdrop-blur-xl">
          <div className="max-w-3xl mx-auto relative group">
            
            {/* Action Bar */}
            <div className="absolute -top-12 left-0 flex items-center gap-2">
              <button 
                onClick={() => setIsSearching(!isSearching)}
                className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-[10px] font-bold uppercase transition-all ${
                  isSearching ? 'bg-orange-500 text-white shadow-lg shadow-orange-500/20' : 'bg-white/5 text-gray-500 hover:text-white'
                }`}
              >
                <Globe size={14} /> Skeptical Search
              </button>
              <button 
                onClick={() => setIsCreative(!isCreative)}
                className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-[10px] font-bold uppercase transition-all ${
                  isCreative ? 'bg-purple-500 text-white shadow-lg shadow-purple-500/20' : 'bg-white/5 text-gray-500 hover:text-white'
                }`}
              >
                <Zap size={14} /> Creative Synthesis
              </button>
            </div>

            <div className="relative flex items-end gap-2 bg-[#1a1a1a] border border-white/10 focus-within:border-blue-500/50 rounded-2xl p-2 transition-all shadow-2xl">
              <div className="flex flex-col flex-1 min-h-[44px]">
                <textarea 
                  rows={1}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && handleSend()}
                  placeholder="Ask the Sovereign Core..."
                  className="bg-transparent border-none outline-none resize-none px-3 py-2 text-sm placeholder:text-gray-600 w-full"
                />
              </div>

              <div className="flex items-center gap-1 p-1">
                <button className="p-2 text-gray-500 hover:text-blue-400 transition-colors"><Paperclip size={20} /></button>
                <button className="p-2 text-gray-500 hover:text-blue-400 transition-colors"><Mic size={20} /></button>
                <button 
                  onClick={handleSend}
                  disabled={!input.trim()}
                  className={`p-2 rounded-xl transition-all ${input.trim() ? 'bg-blue-500 text-white' : 'text-gray-700'}`}
                >
                  <Send size={20} />
                </button>
              </div>
            </div>
            <div className="mt-3 flex justify-center gap-6 text-[10px] font-bold text-gray-700 tracking-widest uppercase">
              <span className="flex items-center gap-1"><Zap size={10} /> Neural Processing</span>
              <span className="flex items-center gap-1"><ShieldAlert size={10} /> Zero-Trust Audit Active</span>
              <span className={`flex items-center gap-1 transition-colors ${sensoryStatus !== 'Idle' ? 'text-green-500' : ''}`}>
                <Cpu size={10} /> Sensory: {sensoryStatus}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ModernDashboard;
