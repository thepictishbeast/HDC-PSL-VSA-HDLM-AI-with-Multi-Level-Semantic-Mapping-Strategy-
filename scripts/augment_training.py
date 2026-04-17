#!/usr/bin/env python3
"""
Training Data Augmentation Pipeline — Triple data volume without new sources.

Techniques:
1. Paraphrase: Rephrase questions while keeping meaning
2. Entity swap: Replace entities with similar ones from same domain
3. Complexity scaling: Simplify complex Q&A for beginner-level training
4. Reverse Q&A: Turn statements into questions
5. Chain extension: Link related Q&A pairs into multi-hop reasoning

Usage:
    python3 augment_training.py --input data.jsonl --output augmented.jsonl --factor 3
"""

import argparse
import json
import hashlib
import os
import random
import re
import sqlite3
import sys

BRAIN_DB = os.path.expanduser("~/.local/share/plausiden/brain.db")


def paraphrase_question(question: str) -> list[str]:
    """Generate paraphrased versions of a question."""
    variants = []
    q = question.strip()

    # Technique 1: Question word swap
    swaps = {
        "What is": ["Explain", "Define", "Describe", "What do you mean by"],
        "How does": ["In what way does", "Explain how", "Describe the mechanism by which"],
        "Why is": ["What is the reason", "Explain why", "What causes"],
        "How can": ["What methods can", "In what ways can", "What approaches can"],
        "What are": ["List", "Name", "Identify", "Enumerate"],
    }
    for prefix, alternatives in swaps.items():
        if q.startswith(prefix):
            alt = random.choice(alternatives)
            variants.append(alt + q[len(prefix):])
            break

    # Technique 2: Add context/specificity
    if q.endswith("?"):
        contexts = [
            f"In simple terms, {q.lower()}",
            f"From a practical perspective, {q.lower()}",
            f"Can you elaborate on: {q}",
        ]
        variants.append(random.choice(contexts))

    # Technique 3: Reverse (statement → question)
    if not q.endswith("?") and " is " in q:
        parts = q.split(" is ", 1)
        if len(parts) == 2:
            variants.append(f"What is {parts[0].strip()}?")

    return variants[:2]  # Max 2 variants per question


def simplify_answer(answer: str) -> str:
    """Create a simplified version of a complex answer."""
    sentences = answer.split(". ")
    if len(sentences) <= 2:
        return answer

    # Keep first and last sentence, drop middle complexity
    simplified = f"{sentences[0]}. {sentences[-1]}"
    return simplified


def reverse_qa(question: str, answer: str) -> dict | None:
    """Turn an answer into a question and the question into context."""
    if len(answer) < 30 or len(question) < 10:
        return None

    first_sentence = answer.split(".")[0].strip()
    if len(first_sentence) < 20:
        return None

    new_q = f"What concept is described by: '{first_sentence}'?"
    new_a = f"This describes: {question.rstrip('?')}. {answer}"

    return {"instruction": new_q, "output": new_a[:1000]}


def augment_pair(pair: dict) -> list[dict]:
    """Generate augmented versions of a single training pair."""
    augmented = []
    instruction = pair.get("instruction", pair.get("prompt", ""))
    output = pair.get("output", pair.get("response", ""))
    domain = pair.get("domain", "general")

    if not instruction or not output:
        return augmented

    # 1. Paraphrased questions with same answer
    for variant in paraphrase_question(instruction):
        augmented.append({
            "instruction": variant,
            "output": output,
            "domain": domain,
            "source": "augmented_paraphrase",
            "quality": float(pair.get("quality", 0.7)) * 0.95,  # Slight quality discount
        })

    # 2. Simplified version
    if len(output) > 200:
        simple = simplify_answer(output)
        augmented.append({
            "instruction": f"Briefly explain: {instruction}",
            "output": simple,
            "domain": domain,
            "source": "augmented_simplified",
            "quality": float(pair.get("quality", 0.7)) * 0.9,
        })

    # 3. Reverse Q&A
    reversed_pair = reverse_qa(instruction, output)
    if reversed_pair:
        reversed_pair["domain"] = domain
        reversed_pair["source"] = "augmented_reversed"
        reversed_pair["quality"] = float(pair.get("quality", 0.7)) * 0.85
        augmented.append(reversed_pair)

    return augmented


def augment_file(input_path: str, output_path: str, factor: int = 3) -> int:
    """Augment an entire JSONL file."""
    pairs = []
    with open(input_path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                pairs.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    augmented = []
    for pair in pairs:
        # Keep original
        augmented.append(pair)
        # Generate augmentations
        aug = augment_pair(pair)
        augmented.extend(aug[:factor - 1])  # Cap augmentations per pair

    # Deduplicate by instruction hash
    seen = set()
    unique = []
    for p in augmented:
        key = hashlib.sha256(p.get("instruction", "").encode()).hexdigest()[:16]
        if key not in seen:
            seen.add(key)
            unique.append(p)

    with open(output_path, "w") as f:
        for p in unique:
            f.write(json.dumps(p) + "\n")

    return len(unique)


def ingest_augmented(path: str, db_path: str = BRAIN_DB) -> int:
    """Ingest augmented pairs into brain.db."""
    conn = sqlite3.connect(db_path, timeout=30)
    conn.execute("PRAGMA busy_timeout=30000")

    added = 0
    with open(path) as f:
        for line in f:
            try:
                d = json.loads(line.strip())
                q = d.get("instruction", "")
                a = d.get("output", "")
                if not q or not a:
                    continue
                domain = d.get("domain", "general")
                quality = float(d.get("quality", 0.7))
                source = d.get("source", "augmented")
                key = f"aug_{hashlib.sha256((q + a).encode()).hexdigest()[:16]}"
                conn.execute(
                    "INSERT OR IGNORE INTO facts (key,value,source,confidence,domain,quality_score) VALUES (?,?,?,?,?,?)",
                    (key, f"Q: {q}\nA: {a}"[:5000], source, quality, domain, quality)
                )
                added += 1
            except Exception:
                continue

    conn.commit()
    conn.close()
    return added


def main():
    parser = argparse.ArgumentParser(description="Training Data Augmentation Pipeline")
    parser.add_argument("--input", required=True, help="Input JSONL file")
    parser.add_argument("--output", default=None, help="Output JSONL file (default: input_augmented.jsonl)")
    parser.add_argument("--factor", type=int, default=3, help="Augmentation factor (default: 3x)")
    parser.add_argument("--ingest", action="store_true", help="Also ingest into brain.db")
    args = parser.parse_args()

    output = args.output or args.input.replace(".jsonl", "_augmented.jsonl")
    count = augment_file(args.input, output, args.factor)
    print(f"Augmented: {count} pairs → {output}")

    if args.ingest:
        added = ingest_augmented(output)
        print(f"Ingested: {added} augmented pairs into brain.db")


if __name__ == "__main__":
    main()
