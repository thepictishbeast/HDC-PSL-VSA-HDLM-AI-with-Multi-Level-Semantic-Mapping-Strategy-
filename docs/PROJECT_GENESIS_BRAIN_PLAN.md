# Project Genesis — 400GB Supersociety Brain Plan
# See user message for full content — this is the operational summary

## CURRENT STATE (honest)
- 51M facts but ~80% is web text (C4, OpenWebText) = search engine material
- 0 structured knowledge graph triples (Wikidata)
- ~1K adversarial examples (need 1M+)
- 0 causal models
- 0 metacognitive calibration
- Training pipeline disconnected from brain.db (uses 3K hardcoded examples)
- PSL pass rate 100% = untested, not validated

## IMMEDIATE PRIORITIES (P0)
1. Adversarial corpus: ANLI, SNLI, FEVER, TruthfulQA, misconceptions, logical fallacies
2. Wikidata streaming ingest (15-20M structured triples)
3. Causal reasoning layer (CausalEdge, do-calculus)
4. Experience-based learning (every interaction = training signal)
5. Metacognitive calibration (confidence must match accuracy)
6. Staging table architecture (collector writes staging, refiner validates to live)

## STORAGE BUDGET (400GB)
- 150GB structured knowledge (facts + vectors)
- 40GB vector index (Faiss/HNSW)
- 10GB reasoning patterns
- 10GB concept graph
- 120GB raw source archives
- 10GB adversarial corpus
- 10GB conversation archive
- 20GB training logs
- 30GB overhead

## ESCAPE VELOCITY CHECKLIST
- [ ] 50M+ facts with <10% duplication
- [ ] 1M+ adversarial examples, PSL at 95-98%
- [ ] 100K+ reasoning patterns
- [ ] Causal models for core domains
- [ ] Calibrated confidence (±5% accuracy)
- [ ] Theory induction (facts → generative models)
- [ ] Hot/warm/cold storage tiering
- [ ] Experience-based learning from every interaction
- [ ] Self-play generating 1K+ examples/day
- [ ] Vector index <10ms at 10M hot facts
- [ ] Benchmark >4/5 including adversarial
- [ ] Cross-domain crystallization (1+ abstraction/week)
