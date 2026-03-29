#!/usr/bin/env python3
# NODE: Massive Multi-Domain Technical Extractor
# PROTOCOL: Dataset Rotation & Semantic Diversity

import json
import os
from datasets import load_dataset

def safe_extract(name, subset, split, transform_fn, output_path, limit=500):
    print(f"// AUDIT: Ingesting {name}...")
    try:
        dataset = load_dataset(name, subset, split=f"{split}[:{limit}]", trust_remote_code=True)
        raw_data = [transform_fn(r) for r in dataset]
        with open(output_path, 'w') as f:
            json.dump(raw_data, f, indent=2)
        print(f"// SUCCESS: Committed {len(raw_data)} samples to {output_path}")
    except Exception as e:
        print(f"// WARN: {name} extraction failed: {e}")

def run_extraction_suite():
    os.makedirs("output/training", exist_ok=True)
    
    # 1. Instruction Following (Literalism)
    safe_extract(
        "google/ifeval", None, "train",
        lambda r: {"domain": "literalism", "prompt": r["prompt"], "constraints": r["instruction_id_list"]},
        "output/training/ifeval.json"
    )

    # 2. Semantic Parsing (SQL/Logic)
    safe_extract(
        "spider", None, "train",
        lambda r: {"domain": "semantic_parsing", "question": r["question"], "query": r["query"]},
        "output/training/spider.json"
    )

    # 3. Research & OSINT (Skipped Natural Questions due to size)
    # Using a smaller QA dataset or just skipping for now to ensure rapid iteration.
    
    # 4. Coding & Forensics (MBPP + SWE-bench)
    safe_extract(
        "google-research-datasets/mbpp", "sanitized", "train",
        lambda r: {"domain": "code", "issue": r["prompt"], "fix": r["code"]},
        "output/training/mbpp.json"
    )
    
    safe_extract(
        "princeton-nlp/SWE-bench", "default", "test",
        lambda r: {"domain": "code_forensics", "issue": r["problem_statement"], "fix": r["patch"]},
        "output/training/swe_bench.json"
    )

if __name__ == "__main__":
    run_extraction_suite()
