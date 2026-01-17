# Ollama
- install ollama for LLM management

# LLM 

## Recommended LLM setup for your machine
✅ Best choice: CPU-only, quantized
Model

Mistral 7B Instruct

or LLaMA 3 8B Instruct

Quantization

Q4_K_M (4-bit) ← ideal for you

Memory usage:

~5.5–6.5 GB RAM

Leaves room for your app + DB

Performance expectation:

~5–10 tokens/sec

Totally fine for short prompts

Your batch workflow will be minutes, not hours

## installation

ollama run mistral:7b-instruct-q4_K_M
