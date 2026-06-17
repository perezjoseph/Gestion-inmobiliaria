---
inclusion: fileMatch
fileMatchPattern: ["infra/k8s/app/shared/vllm.yml", "infra/**/*vllm*"]
---

# vLLM Inference

## Container Image

Always use **`intel/llm-scaler-vllm`** as the base image for the vLLM deployment. This is an Intel-optimized build that includes oneAPI, SYCL runtime, and XPU kernel support required by the Intel Arc GPU on the `inference` node.

## Model

Current model: **`Qwen/Qwen3-14B`** (dense, 14.8B params). Chosen for:
- Native tool calling via `--tool-call-parser hermes` (0.971 BFCL accuracy)
- Fits in 24GB VRAM with `--quantization sym_int4` (~9GB weights, ~15GB free for KV cache → 32K context)
- Intel llm-scaler verified on Arc GPUs (sym_int4 + fp8)
- Strong multilingual support (119 languages including Spanish)

## Key Constraints

- Image tag format: `<vllm_version>-b<ipex_version>` (e.g., `0.14.0-b8.3.1`).
- The container must run `source /opt/intel/oneapi/setvars.sh --force` before launching vLLM.
- Resource requests must include `gpu.intel.com/xe: "1"` for GPU scheduling.
- Model files live on the `vllm-models-pvc` PVC mounted at `/models`; set `HF_HUB_OFFLINE=1` so vLLM never attempts downloads at runtime.
- Quantization uses `sym_int4` via the bundled `vllm_int4_for_multi_arc.so` library.
- Context window: `--max-model-len 32768` (14B dense at INT4 leaves ~15GB for KV cache on 24GB B60 → 32K context fits comfortably).

## When Upgrading

Use the `vllm-upgrade` skill for the step-by-step upgrade procedure.
