#!/bin/bash
# cli-lint.sh — static audit for the PlausiDen CLI once it exists.
#
# The CLI is not yet built; this script lints for the anti-patterns
# called out in docs/CLI_AUDIT.md so the first commit lands correctly.
# Currently points at `cli/` (expected location); override with args.
#
# Usage:
#   scripts/cli-lint.sh                # audit cli/src
#   scripts/cli-lint.sh path/to/src
#
# Exit 0 pass, 1 on blockers. If no CLI source exists, exits 0 with INFO.

set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_TARGET="$ROOT/cli/src"
TARGETS=()
if [ $# -gt 0 ]; then
  TARGETS=("$@")
elif [ -d "$DEFAULT_TARGET" ]; then
  TARGETS=("$DEFAULT_TARGET")
else
  echo "INFO: no CLI source tree found at $DEFAULT_TARGET — audit skipped."
  echo "      Create cli/ with a Cargo manifest + src/main.rs to activate this lint."
  exit 0
fi

COLOR=""; GREEN=""; YELLOW=""; RED=""; RESET=""
if [ -t 1 ]; then
  GREEN=$'\033[0;32m'; YELLOW=$'\033[0;33m'; RED=$'\033[0;31m'; RESET=$'\033[0m'
fi

warn=0; err=0

emit() {
  local tier="$1"; local msg="$2"; local loc="$3"
  if [ "$tier" = "ERR" ]; then
    printf '%s[BLOCKER]%s %s\n  %s\n' "$RED" "$RESET" "$msg" "$loc"
    err=$((err + 1))
  else
    printf '%s[WARN]%s    %s\n  %s\n' "$YELLOW" "$RESET" "$msg" "$loc"
    warn=$((warn + 1))
  fi
}

scan() {
  local tier="$1"; local msg="$2"; local pattern="$3"
  local cmd
  if command -v rg >/dev/null 2>&1; then
    cmd="rg --no-heading --line-number --color=never \"$pattern\""
  else
    cmd="grep -rn --include='*.rs' \"$pattern\""
  fi
  local matches
  matches=$(eval "$cmd" "${TARGETS[@]}" 2>/dev/null || true)
  [ -z "$matches" ] && return
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    emit "$tier" "$msg" "$line"
  done <<< "$matches"
}

echo "CLI audit — ${TARGETS[*]}"
echo
echo "── Blockers ──"

# B1: raw ANSI escape in println!/eprintln! — breaks piping + dumb terminals.
scan "ERR" "raw ANSI escape in println!/eprintln! — gate on stdout.is_terminal() + \$NO_COLOR" \
  'println!\(.*\\x1b\[|eprintln!\(.*\\x1b\['

# B2: token read from command-line args (visible in `ps`).
scan "ERR" "API token read from args — leak via ps. Use \$XDG_CONFIG_HOME/plausiden/token" \
  'env::args\(\).*token|\.arg\(.*token\)'

# B3: unwrap on stdout/stderr write — panics on broken pipe.
scan "ERR" "unwrap() on stdout/stderr — broken pipe panics; handle io::ErrorKind::BrokenPipe" \
  '\.write_all.*\.unwrap\(\)|stdout\(\)\.flush\(\)\.unwrap\(\)'

# B4: hardcoded 80-column or 100-column width.
scan "ERR" "hardcoded terminal width — use term_size::dimensions() or crossterm::terminal::size()" \
  'let\s+width\s*=\s*(80|100|120)\s*;'

echo
echo "── Warnings ──"

# W1: print! without flush — streaming output buffers.
scan "WARN" "print! without flush — streaming chunks buffer behind stdout" \
  '^\s*print!\('

# W2: no SIGPIPE handler.
# Check if main.rs registers one.
if ! grep -q "SIGPIPE\|sigaction" "${TARGETS[@]}"/main.rs 2>/dev/null; then
  emit "WARN" "no SIGPIPE handler — piping to 'head' may panic" "${TARGETS[0]}/main.rs"
fi

# W3: Ctrl+C double-tap missing (should exit on 2x within 2s).
if ! grep -q "ctrlc_at\|double.tap\|ctrl_c_count" "${TARGETS[@]}" 2>/dev/null; then
  emit "WARN" "no Ctrl+C double-tap exit — cancel-then-exit pattern missing" "${TARGETS[0]}"
fi

# W4: missing --self-check flag.
if ! grep -q "self.check\|--self-check" "${TARGETS[@]}" 2>/dev/null; then
  emit "WARN" "no --self-check flag — users can't diagnose install issues" "${TARGETS[0]}"
fi

# W5: missing --json flag.
if ! grep -q '"--json"\|args\.json' "${TARGETS[@]}" 2>/dev/null; then
  emit "WARN" "no --json output flag — downstream tools can't consume" "${TARGETS[0]}"
fi

echo
echo "Summary: ${err} blocker(s), ${warn} warning(s)"
if [ "$err" -gt 0 ]; then
  echo "${RED}FAIL${RESET}"
  exit 1
fi
if [ "$warn" -gt 0 ]; then
  echo "${YELLOW}PASS with warnings${RESET}"
else
  echo "${GREEN}PASS${RESET}"
fi
exit 0
