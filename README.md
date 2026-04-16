# PlausiDen AI Training Data & Scripts

Training infrastructure for the PlausiDen AI neurosymbolic cognitive core.

## Structure

- `scripts/` — Ingestion and training scripts
  - `train_adaptive.sh` — SM-2 adaptive domain rotation trainer
  - `knowledge_loop.sh` — Continuous fact gen + self-play cycle
  - `generate_structured_facts.py` — LLM-based structured triple generation
  - `generate_facts_extended.py` — Extended domain fact generation (12 domains)
  - `generate_adversarial_bulk.py` — Bulk adversarial example generation (10 categories)
  - `self_play.py` — Self-play reasoning chain generation
  - `ingest_gsm8k.py` — GSM8K math reasoning chain ingestion
  - `ingest_specialized_v3.py` — Bulk HuggingFace dataset ingestion (math, science, academic, conversational, code, multi-task)
  - `ingest_arc.py` — ARC Challenge/Easy science QA ingestion
  - `ingest_conceptnet.py` — ConceptNet commonsense knowledge ingestion
- `training_data.rs` — 800+ curated training examples across 40+ domains
- `adversarial_data.rs` — 50+ adversarial examples (fallacies, injections, contradictions, vuln code)
- `training_state.json` — Per-domain adaptive training progress
- `claude1_quality_report.md` — Data quality audit by Claude 1 (40.4M facts)
- `claude1_audit_results.md` — Per-source dedup and distribution analysis

## Brain.db Stats (2026-04-16)

**40.77M+ facts** across 58+ sources in a 24 GB SQLite database.

### Source Distribution (top 15)
| Source | Count | Domain |
|--------|-------|--------|
| C4 (web text) | 15M | web_knowledge |
| OpenWebText | 5M | web_knowledge |
| Amazon Polarity | 3.6M | commerce |
| Wikipedia | 5M | encyclopedic |
| Yahoo Answers | 1.4M | qa_general |
| Yelp Reviews | 650K | commerce |
| DBpedia | 560K | encyclopedic |
| SNLI/MultiNLI | 942K | nli |
| CC News | 500K | news |
| WikiText-103 | 500K | web_knowledge |
| Multilingual NLI (6 lang) | 2.4M | multilingual |
| CodeSearchNet | 369K | code |
| Reasoning (GSM8K, AquaRAT, ARC, etc.) | 180K+ | reasoning |
| Adversarial | 1,010 | adversarial |
| Conversational (OASST, Dolly) | TBD | conversational |

### Quality Metrics
- **Dedup rate**: 0.18% (very clean)
- **Temporal classification**: 6 classes (general, stable, news, code, reasoning, multilingual)
- **Domain sub-classification**: 15+ domains (web_knowledge, commerce, encyclopedic, etc.)
- **Quality scores**: Per-source scoring (adversarial 0.95, academic 0.90, wiki 0.85, web 0.65)
- **Adversarial coverage**: 1,010 facts across 15 categories

## License

Training scripts: MIT. Source datasets have individual licenses (MIT, CC BY-SA, CC0, etc.).
PlausiDen Technologies LLC.
