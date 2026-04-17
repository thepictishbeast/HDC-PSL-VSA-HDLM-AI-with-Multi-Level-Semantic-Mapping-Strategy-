# WiFi Pineapple Mark VII — LFI Adversary Simulation Handoff

## Context

This is a handoff from a planning conversation to Claude Code. The user (Paul, PlausiDen Technologies) is integrating a WiFi Pineapple Mark VII into LFI's adversarial training data pipeline. The goal is to capture 802.11 frame data that lets LFI's defensive AI module distinguish **adversary skill tiers** — not just "is there an attack" but "what kind of adversary is running it."

## Hardware state

- **Device:** WiFi Pineapple Mark VII, firmware 2.1.3, hostname `mk7`
- **Connection:** USB-C ethernet to user's Kali workstation, interface `eth1` on workstation side
- **IP:** 172.16.42.1 (Pineapple), 172.16.42.42 (workstation)
- **Routing:** Policy-based isolation already configured. wlan0 (192.168.1.186) holds the default route for internet. Pineapple traffic uses a separate routing table called `pineapple`. wlan0 internet connectivity is preserved when Pineapple is plugged in.
- **Network setup files already in place:**
  - `/usr/share/iproute2/rt_tables` contains `200 pineapple`
  - `/etc/NetworkManager/conf.d/99-pineapple-unmanaged.conf` excludes Hak5 OUIs from NM
  - `/etc/udev/rules.d/80-pineapple.rules` triggers auto-config on plug-in
  - `/usr/local/bin/pineapple-up.sh` is the bring-up script

## Pineapple radio inventory (from `/etc/config/wireless`)

- `radio0` — onboard 2.4GHz (Atheros, `platform/10300000.wmac`). Currently AP mode hosting `setup-temp` (open, hidden).
- `radio1` — USB 2.4GHz on `1-1.1`. Pre-configured for monitor mode, **disabled by default**. This is the dedicated capture radio.
- `radio2` — USB 2.4GHz on `1-1.2`. Station mode, for upstream client connectivity.
- `radio3` — USB 5GHz on `1-1.3`, `htmode VHT80`. Station mode, 5GHz upstream.

Default radio MAC OUIs are `00:13:37:*` (Hak5) — these are the loudest detection signal and must be randomized for adversary realism.

## SSH access

```bash
ssh pineapple   # uses ~/.ssh/config → root@172.16.42.1 with ed25519 key
```

SSH config at `~/.ssh/config` with `Host pineapple`. Key auth via `~/.ssh/pineapple_ed25519`.

## The core mission: tiered adversary simulation

LFI needs training data that represents adversaries at multiple skill levels. A real-world threat detection system has to distinguish "kid with a default Pineapple at a coffee shop" from "nation-state operator with hardened tradecraft." If LFI only sees default-config Pineapple traffic, it will only catch Tier-1 adversaries and miss everything sophisticated.

### Tier 1 — Skill-floor adversary (script kiddie)

- Default MAC OUI `00:13:37:*` visible
- Default hostname `mk7` in DHCP requests
- Default SSIDs (`Pineapple_*`)
- TX power ~15 dBm (kept below max for home lab containment)
- Default channel (11 on 2.4GHz, 36 on 5GHz)
- PineAP filters: Deny list empty (mass collection)
- All radios broadcasting

### Tier 2 — Intermediate adversary

- MAC randomized to a real-vendor OUI (Apple, Samsung, Intel)
- Hostname changed to match cover identity (e.g., `iPhone`, `Galaxy-S23`)
- SSIDs renamed (no `Pineapple_` prefix)
- TX power ~10-12 dBm
- Channel selected to match target environment
- PineAP filters: targeted allow-lists for specific SSIDs and MAC OUIs

### Tier 3 — Advanced persistent adversary

- Full MAC randomization rotated per session, real-vendor OUI matched to cover identity
- Hostname matches cover device class
- All SSIDs custom, no Pineapple fingerprints
- Low TX power ~5-8 dBm — covert range, room-sized coverage only
- Channel selection matched to specific authorized target
- PineAP filters: tight allow-lists, deny-list including security-tool MAC OUIs
- Beacon frame profile mimicry where possible
- Session-isolated identities — each session gets fresh MAC, hostname, SSID set

## Tools to build

### 1. `pineapple-harden` — adversary identity generator

Rust binary in `tools/pineapple-harden/` that:
- Takes `--tier {1|2|3}` and `--session-id <uuid>`
- Generates realistic adversary identity for requested tier
- For Tier 2/3: picks real-vendor OUI, generates random NIC bytes, matching hostname
- SSHs to Pineapple and applies config via `uci` and `iw` commands
- Logs identity to `~/lfi/sessions/<session-id>/adversary_identity.json`

### 2. `pineapple-capture` — capture daemon

Rust binary in `tools/pineapple-capture/` that:
- Captures management frames via SSH to Pineapple's wlan1
- Streams pcap data back to workstation
- Handles graceful shutdown, proper pcap closing

### 3. `lfi-ingest-pcap` — frame-to-fact converter

Rust binary in `tools/lfi-ingest-pcap/` that:
- Parses pcap, extracts 802.11 frame metadata
- Labels each frame with adversary_tier from session metadata
- Writes structured facts to LFI's SQLite store
- Encodes as BipolarVectors for HDC layer
- **Automatically pseudonymizes ambient device MACs** not on test-device allowlist

### 4. Session orchestration

Script that wires the above together with session ID, tier, duration.

## Network self-preservation — HARD RULES

**NEVER run any of these on the workstation:**
- `airmon-ng` (any subcommand)
- `iw dev wlan0 set type monitor`
- `systemctl stop NetworkManager`
- `systemctl stop wpa_supplicant`
- `killall NetworkManager`
- `nmcli radio wifi off`
- `rfkill block wifi`
- `ip route del default`

**ALL capture operations happen on the Pineapple's wlan1 via SSH.**
The workstation's wlan0 is NEVER put into monitor mode.

Every network-touching tool runs preflight checks:
1. wlan0 must be UP in managed mode
2. Default route must point to wlan0
3. NetworkManager must be running
4. Internet must be reachable (ping 8.8.8.8)

## Operational constraints

1. Scope: user's home lab only. `authorization_basis` field mandatory on every session.
2. Every session has full provenance metadata.
3. TX power defaults to low (5-10 dBm Tier 3, 15 dBm max).
4. Ambient device MACs auto-pseudonymized at ingest (SHA256 hash with session salt).
5. Capture-side parsing minimal; heavy parsing on workstation side.
6. SSH sessions use tmux/nohup for resilience against drops.

## LFI integration points

- `lfi_vsa_core/src/hdc/sensory.rs` — add `wifi_frame.rs` encoder
- `persistence.rs` — extend facts table or add `wifi_facts` table
- Domain: `wireless_security` or `network_forensics`
- Adversarial corpus: tier 0.95. Ambient: tier 0.85. Test device: tier 0.90.
- Every wifi_fact produces `TracedDerivation` for reasoning provenance.
- Causal edges: "adversary technique X observed" → "expected client outcome Y"
