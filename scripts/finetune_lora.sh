#!/bin/bash
# PlausiDen AI — LoRA Fine-Tuning Script
# SUPERSOCIETY: Actual model weight updates, not inference-only "training"
#
# Prerequisites:
#   pip install unsloth transformers datasets peft trl
#   OR: pip install axolotl
#
# Hardware: RTX 3050 Ti (4GB VRAM) → QLoRA on 3B models max
# For 7B+: use Hetzner VPS or cloud GPU
#
# Training data: /root/LFI/lfi_vsa_core/training_data_lora.jsonl
# Format: {"instruction": "...", "input": "...", "output": "...", "domain": "..."}

set -euo pipefail

DATA="/root/LFI/lfi_vsa_core/training_data_lora.jsonl"
MODEL="unsloth/Qwen2.5-3B-Instruct-bnb-4bit"  # 4-bit quantized, fits 4GB VRAM
OUTPUT="/root/LFI/models/plausiden-3b-lora"

echo "=== PlausiDen LoRA Fine-Tuning ==="
echo "Data: $DATA ($(wc -l < $DATA) examples)"
echo "Model: $MODEL"
echo "Output: $OUTPUT"
echo ""

# Check prerequisites
if ! python3 -c "import unsloth" 2>/dev/null; then
    echo "Installing unsloth..."
    pip install "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git"
    pip install --no-deps trl peft accelerate bitsandbytes
fi

python3 << 'PYEOF'
from unsloth import FastLanguageModel
from datasets import load_dataset
from trl import SFTTrainer
from transformers import TrainingArguments
import json

# Load model
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="unsloth/Qwen2.5-3B-Instruct-bnb-4bit",
    max_seq_length=2048,
    load_in_4bit=True,
)

# Apply LoRA
model = FastLanguageModel.get_peft_model(
    model,
    r=16,            # LoRA rank
    lora_alpha=16,
    lora_dropout=0,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                     "gate_proj", "up_proj", "down_proj"],
)

# Load training data
def format_prompt(example):
    return f"""### Instruction:
{example['instruction']}

### Input:
{example.get('input', '')}

### Response:
{example['output']}"""

dataset = load_dataset("json", data_files="/root/LFI/lfi_vsa_core/training_data_lora.jsonl", split="train")

# Train
trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset,
    formatting_func=format_prompt,
    args=TrainingArguments(
        output_dir="/root/LFI/models/plausiden-3b-lora",
        per_device_train_batch_size=2,
        gradient_accumulation_steps=4,
        warmup_steps=10,
        num_train_epochs=1,
        learning_rate=2e-4,
        fp16=True,
        logging_steps=10,
        save_strategy="epoch",
    ),
)
trainer.train()

# Save
model.save_pretrained("/root/LFI/models/plausiden-3b-lora")
tokenizer.save_pretrained("/root/LFI/models/plausiden-3b-lora")
print("Fine-tuning complete! Model saved to /root/LFI/models/plausiden-3b-lora")
PYEOF
