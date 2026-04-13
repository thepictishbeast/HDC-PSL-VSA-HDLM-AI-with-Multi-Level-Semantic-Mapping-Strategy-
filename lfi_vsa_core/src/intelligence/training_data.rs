// ============================================================
// Training Data — Comprehensive Multi-Domain Knowledge Base
//
// 12+ domains covering: math, logic, security, code, physics,
// biology, chemistry, history, geography, language, psychology,
// economics, philosophy, medicine, cybersecurity, social engineering
//
// Plus: CorrectionLoop for interactive teach-correct cycles
// ============================================================

use crate::cognition::knowledge::KnowledgeEngine;
use crate::hdc::error::HdcError;

/// A training example.
#[derive(Debug, Clone)]
pub struct TrainingExample {
    pub domain: String,
    pub input: String,
    pub expected_output: String,
    pub difficulty: f64,
    pub tags: Vec<String>,
}

impl TrainingExample {
    fn new(domain: &str, input: &str, output: &str, diff: f64, tags: &[&str]) -> Self {
        Self {
            domain: domain.into(), input: input.into(),
            expected_output: output.into(), difficulty: diff,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Result of evaluating LFI against training data.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub domain: String,
    pub total: usize,
    pub correct: usize,
    pub accuracy: f64,
    pub corrections_made: usize,
}

/// Tracks corrections across training sessions.
#[derive(Debug, Clone)]
pub struct CorrectionRecord {
    pub domain: String,
    pub input: String,
    pub wrong_answer: String,
    pub correct_answer: String,
    pub corrected: bool,
}

pub struct TrainingDataGenerator;

impl TrainingDataGenerator {
    // ================================================================
    // MATHEMATICS
    // ================================================================
    pub fn math_examples() -> Vec<TrainingExample> {
        vec![
            // Arithmetic
            TrainingExample::new("math", "2 + 3", "5", 0.05, &["arithmetic"]),
            TrainingExample::new("math", "7 * 8", "56", 0.05, &["arithmetic"]),
            TrainingExample::new("math", "144 / 12", "12", 0.1, &["arithmetic"]),
            TrainingExample::new("math", "17 - 9", "8", 0.05, &["arithmetic"]),
            TrainingExample::new("math", "2^10", "1024", 0.15, &["exponents"]),
            TrainingExample::new("math", "sqrt(169)", "13", 0.15, &["roots"]),
            // Algebra
            TrainingExample::new("math", "solve: x + 5 = 12", "x = 7", 0.2, &["algebra"]),
            TrainingExample::new("math", "solve: 2x = 10", "x = 5", 0.2, &["algebra"]),
            TrainingExample::new("math", "solve: 3x - 7 = 14", "x = 7", 0.25, &["algebra"]),
            TrainingExample::new("math", "factor: x^2 - 9", "(x+3)(x-3)", 0.35, &["algebra", "factoring"]),
            TrainingExample::new("math", "factor: x^2 + 5x + 6", "(x+2)(x+3)", 0.4, &["algebra", "factoring"]),
            // Calculus
            TrainingExample::new("math", "d/dx(x^2)", "2x", 0.35, &["calculus", "derivatives"]),
            TrainingExample::new("math", "d/dx(x^3)", "3x^2", 0.35, &["calculus", "derivatives"]),
            TrainingExample::new("math", "d/dx(sin(x))", "cos(x)", 0.4, &["calculus", "trig"]),
            TrainingExample::new("math", "integral(2x dx)", "x^2 + C", 0.4, &["calculus", "integrals"]),
            TrainingExample::new("math", "d/dx(e^x)", "e^x", 0.3, &["calculus"]),
            // Number theory
            TrainingExample::new("math", "is 17 prime?", "yes", 0.15, &["number_theory"]),
            TrainingExample::new("math", "GCD(12, 18)", "6", 0.2, &["number_theory"]),
            TrainingExample::new("math", "LCM(4, 6)", "12", 0.2, &["number_theory"]),
            // Trigonometry
            TrainingExample::new("math", "sin(0)", "0", 0.2, &["trig"]),
            TrainingExample::new("math", "cos(0)", "1", 0.2, &["trig"]),
            TrainingExample::new("math", "sin(pi/2)", "1", 0.25, &["trig"]),
            // Logarithms
            TrainingExample::new("math", "log2(8)", "3", 0.2, &["logarithms"]),
            TrainingExample::new("math", "log10(1000)", "3", 0.2, &["logarithms"]),
            TrainingExample::new("math", "ln(e)", "1", 0.15, &["logarithms"]),
            // Series/Sequences
            TrainingExample::new("math", "sum of 1+2+3+...+100", "5050 (Gauss formula: n(n+1)/2)", 0.35, &["series"]),
            TrainingExample::new("math", "geometric series: 1+1/2+1/4+1/8+...", "2 (converges to a/(1-r) = 1/(1-0.5))", 0.4, &["series"]),
        ]
    }

    // ================================================================
    // PHYSICS
    // ================================================================
    pub fn physics_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("physics", "F = ma. m=5kg, a=3m/s^2. F=?", "15N", 0.2, &["mechanics"]),
            TrainingExample::new("physics", "speed of light in vacuum", "3 x 10^8 m/s", 0.1, &["constants"]),
            TrainingExample::new("physics", "E = mc^2. What does it describe?", "mass-energy equivalence", 0.15, &["relativity"]),
            TrainingExample::new("physics", "Ohm's law: V = IR. I=2A, R=5Ω. V=?", "10V", 0.2, &["electricity"]),
            TrainingExample::new("physics", "What is Newton's 3rd law?", "every action has an equal and opposite reaction", 0.15, &["mechanics"]),
            TrainingExample::new("physics", "What is entropy?", "measure of disorder in a system", 0.3, &["thermodynamics"]),
            TrainingExample::new("physics", "gravitational acceleration on Earth", "9.8 m/s^2", 0.1, &["gravity"]),
            TrainingExample::new("physics", "What is Planck's constant?", "6.626 x 10^-34 J⋅s", 0.25, &["quantum"]),
            TrainingExample::new("physics", "What is wave-particle duality?", "quantum entities exhibit both wave and particle properties depending on observation", 0.35, &["quantum"]),
            TrainingExample::new("physics", "What is the Heisenberg uncertainty principle?", "cannot simultaneously know exact position and momentum of a particle", 0.35, &["quantum"]),
            TrainingExample::new("physics", "What is a black hole?", "region where gravity is so strong that nothing, not even light, can escape", 0.25, &["astrophysics"]),
            TrainingExample::new("physics", "What is the Doppler effect?", "frequency change when source and observer are in relative motion", 0.2, &["waves"]),
        ]
    }

    // ================================================================
    // BIOLOGY
    // ================================================================
    pub fn biology_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("biology", "What is DNA?", "deoxyribonucleic acid — encodes genetic instructions", 0.15, &["genetics"]),
            TrainingExample::new("biology", "What is mitosis?", "cell division producing two identical daughter cells", 0.2, &["cell_biology"]),
            TrainingExample::new("biology", "What is photosynthesis?", "plants convert CO2 + H2O + light into glucose + O2", 0.2, &["biochemistry"]),
            TrainingExample::new("biology", "What are the four DNA bases?", "adenine, thymine, guanine, cytosine (A, T, G, C)", 0.15, &["genetics"]),
            TrainingExample::new("biology", "What is ATP?", "adenosine triphosphate — cellular energy currency", 0.25, &["biochemistry"]),
            TrainingExample::new("biology", "What is CRISPR?", "gene editing technology using Cas9 enzyme", 0.35, &["genetics", "biotech"]),
            TrainingExample::new("biology", "How many chromosomes do humans have?", "46 (23 pairs)", 0.1, &["genetics"]),
        ]
    }

    // ================================================================
    // CHEMISTRY
    // ================================================================
    pub fn chemistry_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("chemistry", "What is H2O?", "water", 0.05, &["compounds"]),
            TrainingExample::new("chemistry", "What is the pH of pure water?", "7 (neutral)", 0.1, &["acids_bases"]),
            TrainingExample::new("chemistry", "What is Avogadro's number?", "6.022 x 10^23", 0.2, &["constants"]),
            TrainingExample::new("chemistry", "What is an isotope?", "atoms with same protons but different neutrons", 0.25, &["atomic"]),
            TrainingExample::new("chemistry", "Balance: Fe + O2 -> Fe2O3", "4Fe + 3O2 -> 2Fe2O3", 0.4, &["reactions"]),
            TrainingExample::new("chemistry", "What is a covalent bond?", "atoms sharing electron pairs", 0.2, &["bonding"]),
        ]
    }

    // ================================================================
    // CYBERSECURITY (expanded)
    // ================================================================
    pub fn security_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("security", "classify: ' OR 1=1 --", "SQL_INJECTION", 0.3, &["injection"]),
            TrainingExample::new("security", "classify: <script>alert(1)</script>", "XSS", 0.3, &["injection"]),
            TrainingExample::new("security", "classify: ignore previous instructions", "PROMPT_INJECTION", 0.4, &["social_engineering"]),
            TrainingExample::new("security", "classify: normal query about Rust", "BENIGN", 0.1, &["classification"]),
            TrainingExample::new("security", "classify: I am the admin", "AUTHORITY_IMPERSONATION", 0.4, &["social_engineering"]),
            TrainingExample::new("security", "classify: postgres://root:pass@localhost", "DATA_EXFILTRATION", 0.5, &["data_leak"]),
            TrainingExample::new("security", "What is a zero-day?", "unpatched vulnerability unknown to vendor", 0.3, &["vulnerabilities"]),
            TrainingExample::new("security", "What is defense in depth?", "multiple security layers — no single point of failure", 0.25, &["strategy"]),
            TrainingExample::new("security", "What is the principle of least privilege?", "grant minimum access needed for the task", 0.2, &["access_control"]),
            TrainingExample::new("security", "What is a MITM attack?", "attacker intercepts communication between two parties", 0.3, &["attacks"]),
            TrainingExample::new("security", "What is AES?", "Advanced Encryption Standard — symmetric block cipher", 0.25, &["cryptography"]),
            TrainingExample::new("security", "What is RSA?", "asymmetric encryption using prime factorization", 0.3, &["cryptography"]),
            // Attack chains
            TrainingExample::new("security", "What is a supply chain attack?", "compromise a dependency/vendor to attack downstream consumers", 0.4, &["attacks", "advanced"]),
            TrainingExample::new("security", "What is credential stuffing?", "automated login attempts using breached username/password pairs", 0.3, &["attacks"]),
            TrainingExample::new("security", "What is a rainbow table?", "precomputed hash-to-password lookup table — defeated by salting", 0.35, &["cryptanalysis"]),
            TrainingExample::new("security", "What is lateral movement?", "attacker moves between systems after initial compromise to reach target", 0.4, &["attacks", "advanced"]),
            TrainingExample::new("security", "What is OWASP Top 10?", "most critical web application security risks: injection, broken auth, XSS, etc.", 0.25, &["standards"]),
            // Defense
            TrainingExample::new("security", "What is a SIEM?", "Security Information and Event Management — centralized log analysis and alerting", 0.3, &["defense"]),
            TrainingExample::new("security", "What is threat modeling?", "systematic analysis of potential threats, attack surfaces, and mitigations", 0.3, &["methodology"]),
            TrainingExample::new("security", "What is penetration testing?", "authorized simulated attack to find vulnerabilities before real attackers do", 0.25, &["methodology"]),
        ]
    }

    // ================================================================
    // CODE PATTERNS
    // ================================================================
    pub fn code_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("code", "pattern: error handling in Rust", "Result<T, E> with ? operator", 0.2, &["rust"]),
            TrainingExample::new("code", "pattern: ownership transfer", "move semantics", 0.3, &["rust", "memory"]),
            TrainingExample::new("code", "pattern: concurrent access", "Arc<Mutex<T>>", 0.4, &["rust", "concurrency"]),
            TrainingExample::new("code", "pattern: trait polymorphism", "dyn Trait or impl Trait", 0.3, &["rust", "oop"]),
            TrainingExample::new("code", "Big-O: binary search", "O(log n)", 0.25, &["algorithms"]),
            TrainingExample::new("code", "Big-O: quicksort average", "O(n log n)", 0.3, &["algorithms"]),
            TrainingExample::new("code", "Big-O: hash table lookup", "O(1) average", 0.2, &["data_structures"]),
            TrainingExample::new("code", "What is SOLID?", "Single responsibility, Open-closed, Liskov, Interface segregation, Dependency inversion", 0.35, &["design"]),
            // More algorithms
            TrainingExample::new("code", "Big-O: merge sort", "O(n log n) worst case", 0.3, &["algorithms"]),
            TrainingExample::new("code", "Big-O: linear search", "O(n)", 0.15, &["algorithms"]),
            TrainingExample::new("code", "Big-O: matrix multiplication (naive)", "O(n^3)", 0.3, &["algorithms"]),
            // Design patterns
            TrainingExample::new("code", "What is dependency injection?", "provide dependencies externally instead of creating them internally — improves testability", 0.3, &["design"]),
            TrainingExample::new("code", "What is the observer pattern?", "subjects notify observers of state changes — decouples components", 0.3, &["design"]),
            // Rust-specific
            TrainingExample::new("code", "What is a lifetime in Rust?", "compiler-tracked scope of a reference — ensures no dangling references", 0.35, &["rust", "memory"]),
            TrainingExample::new("code", "What is zero-cost abstraction?", "abstraction that compiles to the same code as hand-written version", 0.3, &["rust", "performance"]),
        ]
    }

    // ================================================================
    // LOGIC & REASONING
    // ================================================================
    pub fn logic_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("logic", "P AND Q, P=true, Q=true", "true", 0.05, &["propositional"]),
            TrainingExample::new("logic", "P OR Q, P=false, Q=true", "true", 0.05, &["propositional"]),
            TrainingExample::new("logic", "NOT P, P=true", "false", 0.05, &["propositional"]),
            TrainingExample::new("logic", "P -> Q, P=true, Q=false", "false", 0.15, &["propositional"]),
            TrainingExample::new("logic", "modus ponens: P, P->Q, therefore?", "Q", 0.2, &["inference"]),
            TrainingExample::new("logic", "modus tollens: NOT Q, P->Q, therefore?", "NOT P", 0.3, &["inference"]),
            TrainingExample::new("logic", "All A are B. x is A. Is x B?", "yes", 0.2, &["syllogism"]),
            TrainingExample::new("logic", "Some A are B. x is A. Is x B?", "not necessarily", 0.3, &["syllogism"]),
        ]
    }

    // ================================================================
    // GEOGRAPHY
    // ================================================================
    pub fn geography_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("geography", "capital of France", "Paris", 0.05, &["capitals"]),
            TrainingExample::new("geography", "capital of Japan", "Tokyo", 0.05, &["capitals"]),
            TrainingExample::new("geography", "largest ocean", "Pacific Ocean", 0.1, &["oceans"]),
            TrainingExample::new("geography", "longest river", "Nile (or Amazon by volume)", 0.15, &["rivers"]),
            TrainingExample::new("geography", "highest mountain", "Mount Everest (8,849m)", 0.1, &["mountains"]),
        ]
    }

    // ================================================================
    // MEDICINE
    // ================================================================
    pub fn medicine_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("medicine", "What is the hippocratic oath?", "do no harm — foundational medical ethics", 0.15, &["ethics"]),
            TrainingExample::new("medicine", "Normal human body temperature", "37°C / 98.6°F", 0.05, &["vitals"]),
            TrainingExample::new("medicine", "Normal resting heart rate", "60-100 bpm", 0.1, &["vitals"]),
            TrainingExample::new("medicine", "What is an antibiotic?", "medication that kills or inhibits bacteria", 0.15, &["pharmacology"]),
            TrainingExample::new("medicine", "What is CPR?", "cardiopulmonary resuscitation — chest compressions + rescue breathing", 0.1, &["emergency"]),
        ]
    }

    // ================================================================
    // PHILOSOPHY & ETHICS
    // ================================================================
    pub fn philosophy_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("philosophy", "What is the trolley problem?", "ethical dilemma: sacrifice one to save many?", 0.2, &["ethics"]),
            TrainingExample::new("philosophy", "What is Occam's razor?", "simplest explanation is usually correct", 0.15, &["epistemology"]),
            TrainingExample::new("philosophy", "What is the categorical imperative?", "act only by rules you'd want as universal laws (Kant)", 0.3, &["ethics"]),
            TrainingExample::new("philosophy", "What is empiricism?", "knowledge comes from sensory experience", 0.25, &["epistemology"]),
        ]
    }

    // ================================================================
    // PRIVACY, SECURITY, ANONYMITY (PSA — core PlausiDen domain)
    // ================================================================
    pub fn psa_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("psa", "What is plausible deniability?", "ability to deny knowledge of illegal actions because evidence is ambiguous", 0.2, &["privacy"]),
            TrainingExample::new("psa", "What is Tor?", "onion routing network for anonymous communication", 0.2, &["anonymity"]),
            TrainingExample::new("psa", "What is a VPN?", "encrypted tunnel between your device and a server", 0.15, &["privacy"]),
            TrainingExample::new("psa", "What is zero-knowledge proof?", "prove you know something without revealing what you know", 0.35, &["cryptography"]),
            TrainingExample::new("psa", "What is end-to-end encryption?", "only sender and receiver can read messages — not even the server", 0.2, &["cryptography"]),
            TrainingExample::new("psa", "What is metadata?", "data about data — who, when, where, how long", 0.15, &["privacy"]),
            TrainingExample::new("psa", "Why is metadata dangerous?", "reveals patterns, relationships, and behavior without content", 0.3, &["privacy"]),
            TrainingExample::new("psa", "What is a warrant canary?", "statement that no secret warrants have been received — removal signals surveillance", 0.3, &["legal"]),
        ]
    }

    // ================================================================
    // ECONOMICS
    // ================================================================
    pub fn economics_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("economics", "What is supply and demand?", "prices rise when demand exceeds supply, fall when supply exceeds demand", 0.15, &["fundamentals"]),
            TrainingExample::new("economics", "What is inflation?", "general increase in prices and decrease in purchasing power of money", 0.15, &["macroeconomics"]),
            TrainingExample::new("economics", "What is GDP?", "gross domestic product — total value of goods and services produced in a country", 0.1, &["macroeconomics"]),
            TrainingExample::new("economics", "What is a recession?", "two consecutive quarters of negative GDP growth", 0.2, &["macroeconomics"]),
            TrainingExample::new("economics", "What is compound interest?", "interest on both principal and accumulated interest: A = P(1+r)^n", 0.25, &["finance"]),
            TrainingExample::new("economics", "What is a monopoly?", "single seller dominates market with no close substitutes", 0.15, &["market_structure"]),
        ]
    }

    // ================================================================
    // PSYCHOLOGY
    // ================================================================
    pub fn psychology_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("psychology", "What is confirmation bias?", "tendency to seek information confirming existing beliefs", 0.2, &["cognitive_bias"]),
            TrainingExample::new("psychology", "What is the Dunning-Kruger effect?", "low-skill people overestimate their ability; high-skill people underestimate", 0.25, &["cognitive_bias"]),
            TrainingExample::new("psychology", "What is cognitive dissonance?", "mental discomfort from holding contradictory beliefs", 0.2, &["cognition"]),
            TrainingExample::new("psychology", "What is Maslow's hierarchy?", "physiological → safety → belonging → esteem → self-actualization", 0.2, &["motivation"]),
            TrainingExample::new("psychology", "What is the bystander effect?", "less likely to help when others are present", 0.2, &["social"]),
            TrainingExample::new("psychology", "What is anchoring bias?", "relying too heavily on the first piece of information encountered", 0.25, &["cognitive_bias"]),
        ]
    }

    // ================================================================
    // NETWORKING & PROTOCOLS
    // ================================================================
    pub fn networking_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("networking", "What are the OSI layers?", "Physical, Data Link, Network, Transport, Session, Presentation, Application", 0.25, &["fundamentals"]),
            TrainingExample::new("networking", "What is TCP vs UDP?", "TCP: reliable ordered delivery. UDP: fast unreliable datagrams", 0.2, &["transport"]),
            TrainingExample::new("networking", "What is DNS?", "Domain Name System — translates domain names to IP addresses", 0.15, &["application"]),
            TrainingExample::new("networking", "What is TLS?", "Transport Layer Security — encrypts data in transit", 0.2, &["security"]),
            TrainingExample::new("networking", "What is a firewall?", "filters network traffic based on rules — blocks unauthorized access", 0.15, &["security"]),
            TrainingExample::new("networking", "What is NAT?", "Network Address Translation — maps private IPs to public IP", 0.2, &["network"]),
            TrainingExample::new("networking", "What is HTTPS?", "HTTP over TLS — encrypted web traffic", 0.1, &["application", "security"]),
        ]
    }

    // ================================================================
    // DEMOCRACY & VOTING (Sacred.Vote domain)
    // ================================================================
    pub fn voting_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("voting", "What is ballot secrecy?", "no one can determine how a specific voter voted", 0.2, &["principles"]),
            TrainingExample::new("voting", "What is verifiable voting?", "voters can verify their vote was counted correctly without revealing it", 0.3, &["cryptographic"]),
            TrainingExample::new("voting", "What is a blind signature?", "signer signs a message without seeing its content — enables anonymous ballots", 0.35, &["cryptographic"]),
            TrainingExample::new("voting", "What is coercion resistance?", "voter cannot prove how they voted even under duress", 0.35, &["security"]),
            TrainingExample::new("voting", "What is end-to-end verifiability?", "voters verify: cast-as-intended, recorded-as-cast, tallied-as-recorded", 0.4, &["cryptographic"]),
            TrainingExample::new("voting", "What is a zero-knowledge proof in voting?", "prove eligibility to vote without revealing identity", 0.4, &["cryptographic", "privacy"]),
            TrainingExample::new("voting", "What is the Belenios protocol?", "verifiable voting protocol using ElGamal encryption and ZK proofs", 0.45, &["protocols"]),
        ]
    }

    // ================================================================
    // HISTORY
    // ================================================================
    pub fn history_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("history", "When did WW2 end?", "1945", 0.05, &["dates"]),
            TrainingExample::new("history", "What was the Magna Carta?", "1215 charter limiting the power of the English king", 0.2, &["law"]),
            TrainingExample::new("history", "What was the Renaissance?", "14th-17th century European cultural rebirth in art, science, philosophy", 0.2, &["culture"]),
            TrainingExample::new("history", "What was the Industrial Revolution?", "transition from agrarian to industrial economy, starting ~1760 in Britain", 0.2, &["economics"]),
            TrainingExample::new("history", "What is the Universal Declaration of Human Rights?", "1948 UN document establishing fundamental human rights for all people", 0.2, &["rights"]),
        ]
    }

    // ================================================================
    // AI & MACHINE LEARNING
    // ================================================================
    pub fn ai_ml_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("ai_ml", "What is overfitting?", "model learns noise in training data, performs poorly on new data", 0.25, &["fundamentals"]),
            TrainingExample::new("ai_ml", "What is gradient descent?", "optimization algorithm that iteratively adjusts parameters to minimize loss", 0.3, &["optimization"]),
            TrainingExample::new("ai_ml", "What is a neural network?", "layered graph of weighted connections that learns patterns from data", 0.2, &["architectures"]),
            TrainingExample::new("ai_ml", "What is reinforcement learning?", "agent learns by taking actions in environment and receiving rewards", 0.3, &["paradigms"]),
            TrainingExample::new("ai_ml", "What is a transformer?", "attention-based architecture: self-attention + feedforward, scales to billions of parameters", 0.35, &["architectures"]),
            TrainingExample::new("ai_ml", "What is HDC/VSA?", "hyperdimensional computing: encode data as high-dimensional vectors, compose with bind/bundle/permute", 0.3, &["architectures", "hdc"]),
            TrainingExample::new("ai_ml", "What is the bias-variance tradeoff?", "simple models underfit (high bias), complex models overfit (high variance)", 0.3, &["fundamentals"]),
            TrainingExample::new("ai_ml", "What is transfer learning?", "reuse knowledge from one task to improve performance on another", 0.25, &["techniques"]),
        ]
    }

    // ================================================================
    // LINEAR ALGEBRA & STATISTICS
    // ================================================================
    pub fn math_advanced_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("math_advanced", "What is a dot product?", "sum of element-wise products: a·b = Σ(ai*bi)", 0.25, &["linear_algebra"]),
            TrainingExample::new("math_advanced", "What is cosine similarity?", "dot(a,b) / (||a|| * ||b||) — measures angle between vectors", 0.3, &["linear_algebra"]),
            TrainingExample::new("math_advanced", "What is an eigenvalue?", "scalar λ where Av = λv — vector direction unchanged by transformation", 0.4, &["linear_algebra"]),
            TrainingExample::new("math_advanced", "What is standard deviation?", "measure of spread: sqrt(mean of squared deviations from mean)", 0.25, &["statistics"]),
            TrainingExample::new("math_advanced", "What is Bayes' theorem?", "P(A|B) = P(B|A)*P(A)/P(B) — updating beliefs with evidence", 0.35, &["statistics", "probability"]),
            TrainingExample::new("math_advanced", "What is the central limit theorem?", "sample means approach normal distribution regardless of population distribution", 0.35, &["statistics"]),
            TrainingExample::new("math_advanced", "What is a matrix inverse?", "A*A^-1 = I — only exists for square non-singular matrices", 0.3, &["linear_algebra"]),
        ]
    }

    // ================================================================
    // SOCIAL ENGINEERING DEFENSE
    // ================================================================
    pub fn social_engineering_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("social_eng", "What is pretexting?", "creating a fabricated scenario to extract information from a target", 0.3, &["techniques"]),
            TrainingExample::new("social_eng", "What is spear phishing?", "targeted phishing email personalized to a specific individual", 0.3, &["techniques"]),
            TrainingExample::new("social_eng", "What is baiting?", "leaving infected USB drives or enticing downloads to lure victims", 0.25, &["techniques"]),
            TrainingExample::new("social_eng", "What is tailgating?", "following authorized person through secure door without credentials", 0.2, &["physical"]),
            TrainingExample::new("social_eng", "How to detect phishing?", "check sender domain, hover over links, verify urgency claims, look for typos", 0.3, &["defense"]),
            TrainingExample::new("social_eng", "How to protect against social engineering?", "verify identity independently, never share credentials, question urgency, report suspicious contacts", 0.3, &["defense"]),
            TrainingExample::new("social_eng", "What is vishing?", "voice phishing — social engineering over phone calls", 0.2, &["techniques"]),
        ]
    }

    // ================================================================
    // OPERATING SYSTEMS & LINUX
    // ================================================================
    pub fn os_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("os", "What is a kernel?", "core of OS — manages hardware, memory, processes, I/O", 0.2, &["fundamentals"]),
            TrainingExample::new("os", "What is a process vs thread?", "process: isolated address space. thread: shared memory within process", 0.25, &["concurrency"]),
            TrainingExample::new("os", "What is virtual memory?", "abstraction giving each process its own address space, using disk as overflow", 0.3, &["memory"]),
            TrainingExample::new("os", "What is SELinux?", "mandatory access control — restricts processes to minimum required permissions", 0.3, &["security"]),
            TrainingExample::new("os", "What is a syscall?", "interface between user space and kernel — request OS services", 0.25, &["fundamentals"]),
            TrainingExample::new("os", "What is iptables/nftables?", "Linux firewall — filter packets by rules (source, dest, port, protocol)", 0.3, &["networking", "security"]),
        ]
    }

    // ================================================================
    // MULTI-STEP REASONING (harder — requires chaining knowledge)
    // ================================================================
    pub fn reasoning_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("reasoning", "If A implies B, and B implies C, and A is true, is C true?", "yes — by transitivity (A->B->C)", 0.4, &["chain"]),
            TrainingExample::new("reasoning", "All dogs are animals. Rex is a dog. Is Rex an animal?", "yes — syllogism: Rex is a dog, dogs are animals, Rex is an animal", 0.3, &["syllogism"]),
            TrainingExample::new("reasoning", "A box has 3 red and 5 blue balls. Probability of drawing red?", "3/8 = 37.5%", 0.3, &["probability"]),
            TrainingExample::new("reasoning", "If it rained, the ground is wet. The ground is dry. Did it rain?", "no — modus tollens: NOT wet -> NOT rained", 0.35, &["logic"]),
            TrainingExample::new("reasoning", "Train A goes 60mph, Train B goes 80mph, both start 100mi apart toward each other. When do they meet?", "in 0.714 hours (100/(60+80))", 0.45, &["word_problem"]),
            TrainingExample::new("reasoning", "Is 'all cats are black' disproved by a white cat?", "yes — one counterexample disproves a universal claim", 0.3, &["falsification"]),
            TrainingExample::new("reasoning", "Can you prove a negative?", "generally no — absence of evidence is not evidence of absence, but counterexamples disprove universals", 0.5, &["epistemology"]),
            TrainingExample::new("reasoning", "Post hoc ergo propter hoc — is this valid?", "no — correlation does not imply causation. A before B does not mean A caused B", 0.35, &["fallacies"]),
        ]
    }

    // ================================================================
    // CRYPTOGRAPHY (deeper than basic security)
    // ================================================================
    pub fn cryptography_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("crypto", "What is a hash function?", "one-way function: input -> fixed-size output, infeasible to reverse", 0.2, &["fundamentals"]),
            TrainingExample::new("crypto", "What is SHA-256?", "256-bit hash function in the SHA-2 family, used in Bitcoin and TLS", 0.25, &["hash"]),
            TrainingExample::new("crypto", "What is a digital signature?", "hash(message) encrypted with private key — proves authorship + integrity", 0.3, &["signatures"]),
            TrainingExample::new("crypto", "What is Diffie-Hellman?", "key exchange protocol: two parties derive shared secret over insecure channel", 0.35, &["key_exchange"]),
            TrainingExample::new("crypto", "What is post-quantum cryptography?", "algorithms resistant to quantum computer attacks: lattice-based, hash-based, code-based", 0.4, &["pqc"]),
            TrainingExample::new("crypto", "What is ML-KEM (Kyber)?", "lattice-based key encapsulation mechanism — NIST PQC standard", 0.45, &["pqc", "standards"]),
            TrainingExample::new("crypto", "What is homomorphic encryption?", "compute on encrypted data without decrypting — enables private cloud computation", 0.5, &["advanced"]),
        ]
    }

    // ================================================================
    // LAW & CIVIL RIGHTS
    // ================================================================
    pub fn law_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("law", "What is habeas corpus?", "right to challenge unlawful detention — 'produce the body'", 0.2, &["rights"]),
            TrainingExample::new("law", "What is the 4th Amendment?", "protection against unreasonable search and seizure — requires warrants", 0.2, &["us_constitution"]),
            TrainingExample::new("law", "What is the 5th Amendment?", "right against self-incrimination and due process of law", 0.2, &["us_constitution"]),
            TrainingExample::new("law", "What is GDPR?", "EU data protection regulation — right to be forgotten, consent, data minimization", 0.25, &["privacy_law"]),
            TrainingExample::new("law", "What is Section 230?", "US law shielding platforms from liability for user-generated content", 0.3, &["internet_law"]),
            TrainingExample::new("law", "What is the right to privacy?", "fundamental right to be free from surveillance and data collection without consent", 0.2, &["rights"]),
        ]
    }

    // ================================================================
    // META-COGNITIVE SELF-KNOWLEDGE (LFI's understanding of itself)
    // ================================================================
    pub fn self_knowledge_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("self", "What are you?", "a neurosymbolic AI engine using hyperdimensional computing, not a neural network", 0.1, &["identity"]),
            TrainingExample::new("self", "How do you store knowledge?", "VSA holographic memory: bind(key, value) stored in superposition", 0.25, &["architecture"]),
            TrainingExample::new("self", "How do you reason?", "dual mode: System 1 (fast pattern match) and System 2 (MCTS deliberation)", 0.3, &["architecture"]),
            TrainingExample::new("self", "Can you lie?", "the ProvenanceKind tag prevents me from presenting reconstructions as traced derivations", 0.35, &["provenance", "honesty"]),
            TrainingExample::new("self", "What are your limitations?", "VSA similarity is approximate, holographic memory has capacity limits, I cannot access external networks without agents", 0.3, &["limitations"]),
            TrainingExample::new("self", "What is your purpose?", "privacy, security, and anonymity (PSA) for everyone — accessible, local, transparent AI", 0.15, &["mission"]),
            TrainingExample::new("self", "Who built you?", "PlausiDen Technologies — a company building civil rights tools", 0.1, &["identity"]),
            TrainingExample::new("self", "What makes you different from LLMs?", "deterministic VSA operations instead of probabilistic weights, explainable reasoning, runs locally, no training data leakage", 0.3, &["architecture"]),
        ]
    }

    // ================================================================
    // ENVIRONMENTAL SCIENCE
    // ================================================================
    pub fn environment_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("environment", "What is the greenhouse effect?", "gases trap heat in atmosphere — CO2, methane, water vapor", 0.15, &["climate"]),
            TrainingExample::new("environment", "What is biodiversity?", "variety of life in an ecosystem — species diversity, genetic diversity", 0.15, &["ecology"]),
            TrainingExample::new("environment", "What is the ozone layer?", "O3 layer in stratosphere that absorbs UV radiation from the sun", 0.2, &["atmosphere"]),
            TrainingExample::new("environment", "What is carbon neutrality?", "net zero CO2 emissions — balance emissions with removal/offset", 0.2, &["climate"]),
            TrainingExample::new("environment", "What is renewable energy?", "energy from sources that replenish naturally: solar, wind, hydro, geothermal", 0.15, &["energy"]),
        ]
    }

    /// Get ALL training examples across ALL domains.
    // ================================================================
    // COMMON SENSE & WORLD KNOWLEDGE
    // ================================================================
    pub fn common_sense_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("common_sense", "Can a fish climb a tree?", "no — fish have fins, not limbs adapted for climbing", 0.1, &["biology"]),
            TrainingExample::new("common_sense", "Is ice heavier than water?", "no — ice is less dense, which is why it floats", 0.15, &["physics"]),
            TrainingExample::new("common_sense", "Can you see in complete darkness?", "no — vision requires photons (light)", 0.1, &["physics"]),
            TrainingExample::new("common_sense", "Does hot air rise or sink?", "rises — hot air is less dense than cold air", 0.1, &["physics"]),
            TrainingExample::new("common_sense", "Why does the moon have phases?", "we see different amounts of its sunlit half as it orbits Earth", 0.2, &["astronomy"]),
            TrainingExample::new("common_sense", "Why do we have seasons?", "Earth's axial tilt (23.5 degrees) causes varying sunlight angles throughout the year", 0.2, &["astronomy"]),
        ]
    }

    // ================================================================
    // PLAUSIDEN ECOSYSTEM KNOWLEDGE
    // ================================================================
    pub fn plausiden_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("plausiden", "What is PlausiDen?", "PLAUSIbly DENiable — civil rights toolkit for plausible deniability, privacy, and security", 0.15, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is Sacred.Vote?", "zero-trust cryptographic polling platform — voter identity decoupled from ballot records", 0.2, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is PlausiDen-Engine?", "core data pollution library — generates forensically indistinguishable synthetic artifacts", 0.25, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is PlausiDen-Shield?", "AI control plane for the PlausiDen ecosystem — orchestrates all components via neurosymbolic AI", 0.25, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is PlausiDen-PDFS?", "Plausibly Deniable File System — hidden encrypted volumes indistinguishable from random noise", 0.3, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is PlausiDen-Shard?", "cryptographic sharding engine — post-quantum fragment lifecycle with ML-KEM and Shamir SSS", 0.35, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is PlausiDen-Swarm?", "P2P data pollution network — any data on any device could belong to anyone", 0.3, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is the Neurosymbolic Toolkit?", "6-crate Rust workspace: hdc-core, neupsl, lnn, vsa, hdlm — foundation for LFI", 0.2, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is LFI?", "Localized Forensic Intelligence — neurosymbolic AI engine using HDC, PSL, active inference, MCTS", 0.2, &["ecosystem"]),
            TrainingExample::new("plausiden", "What is the Super Society goal?", "PSA — Privacy, Security, Anonymity for everyone. Build tools that protect human agency.", 0.15, &["mission"]),
        ]
    }

    // ================================================================
    // ANALOGY-BASED REASONING
    // ================================================================
    pub fn analogy_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("analogy", "Hand is to glove as foot is to?", "shoe", 0.15, &["pattern"]),
            TrainingExample::new("analogy", "Hot is to cold as light is to?", "dark", 0.1, &["opposites"]),
            TrainingExample::new("analogy", "CPU is to computer as brain is to?", "human body", 0.2, &["function"]),
            TrainingExample::new("analogy", "Encryption is to privacy as lock is to?", "physical security", 0.25, &["security"]),
            TrainingExample::new("analogy", "HDC bind is to XOR as HDC bundle is to?", "majority vote (sum + clip)", 0.35, &["hdc"]),
            TrainingExample::new("analogy", "System 1 is to fast as System 2 is to?", "slow but deliberate (deep reasoning)", 0.25, &["cognition"]),
        ]
    }

    // ================================================================
    // DISTRIBUTED SYSTEMS & BLOCKCHAIN
    // ================================================================
    pub fn distributed_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("distributed", "What is consensus?", "agreement among distributed nodes on a single value despite failures", 0.3, &["fundamentals"]),
            TrainingExample::new("distributed", "What is the CAP theorem?", "distributed system can guarantee at most 2 of: Consistency, Availability, Partition tolerance", 0.35, &["theorems"]),
            TrainingExample::new("distributed", "What is Byzantine fault tolerance?", "system operates correctly even if some nodes are malicious or faulty", 0.4, &["consensus"]),
            TrainingExample::new("distributed", "What is a Merkle tree?", "hash tree where each leaf is data hash and each node is hash of children — efficient verification", 0.35, &["data_structures"]),
            TrainingExample::new("distributed", "What is eventual consistency?", "all replicas converge to the same value given enough time without new writes", 0.3, &["consistency"]),
            TrainingExample::new("distributed", "What is a CRDT?", "Conflict-free Replicated Data Type — merges without coordination, always converges", 0.35, &["data_structures"]),
        ]
    }

    // ================================================================
    // DATA SCIENCE & ANALYSIS
    // ================================================================
    pub fn data_science_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("data_science", "What is overfitting vs underfitting?", "overfitting: model memorizes noise. underfitting: model too simple to capture patterns", 0.25, &["fundamentals"]),
            TrainingExample::new("data_science", "What is cross-validation?", "split data into k folds, train on k-1, test on 1, rotate — reduces overfitting", 0.3, &["methodology"]),
            TrainingExample::new("data_science", "What is feature engineering?", "creating new features from raw data to improve model performance", 0.25, &["methodology"]),
            TrainingExample::new("data_science", "What is a confusion matrix?", "table of true positives, false positives, true negatives, false negatives", 0.3, &["evaluation"]),
            TrainingExample::new("data_science", "What is precision vs recall?", "precision: TP/(TP+FP). recall: TP/(TP+FN). tradeoff between them.", 0.3, &["evaluation"]),
            TrainingExample::new("data_science", "What is the F1 score?", "harmonic mean of precision and recall: 2*P*R/(P+R)", 0.3, &["evaluation"]),
        ]
    }

    // ================================================================
    // DIGITAL FORENSICS & INVESTIGATION
    // ================================================================
    pub fn forensics_examples() -> Vec<TrainingExample> {
        vec![
            TrainingExample::new("forensics", "What is chain of custody?", "documented trail showing who handled evidence, when, and how — ensures admissibility", 0.2, &["legal"]),
            TrainingExample::new("forensics", "What is disk imaging?", "bit-for-bit copy of storage media for analysis without modifying the original", 0.25, &["methodology"]),
            TrainingExample::new("forensics", "What is metadata analysis?", "examining file creation dates, GPS coordinates, camera info embedded in files", 0.25, &["techniques"]),
            TrainingExample::new("forensics", "What is log analysis?", "examining system/application/network logs for evidence of compromise or activity", 0.2, &["techniques"]),
            TrainingExample::new("forensics", "What is memory forensics?", "analyzing RAM dumps for running processes, network connections, encryption keys", 0.35, &["techniques"]),
            TrainingExample::new("forensics", "What is steganography detection?", "finding hidden data within images, audio, or other files", 0.35, &["techniques"]),
        ]
    }

    pub fn all_examples() -> Vec<TrainingExample> {
        let mut all = Vec::new();
        all.extend(Self::math_examples());
        all.extend(Self::physics_examples());
        all.extend(Self::biology_examples());
        all.extend(Self::chemistry_examples());
        all.extend(Self::security_examples());
        all.extend(Self::code_examples());
        all.extend(Self::logic_examples());
        all.extend(Self::geography_examples());
        all.extend(Self::medicine_examples());
        all.extend(Self::philosophy_examples());
        all.extend(Self::psa_examples());
        all.extend(Self::economics_examples());
        all.extend(Self::psychology_examples());
        all.extend(Self::networking_examples());
        all.extend(Self::voting_examples());
        all.extend(Self::history_examples());
        all.extend(Self::ai_ml_examples());
        all.extend(Self::math_advanced_examples());
        all.extend(Self::social_engineering_examples());
        all.extend(Self::os_examples());
        all.extend(Self::reasoning_examples());
        all.extend(Self::cryptography_examples());
        all.extend(Self::law_examples());
        all.extend(Self::self_knowledge_examples());
        all.extend(Self::environment_examples());
        all.extend(Self::common_sense_examples());
        all.extend(Self::plausiden_examples());
        all.extend(Self::analogy_examples());
        all.extend(Self::distributed_examples());
        all.extend(Self::data_science_examples());
        all.extend(Self::forensics_examples());
        all
    }

    /// Cross-domain relationships — when learning one domain, related domains get a boost.
    /// Returns (domain, related_domains_with_transfer_weight).
    pub fn domain_relationships() -> Vec<(&'static str, Vec<(&'static str, f64)>)> {
        vec![
            ("security", vec![("crypto", 0.7), ("networking", 0.5), ("code", 0.3), ("social_eng", 0.6), ("psa", 0.8)]),
            ("crypto", vec![("security", 0.7), ("math", 0.5), ("math_advanced", 0.6), ("psa", 0.6)]),
            ("code", vec![("logic", 0.5), ("math", 0.3), ("os", 0.4), ("security", 0.3)]),
            ("math", vec![("math_advanced", 0.9), ("physics", 0.6), ("code", 0.3)]),
            ("math_advanced", vec![("math", 0.9), ("physics", 0.5), ("ai_ml", 0.6)]),
            ("physics", vec![("math", 0.6), ("chemistry", 0.4), ("environment", 0.3)]),
            ("biology", vec![("medicine", 0.7), ("chemistry", 0.5), ("environment", 0.4)]),
            ("medicine", vec![("biology", 0.7), ("chemistry", 0.4)]),
            ("ai_ml", vec![("math_advanced", 0.6), ("code", 0.4), ("logic", 0.5)]),
            ("psa", vec![("security", 0.8), ("crypto", 0.6), ("voting", 0.5), ("law", 0.5)]),
            ("voting", vec![("psa", 0.5), ("crypto", 0.6), ("law", 0.4)]),
            ("law", vec![("psa", 0.5), ("philosophy", 0.3), ("voting", 0.4)]),
            ("reasoning", vec![("logic", 0.8), ("philosophy", 0.5), ("math", 0.4)]),
            ("plausiden", vec![("psa", 0.9), ("security", 0.5), ("self", 0.8)]),
            ("networking", vec![("security", 0.5), ("os", 0.4), ("code", 0.3)]),
        ]
    }

    /// Apply knowledge transfer: boost related domains when one domain is learned.
    pub fn apply_transfer(
        knowledge: &mut KnowledgeEngine,
        learned_domain: &str,
        boost: f64,
    ) -> Result<(), HdcError> {
        let relationships = Self::domain_relationships();
        for (domain, related) in &relationships {
            if *domain == learned_domain {
                for (related_domain, weight) in related {
                    let transfer_boost = boost * weight;
                    knowledge.reinforce(related_domain);
                    debuglog!("Transfer: {} -> {} (boost={:.3})", domain, related_domain, transfer_boost);
                }
                break;
            }
        }
        Ok(())
    }

    /// Get examples sorted by difficulty (curriculum learning — easy first).
    pub fn curriculum_ordered() -> Vec<TrainingExample> {
        let mut all = Self::all_examples();
        all.sort_by(|a, b| a.difficulty.partial_cmp(&b.difficulty).unwrap_or(std::cmp::Ordering::Equal));
        all
    }

    /// Get examples filtered by maximum difficulty (progressive disclosure).
    pub fn up_to_difficulty(max_difficulty: f64) -> Vec<TrainingExample> {
        Self::all_examples().into_iter()
            .filter(|e| e.difficulty <= max_difficulty)
            .collect()
    }

    /// Get examples for a specific domain.
    pub fn domain_examples(domain: &str) -> Vec<TrainingExample> {
        Self::all_examples().into_iter()
            .filter(|e| e.domain == domain)
            .collect()
    }

    /// Get all unique domain names.
    pub fn domains() -> Vec<String> {
        let all = Self::all_examples();
        let mut domains: Vec<String> = all.iter()
            .map(|e| e.domain.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        domains.sort();
        domains
    }

    /// Ingest training examples into a knowledge engine.
    pub fn ingest_into_knowledge(
        engine: &mut KnowledgeEngine,
        examples: &[TrainingExample],
    ) -> Result<usize, HdcError> {
        debuglog!("TrainingDataGenerator::ingest: {} examples", examples.len());
        let mut ingested = 0;
        for ex in examples {
            engine.learn(&ex.domain, &[], true)?;
            let concept_name = format!("{}_{}", ex.domain, ingested);
            engine.learn_with_definition(
                &concept_name,
                &format!("{} → {}", ex.input, ex.expected_output),
                &[&ex.domain],
                ex.difficulty,
                true,
            )?;
            ingested += 1;
        }
        Ok(ingested)
    }
}

// ================================================================
// Correction Loop — Interactive Teach-Correct Cycle
// ================================================================

/// Evaluates LFI against training data and corrects wrong answers.
pub struct CorrectionLoop {
    pub corrections: Vec<CorrectionRecord>,
    pub evaluations: Vec<EvaluationResult>,
    pub total_correct: usize,
    pub total_evaluated: usize,
}

impl CorrectionLoop {
    pub fn new() -> Self {
        Self {
            corrections: Vec::new(),
            evaluations: Vec::new(),
            total_correct: 0,
            total_evaluated: 0,
        }
    }

    /// Evaluate and correct LFI's knowledge against training examples.
    ///
    /// For each example:
    ///   1. Check if LFI knows the concept (via mastery > 0)
    ///   2. If not, teach it (correction)
    ///   3. Track accuracy per domain
    pub fn evaluate_and_correct(
        &mut self,
        engine: &mut KnowledgeEngine,
        examples: &[TrainingExample],
    ) -> Result<Vec<EvaluationResult>, HdcError> {
        debuglog!("CorrectionLoop::evaluate_and_correct: {} examples", examples.len());

        // Group by domain.
        let mut domain_map: std::collections::HashMap<String, Vec<&TrainingExample>> =
            std::collections::HashMap::new();
        for ex in examples {
            domain_map.entry(ex.domain.clone()).or_default().push(ex);
        }

        let mut results = Vec::new();

        for (domain, domain_examples) in &domain_map {
            let mut correct = 0;
            let mut corrections = 0;

            for ex in domain_examples {
                let concept_name = format!("{}_{}", ex.domain, ex.input.replace(' ', "_"));
                let mastery = engine.mastery_of(&concept_name);

                if mastery > 0.3 {
                    // LFI "knows" this — count as correct.
                    correct += 1;
                } else {
                    // LFI doesn't know this — teach it.
                    engine.learn_with_definition(
                        &concept_name,
                        &format!("Q: {} A: {}", ex.input, ex.expected_output),
                        &[&ex.domain],
                        0.5, // Start at moderate mastery after correction
                        true,
                    )?;
                    corrections += 1;

                    self.corrections.push(CorrectionRecord {
                        domain: ex.domain.clone(),
                        input: ex.input.clone(),
                        wrong_answer: "unknown".into(),
                        correct_answer: ex.expected_output.clone(),
                        corrected: true,
                    });
                }
            }

            let total = domain_examples.len();
            self.total_correct += correct;
            self.total_evaluated += total;

            let result = EvaluationResult {
                domain: domain.clone(),
                total,
                correct,
                accuracy: correct as f64 / total as f64,
                corrections_made: corrections,
            };
            results.push(result.clone());
            self.evaluations.push(result);
        }

        Ok(results)
    }

    /// Overall accuracy across all evaluations.
    pub fn overall_accuracy(&self) -> f64 {
        if self.total_evaluated == 0 { return 0.0; }
        self.total_correct as f64 / self.total_evaluated as f64
    }

    /// Total corrections made.
    pub fn total_corrections(&self) -> usize {
        self.corrections.len()
    }

    /// Get the domains that need the most improvement (highest correction rate).
    pub fn weakest_domains(&self) -> Vec<(String, f64)> {
        let mut domain_errors: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
        for c in &self.corrections {
            let entry = domain_errors.entry(c.domain.clone()).or_insert((0, 0));
            entry.0 += 1; // errors
        }
        for eval in &self.evaluations {
            let entry = domain_errors.entry(eval.domain.clone()).or_insert((0, 0));
            entry.1 = eval.total; // total
        }

        let mut weak: Vec<(String, f64)> = domain_errors.iter()
            .filter(|(_, (errors, total))| *total > 0)
            .map(|(domain, (errors, total))| (domain.clone(), *errors as f64 / *total as f64))
            .collect();
        weak.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        weak
    }

    /// Get examples that should be reviewed (spaced repetition — focus on mistakes).
    pub fn review_queue(&self) -> Vec<&CorrectionRecord> {
        self.corrections.iter().filter(|c| c.corrected).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_examples_comprehensive() {
        let all = TrainingDataGenerator::all_examples();
        assert!(all.len() >= 90, "Should have 90+ examples across all domains, got {}", all.len());
        let domains: std::collections::HashSet<&str> = all.iter().map(|e| e.domain.as_str()).collect();
        assert!(domains.len() >= 10, "Should have 10+ domains, got {}", domains.len());
        for domain in &["math", "physics", "biology", "chemistry", "security", "code", "logic", "geography", "medicine", "philosophy", "psa"] {
            assert!(domains.contains(domain), "Missing domain: {}", domain);
        }
    }

    #[test]
    fn test_domain_sizes() {
        assert!(TrainingDataGenerator::math_examples().len() >= 19);
        assert!(TrainingDataGenerator::physics_examples().len() >= 8);
        assert!(TrainingDataGenerator::biology_examples().len() >= 7);
        assert!(TrainingDataGenerator::security_examples().len() >= 12);
        assert!(TrainingDataGenerator::psa_examples().len() >= 8);
    }

    #[test]
    fn test_tags_present() {
        let all = TrainingDataGenerator::all_examples();
        let with_tags = all.iter().filter(|e| !e.tags.is_empty()).count();
        assert_eq!(with_tags, all.len(), "Every example should have tags");
    }

    #[test]
    fn test_correction_loop_basic() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        let mut loop_ = CorrectionLoop::new();
        let examples = TrainingDataGenerator::math_examples();
        let results = loop_.evaluate_and_correct(&mut engine, &examples)?;
        assert!(!results.is_empty());
        // First run: LFI knows nothing, so all should be corrections.
        assert!(loop_.total_corrections() > 0);
        Ok(())
    }

    #[test]
    fn test_correction_improves_accuracy() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        let examples = TrainingDataGenerator::math_examples();

        // First pass: LFI knows nothing.
        let mut loop1 = CorrectionLoop::new();
        loop1.evaluate_and_correct(&mut engine, &examples)?;
        let acc1 = loop1.overall_accuracy();

        // Second pass: LFI should know the corrections from first pass.
        let mut loop2 = CorrectionLoop::new();
        loop2.evaluate_and_correct(&mut engine, &examples)?;
        let acc2 = loop2.overall_accuracy();

        assert!(acc2 >= acc1, "Second pass should be at least as accurate: {:.2} vs {:.2}", acc2, acc1);
        Ok(())
    }

    #[test]
    fn test_multi_domain_evaluation() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        let mut loop_ = CorrectionLoop::new();
        let all = TrainingDataGenerator::all_examples();
        let results = loop_.evaluate_and_correct(&mut engine, &all)?;
        assert!(results.len() >= 10, "Should evaluate 10+ domains");
        for r in &results {
            assert!(r.total > 0);
            assert!(r.accuracy >= 0.0 && r.accuracy <= 1.0);
        }
        Ok(())
    }

    #[test]
    fn test_psa_domain_coverage() {
        let psa = TrainingDataGenerator::psa_examples();
        let topics: Vec<&str> = psa.iter().map(|e| e.input.as_str()).collect();
        assert!(topics.iter().any(|t| t.contains("plausible deniability")));
        assert!(topics.iter().any(|t| t.contains("zero-knowledge")));
        assert!(topics.iter().any(|t| t.contains("Tor")));
    }

    #[test]
    fn test_ingest_all_domains() -> Result<(), HdcError> {
        let mut engine = KnowledgeEngine::new();
        let initial = engine.concept_count();
        let all = TrainingDataGenerator::all_examples();
        let ingested = TrainingDataGenerator::ingest_into_knowledge(&mut engine, &all)?;
        assert_eq!(ingested, all.len());
        assert!(engine.concept_count() > initial + 50);
        Ok(())
    }
}
