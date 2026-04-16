import sqlite3, hashlib

DB = "/home/user/.local/share/plausiden/brain.db"
def get_conn():
    conn = sqlite3.connect(DB, timeout=300)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA busy_timeout=300000")
    return conn

def make_key(prefix, text):
    return f"{prefix}_{hashlib.md5(text.encode()).hexdigest()[:8]}"

def insert_facts(conn, facts, source, domain, quality):
    cur = conn.cursor()
    count = 0
    for text in facts:
        try:
            cur.execute("INSERT OR IGNORE INTO facts (key, value, source, confidence, domain, quality_score) VALUES (?,?,?,?,?,?)",
                (make_key(source, text), text, source, quality, domain, quality))
            count += cur.rowcount
        except: pass
    conn.commit()
    return count

ai_ml = [
    "Machine learning types: Supervised (labeled data — classification, regression), Unsupervised (no labels — clustering, dimensionality reduction, anomaly detection), Semi-supervised (mix), Self-supervised (creates labels from data — BERT, GPT), Reinforcement learning (agent learns from rewards — AlphaGo, RLHF).",
    "Neural network architectures: Feedforward (basic), CNN (convolutional — images, spatial data), RNN/LSTM/GRU (sequential data, time series), Transformer (attention mechanism — NLP revolution), GAN (generative adversarial — image synthesis), VAE (variational autoencoder), Diffusion models (Stable Diffusion, DALL-E).",
    "The Transformer architecture (Vaswani et al., 2017): Self-attention mechanism computes weighted relationships between all positions in a sequence simultaneously. Key components: multi-head attention, positional encoding, layer normalization, feed-forward networks. Enabled GPT, BERT, T5, and all modern LLMs. Scales with compute (scaling laws).",
    "Large Language Model training pipeline: 1) Pre-training (next-token prediction on massive text corpora, billions of tokens), 2) Supervised fine-tuning (SFT on instruction-following examples), 3) Reward modeling (train a model to predict human preferences), 4) RLHF (Reinforcement Learning from Human Feedback — PPO or DPO to align with preferences). Compute cost: GPT-4 estimated $50-100M to train.",
    "Hyperdimensional Computing (HDC): Represents data as high-dimensional vectors (10,000+ dimensions). Core operations: Binding (XOR — creates associations), Bundling (majority vote — creates superpositions), Permutation (cyclic shift — encodes order). Properties: holographic (information distributed across all dimensions), noise-tolerant, efficient, transparent. Used in PlausiDen AI for semantic memory.",
    "AI safety concerns: Alignment problem (ensuring AI goals match human values), mesa-optimization (AI developing inner goals different from training objective), reward hacking, distributional shift, deceptive alignment, power-seeking behavior, existential risk from superintelligence. Current approaches: RLHF, constitutional AI, interpretability research, red-teaming, governance frameworks.",
    "Vector databases and semantic search: Embed text/images into dense vectors using models (OpenAI embeddings, Sentence-BERT, CLIP). Store in vector DBs (Pinecone, Weaviate, Qdrant, Milvus, pgvector). Retrieve by cosine similarity or approximate nearest neighbor (ANN — HNSW, IVF). Powers RAG (Retrieval-Augmented Generation) for grounding LLM responses in factual data.",
    "Prompt engineering techniques: Zero-shot (direct question), Few-shot (examples in prompt), Chain-of-thought (step-by-step reasoning), Tree-of-thought (explore multiple reasoning paths), Self-consistency (sample multiple answers, vote), ReAct (reasoning + action interleaved), RAG (retrieve relevant context), Constitutional AI (self-critique and revision).",
]

politics = [
    "Forms of government: Democracy (direct or representative), Republic (elected representatives, constitutional limits), Monarchy (hereditary head of state — constitutional or absolute), Oligarchy (rule by few — plutocracy, aristocracy, technocracy), Authoritarianism (concentrated power, limited pluralism), Totalitarianism (total state control over all aspects of life).",
    "US government structure: Three branches — Legislative (Congress: Senate 100 + House 435, makes laws), Executive (President, enforces laws, Commander-in-Chief), Judicial (Supreme Court 9 justices, interprets laws). Checks and balances: veto, override (2/3), judicial review (Marbury v. Madison), Senate confirmation, impeachment.",
    "International relations theories: Realism (states are primary actors, power is central, anarchy in international system — Morgenthau, Waltz), Liberalism (institutions, trade, and democracy promote peace — Kant, Keohane), Constructivism (ideas, norms, and identities shape state behavior — Wendt), Critical theory (power structures, inequality, hegemony — Cox, Gramsci).",
    "The United Nations system: General Assembly (193 members, one vote each), Security Council (5 permanent members with veto: US, UK, France, Russia, China + 10 rotating), Secretariat (Secretary-General), ICJ (International Court of Justice), specialized agencies (WHO, UNESCO, UNICEF). Limitations: Security Council veto, no enforcement mechanism, great power politics.",
    "Electoral systems: First-past-the-post (UK, US — simple plurality, tends toward two parties), Proportional representation (seats match vote share — many European countries, more parties), Mixed (Germany — combines both), Ranked choice/instant runoff (eliminates lowest candidate, redistributes votes), Electoral college (US presidential — 538 electors, 270 to win).",
]

entrepreneurship = [
    "The entrepreneurial mindset: Tolerance for ambiguity, bias toward action, customer obsession, rapid iteration, calculated risk-taking, resourcefulness (doing more with less), resilience (most startups fail — learn from failure), pattern recognition, network building, storytelling ability (selling vision to investors/employees/customers).",
    "Bootstrapping strategies: Revenue from day one (consulting/services while building product), pre-sales (validate demand before building), no-code/low-code MVPs, strategic use of free tiers (AWS/GCP credits), revenue-based financing, crowdfunding (Kickstarter), grants (SBIR/STTR for US), keeping burn rate low (remote team, no office).",
    "Go-to-market strategies: Product-led growth (free product drives adoption — Slack, Zoom, Figma), Sales-led (enterprise sales team, longer cycles), Marketing-led (content, SEO, paid ads drive leads), Community-led (build audience before product — Dev.to, Product Hunt), Partner/channel (sell through existing platforms or resellers).",
    "Intellectual property strategy for startups: File provisional patents early ($300, 12-month window), trademark your brand name, use trade secrets for what you can't patent (algorithms, processes), open-source strategically (build community + ecosystem, monetize with enterprise features), watch for freedom-to-operate issues.",
]

conn = get_conn()
total = 0
total += insert_facts(conn, ai_ml, "curated_ai_ml", "technology", 0.95)
total += insert_facts(conn, politics, "curated_politics", "politics", 0.95)
total += insert_facts(conn, entrepreneurship, "curated_entrepreneurship", "business", 0.95)
conn.close()
print(f"Inserted {total} curated facts (AI/ML, politics, entrepreneurship)")
