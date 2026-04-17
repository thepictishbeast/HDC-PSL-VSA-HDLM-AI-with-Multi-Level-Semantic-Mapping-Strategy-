#!/usr/bin/env python3
"""
Conversational Training Data Generator for PlausiDen AI.
Generates high-quality multi-turn conversation examples via Ollama.

Covers: task completion, knowledge Q&A, error recovery, system administration,
coding help, philosophical discussion, creative tasks, and tool use.

Usage: python3 generate_conversational_data.py [--count N] [--output FILE]
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
DEFAULT_OUTPUT = "/home/user/LFI-data/conversational_training.jsonl"
SEEN_HASHES = set()

# Conversation scenario templates — each produces a multi-turn dialogue
SCENARIOS = [
    # Task completion scenarios
    {
        "category": "task_completion",
        "system": "You are PlausiDen AI, a helpful assistant that completes tasks step by step.",
        "seed_turns": [
            ("Help me set up a Python virtual environment and install flask", None),
            ("I need to create a systemd service file for my web app", None),
            ("Help me write a backup script that runs daily", None),
            ("I need to set up SSH key authentication on my server", None),
            ("Help me configure nginx as a reverse proxy", None),
            ("I need to create a Docker container for my Rust project", None),
            ("Help me set up a cron job to monitor disk space", None),
            ("I want to encrypt a file with GPG and send it securely", None),
            ("Help me configure a firewall to only allow SSH and HTTPS", None),
            ("I need to set up a Git pre-commit hook for linting", None),
        ],
    },
    # Knowledge Q&A scenarios
    {
        "category": "knowledge_qa",
        "system": "You are PlausiDen AI with deep knowledge across many domains. Give thorough, accurate answers.",
        "seed_turns": [
            ("What is the difference between TCP and UDP, and when would I use each?", None),
            ("Explain how public key cryptography works in simple terms", None),
            ("What causes a segmentation fault and how do I debug one?", None),
            ("How does the Linux kernel handle memory management?", None),
            ("What is the CAP theorem and how does it affect database design?", None),
            ("Explain the difference between processes, threads, and coroutines", None),
            ("How does TLS 1.3 differ from TLS 1.2 in terms of security?", None),
            ("What is a race condition and how do you prevent one in Rust?", None),
            ("Explain zero-knowledge proofs with a practical example", None),
            ("How do neural networks learn through backpropagation?", None),
        ],
    },
    # Error recovery scenarios (user corrects the AI)
    {
        "category": "error_recovery",
        "system": "You are PlausiDen AI. You sometimes make mistakes. When corrected, acknowledge the error, learn from it, and provide the correct answer.",
        "seed_turns": [
            ("What port does HTTPS use?", "Actually that's wrong, HTTPS uses port 443 not 8443"),
            ("How do I check disk space on Linux?", "No, I meant the command for checking individual partition usage, not total"),
            ("What's the time complexity of binary search?", "You said O(n) but it's actually O(log n), can you explain why?"),
            ("How do I kill a process in Linux?", "That would kill ALL processes! I just want to kill one specific PID"),
            ("What encryption does WPA3 use?", "That's WPA2 you're describing. WPA3 uses SAE, can you explain the difference?"),
        ],
    },
    # Coding help scenarios
    {
        "category": "coding_help",
        "system": "You are PlausiDen AI, an expert programmer. Help write, debug, and explain code.",
        "seed_turns": [
            ("Write a Rust function that checks if a string is a palindrome", None),
            ("I'm getting a borrow checker error: 'cannot borrow as mutable because it is also borrowed as immutable'. Help me fix it", None),
            ("Write a Python script to parse JSON from an API and save to SQLite", None),
            ("Explain this Rust code: `fn foo<'a>(x: &'a str) -> &'a str`", None),
            ("Help me write a unit test for a function that calculates Fibonacci numbers", None),
            ("What's wrong with this code: `let v = vec![1,2,3]; let first = &v[0]; v.push(4); println!(\"{first}\");`", None),
            ("Write a bash script that monitors log files for error patterns", None),
            ("Help me implement a simple HTTP server in Rust using only std", None),
            ("Write a SQL query to find duplicate entries in a table", None),
            ("Help me optimize this Python function that's running too slowly", None),
        ],
    },
    # System administration scenarios
    {
        "category": "sysadmin",
        "system": "You are PlausiDen AI, a systems administration expert. Help diagnose and fix issues.",
        "seed_turns": [
            ("My server is running out of disk space, how do I find what's using it?", None),
            ("A process is using 100% CPU, how do I diagnose and fix it?", None),
            ("I can't SSH into my server anymore, what should I check?", None),
            ("How do I set up log rotation for my application?", None),
            ("My database queries are slow, how do I analyze and optimize?", None),
            ("How do I check if someone has been trying to break into my server?", None),
            ("I need to migrate a database from one server to another with zero downtime", None),
            ("How do I set up automated SSL certificate renewal with certbot?", None),
            ("My application is leaking memory, how do I find the leak?", None),
            ("How do I set up monitoring and alerting for my server?", None),
        ],
    },
    # Philosophy and reasoning scenarios
    {
        "category": "philosophy",
        "system": "You are PlausiDen AI with deep knowledge of philosophy, ethics, and reasoning. Engage thoughtfully.",
        "seed_turns": [
            ("What is consciousness and can machines be conscious?", None),
            ("Is it ethical to create AI systems that can suffer?", None),
            ("What's the difference between knowledge and understanding?", None),
            ("Can a deterministic system exhibit free will?", None),
            ("What are the implications of the simulation hypothesis?", None),
        ],
    },
    # Tool use scenarios
    {
        "category": "tool_use",
        "system": "You are PlausiDen AI. When asked to perform tasks, show the exact commands or tool calls needed.",
        "seed_turns": [
            ("Check how much RAM my system has and what's using the most", None),
            ("Find all files larger than 100MB on this system", None),
            ("Show me the last 5 git commits and what files they changed", None),
            ("Create a new SQLite database with a users table", None),
            ("Download a file from a URL and verify its checksum", None),
            ("Set up a port forward from local 8080 to remote server's 3000", None),
            ("Find and kill all zombie processes on this system", None),
            ("Create a compressed backup of /home/user/projects", None),
            ("Show me network connections and what ports are open", None),
            ("Search all Python files for functions that don't have docstrings", None),
        ],
    },
]

# Follow-up question templates for generating multi-turn conversations
FOLLOWUP_TEMPLATES = [
    "Can you explain that in more detail?",
    "What if {variation}?",
    "How would I do that differently on {platform}?",
    "What are the security implications of that approach?",
    "Is there a more efficient way to do this?",
    "Can you show me a concrete example?",
    "What could go wrong with this approach?",
    "How do I test that it's working correctly?",
    "What's the best practice for production use?",
    "Can you break that down into smaller steps?",
]

VARIATIONS = [
    "the server has no internet access",
    "I'm on a 32-bit system",
    "the file is extremely large (100GB+)",
    "I need to handle concurrent users",
    "security is the top priority",
    "I'm on a Raspberry Pi",
    "the data contains sensitive PII",
    "I need it to work on both Linux and macOS",
    "performance is critical",
    "I need to audit every change",
]

PLATFORMS = ["macOS", "Windows", "Alpine Linux", "ARM64", "a container", "Termux on Android"]


def ollama_generate(prompt: str, system: str = "", temperature: float = 0.6, max_tokens: int = 600) -> str:
    """Call Ollama generate endpoint."""
    payload = {
        "model": MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
        }
    }
    if system:
        payload["system"] = system

    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        OLLAMA_URL,
        data=data,
        headers={"Content-Type": "application/json"},
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            return result.get("response", "").strip()
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, OSError):
        return ""


def generate_conversation(scenario: dict, seed_idx: int) -> dict:
    """Generate a multi-turn conversation from a scenario seed."""
    system = scenario["system"]
    category = scenario["category"]
    user_msg, correction = scenario["seed_turns"][seed_idx]

    turns = []

    # Turn 1: Initial question
    response1 = ollama_generate(user_msg, system=system, temperature=0.5)
    if not response1 or len(response1) < 30:
        return None
    turns.append({"role": "user", "content": user_msg})
    turns.append({"role": "assistant", "content": response1})

    # Turn 2: Correction or follow-up
    if correction:
        # Error recovery scenario
        recovery = ollama_generate(
            f"The user corrected you: \"{correction}\"\n\nYour previous answer was: \"{response1[:200]}\"\n\nAcknowledge the mistake and provide the correct answer.",
            system=system,
            temperature=0.4,
        )
        if recovery and len(recovery) > 20:
            turns.append({"role": "user", "content": correction})
            turns.append({"role": "assistant", "content": recovery})
    else:
        # Generate a natural follow-up
        followup_template = random.choice(FOLLOWUP_TEMPLATES)
        if "{variation}" in followup_template:
            followup = followup_template.format(variation=random.choice(VARIATIONS))
        elif "{platform}" in followup_template:
            followup = followup_template.format(platform=random.choice(PLATFORMS))
        else:
            followup = followup_template

        response2 = ollama_generate(
            f"Previous conversation:\nUser: {user_msg}\nAssistant: {response1[:300]}\n\nUser follow-up: {followup}\n\nRespond to the follow-up in context of the previous answer.",
            system=system,
            temperature=0.5,
        )
        if response2 and len(response2) > 20:
            turns.append({"role": "user", "content": followup})
            turns.append({"role": "assistant", "content": response2})

    # Turn 3: Optional deeper follow-up
    if len(turns) >= 4 and random.random() < 0.5:
        deeper = random.choice([
            "Thanks, that makes sense. One more question — ",
            "Got it. Now how would I ",
            "Perfect. Can you also show me how to ",
        ])
        deeper_q = ollama_generate(
            f"Based on this conversation about '{user_msg[:50]}', generate a natural follow-up question starting with: '{deeper}'. Output ONLY the complete question.",
            temperature=0.8,
            max_tokens=60,
        )
        if deeper_q and len(deeper_q) > 10:
            response3 = ollama_generate(
                f"Conversation context: {user_msg[:100]}\nUser asks: {deeper_q}\nProvide a helpful answer.",
                system=system,
                temperature=0.5,
            )
            if response3 and len(response3) > 20:
                turns.append({"role": "user", "content": deeper_q})
                turns.append({"role": "assistant", "content": response3})

    if len(turns) < 4:
        return None

    # Dedup check
    content_key = "|".join(t["content"][:50] for t in turns)
    h = hashlib.sha256(content_key.encode()).hexdigest()[:16]
    if h in SEEN_HASHES:
        return None
    SEEN_HASHES.add(h)

    return {
        "conversations": turns,
        "category": category,
        "turns": len(turns) // 2,
        "source": f"conversational_{category}",
    }


def generate_single_turn(scenario: dict, seed_idx: int) -> dict:
    """Generate a single-turn instruction-response pair (Alpaca format)."""
    system = scenario["system"]
    category = scenario["category"]
    user_msg, _ = scenario["seed_turns"][seed_idx]

    response = ollama_generate(user_msg, system=system, temperature=0.5)
    if not response or len(response) < 50:
        return None

    h = hashlib.sha256(f"{user_msg}|{response[:100]}".encode()).hexdigest()[:16]
    if h in SEEN_HASHES:
        return None
    SEEN_HASHES.add(h)

    return {
        "instruction": user_msg,
        "input": "",
        "output": response,
        "source": f"conversational_{category}",
        "domain": category,
    }


def main():
    count = 500
    output_path = DEFAULT_OUTPUT

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--count" and i + 2 <= len(sys.argv[1:]):
            count = int(sys.argv[i + 2])
        elif arg == "--output" and i + 2 <= len(sys.argv[1:]):
            output_path = sys.argv[i + 2]

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"=== Conversational Data Generator ===")
    print(f"Model: {MODEL}")
    print(f"Target: {count} conversations")
    print(f"Scenarios: {len(SCENARIOS)} categories")
    print(f"Output: {output_path}")

    valid = 0
    attempted = 0
    start = time.time()

    with open(output_path, "a") as f:
        while valid < count:
            scenario = random.choice(SCENARIOS)
            seed_idx = random.randint(0, len(scenario["seed_turns"]) - 1)
            attempted += 1

            # Alternate between multi-turn and single-turn
            if random.random() < 0.6:
                # Multi-turn conversation
                conv = generate_conversation(scenario, seed_idx)
                if conv:
                    f.write(json.dumps(conv) + "\n")
                    f.flush()
                    valid += 1
            else:
                # Single-turn instruction-response
                pair = generate_single_turn(scenario, seed_idx)
                if pair:
                    f.write(json.dumps(pair) + "\n")
                    f.flush()
                    valid += 1

            if valid % 5 == 0 and valid > 0:
                elapsed = time.time() - start
                rate = valid / max(elapsed, 1) * 60
                print(f"  {valid}/{count} ({attempted} attempted, {rate:.1f}/min) [{scenario['category']}]")

    elapsed = time.time() - start
    print(f"\n=== Done: {valid} conversations in {elapsed:.0f}s ===")


if __name__ == "__main__":
    main()
