#!/usr/bin/env python3
"""Extended fact generation — more domains, deeper coverage."""
import json, sqlite3, urllib.request, os, time

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen2.5-coder:7b"

DOMAINS = [
    ("operating_systems", "Generate 30 atomic facts about operating systems (Linux, seL4, microkernels, scheduling, memory management). Format: Subject | Predicate | Object"),
    ("systems_engineering", "Generate 30 atomic facts about distributed systems, databases, and systems design. Format: Subject | Predicate | Object"),
    ("active_inference", "Generate 20 atomic facts about the Free Energy Principle, Active Inference, and Karl Friston's work. Format: Subject | Predicate | Object"),
    ("hdc_computing", "Generate 20 atomic facts about Hyperdimensional Computing (HDC), Vector Symbolic Architectures, and holographic representations. Format: Subject | Predicate | Object"),
    ("privacy_security", "Generate 30 atomic facts about privacy engineering, anonymity networks (Tor, I2P), plausible deniability, and zero-knowledge proofs. Format: Subject | Predicate | Object"),
    ("web_technologies", "Generate 20 atomic facts about HTTP, REST APIs, WebSocket, WebAssembly, and modern web architecture. Format: Subject | Predicate | Object"),
    ("logic_reasoning", "Generate 20 atomic facts about formal logic, propositional logic, predicate logic, and logical fallacies. Format: Subject | Predicate | Object"),
    ("neuroscience", "Generate 20 atomic facts about neuroscience, neural networks, brain regions, and cognitive processes. Format: Subject | Predicate | Object"),
    ("economics_game_theory", "Generate 20 atomic facts about economics, game theory, mechanism design, and market dynamics. Format: Subject | Predicate | Object"),
    ("climate_energy", "Generate 20 atomic facts about climate science, renewable energy, carbon cycles, and environmental science. Format: Subject | Predicate | Object"),
    ("space_astronomy", "Generate 20 atomic facts about space, astronomy, cosmology, and astrophysics. Format: Subject | Predicate | Object"),
    ("legal_rights", "Generate 20 atomic facts about digital rights, privacy law (GDPR, CCPA), intellectual property, and cybercrime law. Format: Subject | Predicate | Object"),
]

def query_ollama(prompt):
    data = json.dumps({"model": MODEL, "prompt": prompt, "stream": False}).encode()
    req = urllib.request.Request(OLLAMA_URL, data=data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=180) as resp:
            return json.loads(resp.read()).get("response", "")
    except Exception as e:
        print(f"  ERROR: {e}")
        return ""

def parse_triples(text):
    facts = []
    for line in text.split('\n'):
        line = line.strip().lstrip('0123456789.-) ')
        parts = [p.strip() for p in line.split('|')]
        if len(parts) >= 3 and all(p for p in parts[:3]):
            s, p, o = parts[0], parts[1], parts[2]
            key = f"kg_{s.lower().replace(' ','_')}_{p.lower().replace(' ','_')}_{o.lower().replace(' ','_')}"
            facts.append((key[:200], f"{s} {p} {o}"[:500], 0.8))
    return facts

def main():
    conn = sqlite3.connect(DB_PATH)
    conn.execute("""CREATE TABLE IF NOT EXISTS facts (
        key TEXT PRIMARY KEY, value TEXT NOT NULL,
        source TEXT DEFAULT 'llm_generated', confidence REAL DEFAULT 1.0,
        created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now'))
    )""")
    total = 0
    for domain, prompt in DOMAINS:
        print(f"\n=== {domain} ===")
        resp = query_ollama(prompt)
        if not resp: print("  (no response)"); continue
        facts = parse_triples(resp)
        ins = 0
        for k, v, c in facts:
            try:
                conn.execute("INSERT INTO facts (key,value,source,confidence,updated_at) VALUES (?,?,'llm_generated',?,datetime('now')) ON CONFLICT(key) DO UPDATE SET confidence=MAX(confidence,?),updated_at=datetime('now')", (k,v,c,c))
                ins += 1
            except: pass
        conn.commit()
        total += ins
        print(f"  {ins} facts from {len(facts)} parsed")
        time.sleep(1)
    conn.close()
    print(f"\n=== DONE: {total} new facts ===")

if __name__ == "__main__":
    main()
