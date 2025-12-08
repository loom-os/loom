#!/usr/bin/env python3
"""Download SWE-bench Lite dataset."""

import json
import os
from pathlib import Path
from datasets import load_dataset

# Use Chinese mirror for HuggingFace
os.environ["HF_ENDPOINT"] = "https://hf-mirror.com"

# Download SWE-bench Lite (300 task subset)
print("Downloading SWE-bench Lite dataset from hf-mirror.com...")
dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

# Save to JSON
output_path = Path(__file__).parent.parent / "datasets" / "swe-bench-lite" / "tasks.json"
output_path.parent.mkdir(parents=True, exist_ok=True)

# Convert to list of dicts
tasks = [dict(item) for item in dataset]

print(f"Downloaded {len(tasks)} tasks")
print(f"Saving to {output_path}...")

with open(output_path, "w") as f:
    json.dump(tasks, f, indent=2)

print(f"✅ Saved {len(tasks)} tasks to {output_path}")
print(f"\nSample task: {tasks[0]['instance_id']}")
print(f"Repo: {tasks[0]['repo']}")
print(f"Problem length: {len(tasks[0]['problem_statement'])} chars")
