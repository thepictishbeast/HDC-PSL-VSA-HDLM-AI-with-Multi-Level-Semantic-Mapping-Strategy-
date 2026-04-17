#!/usr/bin/env bash
# Rotate through domains, 100 examples each, never stop.
set -u
mkdir -p /var/log/lfi
cd /root/LFI/lfi_vsa_core
BIN="$PWD/target/release/ollama_train"
DOMAINS=(social math code security philosophy biology chemistry physics language psychology sales)
i=0
while true; do
  d="${DOMAINS[$((i % ${#DOMAINS[@]}))]}"
  echo "[$(date -Iseconds)] cycle=$i domain=$d starting" | tee -a /var/log/lfi/training.jsonl
  "$BIN" --examples 100 --domain "$d" --model qwen2.5-coder:7b \
    >> "/var/log/lfi/training-$d.log" 2>&1 || true
  echo "[$(date -Iseconds)] cycle=$i domain=$d done" | tee -a /var/log/lfi/training.jsonl
  i=$((i+1))
  sleep 30
done
