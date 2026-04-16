#!/usr/bin/env python3
"""
Ingest ConceptNet edges into PlausiDen brain.db as structured facts.
Uses the ConceptNet API to avoid downloading the full 2GB dump.
Targets: the 11 training domains + common knowledge.

Per Training Strategy §2.1: structured knowledge graphs are Tier 1 data —
highest value per byte for HDC training.
"""
import json
import sqlite3
import urllib.request
import time
import os
import sys

DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
API_BASE = "http://api.conceptnet.io"
BATCH_SIZE = 50

# Domain-relevant seed concepts to query
SEEDS = {
    "security": ["encryption", "firewall", "vulnerability", "authentication", "malware",
                  "phishing", "intrusion", "certificate", "zero_trust", "penetration_testing"],
    "math": ["algebra", "calculus", "geometry", "prime_number", "integral",
             "derivative", "matrix", "probability", "theorem", "equation"],
    "code": ["programming", "compiler", "algorithm", "data_structure", "recursion",
             "function", "variable", "debugging", "software", "binary"],
    "philosophy": ["consciousness", "free_will", "existence", "ethics", "epistemology",
                   "ontology", "metaphysics", "logic", "truth", "reality"],
    "physics": ["gravity", "quantum", "relativity", "energy", "momentum",
                "entropy", "electromagnetism", "photon", "wave", "particle"],
    "biology": ["cell", "dna", "evolution", "protein", "mitosis",
                "neuron", "ecosystem", "photosynthesis", "genetics", "immune_system"],
    "psychology": ["cognition", "memory", "emotion", "perception", "motivation",
                   "learning", "personality", "consciousness", "behavior", "anxiety"],
    "general": ["computer", "internet", "language", "science", "technology",
                "human", "earth", "water", "time", "music",
                "food", "city", "country", "animal", "plant"],
}

def fetch_edges(concept, limit=100):
    """Fetch edges for a concept from ConceptNet API."""
    url = f"{API_BASE}/c/en/{concept}?limit={limit}"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "PlausiDen-AI/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
            return data.get("edges", [])
    except Exception as e:
        print(f"  WARN: failed to fetch {concept}: {e}", file=sys.stderr)
        return []

def edge_to_fact(edge):
    """Convert a ConceptNet edge to a (key, value, relation, weight) tuple."""
    start = edge.get("start", {}).get("label", "")
    end = edge.get("end", {}).get("label", "")
    rel = edge.get("rel", {}).get("label", "")
    weight = edge.get("weight", 1.0)
    if not start or not end or not rel:
        return None
    # Skip non-English or very low-weight edges
    start_lang = edge.get("start", {}).get("language", "en")
    end_lang = edge.get("end", {}).get("language", "en")
    if start_lang != "en" or end_lang != "en":
        return None
    if weight < 1.0:
        return None
    key = f"cn_{start.lower().replace(' ', '_')}_{rel.lower().replace(' ', '_')}_{end.lower().replace(' ', '_')}"
    value = f"{start} {rel} {end}"
    return (key[:200], value[:500], rel, min(weight / 10.0, 1.0))

def main():
    conn = sqlite3.connect(DB_PATH)
    # Ensure table exists
    conn.execute("""
        CREATE TABLE IF NOT EXISTS facts (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            source TEXT DEFAULT 'conceptnet',
            confidence REAL DEFAULT 1.0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )
    """)

    total_inserted = 0
    total_skipped = 0

    for domain, seeds in SEEDS.items():
        print(f"\n=== Domain: {domain} ({len(seeds)} seeds) ===")
        for concept in seeds:
            edges = fetch_edges(concept, limit=200)
            batch = []
            for edge in edges:
                fact = edge_to_fact(edge)
                if fact:
                    batch.append(fact)

            inserted = 0
            for key, value, rel, conf in batch:
                try:
                    conn.execute(
                        """INSERT INTO facts (key, value, source, confidence, updated_at)
                           VALUES (?, ?, 'conceptnet', ?, datetime('now'))
                           ON CONFLICT(key) DO UPDATE SET confidence=MAX(confidence, ?), updated_at=datetime('now')""",
                        (key, value, conf, conf)
                    )
                    inserted += 1
                except Exception:
                    total_skipped += 1

            conn.commit()
            total_inserted += inserted
            print(f"  {concept}: {inserted} facts from {len(edges)} edges")
            time.sleep(0.5)  # Rate limit

    conn.close()
    print(f"\n=== DONE: {total_inserted} facts inserted, {total_skipped} skipped ===")
    print(f"DB: {DB_PATH}")

if __name__ == "__main__":
    main()
