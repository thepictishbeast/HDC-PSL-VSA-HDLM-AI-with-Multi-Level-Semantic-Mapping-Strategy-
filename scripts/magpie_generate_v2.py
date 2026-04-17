#!/usr/bin/env python3
"""
Magpie v2 — Domain-focused synthetic data generation for PlausiDen AI training.
Generates high-quality instruction-response pairs via Ollama.

Instead of naive chat-header hallucination, uses topic seeds across 12 domains
to produce diverse, substantive training data.

Usage: python3 magpie_generate_v2.py [--count N] [--output FILE]
"""

import json
import sys
import time
import random
import hashlib
import urllib.request
import urllib.error
from pathlib import Path

OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen2.5-coder:7b"
DEFAULT_OUTPUT = "/home/user/LFI-data/magpie_pairs_v2.jsonl"
DEDUP_HASHES = set()

# Domain topics with seed questions to guide generation
DOMAIN_SEEDS = {
    "cybersecurity": [
        "Explain how buffer overflow exploits work and common mitigations",
        "What is the difference between symmetric and asymmetric encryption",
        "Describe the OWASP Top 10 web application security risks",
        "How does a man-in-the-middle attack work on TLS connections",
        "What are the key principles of zero-trust architecture",
        "Explain SQL injection and parameterized queries",
        "How do hardware security modules protect cryptographic keys",
        "What is the difference between authentication and authorization",
        "Describe common techniques for network intrusion detection",
        "How does certificate pinning prevent MITM attacks",
    ],
    "rust_programming": [
        "Explain Rust's ownership system and how it prevents memory leaks",
        "What is the difference between Box, Rc, and Arc in Rust",
        "How do Rust lifetimes work and when are they needed",
        "Explain the trait system in Rust with examples",
        "What is the difference between String and &str in Rust",
        "How does Rust's async/await model work with tokio",
        "Explain error handling in Rust: Result, Option, and the ? operator",
        "What are Rust macros and when should they be used",
        "How does Rust achieve memory safety without garbage collection",
        "Explain the Send and Sync traits in Rust concurrency",
    ],
    "machine_learning": [
        "What is the difference between supervised and unsupervised learning",
        "Explain how transformer attention mechanisms work",
        "What is gradient descent and how does it optimize neural networks",
        "Describe the bias-variance tradeoff in machine learning",
        "How does LoRA fine-tuning reduce trainable parameters",
        "What is reinforcement learning from human feedback (RLHF)",
        "Explain the difference between precision, recall, and F1 score",
        "How do convolutional neural networks process images",
        "What is transfer learning and why is it effective",
        "Explain the vanishing gradient problem and solutions like ResNets",
    ],
    "distributed_systems": [
        "What is the CAP theorem and its implications for distributed databases",
        "Explain CRDTs and how they enable eventual consistency",
        "How does the Raft consensus algorithm work",
        "What is the difference between optimistic and pessimistic concurrency",
        "Describe the challenges of distributed transaction management",
        "How does consistent hashing work for load distribution",
        "What is a gossip protocol and where is it used",
        "Explain the split-brain problem in distributed systems",
        "How do vector clocks track causality in distributed systems",
        "What is sharding and how does it improve database scalability",
    ],
    "mathematics": [
        "Explain eigenvalues and eigenvectors with geometric intuition",
        "What is the central limit theorem and why is it important",
        "Describe Bayesian inference with a practical example",
        "How does the Fast Fourier Transform work",
        "Explain the concept of entropy in information theory",
        "What is a Markov chain and where are they applied",
        "Describe the difference between L1 and L2 regularization",
        "How does singular value decomposition factor matrices",
        "Explain graph theory basics: vertices, edges, and common algorithms",
        "What is the halting problem and why is it undecidable",
    ],
    "neuroscience": [
        "How do neurons communicate through synaptic transmission",
        "What is neuroplasticity and how does the brain adapt",
        "Explain the role of the hippocampus in memory formation",
        "How does spaced repetition leverage memory consolidation",
        "What is the free energy principle in neuroscience",
        "Describe how attention works in the human brain",
        "What are mirror neurons and what role do they play",
        "How does long-term potentiation strengthen neural connections",
        "Explain the dual-process theory of cognition (System 1 vs System 2)",
        "What is the global workspace theory of consciousness",
    ],
    "systems_programming": [
        "How does virtual memory work with page tables and TLBs",
        "Explain the difference between processes and threads",
        "What is a memory-mapped file and when should you use one",
        "How do operating systems handle interrupt processing",
        "Describe the Linux kernel's scheduling algorithms",
        "What is a futex and how does it enable efficient locking",
        "How does copy-on-write memory management work",
        "Explain the TCP three-way handshake and connection lifecycle",
        "What are cgroups and how do they enable containerization",
        "How does the io_uring interface improve Linux async I/O",
    ],
    "databases": [
        "Explain B-tree indexes and why databases use them",
        "What is write-ahead logging (WAL) in database systems",
        "How does MVCC enable concurrent database access",
        "Describe the differences between row-store and column-store databases",
        "What is FTS5 in SQLite and how does it enable full-text search",
        "How do database query optimizers choose execution plans",
        "Explain database normalization forms (1NF through BCNF)",
        "What is a bloom filter and how does it speed up lookups",
        "How do LSM trees work in databases like RocksDB",
        "Describe the trade-offs between SQL and NoSQL databases",
    ],
    "philosophy_of_mind": [
        "What is the hard problem of consciousness",
        "Explain the Chinese Room argument against strong AI",
        "How does functionalism define mental states",
        "What is the binding problem in consciousness studies",
        "Describe the difference between narrow AI and general intelligence",
        "What are qualia and why are they philosophically significant",
        "How does embodied cognition challenge traditional cognitive science",
        "What is the frame problem in artificial intelligence",
        "Explain Dennett's multiple drafts model of consciousness",
        "What ethical considerations arise from creating sentient AI",
    ],
    "formal_methods": [
        "What is dependent type theory and how does it enable proofs",
        "Explain model checking and temporal logic verification",
        "How does the Curry-Howard correspondence connect proofs and programs",
        "What is abstract interpretation in static analysis",
        "Describe how TLA+ specifies distributed system behavior",
        "What is separation logic and how does it verify memory safety",
        "How do SAT solvers work for constraint satisfaction",
        "Explain property-based testing with QuickCheck/proptest",
        "What is symbolic execution and how does it find bugs",
        "Describe the role of invariants in program verification",
    ],
    "hyperdimensional_computing": [
        "What are hyperdimensional vectors and how do they encode information",
        "Explain the three basic HDC operations: bind, bundle, and permute",
        "How does holographic reduced representation store associations",
        "What is the capacity of a bundled hypervector superposition",
        "How can HDC be used for language classification",
        "Explain the difference between bipolar and binary hypervectors",
        "What is a codebook in hyperdimensional computing",
        "How does random indexing relate to hyperdimensional computing",
        "What advantages does HDC have over deep learning for edge devices",
        "How can tensor networks improve HDC binding precision",
    ],
    "privacy_sovereignty": [
        "What is data sovereignty and why does it matter for AI systems",
        "Explain differential privacy and its epsilon-delta guarantees",
        "How does homomorphic encryption enable computation on encrypted data",
        "What is federated learning and how does it preserve privacy",
        "Describe the concept of a personal knowledge graph",
        "How do zero-knowledge proofs work for identity verification",
        "What is the right to explanation under GDPR for AI decisions",
        "How can decentralized identity systems replace centralized SSO",
        "What are the privacy implications of large language models",
        "Explain secure multi-party computation for collaborative AI",
    ],
}

def ollama_generate(prompt: str, temperature: float = 0.7, max_tokens: int = 500) -> str:
    """Call Ollama generate endpoint."""
    payload = json.dumps({
        "model": MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
        }
    }).encode("utf-8")

    req = urllib.request.Request(
        OLLAMA_URL,
        data=payload,
        headers={"Content-Type": "application/json"},
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data.get("response", "").strip()
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError):
        return ""


def generate_question(domain: str, seed: str) -> str:
    """Generate a novel question inspired by a seed topic."""
    prompt = (
        f"You are an expert in {domain.replace('_', ' ')}. "
        f"Based on this topic: \"{seed}\", generate ONE specific, "
        f"detailed technical question that a student or practitioner would ask. "
        f"Output ONLY the question, nothing else. Make it different from the topic — "
        f"ask about a specific sub-topic, edge case, or practical application."
    )
    return ollama_generate(prompt, temperature=1.0, max_tokens=100)


def generate_answer(question: str, domain: str) -> str:
    """Generate a comprehensive answer to a question."""
    prompt = (
        f"You are a knowledgeable expert in {domain.replace('_', ' ')}. "
        f"Answer the following question thoroughly and accurately. "
        f"Include specific details, examples, and technical depth. "
        f"Be clear and educational.\n\n"
        f"Question: {question}\n\nAnswer:"
    )
    return ollama_generate(prompt, temperature=0.4, max_tokens=600)


def is_quality_pair(instruction: str, output: str) -> bool:
    """Filter out low-quality pairs."""
    # Reject short content
    if len(instruction) < 20 or len(output) < 50:
        return False
    # Reject generic greetings
    low_quality = [
        "how can i assist", "hello!", "how can i help",
        "i'm sorry", "message got cut off", "could you please provide",
        "i'd be happy to help", "feel free to ask",
    ]
    inst_lower = instruction.lower()
    out_lower = output.lower()
    for phrase in low_quality:
        if phrase in inst_lower or (phrase in out_lower and len(output) < 100):
            return False
    # Reject duplicates
    content_hash = hashlib.sha256(f"{instruction}|{output}".encode()).hexdigest()[:16]
    if content_hash in DEDUP_HASHES:
        return False
    DEDUP_HASHES.add(content_hash)
    return True


def main():
    count = 1000
    output_path = DEFAULT_OUTPUT

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--count" and i + 2 <= len(sys.argv[1:]):
            count = int(sys.argv[i + 2])
        elif arg == "--output" and i + 2 <= len(sys.argv[1:]):
            output_path = sys.argv[i + 2]

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"=== Magpie v2 — Domain-Focused Generation ===")
    print(f"Model: {MODEL}")
    print(f"Target: {count} pairs across {len(DOMAIN_SEEDS)} domains")
    print(f"Output: {output_path}")

    # Also generate from seed questions directly (higher quality baseline)
    all_seeds = []
    for domain, seeds in DOMAIN_SEEDS.items():
        for seed in seeds:
            all_seeds.append((domain, seed))
    random.shuffle(all_seeds)

    valid = 0
    attempted = 0
    start = time.time()

    with open(output_path, "a") as f:
        # Phase 1: Direct seed questions (guaranteed quality)
        for domain, seed in all_seeds:
            if valid >= count:
                break

            attempted += 1
            answer = generate_answer(seed, domain)
            if is_quality_pair(seed, answer):
                pair = {
                    "instruction": seed,
                    "input": "",
                    "output": answer,
                    "source": f"magpie_v2_seed_{domain}",
                    "domain": domain,
                }
                f.write(json.dumps(pair) + "\n")
                f.flush()
                valid += 1

            if valid % 5 == 0:
                elapsed = time.time() - start
                rate = valid / max(elapsed, 1) * 60
                print(f"  {valid}/{count} valid ({attempted} attempted, {rate:.1f}/min)")

            # Phase 2: Generated questions (after seeds exhausted)
            if attempted > len(all_seeds) // 2:
                # Generate novel questions from random seeds
                d, s = random.choice(all_seeds)
                question = generate_question(d, s)
                if question and len(question) > 15:
                    answer = generate_answer(question, d)
                    if is_quality_pair(question, answer):
                        pair = {
                            "instruction": question,
                            "input": "",
                            "output": answer,
                            "source": f"magpie_v2_gen_{d}",
                            "domain": d,
                        }
                        f.write(json.dumps(pair) + "\n")
                        f.flush()
                        valid += 1

    elapsed = time.time() - start
    print(f"\n=== Done: {valid} pairs in {elapsed:.0f}s ({output_path}) ===")


if __name__ == "__main__":
    main()
