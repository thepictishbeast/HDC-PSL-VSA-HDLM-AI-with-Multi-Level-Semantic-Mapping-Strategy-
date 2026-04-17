#!/usr/bin/env python3
"""
LoRA Fine-Tuning Pipeline for PlausiDen AI
QLoRA 4-bit on Qwen2.5-1.5B using combined training data.

Requirements:
  pip3 install torch transformers peft bitsandbytes datasets accelerate trl

Before running:
  1. Stop Ollama to free VRAM: systemctl stop ollama
  2. Verify GPU free: nvidia-smi (should show ~3.5GB+ free)

Usage:
  python3 lora_train.py [--epochs N] [--batch-size N] [--model MODEL]

Output:
  /home/user/LFI-data/lora-adapter/ — LoRA adapter weights
  Then merge + export to GGUF for Ollama.
"""

import json
import sys
import os
from pathlib import Path

# Configuration
DEFAULT_MODEL = "Qwen/Qwen2.5-1.5B-Instruct"
DEFAULT_EPOCHS = 3
DEFAULT_BATCH_SIZE = 2  # Small for 4GB VRAM
DEFAULT_LR = 2e-4
LORA_R = 16
LORA_ALPHA = 32
LORA_DROPOUT = 0.05
MAX_SEQ_LEN = 512
GRADIENT_ACCUMULATION = 8  # Effective batch = 2 * 8 = 16
OUTPUT_DIR = "/home/user/LFI-data/lora-adapter"
DATA_FILE = "/home/user/LFI-data/combined_training_v1.jsonl"

def check_dependencies():
    """Verify all required packages are installed."""
    missing = []
    for pkg in ['torch', 'transformers', 'peft', 'bitsandbytes', 'datasets', 'accelerate', 'trl']:
        try:
            __import__(pkg)
        except ImportError:
            missing.append(pkg)
    if missing:
        print(f"Missing packages: {', '.join(missing)}")
        print(f"Install with: pip3 install {' '.join(missing)}")
        return False
    return True

def check_gpu():
    """Verify GPU is available and has enough VRAM."""
    try:
        import torch
        if not torch.cuda.is_available():
            print("WARNING: No CUDA GPU available. Training will be CPU-only (very slow).")
            return False
        vram = torch.cuda.get_device_properties(0).total_memory / (1024**3)
        free = (torch.cuda.get_device_properties(0).total_memory - torch.cuda.memory_allocated(0)) / (1024**3)
        print(f"GPU: {torch.cuda.get_device_name(0)}")
        print(f"VRAM: {vram:.1f}GB total, {free:.1f}GB free")
        if free < 2.0:
            print("WARNING: Less than 2GB VRAM free. Stop Ollama first: systemctl stop ollama")
            return False
        return True
    except Exception as e:
        print(f"GPU check failed: {e}")
        return False

def load_training_data(path, max_samples=50000):
    """Load and format training data for instruction tuning."""
    data = []
    with open(path) as f:
        for i, line in enumerate(f):
            if i >= max_samples:
                break
            try:
                item = json.loads(line)
                # Handle both single-turn and multi-turn formats
                if 'conversations' in item:
                    # Multi-turn: build chat format
                    turns = item['conversations']
                    if len(turns) >= 2:
                        text = ""
                        for turn in turns:
                            role = turn['role']
                            content = turn['content']
                            if role == 'user':
                                text += f"<|im_start|>user\n{content}<|im_end|>\n"
                            elif role == 'assistant':
                                text += f"<|im_start|>assistant\n{content}<|im_end|>\n"
                        if text:
                            data.append({'text': text})
                elif 'instruction' in item and 'output' in item:
                    # Single-turn Alpaca format
                    instruction = item['instruction']
                    inp = item.get('input', '')
                    output = item['output']
                    if inp:
                        prompt = f"{instruction}\n\nContext: {inp}"
                    else:
                        prompt = instruction
                    text = (
                        f"<|im_start|>user\n{prompt}<|im_end|>\n"
                        f"<|im_start|>assistant\n{output}<|im_end|>\n"
                    )
                    data.append({'text': text})
            except (json.JSONDecodeError, KeyError):
                continue
    return data

def train(model_name, epochs, batch_size, data_path, output_dir):
    """Run QLoRA fine-tuning."""
    import torch
    from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig, TrainingArguments
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from trl import SFTTrainer
    from datasets import Dataset

    print(f"\n=== LoRA Fine-Tuning ===")
    print(f"Model: {model_name}")
    print(f"Epochs: {epochs}")
    print(f"Batch size: {batch_size} (effective: {batch_size * GRADIENT_ACCUMULATION})")
    print(f"Data: {data_path}")
    print(f"Output: {output_dir}")

    # Load data
    print("\nLoading training data...")
    raw_data = load_training_data(data_path)
    print(f"Loaded {len(raw_data)} training examples")
    dataset = Dataset.from_list(raw_data)

    # 4-bit quantization config
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.bfloat16,
        bnb_4bit_use_double_quant=True,
    )

    # Load model
    print(f"\nLoading model: {model_name}")
    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        quantization_config=bnb_config,
        device_map="auto",
        trust_remote_code=True,
    )
    tokenizer = AutoTokenizer.from_pretrained(model_name, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    # Prepare for k-bit training
    model = prepare_model_for_kbit_training(model)

    # LoRA config
    lora_config = LoraConfig(
        r=LORA_R,
        lora_alpha=LORA_ALPHA,
        lora_dropout=LORA_DROPOUT,
        bias="none",
        task_type="CAUSAL_LM",
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    )

    model = get_peft_model(model, lora_config)
    trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
    total = sum(p.numel() for p in model.parameters())
    print(f"Trainable: {trainable:,} / {total:,} ({100*trainable/total:.2f}%)")

    # Training arguments
    training_args = TrainingArguments(
        output_dir=output_dir,
        num_train_epochs=epochs,
        per_device_train_batch_size=batch_size,
        gradient_accumulation_steps=GRADIENT_ACCUMULATION,
        learning_rate=DEFAULT_LR,
        weight_decay=0.01,
        warmup_ratio=0.03,
        lr_scheduler_type="cosine",
        logging_steps=10,
        save_strategy="epoch",
        bf16=torch.cuda.is_bf16_supported(),
        fp16=not torch.cuda.is_bf16_supported(),
        optim="paged_adamw_8bit",
        max_grad_norm=0.3,
        report_to="none",
        max_steps=-1,
    )

    # Trainer
    trainer = SFTTrainer(
        model=model,
        args=training_args,
        train_dataset=dataset,
        tokenizer=tokenizer,
        max_seq_length=MAX_SEQ_LEN,
    )

    print("\nStarting training...")
    trainer.train()

    # Save adapter
    print(f"\nSaving LoRA adapter to {output_dir}")
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)

    print("\n=== Training Complete ===")
    print(f"Adapter saved to: {output_dir}")
    print(f"To merge and export: python3 merge_and_export.py")


def main():
    model = DEFAULT_MODEL
    epochs = DEFAULT_EPOCHS
    batch_size = DEFAULT_BATCH_SIZE
    data_path = DATA_FILE

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--epochs" and i + 2 <= len(sys.argv[1:]):
            epochs = int(sys.argv[i + 2])
        elif arg == "--batch-size" and i + 2 <= len(sys.argv[1:]):
            batch_size = int(sys.argv[i + 2])
        elif arg == "--model" and i + 2 <= len(sys.argv[1:]):
            model = sys.argv[i + 2]
        elif arg == "--data" and i + 2 <= len(sys.argv[1:]):
            data_path = sys.argv[i + 2]

    print("=== PlausiDen AI LoRA Training Pipeline ===")

    if not os.path.exists(data_path):
        print(f"ERROR: Training data not found at {data_path}")
        print("Available data files:")
        for f in Path("/home/user/LFI-data").glob("*.jsonl"):
            lines = sum(1 for _ in open(f))
            print(f"  {f.name}: {lines:,} lines ({f.stat().st_size/1024/1024:.1f}MB)")
        sys.exit(1)

    if not check_dependencies():
        print("\nInstall dependencies first:")
        print("  pip3 install torch transformers peft bitsandbytes datasets accelerate trl")
        sys.exit(1)

    has_gpu = check_gpu()
    if not has_gpu:
        print("\nProceeding with CPU training (will be slow)...")

    Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)
    train(model, epochs, batch_size, data_path, OUTPUT_DIR)


if __name__ == "__main__":
    main()
