#!/usr/bin/env python3
"""
CVE v5 Parser — Extract security facts from CVEProject/cvelistV5.
Inserts structured facts into brain.db for PlausiDen AI training.

Extracts: CVE ID, description, CVSS scores, affected products, CWE IDs,
date published, references. Each CVE becomes 1-3 facts depending on richness.

Usage: python3 parse_cve_v5.py [--limit N] [--year YYYY]
"""

import json
import sys
import os
import sqlite3
import time
import hashlib
from pathlib import Path

CVE_DIR = Path("/home/user/LFI-data/cvelistV5/cves")
DB_PATH = os.path.expanduser("~/.local/share/plausiden/brain.db")
BATCH_SIZE = 5000

def extract_cve_facts(filepath: Path) -> list:
    """Extract facts from a single CVE v5 JSON file."""
    facts = []
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (json.JSONDecodeError, UnicodeDecodeError, OSError):
        return facts

    cve_meta = data.get("cveMetadata", {})
    cve_id = cve_meta.get("cveId", "")
    state = cve_meta.get("state", "")
    date_published = cve_meta.get("datePublished", "")

    if not cve_id or state == "REJECTED":
        return facts

    containers = data.get("containers", {})
    cna = containers.get("cna", {})

    # Extract description
    descriptions = cna.get("descriptions", [])
    desc_en = ""
    for d in descriptions:
        if d.get("lang", "").startswith("en"):
            desc_en = d.get("value", "")
            break
    if not desc_en and descriptions:
        desc_en = descriptions[0].get("value", "")

    if not desc_en or len(desc_en) < 10:
        return facts

    # Truncate very long descriptions
    if len(desc_en) > 2000:
        desc_en = desc_en[:2000] + "..."

    # Extract CVSS score
    cvss_score = None
    cvss_severity = ""
    metrics = cna.get("metrics", [])
    for m in metrics:
        for key in ["cvssV3_1", "cvssV3_0", "cvssV4_0", "cvssV2_0"]:
            if key in m:
                cvss_data = m[key]
                cvss_score = cvss_data.get("baseScore")
                cvss_severity = cvss_data.get("baseSeverity", "")
                break
        if cvss_score:
            break

    # Extract affected products
    affected = cna.get("affected", [])
    products = []
    for a in affected[:5]:  # Limit to 5 products
        vendor = a.get("vendor", "unknown")
        product = a.get("product", "unknown")
        products.append(f"{vendor}/{product}")

    # Extract CWE
    problem_types = cna.get("problemTypes", [])
    cwes = []
    for pt in problem_types:
        for desc in pt.get("descriptions", []):
            cwe_id = desc.get("cweId", "")
            if cwe_id:
                cwes.append(cwe_id)

    # Build main fact
    key = f"cve:{cve_id}"
    value_parts = [f"{cve_id}: {desc_en}"]
    if cvss_score:
        value_parts.append(f"CVSS: {cvss_score} ({cvss_severity})")
    if products:
        value_parts.append(f"Affects: {', '.join(products)}")
    if cwes:
        value_parts.append(f"CWE: {', '.join(cwes)}")
    if date_published:
        value_parts.append(f"Published: {date_published[:10]}")

    value = " | ".join(value_parts)

    # Quality score based on richness
    quality = 0.5
    if cvss_score:
        quality += 0.15
    if products:
        quality += 0.1
    if cwes:
        quality += 0.1
    if len(desc_en) > 100:
        quality += 0.1
    if date_published:
        quality += 0.05

    facts.append({
        "key": key,
        "value": value,
        "confidence": min(quality, 1.0),
        "source": "cvelistV5",
        "domain": "cybersecurity",
    })

    # If CVSS is critical/high, add a severity fact
    if cvss_score and cvss_score >= 7.0:
        severity_key = f"cve_severity:{cve_id}"
        severity_val = (
            f"{cve_id} is rated {cvss_severity} ({cvss_score}/10). "
            f"{'CRITICAL: Immediate patching recommended.' if cvss_score >= 9.0 else 'HIGH: Prioritize patching.'}"
        )
        facts.append({
            "key": severity_key,
            "value": severity_val,
            "confidence": 0.95,
            "source": "cvelistV5",
            "domain": "cybersecurity",
        })

    return facts


def insert_batch(conn, facts):
    """Insert a batch of facts into brain.db."""
    cursor = conn.cursor()
    inserted = 0
    for fact in facts:
        try:
            cursor.execute(
                """INSERT OR IGNORE INTO facts (key, value, confidence, source, domain, quality_score)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6)""",
                (
                    fact["key"],
                    fact["value"],
                    fact["confidence"],
                    fact["source"],
                    fact["domain"],
                    fact["confidence"],
                ),
            )
            inserted += cursor.rowcount
        except sqlite3.Error:
            continue
    conn.commit()
    return inserted


def main():
    limit = 0  # 0 = no limit
    year_filter = None

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--limit" and i + 2 <= len(sys.argv[1:]):
            limit = int(sys.argv[i + 2])
        elif arg == "--year" and i + 2 <= len(sys.argv[1:]):
            year_filter = sys.argv[i + 2]

    if not CVE_DIR.exists():
        print(f"ERROR: CVE directory not found at {CVE_DIR}")
        sys.exit(1)

    # Collect all CVE JSON files
    print(f"Scanning CVE files in {CVE_DIR}...")
    cve_files = sorted(CVE_DIR.rglob("CVE-*.json"))
    if year_filter:
        cve_files = [f for f in cve_files if f"/{year_filter}/" in str(f)]
    if limit > 0:
        cve_files = cve_files[:limit]

    print(f"Found {len(cve_files)} CVE files to parse")

    # Connect to brain.db
    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA busy_timeout = 600000")
    conn.execute("PRAGMA journal_mode = WAL")
    conn.execute("PRAGMA synchronous = NORMAL")

    total_facts = 0
    total_inserted = 0
    batch = []
    start = time.time()

    for i, filepath in enumerate(cve_files):
        facts = extract_cve_facts(filepath)
        batch.extend(facts)
        total_facts += len(facts)

        if len(batch) >= BATCH_SIZE:
            inserted = insert_batch(conn, batch)
            total_inserted += inserted
            batch = []
            elapsed = time.time() - start
            rate = (i + 1) / max(elapsed, 1)
            print(
                f"  {i+1}/{len(cve_files)} files "
                f"({total_facts} facts extracted, {total_inserted} new, "
                f"{rate:.0f} files/sec)"
            )

    # Final batch
    if batch:
        inserted = insert_batch(conn, batch)
        total_inserted += inserted

    conn.close()
    elapsed = time.time() - start
    print(
        f"\n=== CVE Parse Complete ===\n"
        f"Files: {len(cve_files)}\n"
        f"Facts extracted: {total_facts}\n"
        f"New facts inserted: {total_inserted}\n"
        f"Time: {elapsed:.0f}s ({len(cve_files)/max(elapsed,1):.0f} files/sec)\n"
    )


if __name__ == "__main__":
    main()
