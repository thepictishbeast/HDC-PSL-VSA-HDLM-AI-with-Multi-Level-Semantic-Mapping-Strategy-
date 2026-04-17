#!/usr/bin/env python3
"""
Ingest GSM8K (Grade School Math 8K) reasoning chains into brain.db.
Per Training Strategy §2.3 Tier 3: reasoning chains train the
DerivationTrace pattern library. GSM8K has 8,500 step-by-step solutions.

Source: https://raw.githubusercontent.com/openai/grade-school-math/master/grade_school_math/data/train.jsonl
License: MIT
"""
import json, sqlite3, urllib.request, os

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
URL = "https://raw.githubusercontent.com/openai/grade-school-math/master/grade_school_math/data/train.jsonl"

def main():
    print("=== Downloading GSM8K training set ===")
    try:
        req = urllib.request.Request(URL, headers={"User-Agent": "PlausiDen-AI/1.0"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = resp.read().decode('utf-8')
    except Exception as e:
        print(f"Download failed: {e}")
        return

    lines = [l for l in data.strip().split('\n') if l.strip()]
    print(f"Downloaded {len(lines)} examples")

    conn = sqlite3.connect(DB_PATH)
    conn.execute("""CREATE TABLE IF NOT EXISTS facts (
        key TEXT PRIMARY KEY, value TEXT NOT NULL,
        source TEXT DEFAULT 'gsm8k', confidence REAL DEFAULT 1.0,
        created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now'))
    )""")

    inserted = 0
    for i, line in enumerate(lines):
        try:
            ex = json.loads(line)
            question = ex.get("question", "")
            answer = ex.get("answer", "")
            if not question or not answer:
                continue
            # Extract the final numeric answer
            final = answer.split("####")[-1].strip() if "####" in answer else answer[-50:]
            # Store as a reasoning chain fact
            key = f"gsm8k_{i:05d}"
            value = f"Q: {question[:200]}\nA: {answer[:300]}"
            conn.execute(
                "INSERT OR IGNORE INTO facts (key, value, source, confidence) VALUES (?, ?, 'gsm8k', 0.95)",
                (key, value[:500])
            )
            inserted += 1
        except json.JSONDecodeError:
            continue

    conn.commit()
    conn.close()
    print(f"=== Inserted {inserted} reasoning chains from GSM8K ===")

if __name__ == "__main__":
    main()
