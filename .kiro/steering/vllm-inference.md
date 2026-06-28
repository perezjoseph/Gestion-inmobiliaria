---
inclusion: fileMatch
fileMatchPattern: ["infra/k8s/app/shared/vllm.yml", "infra/**/*vllm*"]
---

# vLLM Inference

## Container Image

Always use **`intel/vllm`** as the base image for the vLLM deployment. This is Intel's official vLLM build with XPU kernel support for Intel Arc GPUs on the `inference` node.

## Model

Current model: **`Qwen/Qwen3-14B`** (dense, 14.8B params). Chosen for:
- Native tool calling via `--tool-call-parser hermes` (0.971 BFCL accuracy)
- Fits in 32GB VRAM with `--quantization fp8` (~7.5GB weights, ~22GB free for KV cache → 32K context)
- Validated by Intel on Arc B-series GPUs
- Strong multilingual support (119 languages including Spanish)

## Key Constraints

- Image tag format: `<vllm_version>-ubuntu24.04` (e.g., `0.21.0-ubuntu24.04`).
- The image entrypoint sources oneAPI but uses a `serve` wrapper CLI. Override with `command: ["bash", "-c"]` and call `vllm serve` directly.
- Resource requests must include `gpu.intel.com/xe: "1"` for GPU scheduling.
- Model files live on the `vllm-models-pvc` PVC mounted at `/models`; set `HF_HUB_OFFLINE=1` so vLLM never attempts downloads at runtime.
- Quantization: online FP8 (`--quantization fp8`). No calibration data or custom libraries needed.
- Context window: `--max-model-len 32768` (14B dense at FP8 leaves ~22GB for KV cache on 32GB B70).
- `VLLM_API_KEY`: pass via `${VLLM_API_KEY:+--api-key $VLLM_API_KEY}` in the shell args so it's only set when the secret exists.

## When Upgrading

Use the `vllm-upgrade` skill for the step-by-step upgrade procedure.
