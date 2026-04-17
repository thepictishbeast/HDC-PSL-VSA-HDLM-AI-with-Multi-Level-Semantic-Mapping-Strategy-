# PlausiDen AI — Application Sections

## Core (v1)
1. **Chat** — AI conversation interface
2. **Classroom** — Training, data, evaluation, feedback (LMS)
3. **Admin** — System config, server stats, user management

## Knowledge (v2)
4. **Knowledge Graph** — Visual explorer of the 57M fact database
   - Interactive graph visualization (d3.js or cytoscape)
   - Search across all facts by keyword, domain, source
   - Fact detail view: key, value, quality, source, provenance
   - Domain heatmap: which domains are strong vs weak
   - Fact editor: correct/update/delete facts manually

5. **Research Lab** — Deep investigation tools
   - Causal reasoning queries (from causal.rs)
   - "Why does X cause Y?" with derivation traces
   - Provenance explorer: trace any conclusion back to source facts
   - Hypothesis testing: "What would happen if..."
   - Web research: AI searches the web and reports findings

## Security (v3)
6. **Security Center** — Defensive AI dashboard
   - WiFi Pineapple capture sessions (tier 1/2/3)
   - Adversary detection alerts
   - Network monitoring (if Pineapple connected)
   - Vulnerability scan results
   - MITRE ATT&CK mapping of detected threats

7. **Vault** — Secret management (from confidentiality kernel)
   - Stored capabilities (SSH, API keys, etc.)
   - Policy management (consent levels, use counts)
   - Audit log viewer
   - Secret rotation status

## Automation (v4)
8. **Workflows** — Task automation
   - Define recurring tasks (data ingestion, backups, scans)
   - Cron-like scheduler with visual editor
   - Workflow execution history
   - Notifications and alerts

9. **Mesh** — Peer network
   - Connected PlausiDen nodes
   - EigenTrust scores
   - Knowledge exchange status
   - Peer capabilities and facts shared

## Personal (v5)
10. **Profile** — User identity and preferences
    - Name, role, preferences
    - Theme settings
    - API key management
    - Connected devices
    - Data export/import

## Audit & Compliance (v2)
11. **Auditorium** — Central hub for ALL audits
    - Security audits (AVP-2 tier 1-6 results)
    - Code audits (unwrap count, test coverage, mutation testing)
    - Data quality audits (dedup rates, contamination, quality distribution)
    - Compliance audits (GDPR, HIPAA, SOX, PCI-DSS status)
    - Network audits (Pineapple captures, adversary tier detection)
    - Performance audits (response times, throughput, resource usage)
    - Training audits (data quality, model accuracy, regression detection)
    - Dependency audits (cargo audit, supply chain, CVE status)
    - Each audit type has: history, trends, pass/fail, findings, remediation tracking
    - Part of the orchestration platform — audits can be scheduled, auto-run, assigned to agents
    - Dashboard view: audit calendar, compliance scorecard, open findings count
