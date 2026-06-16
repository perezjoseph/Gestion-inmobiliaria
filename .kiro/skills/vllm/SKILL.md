---
name: serving-llms-vllm
description: Serves LLMs with high throughput using vLLM's PagedAttention and continuous batching. Use when deploying production LLM APIs, optimizing inference latency/throughput, or serving models with limited GPU memory. Supports OpenAI-compatible endpoints, quantization (GPTQ/AWQ/FP8/INT4/MXFP4), tensor parallelism, data parallelism, Intel GPU (Arc B60/A770) via llm-scaler-vllm, multi-node distributed deployment, embeddings, rerankers, and multi-modal models. Make sure to use this skill whenever the user mentions vllm, inference serving, model deployment, LLM API, serving models, GPU inference, quantized serving, PagedAttention, continuous batching, Intel Arc GPU inference, llm-scaler, embeddings serving, reranker deployment, multi-modal serving, whisper transcription, or asks about high-throughput LLM serving — even if they don't explicitly say "vLLM".
version: 2.0.0
author: Orchestra Research
license: MIT
tags: [vLLM, Inference Serving, PagedAttention, Continuous Batching, High Throughput, Production, OpenAI API, Quantization, Tensor Parallelism, Intel GPU, Arc B60, llm-scaler, Data Parallelism, Multi-Modal, Embeddings, Reranker]
dependencies: [vllm, torch, transformers]
---

# vLLM — High-Performance LLM Serving

vLLM achieves 24x higher throughput than standard transformers through PagedAttention (block-based KV cache) and continuous batching. It exposes an OpenAI-compatible API, making it a drop-in replacement for most LLM serving needs.

## Platform selection

Pick your path based on hardware:

| Hardware | Image / Install | Key differences |
|----------|----------------|-----------------|
| NVIDIA (A10/A100/H100) | `pip install vllm` or `vllm/vllm-openai` Docker | Primary platform, widest feature support |
| Intel Arc B60 | `intel/llm-scaler-vllm:<VERSION>` Docker | Requires `--enforce-eager`, `--block-size 64`, `--disable-sliding-window`; adds `--dp`, `sym_int4`, `mxfp4` |
| Intel Arc A770 | `intelanalytics/multi-arc-serving:latest` Docker | See [ipex-llm docs](https://github.com/intel/ipex-llm/blob/main/docs/mddocs/DockerGuides/vllm_docker_quickstart.md) |

When working with Intel Arc GPUs, read [references/intel-gpu.md](references/intel-gpu.md) for the full deployment guide — it covers bare-metal setup, Docker container usage, quantization options, data parallelism, multi-node deployment, and load balancer solutions.

## Quick start

**NVIDIA — serve a model:**
```bash
vllm serve meta-llama/Llama-3-8B-Instruct \
  --gpu-memory-utilization 0.9 \
  --port 8000 --host 0.0.0.0
```

**Intel Arc B60 — serve a model (inside llm-scaler-vllm container):**
```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve /llm/models/DeepSeek-R1-Distill-Qwen-7B \
  --served-model-name DeepSeek-R1-Distill-Qwen-7B \
  --dtype float16 --enforce-eager \
  --port 8000 --host 0.0.0.0 \
  --trust-remote-code --disable-sliding-window \
  --gpu-memory-util 0.9 --max-num-batched-tokens 8192 \
  --max-model-len 8192 --block-size 64 \
  --quantization fp8 -tp 1
```

**Query (works with either platform):**
```python
from openai import OpenAI
client = OpenAI(base_url="http://localhost:8000/v1", api_key="EMPTY")
r = client.chat.completions.create(
    model="DeepSeek-R1-Distill-Qwen-7B",
    messages=[{"role": "user", "content": "Hello!"}]
)
print(r.choices[0].message.content)
```

## Choosing a quantization method

| Method | Flag | Compression | Best for |
|--------|------|-------------|----------|
| AWQ | `--quantization awq` | 4-bit | NVIDIA, 70B models, production |
| GPTQ | `--quantization gptq` | 4-bit | NVIDIA, wide model support |
| FP8 | `--quantization fp8` | 8-bit | H100 (NVIDIA) or Intel Arc |
| INT4 | `--quantization sym_int4` | 4-bit | Intel Arc only |
| MXFP4 | `--quantization mxfp4` | 4-bit | Intel Arc, gpt-oss-20b/120b only |

Pre-quantized AWQ/GPTQ models are auto-detected — no flag needed.

For large models that OOM during quantization on Intel: `export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1`

Deep dive: [references/quantization.md](references/quantization.md)

## Scaling strategies

**Tensor parallelism** — split model across GPUs (always power-of-2):
```bash
vllm serve MODEL -tp 4
```

**Data parallelism** (Intel Arc only) — near-linear throughput scaling:
```bash
vllm serve MODEL --dp 2   # 1.9x throughput with 2 GPUs
```

**Pipeline parallelism** — for multi-node:
```bash
vllm serve MODEL -tp 2 -pp 2 --distributed-executor-backend ray
```

**Load balancer** (Intel Arc) — alternative to DP with periodic rotation:
```bash
cd vllm/docker-compose/load_balancer && docker compose up -d
```

## Serving embeddings and rerankers

vLLM supports non-generative tasks via `--task`:

```bash
# Embeddings (bge-m3, bge-large-en, Qwen3-Embedding-8B)
vllm serve /llm/models/bge-m3 --task embed --served-model-name bge-m3

# Reranking (bge-reranker-base/v2-m3, Qwen3-Reranker-8B)
vllm serve /llm/models/bge-reranker-base --task score --served-model-name bge-reranker-base
```

Query endpoints: `/v1/embeddings` and `/v1/rerank` (OpenAI-compatible).

## Multi-modal models

Serve vision/audio/omni models with `--allowed-local-media-path`:

```bash
vllm serve /llm/models/Qwen2.5-VL-7B-Instruct \
  --served-model-name Qwen2.5-VL-7B-Instruct \
  --allowed-local-media-path /llm/models/test \
  --max-num-batched-tokens 5120 --max-model-len 5120 \
  --quantization fp8
```

Supported: Qwen2.5-VL, Qwen3-VL, InternVL3, MiniCPM-V, Qwen2.5-Omni (audio+vision), whisper (transcription).

## Performance tuning checklist

1. Set `--gpu-memory-utilization 0.9` (higher = more KV cache capacity)
2. Enable `--enable-prefix-caching` for repeated prompts (system prompts, RAG)
3. Enable `--enable-chunked-prefill` for long input sequences
4. Increase `--max-num-seqs 512` for throughput-heavy workloads
5. On Intel: use `--dp` for multi-GPU, and `numactl -C <cores>` for NUMA binding
6. Monitor: `curl localhost:9090/metrics | grep vllm`

Deep dive: [references/optimization.md](references/optimization.md)

## Key Intel Arc differences

These flags are required/recommended for Intel Arc B60 (not needed on NVIDIA):

| Flag | Why |
|------|-----|
| `--enforce-eager` | Graph mode not supported on XPU |
| `--block-size 64` | Optimal for Intel GPU memory layout |
| `--disable-sliding-window` | Not supported on XPU |
| `VLLM_WORKER_MULTIPROC_METHOD=spawn` | Required for multi-process on XPU |
| `VLLM_ALLOW_LONG_MAX_MODEL_LEN=1` | Skip context length warnings |

CCL communication (multi-GPU):
```bash
export CCL_TOPO_P2P_ACCESS=1  # P2P mode (~15% better for large batches)
export CCL_TOPO_P2P_ACCESS=0  # USM mode (fallback)
```

BPE-Qwen tokenizer (faster tokenization for Qwen models):
```bash
pip install bpe-qwen
vllm serve MODEL --tokenizer-mode bpe-qwen
```

Full Intel deployment guide: [references/intel-gpu.md](references/intel-gpu.md)

## Common issues

| Problem | Fix |
|---------|-----|
| OOM during model load | `--gpu-memory-utilization 0.7`, or add `--quantization awq/fp8` |
| OOM during quantization (Intel) | `export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1` |
| TTFT > 1s | `--enable-prefix-caching` or `--enable-chunked-prefill` |
| Low throughput | `--max-num-seqs 512`, verify GPU util >80% |
| `No module named 'vllm._C'` (Intel Docker) | Run from `/llm`, not `/llm/vllm` |
| Slow with odd GPU count | TP must be power-of-2: `-tp 4` not `-tp 3` |

Full troubleshooting: [references/troubleshooting.md](references/troubleshooting.md)

## When to use vLLM vs alternatives

| Scenario | Best choice |
|----------|-------------|
| Production API (100+ req/sec) | **vLLM** |
| OpenAI-compatible endpoint | **vLLM** |
| Intel Arc GPU deployment | **vLLM** (llm-scaler) |
| CPU/edge inference, single-user | llama.cpp |
| Research / prototyping | HuggingFace transformers |
| NVIDIA-only, maximum perf | TensorRT-LLM |
| HuggingFace ecosystem | Text-Generation-Inference |

## Reference files

Read these for detailed procedures (loaded on demand, not always in context):

- [references/intel-gpu.md](references/intel-gpu.md) — Full Intel Arc deployment: bare metal setup, Docker, quantization (INT4/FP8/MXFP4), data parallelism, multi-node (Docker Swarm + Ray), load balancer, supported models table, oneAPI environment, device permissions
- [references/optimization.md](references/optimization.md) — PagedAttention internals, continuous batching mechanics, prefix caching, speculative decoding, benchmark comparisons, tuning guide
- [references/quantization.md](references/quantization.md) — AWQ/GPTQ/FP8 setup, self-quantization workflows, accuracy vs compression trade-offs, calibration data
- [references/server-deployment.md](references/server-deployment.md) — Docker, Kubernetes manifests, Nginx load balancing, health checks, Prometheus/Grafana monitoring
- [references/troubleshooting.md](references/troubleshooting.md) — OOM errors, performance issues, model loading, networking, distributed serving, debugging commands

## Resources

- Official docs: https://docs.vllm.ai
- GitHub: https://github.com/vllm-project/vllm
- Intel llm-scaler: https://github.com/intel/llm-scaler/blob/main/vllm/README.md
- Paper: "Efficient Memory Management for Large Language Model Serving with PagedAttention" (SOSP 2023)
