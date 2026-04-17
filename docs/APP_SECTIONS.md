# PlausiDen AI — Application Sections

## Naming: The Campus

| Section | Name | What it is |
|---------|------|------------|
| Chat | **Agora** | Idea exchange, conversation with AI |
| Training | **Classroom** | Training data, evaluation, feedback (LMS) |
| Audits | **Auditorium** | All audit types, compliance, reviews |
| AI Fleet | **Fleet** | Agent orchestration, task queue, idle detection |
| Secrets | **Vault** | PlausiDen-Vault integrated — credential capabilities, policies, audit |
| Facts | **Library** | Knowledge graph, fact browser, search |
| Admin | **Admin** | System config, server stats, user management |
| Research | **Research Lab** | Causal reasoning, provenance, hypothesis testing |
| Security | **Colosseum** | Adversarial combat, pentesting, Pineapple captures, red/blue team |
| Automation | **Workflows** | Task scheduling, recurring operations |
| Network | **Mesh** | P2P nodes, EigenTrust, knowledge exchange |
| User | **Profile** | Identity, preferences, connected devices |

## Vault Disambiguation

**The Vault section in this app IS PlausiDen-Vault** (repo: thepictishbeast/PlausiDen-Vault).
Same project, integrated as a UI page. Enhanced with the confidentiality kernel's
Sealed<T> capability system from LFI-CONFIDENTIALITY-KERNEL-DESIGN.md.

This is DIFFERENT from:
- **Vault PD** in PlausiDen-OS-for-Mobile — deniable encrypted storage on seL4 phone
- Generic "vault" references — always means PlausiDen-Vault unless PD specified

## Build Phases

### v1 (current sprint)
- Agora (Chat) ✓ working
- Classroom — in progress (ClassroomView.tsx created)
- Admin — in progress (AdminModal.tsx, dashboard endpoint)
- Fleet — orchestrator running on :3001

### v2 (next sprint)
- Library (Knowledge Graph)
- Research Lab
- Auditorium
- Vault (PlausiDen-Vault integration)

### v3 (future)
- Colosseum
- Workflows
- Mesh
- Profile

### Overlays (all phases)
- AI Visual Presence (second cursor, activity indicators)
- In-app terminal (xterm.js)
- Notifications + toasts

## Audit & Compliance (Auditorium detail)
- Security audits (AVP-2 tier 1-6 results)
- Code audits (unwrap count, test coverage, mutation testing)
- Data quality audits (dedup rates, contamination, quality distribution)
- Compliance audits (GDPR, HIPAA, SOX, PCI-DSS status)
- Network audits (Pineapple captures, adversary tier detection)
- Performance audits (response times, throughput, resource usage)
- Training audits (data quality, model accuracy, regression detection)
- Dependency audits (cargo audit, supply chain, CVE status)
- Each audit: history, trends, pass/fail, findings, remediation tracking
- Part of orchestration platform — audits auto-scheduled, assigned to agents

## Data Inventory (visible in Library + Classroom)

Every data source, dataset, and tool must be listed in the app UI:

### Data Sources Registry
- Every source in brain.db with: name, URL, license, size, fact count, quality avg, domain, ingestion date
- Currently 360+ sources — all must be browsable, searchable, filterable
- Show: HuggingFace datasets, UCI ML datasets, CVE data, user-uploaded files, Ollama-generated

### Dataset Browser
- All training data files in /home/user/LFI-data/ listed with sizes and pair counts
- JSONL viewer — click to preview first 10 entries
- Import button — drag-and-drop new datasets
- Export button — download any dataset as JSONL

### Tool Registry
- Magpie (synthetic data generation)
- Ollama (local inference + training pair generation)
- pineapple-harden / capture / ingest (adversary simulation)
- Quality classifier (heuristic + Rust)
- DuckDB analytics engine
- FTS5 search index
- Contamination detector
- Dedup engine
- Each tool: status (running/idle), last run, config, logs

### API Endpoint
GET /api/library/sources — all sources with counts
GET /api/library/datasets — all training files
GET /api/library/tools — all data tools with status
