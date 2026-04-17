# LFI Owner's Guide

**Plain-English setup and usage guide for LFI.**
No jargon, no assumed knowledge — everything you need to run your sovereign AI.

---

## 1. What Is LFI (In Plain English)?

LFI is an AI that **runs on your computer**, not in the cloud. You own it, you control it, no one else can see what you ask it or what it learns.

Think of it as a private AI assistant that:
- Answers questions (like ChatGPT, but on your machine)
- Teaches itself over time (gets smarter without you doing anything)
- Protects you from other AI attacks (like scam emails written by AI)
- Never lies about how confident it is (will say "I'm 70% sure" instead of making stuff up)

## 2. What You Need

### Minimum Hardware
- **Computer:** Linux (Debian, Ubuntu, Kali, etc.) with 16GB RAM
- **Disk:** 20GB free space for models and checkpoints
- **CPU:** Any modern Intel/AMD (Intel i5/i7 from last 5 years is great)
- **GPU (optional):** Nvidia GPU speeds things up but isn't required

### Software Prerequisites
- **Rust** — the language LFI is written in
- **Ollama** (optional) — runs local AI models on your machine
- **Git** — to download LFI

## 3. First-Time Setup (Step by Step)

Open a terminal and run these commands one at a time:

### Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify it worked:
```bash
rustc --version
# Should print something like: rustc 1.75.0 (2024-01-01)
```

### Step 2: Install Ollama (optional but recommended)

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

Download a model that works well on laptops:
```bash
ollama pull qwen2.5-coder:7b
# This is ~4.5GB and takes 5-10 minutes on a reasonable connection
```

Start Ollama in the background:
```bash
ollama serve &
```

Verify it's running:
```bash
curl http://localhost:11434/api/tags
# Should return JSON with the model you just pulled
```

### Step 3: Download LFI

```bash
git clone https://github.com/thepictishbeast/PlausiDen-AI.git
cd PlausiDen-AI
```

### Step 4: Build LFI

```bash
cd lfi_vsa_core
cargo build --release
```

This takes 2-5 minutes the first time. You'll see lots of compiler output — that's normal.

### Step 5: Verify Everything Works

```bash
cargo test --release
```

You should see: `test result: ok. 704 passed; 0 failed;`

If you see failures, something is wrong with your setup — open an issue at the repo.

## 4. Using LFI

### Option A: Run a Training Session (Uses Ollama)

This teaches LFI by asking a local AI model questions and grading the answers.

```bash
# Small test run (50 examples, ~10 minutes)
cargo run --release --bin ollama_train -- --examples 50

# Bigger run (200 examples, ~40 minutes)
cargo run --release --bin ollama_train -- --examples 200

# Focus on a specific topic
cargo run --release --bin ollama_train -- --examples 30 --domain security
```

**What you'll see:**
```
[1/50] ✓  8757ms | math → 5 (5)
[2/50] ✓  7826ms | math → 56 (56)
```

Each line is one question. The `✓` means LFI got it right, `✗` means wrong. The number in milliseconds is how long the LLM took to answer.

### Option B: Run the Continuous Daemon (Always-On Self-Improvement)

The daemon runs LFI in the background, continuously improving itself. It cycles between different tasks (training, self-reflection, checkpointing).

```bash
# Starts daemon with mock mode (no real AI needed)
./lfi_daemon.sh
```

To stop it: `Ctrl+C` or `killall lfi_daemon`.

### Option C: Explore Via Code

If you're comfortable with Rust, you can use LFI's modules directly. See `lfi_vsa_core/src/intelligence/` for all the modules. Each has inline documentation and tests showing how to use it.

## 5. What's In LFI — The Parts That Matter

Think of LFI as a team of specialists working together:

| Part | What It Does | Plain English |
|---|---|---|
| HDC Core | Math engine for concepts | Stores ideas as 10,000-number patterns |
| PSL Supervisor | Rule enforcer | Checks every answer against sanity rules |
| Knowledge Engine | Memory | Stores what LFI has learned |
| Self-Improvement | Meta-learning | LFI grades itself and plans how to get better |
| Epistemic Filter | Skepticism | Doesn't believe everything; requires proof |
| Defensive AI | Bodyguard | Detects AI-based attacks coming at you |
| Math Engine | Calculator with proofs | Shows work, verifies each step |
| Code Evaluator | Code reviewer | Compiles and tests code it writes |
| Cross-Domain | Connection-maker | Applies lessons from one topic to another |
| Reasoning Provenance | Honest-explanation guard | Distinguishes "I traced this" from "I'm guessing why" |
| Spaced Repetition | Review scheduler | Decides which concepts to rehearse next, SM-2 style |
| Daemon | Scheduler | Runs everything on a loop forever |

### How Reasoning Provenance Works (in plain English)

When LFI answers a question, it remembers the exact reasoning steps it took
— what rules it applied, what evidence it weighed, how confident it was at
each step. This is the **derivation trace**.

If you later ask "how did you get that answer?", LFI looks up the trace.
There are two possible outcomes:

- **Traced** — the trace exists, so LFI walks you through the actual steps.
- **Reconstructed** — the trace was never recorded (or has been cleared),
  so LFI **explicitly tells you** "I'm reconstructing this after the fact;
  it may not match my real reasoning."

This is the whole point: LFI literally cannot lie about whether its
explanation is real or made up. It's enforced at the data-structure level.

You can interact with the provenance system from the HTTP API:

```bash
# Ask LFI a question; record the reasoning
curl -X POST http://localhost:8080/api/think \
  -H 'content-type: application/json' \
  -d '{"input": "what is sovereignty"}'
# → { "answer": "...", "confidence": 0.87, "conclusion_id": 12345678 }

# Ask LFI to explain its reasoning
curl http://localhost:8080/api/provenance/12345678
# → { "kind": { "kind": "TracedDerivation" },
#     "explanation": "Step 0 [System1FastPath conf=0.87] ...",
#     "confidence_chain": [0.87], "depth": 0 }

# Snapshot the whole reasoning history (after authentication)
curl http://localhost:8080/api/provenance/export
# → { "trace_count": 42, "arena": { ... }}
```

## 6. What LFI Won't Do

- **Promise 100% certainty about anything.** Even "2+2=4" gets 99.99% confidence at most. This is by design.
- **Call home.** LFI has no network telemetry. Nothing leaves your machine unless you explicitly tell it to.
- **Pretend to remember things it didn't actually reason through.** Ask LFI "why" and it will tell you whether it has a real derivation or just a guess. (See the Reasoning Provenance section above — this is enforced structurally, not as policy.)
- **Trust any single source.** Information from one source gets low confidence until corroborated.
- **Run admin commands without authentication.** Endpoints like `/api/provenance/export`, `/api/provenance/reset`, and `/api/provenance/compact` reject unauthenticated requests, so an attacker who can reach the API still can't dump or wipe your reasoning history.

## 7. Keeping LFI Running (Maintenance)

### Daily
Nothing needed. LFI is stateful — it remembers what it learned.

### Weekly
```bash
cd PlausiDen-AI
git pull        # Get the latest improvements
cd lfi_vsa_core
cargo test      # Make sure everything still works
```

### When You Want More Intelligence
```bash
cargo run --release --bin ollama_train -- --examples 200
# Run this overnight for best results
```

### If Something Breaks
```bash
# Clean rebuild
cd lfi_vsa_core
cargo clean
cargo build --release
cargo test
```

## 8. Common Questions

**Q: Is my data safe?**
A: Everything runs on your machine. LFI has `#![forbid(unsafe_code)]` at the root — meaning it literally cannot make raw memory operations. No network calls unless you start Ollama (which also runs locally).

**Q: Can I use it without Ollama?**
A: Yes. LFI has a "mock mode" for testing. It won't learn new things from LLM answers, but all the infrastructure works. You can also use Gemini CLI or Claude CLI as backends.

**Q: How do I know it's actually working?**
A: Run `cargo test --release`. 704 tests should pass. Also check checkpoints in `/tmp/lfi_ollama_training/` after training runs — those are proof of work.

**Q: Why is LFI so cautious?**
A: By design. An AI that says "I'm 100% sure" about anything is either lying or wrong. LFI will never claim 100%. This is what makes it trustworthy.

**Q: What's the difference between this and ChatGPT?**
A: ChatGPT is a cloud service that's a black box. LFI runs locally, shows its work, never claims 100% certainty, and combines multiple reasoning approaches (not just prediction). They're different tools for different purposes.

**Q: How do I know if LFI is just memorizing vs actually understanding?**
A: The `generalization.rs` module tests this. Concepts LFI has memorized (but not understood) get flagged as `RoteMemorization` and scheduled for re-training with variations.

**Q: Can LFI be trained to do my specific job?**
A: Yes — add training examples to `lfi_vsa_core/src/intelligence/training_data.rs` in a new domain, then run training. Each new domain takes about 20 examples to establish basic competence.

**Q: What happens if Ollama crashes during training?**
A: LFI handles it gracefully. The current question gets marked as an error, training continues with the next example. Checkpoint is saved before and after training.

**Q: Where does LFI store what it knows?**
A: In memory during a run, and optionally in checkpoints saved to `/tmp/lfi_ollama_training/` or `/root/.lfi/checkpoints/`. Checkpoints are JSON files with integrity hashes — can be moved between machines.

**Q: How do I back up everything?**
A: `tar -czf lfi_backup.tar.gz /root/.lfi/ /tmp/lfi_ollama_training/`

## 9. Troubleshooting

### "cargo: command not found"
Rust isn't installed. See Step 1 above.

### "cannot find crate `lfi_vsa_core`"
You're in the wrong directory. `cd` into `PlausiDen-AI/lfi_vsa_core`.

### "connection refused: localhost:11434"
Ollama isn't running. Start it: `ollama serve &`

### "model not found"
The model you specified isn't downloaded. Pull it: `ollama pull qwen2.5-coder:7b`

### Training is slow
Normal. Each query takes 3-15 seconds. 50 examples = ~10 min. That's expected on a laptop. Using `--release` mode makes LFI itself faster (Ollama is the bottleneck).

### RAM is getting high during training
Ollama loads the model into RAM. `qwen2.5-coder:7b` uses ~5GB. If you have 16GB, this is fine. If less, use a smaller model or close other programs.

### Tests fail after `git pull`
Something broke. Run `cargo clean && cargo build --release && cargo test` to rebuild from scratch.

## 10. Getting Help

- **Issues:** https://github.com/thepictishbeast/PlausiDen-AI/issues
- **Maintainer:** Paul (PlausiDen Technologies)

## 11. Philosophy

LFI exists because **you deserve an AI that answers only to you**. The big cloud AIs are controlled by corporations. LFI runs on your machine, learns what you teach it, and tells you the truth about what it knows and doesn't know.

It's not trying to be ChatGPT. It's trying to be what ChatGPT can never be: **a sovereign intelligence that serves its owner**.

---

**Questions? Improvements? Open an issue. This guide will be updated as LFI evolves.**
