# PlausiDen AI Training Data & Scripts

Training infrastructure for the LFI neurosymbolic cognitive core.

## Structure

- `scripts/` — Ingestion and training scripts
  - `train_adaptive.sh` — SM-2 adaptive domain rotation trainer
  - `knowledge_loop.sh` — Continuous fact gen + self-play cycle
  - `generate_structured_facts.py` — LLM-based structured triple generation
  - `generate_facts_extended.py` — Extended domain fact generation (12 domains)
  - `generate_adversarial_bulk.py` — Bulk adversarial example generation (10 categories)
  - `self_play.py` — Self-play reasoning chain generation
  - `ingest_gsm8k.py` — GSM8K math reasoning chain ingestion
- `training_data.rs` — 800+ curated training examples across 40+ domains
- `adversarial_data.rs` — 50+ adversarial examples (fallacies, injections, contradictions, vuln code)
- `training_state.json` — Per-domain adaptive training progress

## Brain.db Stats (not uploaded — too large, regenerate from scripts)

Current: 20M+ facts, 12 GB+ across 30+ sources including:
Wikipedia (2M), Amazon (3.6M), SNLI (549K), MultiNLI (393K), C4 (streaming 10M+),
OpenWebText (5M), SQuAD (130K), HellaSwag (40K), PIQA (18K), GSM8K (9K),
CodeSearchNet (349K), multilingual NLI (2.4M in 6 languages), news (500K), 
DBpedia (560K), Yahoo Answers (1.4M), and more.

## License

Training scripts: MIT. Source datasets have individual licenses (MIT, CC BY-SA, CC0, etc.).
