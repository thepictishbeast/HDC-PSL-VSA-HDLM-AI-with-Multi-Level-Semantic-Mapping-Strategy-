# PlausiDen Secrets Design

**Document version:** 1.0
**Author context:** Handoff from planning conversation to Claude Code for implementation
**Repository target:** new top-level crate `plausiden-secrets` in the LFI workspace
**Depends on:** `lfi-conf-kernel` (see LFI-CONFIDENTIALITY-KERNEL-DESIGN.md)
**Status:** Design specification, ready for sprint-by-sprint implementation

---

## Mission statement

The Confidentiality Kernel provides the runtime substrate — `Sealed<T>`, encrypted memory, brokered access, audit chain. `plausiden-secrets` is the **application-layer service** that operators and AI agents actually interact with: a typed, capability-based, policy-enforced secret manager whose interface is designed for AI agents that should be able to use credentials at full capability without ever holding raw secret material.

Where existing secret managers (Vaultwarden, 1Password, Bitwarden, HashiCorp Vault) treat secrets as strings to be retrieved on demand, `plausiden-secrets` treats secrets as **typed capabilities to be exercised through a broker**. The user types each password exactly once, at storage time. After that, the AI exercises capabilities under operator-defined policies, and the operator gets cryptographic audit trails of every exercise.

The end state is the workflow you described: type a password once, set a policy ("Claude Code can use this for SSH to the Pineapple, 13 times max, within 4 hours, with passive notification"), and never type it again until you choose to rotate.

---

## Design principles

**Principle 1 — Capabilities, not strings.** Every secret is wrapped as a `Capability` with an unforgeable type signature describing what it can be used for. SSH credentials cannot be redeemed as API keys. WebAuth cookies cannot be redeemed as GPG signing operations. Type confusion is impossible.

**Principle 2 — Policy at storage time, not access time.** When a secret is first stored, its policy is attached. The policy specifies max uses, time windows, consent requirements, audit demands. This eliminates the "what should this secret be used for" ambiguity that plagues conventional secret managers.

**Principle 3 — Brokers do the work.** The AI agent never receives plaintext. It hands a `CapabilityRef` to a broker along with the operation it wants performed. The broker validates, obtains consent if required, performs the operation, and returns the result. Plaintext exists for the broker's operation duration only.

**Principle 4 — Delegation via macaroons.** Capabilities can be delegated across mesh nodes, AI instances, or time windows with cryptographic restriction. A master capability can issue a derived capability scoped to "next 4 hours, this specific AI session, max 13 uses." Revoking the master kills all derivations.

**Principle 5 — PII is a first-class capability class.** Personally identifiable information receives stricter defaults aligned with GDPR/CCPA: legal basis required, retention dates enforced, breach notification triggers built in. Right-to-erasure is a single operation that revokes all references.

---

## Capability taxonomy

The set of capability classes is closed. Every secret in the vault belongs to exactly one class. The class determines:
- What broker handles redemptions
- What policies are sensible defaults
- What audit fields are required
- What rotation handler applies

```rust
pub enum Capability {
    /// SSH credentials. Redeemed via SshBroker for connections to specific hosts.
    SshLogin {
        host: HostPattern,
        user: String,
        auth_method: SshAuthMethod, // password, key, certificate
    },

    /// Web authentication. Redeemed via WebAuthBroker for HTTP(S) sessions.
    WebAuth {
        domain: DomainPattern,
        scheme: AuthScheme, // basic, bearer, cookie, oauth2
        scope: Vec<OAuthScope>, // for OAuth2
    },

    /// API keys. Redeemed via ApiBroker for HTTP API calls.
    ApiKey {
        service: ServiceId,
        scope: Vec<ApiScope>,
        rate_limit: Option<RateLimit>,
    },

    /// GPG signing keys. Redeemed via GpgBroker for sign/encrypt/decrypt operations.
    GpgSigning {
        key_id: KeyId,
        capabilities: GpgCapabilities, // sign, encrypt, certify, authenticate
    },

    /// Disk encryption. Redeemed via DiskBroker for mount/unmount operations.
    DiskEncryption {
        volume: VolumeId,
        cipher: CipherSuite,
    },

    /// Encrypted communications with a specific peer (WireGuard, Signal-protocol).
    EncryptedComms {
        peer: PeerIdentity,
        protocol: CommsProtocol,
    },

    /// PII about a specific data subject.
    Pii {
        subject: SubjectId,
        field: PiiField,
        legal_basis: LegalBasis,
        retention_until: Timestamp,
    },

    /// Biometric template. Always non-revocable; treat as super-PII.
    Biometric {
        subject: SubjectId,
        modality: BiometricModality, // face, voice, fingerprint, iris
        template_format: TemplateFormat,
    },

    /// Vault master keys, KEKs, signing keys for the audit log.
    /// These are bootstrap material; access is heavily restricted.
    VaultMaster {
        purpose: MasterPurpose,
    },

    /// Single-use tokens: nonces, OTPs, ephemeral session keys.
    Ephemeral {
        purpose: EphemeralPurpose,
        consumed_on_use: bool,
    },

    /// Mesh-shared secret. Held collectively across N peers with M-of-N threshold.
    MeshShared {
        share_holders: Vec<PeerIdentity>,
        threshold: u8,
        purpose: SharedPurpose,
    },
}
```

---

## Policy specification

Every secret has an attached `SecretPolicy` set at storage time. Policies are immutable after attachment; modifying a policy requires re-sealing the secret as a new capability with a new policy.

```rust
pub struct SecretPolicy {
    /// Maximum total uses. None = unlimited.
    /// On reaching limit, capability auto-revokes.
    pub max_uses: Option<u32>,

    /// Already-used count. Incremented atomically on each redemption.
    pub uses_consumed: AtomicU32,

    /// Earliest valid use time.
    pub valid_from: Timestamp,

    /// Latest valid use time. None = no expiry.
    pub valid_until: Option<Timestamp>,

    /// Use-window: after first issuance, must be used within this many seconds.
    /// Useful for "session-scoped" capabilities.
    pub use_window_seconds: Option<u32>,

    /// What level of consent is required per redemption.
    pub consent_level: ConsentLevel,

    /// Whether this operation requires audit log entry.
    /// Almost always true; false only for ephemeral/session secrets.
    pub audit_required: bool,

    /// Whether this secret can appear in trace logs at all.
    /// Always false in production; true only in debug builds with explicit override.
    pub can_appear_in_logs: bool,

    /// Maximum plaintext lifetime in mlocked memory during redemption (seconds).
    /// Broker zeroes plaintext after this duration regardless of operation completion.
    pub plaintext_ttl_seconds: u32,

    /// How the secret is redacted in any context where its identity must appear
    /// (audit logs, trace markers, error messages).
    pub redaction_pattern: RedactionPattern,

    /// Whether this capability can be delegated via macaroon.
    pub delegation_allowed: DelegationPolicy,

    /// Confidentiality level required for redemption.
    /// Controls TEE/FHE/MPC routing in the kernel.
    pub confidentiality: Confidentiality,

    /// Rotation policy: when and how to rotate.
    pub rotation: RotationPolicy,

    /// Geographic restrictions. Operations from non-allowed locations refused.
    pub allowed_locations: Option<Vec<LocationId>>,

    /// Time-of-day restrictions.
    pub allowed_hours: Option<TimeOfDayPolicy>,

    /// Network restrictions: only redeemable when LFI is on specific networks.
    pub allowed_networks: Option<Vec<NetworkId>>,
}

pub enum ConsentLevel {
    /// AI can use freely. Use only for test fixtures and ephemeral material.
    None,

    /// AI uses freely; user gets passive notification post-hoc (push, email, log).
    PassiveNotify { channels: Vec<NotifyChannel> },

    /// User must approve each use synchronously before redemption proceeds.
    /// Approval channel: WebAuthn, mobile push, terminal prompt.
    ActiveConfirm { timeout_seconds: u32, channels: Vec<ApprovalChannel> },

    /// WebAuthn or TOTP required per use.
    /// Stronger than ActiveConfirm because it proves possession of a hardware token.
    BiometricGate { factor: AuthFactor },

    /// Multi-party approval required.
    /// Useful for high-value operations: production deploys, fund transfers, etc.
    QuorumGate {
        signers: Vec<UserId>,
        threshold: u8,
        timeout_seconds: u32,
    },
}

pub enum DelegationPolicy {
    /// Cannot be delegated.
    Forbidden,

    /// Can be delegated, but derived capabilities cannot themselves delegate further.
    SingleHop,

    /// Can be delegated transitively. Each hop adds a constraint.
    Multihop { max_depth: u8 },
}

pub enum RotationPolicy {
    /// Manual rotation only. Operator decides when.
    Manual,

    /// Auto-rotate on schedule.
    Scheduled { interval_days: u32 },

    /// Auto-rotate on use count.
    UseCountBased { rotate_after_uses: u32 },

    /// Auto-rotate immediately if scrubber detects exposure.
    OnExposure,

    /// Combination.
    Combined(Vec<RotationPolicy>),
}
```

### Policy templates

To avoid forcing operators to specify all fields for every secret, the system provides templates:

```rust
pub enum PolicyTemplate {
    /// Standard operator credential: unlimited uses, passive notify, 60s plaintext TTL.
    StandardCredential,

    /// AI-agent credential: 100 uses, passive notify, 30s TTL, 24h validity window after issue.
    AgentSession,

    /// High-value secret: 1 use per redemption, biometric gate, 5s TTL, audit required.
    HighValue,

    /// Production deploy: quorum gate (2-of-3), 1 use, 5s TTL, full audit + ZK proof.
    ProductionDeploy,

    /// PII access: legal basis required, retention enforced, GDPR-aligned.
    PiiAccess { subject: SubjectId },

    /// Mesh shared: M-of-N threshold, MPC-only execution.
    MeshOperational { threshold: u8, total: u8 },

    /// Ephemeral: single-use, 10s TTL, auto-revoke on use.
    OneShot,
}
```

Operators construct policies via `PolicyTemplate::StandardCredential.with_overrides(|p| { p.max_uses = Some(13); ... })` rather than building from raw fields.

---

## Broker architecture

A broker is a long-running service inside LFI that handles redemptions for a specific capability class. Brokers own the redemption logic, the consent flow, the plaintext lifetime, and the operation execution.

```rust
pub trait Broker: Send + Sync {
    type Capability: CapabilityClass;
    type Operation: BrokeredOperation;
    type Output: NonSensitive;

    /// Validate capability + operation match.
    fn validate(&self, cap: &Self::Capability, op: &Self::Operation) -> Result<(), BrokerError>;

    /// Obtain consent per policy.
    fn obtain_consent(&self, policy: &SecretPolicy) -> Result<ConsentToken, ConsentError>;

    /// Execute the operation. Plaintext exists only inside this method.
    fn execute(
        &self,
        cap_ref: &CapabilityRef,
        op: Self::Operation,
        consent: ConsentToken,
    ) -> Result<Self::Output, BrokerError>;
}
```

### Broker implementations (build order)

**Sprint 1 — `SshBroker`.** Handles SSH connections via `ssh2-rs` or `russh`. Plaintext password or key material exists only for the duration of the SSH handshake. Subsequent operations on the SSH session use the established session, never re-touching the credential. On session close, all credential material is zeroized.

```rust
pub struct SshBroker {
    kernel: Arc<ConfidentialityKernel>,
    consent_dispatcher: Arc<ConsentDispatcher>,
}

impl SshBroker {
    pub fn session<R, F>(
        &self,
        cap: CapabilityRef, // must be SshLogin variant
        f: F,
    ) -> Result<R, BrokerError>
    where
        F: FnOnce(&mut SshSession) -> Result<R, std::io::Error>,
        R: NonSensitive,
    {
        // 1. Validate capability is SshLogin and matches expected host
        // 2. Obtain consent per policy
        // 3. Unseal credential into SecretBuf
        // 4. Establish SSH session
        // 5. Zeroize credential SecretBuf immediately after session established
        // 6. Pass SshSession to f for arbitrary operations
        // 7. On f return, close session, write audit entry
    }
}
```

Usage:
```rust
let cap = vault.find::<SshLogin>("pineapple-mk7")?;
let wireless_config = ssh_broker.session(cap, |session| {
    session.exec("uci show wireless")
})?;
```

**Sprint 2 — `WebAuthBroker`.** HTTP(S) sessions with cookie/bearer/basic auth. Uses `reqwest` with rustls. Credentials attached as headers at request-build time, scrubbed from request struct after send.

**Sprint 3 — `ApiBroker`.** Specialized for API key injection. Supports the major auth schemes (header, query parameter, body field) and major services with built-in rate-limit awareness (Anthropic, OpenAI, GitHub, AWS, GCP, Stripe, Twilio).

**Sprint 4 — `GpgBroker`.** Delegates to `sequoia-pgp` for sign, encrypt, decrypt, certify operations. Private key material stays inside the broker; the AI agent receives sign/encrypt outputs.

**Sprint 5 — `DiskBroker`.** Disk encryption operations: mount LUKS volumes, unlock VeraCrypt containers, decrypt encrypted file blobs.

**Sprint 6 — `CommsBroker`.** WireGuard tunnel setup, Signal-protocol session establishment, libp2p secure-channel initiation.

**Sprint 7 — `PiiBroker`.** Specialized for PII operations: format PII into outbound messages (email, SMS, document fill), with strict redaction rules and consent enforcement.

**Sprint 8 — `MeshBroker`.** Coordinates mesh operations on `MeshShared` capabilities: MPC sessions, threshold signatures, secret-share reconstruction at threshold.

---

## Vault storage layer

The vault is the persistent store for sealed capabilities. It sits behind `lfi-conf-kernel`'s `Sealed<T>` infrastructure and the brokers' redemption flows.

### Storage backends

```rust
pub trait VaultStorage: Send + Sync {
    fn put(&self, sealed: &Sealed<dyn CapabilityClass>) -> Result<CapabilityId, StorageError>;
    fn get(&self, id: &CapabilityId) -> Result<Sealed<dyn CapabilityClass>, StorageError>;
    fn list(&self, filter: &CapabilityFilter) -> Result<Vec<CapabilityMetadata>, StorageError>;
    fn delete(&self, id: &CapabilityId) -> Result<(), StorageError>;
    fn rotate(&self, id: &CapabilityId, new: Sealed<dyn CapabilityClass>) -> Result<(), StorageError>;
}
```

**Backend A — Vaultwarden integration.** Reuse the existing Vaultwarden deployment as a storage backend. PlausiDen Secrets adds a thin Rust client (Vaultwarden's API is documented and stable). Operators get a familiar UI; AI agents get the typed capability layer on top.

**Backend B — Local sled/RocksDB.** For deployments that don't want to run Vaultwarden, a local embedded store with the same interface. Backed by `sled` for write performance or `rocksdb` for query patterns.

**Backend C — Mesh distributed store.** For multi-node deployments where capabilities must be available to multiple LFI instances. Uses libp2p IPLD-DAG with content-addressed storage; capabilities are fetched on demand and cached locally. Encrypted at rest under each node's local KEK.

### Capability lookup

```rust
pub trait Vault {
    /// Find a capability by typed pattern.
    /// Returns None if no matching capability exists.
    /// Returns error if multiple capabilities match (operator must disambiguate).
    fn find<C: CapabilityClass>(&self, pattern: &C::Pattern) -> Result<Option<CapabilityRef>, VaultError>;

    /// Find all capabilities matching a pattern.
    fn find_all<C: CapabilityClass>(&self, pattern: &C::Pattern) -> Result<Vec<CapabilityRef>, VaultError>;

    /// Add a new capability with the given policy.
    fn add<C: CapabilityClass>(
        &self,
        plaintext: SecretBuf,
        capability: C,
        policy: SecretPolicy,
    ) -> Result<CapabilityRef, VaultError>;

    /// Revoke a capability. All future redemptions refused.
    fn revoke(&self, cap: &CapabilityRef, reason: RevocationReason) -> Result<(), VaultError>;

    /// Rotate a capability: store new plaintext, mark old as superseded.
    /// Audit log records the rotation.
    fn rotate<C: CapabilityClass>(
        &self,
        old: &CapabilityRef,
        new_plaintext: SecretBuf,
    ) -> Result<CapabilityRef, VaultError>;
}
```

---

## Consent dispatcher

Consent flows are pluggable. The dispatcher routes consent requests to whichever channel the operator has configured.

```rust
pub trait ConsentDispatcher: Send + Sync {
    fn request_consent(
        &self,
        request: ConsentRequest,
    ) -> Result<ConsentToken, ConsentError>;
}

pub struct ConsentRequest {
    pub capability_id: CapabilityId,
    pub operation_summary: String, // human-readable; what is being done
    pub actor: ActorIdentity,
    pub urgency: Urgency,
    pub policy_required_factor: AuthFactor,
}
```

### Consent channels (build order)

**Channel 1 — WebAuthn via `webauthn-rs 0.5.2`.** Hardware security key or platform authenticator (TouchID, Windows Hello). Bind challenge to `hash(operation_summary || nonce)` so the user is approving this specific operation.

**Channel 2 — TOTP via `totp-rs 5.7`.** Six-digit code from an authenticator app. Use as breakglass when WebAuthn isn't available.

**Channel 3 — Mobile push.** Signed message over libp2p to the operator's phone (per the planned PlausiDenOS architecture). Phone displays operation details, operator approves with biometric or PIN, signed approval returned.

**Channel 4 — Terminal prompt.** For operators at a terminal, prompt with the operation details and require explicit `yes`. Bypassable in headless contexts; should not be used for `BiometricGate` policies.

**Channel 5 — Email/SMS.** For low-urgency `PassiveNotify`. One-way notification; no approval required.

**Channel 6 — Quorum collection.** For `QuorumGate` policies. Dispatches approval requests to multiple signers via their preferred channels, collects responses, validates threshold met.

---

## Macaroon-based delegation

Macaroons (Birgisson et al., NDSS 2014) are the right cryptographic primitive for capability delegation. They allow a holder of a master capability to mint derived capabilities with **monotonically more restrictive** caveats, without contacting the issuer.

### Why macaroons over JWTs

JWTs require the verifier to trust the issuer's signing key. They don't compose cleanly: deriving a more-restricted JWT from an existing JWT requires the original signer.

Macaroons compose by chained HMACs. Each caveat adds a new HMAC layer using the previous HMAC as the key. The verifier needs only the original macaroon's secret to validate the entire chain. **Holders can derive without contacting issuers.**

This is exactly the property needed for AI delegation. Your phone holds the master capability for SSH to your home lab. It mints a derived capability for a specific Claude Code session: `valid_for: 4h, max_uses: 13, scope: PineappleHost`. The Claude Code session can use that derived capability without contacting your phone for each operation. Revoke the master, all derivations invalidate.

### Implementation

```rust
pub struct Macaroon {
    pub identifier: MacaroonId,
    pub location: VaultLocation,
    pub caveats: Vec<Caveat>,
    pub signature: MacaroonSignature,
}

pub enum Caveat {
    /// First-party caveat. Verifier checks against local context.
    FirstParty(Predicate),

    /// Third-party caveat. Verifier checks against external service.
    /// Used for cross-organization delegation (rare in LFI's use case).
    ThirdParty {
        location: ServiceUrl,
        identifier: CaveatId,
        verification_key_hint: KeyHint,
    },
}

pub enum Predicate {
    ValidUntil(Timestamp),
    MaxUses(u32),
    AllowedHost(HostPattern),
    AllowedOperation(OperationKind),
    BoundToSession(SessionId),
    BoundToActor(ActorIdentity),
    GeographicRestriction(Vec<LocationId>),
    Custom { tag: String, value: Vec<u8> },
}

impl Macaroon {
    /// Mint a new macaroon from a master capability.
    pub fn mint(
        master: &CapabilityRef,
        location: VaultLocation,
        signing_key: &SymmetricKey,
    ) -> Result<Self, MacaroonError>;

    /// Derive a more-restricted macaroon by adding a caveat.
    /// No contact with issuer needed; holder computes locally.
    pub fn add_caveat(self, caveat: Caveat) -> Self;

    /// Verify the macaroon and its caveat chain.
    pub fn verify(
        &self,
        secret: &SymmetricKey,
        verifier: &CaveatVerifier,
    ) -> Result<(), VerificationError>;
}
```

### Delegation patterns

**Pattern 1 — AI session delegation.** Operator mints a macaroon for "all SSH capabilities, valid for 8 hours, bound to AI session ID X." The AI session uses this macaroon for any SSH operation needed during the session, without per-operation operator approval. Session expiry auto-invalidates.

**Pattern 2 — Mesh peer delegation.** PlausiDen node A mints a macaroon for peer B: "read-only access to Sacred.Vote tally data, valid until election close." Peer B verifies locally, performs operations, no contact with A needed per operation.

**Pattern 3 — Contractor delegation.** When you bring on a contractor for a specific engagement, mint them a macaroon for "engagement X resources, valid for project duration, max N operations of type Y." Contractor's tooling exercises capability without ever seeing your master credential.

**Pattern 4 — Time-boxed elevation.** Operator escalates their own capabilities for a specific high-risk operation: "valid for next 5 minutes, max 1 use, bound to my current TOTP." Reduces blast radius of operator credential compromise.

---

## PII subsystem

PII is a special capability class with stricter defaults aligned with GDPR/CCPA.

### Legal basis enforcement

```rust
pub enum LegalBasis {
    /// Subject explicitly consented. consent_record is link to ConsentArtifact.
    UserConsent { consent_record: ConsentId },

    /// Necessary for contract performance.
    ContractualNecessity { contract_id: ContractId },

    /// Required by law.
    LegalObligation { statute_ref: StatuteRef },

    /// Necessary to protect vital interests of subject or another natural person.
    VitalInterests { justification: String },

    /// Performance of a task in the public interest.
    PublicTask { authority: PublicAuthorityId },

    /// Legitimate interest, with documented assessment.
    LegitimateInterest { assessment: AssessmentId },
}
```

PII capabilities cannot be created without a legal basis attached. The `PiiBroker` enforces this at storage time; attempting to add a PII capability without a `LegalBasis` is a kernel error.

### Retention enforcement

Every PII capability has a `retention_until` timestamp. The vault runs a background job that, daily:

1. Identifies PII capabilities past their retention date
2. Securely deletes the underlying sealed material (DoD 5220.22-M overwrite or NIST 800-88 sanitize, depending on storage backend)
3. Records the deletion in the audit log
4. Notifies the operator if any deletion failed

Retention extension requires documented legal basis. The PiiBroker emits a warning to the operator 30/14/7/1 days before retention.

### Right-to-erasure

```rust
impl Vault {
    /// GDPR Article 17 right to erasure.
    /// Revokes all PII capabilities for the subject, securely deletes
    /// underlying material, propagates deletion to mesh peers,
    /// records the operation in the audit log.
    pub fn erase_subject(
        &self,
        subject: SubjectId,
        request: ErasureRequest,
    ) -> Result<ErasureReport, VaultError>;
}
```

This is one operation. Because PII never existed in plaintext in LFI's reasoning context (only as `Sealed<Pii>` references brokered through the PiiBroker), there's no scattered plaintext to chase down. Erasure of the underlying sealed material renders all references unrevealable.

### Breach notification

PII capabilities marked `breach_notification_required: true` trigger automated breach response if exposure is detected (by the chat scrubber, by audit log analysis, by external report):

1. Identify all subjects whose PII was exposed
2. Generate breach notification draft per applicable law (GDPR 72-hour rule, CCPA, state breach laws)
3. Surface to operator for review and dispatch
4. Auto-rotate any rotatable elements (passwords, tokens; not biometrics or immutable PII)

---

## User interface

### Operator dashboard

A web UI integrated into LFI's existing dashboard. Capabilities displayed as:

- Capability ID, classification, creation date
- Current policy summary
- Use count (current / max), last used time
- Recent audit log entries for this capability
- Rotation status, next scheduled rotation
- Delegation tree (if any macaroons derived from this capability)
- Action buttons: rotate now, revoke, modify policy (creates new capability), inspect plaintext (requires consent)

Per-operation activity feed: shows recent broker activity, who/what/when. Filterable by capability, actor, operation type, time range.

Audit log viewer: chronological, filterable, exportable. Verification button runs full chain integrity check.

### CLI

```bash
# Add a capability
$ pds add ssh pineapple-mk7 --host 172.16.42.1 --user root --template AgentSession

# List capabilities
$ pds list --class ssh

# Show capability details (no plaintext)
$ pds show pineapple-mk7

# Inspect plaintext (requires consent per policy)
$ pds inspect pineapple-mk7

# Rotate
$ pds rotate pineapple-mk7

# Revoke
$ pds revoke pineapple-mk7 --reason "device retired"

# Delegate (mint macaroon)
$ pds delegate pineapple-mk7 --max-uses 13 --valid-for 4h --bound-to-session $SESSION_ID

# Audit query
$ pds audit --capability pineapple-mk7 --since 1d
```

### Programmatic API for AI agents

```rust
// In LFI agent code:
let cap = vault.find::<SshLogin>(SshLogin::pattern()
    .host("172.16.42.1")
    .user("root"))?
    .ok_or(NoSuchCapability)?;

let result = ssh_broker.session(cap, |s| {
    s.exec("uci show wireless")
})?;
```

The agent never holds plaintext; the broker handles consent, unsealing, and zeroization. The agent simply calls operations.

---

## Integration with existing PlausiDen architecture

### Vaultwarden

`plausiden-secrets` reuses the existing Vaultwarden deployment as Backend A. The Vaultwarden web UI handles operator-facing storage; the typed capability layer sits on top via API integration. Operators continue using Vaultwarden directly for operations not yet typed (browser autofill, mobile app sync); PlausiDen Secrets handles AI-broker operations.

### plausiden-vault (planned)

The previously planned `plausiden-vault` crate (vault-core, vault-daemon, vault-cli, vault-shield) is **subsumed by this design**. Backend B (local sled/RocksDB) implements the storage layer originally planned for vault-core. The vault-daemon role is split between the brokers (operation execution) and the consent dispatcher. The vault-cli role is the `pds` command above. The vault-shield role is provided by `lfi-conf-kernel`'s `Sealed<T>` enforcement and memory protection.

If the operator wants to keep `plausiden-vault` as a separate crate name, the design above can be packaged as `plausiden-vault` instead. The functionality is identical.

### plausiden-auth (planned)

The previously planned `plausiden-auth` crate (Argon2id, TOTP, WebAuthn, axum middleware) is **subsumed by the consent dispatcher**. Argon2id is used for operator master passphrase derivation when unlocking the vault. TOTP and WebAuthn are consent channels. The axum middleware role applies to the dashboard UI authentication.

### Sacred.Vote integration

Sacred.Vote's cryptographic operations (vote signing, tally verification) become broker operations on `GpgSigning` and `MeshShared` capabilities. The DLD zkTLS proof-of-concept ties into ZK proof generation via `lfi-conf-kernel`'s Subsystem 8.

Multi-party operations (you + Tim + DLD) use `MeshShared` capabilities with M-of-N threshold, executed via `MeshBroker`.

---

## Sprint plan

### Sprint 1 — Core types and SshBroker (3 weeks)

**Deliverables:**
- `plausiden-secrets` crate scaffold
- `Capability` enum with all variants defined (implementations stubbed)
- `SecretPolicy`, `PolicyTemplate`, `ConsentLevel` types
- `Vault` trait + Backend A (Vaultwarden integration) + Backend B (local sled)
- `Broker` trait + `SshBroker` full implementation
- `ConsentDispatcher` trait + Channel 4 (terminal prompt) full implementation
- `pds` CLI for: add, list, show, inspect, revoke (rotate stubbed)
- Integration with `lfi-conf-kernel` (depends on Kernel Sprints A-B)

**Acceptance criteria:**
- Add an SSH capability for the Pineapple via `pds add ssh pineapple-mk7 ...`
- Issue an SSH operation via the broker; plaintext exists for <100ms
- Audit log entry written, chain integrity verifiable
- User types Pineapple password exactly once; subsequent operations use vault

This sprint delivers the **immediate user-facing benefit**: the workflow described in the original conversation. After Sprint 1, the user types each password once and AI agents use them under policy without re-prompting.

### Sprint 2 — WebAuth, ApiKey, GpgSigning brokers (3 weeks)

**Deliverables:**
- `WebAuthBroker` for cookie/bearer/basic auth
- `ApiBroker` with built-in handlers for Anthropic, OpenAI, GitHub, AWS, GCP, Stripe, Twilio
- `GpgBroker` delegating to `sequoia-pgp`
- WebAuthn channel via `webauthn-rs 0.5.2`
- TOTP channel via `totp-rs 5.7`
- `pds rotate` command for supported services
- Web dashboard MVP: capability list, audit log viewer

**Acceptance criteria:**
- Browse the LFI dashboard, see all capabilities, audit history
- Rotate a GitHub PAT via `pds rotate`; new token is generated, old token revoked, vault updated, audit logged
- WebAuthn touch required for `BiometricGate` operations

### Sprint 3 — Macaroon delegation + mesh integration (4 weeks)

**Deliverables:**
- Macaroon implementation with first-party caveats
- Delegation API: mint, derive, verify, revoke
- `MeshBroker` for distributed operations
- Backend C (mesh-distributed vault via libp2p IPLD-DAG)
- Mobile push channel via signed libp2p messages
- Quorum gate consent collection

**Acceptance criteria:**
- Operator mints a macaroon scoped to "PineappleHost SSH, 4h, 13 uses, bound to session X"
- Claude Code session uses the macaroon for SSH operations without operator approval per-operation
- Macaroon expiry auto-invalidates; subsequent attempts refused
- Revoking the master capability invalidates all derived macaroons

### Sprint 4 — PII subsystem and chat scrubber integration (3 weeks)

**Deliverables:**
- `PiiBroker` with legal basis enforcement
- `Pii` capability variants for common PII fields
- Retention enforcement background job
- Right-to-erasure operation
- Breach notification automation
- Integration with `lfi-conf-kernel` chat scrubber: detected secrets in scrubbed history trigger vault rotation

**Acceptance criteria:**
- Add a PII capability without legal basis: refused
- Add with legal basis: stored, retention countdown begins
- Past retention: auto-deleted, audit logged
- `pds erase-subject <subject_id>`: all PII for subject erased, mesh peers notified, report generated
- Chat scrubber detects a known credential in history: corresponding vault entry rotates automatically

### Sprint 5 — Disk, Comms, Biometric brokers + UI polish (3 weeks)

**Deliverables:**
- `DiskBroker` for LUKS/VeraCrypt operations
- `CommsBroker` for WireGuard/Signal-protocol session establishment
- `BiometricBroker` for biometric template operations (with strict consent requirements)
- Dashboard delegation tree visualization
- Dashboard rotation scheduler
- Dashboard policy editor (creates new capability rather than mutating existing)

**Acceptance criteria:**
- Mount an encrypted volume via DiskBroker; password never in shell history or logs
- Establish a WireGuard tunnel via CommsBroker; private key stays in vault
- Visual delegation tree shows master + all derived macaroons; revoke any node and downstream invalidates

### Sprint 6 — Production hardening (2 weeks)

**Deliverables:**
- Comprehensive property-based tests (proptest)
- Fuzzing harness for parser surfaces (caveat parsing, macaroon parsing, vault import/export)
- Performance benchmarks; identify and optimize hot paths
- Documentation: operator guide, AI-agent integration guide, security posture statement
- Sigstore signing of release artifacts

**Acceptance criteria:**
- 1000+ property tests pass
- Fuzz harness runs 24h with no crashes or vulnerabilities
- Operations complete within performance budgets (SSH session: <500ms total including consent; API call: <100ms broker overhead)
- All artifacts Sigstore-signed; verification documented

---

## Operational guarantees

After Sprints 1-4 are complete, `plausiden-secrets` provides:

**G1.** Each operator credential is typed exactly once; subsequent uses by AI agents require no re-typing.

**G2.** Every credential use is recorded with capability ID, actor, operation, timestamp, outcome — in a tamper-evident audit log.

**G3.** Operators can configure per-credential policies covering: max uses, time windows, consent requirements, audit demands, plaintext lifetime, geographic/network/temporal restrictions.

**G4.** AI agents exercise capabilities through brokers; they never receive plaintext credential material.

**G5.** Capabilities can be delegated cryptographically: a master capability mints derived capabilities with monotonically more restrictive constraints, without contacting the issuer per derivation.

**G6.** PII is handled with GDPR-aligned defaults: legal basis required, retention enforced, right-to-erasure available as a single operation.

**G7.** When a credential appears in plaintext anywhere LFI can scan (chat history, shell history, logs), the corresponding vault entry rotates automatically.

**G8.** Mesh-shared secrets are held collectively across N peers with M-of-N threshold; no single party can reconstruct the full secret.

These guarantees compose with `lfi-conf-kernel`'s structural confidentiality properties (Sealed types, encrypted memory, brokered access, audit chain) to produce a complete confidential-AI substrate.

---

## What this enables for PlausiDen as a business

The combination of `lfi-conf-kernel` and `plausiden-secrets` is **the productized capability that distinguishes PlausiDen from every other AI offering**.

Anthropic doesn't have this. OpenAI doesn't have this. The MCP ecosystem is still doing "paste your API key into config." Apple's Private Cloud Compute is the closest analog and is iOS-internal.

PlausiDen offers: AI that can be deployed in HIPAA, SOX, PCI-DSS, FedRAMP, GDPR-strict environments with structural confidentiality guarantees, not policy-and-pinky-promise. The audit log and ZK proofs satisfy compliance auditors directly. The macaroon delegation supports multi-party trust scenarios (Sacred.Vote's exact use case). The chat-scrubber and rotation automation prevent the "credentials accidentally in chat history" leak that plagues every AI-assistant deployment.

This is the kind of architectural commitment that becomes a moat. The work is real — Sprints 1-4 are roughly 13 weeks for a complete first version — but the payoff is qualitative: PlausiDen becomes deployable in environments currently categorically excluded from AI use, and the price tag for those deployments reflects the difficulty competitors would have replicating.

For Sacred.Vote specifically: the multi-party operations (you + Tim + DLD coordinating on cryptographic ballot operations) become natural broker operations on `MeshShared` capabilities. The DLD zkTLS work integrates with the ZK proof subsystem. Government partners get cryptographic auditability of every operation against their data, satisfying state-level audit requirements that no competing AI vendor can match.

---

## Final notes for Claude Code

Build Sprint 1 first and ship it as soon as it works. The user-facing benefit (type each password once, never again) is so immediate that the rest of the design can roll out incrementally without losing momentum.

Build the SshBroker with the Pineapple as the primary integration test target. The Pineapple session in the planning conversation is the canonical example: Sprint 1 must reproduce that workflow with zero password re-prompting.

Reuse Vaultwarden for storage; do not rebuild credential storage from scratch. Vaultwarden is mature, the API is stable, the operator already runs it. PlausiDen Secrets is the typed-capability layer on top.

Macaroons are the right primitive for delegation. Resist the temptation to use JWTs because they're more familiar; macaroons compose locally, JWTs require issuer round-trips for derivation. The mesh use cases require local composition.

PII handling is not optional. Even if the initial deployment doesn't touch PII, the PiiBroker must be present and enforced because once an operator stores their first PII capability without legal basis, the deployment is out of GDPR compliance. Build it in from Sprint 4 latest.

Default to the more restrictive option in every design choice. Easier to relax later than to tighten after operators rely on a capability behavior.

Coordinate with the `lfi-conf-kernel` design (LFI-CONFIDENTIALITY-KERNEL-DESIGN.md). The kernel and `plausiden-secrets` are co-designed; changes to one likely affect the other.
