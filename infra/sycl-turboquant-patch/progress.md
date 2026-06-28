# TurboQuant SYCL Repair Progress

## Corrections (read before every iteration — never repeat these)

- **WRONG**: Using Eliza codebooks `TURBO4_CENTROIDS[16] = {-2.7321..}` — these are for a different implementation.
  **RIGHT**: Use `{-0.173926f, -0.117195f, ... 0.173926f}` from fork's `ggml-turbo-quant.c`.

- **WRONG**: Treating QK_TURBO3=32 (4 sub-blocks of 32 per group of 128).
  **RIGHT**: `QK_TURBO3=128` — one block covers the full 128-element rotation group.

- **WRONG**: Using custom group-based loop structure (`blocks_per_row × ne01 × ne02 × ne03`).
  **RIGHT**: Follow the exact `set_rows_sycl_q` template: flat index `i` over `(ne00 × ne01 × ne02 × ne03) / QK` blocks, with `calculate_offset<3>` for src and dst.

## Iteration 1 — Eliza codebooks
- Fix attempted: Used ELIZA_TURBO_CENTROIDS values (±2.7x range)
- Result: FAIL — garbled output, "Failed to parse input at pos 22"

## Iteration 2 — Correct codebooks, wrong QK
- Fix: Correct codebook values, but block struct had QK=32 sub-blocks
- Result: FAIL — same garbled output

## Iteration 3 — Correct codebooks + QK=128, custom loop
- Fix: QK=128, correct codebooks, BUT used non-standard loop structure
- Result: FAIL — still garbled
- Suspected cause: dst offset calculation diverges from set_rows_sycl_q contract

## Current state
- turbo_quant.hpp has correct codebooks + QK=128
- SET_ROWS dispatch added to set_rows.cpp via apply.sh
- supports_op patched in ggml-sycl.cpp
- Bug: kernel uses SEQUENTIAL per-work-item quantization (one thread loops 128 elements)
- The Vulkan reference uses COOPERATIVE parallelism: 128 threads per workgroup, shared memory, barriers

## Vulkan index computation (MUST replicate in SYCL)
```glsl
// From copy_to_quant.comp SET_ROWS + DATA_A_TURBO3_0:
const uint t   = gl_LocalInvocationID.x;   // thread 0..127
const uint g   = gl_WorkGroupID.z * 262144 + gl_WorkGroupID.y * 512 + gl_WorkGroupID.x;
const uint gpr = p.ne00 / 128;             // groups (blocks) per row

uint tmp = g;
const uint ig  = tmp % gpr; tmp /= gpr;    // which block in the row
const uint i01 = tmp % p.ne01; tmp /= p.ne01;
const uint i02 = tmp % p.ne12;
const uint i03 = tmp / p.ne12;

const uint sb  = src0_idx(ig * 128, i01, i02, i03);  // source offset
const uint i1  = data_i[src1_idx(i01, i02 % ne11, i03 % ne12, 0)];  // dst row from index tensor
const uint db  = dst_idx(ig, i1, i02, i03);  // destination block index
```

## Key differences from current SYCL kernel
1. Current: `sycl::nd_range<1>(grid*wg, wg)` with wg=64, one thread does ALL 128 elements
2. Correct: `sycl::nd_range<1>(total_blocks*128, 128)` — one workgroup of 128 threads per block
3. Current: sequential loop `for(i=0;i<128;i++) buf[i]=src[i]; quantize_block(buf, dst)`
4. Correct: each thread loads ONE element into local memory, cooperates on WHT via barriers
5. Current: missing `sycl::local_accessor` for shared WHT buffer
6. Correct: needs local memory for wht[128] and sg_acc[N] reduction arrays

## Iteration 4 — fp16 + constexpr + sqrt fixes → PASS
- Root cause: SYCL device code issues:
  1. `__builtin_memcpy` for fp16 conversion unreliable on device → use `sycl::bit_cast<uint16_t>` and direct `sycl::half()` assignment
  2. `sycl::sqrt()` namespace ambiguity on device → use `sqrtf()`
  3. `static constexpr` arrays at namespace scope not reliably captured by SYCL kernel lambda → inline `constexpr` arrays inside functions
  4. Centroid arrays (`TURBO3_CENTROIDS`, `TURBO4_CENTROIDS`) also moved inline
- Result: **PASS** — response "54" with coherent reasoning
- Verified: 2026-06-28T03:45 UTC
