#ifndef GGML_SYCL_TURBO_QUANT_HPP
#define GGML_SYCL_TURBO_QUANT_HPP

// TurboQuant SYCL kernel support for SET_ROWS (f32 -> turbo3_0 / turbo4_0)
// Derived from TheTom/llama-cpp-turboquant ggml/src/ggml-turbo-quant.c
// and ggml/src/ggml-common.h (branch: feature/turboquant-kv-cache)
//
// Block layouts (must match ggml-common.h exactly):
//
//   block_turbo3_0  (14 bytes, QK_TURBO3=128):
//     ggml_half norm;         2 bytes
//     uint8_t   qs[32];       8 bytes  (lower 2 bits of 3-bit index, 4 per byte)
//     uint8_t   signs[16];    4 bytes  (upper 1 bit of 3-bit index, 8 per byte)
//
//   block_turbo4_0  (68 bytes, QK_TURBO4=128, TURBO4_USE_4BIT=1):
//     ggml_half norm;         2 bytes
//     ggml_half rnorm;        2 bytes  (reserved)
//     uint8_t   qs[64];      64 bytes  (nibble-packed 4-bit indices)

#include "common.hpp"

// ── TURBO3 codebook (3-bit, 8 centroids) ─────────────────────────────────────
// Lloyd-Max for N(0, 1/128), from ggml-turbo-quant.c CENTROIDS_3BIT
static constexpr float TURBO3_CENTROIDS[8] = {
    -0.190685f, -0.117832f, -0.065717f, -0.021460f,
     0.021460f,  0.065717f,  0.117832f,  0.190685f,
};

static constexpr float TURBO3_MIDPOINTS[7] = {
    -0.154259f, -0.091775f, -0.043589f, 0.0f, 0.043589f, 0.091775f, 0.154259f,
};

// ── TURBO4 codebook (4-bit, 16 centroids) ────────────────────────────────────
// From ggml-turbo-quant.c CENTROIDS_4BIT (TURBO4_USE_4BIT path)
static constexpr float TURBO4_CENTROIDS[16] = {
    -0.173926f, -0.117195f, -0.089527f, -0.068756f,
    -0.051262f, -0.035597f, -0.020989f, -0.006938f,
     0.006938f,  0.020989f,  0.035597f,  0.051262f,
     0.068756f,  0.089527f,  0.117195f,  0.173926f,
};

static constexpr float TURBO4_MIDPOINTS[15] = {
    -0.145561f, -0.103361f, -0.079142f, -0.060009f, -0.043430f,
    -0.028293f, -0.013963f,  0.000000f,  0.013963f,  0.028293f,
     0.043430f,  0.060009f,  0.079142f,  0.103361f,  0.145561f,
};

// ── WHT sign vectors (seed=42, from ggml-turbo-quant.c turbo_cpu_s1/s2) ──────
static constexpr float TURBO_WHT_S1[128] = {
    -1,1,1,-1,-1,1,-1,1,-1,-1,1,1,1,1,1,1,1,-1,1,-1,1,-1,-1,1,1,1,-1,1,1,-1,-1,-1,
    -1,1,1,-1,1,1,-1,1,-1,1,1,-1,-1,1,-1,1,1,1,1,-1,-1,-1,-1,-1,1,-1,1,1,1,1,-1,1,
    -1,-1,1,-1,-1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,1,-1,-1,1,1,1,-1,-1,1,1,-1,1,1,-1,1,-1,
    -1,1,1,-1,1,-1,1,-1,1,1,1,1,-1,1,-1,1,1,-1,1,1,-1,-1,-1,-1,-1,1,1,-1,1,1,-1,1
};

static constexpr float TURBO_WHT_S2[128] = {
    1,1,1,1,-1,1,1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,-1,-1,1,-1,1,-1,1,-1,-1,1,-1,1,1,1,
    1,1,-1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,1,-1,1,-1,1,1,1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,
    1,-1,1,-1,-1,-1,-1,1,-1,1,-1,1,-1,-1,1,1,-1,1,-1,1,1,-1,1,-1,-1,-1,-1,1,-1,-1,1,-1,
    1,-1,1,1,1,-1,-1,1,-1,1,-1,1,1,-1,-1,1,-1,1,-1,1,1,-1,1,-1,1,-1,-1,-1,-1,-1,1,-1
};

static constexpr float INV_SQRT_128 = 0.08838834764831845f;

// ── fp16 helpers (device-safe, no __builtin_memcpy) ───────────────────────────
static inline uint16_t turbo_fp32_to_fp16(float f) {
    sycl::half h(f);
    return sycl::bit_cast<uint16_t>(h);
}

static inline float turbo_fp16_to_fp32(uint16_t bits) {
    sycl::half h = sycl::bit_cast<sycl::half>(bits);
    return float(h);
}

// ── FWHT (128 elements, in-place) ────────────────────────────────────────────
// Matches turbo_cpu_fwht() from ggml-turbo-quant.c:
//   signs1 → butterfly → normalize×signs2
static inline void fwht_128(float * x) {
    constexpr float s1[128] = {
        -1,1,1,-1,-1,1,-1,1,-1,-1,1,1,1,1,1,1,1,-1,1,-1,1,-1,-1,1,1,1,-1,1,1,-1,-1,-1,
        -1,1,1,-1,1,1,-1,1,-1,1,1,-1,-1,1,-1,1,1,1,1,-1,-1,-1,-1,-1,1,-1,1,1,1,1,-1,1,
        -1,-1,1,-1,-1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,1,-1,-1,1,1,1,-1,-1,1,1,-1,1,1,-1,1,-1,
        -1,1,1,-1,1,-1,1,-1,1,1,1,1,-1,1,-1,1,1,-1,1,1,-1,-1,-1,-1,-1,1,1,-1,1,1,-1,1
    };
    constexpr float s2[128] = {
        1,1,1,1,-1,1,1,-1,1,-1,-1,-1,1,-1,-1,-1,1,1,-1,-1,1,-1,1,-1,1,-1,-1,1,-1,1,1,1,
        1,1,-1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,1,-1,1,-1,1,1,1,-1,-1,1,-1,-1,-1,-1,-1,-1,1,1,
        1,-1,1,-1,-1,-1,-1,1,-1,1,-1,1,-1,-1,1,1,-1,1,-1,1,1,-1,1,-1,-1,-1,-1,1,-1,-1,1,-1,
        1,-1,1,1,1,-1,-1,1,-1,1,-1,1,1,-1,-1,1,-1,1,-1,1,1,-1,1,-1,1,-1,-1,-1,-1,-1,1,-1
    };
    constexpr float inv_sqrt = 0.08838834764831845f;

    for (int i = 0; i < 128; i++) x[i] *= s1[i];
    for (int h = 1; h < 128; h *= 2) {
        for (int i = 0; i < 128; i += h * 2) {
            for (int j = i; j < i + h; j++) {
                float a = x[j], b = x[j + h];
                x[j]     = a + b;
                x[j + h] = a - b;
            }
        }
    }
    for (int i = 0; i < 128; i++) x[i] *= inv_sqrt * s2[i];
}

// ── Nearest centroid lookups ──────────────────────────────────────────────────
static inline uint8_t nearest_3bit(float v) {
    if (v < TURBO3_MIDPOINTS[0]) return 0;
    if (v < TURBO3_MIDPOINTS[1]) return 1;
    if (v < TURBO3_MIDPOINTS[2]) return 2;
    if (v < TURBO3_MIDPOINTS[3]) return 3;
    if (v < TURBO3_MIDPOINTS[4]) return 4;
    if (v < TURBO3_MIDPOINTS[5]) return 5;
    if (v < TURBO3_MIDPOINTS[6]) return 6;
    return 7;
}

static inline uint8_t nearest_4bit(float v) {
    if (v < TURBO4_MIDPOINTS[ 0]) return  0;
    if (v < TURBO4_MIDPOINTS[ 1]) return  1;
    if (v < TURBO4_MIDPOINTS[ 2]) return  2;
    if (v < TURBO4_MIDPOINTS[ 3]) return  3;
    if (v < TURBO4_MIDPOINTS[ 4]) return  4;
    if (v < TURBO4_MIDPOINTS[ 5]) return  5;
    if (v < TURBO4_MIDPOINTS[ 6]) return  6;
    if (v < TURBO4_MIDPOINTS[ 7]) return  7;
    if (v < TURBO4_MIDPOINTS[ 8]) return  8;
    if (v < TURBO4_MIDPOINTS[ 9]) return  9;
    if (v < TURBO4_MIDPOINTS[10]) return 10;
    if (v < TURBO4_MIDPOINTS[11]) return 11;
    if (v < TURBO4_MIDPOINTS[12]) return 12;
    if (v < TURBO4_MIDPOINTS[13]) return 13;
    if (v < TURBO4_MIDPOINTS[14]) return 14;
    return 15;
}

// ── TURBO3 quantize: 128 f32 → one block_turbo3_0 (14 bytes) ─────────────────
// Mirrors quantize_row_turbo3_0_ref() from ggml-turbo-quant.c exactly.
// QK_TURBO3=128: one block covers the full rotation group.
static inline void quantize_turbo3_block(const float * src, void * dst_v) {
    auto * blk = static_cast<block_turbo3_0 *>(dst_v);

    float buf[128];
    float norm_sq = 0.0f;
    for (int j = 0; j < 128; j++) {
        buf[j] = src[j];
        norm_sq += buf[j] * buf[j];
    }
    float grp_norm = sqrtf(norm_sq);
    float inv_norm = grp_norm > 1e-10f ? 1.0f / grp_norm : 0.0f;
    for (int j = 0; j < 128; j++) buf[j] *= inv_norm;

    // Forward WHT rotation
    fwht_128(buf);

    // Quantize + pack — qs[32] holds lower 2 bits, signs[16] holds upper 1 bit
    for (int i = 0; i < 32;  i++) blk->qs[i]    = 0;
    for (int i = 0; i < 16;  i++) blk->signs[i] = 0;

    float recon_sq = 0.0f;
    constexpr float C3[8] = {
        -0.190685f, -0.117832f, -0.065717f, -0.021460f,
         0.021460f,  0.065717f,  0.117832f,  0.190685f,
    };
    for (int j = 0; j < 128; j++) {
        uint8_t idx = nearest_3bit(buf[j]);
        blk->qs[j / 4]    |= (uint8_t)((idx & 0x3) << ((j % 4) * 2));
        if (idx & 0x4)
            blk->signs[j / 8] |= (uint8_t)(1 << (j % 8));
        recon_sq += C3[idx] * C3[idx];
    }

    float recon_norm = sqrtf(recon_sq);
    float corrected  = recon_norm > 1e-10f ? grp_norm / recon_norm : grp_norm;
    blk->norm = sycl::half(corrected);
}

// ── TURBO4 quantize: 128 f32 → one block_turbo4_0 (68 bytes) ─────────────────
// Mirrors quantize_row_turbo4_0_ref() TURBO4_USE_4BIT=1 path.
// 4-bit nibble-packed PolarQuant with same WHT rotation as turbo3.
static inline void quantize_turbo4_block(const float * src, void * dst_v) {
    auto * blk = static_cast<block_turbo4_0 *>(dst_v);

    float buf[128];
    float norm_sq = 0.0f;
    for (int j = 0; j < 128; j++) {
        buf[j] = src[j];
        norm_sq += buf[j] * buf[j];
    }
    float norm    = sqrtf(norm_sq);
    float inv_norm = norm > 1e-10f ? 1.0f / norm : 0.0f;
    for (int j = 0; j < 128; j++) buf[j] *= inv_norm;

    // Forward WHT rotation (same as turbo3)
    fwht_128(buf);

    // 4-bit nearest centroid + nibble pack into qs[64]
    for (int i = 0; i < 64; i++) blk->qs[i] = 0;

    float recon_sq = 0.0f;
    constexpr float C4[16] = {
        -0.173926f, -0.117195f, -0.089527f, -0.068756f,
        -0.051262f, -0.035597f, -0.020989f, -0.006938f,
         0.006938f,  0.020989f,  0.035597f,  0.051262f,
         0.068756f,  0.089527f,  0.117195f,  0.173926f,
    };
    for (int j = 0; j < 128; j++) {
        uint8_t idx = nearest_4bit(buf[j]);
        blk->qs[j / 2] |= (uint8_t)((idx & 0xF) << ((j % 2) * 4));
        recon_sq += C4[idx] * C4[idx];
    }

    float recon_norm = sqrtf(recon_sq);
    float corrected  = recon_norm > 1e-10f ? norm / recon_norm : norm;
    blk->norm  = sycl::half(corrected);
    blk->rnorm = sycl::half(0.0f);  // reserved
}

// ── SET_ROWS SYCL kernels: one work-item per 128-element block ────────────────

template <typename TIdx>
static void set_rows_sycl_turbo3(
        const char * __restrict__ src0_d,
        const TIdx * __restrict__ src1_d,
        block_turbo3_0 * __restrict__ dst_d,
        const int64_t ne00, const int64_t ne01, const int64_t ne02, const int64_t ne03,
        const int64_t ne10, const int64_t ne11, const int64_t ne12, const int64_t ne13,
        const size_t nb00, const size_t nb01, const size_t nb02, const size_t nb03,
        const size_t nb10, const size_t nb11, const size_t nb12, const size_t nb13,
        const size_t nb1,  const size_t nb2,  const size_t nb3,
        queue_ptr stream) {

    // One block covers 128 elements (QK_TURBO3=128)
    const int64_t blocks_per_row = ne00 / 128;
    const int64_t total_blocks   = blocks_per_row * ne01 * ne02 * ne03;
    constexpr int wg = 64;
    const int64_t grid = ceil_div(total_blocks, wg);

    stream->parallel_for(sycl::nd_range<1>(grid * wg, wg), [=](sycl::nd_item<1> it) {
        const int64_t ib = it.get_global_linear_id();
        if (ib >= total_blocks) return;

        const int64_t ib_row  = ib % blocks_per_row;
        const int64_t rem     = ib / blocks_per_row;
        const int64_t i01     = rem % ne01;
        const int64_t rem2    = rem / ne01;
        const int64_t i02     = rem2 % ne02;
        const int64_t i03     = rem2 / ne02;
        const int64_t elem0   = ib_row * 128;

        const int64_t i12  = i03 % ne12;
        const int64_t i11  = i02 % ne11;
        const int64_t i10  = i01;

        const size_t src_off = i01 * nb01 + i02 * nb02 + i03 * nb03;
        const float * src_ptr = reinterpret_cast<const float *>(src0_d + src_off) + elem0;

        const size_t s1_off = i10 * nb10 + i11 * nb11 + i12 * nb12;
        const int64_t dst_row = src1_d[s1_off / sizeof(TIdx)];

        const size_t dst_off = dst_row * nb1 + i02 * nb2 + i03 * nb3
                             + ib_row * sizeof(block_turbo3_0);
        block_turbo3_0 * dst_ptr = reinterpret_cast<block_turbo3_0 *>(
            reinterpret_cast<char *>(dst_d) + dst_off);

        float buf[128];
        for (int i = 0; i < 128; i++) buf[i] = src_ptr[i];
        quantize_turbo3_block(buf, dst_ptr);
    });

    GGML_UNUSED(ne10); GGML_UNUSED(ne13);
    GGML_UNUSED(nb00); GGML_UNUSED(nb13);
}

template <typename TIdx>
static void set_rows_sycl_turbo4(
        const char * __restrict__ src0_d,
        const TIdx * __restrict__ src1_d,
        block_turbo4_0 * __restrict__ dst_d,
        const int64_t ne00, const int64_t ne01, const int64_t ne02, const int64_t ne03,
        const int64_t ne10, const int64_t ne11, const int64_t ne12, const int64_t ne13,
        const size_t nb00, const size_t nb01, const size_t nb02, const size_t nb03,
        const size_t nb10, const size_t nb11, const size_t nb12, const size_t nb13,
        const size_t nb1,  const size_t nb2,  const size_t nb3,
        queue_ptr stream) {

    const int64_t blocks_per_row = ne00 / 128;
    const int64_t total_blocks   = blocks_per_row * ne01 * ne02 * ne03;
    constexpr int wg = 64;
    const int64_t grid = ceil_div(total_blocks, wg);

    stream->parallel_for(sycl::nd_range<1>(grid * wg, wg), [=](sycl::nd_item<1> it) {
        const int64_t ib = it.get_global_linear_id();
        if (ib >= total_blocks) return;

        const int64_t ib_row  = ib % blocks_per_row;
        const int64_t rem     = ib / blocks_per_row;
        const int64_t i01     = rem % ne01;
        const int64_t rem2    = rem / ne01;
        const int64_t i02     = rem2 % ne02;
        const int64_t i03     = rem2 / ne02;
        const int64_t elem0   = ib_row * 128;

        const int64_t i12 = i03 % ne12;
        const int64_t i11 = i02 % ne11;
        const int64_t i10 = i01;

        const size_t src_off = i01 * nb01 + i02 * nb02 + i03 * nb03;
        const float * src_ptr = reinterpret_cast<const float *>(src0_d + src_off) + elem0;

        const size_t s1_off = i10 * nb10 + i11 * nb11 + i12 * nb12;
        const int64_t dst_row = src1_d[s1_off / sizeof(TIdx)];

        const size_t dst_off = dst_row * nb1 + i02 * nb2 + i03 * nb3
                             + ib_row * sizeof(block_turbo4_0);
        block_turbo4_0 * dst_ptr = reinterpret_cast<block_turbo4_0 *>(
            reinterpret_cast<char *>(dst_d) + dst_off);

        float buf[128];
        for (int i = 0; i < 128; i++) buf[i] = src_ptr[i];
        quantize_turbo4_block(buf, dst_ptr);
    });

    GGML_UNUSED(ne10); GGML_UNUSED(ne13);
    GGML_UNUSED(nb00); GGML_UNUSED(nb13);
}

#endif // GGML_SYCL_TURBO_QUANT_HPP
