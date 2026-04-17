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
