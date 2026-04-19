#!/bin/bash
# mobile-lint.sh — static audit for mobile-hostile patterns.
# Fast (< 500ms). Emits WARN (review) and BLOCKER (must fix).
#
# Usage:
#   scripts/mobile-lint.sh            # audit src/
#   scripts/mobile-lint.sh src/App.tsx
#
# Exit code 0 on pass, 1 on blocker(s). CI-safe.

set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGETS=("${@:-$ROOT/src}")
COLOR=""; GREEN=""; YELLOW=""; RED=""; RESET=""
if [ -t 1 ]; then
  GREEN=$'\033[0;32m'; YELLOW=$'\033[0;33m'; RED=$'\033[0;31m'; RESET=$'\033[0m'
fi

warn=0; err=0
tmp=$(mktemp)
cleanup() { rm -f "$tmp"; }; trap cleanup EXIT

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

# Emit a tier's matches, but skip any line that contains an exemption string.
# Usage: scan TIER MESSAGE PATTERN [EXEMPTION_GREP]
scan() {
  local tier="$1"; local msg="$2"; local pattern="$3"; local exempt="${4:-}"
  > "$tmp"
  if command -v rg >/dev/null 2>&1; then
    rg --no-heading --line-number --color=never -t ts -t tsx "$pattern" "${TARGETS[@]}" > "$tmp" 2>/dev/null || true
  else
    grep -rn --include='*.ts' --include='*.tsx' "$pattern" "${TARGETS[@]}" > "$tmp" 2>/dev/null || true
  fi
  if [ -n "$exempt" ]; then
    grep -v -- "$exempt" "$tmp" > "$tmp.2" 2>/dev/null || true
    mv "$tmp.2" "$tmp"
  fi
  [ -s "$tmp" ] || return
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    emit "$tier" "$msg" "$line"
  done < "$tmp"
}

echo "Mobile audit — ${TARGETS[*]}"

# ----- BLOCKERS -----

# B1: `height: '100vh'` as a primary layout without a dvh partner in the
# SAME file is a mobile keyboard footgun. (We allow `100vh` as a property
# VALUE when '100dvh' is also present in the file — see .lfi-app-root
# class rule in App.tsx which lists both in @supports.)
echo
echo "── Blockers ──"
# Collect files that have 100vh
if command -v rg >/dev/null 2>&1; then
  vh_files=$(rg -l "100vh" "${TARGETS[@]}" 2>/dev/null | sort -u)
else
  vh_files=$(grep -rl "100vh" "${TARGETS[@]}" 2>/dev/null | sort -u)
fi
for f in $vh_files; do
  if ! grep -q "100dvh" "$f" 2>/dev/null; then
    # 100vh without any 100dvh partner — blocker.
    while IFS= read -r loc; do
      emit "ERR" "100vh without 100dvh partner (mobile keyboard footgun)" "$loc"
    done < <(grep -n "100vh" "$f")
  fi
done

# ----- WARNINGS -----

echo
echo "── Warnings ──"

# W1: Fixed large width (≥ 200px) that has no min/max paired. Heuristic:
# match `width: 'Npx'` where N >= 200 and the same source line doesn't
# already mention `maxWidth`, `minWidth`, or `flex:`.
scan "WARN" "fixed width ≥ 200px without maxWidth/minWidth/flex fallback" \
  "width: '([2-9][0-9]{2}|[1-9][0-9]{3})px'" "flex"

# W2: Horizontal flex row with > 2 children hints (gap: + alignItems)
# but no flexWrap. False-positive-prone — we only flag dense rows that
# are definitely horizontal toolbars.
scan "WARN" "flex row with alignItems:'center' but no flexWrap on the SAME line (inspect)" \
  "display: 'flex', alignItems: 'center'" "flexWrap"

# W3: Truncating with ellipsis but no title attribute gives no touch
# fallback (hover tooltips don't fire on mobile).
scan "WARN" "ellipsis without title attribute — content unreadable on touch" \
  "textOverflow: 'ellipsis'" "title"

# W4: Very small text with very small width (likely icon label hard to tap).
scan "WARN" "small font-size + fixed width (likely tight tap target)" \
  "fontSize: '9px'.*width: '1[0-9]px'"

# Informational: viewport-aware patterns we're happy to see.
if command -v rg >/dev/null 2>&1; then
  dvh_count=$(rg --no-heading --count-matches --no-filename "100dvh" "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$1} END {print s+0}')
  minmax_count=$(rg --no-heading --count-matches --no-filename "minmax\\(" "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$1} END {print s+0}')
  flex11_count=$(rg --no-heading --count-matches --no-filename "flex: '1 1 " "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$1} END {print s+0}')
else
  dvh_count=$(grep -rc "100dvh" "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$NF} END {print s+0}')
  minmax_count=$(grep -rc 'minmax(' "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$NF} END {print s+0}')
  flex11_count=$(grep -rc "flex: '1 1 " "${TARGETS[@]}" 2>/dev/null | awk -F: '{s+=$NF} END {print s+0}')
fi

echo
echo "── Mobile pattern adoption ──"
echo "${GREEN}[INFO]${RESET}    100dvh usages:       $dvh_count"
echo "${GREEN}[INFO]${RESET}    minmax() usages:     $minmax_count"
echo "${GREEN}[INFO]${RESET}    flex: '1 1 N' uses:  $flex11_count"

echo
echo "Summary: ${err} blocker(s), ${warn} warning(s)"
if [ "$err" -gt 0 ]; then
  echo "${RED}FAIL${RESET} — resolve blockers before shipping"
  exit 1
fi
if [ "$warn" -gt 0 ]; then
  echo "${YELLOW}PASS with warnings${RESET} — review flagged sites"
else
  echo "${GREEN}PASS${RESET} — no mobile anti-patterns detected"
fi
exit 0
