import sqlite3, json, random
DB = "/home/user/.local/share/plausiden/brain.db"
OUTPUT = "/root/LFI/lfi_vsa_core/training_data_lora.jsonl"
conn = sqlite3.connect(DB, timeout=300)
conn.execute("PRAGMA busy_timeout=30000")
pairs = []

# Curated facts → Q&A
for value, domain, source in conn.execute("SELECT value, domain, source FROM facts WHERE quality_score >= 0.90 AND length(value) > 100 AND source LIKE 'curated_%' LIMIT 5000").fetchall():
    if ":" in value and len(value.split(":")[0]) < 100:
        pairs.append({"instruction": f"Explain {value.split(':')[0].strip()}", "input": "", "output": value, "domain": domain or "general"})

# Adversarial → rejection pairs
for claim, label, evidence, explanation in conn.execute("SELECT claim, label, evidence, explanation FROM adversarial WHERE length(claim) > 50 ORDER BY RANDOM() LIMIT 5000").fetchall():
    if label in ("refuted", "contradiction"):
        pairs.append({"instruction": f"Is this true? {claim[:300]}", "input": "", "output": f"FALSE. {(evidence or explanation or '')[:300]}", "domain": "adversarial"})

# Instruction pairs
for value, domain in conn.execute("SELECT value, domain FROM facts WHERE quality_score >= 0.88 AND length(value) > 100 AND value LIKE 'Q:%' LIMIT 10000").fetchall():
    if "A:" in value:
        q, a = value.split("A:", 1)
        q = q.replace("Q:", "").strip()
        if q and a.strip():
            pairs.append({"instruction": q, "input": "", "output": a.strip(), "domain": domain or "general"})

conn.close()
random.shuffle(pairs)
with open(OUTPUT, 'w') as f:
    for p in pairs:
        f.write(json.dumps(p) + '\n')
print(f"Generated {len(pairs)} LoRA pairs")
domains = {}
for p in pairs:
    domains[p['domain']] = domains.get(p['domain'], 0) + 1
for d, c in sorted(domains.items(), key=lambda x: -x[1])[:10]:
    print(f"  {d}: {c}")
