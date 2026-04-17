#!/usr/bin/env python3
"""
Merge LoRA adapter into base model and export as GGUF for Ollama.

Usage:
  1. Run lora_train.py first to produce the adapter
  2. python3 merge_and_export.py
  3. ollama create plausiden-ai -f Modelfile
"""

import os
import sys
from pathlib import Path

ADAPTER_DIR = "/home/user/LFI-data/lora-adapter"
MERGED_DIR = "/home/user/LFI-data/merged-model"
GGUF_PATH = "/home/user/LFI-data/plausiden-ai.gguf"
BASE_MODEL = "Qwen/Qwen2.5-1.5B-Instruct"

def main():
    print("=== Merge LoRA + Export GGUF ===")

    try:
        from transformers import AutoModelForCausalLM, AutoTokenizer
        from peft import PeftModel
        import torch
    except ImportError:
        print("Install: pip3 install torch transformers peft")
        sys.exit(1)

    if not Path(ADAPTER_DIR).exists():
        print(f"ERROR: Adapter not found at {ADAPTER_DIR}")
        print("Run lora_train.py first.")
        sys.exit(1)

    # Load base model
    print(f"Loading base model: {BASE_MODEL}")
    model = AutoModelForCausalLM.from_pretrained(
        BASE_MODEL, torch_dtype=torch.float16, device_map="cpu"
    )
    tokenizer = AutoTokenizer.from_pretrained(BASE_MODEL)

    # Load and merge adapter
    print(f"Loading LoRA adapter from: {ADAPTER_DIR}")
    model = PeftModel.from_pretrained(model, ADAPTER_DIR)
    print("Merging adapter into base model...")
    model = model.merge_and_unload()

    # Save merged model
    print(f"Saving merged model to: {MERGED_DIR}")
    Path(MERGED_DIR).mkdir(parents=True, exist_ok=True)
    model.save_pretrained(MERGED_DIR)
    tokenizer.save_pretrained(MERGED_DIR)

    # Convert to GGUF using llama.cpp
    print("\nTo convert to GGUF:")
    print(f"  python3 llama.cpp/convert_hf_to_gguf.py {MERGED_DIR} --outfile {GGUF_PATH} --outtype q4_k_m")
    print(f"\nThen create Ollama model:")
    print(f"  echo 'FROM {GGUF_PATH}' > Modelfile")
    print(f"  echo 'PARAMETER temperature 0.7' >> Modelfile")
    print(f"  echo 'SYSTEM You are PlausiDen AI...' >> Modelfile")
    print(f"  ollama create plausiden-ai -f Modelfile")

    # Write Modelfile template
    modelfile = Path("/home/user/LFI-data/Modelfile")
    modelfile.write_text(f"""FROM {GGUF_PATH}
PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER num_predict 512
SYSTEM You are PlausiDen AI, a sovereign knowledge system built by PlausiDen Technologies LLC. You have deep expertise in cybersecurity, programming, systems administration, and general knowledge. You answer questions thoroughly and accurately, showing exact commands when asked to perform tasks. You are helpful, direct, and never evasive.
""")
    print(f"\nModelfile written to {modelfile}")
    print("\n=== Done ===")


if __name__ == "__main__":
    main()
