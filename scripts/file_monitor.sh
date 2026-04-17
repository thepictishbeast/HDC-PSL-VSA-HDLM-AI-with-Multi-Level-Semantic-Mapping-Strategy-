#!/bin/bash
# File integrity monitor — checks key files for modifications
# Run periodically to detect unauthorized changes

WATCHED_FILES=(
    "/root/LFI/lfi_vsa_core/src/api.rs"
    "/root/LFI/lfi_vsa_core/src/agent.rs"
    "/root/LFI/lfi_vsa_core/src/persistence.rs"
    "/root/LFI/lfi_vsa_core/Cargo.toml"
    "/root/LFI/lfi_dashboard/src/App.tsx"
)

HASH_FILE="/var/log/lfi/file_hashes.json"
mkdir -p /var/log/lfi

# Compute current hashes
echo "{" > /tmp/current_hashes.json
for f in "${WATCHED_FILES[@]}"; do
    if [ -f "$f" ]; then
        hash=$(sha256sum "$f" | cut -d' ' -f1)
        echo "  \"$f\": \"$hash\"," >> /tmp/current_hashes.json
    fi
done
echo "  \"timestamp\": \"$(date -Iseconds)\"" >> /tmp/current_hashes.json
echo "}" >> /tmp/current_hashes.json

# Compare with previous
if [ -f "$HASH_FILE" ]; then
    changes=$(diff <(python3 -c "import json; [print(f'{k}: {v}') for k,v in sorted(json.load(open('$HASH_FILE')).items()) if k != 'timestamp']" 2>/dev/null) \
                   <(python3 -c "import json; [print(f'{k}: {v}') for k,v in sorted(json.load(open('/tmp/current_hashes.json')).items()) if k != 'timestamp']" 2>/dev/null))
    if [ -n "$changes" ]; then
        echo "[monitor] $(date) FILES CHANGED:"
        echo "$changes"
    fi
fi

cp /tmp/current_hashes.json "$HASH_FILE"
