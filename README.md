# PlausiDen AI

**Sovereign neurosymbolic AI platform.** 58.8M facts, 360 sources, 40 domains. Grade B+.

Built by [PlausiDen Technologies LLC](https://plausiden.com).

## Architecture

```
UI (Web + Desktop + Android) → API Layer → Intelligence → HDC/VSA → Security → Data → Mesh
```

See [PLAUSIDEN_LAYERS.md](docs/PLAUSIDEN_LAYERS.md) for the complete 9-layer architecture.

## Quick Start

```bash
# Start the server
systemctl start plausiden-server    # port 3000

# Start the dashboard  
cd lfi_dashboard && npm run dev     # port 5173

# Start the orchestrator
systemctl start plausiden-orchestrator  # port 3001

# Launch desktop app
plausiden-desktop
```

## Key Stats

| Metric | Value |
|--------|-------|
| Total facts | 58,770,317 |
| Sources | 360 |
| Domains | 40 |
| Conversational facts | 1,457,738 |
| Cybersecurity facts | 634,185 |
| Tests passing | 1,802 |
| Grade | B+ (77.6) |

## Application Sections

| Section | Name | Purpose |
|---------|------|---------|
| Chat | **Agora** | Idea exchange with AI |
| Training | **Classroom** | Training data, evaluation, feedback |
| Audits | **Auditorium** | All audit types, compliance |
| Security | **Colosseum** | Adversarial combat, pentesting |
| Secrets | **Vault** | Credential capabilities |
| Knowledge | **Library** | 58.8M fact browser |
| AI Fleet | **Fleet** | Agent orchestration |
| Admin | **Admin** | System management |

## Tools

- `pineapple-harden` — WiFi Pineapple adversary identity generator
- `pineapple-capture` — 802.11 frame capture daemon
- `lfi-ingest-pcap` — frame-to-fact converter
- `plausiden-orchestrator` — Claude fleet task queue
- `plausiden-desktop` — Tauri desktop app (X11 + Wayland)

## License

Proprietary — PlausiDen Technologies LLC
