# LFI Confidentiality Kernel Design

**Document version:** 1.0
**Author context:** Handoff from planning conversation to Claude Code for implementation
**Repository target:** new top-level crate `lfi-conf-kernel` in the LFI workspace
**Status:** Design specification, ready for sprint-by-sprint implementation

---

## Mission statement

LFI handles credentials, PII, signing keys, biometric data, mesh peer secrets, and arbitrary user-confidential data. The current architecture treats these as application-layer strings — passed through function arguments, logged via `tracing`, serialized via `serde`, transmitted via whatever network code is convenient. This is the same architectural mistake every credential-handling system makes, and it produces the same failure mode: **plaintext leaks through paths the application developer didn't anticipate**.

The Confidentiality Kernel is a runtime substrate that makes plaintext leaks **structurally impossible**, not best-effort prevented. Every operation that touches sensitive data passes through the kernel. The kernel enforces: type discipline, memory protection, computation protection, network egress control, retroactive history scrubbing, and tamper-evident audit. Application code physically cannot bypass these guarantees because the type system refuses to compile code that would.

This is positioned as a foundational LFI subsystem at the same architectural level as PSL, the HDC core, the reasoning provenance system, and the crypto epistemology layer. It is not a service or a feature. It is a kernel.

---

## Threat model

**In scope:**
- Plaintext exposure of secrets in process memory (cold-boot attacks, kernel memory dumps, swap exposure, hibernation files, core dumps)
- Plaintext exposure in logs, traces, telemetry, error messages, debug dumps
- Plaintext exposure over network channels (downgraded TLS, plaintext HTTP, plaintext SMTP, accidentally-public Slack webhooks)
- Plaintext echoed back from LLM API calls (the secret was in the prompt; the model echoes it in the response or remembers it across sessions)
- Plaintext persisted in chat histories, command histories, terminal scrollback, browser caches
- Compromise of an individual LFI module via malicious crate dependency, supply-chain attack, or buggy parser
- Mesh peer attempting to extract secrets from a sealed payload they don't have the capability to unseal
- Insider with root on the host attempting to read in-flight plaintext outside an enclave
- Forensic recovery of secrets from disk after process exit

**Out of scope (explicitly):**
- Adversary with arbitrary code execution inside the TEE/enclave (different defense layer)
- Compromise of the hardware root of trust (TPM, SGX MRENCLAVE, SEV firmware)
- Side-channel attacks on the encryption primitives themselves (Spectre, RowHammer of key material) — mitigated by hardware-level defenses, not kernel design
- Attackers physically present at the keyboard while a `use_within` block is executing (not a software problem)
- Quantum cryptanalysis of currently-deployed primitives — addressed by parallel PQ migration, not by this kernel

---

## The four invariants

The kernel maintains four invariants. Every subsystem either upholds an invariant or enables one. If an operation can't satisfy all four, the kernel refuses the operation.

**Invariant 1 — Sealed-by-default.** Sensitive data exists in process memory only as `Sealed<T>` ciphertext, except inside a controlled `use_within` closure where plaintext exists for the closure's duration in mlocked memory.

**Invariant 2 — Capability-gated unsealing.** Plaintext access requires a valid `CapabilityRef` whose policy permits the requested operation, with consent obtained at the level the policy demands.

**Invariant 3 — Egress-scanned.** No bytes leave the host (network, disk, IPC, log) without passing through either (a) an end-to-end-encrypted channel to a verified peer, or (b) the egress scanner, which redacts detected sensitive data per policy.

**Invariant 4 — Audit-chained.** Every operation against sealed data produces an entry in a Merkle-chained, append-only audit log, optionally accompanied by a zero-knowledge proof of operation correctness.

The rest of this document specifies how the eight subsystems implement these invariants.

---

## Subsystem 1 — Type discipline

### Core trait hierarchy

```rust
/// Marker trait. Anything implementing this is sensitive.
/// The orphan rule prevents external crates from implementing it for
/// types they don't own — extension requires explicit kernel coordination.
pub trait Sensitive: private::SealedMarker {
    const CLASSIFICATION: Classification;
    const DEFAULT_POLICY: PolicyTemplate;
    fn provenance(&self) -> &ProvenanceChain;
}

/// Classification taxonomy. Used by the egress scanner and the audit
/// kernel to decide handling. Loosely modeled on US classification levels
/// but adapted for AI/data context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Classification {
    /// Generic credentials: passwords, API keys, OAuth tokens.
    /// Lifetime-bounded by policy; rotatable.
    Credential,
    /// Asymmetric private keys, symmetric encryption keys.
    /// Long-lived; rotation is expensive; theft is catastrophic.
    KeyMaterial,
    /// Personally identifiable information about a specific subject.
    /// GDPR/CCPA-aligned handling required.
    Pii { subject: SubjectId },
    /// Biometric templates: face embeddings, fingerprint hashes,
    /// voice profiles. Treat as super-PII; revocation impossible.
    Biometric { subject: SubjectId },
    /// Operator-marked confidential business data.
    /// Application-defined sensitivity.
    Confidential { tag: ConfidentialityTag },
    /// Mesh-peer-shared secret that LFI holds in trust for another node.
    /// Cannot be revealed even to the local operator without that peer's consent.
    MeshTrust { peer: PeerIdentity },
    /// Ephemeral data that must not persist beyond the current operation.
    /// Examples: nonces, session tokens, intermediate computation results.
    Ephemeral,
}
```

### The `Sealed<T>` wrapper

```rust
/// Wraps a sensitive value in encrypted-at-rest form.
///
/// The inner T exists in plaintext only inside `use_within` closures
/// brokered through the kernel. The `Sealed<T>` itself is safe to
/// store, transmit (over verified channels), and reason over via HDC
/// encoding without exposing the underlying plaintext.
pub struct Sealed<T: Sensitive> {
    /// XChaCha20-Poly1305 ciphertext. Nonce in the AAD.
    ciphertext: SealedBytes,
    /// Reference to the symmetric key. The key itself lives in
    /// the protected key store; this is just a key identifier.
    key_ref: KeyRef,
    /// Type-level proof of which T was sealed. Prevents type confusion
    /// (e.g., unsealing an SshCredential and accidentally treating it
    /// as an ApiKey because the bytes happen to fit both shapes).
    _phantom: PhantomData<T>,
    /// Capability required to unseal. Mismatched capability →
    /// kernel refuses the unseal regardless of key availability.
    capability: CapabilityRef,
    /// Policy attached at seal time. Immutable.
    policy: PolicyRef,
    /// Provenance: where this sealed value originated. Links into
    /// the reasoning provenance system so any conclusion derived
    /// from this secret is traceable.
    provenance: ProvenanceChain,
    /// Sealing metadata: when, by whom, under what session.
    sealing_metadata: SealingMetadata,
}

impl<T: Sensitive> Sealed<T> {
    /// The ONLY way to access plaintext.
    ///
    /// `op` describes what the caller intends to do with the plaintext.
    /// The broker validates that the capability permits this operation,
    /// the policy allows this operation now, and consent (if required)
    /// has been obtained.
    ///
    /// `f` receives a borrowed reference to the plaintext. The plaintext
    /// lives in mlocked memory and is zeroized when the closure returns.
    /// The closure cannot leak the reference because of lifetime constraints.
    pub fn use_within<R, F>(
        &self,
        broker: &Broker,
        op: Operation,
        f: F,
    ) -> Result<R, KernelError>
    where
        F: FnOnce(&T) -> R,
        R: NonSensitive, // compile-time: closure can't return the secret
    {
        broker.unseal_and_use(self, op, f)
    }
}

// CRITICAL: Sealed<T> deliberately does NOT implement:
//   - Debug    (printing leaks structure)
//   - Display  (formatting leaks structure)
//   - Serialize (serialization may leak through any sink)
//   - Clone    (each sealing has unique nonce; cloning ciphertext is fine,
//              but the API discourages handle proliferation)
//   - Send     (configurable per-classification; defaults conservative)
//   - Sync     (configurable per-classification)
//
// The `NonSensitive` trait on closure return prevents the obvious bypass:
//
//     sealed.use_within(broker, op, |plaintext| plaintext.clone())
//                                               ^^^^^^^^^^^^^^^^^
//                                               returns sensitive T;
//                                               compile error.
```

### The `NonSensitive` marker — preventing return-channel exfiltration

```rust
/// Marker for types known not to contain sensitive data.
/// `use_within` closures must return `NonSensitive` types.
pub trait NonSensitive: private::NonSensitiveMarker {}

// Implemented for all primitive non-sensitive types.
impl NonSensitive for () {}
impl NonSensitive for bool {}
impl NonSensitive for u8 {}
// ... etc.

// Container impls: container is NonSensitive iff contents are.
impl<T: NonSensitive> NonSensitive for Vec<T> {}
impl<T: NonSensitive, E: NonSensitive> NonSensitive for Result<T, E> {}
impl<T: NonSensitive> NonSensitive for Option<T> {}

// Sealed<T> is NonSensitive — you can re-seal inside the closure
// and return the new sealed wrapper. This is the supported pattern
// for transformation operations.
impl<T: Sensitive> NonSensitive for Sealed<T> {}

// Operation results: structural data about what happened, with no plaintext.
impl NonSensitive for OperationOutcome {}
```

### Forbidden trait combinations

The kernel uses `private::SealedMarker` (sealed trait pattern) to prevent external crates from creating types that bypass the discipline. A type cannot be both `Sensitive` and `Display` — the build fails with a custom diagnostic via `compile_error!` invoked from a derive macro:

```rust
#[derive(Sensitive)]
#[classification(Credential)]
#[default_policy(StandardCredential)]
struct ApiKey {
    bytes: [u8; 32],
}

// If you also write:
//     impl Display for ApiKey { ... }
// the kernel macro emits:
//     compile_error!("ApiKey is Sensitive and cannot impl Display.
//                     Use Sealed<ApiKey>::use_within if you need to format.");
```

---

## Subsystem 2 — Memory protection

### Key store hierarchy

The kernel detects available hardware at boot and selects the strongest available key store. Operator policy can require a minimum level; LFI refuses to start if the requirement isn't met.

```rust
pub enum KeyStore {
    /// AMD SEV-SNP confidential VM.
    /// Memory is encrypted by the CPU; even the host hypervisor
    /// cannot read guest memory.
    /// Detection: cpuid leaf 0x8000001F bit 4.
    SevSnp { vmpl: u8, attestation: SnpAttestation },

    /// Intel TDX confidential VM. Similar properties to SEV-SNP.
    /// Detection: cpuid leaf 0x21 + MSR 0x1500.
    Tdx { attestation: TdxQuote },

    /// Intel SGX enclave. Per-process EPC pages, hardware-encrypted.
    /// Limited EPC size on most consumer hardware (~128MB pre-Ice Lake).
    /// Detection: cpuid leaf 0x12.
    SgxEnclave { mrenclave: [u8; 32], mrsigner: [u8; 32] },

    /// ARM TrustZone secure world.
    /// Target for PlausiDenOS on Pixel 10 Pro XL.
    /// Detection: SMC interface availability.
    TrustZone { tee_implementation: TeeImpl },

    /// Apple Secure Enclave Processor.
    /// Available on Apple Silicon Macs and iOS devices.
    /// Detection: IOSurface SEP attestation interface.
    SecureEnclave { uid_class: SepUidClass },

    /// TPM 2.0 sealed against PCR state.
    /// Keys unseal only when boot measurements match.
    /// Available on most modern PCs.
    /// Detection: /dev/tpmrm0 + TPM2_GetCapability.
    TpmSealed {
        pcr_policy: PcrPolicy,
        sealed_blob: TpmSealedBlob,
    },

    /// Linux kernel keyring with mlock + MADV_DONTDUMP.
    /// Keys never visible to userspace except via brief broker ops.
    /// Always available on Linux 4.x+.
    LockedKernelKeyring { keyring_id: KeyringSerial },

    /// Userspace mlock as last resort.
    /// Same protections, no kernel separation.
    /// Always available; degraded mode.
    LockedUserspace,
}

impl KeyStore {
    pub fn detect_strongest(min_required: KeyStoreLevel) -> Result<Self, KernelError> {
        // Probe in priority order; return first available.
        // If none meets min_required, error out.
    }
}
```

### Plaintext memory discipline

Plaintext exists in only two places:

1. **Inside the protected key store** — encrypted at rest, opaque to host kernel under SEV-SNP/TDX, hardware-isolated under SGX/TrustZone/SEP, kernel-namespaced under TPM/keyring.
2. **Inside an active `use_within` closure** — in mlocked, MADV_DONTDUMP'd userspace memory, allocated via the kernel's `SecretAllocator`.

The `SecretAllocator`:

```rust
pub struct SecretAllocator {
    arena: MlockedArena,
}

impl SecretAllocator {
    /// Allocate `size` bytes in mlocked, MADV_DONTDUMP memory.
    /// Memory is zeroed before being handed to the caller.
    pub fn allocate_secret_buffer(&self, size: usize) -> Result<SecretBuf, AllocError> {
        // 1. mmap(MAP_ANONYMOUS | MAP_PRIVATE)
        // 2. mlock() — refuses to write to swap, refuses to be paged out
        // 3. madvise(MADV_DONTDUMP) — excluded from core dumps
        // 4. madvise(MADV_DONTFORK) — child processes cannot inherit
        // 5. mprotect(PROT_READ | PROT_WRITE) — no execute
        // 6. zero the region
        // 7. wrap in SecretBuf with Drop impl that zeroes + munlocks + munmaps
    }
}

pub struct SecretBuf {
    ptr: NonNull<u8>,
    len: usize,
    _phantom: PhantomData<*mut ()>, // !Send !Sync by default
}

impl Drop for SecretBuf {
    fn drop(&mut self) {
        // 1. zero the contents (using zeroize::Zeroize, compiler-fence to
        //    prevent dead-store elimination)
        // 2. munlock
        // 3. munmap
    }
}
```

### Process-level hardening

LFI must launch with these protections active:

- `MCL_FUTURE` mlock applied at process start so all future allocations are pinned (where the kernel allows it without root, otherwise documented as a deployment requirement).
- `prctl(PR_SET_DUMPABLE, 0)` — process cannot be ptraced or core-dumped.
- `RLIMIT_CORE` set to 0.
- `prctl(PR_SET_NO_NEW_PRIVS)` — no setuid escalation possible from any descendent.
- Swap should be off, or `cryptsetup`-encrypted with a key that does not survive reboot.
- Hibernation should be disabled at the OS level, or hibernation file encrypted with the same constraint.

These are enforced via a `PreflightCheck` at LFI startup; if any check fails and the operator policy requires it, LFI refuses to start.

---

## Subsystem 3 — Encrypted computation

For operations that cannot be performed without the plaintext, three execution backends, selected per operation by the policy's `Confidentiality::Required` level:

### Backend A — TEE-resident execution (default when hardware available)

The operation runs inside the SGX enclave, SEV-SNP VM, TDX VM, TrustZone, or Secure Enclave. Plaintext exists only in hardware-encrypted memory. The host OS cannot read it even with root.

LFI ships with a minimal "broker enclave" image — a stripped-down Rust binary that does only the operations brokers need (sign, decrypt, derive, authenticate). The enclave image is reproducibly built; its measurement hash is committed at LFI install time; subsequent operations verify the measurement before sending sealed data.

For SGX: build with the `enarx` toolchain or the `sgx-rs` SDK. For SEV-SNP: deploy LFI itself inside the confidential VM. For TrustZone (PlausiDenOS): the broker is a TA (Trusted Application) loaded via OP-TEE.

Cost: 5-20% slowdown vs unprotected. Acceptable for almost all operations.

### Backend B — Fully Homomorphic Encryption (selective)

For operations where the secret must not even be visible to the local TEE — e.g., a query to a remote service that should learn nothing about the input — the kernel routes through TFHE-rs (Zama, IND-CPA^D secure as of 2024).

```rust
pub trait FheOperation {
    type Input;
    type Output;
    fn evaluate_encrypted(
        &self,
        input: FheCiphertext<Self::Input>,
        params: &FheParams,
    ) -> Result<FheCiphertext<Self::Output>, FheError>;
}
```

Realistic LFI use cases:
- Querying a remote knowledge base where the query terms are sensitive
- Federated mesh queries where the querying node should learn answer-without-question
- Auditor verifying a computation result without learning the inputs

Cost: 10⁴× slowdown for general computation. Programmable bootstrap (TFHE) is closer to 10²–10³× for boolean circuits. Reserve for the small set of operations where the secret cannot be exposed to the API endpoint.

### Backend C — Multi-Party Computation (mesh-shared secrets)

When a secret is split across mesh nodes (e.g., Sacred.Vote ballot key shared between you, Tim, and the DLD), no single node should be able to reconstruct it. The kernel uses Shamir's Secret Sharing for storage (`sharks` crate) and BGW or SPDZ-style MPC for computation (`mpc-core` or build on `arkworks`).

```rust
pub struct SharedSecret<T: Sensitive> {
    /// This node's share. Cannot reconstruct T alone.
    share: SecretShare,
    /// Threshold: how many shares needed to reconstruct or compute.
    threshold: u8,
    /// Total shares distributed.
    total: u8,
    /// Identities of share-holders.
    holders: Vec<PeerIdentity>,
}

impl<T: Sensitive> SharedSecret<T> {
    /// Compute on the secret without reconstructing it.
    /// Requires `threshold` participating peers to be online.
    pub fn mpc_compute<F, R>(
        &self,
        broker: &MeshBroker,
        op: MpcOperation,
        f: F,
    ) -> Result<R, MpcError>
    where F: FnOnce(MpcSession) -> R, R: NonSensitive
    {
        // Coordinate with peers via libp2p
        // Run the MPC protocol
        // Return result; each peer sees only their own share + protocol messages
    }
}
```

### Selection policy

```rust
pub enum Confidentiality {
    /// Plaintext briefly visible inside use_within in mlocked memory.
    /// Default for most operations.
    Standard,
    /// Operation must execute inside the TEE/enclave.
    /// Plaintext never visible to host kernel.
    TeeRequired,
    /// Operation must execute in FHE.
    /// Plaintext never reconstructed in memory.
    HomomorphicOnly,
    /// Operation must execute via MPC across N-of-M peers.
    /// Secret is share-distributed; never reassembled.
    SecretShared { threshold: u8, total: u8 },
}
```

Per-secret policy attaches a `Confidentiality` level at seal time. Operations against the secret must respect this level. Attempting `Standard` execution against a secret marked `TeeRequired` is a kernel error.

---

## Subsystem 4 — Network egress

### The typed channel API

All network egress in LFI goes through `SecureChannel`:

```rust
pub trait SecureChannel: Send {
    /// Verified peer. Identity proven via TLS cert + cert-pinning + CT log.
    fn peer_identity(&self) -> &PeerIdentity;

    /// TLS protocol version. Kernel refuses anything below 1.3.
    fn protocol(&self) -> Protocol;

    /// Send sealed payload. Recipient unseals with their key.
    fn send_sealed<T: Sensitive>(
        &mut self,
        sealed: Sealed<T>,
    ) -> Result<(), ChannelError>;

    /// Send plaintext that has been confirmed scrub-clean by the egress scanner.
    fn send_scanned(
        &mut self,
        payload: ScrubBuffer,
    ) -> Result<(), ChannelError>;
}

// CRITICAL: there is no `send_plaintext` method. By design.
// Plaintext send requires either sealing (recipient unseals) or
// scanning (egress scanner confirms no sensitive data present).
```

### Concrete channel implementations

```rust
pub struct RustlsChannel {
    inner: rustls::ClientConnection,
    pinned_cert: Option<CertFingerprint>,
    ct_verifier: CtLogVerifier,
}

impl RustlsChannel {
    pub fn connect(
        endpoint: &Endpoint,
        config: &ChannelConfig,
    ) -> Result<Self, ChannelError> {
        // 1. rustls config: TLS 1.3 only, ECDHE+ChaCha20-Poly1305 or AES-GCM
        // 2. Certificate transparency log verification
        // 3. HSTS preload list check
        // 4. Cert pinning if endpoint is in known set
        // 5. Refuse if any check fails
    }
}

pub struct LibP2pChannel { /* QUIC + Noise via libp2p */ }
pub struct WireGuardChannel { /* for mesh peer comms */ }
pub struct SshChannel { /* for remote ops, e.g., the Pineapple */ }
```

### Outbound firewall integration

Beyond type-system enforcement, the kernel collaborates with OS-level controls:

```rust
pub struct EgressFirewall {
    /// Only the kernel's egress subsystem is allowed to make outbound
    /// connections from the LFI process. Enforced via:
    ///   - Linux: cgroup network classifier + nftables rules
    ///   - Or: seccomp filter restricting connect() to broker UID
    ///   - Or: SELinux/AppArmor policy
    enforced_by: EnforcementMechanism,
}
```

If a misbehaving module or compromised dependency tries to bypass the kernel and call `connect()` directly, the OS kills the syscall. Defense in depth.

### LLM API call wrapping — preventing prompts-with-secrets

This is the specific failure mode that triggered this entire design. The pattern:

```rust
pub struct LlmApiBroker {
    channel: RustlsChannel,
    prompt_builder: SecuredPromptBuilder,
    response_handler: SecuredResponseHandler,
}

impl LlmApiBroker {
    pub fn complete(
        &mut self,
        template: PromptTemplate,
        context: PromptContext,
    ) -> Result<Sealed<LlmResponse>, BrokerError> {
        // Step 1: Build prompt with capability substitution.
        // Any {api_key}, {password}, {secret} markers in the template
        // are NOT substituted with plaintext. They become opaque tokens
        // like <SECRET_REF:cap_7f3a9b2c> visible to LFI but not sent in
        // the prompt body.
        let prompt = self.prompt_builder.build(template, context)?;

        // Step 2: Strip markers from the outbound prompt body.
        // The LLM sees the prompt with secrets stripped/redacted.
        let outbound_body = prompt.scrub_for_egress()?;

        // Step 3: Attach actual secrets only as out-of-band auth.
        // E.g., Anthropic API key goes in the Authorization header
        // via use_within, never in the prompt body.
        let request = self.assemble_request(outbound_body, prompt.auth_capability())?;

        // Step 4: Send over verified TLS 1.3 channel.
        let raw_response = self.channel.send_request(request)?;

        // Step 5: Treat response as untrusted input.
        // Run egress scanner backwards on incoming text — if the response
        // contains anything that looks like one of LFI's known secrets,
        // flag it (the LLM may have echoed back data from its training set
        // that happens to match one of our secrets, or an attacker may have
        // compromised the LLM provider). Either way, the response is sealed
        // before it enters LFI's reasoning context.
        let sealed_response = self.response_handler.seal_and_validate(raw_response)?;
        Ok(sealed_response)
    }
}
```

The structural property: **the LLM never sees a plaintext secret in the prompt**. It cannot echo what it was never given. It cannot remember what it never had in its context. It cannot leak across sessions material it was never exposed to.

For operations where the LLM genuinely needs to reason about a secret (e.g., "is this password strong enough?"), the broker offers a `derived_property` API: LFI computes the derived property locally (entropy, length, pattern presence) and sends the property, not the secret, to the LLM.

---

## Subsystem 5 — Egress scanner

### Architecture

```rust
pub struct EgressScanner {
    classifiers: Vec<Box<dyn SecretClassifier + Send + Sync>>,
    redaction_policy: RedactionPolicy,
    vault_membership_check: Option<Arc<VaultMembershipChecker>>,
}

pub trait SecretClassifier: Send + Sync {
    fn name(&self) -> &str;
    fn scan(&self, payload: &[u8]) -> Vec<DetectedSecret>;
}

pub struct DetectedSecret {
    pub byte_range: Range<usize>,
    pub kind: SecretKind,
    pub confidence: Confidence,
    pub classifier: &'static str,
}

pub enum SecretKind {
    AwsAccessKey,
    AwsSecretAccessKey,
    GitHubToken,
    GitlabToken,
    NpmToken,
    SshPrivateKey,
    GpgPrivateKey,
    JwtToken,
    OpenAiApiKey,
    AnthropicApiKey,
    CreditCard,
    SocialSecurityNumber,
    EmailAddress,
    PhoneNumber,
    IpAddress,
    HighEntropyString { entropy_bits_per_byte: f32 },
    KnownVaultSecret { capability_id: CapabilityId },
    Custom { tag: &'static str },
}
```

### Concrete classifiers (build order)

**Tier 1 — Pattern matchers** (fast, high precision):
- Port the gitleaks rule set (~200 patterns covering AWS, GCP, Azure, GitHub, GitLab, Slack, Stripe, Twilio, etc.)
- Add LFI-specific patterns: Pineapple management AP keys (`option key '<wpa-psk>'`), Sacred.Vote DLD tokens (when defined), PlausiDen vault format

**Tier 2 — Entropy detector** (medium speed, medium precision):
- Sliding window Shannon entropy over the payload
- Strings of length ≥20 with H ≥ 4.5 bits/byte are likely secrets
- Whitelist common high-entropy non-secrets: Base64-encoded images (start with `iVBORw0KGgo` etc.), UUIDs (specific format), Git SHAs (40 hex chars in commit context)

**Tier 3 — NER for PII** (slow, leverages existing LFI infra):
- GLiNER ONNX via `ort` — already in LFI's ingestion stack
- Detects names, addresses, phone numbers, emails, credit cards in natural-language context
- ~50 docs/sec/core CPU; reserve for outbound text where speed isn't critical

**Tier 4 — Vault membership check** (slow, highest precision):
- For each high-entropy string in the payload, compute its hash with the same parameters as vault sealing
- Query the vault: does this hash match a known sealed secret?
- If match: definitely a leaking secret; flag with `KnownVaultSecret` classification
- This catches secrets that don't match any pattern but are known to LFI

### Redaction modes

```rust
pub enum RedactionMode {
    /// Replace with a fixed marker.
    /// "[REDACTED:api_key]"
    FixedMarker,

    /// Replace with a stable hash-derived pseudonym.
    /// Same input always produces same pseudonym.
    /// Useful when downstream systems need to correlate without seeing.
    /// "[USER_a3f9c2]"
    Pseudonym { salt: SaltRef },

    /// Replace with a fresh random token.
    /// No correlation possible across redactions.
    /// "[TOKEN_e8f1d9]"
    FreshRandom,

    /// Refuse the egress entirely.
    Refuse,
}
```

### Output

```rust
pub struct ScrubBuffer {
    payload: Vec<u8>,
    redactions: Vec<Redaction>,
    scanner_version: ScannerVersion,
    scan_metadata_hash: Blake3Hash,
}

pub enum ScrubBuffer {
    Clean(Vec<u8>),
    Redacted { payload: Vec<u8>, redactions: Vec<Redaction> },
    Refused { reason: RefuseReason },
}
```

The egress channel only accepts `ScrubBuffer::Clean` or `ScrubBuffer::Redacted`. `Refused` propagates up as an error.

---

## Subsystem 6 — Chat history scrubber

### Daemon architecture

```rust
pub struct ChatScrubber {
    sources: Vec<Box<dyn ConversationSource>>,
    scanner: Arc<EgressScanner>,
    sink: Box<dyn ScrubbedSink>,
    rotation_dispatcher: Arc<RotationDispatcher>,
}

pub trait ConversationSource: Send + Sync {
    fn name(&self) -> &str;
    fn iter_messages(&self) -> Box<dyn Iterator<Item = Message> + '_>;
    fn supports_replace(&self) -> bool;
    fn replace_message(&mut self, id: MessageId, new: Message) -> Result<(), ScrubError>;
    fn supports_delete(&self) -> bool;
    fn delete_message(&mut self, id: MessageId) -> Result<(), ScrubError>;
}
```

### Source adapters (build order)

1. **Local shell history** — `~/.bash_history`, `~/.zsh_history`, `~/.local/share/zsh/history`. Read, scan, rewrite in place. Scrubber must handle the file's locking/append semantics so an active shell doesn't lose history.

2. **Terminal scrollback** — for terminals with persistent scrollback (tmux, screen, kitty, alacritty). Each terminal's scrollback format differs; adapter per terminal.

3. **Local LFI conversation logs** — LFI's own stored conversations with users. Direct rewrite supported.

4. **Browser history** — via WebExtension API for Chromium-based and Firefox browsers. Read-only scan, surface findings to user with "open this URL to delete" actions where deletion isn't programmatically supported.

5. **Email** — IMAP/JMAP via authenticated connection. Scan messages, surface findings; user decides whether to delete or move to a sealed folder.

6. **Anthropic chat history** — request transcript export via `claude.ai` data export, scan locally, save scrubbed copy, request deletion of original. Anthropic's deletion SLA is 30 days per their privacy policy, so plaintext exposure has a bounded lifetime.

7. **Slack / Discord / Teams** — via export APIs. Scan, surface findings, support bulk delete via API where available.

### Scrub workflow per detected secret

1. Identify the `DetectedSecret` and its source location
2. Check vault: does this secret correspond to a known `Sealed<T>`?
   - If yes: secret is **burned** (it appeared in plaintext somewhere). Trigger automatic rotation.
   - If no: prompt user — "Detected what looks like an unmanaged secret in [source] at [date/time]. Options: (a) seal it now under a new capability, (b) confirm it's not a secret (false positive), (c) just redact and forget."
3. Execute the chosen action:
   - **Seal**: read the secret, create new `Sealed<T>` with appropriate classification and policy, replace the source occurrence with a reference marker
   - **Redact**: replace the source occurrence with the configured `RedactionMode` output, no vault entry created
   - **False positive**: add the specific value to the false-positive allowlist for future scans
4. Update audit log with: source, detection time, action taken, outcome
5. If rotation triggered: dispatch to `RotationDispatcher` for the credential type

### Rotation dispatcher

```rust
pub struct RotationDispatcher {
    handlers: HashMap<SecretKind, Box<dyn RotationHandler>>,
}

pub trait RotationHandler: Send + Sync {
    /// Generate new credential and update at the destination service.
    /// On success, returns the new sealed credential.
    /// On failure (no API support), returns ManualRotationRequired with instructions.
    fn rotate(
        &self,
        old: &Sealed<dyn Sensitive>,
        broker: &Broker,
    ) -> Result<RotationOutcome, RotationError>;
}
```

Concrete handlers for: AWS IAM access keys (rotate via AWS API), GitHub PATs (regenerate via GH API), GitHub fine-grained tokens, GitLab tokens, npm tokens, OpenAI API keys (manual + email), Anthropic API keys (manual + email), SSH keys (generate new keypair + push to authorized_keys + remove old), GPG subkeys (`sequoia-pgp` rotation).

For services without rotation APIs, handler emits a `ManualRotationRequired` task with step-by-step instructions, and surfaces it in the LFI dashboard.

---

## Subsystem 7 — Audit kernel

### Append-only Merkle log

```rust
pub struct AuditLog {
    storage: Arc<dyn AppendOnlyStorage>,
    sequence: AtomicU64,
    head_hash: RwLock<Blake3Hash>,
    signing_key: TpmAttestedKey,
}

pub struct AuditEntry {
    pub sequence: u64,
    pub timestamp: Timestamp, // monotonic + wall-clock
    pub operation: OperationKind,
    pub capability_id: CapabilityId,
    pub policy_hash: Blake3Hash,
    pub actor: ActorIdentity,
    pub outcome: OperationOutcome,
    /// Hash of operation-specific metadata. NEVER plaintext.
    pub operation_metadata_hash: Blake3Hash,
    /// Merkle chain
    pub prev_entry_hash: Blake3Hash,
    pub self_hash: Blake3Hash,
    /// Optional ZK proof of operation correctness.
    pub correctness_proof: Option<NovaProof>,
}

pub enum ActorIdentity {
    Human { user_id: UserId },
    Ai { agent_id: AgentId, model_version: ModelVersion },
    MeshPeer { peer: PeerIdentity },
    Cron { task_id: TaskId },
    System { component: SystemComponent },
}
```

### Tamper evidence

Every entry hashes its predecessor. Periodic checkpoints (every 1000 entries or every hour, whichever is sooner) get signed by the TPM-attested signing key. Optionally publish checkpoint hashes to:

- **Sigstore Rekor** transparency log (centralized but widely trusted)
- **Your own libp2p-published log** with mesh peer co-signing (fully sovereign)

External auditors verify (a) chain integrity from genesis to current head, (b) checkpoint signatures, (c) checkpoint inclusion in the transparency log. They cannot read operation contents (only metadata hashes), but they can prove no entries were silently deleted or altered.

### Query interface

```rust
pub trait AuditQuery {
    /// All operations against capability X in time range.
    fn operations_for_capability(
        &self,
        cap: CapabilityId,
        range: TimeRange,
    ) -> Result<Vec<AuditEntry>, QueryError>;

    /// All operations by actor X.
    fn operations_by_actor(
        &self,
        actor: &ActorIdentity,
        range: TimeRange,
    ) -> Result<Vec<AuditEntry>, QueryError>;

    /// Did any unsealing of capability X occur outside policy P's bounds?
    fn policy_violations(
        &self,
        cap: CapabilityId,
        policy: PolicyRef,
    ) -> Result<Vec<AuditEntry>, QueryError>;

    /// Verify the chain from genesis to current head.
    fn verify_chain(&self) -> Result<ChainVerification, VerificationError>;

    /// Prove inclusion of entry X without revealing other entries.
    fn inclusion_proof(&self, entry_id: u64) -> Result<MerkleInclusionProof, QueryError>;
}
```

---

## Subsystem 8 — Zero-knowledge operation proofs

### Proof targets

The kernel can attach a Nova folding proof (Microsoft `nova-snark` crate) to any operation. Three concrete proof targets:

**Proof 1 — Authentication without password reveal.**
For services supporting OPAQUE or similar PAKE protocols: prove "I know x such that hash(x) = stored_hash" without sending x. Server-side cooperation required; rare in 2026 but increasing.

**Proof 2 — Correct computation on sealed data.**
When LFI's reasoning core derives a conclusion from sealed input, generate a Nova proof that the conclusion follows from a valid input and the stated computation. Verifier learns the computation was correct without seeing input or intermediate state. Integrates with `DerivationTrace` — every traced derivation can carry an optional correctness proof.

**Proof 3 — Policy compliance.**
Every secret operation can carry a SNARK proof of "this operation satisfied its attached policy" — capability matched, consent obtained, use count within bounds, time window valid. Verifier confirms compliance without seeing the policy details.

### Implementation

```rust
pub struct OperationProofGenerator {
    backend: ProofBackend,
}

pub enum ProofBackend {
    /// Nova folding (recommended for IVC over operation chains)
    Nova { setup: NovaSetup },
    /// Halo2 (mature, good for fixed circuits)
    Halo2 { setup: Halo2Setup },
    /// SP1 zkVM (general purpose, slower but flexible)
    Sp1 { setup: Sp1Setup },
}

impl OperationProofGenerator {
    pub fn prove<O: ProvableOperation>(
        &self,
        op: O,
        witness: O::Witness,
    ) -> Result<O::Proof, ProofError> {
        // Cost: Nova folding ~ms-to-seconds per step depending on circuit
        // Halo2 ~ seconds per proof
        // SP1 ~ seconds for simple programs, longer for complex
    }
}
```

Proofs are optional — the kernel works fully without them, with audit log providing tamper-evidence. Enabling proofs adds verifiability for environments where pure audit isn't enough (regulated industries, multi-party trust scenarios, mesh consensus on operations).

---

## Integration with existing LFI subsystems

### HDC layer (`lfi_vsa_core`)

`Sealed<T>` types encode as bipolar hypervectors via role-binding:

```
H_sealed = R_capability ⊗ E_capability_id
         + R_classification ⊗ E_classification
         + R_policy ⊗ E_policy_hash
         + R_provenance ⊗ E_provenance_hash
```

The hypervector is safe to bundle into prototypes, query for similarity, reason over via cognition modules. The plaintext is never encoded; only its identity, type, and policy. LFI can have facts like "SecretRef-7f3a9b2c was used to authenticate to PineappleHost at time T, producing wireless config dump W" without any plaintext entering the fact graph.

### Crypto epistemology (`reasoning_provenance.rs`)

Every sealed-data operation produces a `TracedDerivation` whose committed bytes are:

```
Blake3(operation_kind || capability_id || policy_hash || timestamp || actor_id)
```

The reveal proves the operation occurred against a specific capability under a specific policy without revealing the secret itself. Adversarial reclamation tests confirm "yes, this audit log faithfully records what happened" without the auditor seeing plaintexts.

### PSL Supervisor

Add the security axiom:

```
∀ op : SecretOperation .
    permitted(op) ⟺
        capability_validates(op) ∧
        policy_allows(op) ∧
        consent_satisfied(op) ∧
        confidentiality_level_supported(op)
```

The supervisor refuses to dispatch any operation that doesn't satisfy this axiom, even if the reasoning core asks for it. This is the security kernel — it cannot be bypassed by clever prompting because it constrains the action space, not the reasoning space.

### Cognition modules

Causal reasoning (`cognition/causal.rs`): operations against secrets create causal edges:
- `(Operation X used Capability Y) → (Service Z authenticated)`
- `(Capability Y exposed in chat) → (Capability Y rotated)`

Metacognitive profiler: tracks the operator's secret-handling patterns and surfaces hygiene improvements.

### Defensive AI (`intelligence/defensive_ai.rs`)

Consumes the audit log to detect anomalous operation patterns: unusual access frequency, unusual time-of-day, unusual actor, unusual operation kind for capability class. Flags for review.

---

## Sprint plan

### Sprint A — Type system foundation (3 weeks)

**Deliverables:**
- `lfi-conf-kernel` crate scaffold
- `Sensitive` trait, `Sealed<T>`, `NonSensitive` marker, `SealedMarker` private trait
- `Classification` enum with the full taxonomy
- `Capability`, `CapabilityRef`, `Policy`, `PolicyRef` types
- `Broker` skeleton with `unseal_and_use` API
- `derive(Sensitive)` proc macro with `compile_error!` diagnostics for forbidden trait combinations
- Basic audit log with append-only storage and Blake3 chain
- Test: every existing LFI module that handles secrets must compile against the kernel; force every existing secret-handling site through `Sealed<T>` and `use_within`

**Acceptance criteria:**
- 100% of existing LFI secret-handling code routes through the kernel
- `cargo build` fails with helpful diagnostic if any module attempts forbidden patterns
- Unit tests cover the type-discipline guarantees (compile-fail tests via `trybuild`)

### Sprint B — Memory protection (2 weeks)

**Deliverables:**
- `KeyStore` enum with detection logic for all listed backends
- `LockedKernelKeyring` and `LockedUserspace` backends fully implemented (always available)
- `TpmSealed` backend implementation (most modern PCs have TPM)
- `SecretAllocator` with mlock + MADV_DONTDUMP + zeroize
- `SecretBuf` with Drop-time scrubbing
- Process-level hardening: `prctl(PR_SET_DUMPABLE, 0)`, `RLIMIT_CORE = 0`, `MCL_FUTURE`
- Preflight check at LFI startup; refuses to start if hardware policy not met
- Stub interfaces for SGX, SEV-SNP, TDX, TrustZone, SEP — implementation deferred to platform-specific sprints

**Acceptance criteria:**
- LFI process refuses to be ptraced
- Core dumps don't contain secret material (test via deliberate crash + dump inspection)
- Swap is verifiably never written with plaintext (test under swap-pressure)
- Plaintext lifetime in `use_within` is bounded by closure execution

### Sprint C — Network egress (3 weeks)

**Deliverables:**
- `SecureChannel` trait, `RustlsChannel`, `LibP2pChannel`, `WireGuardChannel`, `SshChannel`
- TLS 1.3-only enforcement; rejection of weaker versions
- Certificate transparency verification, HSTS preload list, cert pinning for known endpoints
- `EgressFirewall` integration via cgroups + nftables (Linux)
- `LlmApiBroker` with prompt-builder secret substitution
- `SecuredPromptBuilder` and `SecuredResponseHandler`
- Integration tests against Anthropic API, Hak5 update server, Vaultwarden API

**Acceptance criteria:**
- All outbound HTTP from LFI uses TLS 1.3
- Plaintext HTTP outbound is structurally impossible (refused at multiple layers)
- LLM API calls never contain plaintext secrets in request body
- Test: deliberately attempt to send a known sealed secret in a prompt; kernel must refuse

### Sprint D — Egress scanner + chat scrubber (2 weeks)

**Deliverables:**
- `EgressScanner` with all four classifier tiers
- gitleaks pattern set ported to Rust regexes
- Entropy detector with configurable thresholds and whitelisting
- GLiNER ONNX integration for PII (reuse from existing LFI ingestion pipeline)
- `VaultMembershipChecker` for known-secret detection
- `ChatScrubber` daemon with shell-history, terminal-scrollback, and LFI-conversation source adapters
- `RotationDispatcher` with handlers for AWS IAM, GitHub PAT, GitLab token, npm token, SSH key, GPG subkey
- Manual rotation UI for unsupported services

**Acceptance criteria:**
- Scrubber processes a corpus of test data with known secrets and reaches ≥95% detection rate
- False positive rate <1% on a corpus of known non-secrets
- Rotation handlers tested against live AWS/GitHub/GitLab APIs (with throwaway credentials)
- Test: feed the scrubber a copy of the user's actual chat history with known leaked credentials; verify all are detected and rotation triggered

### Sprint E — Encrypted compute (4-6 weeks, optional)

**Deliverables:**
- TEE adapter for whichever hardware is available on user's deployment (likely TPM + LockedKernelKeyring as baseline; SGX/SEV when on capable hardware)
- TFHE-rs integration for the FHE backend
- Shamir secret sharing for `SharedSecret<T>`
- MPC framework via `mpc-core` or `arkworks` for mesh-shared operations
- `Confidentiality` policy enforcement at broker dispatch time

**Acceptance criteria:**
- Operations with `Confidentiality::TeeRequired` execute inside a TEE or fail at policy check
- FHE backend correctly evaluates a reference circuit (e.g., AES round) with measurable but acceptable performance
- MPC backend handles a 3-party operation with one byzantine participant correctly

### Sprint F — ZK provenance proofs (3 weeks, optional)

**Deliverables:**
- `OperationProofGenerator` with Nova backend
- Proof attachment to audit log entries
- Verifier service for external auditors
- Integration with `DerivationTrace` so traced reasoning can carry optional correctness proofs

**Acceptance criteria:**
- Nova folding proofs generate and verify for the standard operation set
- Proof generation overhead <5 seconds per operation on commodity hardware
- External verifier validates proof + audit log + checkpoint signature without access to operation contents

### Sprint G — PlausiDenOS integration (deferred to mobile build)

**Deliverables:**
- TrustZone TA implementation of the broker enclave
- Secure Enclave integration on Apple platforms
- seL4 capability-based memory protection for Sealed<T>
- Hardware-rooted key storage on mobile silicon

This sprint is deferred until the PlausiDenOS work begins on the Pixel 10 Pro XL.

---

## Operational guarantees

After Sprints A-D are complete, LFI provides these guarantees to operators:

**G1.** No sensitive data appears in plaintext in process memory outside an active `use_within` closure.

**G2.** No sensitive data appears in plaintext in any log, trace, telemetry, error message, or debug dump emitted by LFI or its subsystems.

**G3.** No sensitive data is transmitted over any network channel except (a) sealed for a recipient holding the unseal capability, or (b) after egress-scanner confirmation that no sensitive data is present.

**G4.** No LLM prompt sent to any external API contains plaintext credentials, API keys, or operator-classified PII.

**G5.** Any sensitive data inadvertently present in scrubbable history (shell, terminal, chat, LFI conversation log) is automatically detected, redacted, and (if matching a known vault entry) triggers rotation of the corresponding secret.

**G6.** Every operation against sensitive data is recorded in a Merkle-chained audit log with cryptographic tamper evidence.

After Sprints E-F are also complete:

**G7.** Operations marked `TeeRequired` execute inside hardware-encrypted memory invisible to the host kernel.

**G8.** Operations on `SharedSecret<T>` execute via MPC without any single party holding the complete secret.

**G9.** Operation correctness can be cryptographically proven to external auditors without revealing operation inputs or intermediate state.

These are structural guarantees enforced by the type system, the kernel architecture, and the OS-level integrations — not best-effort policies enforced by code review.

---

## Strategic positioning

The combination of these eight subsystems doesn't exist as an integrated stack anywhere as of April 2026. Pieces exist:

- **HashiCorp Vault** has the storage but not the type system, computation, or chat-scrubbing layers
- **AWS Nitro Enclaves** have the TEE but no integration with reasoning provenance or audit
- **Apple Private Cloud Compute** is the closest commercial analog but is iOS-internal and not a substrate for sovereign systems
- **Microsoft Confidential Inference** is similar but cloud-bound
- **The MCP ecosystem** is still doing "paste your API key into config"

What LFI uniquely provides by building this on its existing crypto-epistemology + provenance + HDC + PSL substrate: **a sovereign AI system where the confidentiality property is structural, not aspirational**. Operators can deploy LFI with sensitive data and obtain formal guarantees about what can leak, where, when, and to whom.

This unlocks deployment in environments currently categorically excluded from AI: healthcare (HIPAA), defense (classified data handling), finance (PCI-DSS, SOX), government (FedRAMP, IL5/IL6), legal (privilege protection), and any environment subject to GDPR/CCPA where current AI systems cannot demonstrate compliance.

The investment is real — Sprints A-D are roughly 10 weeks of focused engineering, plus E-F if pursued — but produces a moat at the architectural level that competitors cannot easily replicate without rebuilding their own substrates from scratch.

---

## Open questions for operator decision

1. **Hardware key store selection on the Hetzner EX44.** Confirm whether SEV-SNP is available; if not, baseline is TPM 2.0 sealed against PCR state with `LockedKernelKeyring` fallback.

2. **Mobile target priority.** PlausiDenOS work is planned for Pixel 10 Pro XL — should TrustZone integration be Sprint G or accelerated to parallel-track with Sprint B?

3. **Mesh secret sharing scope.** Does Sacred.Vote require MPC-based shared secrets between you, Tim, and the DLD in v1, or is single-party operation sufficient until Phase 2?

4. **External transparency log.** Self-hosted via libp2p (full sovereignty) or piggyback on Sigstore Rekor (less infrastructure, less sovereignty)? Hybrid is possible.

5. **Optional sprint scheduling.** Sprints E and F deliver capabilities the core (A-D) doesn't have. Schedule them parallel-track with mesh/PlausiDenOS work, or sequential after the core ships?

These are decisions for the operator; the design accommodates either path on each.

---

## Final notes for Claude Code

Build A through D as a strict sequence — each depends on the prior. Build the type discipline (Sprint A) first and most carefully; the rest of the kernel's properties depend on Rust's type system enforcing invariants that the runtime alone cannot.

Test every sprint against the existing LFI codebase. Each sprint's acceptance criterion includes "every existing LFI module that handles secrets compiles and operates correctly against the new kernel." This forces the kernel to be usable, not just elegant.

Do not let scaffolding accumulate. If you write a Python or shell prototype to validate an approach, mark it explicitly as scaffolding and rewrite in Rust before merging. The kernel is the foundation of LFI's confidentiality story; it cannot ship with non-Rust components in production paths.

When in doubt about a design choice, default to the more restrictive option. It is much easier to relax a kernel-level guarantee later than to tighten one after it has been promised to operators.
