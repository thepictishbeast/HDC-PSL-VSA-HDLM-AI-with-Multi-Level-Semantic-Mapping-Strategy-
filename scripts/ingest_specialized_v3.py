#!/usr/bin/env python3
"""Ingest specialized datasets — v3 with correct schema (key, value)"""

import sqlite3, hashlib, time, traceback

DB = "/home/user/.local/share/plausiden/brain.db"

def get_conn():
    conn = sqlite3.connect(DB, timeout=300)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA busy_timeout=300000")
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn

def make_key(source, idx):
    return f"{source}_{idx:07d}"

def insert_batch(conn, rows):
    """rows: list of (key, value, source, confidence, domain)"""
    cur = conn.cursor()
    inserted = 0
    for key, value, source, conf, domain in rows:
        try:
            cur.execute(
                "INSERT OR IGNORE INTO facts (key, value, source, confidence, domain) VALUES (?, ?, ?, ?, ?)",
                (key, value, source, conf, domain)
            )
            inserted += cur.rowcount
        except Exception:
            continue
    conn.commit()
    return inserted

def ingest(name, config, source, domain, extract_fn, max_rows=500000):
    try:
        from datasets import load_dataset
        print(f"  Loading {name} (config={config})...", flush=True)
        
        kwargs = {"streaming": True}
        if config:
            ds = load_dataset(name, config, **kwargs)
        else:
            ds = load_dataset(name, **kwargs)
        
        # Try splits in order: train, test, validation, auxiliary_train
        split_ds = None
        for split_name in ["train", "test", "validation", "auxiliary_train"]:
            try:
                if config:
                    split_ds = load_dataset(name, config, split=split_name, streaming=True)
                else:
                    split_ds = load_dataset(name, split=split_name, streaming=True)
                print(f"    Using split: {split_name}", flush=True)
                break
            except (ValueError, KeyError):
                continue
        
        if split_ds is None:
            print(f"    No usable split found", flush=True)
            return 0
        
        conn = get_conn()
        batch = []
        total = 0
        idx = 0
        
        for row in split_ds:
            if idx >= max_rows:
                break
            idx += 1
            
            try:
                text = extract_fn(row)
                if not text or len(text.strip()) < 20:
                    continue
            except Exception:
                continue
            
            key = make_key(source, idx)
            batch.append((key, text[:4000], source, 0.85, domain))
            
            if len(batch) >= 5000:
                ins = insert_batch(conn, batch)
                total += ins
                batch = []
                if total % 50000 < 5000:
                    print(f"    {source}: {total:,} inserted", flush=True)
        
        if batch:
            total += insert_batch(conn, batch)
        
        conn.close()
        print(f"    {source}: DONE — {total:,} facts", flush=True)
        return total
    except Exception as e:
        print(f"    {source}: FAILED — {e}", flush=True)
        return 0

# --- Extraction functions ---
def ex_gsm8k(r):
    q, a = r.get("question",""), r.get("answer","")
    return f"Q: {q}\nA: {a}" if q and a else None

def ex_text(r):
    for f in ['text','content','sentence','passage','document','article','ctx','input','inputs']:
        v = r.get(f)
        if v and isinstance(v, str) and len(v) > 25: return v
    return None

def ex_qa(r):
    q = r.get("question", r.get("input", ""))
    a = r.get("answer", r.get("output", r.get("targets", "")))
    if q: return f"Q: {q}\nA: {a}" if a else f"Q: {q}"
    return None

def ex_dolly(r):
    inst = r.get("instruction",""); ctx = r.get("context",""); resp = r.get("response","")
    if inst and resp:
        s = f"Q: {inst}"
        if ctx: s += f"\nContext: {ctx[:1000]}"
        return s + f"\nA: {resp}"
    return None

def ex_oasst(r):
    return r.get("text") if r.get("lang","en") == "en" else None

def ex_mmlu(r):
    q = r.get("question",""); choices = r.get("choices",[]); ans = r.get("answer",0)
    subj = r.get("subject","")
    if q and choices:
        try: at = choices[int(ans)]
        except: at = str(choices[0])
        return f"[{subj}] Q: {q}\nA: {at}"
    return None

def ex_squad(r):
    q = r.get("question",""); ctx = r.get("context","")
    answers = r.get("answers",{})
    at = ""
    if isinstance(answers, dict) and "text" in answers:
        texts = answers["text"]
        at = texts[0] if texts else ""
    if q and ctx: return f"Context: {ctx[:1500]}\nQ: {q}\nA: {at}"
    return None

def ex_code(r):
    inst = r.get("instruction", r.get("prompt", ""))
    out = r.get("output", r.get("completion", ""))
    if inst and out: return f"Task: {inst}\nSolution: {out}"
    return None

def ex_hellaswag(r):
    ctx = r.get("ctx", r.get("ctx_a",""))
    endings = r.get("endings",[])
    label = r.get("label","0")
    if ctx and endings:
        try: return f"{ctx} {endings[int(label)]}"
        except: return ctx if len(ctx) > 30 else None
    return None

def ex_arc(r):
    q = r.get("question","")
    choices = r.get("choices",{})
    ak = r.get("answerKey","")
    at = ""
    if isinstance(choices, dict) and "text" in choices:
        for l, t in zip(choices.get("label",[]), choices.get("text",[])):
            if l == ak: at = t; break
    return f"Q: {q}\nA: {at}" if q else None

print("=" * 60, flush=True)
print("SPECIALIZED DOMAIN INGESTION v3 — correct schema", flush=True)
print("=" * 60, flush=True)

grand_total = 0
datasets = [
    # MATH/REASONING
    ("openai/gsm8k", "main", "gsm8k", "mathematics", ex_gsm8k, 50000),
    ("Rowan/hellaswag", None, "hellaswag", "commonsense", ex_hellaswag, 100000),
    ("allenai/ai2_arc", "ARC-Challenge", "arc_challenge", "science", ex_arc, 50000),
    ("allenai/ai2_arc", "ARC-Easy", "arc_easy", "science", ex_arc, 50000),
    
    # ACADEMIC
    ("cais/mmlu", "all", "mmlu", "academic", ex_mmlu, 500000),
    
    # READING COMPREHENSION
    ("rajpurkar/squad_v2", None, "squad_v2", "reading_comprehension", ex_squad, 200000),
    
    # INSTRUCTION FOLLOWING
    ("databricks/databricks-dolly-15k", None, "dolly", "instruction", ex_dolly, 50000),
    ("sahil2801/CodeAlpaca-20k", None, "code_alpaca", "code", ex_code, 50000),
    
    # CONVERSATIONAL
    ("OpenAssistant/oasst1", None, "oasst1", "conversational", ex_oasst, 200000),
    
    # LARGE MULTI-TASK
    ("Muennighoff/flan", None, "flan", "multi_task", ex_qa, 1000000),
]

for name, config, source, domain, extract_fn, max_rows in datasets:
    count = ingest(name, config, source, domain, extract_fn, max_rows)
    grand_total += count
    time.sleep(2)

print(f"\n{'=' * 60}", flush=True)
print(f"GRAND TOTAL: {grand_total:,} new facts", flush=True)
conn = get_conn()
total = conn.execute("SELECT count(*) FROM facts").fetchone()[0]
conn.close()
print(f"brain.db total: {total:,} facts", flush=True)
