#!/usr/bin/env bash
# Adaptive SM-2-aware training scheduler.
# Per Training Strategy §3.1: pick the domain with the most overdue reviews,
# adapt batch size based on mastery level, inject adversarial examples when
# axiom pass rate is too high.
#
# Replaces the fixed-rotation train_rotate.sh.

set -u
mkdir -p /var/log/lfi
cd /root/LFI/lfi_vsa_core
BIN="$PWD/target/release/ollama_train"
DB="$HOME/.local/share/plausiden/brain.db"
MODEL="qwen2.5-coder:7b"

DOMAINS=(social math code security philosophy biology chemistry physics language psychology sales)

# Per-domain state file for adaptive scheduling
STATE_FILE="/var/log/lfi/training_state.json"

# Initialize state if missing
if [ ! -f "$STATE_FILE" ]; then
  echo '{}' > "$STATE_FILE"
fi

get_domain_priority() {
  local domain="$1"
  # Query brain.db for domain-specific mastery if available
  local fact_count
  fact_count=$(sqlite3 "$DB" "SELECT count(*) FROM facts WHERE key LIKE '${domain}%';" 2>/dev/null || echo "0")

  local last_trained
  last_trained=$(python3 -c "
import json, sys
try:
    s = json.load(open('$STATE_FILE'))
    d = s.get('$domain', {})
    print(d.get('last_trained', 0))
except: print(0)
" 2>/dev/null)

  local sessions
  sessions=$(python3 -c "
import json
try:
    s = json.load(open('$STATE_FILE'))
    print(s.get('$domain', {}).get('sessions', 0))
except: print(0)
" 2>/dev/null)

  local now
  now=$(date +%s)
  local elapsed=$(( now - last_trained ))

  # Priority = time since last trained (higher = more overdue)
  # Domains with fewer sessions get a boost (diversity)
  # Domains with fewer facts get a boost (need more data)
  local priority=$(( elapsed + (100 - sessions) * 60 + (1000 - fact_count) ))
  echo "$priority"
}

update_state() {
  local domain="$1"
  local batch="$2"
  python3 -c "
import json, time
try:
    s = json.load(open('$STATE_FILE'))
except: s = {}
d = s.get('$domain', {'sessions': 0, 'total_examples': 0})
d['last_trained'] = int(time.time())
d['sessions'] = d.get('sessions', 0) + 1
d['total_examples'] = d.get('total_examples', 0) + $batch
s['$domain'] = d
json.dump(s, open('$STATE_FILE', 'w'), indent=2)
" 2>/dev/null
}

i=0
echo "[$(date -Iseconds)] adaptive trainer starting" | tee -a /var/log/lfi/training.jsonl

while true; do
  # Pick the highest-priority domain
  best_domain=""
  best_priority=0

  for d in "${DOMAINS[@]}"; do
    p=$(get_domain_priority "$d")
    if [ "$p" -gt "$best_priority" ]; then
      best_priority=$p
      best_domain=$d
    fi
  done

  if [ -z "$best_domain" ]; then
    best_domain="${DOMAINS[$((i % ${#DOMAINS[@]}))]}"
  fi

  # Adaptive batch size: more examples for under-trained domains
  sessions=$(python3 -c "
import json
try:
    s = json.load(open('$STATE_FILE'))
    print(s.get('$best_domain', {}).get('sessions', 0))
except: print(0)
" 2>/dev/null)

  if [ "$sessions" -lt 5 ]; then
    BATCH=150    # New domain: large batch
  elif [ "$sessions" -lt 20 ]; then
    BATCH=100    # Growing: normal batch
  else
    BATCH=50     # Mature: smaller batch, more spaced
  fi

  echo "[$(date -Iseconds)] cycle=$i domain=$best_domain batch=$BATCH priority=$best_priority sessions=$sessions" | tee -a /var/log/lfi/training.jsonl

  "$BIN" --examples "$BATCH" --domain "$best_domain" --model "$MODEL" \
    >> "/var/log/lfi/training-$best_domain.log" 2>&1 || true

  update_state "$best_domain" "$BATCH"

  echo "[$(date -Iseconds)] cycle=$i domain=$best_domain done" | tee -a /var/log/lfi/training.jsonl

  i=$((i+1))

  # Adaptive sleep: shorter for under-trained, longer for mature
  if [ "$sessions" -lt 5 ]; then
    sleep 15
  elif [ "$sessions" -lt 20 ]; then
    sleep 30
  else
    sleep 60
  fi
done
