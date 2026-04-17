# PlausiDen Supersociety Stack — Migration & Architecture Plan

## Philosophy
Every layer uses the best tool in its field — not the most popular, but the most
correct, auditable, performant, and future-proofed. This stack assumes state-level
adversaries with unlimited budget and nation-state forensic labs.

---

## Layer 1: Core Engine & Crypto (Rust)
**Status:** Already Rust. No migration needed.
**Why Rust:** Memory safety without GC, zero-cost abstractions, `no_std` for embedded,
formal verification tooling (Prusti, Kani), WASM compilation for browser extension.

### Crypto Stack (audited crates only)
| Function | Crate | Why |
|----------|-------|-----|
| Symmetric encryption | `chacha20poly1305` | AEAD, constant-time, RustCrypto audited |
| Hashing | `blake3` | 3x faster than SHA-256, tree hashing for parallelism |
| Key derivation | `argon2` | Memory-hard, resists ASIC/GPU attacks |
| Signatures | `ed25519-dalek` | Deterministic, no nonce reuse risk |
| Secret sharing | Custom GF(256) | Shamir's over GF(256), no external dep |
| Erasure coding | Custom Reed-Solomon | Cauchy matrix, GF(256) arithmetic |
| Secure RNG | `rand_chacha` 0.3 | ChaCha20-based CSPRNG |
| Constant-time ops | `subtle` | Timing attack resistance |
| Zeroization | `zeroize` | Compile-time guaranteed secret clearing |

### Future Upgrades
- **Post-quantum:** Migrate to `pqcrypto` (Kyber/ML-KEM for key exchange, Dilithium/ML-DSA for signatures) when NIST finalizes. Hybrid mode first (classical + PQ).
- **Formal verification:** Use Kani model checker for crypto paths. Prusti for invariants.
- **Fuzzing:** `cargo-fuzz` + `honggfuzz-rs` on all parsers and crypto interfaces.

---

## Layer 2: Networking (Rust + eBPF)
**Current:** Pure Rust userspace.
**Migration:** eBPF for kernel-level packet filtering.

| Component | Tool | Why |
|-----------|------|-----|
| Firewall | `aya` (Rust eBPF) | Type-safe eBPF programs, no C required |
| Packet capture | `AF_XDP` via aya | Zero-copy kernel bypass |
| DNS filtering | `hickory-dns` | Pure Rust, async, DoT/DoH native |
| TLS inspection | `rustls` | No OpenSSL, memory-safe TLS |
| Protocol parsing | `nom` | Zero-copy parser combinators |
| Async runtime | `tokio` | Industry standard, io_uring support |

### P2P / Swarm
| Component | Tool | Why |
|-----------|------|-----|
| Transport | `libp2p` (Rust) | Battle-tested, Noise protocol, muxing |
| NAT traversal | `libp2p-autonat` + STUN | Works behind carrier-grade NAT |
| Discovery | `libp2p-kad` | Kademlia DHT, Sybil-resistant |
| Onion routing | Custom 3-hop | Tor-inspired, ChaCha20 layers |
| Pluggable transports | `obfs4`, `meek`, `snowflake` | Censorship resistance |

### Future
- **QUIC everywhere:** Migrate from TCP to `quinn` (Rust QUIC) for all P2P traffic.
- **WireGuard tunnels:** `boringtun` for point-to-point encrypted tunnels.
- **Tor integration:** Consider `arti` (Rust Tor client) for real anonymity.

---

## Layer 3: Desktop Application
**Current:** Tauri shell (Rust + HTML).
**Stack:**

| Component | Tool | Why |
|-----------|------|-----|
| Framework | Tauri 2.x | Rust backend, system webview, tiny binary |
| Frontend | Leptos (Rust→WASM) | No JavaScript runtime, SSR, fine-grained reactivity |
| Styling | TailwindCSS | Utility-first, tree-shaking, dark mode native |
| IPC | Tauri commands | Type-safe Rust↔frontend bridge |
| System tray | Tauri tray plugin | Native OS integration |
| Notifications | Tauri notification plugin | OS-native notifications |

### Why Leptos over React/Svelte
- Compiles to WASM — no V8/SpiderMonkey attack surface
- Rust type safety end-to-end (backend → frontend)
- Fine-grained reactivity (no virtual DOM diffing)
- Isomorphic — same code renders server-side and client-side

### Future
- **Iced** (pure Rust GUI): Evaluate when Iced hits 1.0 for native rendering without webview.
- **Slint** (declarative Rust GUI): Alternative for embedded/resource-constrained.

---

## Layer 4: Browser Extension
**Current:** TypeScript with planned WASM bridge.
**Migration:** Full WASM.

| Component | Tool | Why |
|-----------|------|-----|
| Engine | `wasm-bindgen` + engine-browser | Rust→WASM, no JS for data generation |
| Build | `wasm-pack` | Optimized WASM bundles |
| Storage | `web-sys` IndexedDB | Direct browser API access from WASM |
| Manifest | MV3 service worker | Chrome/Firefox compatible |

### Future
- **WasmGC:** When browsers ship WasmGC, eliminate JS glue entirely.
- **WebGPU:** Accelerate ML-based distinguisher with compute shaders.

---

## Layer 5: Mobile (Android)
**Current:** Kotlin shell with planned JNI bridge.
**Stack:**

| Component | Tool | Why |
|-----------|------|-----|
| UI | Jetpack Compose | Modern declarative Android UI |
| Engine bridge | `cargo-ndk` → JNI | Compile Rust to .so, call via JNI |
| Background work | WorkManager | Battery-efficient scheduling |
| Crypto | Rust via JNI | No Java crypto — all in Rust |

### Future
- **Kotlin Multiplatform + Rust:** Share business logic across Android/iOS.
- **Flutter + Rust FFI:** Evaluate for true cross-platform mobile.

---

## Layer 6: Embedded / USB
**Current:** RP2040 scaffold.
**Stack:**

| Component | Tool | Why |
|-----------|------|-----|
| MCU framework | `embassy-rs` | Async embedded Rust, no RTOS needed |
| Target | RP2040 (ARM Cortex-M0+) | Cheap, widely available, USB OTG |
| USB stack | `embassy-usb` | Pure Rust USB device stack |
| Crypto | `chacha20poly1305` no_std | Same crypto as desktop, no alloc |
| Signing | `ed25519-dalek` no_std | Hardware attestation |

---

## Layer 7: Infrastructure & DevOps
| Component | Tool | Why |
|-----------|------|-----|
| CI/CD | GitHub Actions | Already in use, free for open source |
| Container | Podman (rootless) | No daemon, OCI-compatible, no Docker attack surface |
| Package | Nix | Reproducible builds, hermetic environments |
| Secret management | `age` | Simple, auditable encryption for secrets |
| Monitoring | Prometheus + Grafana | Industry standard, self-hosted |
| Log aggregation | `Vector` (Rust) | High-performance, written in Rust |

### Build System
| Tool | Purpose |
|------|---------|
| `cargo-nextest` | Parallel test runner (10x faster than cargo test) |
| `cargo-deny` | License + vulnerability auditing |
| `cargo-audit` | CVE checking for dependencies |
| `cargo-mutants` | Mutation testing |
| `cargo-llvm-cov` | Code coverage |
| `cargo-semver-checks` | API compatibility checking |

---

## Layer 8: Marketplace
**Current:** Axum + server-rendered HTML.
**Stack:**

| Component | Tool | Why |
|-----------|------|-----|
| HTTP framework | `axum` | Tower middleware, type-safe extractors |
| Templating | `askama` | Compile-time checked templates |
| Database | SQLite via `rusqlite` | Zero-config, embedded, encrypted with SQLCipher |
| Crypto payments | Monero RPC | Privacy-preserving cryptocurrency |
| Content delivery | IPFS via `iroh` | Decentralized, censorship-resistant |
| Onion service | Tor hidden service | Anonymous hosting |
| Rate limiting | `governor` | Token bucket, in-process |

### Future
- **CRDTs:** Use `automerge-rs` for conflict-free replicated catalog data.
- **Nostr:** Evaluate as decentralized marketplace protocol.

---

## Layer 9: PlausiDenOS (LAST PRIORITY)
| Component | Tool | Why |
|-----------|------|-----|
| Microkernel | seL4 | Formally verified, capability-based |
| Component framework | CAmkES | Type-safe IPC between components |
| Userland | Rust `no_std` | Memory safety in every component |
| Filesystem | Custom encrypted | Full-disk encryption, plausible deniability |
| Networking | Rust + seL4 network stack | Isolated from kernel |

---

## Migration Priority Order

1. **Immediate (this sprint):**
   - `cargo-nextest` for all repos (drop-in replacement, instant speedup)
   - `cargo-deny` + `cargo-audit` in CI
   - `wasm-pack` build for engine-browser → browser extension

2. **Short-term (next 2 weeks):**
   - `aya` eBPF programs for Firewall kernel bypass
   - `leptos` frontend for Desktop (replace raw HTML)
   - `cargo-ndk` JNI bridge for Android
   - `nom` parsers for DPI protocol analysis

3. **Medium-term (1-2 months):**
   - `libp2p` migration for Swarm networking
   - `iroh` (Rust IPFS) for marketplace catalog
   - `arti` Tor client for Swarm anonymity
   - Nix flake for reproducible builds

4. **Long-term (3-6 months):**
   - Post-quantum crypto migration (hybrid mode)
   - `embassy-rs` firmware for USB device
   - seL4 kernel configuration for PlausiDenOS
   - Formal verification of critical paths (Kani + Prusti)

---

## Principles

1. **Rust everywhere possible.** Every line of C/C++ is a potential memory corruption.
2. **Audited dependencies only** for crypto. Self-audit everything else.
3. **No JavaScript in security-critical paths.** WASM for browser, native for everything else.
4. **Reproducible builds.** Every binary must be deterministically rebuildable.
5. **Zero trust between components.** Every IPC boundary validates inputs.
6. **Future-proof crypto.** Hybrid classical+PQ from day one when available.
7. **Offline-first.** Every tool must function without internet access.
8. **Minimal dependencies.** `cargo-deny` rejects anything with unacceptable licenses or known CVEs.
