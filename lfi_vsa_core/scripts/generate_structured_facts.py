#!/usr/bin/env python3
"""
Generate structured facts by querying the local Ollama model.
Per Training Strategy §6.2: use an LLM to generate structured triples
that LFI can learn in its native HDC format.

This runs locally — no external API calls. Uses the same qwen2.5-coder:7b
that the training pipeline uses.
"""
import json
import sqlite3
import urllib.request
import os
import sys
import time

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen2.5-coder:7b"

DOMAINS = [
    ("computer_science", "Generate 20 atomic facts about computer science. Format each as: Subject | Predicate | Object\nExample: TCP | is_a | transport_protocol\nExample: quicksort | has_complexity | O(n log n) average"),
    ("rust_programming", "Generate 20 atomic facts about the Rust programming language. Format each as: Subject | Predicate | Object\nExample: Rust | uses | ownership_system\nExample: Vec | is_a | growable_array"),
    ("cryptography", "Generate 20 atomic facts about cryptography and security. Format each as: Subject | Predicate | Object\nExample: AES-256 | is_a | symmetric_cipher\nExample: RSA | uses | prime_factorization"),
    ("networking", "Generate 20 atomic facts about computer networking. Format each as: Subject | Predicate | Object\nExample: DNS | resolves | domain_names\nExample: HTTP/3 | uses | QUIC_protocol"),
    ("mathematics", "Generate 20 atomic facts about mathematics. Format each as: Subject | Predicate | Object\nExample: pi | approximately_equals | 3.14159\nExample: Euler_identity | relates | e_pi_i_1_0"),
    ("physics", "Generate 20 atomic facts about physics. Format each as: Subject | Predicate | Object\nExample: speed_of_light | equals | 299792458_m/s\nExample: entropy | always_increases_in | isolated_systems"),
    ("biology", "Generate 20 atomic facts about biology. Format each as: Subject | Predicate | Object\nExample: DNA | encodes | genetic_information\nExample: mitochondria | produces | ATP"),
    ("philosophy", "Generate 20 atomic facts about philosophy. Format each as: Subject | Predicate | Object\nExample: Descartes | proposed | cogito_ergo_sum\nExample: utilitarianism | maximizes | overall_happiness"),
    ("history", "Generate 20 atomic facts about world history. Format each as: Subject | Predicate | Object\nExample: World_War_2 | ended_in | 1945\nExample: printing_press | invented_by | Gutenberg"),
    ("psychology", "Generate 20 atomic facts about psychology and cognitive science. Format each as: Subject | Predicate | Object\nExample: working_memory | has_capacity | 7_plus_minus_2\nExample: Maslow | proposed | hierarchy_of_needs"),
]

def query_ollama(prompt, model=MODEL):
    data = json.dumps({"model": model, "prompt": prompt, "stream": False}).encode()
    req = urllib.request.Request(OLLAMA_URL, data=data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            result = json.loads(resp.read())
            return result.get("response", "")
    except Exception as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return ""

def parse_triples(text):
    """Parse 'Subject | Predicate | Object' lines from LLM output."""
    facts = []
    for line in text.split('\n'):
        line = line.strip().lstrip('0123456789.-) ')
        parts = [p.strip() for p in line.split('|')]
        if len(parts) >= 3 and all(p for p in parts[:3]):
            subject, predicate, obj = parts[0], parts[1], parts[2]
            key = f"kg_{subject.lower().replace(' ','_')}_{predicate.lower().replace(' ','_')}_{obj.lower().replace(' ','_')}"
            value = f"{subject} {predicate} {obj}"
            facts.append((key[:200], value[:500], 0.8))
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
        response = query_ollama(prompt)
        if not response:
            print("  (no response)")
            continue
        facts = parse_triples(response)
        inserted = 0
        for key, value, conf in facts:
            try:
                conn.execute(
                    """INSERT INTO facts (key, value, source, confidence, updated_at)
                       VALUES (?, ?, 'llm_generated', ?, datetime('now'))
                       ON CONFLICT(key) DO UPDATE SET confidence=MAX(confidence, ?), updated_at=datetime('now')""",
                    (key, value, conf, conf))
                inserted += 1
            except Exception:
                pass
        conn.commit()
        total += inserted
        print(f"  {inserted} facts from {len(facts)} parsed triples")
        time.sleep(1)

    conn.close()
    print(f"\n=== DONE: {total} total facts generated ===")

if __name__ == "__main__":
    main()
