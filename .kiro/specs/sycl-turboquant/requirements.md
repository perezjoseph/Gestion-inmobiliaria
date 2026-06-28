# SYCL TurboQuant KV Cache Compression

## Goal
Add working `--cache-type-k turbo4 --cache-type-v turbo3` support to the TheTom/llama-cpp-turboquant fork compiled with the SYCL backend on Intel Arc Pro B70. Currently the Vulkan backend works (confirmed 54=6×9 correct) but is 5 tok/s. SYCL is 60 tok/s but turbo types crash with "Failed to parse input at pos 22" garbage output.

## Success Criteria
- `llama-server` starts on SYCL backend with `--cache-type-k turbo4 --cache-type-v turbo3`
- A test inference request returns a coherent, correct response (not garbled text)
- Token generation speed is ≥ 30 tok/s (should be ~60 tok/s)

## Hardware & Environment
- GPU: Intel Arc Pro B70, 32GB VRAM, SYCL/Level Zero driver
- Container: `intel/vllm:0.21.0-ubuntu24.04` (has oneAPI/icpx compilers)
- Kubernetes pod: `llama-cpp` in `realestate` namespace
- Model: `/models/gguf/ornith-1.0-9b-Q4_K_M.gguf`
- Inference endpoint: `http://192.168.88.115:30801`
- Source: `TheTom/llama-cpp-turboquant` branch `feature/turboquant-kv-cache`
- Patch files on PVC at `/models/sycl-patch/` (turbo_quant.hpp, turbo_dequant.hpp, turbo_fattn.hpp, apply.sh)

## Key Files
- `infra/k8s/app/shared/llama-cpp.yml` — K8s deployment (change cache-type-k/v here)
- `infra/sycl-turboquant-patch/turbo_quant.hpp` — SYCL SET_ROWS kernels (main fix needed here)
- `infra/sycl-turboquant-patch/apply.sh` — Patch script run during build

## Known Facts
- `block_turbo3_0`: 14 bytes, QK=128, `norm(fp16) + qs[32] + signs[16]`
- `block_turbo4_0`: 68 bytes, QK=128, `norm(fp16) + rnorm(fp16) + qs[64]` (nibble-packed)
- Codebooks: turbo3 `CENTROIDS_3BIT[8]={-0.190685..0.190685}`, turbo4 `CENTROIDS_4BIT[16]={-0.173926..0.173926}`
- WHT: signs1→butterfly→normalize×signs2 (seed=42), same for both types
- Vulkan works → the block layout and quantize algorithm are correct in principle
- SYCL garbled output → likely a memory indexing error in the SYCL kernel

## Feedback Loop
After each code change:
1. Upload patch files to PVC: `Get-Content -Raw <file> | kubectl exec -i <pod> -n realestate -- tee /models/sycl-patch/<file>`
2. Trigger rebuild: `kubectl delete pod -n realestate -l app.kubernetes.io/name=llama-cpp --force`
3. Wait ~25 min for SYCL build to complete
4. Test: `Invoke-RestMethod -Uri http://192.168.88.115:30801/v1/chat/completions ...`
5. Check if response is coherent (not garbled)

## Previous Attempts
1. First attempt: Wrong codebooks (Eliza values ±2.7 instead of ±0.17) → garbled
2. Second attempt: Correct codebooks but wrong block size (4×32 sub-blocks instead of 1×128) → still garbled
3. Current state: turbo_quant.hpp has correct codebooks and QK=128 but still garbled — likely kernel indexing bug
