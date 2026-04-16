#!/usr/bin/env python3
"""Ingest ARC (AI2 Reasoning Challenge) — science reasoning questions."""
import json, sqlite3, urllib.request, os

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
# ARC Easy + Challenge sets
URLS = [
    ("arc_easy", "https://ai2-public-datasets.s3.amazonaws.com/arc/ARC-V1-Feb2018-2/ARC-Easy/ARC-Easy-Train.jsonl"),
    ("arc_challenge", "https://ai2-public-datasets.s3.amazonaws.com/arc/ARC-V1-Feb2018-2/ARC-Challenge/ARC-Challenge-Train.jsonl"),
]

def main():
    conn = sqlite3.connect(DB_PATH)
    total = 0
    for source_name, url in URLS:
        print(f"=== Downloading {source_name} ===")
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "PlausiDen-AI/1.0"})
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = resp.read().decode('utf-8')
        except Exception as e:
            print(f"  Failed: {e}")
            continue
        lines = [l for l in data.strip().split('\n') if l.strip()]
        ins = 0
        for i, line in enumerate(lines):
            try:
                ex = json.loads(line)
                q = ex.get("question", {})
                stem = q.get("stem", "")
                choices = q.get("choices", [])
                answer_key = ex.get("answerKey", "")
                if not stem: continue
                choices_text = " | ".join(f"{c['label']}: {c['text']}" for c in choices)
                correct = next((c['text'] for c in choices if c['label'] == answer_key), answer_key)
                key = f"{source_name}_{i:05d}"
                value = f"Q: {stem[:200]}\nChoices: {choices_text[:200]}\nAnswer: {correct}"
                conn.execute("INSERT OR IGNORE INTO facts (key, value, source, confidence) VALUES (?, ?, ?, 0.9)",
                    (key, value[:500], source_name))
                ins += 1
            except: continue
        conn.commit()
        total += ins
        print(f"  {ins} reasoning questions ingested")
    conn.close()
    print(f"=== Total: {total} ARC examples ===")

if __name__ == "__main__":
    main()
