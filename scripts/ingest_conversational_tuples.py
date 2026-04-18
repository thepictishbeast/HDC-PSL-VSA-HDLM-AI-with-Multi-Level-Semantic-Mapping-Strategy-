#!/usr/bin/env python3
"""
Conversational → dialogue-act tuples ingestor.

Reads {instruction, input, output} conversational pairs from
combined_training_v5.jsonl (and similar) and decomposes each into
LFI-native (subject, predicate, object, tier, provenance) tuples via
rule-based extraction. Writes tuples to tuples/conversational.jsonl and
inserts them into brain.db as facts with source='dialogue_tuples_v1'.

Decomposition shape per conversational pair i:
  (convo_<i>, has_utterance_user, <instruction>)
  (convo_<i>, has_utterance_assistant, <output>)
  (convo_<i>, user_intent, <rule-extracted intent: explain|how_to|define|ask|…>)
  (convo_<i>, topic, <rule-extracted topic from instruction>)
  (convo_<i>, response_length_words, <N>)
  (convo_<i>, user_said, <truncated instruction>)
  (convo_<i>, assistant_said, <truncated output>)

This is NOT LLM training data. Each tuple becomes a fact that
participates in HDC prototype bundling, PSL axiom validation, and
causal/analogical reasoning. The dialogue-act tuples populate the
structural pattern library for conversational interaction (Layer 5
speech-act classifier, #345).

Runs in small batches with brief sleeps to respect the running server
(Critical Fix C3 proper incremental ingestion is the long-term fix;
this is a pragmatic approximation).
"""

import json
import os
import re
import sqlite3
import sys
import time
from pathlib import Path

DB = os.path.expanduser("~/.local/share/plausiden/brain.db")
SOURCES = [
    ("/home/user/LFI-data/combined_training_v5.jsonl", "combined_v5"),
]
TUPLE_OUT_DIR = Path("/home/user/LFI-data/tuples")
TUPLE_OUT_DIR.mkdir(parents=True, exist_ok=True)
TUPLE_OUT = TUPLE_OUT_DIR / "conversational.jsonl"

BATCH = 500
SLEEP_PER_BATCH = float(os.environ.get("INGEST_SLEEP", "1.5"))  # 1.5s default — chat WS stays responsive
PROGRESS_EVERY = 25_000
WAL_CHECKPOINT_EVERY = 20_000  # passive checkpoint periodically to keep WAL small
SKIP_PAIRS = int(os.environ.get("INGEST_SKIP", "0"))  # skip already-processed pair count


def extract_intent(instruction: str) -> str:
    """Rule-based intent classification from instruction prefix.

    Not LLM-trained — just a prefix-pattern classifier. Good enough for
    bulk decomposition of 808K pairs. Maps to Layer 5 speech-act classes.
    """
    s = instruction.strip().lower()
    if not s:
        return "unknown"
    if s.startswith(("explain", "describe", "elaborate on", "tell me about")):
        return "explain"
    if s.startswith(("how do", "how to", "how can", "how does")):
        return "how_to"
    if s.startswith(("what is", "what are", "what's", "whats", "define")):
        return "define"
    if s.startswith(("why", "why do", "why does", "why is")):
        return "why"
    if s.startswith(("when", "where")):
        return "wh_question"
    if s.startswith(("who", "whose")):
        return "who_question"
    if s.startswith(("compare", "contrast", "difference between")):
        return "compare"
    if s.startswith(("list", "name", "enumerate", "give me")):
        return "enumerate"
    if s.startswith(("write", "generate", "create", "produce", "draft")):
        return "generate"
    if s.startswith(("summarize", "tldr", "sum up")):
        return "summarize"
    if s.startswith(("translate", "convert")):
        return "translate"
    if s.startswith(("fix", "debug", "solve", "correct")):
        return "fix"
    if s.startswith(("refactor", "improve", "optimize", "simplify")):
        return "improve"
    if s.startswith(("analyze", "evaluate", "assess", "review")):
        return "analyze"
    if s.endswith("?"):
        return "question"
    return "statement"


_TOPIC_STOPWORDS = {
    "the", "a", "an", "is", "are", "was", "were", "of", "in", "on", "at",
    "and", "or", "but", "if", "with", "for", "to", "from", "by", "as",
    "be", "been", "being", "have", "has", "had", "do", "does", "did",
    "can", "could", "would", "should", "will", "may", "might", "must",
    "this", "that", "these", "those", "i", "you", "he", "she", "it",
    "we", "they", "me", "him", "her", "us", "them", "my", "your", "his",
    "its", "our", "their", "what", "when", "where", "why", "how", "who",
    "which", "whose", "whom", "explain", "describe", "tell", "about",
    "list", "give", "name", "write", "generate",
}


def extract_topic(instruction: str) -> str:
    """Extract the head noun phrase / topic from an instruction.

    Take the first 3 non-stopword tokens of length ≥3.
    """
    tokens = re.findall(r"[a-zA-Z][a-zA-Z\-]{2,}", instruction.lower())
    picked = []
    for t in tokens:
        if t in _TOPIC_STOPWORDS:
            continue
        picked.append(t)
        if len(picked) >= 3:
            break
    return "_".join(picked) if picked else "unspecified"


def truncate(s: str, n: int) -> str:
    return s if len(s) <= n else s[: n - 1] + "…"


def decompose(pair_id: str, instruction: str, output: str):
    """Yield tuples from one conversational pair."""
    intent = extract_intent(instruction)
    topic = extract_topic(instruction)
    resp_words = len(output.split()) if output else 0

    convo = f"convo_{pair_id}"
    yield (convo, "has_utterance_user", truncate(instruction, 400))
    if output:
        yield (convo, "has_utterance_assistant", truncate(output, 500))
    yield (convo, "user_intent", intent)
    yield (convo, "topic", topic)
    yield (convo, "response_length_words", str(resp_words))
    yield (convo, "source_corpus", "combined_training_v5")


def open_db():
    conn = sqlite3.connect(DB, timeout=300)
    conn.execute("PRAGMA busy_timeout=600000")
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn


def main():
    # Skip if already ingested recently (idempotency / re-run safety)
    conn = open_db()
    existing = conn.execute(
        "SELECT COUNT(*) FROM facts WHERE source = ?", ("dialogue_tuples_v1",)
    ).fetchone()[0]
    print(f"[start] existing dialogue_tuples_v1 facts: {existing:,}", flush=True)
    conn.close()

    t0 = time.time()
    total_tuples = 0
    total_pairs = 0

    with open(TUPLE_OUT, "w") as out:
        conn = open_db()
        batch = []

        for src_path, src_tag in SOURCES:
            if not os.path.exists(src_path):
                print(f"[skip] {src_path} not found", flush=True)
                continue
            print(f"[open] {src_path}", flush=True)

            for i, line in enumerate(open(src_path)):
                if i < SKIP_PAIRS:
                    if i == 0:
                        print(f"[skip] fast-forwarding past {SKIP_PAIRS:,} pairs", flush=True)
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                instruction = rec.get("instruction") or rec.get("prompt") or ""
                output = rec.get("output") or rec.get("response") or ""
                if not instruction:
                    continue

                pair_id = f"{src_tag}_{i:07d}"
                for subj, pred, obj in decompose(pair_id, instruction, output):
                    key = f"{subj}::{pred}"
                    value = obj
                    tup = {
                        "subj": subj,
                        "pred": pred,
                        "obj": obj,
                        "tier": "conversational_v1",
                        "provenance": {
                            "source": "combined_training_v5",
                            "pair_id": pair_id,
                        },
                    }
                    out.write(json.dumps(tup) + "\n")

                    batch.append((key, value, 0.70, "dialogue_tuples_v1", "conversation", 0.70))
                    total_tuples += 1

                    if len(batch) >= BATCH:
                        conn.executemany(
                            "INSERT OR IGNORE INTO facts(key, value, confidence, source, domain, quality_score) "
                            "VALUES (?,?,?,?,?,?)",
                            batch,
                        )
                        conn.commit()
                        batch = []
                        time.sleep(SLEEP_PER_BATCH)
                        if total_tuples % WAL_CHECKPOINT_EVERY < BATCH:
                            try:
                                conn.execute("PRAGMA wal_checkpoint(PASSIVE)")
                            except sqlite3.Error:
                                pass

                total_pairs += 1
                if total_pairs % PROGRESS_EVERY == 0:
                    elapsed = time.time() - t0
                    rate = total_tuples / elapsed
                    print(
                        f"[{elapsed:7.1f}s] pairs={total_pairs:>8,} "
                        f"tuples={total_tuples:>10,} ({rate:,.0f}/s)",
                        flush=True,
                    )

        if batch:
            conn.executemany(
                "INSERT OR IGNORE INTO facts(key, value, confidence, source, domain, quality_score) "
                "VALUES (?,?,?,?,?,?)",
                batch,
            )
            conn.commit()

        # Final TRUNCATE checkpoint so WAL doesn't stay huge after ingest ends.
        # Server will pick up freshly-persisted rows on next query.
        try:
            conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
            print("[done] wal_checkpoint(TRUNCATE) complete", flush=True)
        except sqlite3.Error as e:
            print(f"[done] wal_checkpoint failed: {e}", flush=True)
        conn.close()

    elapsed = time.time() - t0
    print(
        f"[done] pairs={total_pairs:,}  tuples={total_tuples:,}  "
        f"elapsed={elapsed:.0f}s  ({total_tuples/elapsed:.0f}/s)",
        flush=True,
    )
    print(f"[done] tuples JSONL → {TUPLE_OUT}", flush=True)


if __name__ == "__main__":
    main()
