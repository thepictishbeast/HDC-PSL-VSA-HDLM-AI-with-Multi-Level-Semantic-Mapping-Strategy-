#!/usr/bin/env python3
"""
DPO (Direct Preference Optimization) Training Pipeline.

Creates chosen/rejected pairs from:
1. User corrections (correction_trainer data)
2. Reward model scoring (high vs low quality variants)
3. Self-play: generate 2 responses, pick the better one

Output: DPO-format JSONL for fine-tuning.

Usage:
    python3 dpo_pipeline.py --source corrections --output dpo_pairs.jsonl
    python3 dpo_pipeline.py --source reward_model --output dpo_pairs.jsonl
    python3 dpo_pipeline.py --source self_play --count 50
"""

import argparse
import json
import hashlib
import os
import sqlite3
import requests
import time

BRAIN_DB = os.path.expanduser("~/.local/share/plausiden/brain.db")
MODEL = os.environ.get("PLAUSIDEN_MODEL", "qwen2.5-coder:7b")
OUTPUT_DIR = os.path.expanduser("~/LFI-data/dpo_pairs")


def generate_response(prompt: str, temperature: float = 0.7) -> str:
    try:
        r = requests.post("http://localhost:11434/api/generate", json={
            "model": MODEL, "stream": False, "prompt": prompt,
            "options": {"temperature": temperature, "num_predict": 300}
        }, timeout=60)
        return r.json().get("response", "").strip()
    except Exception:
        return ""


def from_corrections() -> list:
    """Generate DPO pairs from user corrections stored in fact_versions."""
    conn = sqlite3.connect(BRAIN_DB, timeout=30)
    conn.execute("PRAGMA busy_timeout=30000")

    rows = conn.execute(
        "SELECT fact_key, old_value, new_value, reason FROM fact_versions "
        "WHERE change_type = 'user_correction' AND old_value IS NOT NULL "
        "ORDER BY id DESC LIMIT 200"
    ).fetchall()
    conn.close()

    pairs = []
    for key, old_val, new_val, reason in rows:
        prompt = reason or key
        pairs.append({
            "prompt": prompt,
            "chosen": new_val,
            "rejected": old_val,
            "source": "user_correction",
        })
    return pairs


def from_reward_model(count: int = 100) -> list:
    """Generate DPO pairs by scoring existing facts with reward model heuristics."""
    conn = sqlite3.connect(BRAIN_DB, timeout=30)
    conn.execute("PRAGMA busy_timeout=30000")

    # Get high-quality and low-quality fact pairs from same domain
    pairs = []
    domains = conn.execute(
        "SELECT DISTINCT domain FROM facts WHERE domain IS NOT NULL AND quality_score IS NOT NULL LIMIT 20"
    ).fetchall()

    for (domain,) in domains:
        if len(pairs) >= count:
            break

        high = conn.execute(
            "SELECT key, value FROM facts WHERE domain=? AND quality_score >= 0.8 "
            "ORDER BY RANDOM() LIMIT 5", (domain,)
        ).fetchall()

        low = conn.execute(
            "SELECT key, value FROM facts WHERE domain=? AND quality_score < 0.5 "
            "ORDER BY RANDOM() LIMIT 5", (domain,)
        ).fetchall()

        for (hk, hv), (lk, lv) in zip(high, low):
            # Extract Q&A if present
            hq = hv.split("A:", 1)[0].replace("Q:", "").strip() if "Q:" in hv else hv[:80]
            ha = hv.split("A:", 1)[1].strip() if "A:" in hv else hv
            la = lv.split("A:", 1)[1].strip() if "A:" in lv else lv

            pairs.append({
                "prompt": hq,
                "chosen": ha[:1000],
                "rejected": la[:1000],
                "source": "reward_model_ranking",
                "domain": domain,
            })

    conn.close()
    return pairs[:count]


def from_self_play(count: int = 20) -> list:
    """Generate DPO pairs via self-play: generate 2 responses, pick better one."""
    conn = sqlite3.connect(BRAIN_DB, timeout=30)
    conn.execute("PRAGMA busy_timeout=30000")

    # Get random questions
    questions = conn.execute(
        "SELECT value FROM facts WHERE value LIKE 'Q:%' ORDER BY RANDOM() LIMIT ?",
        (count,)
    ).fetchall()
    conn.close()

    pairs = []
    for (val,) in questions:
        q = val.split("A:", 1)[0].replace("Q:", "").strip() if "A:" in val else val[:100]
        if len(q) < 10:
            continue

        # Generate two responses at different temperatures
        r1 = generate_response(f"Answer this question thoroughly:\n{q}", temperature=0.5)
        r2 = generate_response(f"Answer this question thoroughly:\n{q}", temperature=0.9)

        if not r1 or not r2:
            continue

        # Simple heuristic: longer, more structured = better
        score1 = len(r1) + r1.count('\n') * 10 + r1.count('```') * 20
        score2 = len(r2) + r2.count('\n') * 10 + r2.count('```') * 20

        if score1 > score2:
            chosen, rejected = r1, r2
        else:
            chosen, rejected = r2, r1

        pairs.append({
            "prompt": q,
            "chosen": chosen[:1000],
            "rejected": rejected[:1000],
            "source": "self_play",
        })
        print(f"  Self-play: {q[:50]}... (scores: {score1} vs {score2})")

    return pairs


def save_pairs(pairs: list, output: str):
    os.makedirs(os.path.dirname(output) or ".", exist_ok=True)
    with open(output, "w") as f:
        for p in pairs:
            f.write(json.dumps(p) + "\n")
    print(f"Saved {len(pairs)} DPO pairs to {output}")


def main():
    parser = argparse.ArgumentParser(description="DPO Training Pipeline")
    parser.add_argument("--source", choices=["corrections", "reward_model", "self_play", "all"], default="all")
    parser.add_argument("--output", default=None)
    parser.add_argument("--count", type=int, default=50)
    args = parser.parse_args()

    os.makedirs(OUTPUT_DIR, exist_ok=True)
    ts = time.strftime("%Y%m%d_%H%M%S")
    output = args.output or os.path.join(OUTPUT_DIR, f"dpo_{args.source}_{ts}.jsonl")

    all_pairs = []

    if args.source in ("corrections", "all"):
        pairs = from_corrections()
        all_pairs.extend(pairs)
        print(f"Corrections: {len(pairs)} DPO pairs")

    if args.source in ("reward_model", "all"):
        pairs = from_reward_model(args.count)
        all_pairs.extend(pairs)
        print(f"Reward model: {len(pairs)} DPO pairs")

    if args.source in ("self_play", "all"):
        pairs = from_self_play(min(args.count, 10))  # Self-play is slow
        all_pairs.extend(pairs)
        print(f"Self-play: {len(pairs)} DPO pairs")

    if all_pairs:
        save_pairs(all_pairs, output)
    else:
        print("No DPO pairs generated")


if __name__ == "__main__":
    main()
