# SYCL TurboQuant Kernel Implementation Plan

Add SYCL `SET_ROWS` (f32→turbo quantization), dequantization, and flash-attention dot-product kernels for `GGML_TYPE_TURBO3_0` and `GGML_TYPE_TURBO4_0` in the TheTom/llama-cpp-turboquant fork.

## Affected Files

| Path (relative to repo root) | Action | What Changes |
|---|---|---|
| `ggml/src/ggml-sycl/turbo_quant.hpp` | **create** | SYCL device functions: FWHT, sign conditioning, nearest-centroid, `cpy_blck_f32_turbo3_0`, `cpy_blck_f32_turbo4_0` |
| `ggml/src/ggml-sycl/turbo_dequant.hpp` | **create** | SYCL dequantize functions: `dequantize_turbo3_0`, `dequantize_turbo4_0` (for get_rows + fattn V path) |
| `ggml/src/ggml-sycl/turbo_fattn.hpp` | **create** | Flash-attention K dot-product: `vec_dot_fattn_vec_KQ_turbo3_0`, `vec_dot_fattn_vec_KQ_turbo4_0`, V dequant: `dequantize_V_turbo3_0`, `dequantize_V_turbo4_0` |
| `ggml/src/ggml-sycl/set_rows.cpp` | **modify** | Add `case GGML_TYPE_TURBO3_0` and `case GGML_TYPE_TURBO4_0` dispatching to group-aware quantize kernels |
| `ggml/src/ggml-sycl/cpy.hpp` | **modify** | Include `turbo_quant.hpp`, expose `cpy_blck_f32_turbo3_0` / `cpy_blck_f32_turbo4_0` |
| `ggml/src/ggml-sycl/fattn-common.hpp` | **modify** | Add turbo3/turbo4 cases to `get_vec_dot_KQ()` and `get_dequantize_V()` template dispatchers |
| `ggml/src/ggml-sycl/fattn.cpp` | **modify** | Add `FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO4_0, ...)` and turbo3 entries under `GGML_SYCL_FA_ALL_QUANTS` |
| `ggml/src/ggml-sycl/fattn-vec.hpp` | **modify** | Add `EXTERN_DECL_FATTN_VEC_CASES` for turbo3/turbo4 K types + explicit instantiations |
| `ggml/src/ggml-sycl/ggml-sycl.cpp` | **modify** | Add `GGML_TYPE_TURBO3_0` / `GGML_TYPE_TURBO4_0` to `do_ggml_backend_sycl_device_supports_op` SET_ROWS case + FLASH_ATTN_EXT K/V type switch |
| `ggml/src/ggml-sycl/CMakeLists.txt` | **modify** | No new .cpp files needed (headers only), but verify turbo_quant.hpp is found via include path |

## Steps

### 1. Create `ggml/src/ggml-sycl/turbo_quant.hpp` — Quantization Primitives

This file provides all device-side building blocks for SET_ROWS quantization.

**Constants (compile-time, `constexpr`):**
- `TURBO3_CENTROIDS_3BIT[8]` — Lloyd-Max centroids
- `TURBO3_MIDPOINTS_3BIT[7]` — decision boundaries
- `TURBO4_CENTROIDS_4BIT[16]` — Lloyd-Max centroids
- `TURBO4_MIDPOINTS_4BIT[15]` — decision boundaries (midpoints between consecutive centroids)
- `TURBO_WHT_SIGNS1[128]` — pre-conditioning ±1 signs (seed=42)
- `TURBO_WHT_SIGNS2[128]` — post-conditioning ±1 signs (seed=42)
- `TBQ_SIGNS_32[32]` — per-block sign conditioning for turbo4

**Device functions:**
```cpp
// In-place FWHT on 128 floats (iterative butterfly, 7 stages)
static inline void fwht_128(float x[128]);

// turbo3 full-group rotation: signs1 → fwht128 → signs2
static inline void turbo_rotate_forward_128(float x[128]);

// turbo4 per-block: signs32 → hadamard32
static inline void tbq_hadamard32(float x[32]);
static inline void tbq_precondition_block32(const float *src, float dst[32]);

// Nearest centroid lookup
static inline uint8_t nearest_3bit(float v);  // 7 comparisons against midpoints
static inline uint8_t nearest_4bit(float v);  // 15 comparisons against midpoints
```

**Group-level quantize (called by SET_ROWS kernel):**

The critical difference from standard `cpy_blck_*` functions: turbo types require a **group of 4 consecutive blocks** (128 elements) to be processed together. The SET_ROWS template calls `cpyblck` per-block (32 elements). Two approaches:

**Approach chosen: Custom SET_ROWS template specialization for turbo types.**

Instead of using the per-block `cpy_kernel_t` signature, introduce:
```cpp
template <typename TIdx>
static void set_rows_sycl_turbo3(/* same params as set_rows_sycl_q */) { ... }

template <typename TIdx>
static void set_rows_sycl_turbo4(/* same params as set_rows_sycl_q */) { ... }
```

Each kernel thread processes one 128-element group (4 blocks). The work-group size = `ne00 / 128` blocks per row.

**Rationale:** The turbo3 algorithm requires the full 128-element vector to compute L2 norm and apply FWHT rotation before per-block quantization. A per-block `cpyblck` function cannot access neighboring blocks.

### 2. Create `ggml/src/ggml-sycl/turbo_dequant.hpp` — Dequantization

For get_rows and V-path flash attention:

```cpp
// turbo3: unpack 3-bit index, multiply centroid by norm
// Note: returns values in ROTATED space (no inverse rotation needed for
// flash-attn because Q is pre-rotated by the model)
static __dpct_inline__ void dequantize_turbo3_0(
    const void *vx, const int64_t ib, const int iqs, dfloat2 &v);

// turbo4: unpack 4-bit index from packed layout, multiply centroid by norm
static __dpct_inline__ void dequantize_turbo4_0(
    const void *vx, const int64_t ib, const int iqs, dfloat2 &v);
```

These follow the exact signature pattern of `dequantize_q4_0` in `dequantize.hpp`.

**turbo3 unpack logic per element j in block b:**
```
low2 = (qs[j/4] >> ((j%4)*2)) & 0x3
hi1  = (signs[j/8] >> (j%8)) & 0x1
idx  = low2 | (hi1 << 2)
value = CENTROIDS_3BIT[idx] * fp16_to_f32(norm)
```

**turbo4 unpack logic per element j in block b:**
```
byte = j & 15
packed = qs[byte]
idx = (j < 16) ? (packed & 0x0F) : (packed >> 4)
value = CENTROIDS_4BIT[idx] * fp16_to_f32(norm)
```

### 3. Create `ggml/src/ggml-sycl/turbo_fattn.hpp` — Flash Attention Kernels

**K dot-product (`vec_dot_KQ` pattern):**

For turbo K types, the Q vector is expected to already be in rotated space (pre-rotated by the model's attention implementation). The kernel computes `sum(Q[i] * dequant_K[i])` across the head dimension D.

```cpp
template <int D, int nthreads, int warp_size>
static __dpct_inline__ float vec_dot_fattn_vec_KQ_turbo3_0(
    const char * __restrict__ K_c,
    const void * __restrict__ Q_v,
    const int  * __restrict__ Q_q8,
    const void * __restrict__ Q_ds_v);

template <int D, int nthreads, int warp_size>
static __dpct_inline__ float vec_dot_fattn_vec_KQ_turbo4_0(/* same */);
```

**Implementation strategy:** These dequantize K on-the-fly and dot with Q (stored as q8_1 by upstream fattn). Each thread handles `D / nthreads` elements. The dequant is cheap (table lookup + fp16 multiply).

**V dequantize (`dequantize_V` pattern):**

```cpp
template <typename T, int ne>
static __dpct_inline__ void dequantize_V_turbo3_0(
    const void * __restrict__ vx, void * __restrict__ dst, const int64_t i0);

template <typename T, int ne>
static __dpct_inline__ void dequantize_V_turbo4_0(
    const void * __restrict__ vx, void * __restrict__ dst, const int64_t i0);
```

These load `ne` consecutive dequantized values starting at element `i0` of the V tensor.

### 4. Modify `ggml/src/ggml-sycl/set_rows.cpp` — Dispatch Turbo Cases

In the `set_rows_impl` function's switch statement, add after the existing `GGML_TYPE_Q8_0` case:

```cpp
case GGML_TYPE_TURBO3_0:
    set_rows_sycl_turbo3<TIdx>(src0_d, src1_d, (block_turbo3_0 *)dst->data,
        ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13,
        nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13,
        nb1, nb2, nb3, stream);
    break;
case GGML_TYPE_TURBO4_0:
    set_rows_sycl_turbo4<TIdx>(src0_d, src1_d, (block_turbo4_0 *)dst->data,
        ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13,
        nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13,
        nb1, nb2, nb3, stream);
    break;
```

Add `#include "turbo_quant.hpp"` at the top.

### 5. Modify `ggml/src/ggml-sycl/cpy.hpp` — Include Turbo Header

Add `#include "turbo_quant.hpp"` so that any other copy paths that need turbo quantization can access it.

### 6. Modify `ggml/src/ggml-sycl/fattn-common.hpp` — Register Turbo Dispatch

In `get_vec_dot_KQ()`:
```cpp
} else if constexpr (type_K == GGML_TYPE_TURBO3_0) {
    return vec_dot_fattn_vec_KQ_turbo3_0<D, nthreads, warp_size>;
} else if constexpr (type_K == GGML_TYPE_TURBO4_0) {
    return vec_dot_fattn_vec_KQ_turbo4_0<D, nthreads, warp_size>;
}
```

In `get_dequantize_V()`:
```cpp
} else if constexpr (type_V == GGML_TYPE_TURBO3_0) {
    return dequantize_V_turbo3_0<T, ne>;
} else if constexpr (type_V == GGML_TYPE_TURBO4_0) {
    return dequantize_V_turbo4_0<T, ne>;
}
```

Add `#include "turbo_fattn.hpp"` and `#include "turbo_dequant.hpp"`.

### 7. Modify `ggml/src/ggml-sycl/fattn.cpp` — Add VEC Cases

Under `#ifdef GGML_SYCL_FA_ALL_QUANTS`:
```cpp
FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO3_0, GGML_TYPE_F16)
FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO4_0, GGML_TYPE_F16)
FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO3_0, GGML_TYPE_TURBO3_0)
FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO4_0, GGML_TYPE_TURBO4_0)
FATTN_VEC_CASES_ALL_D(GGML_TYPE_TURBO4_0, GGML_TYPE_TURBO3_0)
```

In `ggml_sycl_get_best_fattn_kernel`, add turbo types to the `switch (K->type)` accepted types.

### 8. Modify `ggml/src/ggml-sycl/fattn-vec.hpp` — Extern Declarations

Add:
```cpp
EXTERN_DECL_FATTN_VEC_CASES( 64, GGML_TYPE_TURBO3_0)
EXTERN_DECL_FATTN_VEC_CASES( 64, GGML_TYPE_TURBO4_0)
EXTERN_DECL_FATTN_VEC_CASES(128, GGML_TYPE_TURBO3_0)
EXTERN_DECL_FATTN_VEC_CASES(128, GGML_TYPE_TURBO4_0)
EXTERN_DECL_FATTN_VEC_CASES(256, GGML_TYPE_TURBO3_0)
EXTERN_DECL_FATTN_VEC_CASES(256, GGML_TYPE_TURBO4_0)
```

Also update the `EXTERN_DECL_FATTN_VEC_CASES` macro to include turbo V types in its expansion, or add separate explicit instantiations.

### 9. Modify `ggml/src/ggml-sycl/ggml-sycl.cpp` — supports_op Gate

In `do_ggml_backend_sycl_device_supports_op`, case `GGML_OP_SET_ROWS`:

Add to the `auto res = (...)` expression:
```cpp
op->type == GGML_TYPE_TURBO3_0 || op->type == GGML_TYPE_TURBO4_0 ||
```

In the `GGML_OP_FLASH_ATTN_EXT` case (if there's a K/V type whitelist), add turbo types there too.

### 10. Verify Constants Match the Fork's `ggml-common.h`

Before implementation, the coder must read the fork's `ggml/src/ggml-common.h` to confirm:
- `GGML_TYPE_TURBO3_0` enum value
- `GGML_TYPE_TURBO4_0` enum value
- `block_turbo3_0` struct layout (14 bytes: uint16_t norm, uint8_t qs[8], uint8_t signs[4])
- `block_turbo4_0` struct layout (18 bytes: uint16_t norm, uint8_t qs[16])
- `QK_TURBO3` and `QK_TURBO4` constants (both 32)

Also verify the sign vectors `TURBO_WHT_SIGNS1[128]` and `TURBO_WHT_SIGNS2[128]` are defined in ggml-common.h or need to be added.

## Key Implementation Details

### FWHT-128 in SYCL
The Fast Walsh-Hadamard Transform is an iterative butterfly:
```
for (len = 1; len < 128; len <<= 1)
  for (i = 0; i < 128; i += 2*len)
    for (j = 0; j < len; ++j)
      a = x[i+j], b = x[i+j+len]
      x[i+j] = a + b, x[i+j+len] = a - b
scale by 1/sqrt(128)
```
This runs entirely in registers/local memory within a single work-item processing one 128-element group.

### Group-Aware SET_ROWS Kernel Design

The standard `set_rows_sycl_q` launches one work-item per block (32 elements). For turbo types, launch one work-item per **group** (128 elements = 4 blocks). The kernel:

1. Reads 128 consecutive f32 values from src
2. For turbo3: computes L2 norm, normalizes, applies FWHT rotation, quantizes all 4 blocks, computes corrected norm
3. For turbo4: processes each of 4 sub-blocks independently (hadamard32 + RMS normalize + quantize)
4. Writes 4 consecutive blocks to dst

The nd_range is adjusted: `num_groups = ne00 / 128` instead of `ne00 / 32`.

### Flash Attention: Pre-Rotated Q Assumption

The TurboQuant system assumes Q is pre-rotated before the attention dot product. The stored K values are in rotated space. The dot product `Q_rotated · K_rotated` equals the original `Q · K` (orthogonal rotation preserves inner products). The dequantized K values are NOT inverse-rotated — they stay in rotated space, and the model's attention path pre-rotates Q. This is handled outside the SYCL kernel (in the model graph).

### turbo4 Per-Block Independence

Unlike turbo3 (which shares a group norm across 4 blocks), turbo4 has a per-block RMS norm. The turbo4 SET_ROWS kernel can technically work per-block (32 elements), BUT it still uses 32-element Hadamard + sign conditioning. Since the cpy_blck signature is `void(const char*, char*)` taking exactly QK=32 floats, a per-block `cpy_blck_f32_turbo4_0` IS feasible. However, for code consistency and future-proofing (turbo4 may gain group-level features), use the same group-level kernel approach.

## Risks & Edge Cases

1. **Register pressure**: FWHT-128 needs 128 floats in registers (512 bytes). Intel Arc's register file is generous (128 GRFs × 32 bytes = 4KB per thread), but high occupancy may be impacted. Mitigation: Use local memory (`sycl::local_accessor`) for the FWHT scratch if register spilling is observed.

2. **Sign vector correctness**: The FWHT signs (seed=42) must match exactly what the fork's model quantization used. Get them from the fork's `ggml-common.h` or the Eliza reference. Any mismatch produces garbage KV cache.

3. **fp16 norm precision**: The `norm` field stores fp16. For very small vectors (near-zero KV entries), the corrected norm may underflow to zero. The reference handles this with `> 1e-10f` guards.

4. **4-bit midpoint computation**: The plan uses precomputed midpoints. These must be the exact averages of consecutive centroids. Verify against the fork's codebook.

5. **Flash attention tile kernel**: The tile-based FA kernel uses `sycl::half2` loads. Turbo types don't pack into half2 naturally. Initial support targets the VEC kernel only. The tile kernel falls back for unsupported types.

6. **Compilation time**: Each `FATTN_VEC_CASE` instantiation adds a template specialization. Adding 2 K types × multiple V types × 4 D values = many instantiations. Guard with `GGML_SYCL_FA_ALL_QUANTS` (already the pattern).

7. **ne00 not divisible by 128**: The SET_ROWS kernel assumes `ne00 % 128 == 0`. KV cache head dimensions are typically 64, 128, 256 — all multiples of 128 except 64. For D=64, a single group would span 2 heads. Need to handle: assert `ne00 % 128 == 0` or fall back to CPU for non-aligned sizes.

8. **SYCL sub-group (warp) size**: Intel Arc uses sub-group size 16 or 32. The fattn-vec kernel templates on `warp_size`. Turbo dot-product kernels must be tested with both sub-group sizes.

## Verification

1. **Build**: `cmake --build . --target llama-server` with `-DGGML_SYCL=ON` on a system with Intel oneAPI. Confirm no compilation errors.

2. **Smoke test**: Run `llama-server` with a small model using `--cache-type-k turbo4 --cache-type-v turbo3`. Verify no crash (the SET_ROWS error disappears).

3. **Correctness**: Compare perplexity of a short text with turbo KV cache vs f16 KV cache. Expected: turbo3 within ~0.1 PPL, turbo4 within ~0.05 PPL of f16.

4. **Performance**: Measure tok/s with turbo KV vs f16 KV. Turbo should be faster for long contexts (4.6x less memory bandwidth for KV cache reads).

5. **Unit test**: Port the Eliza reference `gen_fixture.c` self-test pattern — quantize a random 128-element vector, dequantize, verify dot-product matches reference within tolerance (1e-3 for turbo4, 1e-2 for turbo3).
