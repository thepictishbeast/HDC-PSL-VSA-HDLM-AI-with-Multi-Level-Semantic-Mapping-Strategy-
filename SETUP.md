# LFI Sovereign Intelligence — Setup & Interaction Guide

Welcome to the LFI Sovereign Intelligence. This system utilizes a sovereign cognitive architecture built on Vector Symbolic Architectures (VSA), Hyperdimensional Computing (HDC), and Liquid Neural Networks (LNN).

## 1. Prerequisites

Before building and interacting with the system, ensure you have the following installed:
- **Rust Toolchain:** Stable Rust (1.70+ recommended). You can install it via [rustup](https://rustup.rs/).
- **Cargo:** Included with `rustup`.
- **System Dependencies:** Standard build tools (`build-essential` on Linux).

## 2. Building the Project

Navigate to the core directory and build the workspace:

```bash
cd lfi_vsa_core
cargo build --release
```

## 3. Interacting with the AI

The primary way to test the cognitive reasoning engine directly is via the interactive terminal interface.

Start the chat terminal:

```bash
cargo run --bin chat
```

### Example Prompts

You can test the system's dual-mode reasoning (System 1 fast retrieval vs. System 2 deep planning) using the following prompts.

**Conversational & System Self-Awareness:**
- `hello how are you?`
- `who are you?`

**Code Generation (System 2 Planning):**
- `write a function in rust to calculate the fibonacci sequence`
- `implement a distributed consensus algorithm using Raft`

**Novelty & Research (OSINT Delegation):**
- `What is the square root of 52934?`
- `Who won the World Series in 2025?`
*(Note: Because these facts are novel and not in the immediate VSA core memory, the AI will dynamically delegate them to the OSINT / Web Ingestor modules.)*

To exit the terminal, simply type:
`exit` or `quit`

## 4. Running the Audit Suite

To verify the integrity of the Zero-Trust PSL axioms and the HDC core, run the test suite:

```bash
cargo test
cargo test --test opsec_test
```

## 5. Web UI Dashboard

The project includes a web dashboard built with React and Vite. To use the UI instead of the terminal:

1. Navigate to the frontend directory:
   ```bash
   cd lfi_dashboard
   ```

2. Install dependencies (if you haven't already):
   ```bash
   npm install
   ```

3. Start the development server:
   ```bash
   npm start
   ```

4. The dashboard will be accessible via your web browser (typically at `http://localhost:5173`).
