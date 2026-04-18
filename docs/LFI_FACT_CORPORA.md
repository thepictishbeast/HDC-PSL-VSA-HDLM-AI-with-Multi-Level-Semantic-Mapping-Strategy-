# LFI Fact Corpora — Structured Ingestion Sources for Neurosymbolic Reasoning

**Framing discipline:** This document lists corpora of structured facts that get decomposed into `(subject, predicate, object, tier, provenance)` tuples, encoded as `BipolarVector` hypervectors via role-binding, and stored in the fact store. **These are not LLM training datasets.** They are factual knowledge sources whose ingestion produces hypervectors that participate in HDC prototype bundling, PSL axiom validation, causal graph construction, and analogical reasoning.

Quality for LFI = "well-formed structured claims that survive PSL axiom validation and bind cleanly into role-filler hypervectors." A badly-written Wikipedia article with accurate structured facts is more valuable than a beautifully-written opinion piece, because the former produces clean tuples and the latter produces claims without clear logical structure.

## Existing ingested corpora (do not re-ingest)

Already in LFI's 50.7M fact store per session context:
- Wikidata, ConceptNet, ARC, GSM8K, OpenOrca, SlimOrca, MMLU-aux, HH-RLHF, WizardLM, SciQ, TruthfulQA, RACE, BoolQ, NQ-Open, MultiRC, C4 realnewslike, Pile-uncopyrighted, ELI5, WikiQA, AquaRAT, Winogrande, OpenBookQA
- 49 curated training domains (pentesting, defensive AI, business/law/econ, crypto/rust/arch/net, etc.)

## Tier 0.95 — Machine-verified structured facts

### Formal mathematics

- **Metamath set.mm** — github.com/metamath/set.mm — CC0. ~48 MB, ~43K theorems. Tuples: `(theorem_name, has_statement, formal_statement)`, `(theorem_name, has_proof, proof_object)`, `(theorem_name, depends_on, [antecedents])`, `(theorem_name, belongs_to, mathematical_domain)`. Tier 0.95 on kernel verification.
- **Mathlib4** — Apache-2.0. 200K+ theorems, growing. `lake build` extracts tuples; Lean kernel verifies each; tier 0.95 automatic.
- **Isabelle AFP** — per-entry BSD/LGPL. ~963 entries, ~314K lemmas, ~5.15M LOC.
- **miniF2F, ProofNet, PutnamBench** — curated NL↔formal paired statements. Dual-encoding: same theorem's NL + formal form bind to same semantic hypervector.

### Formal verification of program correctness

- **SMT-LIB benchmark library** — smt-lib.org, permissive. Hundreds of thousands of verified satisfiability claims.
- **CompCert verified-compiler proofs** — AbsInt source, CC-BY verification artifacts.

### Structured scientific data

- **NIST reference data** — Public domain. Physical constants with uncertainty. `(physical_constant, has_value, V)`, `(physical_constant, has_uncertainty, U)`, `(physical_constant, defined_by, standard_reference)`. Tier 0.95.
- **UniProt** — CC-BY 4.0. Protein sequence + function. RDF/OWL. Millions of entries, tier 0.90-0.95 per evidence code.
- **ChEMBL** — CC-BY-SA 3.0. Bioactive molecule data with assay confidence. Tier 0.85-0.95.

### Cryptographic attestation

- **Sigstore Rekor transparency log** — append-only, Merkle-verifiable. `(artifact_hash, signed_by, at_timestamp)`. Tier 0.95.
- **CVE v5** — CC0. Tier 0.90 from trusted CNAs, 0.85 otherwise.

## Tier 0.85-0.90 — Expert-curated structured sources

### Causal knowledge

- **CauseNet-Precision** — CC-BY 4.0. 199,806 relations, 135 MB. `(cause_concept, causes, effect_concept, evidence_sentence, source)`. Primary corpus for causal.rs CausalGraph.
- **CauseNet-Full** — 11.6M relations, 1.8 GB bz2. Tier sub-sourced: infobox→0.90, Wikipedia-sentence→0.85, ClueWeb→0.75.
- **ATOMIC-2020** — CC-BY. 1.33M social/event commonsense across 23 relations (xIntent, xReact, xEffect, HinderedBy, etc.).
- **GLUCOSE** — CC-BY. ~670K story-causal pairs, 10 dimensions.
- **e-CARE** — CC-BY 4.0. 21K causal reasoning questions with NL chains.

### Biomedical structured knowledge

- **SemMedDB** — UMLS license required. 130.5M semantic predications from 37M PubMed. Predicates: CAUSES, TREATS, PREDISPOSES, AFFECTS, COEXISTS_WITH.
- **DrugBank** — Academic free; commercial license required. ~15K drugs.
- **DisGeNET** — CC-BY-NC-SA academic. Gene-disease associations.

### Legal structured knowledge

- **CaseLaw Access Project** — CC0 most, Harvard license bulk. Millions of US decisions.
- **EU Legislation XML (EUR-Lex)** — reuse permitted. Akoma Ntoso XML.
- **US Code via Cornell LII** — free use. Federal law structured.

### Academic knowledge graphs

- **OpenAlex** — CC0. ~330 GB gz, ~1.6 TB decompressed. 260M+ works, 100M+ authors. Citation graph, author-work-institution-topic relationships.
- **Semantic Scholar Open Research Corpus (S2ORC)** — ODC-BY 1.0, gated. 136M papers, 12M parsed full-text.
- **CORE** — Academic only. 250M+ open-access papers. Dedup against OpenAlex.

### Historical and cultural

- **Getty Vocabularies (AAT, TGN, ULAN, CONA)** — ODC-BY. Art/architecture thesaurus, geographic names, artist biographies.
- **VIAF** — Public domain. Cross-linked authoritative names.
- **DBpedia** — CC-BY-SA 3.0. Tier 0.80 (extraction noise).

### Analogical reasoning corpora

- **AnalogyKB** — with-paper release. Million-scale analogies `(A, B, C, D, relation_type)`. Ideal for HDC: `A ⊗ B⁻¹ ≈ C ⊗ D⁻¹` directly.
- **E-KAR** — academic. 870/119/262 Chinese civil-service exam analogies.
- **BATS** — academic. ~99,280 analogy questions, 40 relation types.
- **I-RAVEN** — 42K debiased Raven's Progressive Matrices.
- **PGM** — DeepMind, 1.42M matrices.

### Calibration and metacognition

- **SelfAware** — CC-BY-SA-4.0. 1,032 unanswerable + 2,337 answerable, ground truth for abstention.
- **HoneSet** — academic. 930 queries across 6 honesty categories.
- **CalibratedMath** — academic. 21 arithmetic tasks with verbalized confidence and correctness labels — feeds Platt calibrator.

## Tier 0.75-0.85 — Broad-coverage semi-structured

### Encyclopedic and reference

- **Wikidata** — re-ingest with delta updates. Truthy dump ~140 GB N-triples, loadable into Oxigraph.
- **Wikipedia (structured extractions)** — CC-BY-SA 4.0. Infoboxes via dbpedia-style pipelines.
- **Wiktionary structured data** — CC-BY-SA 4.0. Lexical facts, etymologies, morphological derivations (feeds HDLM lemma-to-codebook mapping multilingually).

### Common Crawl structured

- **Web Data Commons** — permissive. Extracted RDF + schema.org microdata. Noisy but volume enables statistical filtering; tier 0.70-0.80.

### Security and threat intelligence

- **MITRE ATT&CK STIX** — Apache 2.0. ~800 techniques, ~160 groups, ~700 software. `(group, uses, technique)`, `(technique, achieves, tactic)`, `(software, implements, technique)`. Tier 0.90.
- **CAPEC** — permissive. 559 attack patterns. Tier 0.85.
- **CWE** — permissive. 900+ weaknesses with relationships. Tier 0.90.
- **NVD API 2.0** — public domain. Enriched CVE with CVSS, CPE. Tier 0.85-0.90.
- **Primus-FineWeb** — ODC-BY. 2.57B cyber-filtered tokens. Tier 0.75-0.80.

### Code and software

- **crates.io full dumps** — per-crate licenses (mostly MIT/Apache-2.0). Top 5000 crates. `syn` AST parsing → public API elements as facts. Tier 0.80-0.90.
- **CommitPackFT** — MIT metadata. ~702K commits, 277 langs. `(repo, at_sha, changed_file, old_content, new_content, commit_message)`. Tier 0.80.
- **Software Heritage Archive** — permissive, content-addressed. Tier 0.85.

### Geographic and environmental

- **OpenStreetMap** — ODbL. Nodes/ways/relations with tags.
- **GBIF** — CC0 occurrence records. 2B+ records.

### Multilingual

- **FLORES-200** — CC-BY-SA 4.0. 204 languages × ~2K sentences, professional translations. Gold standard for HDLM multilingual semantic alignment. Tier 0.90.
- **Aya Collection** — Apache-2.0. ~513M instances, 114 languages.
- **Panlex** — CC-BY 4.0. 26M translations, 2500+ varieties. Direct multilingual lemma-to-concept feed.
- **Open Multilingual Wordnet** — permissive.

### Temporal and event

- **EventKG** — CC-BY 4.0. 690K events, 2.3M temporal relations.
- **Wikidata events subset** — CC0. Start/end dates, participants, locations.

### Commonsense

- **Quasimodo** — CC-BY 4.0. 4.4M commonsense statements from query logs. Tier 0.70.
- **WebChild 2.0** — CC-BY-SA 4.0. Noun properties. Tier 0.75.

## Tier 0.60-0.75 — Raw text for extraction

Used via GLiNER, REBEL-large, grammar-constrained parsers. PSL-validated post-extraction.

- **arXiv bulk source** — mixed per-paper, requester-pays S3. ~2.7M papers, ~1.5 TB LaTeX. Equations, definitions, theorem statements via math-NLP. Tier 0.75 structured, 0.65 full-text.
- **PubMed Central OA** — mixed. 5M+ full-text biomedical. Tier 0.75-0.80.
- **bioRxiv, medRxiv** — CC-BY mostly. Preprints. Tier 0.65-0.75.
- **GDELT** — CC-BY 4.0. ~250K events/day.
- **Federal Register** — public domain. Tier 0.90.
- **EDGAR** — public domain. SEC XBRL. Tier 0.90.
- **data.gov etc.** — varies per-dataset.

## Ingestion pipeline architecture

Every corpus flows through:

1. **Download + verify** — SHA-256 against provider-published hashes.
2. **Format-specific parser** — RDF/N-Triples, JSON-LD, Lean olean, Metamath, XML, LaTeX AST.
3. **Fact extraction** — `(subject, predicate, object, tier_hint, provenance_chain)`. Structured: direct mapping. Text: GLiNER + REBEL-large + grammar-constrained local model.
4. **PSL axiom check** — violations tier-demote or reject.
5. **Tier assignment** — combine `tier_hint` + extraction confidence + PSL soft-truth.
6. **Deduplication** — MinHash+LSH per-source (FineWeb config: 112 perms, 14×8 bands, 5-gram word shingles).
7. **Decontamination** — 13-gram Bloom filter against benchmarks; matches → eval-only tier.
8. **HDC encoding** — `fact_hv = bind(R_subj, E_s) + bind(R_pred, E_p) + bind(R_obj, E_o)` with tier-tag binding.
9. **Prototype update** — tier-weighted two-stage voted bundle `P = sign(Σ_t w_t × sign(Σ_{i∈Tier_t} H_i))`.
10. **Audit commit** — TracedDerivation with content hash of source byte range + extracted fact + pipeline version.

## Non-recommendations — skip

- **HLE (Humanity's Last Exam)** — contains canary, license forbids training use. Eval-only or skip.
- **Non-commercial-restricted** that might touch commercial use: ANLI (CC-BY-NC), XNLI (CC-BY-NC), Persona-Hub (CC-BY-NC-SA 4.0), Aya Evaluation non-commercial portions.
- **Global MinHash dedup** — per-source only. FineWeb ablations show global dedup hurts.
- **Scraped/unauthorized datasets** — sovereignty requires clean provenance.
- **Generic web crawl for "fluency"** — doesn't apply. LFI fluency = HDLM Tier-2 rendering, not token distribution coverage.

## Ingestion priority order

### Sprint 1 — Formal verification backbone (1-2 weeks)
Metamath set.mm, Mathlib4, NIST physical constants, Sigstore Rekor snapshot.

### Sprint 2 — Causal and commonsense expansion (2-3 weeks)
CauseNet-Precision → Full, ATOMIC-2020, GLUCOSE, e-CARE, AnalogyKB.

### Sprint 3 — Security and code (2-3 weeks)
MITRE ATT&CK STIX + CAPEC + CWE, CVE v5 full history, top 5000 crates.io source, Primus-FineWeb cyber subset.

### Sprint 4 — Biomedical (2 weeks, license dependent)
UniProt, ChEMBL, SemMedDB (if UMLS secured), DrugBank academic tier.

### Sprint 5 — Academic and legal (3 weeks)
OpenAlex monthly, CaseLaw Access Project, US Code via Cornell LII, Getty Vocabularies.

### Sprint 6 — Multilingual and calibration (2 weeks)
FLORES-200, Aya Collection, Panlex, Open Multilingual Wordnet, SelfAware/HoneSet/CalibratedMath.

### Sprint 7 — Broad-coverage and events (ongoing)
Wikipedia structured extractions, Web Data Commons, EventKG, arXiv selective.

**Total sprint budget:** ~15 weeks for full identified-corpus ingestion.

## Storage and compute budget

- **Ingestion pipeline:** 64 GB RAM recommended, 2 TB scratch per concurrent sprint
- **Fact store after full ingestion:** 150-250M facts, 30-50 GB on disk, 500 MB resident cache
- **Hypervector storage:** 1.25 KB per fact at D=10,000. Full store ~200-300 GB if fully materialized; practice computes on demand from tuple encoding with hot cache
- **Ingestion time:** 10K-100K facts/minute on commodity hardware. Full plan ~40-80 hours of compute, parallelizable
- **Hetzner EX44 (128 GB RAM):** holds full store comfortably
- **Mobile LFI:** top ~10M facts by retrieval frequency, ~2 GB disk, mesh-federated long tail
