# RAG Retrieval Strategy for 51M+ Facts
**Author:** Claude-1 | **Date:** 2026-04-16 | **Status:** Proposed

## Architecture

```
User Query → Intent Classification → Query Expansion → Multi-Stage Retrieval → Context Assembly → Ollama Prompt
```

## Stage 1: Intent Classification (existing)
The server already classifies intents (Search, Analyze, Learn, etc.). RAG augments Search and Analyze intents.

## Stage 2: Query Expansion
Before searching, expand the user query:
- Extract key terms (nouns, verbs, entities)
- Add domain-specific synonyms (e.g., "hack" → also search "exploit", "vulnerability", "CVE")
- Use the query's detected domain to filter facts

## Stage 3: Multi-Stage Retrieval

### 3a. FTS5 Full-Text Search (primary)
```sql
SELECT f.value, f.domain, f.quality_score, f.source
FROM facts f
JOIN facts_fts ON facts_fts.rowid = f.rowid
WHERE facts_fts MATCH ?
  AND f.quality_score >= 0.75
ORDER BY rank
LIMIT 20
```
- Returns top-20 most relevant facts by BM25 ranking
- Quality floor at 0.75 prevents low-quality results
- FTS5 handles tokenization, stemming, phrase matching

### 3b. Domain-Filtered Retrieval (secondary)
```sql
SELECT value, quality_score FROM facts
WHERE domain = ? AND quality_score >= 0.80
ORDER BY RANDOM() LIMIT 5
```
- If FTS5 returns < 5 results, supplement with random high-quality facts from the detected domain
- Ensures the model always has domain context even for novel queries

### 3c. Adversarial Guard (always)
```sql
SELECT value FROM facts
WHERE source = 'adversarial' AND value LIKE '%' || ? || '%'
LIMIT 3
```
- Check if the query matches known adversarial patterns
- If matched, prepend adversarial context to help the model recognize and reject attacks

## Stage 4: Context Assembly

Build the Ollama prompt:
```
System: You are a helpful AI assistant with access to a knowledge base of 51M+ verified facts.
Use the following relevant facts to inform your answer:

[FACT 1] (domain: cybersecurity, quality: 0.95)
Nmap SYN scan: nmap -sS -p- target — Network reconnaissance...

[FACT 2] (domain: pentesting, quality: 0.90)
Hydra brute force: hydra -l admin -P rockyou.txt ssh://target...

...up to 10 most relevant facts...

User question: {user_query}

Answer thoroughly using the facts above. If the facts don't cover the question, say so.
```

### Context Budget
- Max 10 facts in context (prevents prompt overflow)
- Total context budget: 2000 tokens for facts (~500 words)
- Prioritize by: (1) FTS5 rank, (2) quality_score, (3) domain match

## Stage 5: Quality Feedback Loop
After the model responds:
- Log which facts were used (for training data refinement)
- Track which domains get the most queries (for ingestion priority)
- Flag responses where the model ignores provided facts (quality signal)

## Implementation Plan
1. Add `search_facts_fts()` function to persistence layer (uses FTS5)
2. Add `get_domain_context()` function (domain-filtered random sample)
3. Modify chat handler to call both before Ollama query
4. Build context string with fact metadata
5. Inject into Ollama prompt

## Performance Targets
- FTS5 query: < 50ms for 51M facts (indexed)
- Context assembly: < 10ms
- Total RAG overhead: < 100ms per query
- Quality improvement: measurable via user satisfaction or answer correctness
