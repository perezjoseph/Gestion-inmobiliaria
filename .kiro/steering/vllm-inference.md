---
inclusion: fileMatch
fileMatchPattern: ["infra/k8s/app/shared/vllm.yml", "infra/**/*vllm*"]
---

# vLLM Inference

## Container Image

Always use **`intel/llm-scaler-vllm`** as the base image for the vLLM deployment. This is an Intel-optimized build that includes oneAPI, SYCL runtime, and XPU kernel support required by the Intel Arc GPU on the `inference` node.

## Model

Current model: **`Qwen/Qwen3-Coder-30B-A3B-Instruct`** (MoE, 30B total / 3B active per token). Chosen for:
- Native agentic tool calling via `--tool-call-parser qwen3_coder`
- Fits in 32GB VRAM with `--quantization sym_int4` (~17GB weights + KV cache for 8K context)
- Intel llm-scaler verified on Arc GPUs

## Key Constraints

- Image tag format: `<vllm_version>-b<ipex_version>` (e.g., `0.14.0-b8.3.1`).
- The container must run `source /opt/intel/oneapi/setvars.sh --force` before launching vLLM.
- Resource requests must include `gpu.intel.com/xe: "1"` for GPU scheduling.
- Model files live on the `vllm-models-pvc` PVC mounted at `/models`; set `HF_HUB_OFFLINE=1` so vLLM never attempts downloads at runtime.
- Quantization uses `sym_int4` via the bundled `vllm_int4_for_multi_arc.so` library.
- Context window: `--max-model-len 32768` (B70 Pro 32GB fits 32K easily with MoE's small KV footprint — blog confirms >50K viable on B-series).

## When Upgrading

Use the `vllm-upgrade` skill for the step-by-step upgrade procedure.
