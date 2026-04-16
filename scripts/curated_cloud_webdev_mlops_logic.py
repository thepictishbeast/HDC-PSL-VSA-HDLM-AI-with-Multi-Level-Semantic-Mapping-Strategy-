import sqlite3, hashlib
DB = "/home/user/.local/share/plausiden/brain.db"
def get_conn():
    conn = sqlite3.connect(DB, timeout=300)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA busy_timeout=300000")
    return conn
def mk(p,t): return f"{p}_{hashlib.md5(t.encode()).hexdigest()[:8]}"
def ins(conn, facts, src, dom, q):
    c = conn.cursor()
    n = 0
    for t in facts:
        try:
            c.execute("INSERT OR IGNORE INTO facts (key,value,source,confidence,domain,quality_score) VALUES (?,?,?,?,?,?)", (mk(src,t),t,src,q,dom,q))
            n += c.rowcount
        except: pass
    conn.commit()
    return n

cloud = [
    "AWS core services: EC2 (compute — virtual machines), S3 (object storage — 11 nines durability), RDS (managed databases), Lambda (serverless functions — pay per invocation), VPC (networking — isolated virtual networks), IAM (identity/access), CloudFront (CDN), SQS/SNS (messaging), DynamoDB (NoSQL), ECS/EKS (containers). Start with: EC2 + S3 + RDS + IAM.",
    "Cloud architecture patterns: Multi-AZ (high availability — same region, different datacenters), Multi-region (disaster recovery — geographically separated), Auto-scaling groups (scale compute with demand), Serverless (Lambda + API Gateway + DynamoDB — no servers to manage), Event-driven (SQS/SNS/EventBridge), Microservices (ECS/EKS + service mesh). Well-Architected Framework: operational excellence, security, reliability, performance, cost optimization, sustainability.",
    "Cloud cost optimization: Right-size instances (most are over-provisioned), Reserved Instances / Savings Plans (up to 72% savings for 1-3 year commit), Spot instances (up to 90% savings for fault-tolerant workloads), S3 lifecycle policies (move to cheaper tiers), auto-scaling (match capacity to demand), delete unused resources (EBS volumes, old snapshots, idle load balancers), use managed services (reduce ops overhead).",
]

webdev = [
    "Modern web architecture: SPA (React/Vue/Svelte — client-side routing, API-driven) vs SSR (Next.js/Nuxt — server renders HTML, better SEO/performance) vs SSG (static site generation — fastest, pre-built at deploy time) vs Islands (Astro — static shell with interactive islands). Edge rendering (Cloudflare Workers, Vercel Edge) — run server logic at CDN edge for <50ms TTFB globally.",
    "Web performance optimization: Core Web Vitals (LCP <2.5s largest paint, FID <100ms first input delay, CLS <0.1 layout shift). Techniques: lazy loading (images, routes), code splitting (dynamic imports), image optimization (WebP/AVIF, responsive srcset), caching (Cache-Control headers, service workers), minification, tree shaking, HTTP/2 multiplexing, preload/prefetch hints, CDN for static assets.",
    "Web security checklist: HTTPS everywhere (HSTS header), Content-Security-Policy (prevent XSS), CORS (restrict cross-origin requests), HttpOnly + Secure + SameSite cookies, CSRF tokens, rate limiting, input validation (server-side, never trust client), parameterized queries (prevent SQLi), dependency scanning (npm audit, Snyk), subresource integrity (SRI hashes for CDN scripts).",
    "TypeScript best practices: Strict mode always (strict: true in tsconfig), avoid 'any' (use 'unknown' for truly unknown types), discriminated unions for state management, use const assertions for literal types, prefer interfaces for object shapes (extends vs intersection), generic constraints (T extends Something), utility types (Partial, Required, Pick, Omit, Record), Zod for runtime type validation at API boundaries.",
]

mlops = [
    "MLOps pipeline: Data versioning (DVC) → Feature store (Feast) → Experiment tracking (MLflow, W&B) ��� Model training (distributed: PyTorch DDP, Horovod) → Model registry (MLflow) → Model serving (TorchServe, Triton, BentoML) ��� Monitoring (data drift detection, model performance). CI/CD for ML: test data pipelines, validate model quality gates, canary deployments.",
    "Model deployment patterns: Batch inference (scheduled, process accumulated data), Real-time inference (REST API, <100ms latency), Streaming inference (process events as they arrive — Kafka + model), Edge inference (on-device — ONNX Runtime, TFLite), Serverless (Lambda + SageMaker endpoint). Shadow mode: run new model alongside old one, compare outputs before switching traffic.",
    "LLM deployment: Quantization (reduce precision: FP32 → FP16 → INT8 → INT4 — 2-4x less memory, minor quality loss), KV-cache (avoid recomputing attention for previous tokens), vLLM (PagedAttention for efficient batching), GGUF format (llama.cpp, runs on CPU), speculative decoding (draft model proposes, large model verifies), prompt caching (reuse computed prefixes).",
]

logic_reasoning = [
    "Formal logic: Propositional logic (AND, OR, NOT, IMPLIES, IFF — truth tables), First-order logic (adds quantifiers ∀ ∃ and predicates), Modal logic (adds necessity □ and possibility ◇), Temporal logic (adds always, eventually, until — used in model checking). Inference rules: Modus ponens (P, P→Q ⊢ Q), Modus tollens (¬Q, P→Q ⊢ ¬P), Resolution (for automated theorem proving).",
    "Common logical fallacies: Ad hominem (attack the person not the argument), Straw man (misrepresent then attack), Appeal to authority (expert opinion ≠ proof), False dilemma (only 2 options when more exist), Slippery slope (A will inevitably lead to Z), Circular reasoning (conclusion in premises), Red herring (irrelevant distraction), Post hoc (after therefore because of), Equivocation (same word, different meanings).",
    "Critical thinking framework: 1) Identify the claim, 2) Check the evidence (source quality, sample size, methodology), 3) Consider alternative explanations, 4) Check for logical fallacies, 5) Assess your own biases, 6) Evaluate the strength of the argument (deductive = certain if premises true, inductive = probable, abductive = best explanation). Ask: What would change my mind? If nothing, that's a belief, not a conclusion.",
]

conn = get_conn()
t = 0
t += ins(conn, cloud, "curated_cloud", "technology", 0.95)
t += ins(conn, webdev, "curated_webdev", "technology", 0.95)
t += ins(conn, mlops, "curated_mlops", "technology", 0.95)
t += ins(conn, logic_reasoning, "curated_logic", "reasoning", 0.95)
conn.close()
print(f"Inserted {t} curated facts (cloud, webdev, MLOps, logic/reasoning)")
