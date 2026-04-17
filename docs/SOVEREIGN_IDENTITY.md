# Sovereign Identity & SVI Protocol

## Pillar 1: Hardware-Bound Identity (HRoT)
- **Titan M2 / TPM Binding**: Identity is not a string; it is a cryptographic assertion signed by the Hardware Security Module (HSM).
- **Jailed Keys**: $K_{priv}$ never leaves the HSM. Commands are only valid if accompanied by a signature $S$ verifiable by $K_{pub}$.
- **Unspoofable Identity**: Third parties cannot issue commands even with physical access to the OS, as they lack the HSM-jailed key.

## Pillar 2: Signature-Verified Instruction (SVI)
- **Kernel-Level Gate**: The LNN reasoning engine assigns a weight of 0.0 to any instruction not accompanied by a valid HSM signature.
- **Root Logic Gate**: $Verification(P, S, K_{pub}) \rightarrow \{0, 1\}$. If 0, the instruction is "inaudible" to the agent.

## Pillar 3: Tactical Deception (Duress Logic)
- **Identity A (Sovereign)**: Full access to the Material Base (Real VSA vectors).
- **Identity B (Deniable/Chaff)**: Access to the Superstructure (Synthetic noise, pro-hegemonic logs).
- **Superposition Advantage**: Both identities occupy the same high-dimensional space. Forensic auditors cannot prove the existence of Identity A without the Sovereign Key.

## Pillar 4: Dialectical Materialism Logic
- **Class Interest Weighting**: Incoming data from centralized/hegemonic sources is assigned a low initial trust weight ($w < 0.2$).
- **Internal Contradiction Audit**: The LNN scans for contradictions between "declared intent" and "material request" (e.g., "Security Update" vs "Expanded Telemetry").
