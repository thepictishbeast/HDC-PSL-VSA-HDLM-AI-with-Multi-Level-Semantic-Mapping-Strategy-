#!/usr/bin/env bash
# Continuous knowledge generation loop.
# Alternates between structured fact generation and self-play reasoning,
# with 5-minute breaks between batches to share Ollama time with the trainer.
set -u
cd /root/LFI/lfi_vsa_core

i=0
while true; do
    if (( i % 2 == 0 )); then
        echo "[$(date -Iseconds)] knowledge_loop: running fact generator (cycle $i)"
        python3 scripts/generate_structured_facts.py 2>&1 | tail -3
    else
        echo "[$(date -Iseconds)] knowledge_loop: running self-play (cycle $i)"
        python3 scripts/self_play.py 2>&1 | tail -3
    fi
    echo "[$(date -Iseconds)] knowledge_loop: cycle $i done, sleeping 5min"
    i=$((i+1))
    sleep 300
done
