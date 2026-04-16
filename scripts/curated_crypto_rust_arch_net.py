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

cryptography = [
    "Symmetric encryption: Same key for encrypt/decrypt. AES-256 (block cipher, current standard, 128-bit blocks), ChaCha20-Poly1305 (stream cipher, fast on mobile/no hardware AES). Modes: GCM (authenticated, parallelizable), CBC (needs separate MAC, IV must be random). Never use ECB (patterns visible in ciphertext).",
    "Asymmetric (public-key) cryptography: Different keys for encrypt/decrypt. RSA (factoring hard problem, 2048+ bit keys), ECDSA/Ed25519 (elliptic curve, smaller keys for same security — Ed25519 preferred), Diffie-Hellman (key exchange, not encryption). Post-quantum threats: Shor's algorithm breaks RSA/ECC. NIST PQC standards: CRYSTALS-Kyber (key exchange), CRYSTALS-Dilithium (signatures).",
    "Hash functions: Deterministic one-way functions. SHA-256 (current standard), SHA-3 (Keccak, different construction), BLAKE3 (very fast, tree-structured). For passwords: bcrypt (adaptive cost), Argon2id (memory-hard, current best — recommended by OWASP), scrypt (memory-hard). Never use MD5 or SHA-1 for security (collision attacks). HMAC = keyed hash for authentication.",
    "TLS 1.3 handshake: 1-RTT (vs 2-RTT in TLS 1.2). Removed: RSA key exchange, CBC mode, SHA-1, custom DH groups, renegotiation, compression. Only supports AEAD ciphers (AES-GCM, ChaCha20-Poly1305). 0-RTT resumption available but replay-vulnerable for non-idempotent requests. Certificate transparency (CT logs) prevents rogue CA certificates.",
    "Zero-knowledge proofs: Prove you know something without revealing it. Properties: completeness (honest prover convinces), soundness (cheater can't convince), zero-knowledge (verifier learns nothing beyond the statement's truth). Applications: ZK-SNARKs/STARKs in blockchain (Zcash, zkSync), password-less authentication, anonymous credentials, verifiable computation.",
    "Key management best practices: Generate with CSPRNG (not Math.random), separate keys by purpose (encryption vs signing vs authentication), rotate regularly, store in HSM/KMS (never in source code), use key derivation functions (HKDF) for deriving multiple keys from one master, implement key escrow carefully, zeroize memory after use (Rust: zeroize crate).",
]

rust_programming = [
    "Rust ownership rules: 1) Each value has exactly one owner, 2) When the owner goes out of scope, the value is dropped, 3) References must always be valid. Borrowing: &T (shared/immutable, multiple allowed) or &mut T (exclusive/mutable, only one at a time). The borrow checker enforces these at compile time — no runtime cost.",
    "Rust error handling: Result<T, E> for recoverable errors, panic! for unrecoverable. The ? operator propagates errors. thiserror crate for deriving Error on custom types. anyhow crate for application-level error handling with context. Never use unwrap() in library code — use expect() with a message or proper error propagation.",
    "Rust async: async fn returns a Future. Futures are lazy — they don't run until polled. tokio is the dominant runtime (multi-threaded, work-stealing). Key patterns: tokio::spawn (spawn task), tokio::select! (first-to-complete), tokio::join! (run concurrently), channels (mpsc, oneshot, broadcast). Pitfall: holding a Mutex guard across .await causes deadlocks — drop before awaiting.",
    "Rust traits and generics: Traits define shared behavior (like interfaces). impl Trait in function params = static dispatch (monomorphized, zero-cost). dyn Trait = dynamic dispatch (vtable, heap allocation). Common traits: Display, Debug, Clone, Copy, Send, Sync, Iterator, From/Into, Deref. Orphan rule: can only impl trait if you own the trait or the type.",
    "Rust memory safety without GC: Stack allocation by default (fast), Box<T> for heap, Arc<T> for shared ownership (atomic reference counting), Rc<T> for single-threaded shared ownership. Interior mutability: Cell<T> (copy), RefCell<T> (runtime borrow checking), Mutex<T>/RwLock<T> (thread-safe). Pin<T> for self-referential structs (needed by async). Zero-cost abstractions: iterators compile to the same code as hand-written loops.",
    "Cargo and Rust tooling: cargo build/test/run/bench/doc/publish. Clippy (linting, catches common mistakes), rustfmt (formatting), miri (undefined behavior detector), cargo-audit (CVE scanning), cargo-deny (license/advisory checking), cargo-geiger (unsafe usage counter), proptest/quickcheck (property-based testing), criterion (benchmarking). rust-analyzer for IDE support.",
]

software_architecture = [
    "SOLID principles: S (Single Responsibility — one reason to change), O (Open/Closed — open for extension, closed for modification), L (Liskov Substitution — subtypes must be substitutable), I (Interface Segregation — many specific interfaces > one general), D (Dependency Inversion — depend on abstractions, not concretions). Guidelines not laws — apply pragmatically.",
    "Microservices vs monolith: Monolith = single deployable unit (simpler, easier debugging, shared state). Microservices = independent services communicating via APIs (independent deployment, technology flexibility, team autonomy, but distributed systems complexity: network failures, data consistency, observability). Start monolith, extract services when you have clear domain boundaries and team scaling needs.",
    "Event-driven architecture: Services communicate through events (published asynchronously) rather than direct API calls. Patterns: Event Sourcing (store events, not state — rebuild by replaying), CQRS (separate read/write models), Saga (distributed transactions as event chains). Tools: Kafka, RabbitMQ, NATS. Benefits: loose coupling, temporal decoupling, audit trail. Costs: eventual consistency, debugging complexity.",
    "Database scaling patterns: Read replicas (multiple read copies), sharding (partition data across databases by key), caching (Redis/Memcached for hot data), connection pooling (PgBouncer), materialized views (precomputed query results), denormalization (sacrifice normalization for read performance), CQRS (separate read/write stores). CAP theorem: choose 2 of Consistency, Availability, Partition tolerance.",
    "API design best practices: RESTful conventions (nouns not verbs, HTTP methods map to CRUD, plural resource names), versioning (URL path /v1/ or header), pagination (cursor-based > offset for large datasets), rate limiting (token bucket), idempotency keys (for safe retries), HATEOAS (hypermedia links), OpenAPI/Swagger documentation. GraphQL alternative for complex nested queries.",
    "Observability stack: Metrics (Prometheus — counters, gauges, histograms), Logs (structured JSON, ELK stack or Loki), Traces (distributed tracing — Jaeger, Zipkin, OpenTelemetry). The three pillars answer different questions: Metrics = what's broken? Logs = why? Traces = where in the request chain? Alert on symptoms (latency, errors, saturation) not causes.",
]

networking = [
    "OSI model layers: 7-Application (HTTP, DNS, SMTP), 6-Presentation (TLS/SSL, encoding), 5-Session (sessions, sockets), 4-Transport (TCP reliable/ordered, UDP fast/unreliable), 3-Network (IP addressing, routing), 2-Data Link (MAC addresses, Ethernet, WiFi), 1-Physical (cables, signals). TCP/IP model simplifies to 4 layers: Application, Transport, Internet, Link.",
    "DNS resolution flow: Browser cache → OS cache → Recursive resolver (ISP or 8.8.8.8/1.1.1.1) → Root nameserver (13 clusters worldwide) → TLD nameserver (.com, .org) → Authoritative nameserver (holds actual records). Record types: A (IPv4), AAAA (IPv6), CNAME (alias), MX (mail), TXT (verification/SPF/DKIM), NS (nameserver), SOA (zone authority). TTL controls caching duration.",
    "TCP three-way handshake: SYN → SYN-ACK → ACK. Connection teardown: FIN → ACK, FIN → ACK (four-way). TCP provides: reliable delivery (retransmission), ordered delivery (sequence numbers), flow control (sliding window), congestion control (slow start, congestion avoidance). UDP: no handshake, no guarantees — used for DNS, video streaming, gaming, VoIP where speed > reliability.",
    "IPv4 vs IPv6: IPv4 = 32-bit addresses (~4.3 billion), exhausted. NAT allows sharing. IPv6 = 128-bit addresses (3.4×10^38), no NAT needed, built-in IPSec, simplified headers. Transition: dual-stack (run both), tunneling (6to4, Teredo), translation (NAT64/DNS64). Private ranges: IPv4 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16. IPv6 link-local: fe80::/10.",
    "Load balancing algorithms: Round robin (simple rotation), Weighted round robin (proportional to capacity), Least connections (route to least busy), IP hash (sticky sessions by client IP), random. Layer 4 (TCP/UDP — fast, no content inspection) vs Layer 7 (HTTP — can route by URL/header/cookie). Tools: Nginx, HAProxy, AWS ALB/NLB, Cloudflare, Envoy. Health checks prevent routing to dead backends.",
]

conn = get_conn()
total = 0
total += insert_facts(conn, cryptography, "curated_crypto", "cybersecurity", 0.95)
total += insert_facts(conn, rust_programming, "curated_rust", "technology", 0.95)
total += insert_facts(conn, software_architecture, "curated_sw_arch", "technology", 0.95)
total += insert_facts(conn, networking, "curated_networking", "technology", 0.95)
conn.close()
print(f"Inserted {total} curated facts (crypto, Rust, software arch, networking)")
