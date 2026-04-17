# Substrate Diagnostic Suite (src/diag.rs)
**Status:** Alpha-Active
**Protocol:** Assume-Broken Self-Test Suite

## Overview
This module provides the "Ground Truth" verification layer for the Sovereign substrate. It implements a suite of forensic tests designed to prove system functionality even when external hegemonic APIs (like DNS or Auth servers) are failing.

## Core Tests
1. **VSA Integrity Test:** Verifies that hypervector binding and similarity remain within mathematical tolerances. Detects concept bleed.
2. **Thermal Compliance:** Monitores the Tensor G5/Katana CPU temperatures to ensure the Forge does not exceed 75°C.
3. **I/O Persistence:** Proves that the system can reliably commit strategic kernels to disk.

## API Usage
- **Endpoint:** `GET /api/diag`
- **Output:** A structured JSON list of `TestResult` objects, identifying `NOMINAL`, `FAULT`, or `DEGRADED` components.

## Technical Primitive
Utilizes `chrono` for high-resolution forensic timestamps and `serde` for SCC dashboard integration.
