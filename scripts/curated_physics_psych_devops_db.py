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

physics = [
    "Quantum mechanics fundamentals: Wave-particle duality (everything behaves as both wave and particle), superposition (systems exist in multiple states until measured), entanglement (correlated particles remain connected regardless of distance), uncertainty principle (can't simultaneously know exact position and momentum — Heisenberg). Not about observer consciousness — it's about interaction with measuring apparatus.",
    "General relativity (Einstein, 1915): Gravity isn't a force — it's the curvature of spacetime caused by mass and energy. Predictions confirmed: gravitational lensing, gravitational waves (LIGO 2015), black holes (Event Horizon Telescope 2019), GPS requires relativistic corrections (clocks run faster in weaker gravity). Breaks down at singularities — needs quantum gravity to complete.",
    "Standard Model of particle physics: 6 quarks (up, down, charm, strange, top, bottom), 6 leptons (electron, muon, tau + 3 neutrinos), 4 force carriers (photon = EM, W/Z = weak, gluon = strong), Higgs boson (gives mass via Higgs field, discovered 2012). Does NOT include gravity (graviton hypothetical), dark matter, or dark energy. Covers ~5% of universe's energy content.",
    "Thermodynamics laws: 0th (thermal equilibrium is transitive — defines temperature), 1st (energy is conserved — can't create or destroy, only transform), 2nd (entropy always increases in isolated systems — arrow of time, heat flows hot→cold), 3rd (can't reach absolute zero). Entropy is not 'disorder' — it's the number of microstates consistent with a macrostate. The 2nd law is why perpetual motion machines are impossible.",
    "String theory and beyond: Proposes fundamental entities are 1D strings vibrating at different frequencies (each mode = different particle). Requires 10-11 dimensions (extra ones compactified). Variants: Type I, IIA, IIB, heterotic SO(32), heterotic E8×E8, unified by M-theory (Witten, 1995). No experimental evidence yet. Alternative: Loop Quantum Gravity (quantizes spacetime itself, no extra dimensions needed).",
]

psychology = [
    "Cognitive biases: Confirmation bias (seek info that confirms beliefs), Anchoring (over-relying on first piece of info), Dunning-Kruger (low skill → overestimate ability, high skill → underestimate), Availability heuristic (judge probability by ease of recall), Sunk cost fallacy (continue because of past investment), Survivorship bias (focus on successes, ignore failures). Awareness helps but doesn't eliminate them.",
    "Maslow's hierarchy of needs (1943): Physiological → Safety → Love/Belonging → Esteem → Self-Actualization. Often depicted as pyramid but Maslow never drew one. Criticized for: Western individualist bias, needs aren't strictly hierarchical (people pursue meaning in poverty), lacks empirical validation. Updated models add cognitive needs (curiosity) and transcendence (helping others self-actualize).",
    "Growth mindset (Carol Dweck): Belief that abilities can be developed through effort, strategy, and input from others. Fixed mindset: talent is innate and static. Evidence: students praised for effort outperform those praised for intelligence. Criticisms: effect sizes smaller than initially reported, 'mindset' alone doesn't overcome systemic barriers, can become toxic ('just try harder' dismisses real obstacles).",
    "Behavioral economics (Kahneman & Tversky): System 1 (fast, intuitive, automatic) vs System 2 (slow, deliberate, effortful). Loss aversion (losses feel ~2x stronger than equivalent gains), framing effects (how you present choices changes decisions), prospect theory (people evaluate outcomes relative to reference points, not absolute values). Nudge theory: design choice architecture to promote better decisions without restricting options.",
    "Attachment theory (Bowlby/Ainsworth): Early caregiver bonds shape adult relationship patterns. Secure (comfortable with intimacy and independence), Anxious (crave closeness, fear abandonment), Avoidant (value independence, uncomfortable with closeness), Disorganized (no consistent strategy, often from trauma). ~60% of adults are securely attached. Attachment styles can change with therapy and healthy relationships.",
]

devops = [
    "CI/CD pipeline: Continuous Integration (merge code frequently, automated build + test on every push), Continuous Delivery (code always deployable, manual release trigger), Continuous Deployment (every passing commit deploys automatically). Tools: GitHub Actions, GitLab CI, Jenkins, CircleCI, ArgoCD. Key metrics (DORA): deployment frequency, lead time, change failure rate, MTTR.",
    "Container orchestration (Kubernetes): Pods (smallest unit, 1+ containers), Deployments (declarative pod management, rolling updates), Services (stable networking endpoint), Ingress (HTTP routing), ConfigMaps/Secrets (configuration), PersistentVolumeClaims (storage), HPA (auto-scaling). Key concepts: desired state reconciliation, self-healing, service discovery. Alternatives: Docker Swarm, Nomad, ECS.",
    "Infrastructure as Code (IaC): Define infrastructure in version-controlled files. Terraform (multi-cloud, declarative, state management), Pulumi (real programming languages), CloudFormation (AWS-specific), Ansible (procedural, agentless, SSH-based). Benefits: reproducibility, drift detection, code review for infra changes, disaster recovery. GitOps: infrastructure changes go through Git PRs → auto-applied by controllers.",
    "Site Reliability Engineering (SRE): Google's approach to operations. Key concepts: SLIs (service level indicators — measurable metrics), SLOs (objectives — target thresholds), SLAs (agreements — contractual consequences). Error budgets: if SLO is 99.9% uptime, you get 43.8 min/month of allowed downtime — use it for deployments and experiments. Toil reduction: automate repetitive operational work.",
    "Monitoring and alerting: Monitor the four golden signals: Latency (response time), Traffic (requests/sec), Errors (error rate), Saturation (resource usage %). Page on symptoms, not causes. Reduce alert fatigue: no flapping (use hysteresis), no duplicates, every alert must be actionable. Runbooks for each alert. On-call rotation with proper handoff documentation.",
]

databases = [
    "SQL vs NoSQL: SQL (relational: PostgreSQL, MySQL — ACID transactions, schema enforcement, JOINs, mature tooling). NoSQL types: Document (MongoDB — flexible schema, nested data), Key-Value (Redis — fast cache/sessions), Column-family (Cassandra — write-heavy, distributed), Graph (Neo4j — relationships as first-class). Choose based on data model, query patterns, consistency needs, not hype.",
    "PostgreSQL advanced features: JSONB (document storage + SQL queries), CTEs (WITH clauses for readable complex queries), window functions (RANK, ROW_NUMBER, running totals), full-text search (tsvector/tsquery), partitioning (range, list, hash), LISTEN/NOTIFY (real-time events), extensions (PostGIS for geo, pg_trgm for fuzzy matching, pgvector for embeddings), logical replication, advisory locks.",
    "Database indexing: B-tree (default, good for equality + range queries), Hash (equality only, faster), GIN (generalized inverted — full-text, arrays, JSONB), GiST (generalized search tree — geometry, range types), BRIN (block range — very large sorted tables). Covering indexes (INCLUDE) avoid table lookups. Partial indexes (WHERE clause) reduce size. Too many indexes slow writes — profile with EXPLAIN ANALYZE.",
    "SQLite strengths and limits: Serverless (file-based), zero-config, ACID-compliant, 281 TB max size, ~35% faster than filesystem for small reads. WAL mode enables concurrent reads during writes. Limits: single-writer (no write concurrency), no built-in replication, not ideal for high-concurrency web apps. Perfect for: embedded apps, mobile, desktop, prototyping, read-heavy workloads, edge computing. Litestream for streaming replication.",
]

communication = [
    "Effective technical writing: Lead with the conclusion (inverted pyramid), use concrete examples, avoid jargon unless audience expects it, one idea per paragraph, active voice ('the server processes requests' not 'requests are processed by the server'), use bullet lists for scannable content, include code examples for developers, define acronyms on first use.",
    "Presentation skills: 10-20-30 rule (Guy Kawasaki: 10 slides, 20 minutes, 30pt font). Structure: Problem → Solution → Evidence → Call to action. Start with a hook (story, surprising statistic, question). One idea per slide. Practice out loud. Anticipate questions. Handle nerves: prepare extensively, arrive early, focus on helping the audience not performing for them.",
    "Negotiation frameworks: BATNA (Best Alternative to Negotiated Agreement — your walkaway point), ZOPA (Zone of Possible Agreement — overlap between both sides' acceptable ranges). Fisher & Ury's principled negotiation: separate people from problem, focus on interests not positions, generate options for mutual gain, use objective criteria. Never negotiate against yourself (don't lower your ask before hearing theirs).",
]

conn = get_conn()
total = 0
total += insert_facts(conn, physics, "curated_physics", "science", 0.95)
total += insert_facts(conn, psychology, "curated_psychology", "social_science", 0.95)
total += insert_facts(conn, devops, "curated_devops", "technology", 0.95)
total += insert_facts(conn, databases, "curated_databases", "technology", 0.95)
total += insert_facts(conn, communication, "curated_communication", "business", 0.95)
conn.close()
print(f"Inserted {total} curated facts (physics, psychology, DevOps, databases, communication)")
