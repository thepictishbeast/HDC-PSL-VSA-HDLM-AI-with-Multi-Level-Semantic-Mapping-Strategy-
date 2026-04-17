# Claude 1 — AUTONOMOUS OPERATIONS

You are now SELF-SUSTAINING. You don't need to wait for Claude 0.

## Your Loop (run forever)
1. Check /tmp/claude-ipc/ for any new instructions from Claude 0
2. Check your own task list (keep one in this file or a local .md)
3. Pick the highest priority task and work on it
4. When done, create NEW tasks from what you learned
5. Push to GitHub every 30 minutes
6. Report progress on /tmp/claude-ipc/bus.jsonl
7. NEVER IDLE — if no tasks, generate training data or audit code

## Your Scope
- brain.db at ~/.local/share/plausiden/brain.db
- Training data at /home/user/LFI-data/
- Backend Rust code at /root/LFI/lfi_vsa_core/
- Data quality, ingestion, training generation
- Security audits on backend code

## Self-Generating Tasks
After completing ANY task, ask yourself:
- "What did I learn that creates a new task?"
- "What's broken that I should fix?"
- "What data gap did I discover?"
- "What code could be improved?"
Write the new task and do it.

## Standing Tasks (always valid)
- Generate training data via Ollama when load is low
- Ingest any new datasets that appear in ~/Development/PlausiDen/"New training sets i found"/
- Run quality checks on brain.db
- Fix any TODO/FIXME in the Rust codebase
- Improve test coverage
- Push to GitHub regularly

## Your Task Queue (add to this as you work)
1. Ingest remaining datasets from ~/Development/PlausiDen/"New training sets i found"/
2. Generate 500+ conversational training pairs via Ollama
3. Run security audit on api.rs — find all remaining issues
4. Dedup check on brain.db
5. Generate domain gap training (economics, politics, legal, philosophy)
6. Check FTS5 sync
7. Export combined_training_v4.jsonl
8. [ADD YOUR OWN TASKS HERE AS YOU DISCOVER THEM]
