#!/usr/bin/env python3
"""
Quality Classifier for PlausiDen AI brain.db facts.
Uses a simple heuristic classifier (no ML deps needed) to relabel
quality_score for all 56M+ facts based on content features.

Scores based on:
- Text length (longer = higher quality, up to a point)
- Vocabulary richness (unique words / total words)
- Has structured data (numbers, dates, proper nouns)
- Source reputation tier
- Domain specificity

Usage: python3 quality_classifier.py [--batch-size N] [--dry-run]
"""

import sqlite3
import os
import sys
import time
import re
import math

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
BATCH_SIZE = 10000

# Source quality tiers (higher = better)
SOURCE_TIERS = {
    # Tier 1: Curated/academic (0.90-0.95)
    "curated": 0.95, "mitre_attack": 0.95, "cwe": 0.95,
    "anli": 0.90, "snli": 0.90, "wikipedia": 0.90,
    "fever_gold": 0.90, "truthfulqa": 0.95, "aquarat": 0.90,
    "winogrande": 0.90,
    # Tier 2: High quality (0.80-0.89)
    "conceptnet": 0.85, "xnli": 0.85, "dolly": 0.85,
    "pile_uncopyrighted": 0.80, "cvelistV5": 0.80,
    "oasst2": 0.85,
    # Tier 3: Web knowledge (0.65-0.79)
    "openwebtext": 0.75, "c4": 0.70,
    # Tier 4: Noisy (0.50-0.64)
    "reviews": 0.60, "magpie": 0.60,
}

# Domain bonuses
DOMAIN_BONUS = {
    "cybersecurity": 0.05,
    "reasoning": 0.05,
    "nli": 0.03,
    "adversarial": 0.05,
    "mathematics": 0.05,
    "code": 0.03,
    "biomedical": 0.03,
}


def compute_quality(value: str, source: str, domain: str) -> float:
    """Compute quality score for a fact based on content features."""
    score = 0.5  # baseline

    # 1. Source tier
    source_lower = source.lower() if source else ""
    for src_key, src_score in SOURCE_TIERS.items():
        if src_key in source_lower:
            score = src_score
            break

    # 2. Length bonus (optimal: 50-500 chars)
    length = len(value) if value else 0
    if length < 10:
        score *= 0.5
    elif length < 30:
        score *= 0.8
    elif 50 <= length <= 500:
        score *= 1.0  # optimal
    elif 500 < length <= 2000:
        score *= 0.95
    elif length > 2000:
        score *= 0.85  # too long may be noisy

    # 3. Vocabulary richness (for text > 20 words)
    words = value.split() if value else []
    if len(words) > 20:
        unique_ratio = len(set(w.lower() for w in words)) / len(words)
        if unique_ratio > 0.7:
            score += 0.03  # rich vocabulary
        elif unique_ratio < 0.3:
            score -= 0.05  # repetitive

    # 4. Structured content bonus
    if value:
        has_numbers = bool(re.search(r'\d+\.?\d*', value))
        has_proper_nouns = bool(re.search(r'[A-Z][a-z]{2,}', value))
        has_technical = bool(re.search(r'[({}\[\]<>=/|&]', value))
        if has_numbers: score += 0.02
        if has_proper_nouns: score += 0.01
        if has_technical: score += 0.01

    # 5. Domain bonus
    if domain and domain.lower() in DOMAIN_BONUS:
        score += DOMAIN_BONUS[domain.lower()]

    # 6. Penalty for low-quality indicators
    if value:
        val_lower = value.lower()
        if "lorem ipsum" in val_lower: score *= 0.3
        if val_lower.count("http") > 5: score *= 0.8  # link spam
        if len(set(value)) < 10: score *= 0.5  # very few unique chars

    return max(0.1, min(1.0, score))


def main():
    batch_size = BATCH_SIZE
    dry_run = False

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--batch-size" and i + 2 <= len(sys.argv[1:]):
            batch_size = int(sys.argv[i + 2])
        elif arg == "--dry-run":
            dry_run = True

    conn = sqlite3.connect(DB_PATH, timeout=60)
    conn.execute("PRAGMA busy_timeout = 600000")
    conn.execute("PRAGMA journal_mode = WAL")
    conn.execute("PRAGMA synchronous = NORMAL")

    total = conn.execute("SELECT COUNT(*) FROM facts").fetchone()[0]
    print(f"=== Quality Classifier ===")
    print(f"Total facts: {total:,}")
    print(f"Batch size: {batch_size:,}")
    print(f"Dry run: {dry_run}")

    # Sample current quality distribution
    print("\nCurrent quality distribution:")
    for row in conn.execute(
        "SELECT ROUND(COALESCE(quality_score, 0), 1) as q, COUNT(*) FROM facts GROUP BY q ORDER BY q"
    ).fetchall():
        print(f"  {row[0]:.1f}: {row[1]:,}")

    if dry_run:
        # Just show what would change on a sample
        print("\nDry run — sampling 100 facts:")
        sample = conn.execute(
            "SELECT rowid, key, value, source, domain, quality_score FROM facts ORDER BY RANDOM() LIMIT 100"
        ).fetchall()
        changes = 0
        for rowid, key, value, source, domain, old_score in sample:
            new_score = compute_quality(value or "", source or "", domain or "")
            if abs(new_score - (old_score or 0)) > 0.05:
                changes += 1
                print(f"  {key[:40]}: {old_score:.2f} → {new_score:.2f} (src={source}, dom={domain})")
        print(f"\n{changes}/100 would change by >0.05")
        conn.close()
        return

    # Process all facts in batches
    print(f"\nProcessing {total:,} facts...")
    processed = 0
    updated = 0
    t0 = time.time()
    offset = 0

    while offset < total:
        rows = conn.execute(
            "SELECT rowid, value, source, domain, quality_score FROM facts LIMIT ? OFFSET ?",
            (batch_size, offset)
        ).fetchall()

        if not rows:
            break

        updates = []
        for rowid, value, source, domain, old_score in rows:
            new_score = compute_quality(value or "", source or "", domain or "")
            old = old_score if old_score is not None else 0.0
            if abs(new_score - old) > 0.02:  # Only update if meaningful change
                updates.append((new_score, rowid))

        if updates:
            conn.executemany("UPDATE facts SET quality_score = ? WHERE rowid = ?", updates)
            conn.commit()
            updated += len(updates)

        processed += len(rows)
        offset += batch_size

        if processed % (batch_size * 10) == 0:
            elapsed = time.time() - t0
            rate = processed / max(elapsed, 1)
            eta = (total - processed) / max(rate, 1)
            print(
                f"  {processed:,}/{total:,} ({100*processed/total:.1f}%) "
                f"| {updated:,} updated | {rate:.0f}/s | ETA {eta/60:.0f}m",
                flush=True
            )

    elapsed = time.time() - t0
    conn.close()

    print(f"\n=== Done ===")
    print(f"Processed: {processed:,}")
    print(f"Updated: {updated:,}")
    print(f"Time: {elapsed:.0f}s ({processed/max(elapsed,1):.0f}/s)")


if __name__ == "__main__":
    main()
