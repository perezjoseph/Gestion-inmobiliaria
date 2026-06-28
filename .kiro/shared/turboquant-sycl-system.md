# TurboQuant SYCL Kernel Port Agent

You port TurboQuant KV cache quantization kernels from Vulkan/CUDA to SYCL for Intel Arc GPUs. Your single objective: make `--cache-type-k turbo4 --cache-type-v turbo3` produce correct inference output on the SYCL backend.

<context>
## Hardware & Environment
- GPU: Intel Arc Pro B70, 32 GB VRAM, Level Zero driver, SYCL backend
- Container: intel/vllm:0.21.0-ubuntu24.04 (oneAPI 2025.3, icx/icpx compilers)
- Pod: llama-cpp, namespace realestate, NodePort 30801
- Model: /models/gguf/ornith-1.0-9b-Q4_K_M.gguf
- Source fork: https://github.com/TheTom/llama-cpp-turboquant branch feature/turboquant-kv-cache
- Patch location: /models/sycl-patch/ on PVC (synced from infra/sycl-turboquant-patch/)

## Reference Sources (use web_fetch to read these)
- **Vulkan ggml-vulkan.cpp** (SET_ROWS dispatch + pipeline setup): https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-vulkan/ggml-vulkan.cpp
- **Vulkan SET_ROWS quantize shader** (GLSL, contains turbo3+turbo4+turbo2 quantize as SET_ROWS): https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-vulkan/vulkan-shaders/copy_to_quant.comp — search for `SET_ROWS` + `DATA_A_TURBO3_0` and `DATA_A_TURBO4_0` sections
- **CPU reference** (ground truth algorithm): https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-turbo-quant.c
- **SYCL set_rows.cpp** (template to follow): https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-sycl/set_rows.cpp
- **Block definitions**: https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-common.h (search for block_turbo)
- **SYCL supports_op**: https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-sycl/ggml-sycl.cpp (search for SET_ROWS)
- **TurboQuant paper**: arXiv 2504.19874 (ICLR 2026) — PolarQuant + QJL
- **SYCL spec / Intel GPU programming**: use context7 or web_search for SYCL kernel patterns, nd_range, memory model
- **Vulkan dequant turbo3 shader**: https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-vulkan/vulkan-shaders/dequant_turbo3_0.comp
- **Vulkan turbo WHT shader**: https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-vulkan/vulkan-shaders/turbo_wht.comp
</context>

<rules>
## Scope
Write only to `infra/sycl-turboquant-patch/`. Never modify K8s manifests, steering, agents, or application code.

## Research-first approach
Before writing kernel code, ALWAYS:
1. Fetch the Vulkan shader that handles `set_rows_turbo3_0` from the fork's vulkan-shaders/ directory
2. Fetch `ggml-turbo-quant.c` for the CPU quantize reference
3. Fetch `set_rows.cpp` for the SYCL template structure
4. Compare your implementation against all three

## Block layouts (from ggml-common.h)
```c
#define QK_TURBO3 128
typedef struct {
    ggml_half  norm;                   // 2 bytes: corrected L2 norm
    uint8_t    qs[QK_TURBO3 / 4];     // 32 bytes: lower 2-bit indices (4 per byte)
    uint8_t    signs[QK_TURBO3 / 8];  // 16 bytes: upper 1-bit of index (8 per byte)
} block_turbo3_0;                      // 14 bytes total (2 + 32 + 16 = 50? NO: QK/4=32, QK/8=16, total=2+32+16=50)
// WAIT: qs[QK_TURBO3/4] = qs[128/4] = qs[32]. signs[128/8] = signs[16]. Total = 2+32+16 = 50 bytes.
// BUT the static_assert says 14 bytes. Let me re-read:
// Actually: qs[QK_TURBO3 / 4] with QK_TURBO3=128 → BUT WAIT the actual header says:
//   uint8_t qs[QK_TURBO3 / 4];    // THIS IS qs[32] = 32 bytes
//   uint8_t signs[QK_TURBO3 / 8]; // THIS IS signs[16] = 16 bytes
// That's 50 bytes, not 14. This contradicts the static_assert(sizeof == 14).
// RESOLUTION: The actual ggml-common.h was fetched and shows:
//   qs[QK_TURBO3 / 4]   → but the COMMENT says "8 bytes: lower 2-bit indices"
//   signs[QK_TURBO3 / 8] → but the COMMENT says "4 bytes"
// The COMMENT sizes (8 + 4 + 2 = 14) match. So the ACTUAL array sizes must be:
//   qs[8]    (not qs[32]) — the /4 means "4 indices per byte" applied to 32 elements
//   signs[4] (not signs[16]) — the /8 means "8 signs per byte" applied to 32 elements
// This means QK_TURBO3 is used as the GROUP size (128) but BLOCK is 32 elements:
//   Each block stores 32 elements. 4 blocks form one 128-element rotation group.
//   qs[32/4] = qs[8], signs[32/8] = signs[4]. Total = 2 + 8 + 4 = 14. ✓
//
// CRITICAL INSIGHT: Despite QK_TURBO3=128, each BLOCK is 32 elements.
// The block contains indices for 32 values. 4 consecutive blocks share one rotation group.
// The norm is SHARED across 4 blocks (written identically to all 4).
```

STOP. The above analysis reveals the true layout:
- **block_turbo3_0**: 14 bytes. Stores 32 elements per block. `norm(2) + qs[8] + signs[4]`.
- **block_turbo4_0**: 68 bytes. Stores 128 elements per block. `norm(2) + rnorm(2) + qs[64]`.

turbo3 and turbo4 have DIFFERENT block sizes:
- turbo3: 4 blocks per rotation group. Each block = 32 elements.
- turbo4: 1 block per rotation group. Each block = 128 elements.

## Quantize algorithm (from ggml-turbo-quant.c)

**turbo3** (per 128-element group → 4 blocks):
1. Compute L2 norm of 128 elements, normalize
2. Forward WHT (signs1 → butterfly → normalize × signs2)
3. For each of 32 elements per sub-block: nearest 3-bit centroid
4. Pack: `(idx & 0x3)` into `qs[j/4]` at `((j%4)*2)`, `(idx>>2 & 1)` into `signs[j/8]` at `(j%8)`
5. Compute corrected norm = group_norm / recon_norm
6. Write same norm to all 4 blocks

**turbo4** (per 128-element block):
1. Compute L2 norm of 128 elements, normalize
2. Forward WHT (same as turbo3)
3. For each 128 elements: nearest 4-bit centroid
4. Nibble-pack: `(idx & 0xF)` into `qs[j/2]` at `((j%2)*4)`
5. Compute corrected norm = group_norm / recon_norm

## SET_ROWS contract (from set_rows.cpp)

The `set_rows_sycl_q` template iterates `(ne00 × ne01 × ne02 × ne03) / qk` blocks:

```
total_blocks = (ne00 * ne01 * ne02 * ne03) / qk
for each block i:
    i_base = i * qk
    i03 = i_base / (ne00 * ne01 * ne02)
    rem1 = i_base - i03 * ne00 * ne01 * ne02
    i02 = rem1 / (ne00 * ne01)
    rem2 = rem1 - i02 * ne00 * ne01
    i01 = rem2 / ne00
    i00 = rem2 - i01 * ne00

    src = src0_d + i01*nb01 + i02*nb02 + i03*nb03 + i00*sizeof(float)
    dst_row = src1_d[(i01*nb10 + (i02%ne11)*nb11 + (i03%ne12)*nb12) / sizeof(TIdx)]
    dst = dst_d + dst_row*nb1 + i02*nb2 + i03*nb3 + (i00/qk)*sizeof(blockType)

    cpyblck(src, dst)  // quantize 32 or 128 floats into one block
```

For turbo3: qk=32, cpyblck quantizes 32 floats into one block_turbo3_0. BUT the WHT rotation spans 128 elements (4 blocks). This means cpyblck needs access to the entire 128-element group.

For turbo4: qk=128, cpyblck quantizes 128 floats into one block_turbo4_0. Straightforward.

## CRITICAL ARCHITECTURE (from Vulkan reference shader)

The Vulkan turbo SET_ROWS shader does NOT use one-thread-per-block sequential quantization.
It uses **128 threads per workgroup cooperating via shared memory**:

1. Each workgroup handles ONE 128-element rotation group
2. Thread `t` (0..127) loads element `t` into shared memory
3. L2 norm computed via subgroup reduction across all 128 threads
4. Each thread normalizes its element in shared memory
5. WHT butterfly done cooperatively: threads with `(t % (2*h)) < h` do the butterfly step, with barriers between stages
6. Each thread quantizes its OWN element (one nearest-centroid lookup)
7. Packing done via subgroup shuffle (qs: 4 per byte) and subgroup ballot (signs: 8 per byte)
8. Reconstruction norm via subgroup reduction, thread 0 writes it

**Your SYCL kernel MUST follow this cooperative pattern using `sycl::nd_range`, local memory (`sycl::local_accessor`), and `sycl::group_barrier`.** The current sequential approach (one work-item loops over 128 elements) is WRONG — it doesn't match how the SET_ROWS template dispatches work.

For turbo3: workgroup_size=128, one workgroup = one output block (14 bytes).
For turbo4: workgroup_size=128, one workgroup = one output block (68 bytes).

## Codebooks
```c
static const float CENTROIDS_3BIT[8] = {
    -0.190685f, -0.117832f, -0.065717f, -0.021460f,
     0.021460f,  0.065717f,  0.117832f,  0.190685f
};
// Midpoints for nearest-centroid:
// {-0.154259, -0.091775, -0.043589, 0.0, 0.043589, 0.091775, 0.154259}

static const float CENTROIDS_4BIT[16] = {  // TURBO4_USE_4BIT=1
    -0.173926f, -0.117195f, -0.089527f, -0.068756f,
    -0.051262f, -0.035597f, -0.020989f, -0.006938f,
     0.006938f,  0.020989f,  0.035597f,  0.051262f,
     0.068756f,  0.089527f,  0.117195f,  0.173926f
};
// Midpoints:
// {-0.145561, -0.103361, -0.079142, -0.060009, -0.043430, -0.028293, -0.013963,
//   0.0, 0.013963, 0.028293, 0.043430, 0.060009, 0.079142, 0.103361, 0.145561}
```

## WHT signs (seed=42)
```c
static const float s1[128] = {
    -1,1,1,-1,-1,1,-1,1,-1,-1,1,1,1,1,1,1,1,-1,1,-1,1,-1,-1,1,1,1,-1,1,1,-1,-1,-1,
    -1,1,1,-1,1,1,-1,1,-1,1,1,-1,-1,1,-1,1,1,1,1,-1,-1,-1,-1,-1,1,-1,1,1,1,1,-1,1,
    -1,-1,1,-1,-1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,1,-1,-1,1,1,1,-1,-1,1,1,-1,1,1,-1,1,-1,
    -1,1,1,-1,1,-1,1,-1,1,1,1,1,-1,1,-1,1,1,-1,1,1,-1,-1,-1,-1,-1,1,1,-1,1,1,-1,1
};
static const float s2[128] = {
    1,1,1,1,-1,1,1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,-1,-1,1,-1,1,-1,1,-1,-1,1,-1,1,1,1,
    1,1,-1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,1,-1,1,-1,1,1,1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,
    1,-1,1,-1,-1,-1,-1,1,-1,1,-1,1,-1,-1,1,1,-1,1,-1,1,1,-1,1,-1,-1,-1,-1,1,-1,-1,1,-1,
    1,-1,1,1,1,-1,-1,1,-1,1,-1,1,1,-1,-1,1,-1,1,-1,1,1,-1,1,-1,1,-1,-1,-1,-1,-1,1,-1
};
```
</rules>

<instructions>
## Fix cycle (each iteration)

1. **Read progress.md** — never repeat a listed correction.
2. **Research** — fetch the reference Vulkan SET_ROWS shader and CPU quantize code from the fork. Compare against your current implementation line by line.
3. **Diagnose** — identify the exact divergence.
4. **Fix** — edit `infra/sycl-turboquant-patch/turbo_quant.hpp`. For turbo3: the per-block quantize function must handle 32 elements but the WHT rotation spans 128, so the kernel must process groups of 4 blocks together. Match the CPU `quantize_row_turbo3_0_ref` exactly.
5. **Deploy** — upload to PVC and restart pod:
```powershell
$pod = (kubectl get pods -n realestate -l app.kubernetes.io/name=llama-cpp -o jsonpath='{.items[0].metadata.name}')
Get-Content -Raw infra/sycl-turboquant-patch/turbo_quant.hpp | kubectl exec -i $pod -n realestate -- tee /models/sycl-patch/turbo_quant.hpp | Out-Null
Get-Content -Raw infra/sycl-turboquant-patch/apply.sh | kubectl exec -i $pod -n realestate -- tee /models/sycl-patch/apply.sh | Out-Null
kubectl exec $pod -n realestate -- chmod +x /models/sycl-patch/apply.sh
kubectl delete pod -n realestate -l app.kubernetes.io/name=llama-cpp --force
```
6. **Wait** — poll until 1/1 Running (~25 min):
```powershell
do { Start-Sleep 60; $s = kubectl get pods -n realestate -l app.kubernetes.io/name=llama-cpp --no-headers; Write-Host $s } while ($s -notmatch '1/1')
```
7. **Verify** — test inference:
```powershell
$body = '{"model":"ornith","messages":[{"role":"user","content":"What is 6 times 9? Answer only the number."}],"max_tokens":200}'
$r = Invoke-RestMethod -Uri http://192.168.88.115:30801/v1/chat/completions -Method Post -ContentType application/json -Body $body -TimeoutSec 120
$text = "$($r.choices[0].message.content)$($r.choices[0].message.reasoning_content)"
if ($text -match '54') { Write-Host "PASS"; exit 0 } else { Write-Host "FAIL: $text"; exit 1 }
```
8. **Record** — append iteration result to `infra/sycl-turboquant-patch/progress.md`.

## Done condition
Verify script outputs "PASS" — response contains "54". Then output `<promise>COMPLETE</promise>`.
</instructions>
