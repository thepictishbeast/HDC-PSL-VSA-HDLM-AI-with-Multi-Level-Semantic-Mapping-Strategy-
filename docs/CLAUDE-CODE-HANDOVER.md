# Claude Code Handover Package — see user message for full content
# Instance A: The Refiner — quality, dedup, PSL calibration, validation
# Instance B: The Collector — download, parse, extract, stage
# Task priority: B2 (adversarial) → B1 (Wikidata) → B3 (security) → ...
# Refiner: A1 (dedup) → A2 (temporal) → A3 (PSL calibration) → ...
# Coordination: /tmp/lfi_ingest_status.json
# Staging table: facts_staging (B writes, A validates to live)
# Reference docs: /root/LFI/docs/
