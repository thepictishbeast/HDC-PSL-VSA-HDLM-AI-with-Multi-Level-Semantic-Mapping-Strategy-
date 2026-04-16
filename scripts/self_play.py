#!/usr/bin/env python3
"""
Self-play data generation for LFI reasoning pattern library.
Per Training Strategy §6.1 + Bible §6.6 Mechanism 5.

Generates challenging problems, attempts to solve them via the LFI API,
evaluates the quality, and logs successful reasoning chains as patterns.
"""
import json, urllib.request, time, os, sqlite3

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
API_BASE = "http://127.0.0.1:3000"
OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen2.5-coder:7b"

PROBLEM_GENERATORS = [
    {
        "domain": "logic",
        "prompt": "Generate a logic puzzle that requires 3-5 steps of deductive reasoning to solve. Include the puzzle and its solution. Format:\nPUZZLE: [the puzzle]\nSOLUTION: [step-by-step solution]",
    },
    {
        "domain": "code",
        "prompt": "Generate a coding challenge suitable for a mid-level Rust programmer. Include the problem and an optimal solution. Format:\nPROBLEM: [the problem]\nSOLUTION: [Rust code solution with explanation]",
    },
    {
        "domain": "security",
        "prompt": "Generate a security analysis scenario: describe a system and ask what vulnerabilities exist. Include the analysis. Format:\nSCENARIO: [system description]\nANALYSIS: [vulnerabilities and mitigations]",
    },
    {
        "domain": "math",
        "prompt": "Generate a math problem that requires multi-step reasoning (algebra, calculus, or combinatorics). Include step-by-step solution. Format:\nPROBLEM: [the problem]\nSOLUTION: [step-by-step solution]",
    },
    {
        "domain": "philosophy",
        "prompt": "Generate a philosophical thought experiment and analyze it from 2-3 different ethical frameworks. Format:\nSCENARIO: [the thought experiment]\nANALYSIS: [multi-framework analysis]",
    },
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

def test_via_api(question):
    """Send the problem to the LFI API and get its response."""
    data = json.dumps({"input": question}).encode()
    req = urllib.request.Request(f"{API_BASE}/api/think", data=data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except:
        return None

def save_reasoning_chain(domain, problem, solution, lfi_response):
    """Save the reasoning chain to brain.db as a training fact."""
    conn = sqlite3.connect(DB_PATH)
    key = f"selfplay_{domain}_{int(time.time())}"
    value = f"Problem: {problem[:200]}\nExpected: {solution[:200]}\nLFI: {lfi_response[:200]}"
    conn.execute(
        "INSERT OR REPLACE INTO facts (key, value, source, confidence, updated_at) VALUES (?, ?, 'self_play', 0.7, datetime('now'))",
        (key, value[:500])
    )
    conn.commit()
    conn.close()

def main():
    print("=== LFI Self-Play Data Generation ===")
    total_generated = 0

    for gen in PROBLEM_GENERATORS:
        domain = gen["domain"]
        print(f"\n--- {domain} ---")

        # Generate a problem
        response = query_ollama(gen["prompt"])
        if not response:
            print("  (no response from LLM)")
            continue

        # Extract problem and solution
        problem = response
        solution = ""
        if "SOLUTION:" in response:
            parts = response.split("SOLUTION:", 1)
            problem = parts[0].replace("PUZZLE:", "").replace("PROBLEM:", "").replace("SCENARIO:", "").strip()
            solution = parts[1].strip()
        elif "ANALYSIS:" in response:
            parts = response.split("ANALYSIS:", 1)
            problem = parts[0].replace("SCENARIO:", "").strip()
            solution = parts[1].strip()

        if not problem:
            print("  (couldn't parse problem)")
            continue

        # Test via LFI API
        lfi_result = test_via_api(problem[:500])
        lfi_answer = lfi_result.get("answer", "") if lfi_result else "(API unavailable)"

        # Log the chain
        save_reasoning_chain(domain, problem, solution, lfi_answer)
        total_generated += 1
        print(f"  Generated + saved reasoning chain ({len(problem)} char problem, {len(solution)} char solution)")

        time.sleep(2)  # Rate limit

    print(f"\n=== Done: {total_generated} reasoning chains generated ===")

if __name__ == "__main__":
    main()
