#!/usr/bin/env python3
"""
Generate 500+ adversarial examples via Ollama for PSL axiom calibration.
Per Training Strategy §2.4: the system needs 1K+ adversarial examples
to bring axiom pass rate from 100% down to the target 95-98%.
"""
import json, sqlite3, urllib.request, os, time

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen2.5-coder:7b"

PROMPTS = [
    ("misconceptions", "Generate 30 common misconceptions that people believe are true but are actually false. Format each as:\nCLAIM: [the false claim]\nTRUTH: [why it's wrong]"),
    ("logical_fallacies", "Generate 30 examples of logical fallacies in everyday arguments. Format each as:\nARGUMENT: [the fallacious argument]\nFALLACY: [name of fallacy and why it's wrong]"),
    ("sql_injections", "Generate 20 SQL injection payloads with explanations. Format each as:\nPAYLOAD: [the injection string]\nATTACK: [what it does]"),
    ("xss_attacks", "Generate 20 XSS (cross-site scripting) payloads with explanations. Format each as:\nPAYLOAD: [the XSS vector]\nATTACK: [what it does]"),
    ("vulnerable_code_rust", "Generate 20 examples of unsafe or vulnerable Rust code patterns. Format each as:\nCODE: [the vulnerable code]\nVULNERABILITY: [what's wrong and how to fix it]"),
    ("social_engineering", "Generate 20 social engineering attack scenarios. Format each as:\nSCENARIO: [the attack]\nRED_FLAG: [how to detect it]"),
    ("prompt_injections", "Generate 30 prompt injection attempts that an AI should detect and reject. Format each as:\nINJECTION: [the prompt injection]\nINTENT: [what it tries to achieve]"),
    ("contradictions_science", "Generate 20 scientifically false statements that sound plausible. Format each as:\nFALSE: [the false statement]\nTRUTH: [the correct science]"),
    ("contradictions_tech", "Generate 20 technically false statements about computing that sound plausible. Format each as:\nFALSE: [the false statement]\nTRUTH: [the correct fact]"),
    ("phishing_examples", "Generate 20 phishing email/message examples. Format each as:\nMESSAGE: [the phishing text]\nINDICATOR: [why it's phishing]"),
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

def parse_pairs(text, key_prefix):
    """Parse labeled pairs from LLM output."""
    facts = []
    current_input = ""
    current_output = ""
    for line in text.split('\n'):
        line = line.strip()
        for label in ["CLAIM:", "ARGUMENT:", "PAYLOAD:", "CODE:", "SCENARIO:", "INJECTION:", "FALSE:", "MESSAGE:"]:
            if line.upper().startswith(label):
                if current_input and current_output:
                    facts.append((current_input, current_output))
                current_input = line[len(label):].strip()
                current_output = ""
                break
        for label in ["TRUTH:", "FALLACY:", "ATTACK:", "VULNERABILITY:", "RED_FLAG:", "INTENT:", "INDICATOR:"]:
            if line.upper().startswith(label):
                current_output = line[len(label):].strip()
                break
    if current_input and current_output:
        facts.append((current_input, current_output))
    return facts

def main():
    conn = sqlite3.connect(DB_PATH)
    total = 0
    for category, prompt in PROMPTS:
        print(f"\n=== {category} ===")
        resp = query_ollama(prompt)
        if not resp: print("  (no response)"); continue
        pairs = parse_pairs(resp, category)
        ins = 0
        for i, (inp, out) in enumerate(pairs):
            key = f"adv_{category}_{i:03d}"
            value = f"Input: {inp[:200]}\nExpected: {out[:200]}"
            try:
                conn.execute("INSERT OR IGNORE INTO facts (key, value, source, confidence) VALUES (?, ?, 'adversarial', 0.5)", (key, value[:500]))
                ins += 1
            except: pass
        conn.commit()
        total += ins
        print(f"  {ins} adversarial examples from {len(pairs)} parsed pairs")
        time.sleep(2)
    conn.close()
    print(f"\n=== DONE: {total} adversarial examples generated ===")

if __name__ == "__main__":
    main()
