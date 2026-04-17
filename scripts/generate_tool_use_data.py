#!/usr/bin/env python3
"""
Tool-Use Training Data Generator for PlausiDen AI.
Creates structured examples showing how to use system tools,
navigate the OS, call APIs, and execute commands.

These are manually-crafted high-quality examples (no Ollama needed for the base set).

Usage: python3 generate_tool_use_data.py [--output FILE]
"""

import json
import sys
from pathlib import Path

DEFAULT_OUTPUT = "/home/user/LFI-data/tool_use_training.jsonl"

# Each example: (instruction, output, tool_calls, domain)
TOOL_USE_EXAMPLES = [
    # === File Operations ===
    (
        "List all Rust source files in the current project",
        "I'll search for all `.rs` files recursively:\n\n```bash\nfind . -name '*.rs' -type f | sort\n```\n\nAlternatively, using `fd` for a faster search:\n```bash\nfd -e rs\n```",
        [{"tool": "bash", "command": "find . -name '*.rs' -type f | sort"}],
        "file_operations",
    ),
    (
        "Find all files modified in the last 24 hours",
        "Using `find` with the `-mtime` flag:\n\n```bash\nfind . -type f -mtime -1\n```\n\nThis finds all files modified less than 1 day ago. For more precision (last 24 hours exactly):\n```bash\nfind . -type f -newermt '24 hours ago'\n```",
        [{"tool": "bash", "command": "find . -type f -mtime -1"}],
        "file_operations",
    ),
    (
        "Create a directory structure for a new Rust project",
        "I'll create the standard Rust project layout:\n\n```bash\nmkdir -p src/{bin,lib} tests docs\ntouch src/lib.rs src/main.rs src/bin/.gitkeep tests/.gitkeep\necho '[package]\nname = \"my_project\"\nversion = \"0.1.0\"\nedition = \"2021\"' > Cargo.toml\n```\n\nThis creates:\n- `src/` — main source directory\n- `src/bin/` — additional binary targets\n- `tests/` — integration tests\n- `docs/` — documentation",
        [{"tool": "bash", "command": "mkdir -p src/{bin,lib} tests docs"}],
        "file_operations",
    ),
    (
        "Search for the string 'TODO' in all Python files",
        "Using `grep` with recursive search and line numbers:\n\n```bash\ngrep -rn 'TODO' --include='*.py' .\n```\n\nOr using `ripgrep` for faster results:\n```bash\nrg 'TODO' -t py\n```\n\nThis will show each file, line number, and the matching line containing 'TODO'.",
        [{"tool": "grep", "pattern": "TODO", "file_type": "py"}],
        "file_operations",
    ),
    (
        "Compare two configuration files and show the differences",
        "Using `diff` to compare files:\n\n```bash\ndiff -u config_old.yaml config_new.yaml\n```\n\nThe `-u` flag gives unified diff format. For a colorized side-by-side comparison:\n```bash\ndiff --color -y config_old.yaml config_new.yaml\n```\n\nOr use `colordiff` for better highlighting.",
        [{"tool": "bash", "command": "diff -u config_old.yaml config_new.yaml"}],
        "file_operations",
    ),
    (
        "Securely delete a file so it can't be recovered",
        "Using `shred` to overwrite the file before deleting:\n\n```bash\nshred -vfz -n 5 sensitive_file.txt\nrm sensitive_file.txt\n```\n\n- `-v`: verbose (show progress)\n- `-f`: force permissions\n- `-z`: overwrite with zeros last pass\n- `-n 5`: 5 overwrite passes\n\n**Note:** On SSDs and copy-on-write filesystems, `shred` may not be effective. For SSDs, use full-disk encryption (LUKS) and securely erase the key instead.",
        [{"tool": "bash", "command": "shred -vfz -n 5 sensitive_file.txt && rm sensitive_file.txt"}],
        "file_operations",
    ),

    # === System Diagnostics ===
    (
        "Check how much disk space is being used",
        "Using `df` for filesystem-level overview and `du` for directory-level detail:\n\n```bash\n# Filesystem overview\ndf -h\n\n# Top space consumers in current directory\ndu -sh * | sort -rh | head -20\n\n# Find largest files system-wide\nfind / -type f -size +100M 2>/dev/null | head -20\n```\n\n`-h` gives human-readable sizes (GB, MB, etc.).",
        [{"tool": "bash", "command": "df -h"}, {"tool": "bash", "command": "du -sh * | sort -rh | head -20"}],
        "system_diagnostics",
    ),
    (
        "Check memory usage and what processes are using the most",
        "Using `free` for overall memory and `ps` for per-process:\n\n```bash\n# Overall memory usage\nfree -h\n\n# Top 10 memory consumers\nps aux --sort=-%mem | head -11\n\n# Or use top in batch mode\ntop -b -n1 -o %MEM | head -20\n```\n\nFor detailed memory mapping of a specific process:\n```bash\npmap -x <PID>\n```",
        [{"tool": "bash", "command": "free -h"}, {"tool": "bash", "command": "ps aux --sort=-%mem | head -11"}],
        "system_diagnostics",
    ),
    (
        "Check what processes are using the most CPU",
        "Using `ps` sorted by CPU usage:\n\n```bash\nps aux --sort=-%cpu | head -11\n```\n\nFor real-time monitoring:\n```bash\ntop -b -n1 -o %CPU | head -20\n```\n\nTo track a specific process over time:\n```bash\npidstat -p <PID> 1 10  # Sample every 1 second, 10 times\n```",
        [{"tool": "bash", "command": "ps aux --sort=-%cpu | head -11"}],
        "system_diagnostics",
    ),
    (
        "Check network connectivity and open ports",
        "Multiple diagnostic commands:\n\n```bash\n# Check internet connectivity\nping -c 3 8.8.8.8\n\n# Show listening ports\nss -tlnp\n\n# Show all active connections\nss -tunp\n\n# Check DNS resolution\ndig google.com +short\n\n# Show network interfaces\nip addr show\n```",
        [{"tool": "bash", "command": "ss -tlnp"}, {"tool": "bash", "command": "ip addr show"}],
        "system_diagnostics",
    ),
    (
        "Check system uptime and load average",
        "```bash\n# Uptime with load average\nuptime\n\n# More detailed system info\nuname -a\n\n# CPU info\nnproc  # number of CPUs\nlscpu  # detailed CPU info\n```\n\nThe load average numbers show 1-minute, 5-minute, and 15-minute averages. A load of 1.0 per CPU core means full utilization.",
        [{"tool": "bash", "command": "uptime"}],
        "system_diagnostics",
    ),

    # === Git Operations ===
    (
        "Show me what changed in the last commit",
        "Using `git show` to view the most recent commit:\n\n```bash\n# Full diff of last commit\ngit show HEAD\n\n# Just the stats (files changed)\ngit show --stat HEAD\n\n# Only the file names\ngit diff-tree --no-commit-id --name-only -r HEAD\n```",
        [{"tool": "bash", "command": "git show --stat HEAD"}],
        "git_operations",
    ),
    (
        "Create a new branch, make changes, and create a pull request",
        "Step-by-step:\n\n```bash\n# 1. Create and switch to new branch\ngit checkout -b feature/my-new-feature\n\n# 2. Make your changes, then stage them\ngit add src/modified_file.rs\n\n# 3. Commit with descriptive message\ngit commit -m \"Add new feature: description of what it does\"\n\n# 4. Push branch to remote\ngit push -u origin feature/my-new-feature\n\n# 5. Create PR using GitHub CLI\ngh pr create --title \"Add my new feature\" --body \"Description of changes\"\n```",
        [
            {"tool": "bash", "command": "git checkout -b feature/my-new-feature"},
            {"tool": "bash", "command": "git push -u origin feature/my-new-feature"},
            {"tool": "bash", "command": "gh pr create --title 'Add my new feature'"},
        ],
        "git_operations",
    ),
    (
        "Find which commit introduced a bug",
        "Using `git bisect` to binary-search for the bad commit:\n\n```bash\n# Start bisecting\ngit bisect start\n\n# Mark current commit as bad\ngit bisect bad\n\n# Mark a known good commit\ngit bisect good v1.0.0\n\n# Git checks out a middle commit — test it, then mark:\ngit bisect good  # or\ngit bisect bad\n\n# Repeat until Git finds the first bad commit\n# When done:\ngit bisect reset\n```\n\nFor automated bisect with a test script:\n```bash\ngit bisect start HEAD v1.0.0 -- \ngit bisect run cargo test --lib my_test\n```",
        [{"tool": "bash", "command": "git bisect start"}],
        "git_operations",
    ),
    (
        "Show who last modified each line of a file",
        "Using `git blame`:\n\n```bash\n# Full blame with timestamps\ngit blame src/main.rs\n\n# Blame specific lines (50-70)\ngit blame -L 50,70 src/main.rs\n\n# Ignore whitespace changes\ngit blame -w src/main.rs\n\n# Show original author (ignoring moves/copies)\ngit blame -C -C src/main.rs\n```",
        [{"tool": "bash", "command": "git blame src/main.rs"}],
        "git_operations",
    ),

    # === Database Operations ===
    (
        "Create a new SQLite database with a users table and insert some data",
        "Using the `sqlite3` command-line tool:\n\n```bash\nsqlite3 myapp.db <<'SQL'\nCREATE TABLE IF NOT EXISTS users (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    username TEXT NOT NULL UNIQUE,\n    email TEXT NOT NULL,\n    created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n);\n\nINSERT INTO users (username, email) VALUES ('alice', 'alice@example.com');\nINSERT INTO users (username, email) VALUES ('bob', 'bob@example.com');\n\nSELECT * FROM users;\nSQL\n```",
        [{"tool": "bash", "command": "sqlite3 myapp.db 'CREATE TABLE users ...'"}],
        "database_operations",
    ),
    (
        "Check the size and structure of a SQLite database",
        "```bash\n# Show all tables\nsqlite3 brain.db '.tables'\n\n# Show schema for a specific table\nsqlite3 brain.db '.schema facts'\n\n# Count rows in a table\nsqlite3 brain.db 'SELECT COUNT(*) FROM facts;'\n\n# Check database file size\nls -lh brain.db\n\n# Show database stats\nsqlite3 brain.db 'PRAGMA page_count; PRAGMA page_size;'\n\n# Check WAL mode and journal\nsqlite3 brain.db 'PRAGMA journal_mode;'\n```",
        [{"tool": "bash", "command": "sqlite3 brain.db '.tables'"}, {"tool": "bash", "command": "sqlite3 brain.db '.schema facts'"}],
        "database_operations",
    ),
    (
        "Export SQLite data to CSV format",
        "```bash\nsqlite3 -header -csv brain.db 'SELECT key, value, confidence FROM facts WHERE domain=\"security\" LIMIT 100;' > security_facts.csv\n```\n\nFor more control over the format:\n```bash\nsqlite3 brain.db <<'SQL'\n.mode csv\n.headers on\n.output export.csv\nSELECT * FROM facts WHERE quality_score > 0.8 LIMIT 1000;\n.output stdout\nSQL\n```",
        [{"tool": "bash", "command": "sqlite3 -header -csv brain.db 'SELECT ...' > export.csv"}],
        "database_operations",
    ),

    # === Network Operations ===
    (
        "Make an API request and parse the JSON response",
        "Using `curl` with JSON parsing:\n\n```bash\n# Simple GET request\ncurl -s https://api.example.com/data | python3 -m json.tool\n\n# POST with JSON body\ncurl -s -X POST https://api.example.com/submit \\\n  -H 'Content-Type: application/json' \\\n  -d '{\"key\": \"value\"}'\n\n# Extract specific field with jq\ncurl -s https://api.example.com/data | jq '.results[].name'\n\n# With authentication\ncurl -s -H 'Authorization: Bearer TOKEN' https://api.example.com/protected\n```",
        [{"tool": "bash", "command": "curl -s https://api.example.com/data | jq ."}],
        "network_operations",
    ),
    (
        "Download a file and verify its integrity",
        "Download with `wget` and verify checksum:\n\n```bash\n# Download the file\nwget -q https://example.com/release.tar.gz\n\n# Download the checksum file\nwget -q https://example.com/release.tar.gz.sha256\n\n# Verify integrity\nsha256sum -c release.tar.gz.sha256\n```\n\nOr compute and compare manually:\n```bash\nexpected=\"abc123...\"  # known good hash\nactual=$(sha256sum release.tar.gz | cut -d' ' -f1)\n[ \"$actual\" = \"$expected\" ] && echo 'OK' || echo 'CORRUPTED!'\n```",
        [{"tool": "bash", "command": "wget -q URL && sha256sum -c file.sha256"}],
        "network_operations",
    ),

    # === Security Operations ===
    (
        "Check file permissions and fix them for a web application",
        "Audit and fix permissions:\n\n```bash\n# Check current permissions\nls -la /var/www/myapp/\n\n# Fix ownership (web server user)\nchown -R www-data:www-data /var/www/myapp/\n\n# Directories: 755 (rwxr-xr-x)\nfind /var/www/myapp -type d -exec chmod 755 {} \\;\n\n# Files: 644 (rw-r--r--)\nfind /var/www/myapp -type f -exec chmod 644 {} \\;\n\n# Scripts: 750 (rwxr-x---)\nchmod 750 /var/www/myapp/scripts/*.sh\n\n# Config with secrets: 600 (rw-------)\nchmod 600 /var/www/myapp/.env\n```",
        [{"tool": "bash", "command": "ls -la /var/www/myapp/"}],
        "security_operations",
    ),
    (
        "Generate a strong random password",
        "Several methods for generating secure passwords:\n\n```bash\n# Using /dev/urandom (most portable)\nhead -c 32 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9!@#$%' | head -c 24\n\n# Using openssl\nopenssl rand -base64 24\n\n# Using Python\npython3 -c \"import secrets; print(secrets.token_urlsafe(24))\"\n\n# Generate a passphrase (easier to remember)\nshuf -n 5 /usr/share/dict/words | tr '\\n' '-'\n```\n\n**Never** use `$RANDOM`, `date`, or predictable sources for passwords.",
        [{"tool": "bash", "command": "openssl rand -base64 24"}],
        "security_operations",
    ),
    (
        "Check for failed SSH login attempts",
        "```bash\n# Check auth log for failed attempts\ngrep 'Failed password' /var/log/auth.log | tail -20\n\n# Count failed attempts per IP\ngrep 'Failed password' /var/log/auth.log | grep -oP '\\d+\\.\\d+\\.\\d+\\.\\d+' | sort | uniq -c | sort -rn | head -10\n\n# Check for brute force (>10 attempts from same IP)\ngrep 'Failed password' /var/log/auth.log | grep -oP '\\d+\\.\\d+\\.\\d+\\.\\d+' | sort | uniq -c | sort -rn | awk '$1 > 10'\n\n# Last successful logins\nlast -n 10\n```",
        [{"tool": "bash", "command": "grep 'Failed password' /var/log/auth.log | tail -20"}],
        "security_operations",
    ),

    # === Process Management ===
    (
        "Start a background service and ensure it restarts on failure",
        "Using systemd for service management:\n\n```bash\n# Create service file\ncat > /etc/systemd/system/myapp.service <<'EOF'\n[Unit]\nDescription=My Application\nAfter=network.target\n\n[Service]\nType=simple\nUser=appuser\nWorkingDirectory=/opt/myapp\nExecStart=/opt/myapp/server\nRestart=always\nRestartSec=5\nEnvironment=RUST_LOG=info\n\n[Install]\nWantedBy=multi-user.target\nEOF\n\n# Enable and start\nsystemctl daemon-reload\nsystemctl enable myapp\nsystemctl start myapp\n\n# Check status\nsystemctl status myapp\njournalctl -u myapp -f  # follow logs\n```",
        [
            {"tool": "bash", "command": "systemctl daemon-reload"},
            {"tool": "bash", "command": "systemctl enable myapp && systemctl start myapp"},
        ],
        "process_management",
    ),

    # === Ollama/AI Operations ===
    (
        "Check which Ollama models are available and run a query",
        "```bash\n# List available models\nollama list\n\n# Pull a new model\nollama pull qwen2.5-coder:7b\n\n# Run a query\ncurl -s http://localhost:11434/api/generate \\\n  -d '{\"model\":\"qwen2.5-coder:7b\",\"prompt\":\"Explain Rust ownership\",\"stream\":false}' | \\\n  python3 -c \"import json,sys; print(json.load(sys.stdin)['response'])\"\n\n# Check Ollama status\ncurl -s http://localhost:11434/api/tags | python3 -m json.tool\n```",
        [{"tool": "bash", "command": "ollama list"}, {"tool": "bash", "command": "curl -s http://localhost:11434/api/tags"}],
        "ai_operations",
    ),
    (
        "Monitor Ollama GPU usage while running inference",
        "```bash\n# In one terminal, watch GPU usage\nwatch -n 1 nvidia-smi\n\n# Or for a one-shot check\nnvidia-smi --query-gpu=name,memory.used,memory.total,utilization.gpu --format=csv\n\n# Monitor Ollama process specifically\npidstat -p $(pgrep -f ollama) 1\n\n# Check VRAM usage per process\nnvidia-smi pmon -s um -c 5\n```",
        [{"tool": "bash", "command": "nvidia-smi --query-gpu=name,memory.used,memory.total,utilization.gpu --format=csv"}],
        "ai_operations",
    ),

    # === Package Management ===
    (
        "Update all system packages safely",
        "```bash\n# Update package lists\nsudo apt update\n\n# Show what would be upgraded (dry run)\napt list --upgradable\n\n# Upgrade (safe mode — won't remove packages)\nsudo apt upgrade -y\n\n# For Rust toolchain\nrustup update\n\n# For Python packages\npip3 list --outdated\npip3 install --upgrade <package>\n\n# For Node.js\nnpm outdated\nnpm update\n```\n\n**Tip:** Always review what's being upgraded before confirming, especially on production systems.",
        [{"tool": "bash", "command": "sudo apt update && apt list --upgradable"}],
        "package_management",
    ),
    (
        "Check for known security vulnerabilities in Rust dependencies",
        "```bash\n# Install and run cargo-audit\ncargo install cargo-audit\ncargo audit\n\n# For more detail including advisory info\ncargo audit --json | python3 -m json.tool\n\n# Check dependency tree for specific crate\ncargo tree -p suspicious-crate\n\n# Run cargo-deny for comprehensive checks\ncargo install cargo-deny\ncargo deny check\n\n# Check for unsafe code usage\ncargo install cargo-geiger\ncargo geiger\n```",
        [{"tool": "bash", "command": "cargo audit"}, {"tool": "bash", "command": "cargo geiger"}],
        "package_management",
    ),

    # === Text Processing ===
    (
        "Parse a log file to find the top error types",
        "```bash\n# Extract error lines and count unique types\ngrep -i 'error' server.log | \\\n  sed 's/.*ERROR //' | \\\n  sed 's/\\[.*\\]//' | \\\n  sort | uniq -c | sort -rn | head -20\n\n# Find errors in the last hour\nawk -v d=\"$(date -d '1 hour ago' '+%Y-%m-%d %H')\" '$0 >= d' server.log | grep -i error\n\n# Count errors per minute (for rate analysis)\ngrep -i error server.log | \\\n  awk '{print $1\" \"$2}' | \\\n  cut -d: -f1,2 | \\\n  sort | uniq -c\n```",
        [{"tool": "bash", "command": "grep -i 'error' server.log | sort | uniq -c | sort -rn | head -20"}],
        "text_processing",
    ),
    (
        "Extract all email addresses from a file",
        "Using `grep` with regex:\n\n```bash\ngrep -oE '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}' input.txt | sort -u\n```\n\nOr using Python for more robust parsing:\n```python\nimport re\nwith open('input.txt') as f:\n    emails = set(re.findall(r'[\\w.+-]+@[\\w-]+\\.[\\w.]+', f.read()))\n    for e in sorted(emails):\n        print(e)\n```",
        [{"tool": "bash", "command": "grep -oE '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}' input.txt"}],
        "text_processing",
    ),
]


def main():
    output_path = DEFAULT_OUTPUT
    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--output" and i + 2 <= len(sys.argv[1:]):
            output_path = sys.argv[i + 2]

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"=== Tool-Use Training Data Generator ===")
    print(f"Output: {output_path}")
    print(f"Examples: {len(TOOL_USE_EXAMPLES)}")

    with open(output_path, "w") as f:
        for instruction, output, tool_calls, domain in TOOL_USE_EXAMPLES:
            pair = {
                "instruction": instruction,
                "input": "",
                "output": output,
                "tool_calls": tool_calls,
                "source": "tool_use_training",
                "domain": domain,
            }
            f.write(json.dumps(pair) + "\n")

    print(f"=== Done: {len(TOOL_USE_EXAMPLES)} tool-use examples written ===")


if __name__ == "__main__":
    main()
