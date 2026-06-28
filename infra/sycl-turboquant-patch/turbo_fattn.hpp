#ifndef GGML_SYCL_TURBO_FATTN_HPP
#define GGML_SYCL_TURBO_FATTN_HPP

#include "common.hpp"
#include "turbo_dequant.hpp"

// Flash Attention support for TurboQuant KV cache types.
// K dot-product and V dequantization for the fattn-vec kernel.

// vec_dot for turbo3 K: compute dot(Q_f16, dequant(K_turbo3)) for D elements
// Q is pre-rotated by the attention graph, K is stored in rotated space.
// The dot product in rotated space equals the original dot product
// (orthogonal rotation preserves inner products).
template <int D>
static __dpct_inline__ float vec_dot_fattn_vec_KQ_turbo3_0(
    const char * __restrict__ K_c,
    const void * __restrict__ Q_v,
    const uint16_t * __restrict__ Q_ds_v,
    const int iKV) {

    const block_turbo3_0 * K_blocks = (const block_turbo3_0 *)K_c;
    const sycl::half * Q_h = (const sycl::half *)Q_v;

    constexpr int blocks_per_head = D / 32;
    const int block_offset = iKV * blocks_per_head;

    float sum = 0.0f;
    for (int b = 0; b < blocks_per_head; b++) {
        const block_turbo3_0 & blk = K_blocks[block_offset + b];
        const float norm = turbo_fp16_to_fp32(blk.norm);

        for (int j = 0; j < 32; j++) {
            uint8_t low2 = (uint8_t)((blk.qs[j / 4] >> ((j % 4) * 2)) & 0x3);
            uint8_t hi1  = (uint8_t)((blk.signs[j / 8] >> (j % 8)) & 0x1);
            uint8_t idx  = (uint8_t)(low2 | (hi1 << 2));
            float k_val = TURBO3_CENTROIDS[idx] * norm;
            float q_val = (float)Q_h[b * 32 + j];
            sum += q_val * k_val;
        }
    }
    return sum;
}

// vec_dot for turbo4 K
template <int D>
static __dpct_inline__ float vec_dot_fattn_vec_KQ_turbo4_0(
    const char * __restrict__ K_c,
    const void * __restrict__ Q_v,
    const uint16_t * __restrict__ Q_ds_v,
    const int iKV) {

    const block_turbo4_0 * K_blocks = (const block_turbo4_0 *)K_c;
    const sycl::half * Q_h = (const sycl::half *)Q_v;

    constexpr int blocks_per_head = D / 32;
    const int block_offset = iKV * blocks_per_head;

    float sum = 0.0f;
    for (int b = 0; b < blocks_per_head; b++) {
        const block_turbo4_0 & blk = K_blocks[block_offset + b];
        const float norm = turbo_fp16_to_fp32(blk.norm);

        for (int j = 0; j < 32; j++) {
            int byte_idx = j & 15;
            uint8_t packed = blk.qs[byte_idx];
            uint8_t idx = j < 16 ? (uint8_t)(packed & 0x0F) : (uint8_t)(packed >> 4);
            float k_val = TURBO4_CENTROIDS[idx] * norm;
            float q_val = (float)Q_h[b * 32 + j];
            sum += q_val * k_val;
        }
    }
    return sum;
}

// V dequantize for flash attention: load ne consecutive elements starting at i0
// Used during softmax(scores) * V accumulation.
template <typename T, int ne>
static __dpct_inline__ void dequantize_V_turbo3_0(
    const void * __restrict__ vx, T * __restrict__ dst, const int64_t i0) {

    const block_turbo3_0 * blocks = (const block_turbo3_0 *)vx;

    for (int i = 0; i < ne; i++) {
        const int64_t elem = i0 + i;
        const int64_t ib = elem / 32;
        const int j = (int)(elem % 32);

        const block_turbo3_0 & blk = blocks[ib];
        const float norm = turbo_fp16_to_fp32(blk.norm);

        uint8_t low2 = (uint8_t)((blk.qs[j / 4] >> ((j % 4) * 2)) & 0x3);
        uint8_t hi1  = (uint8_t)((blk.signs[j / 8] >> (j % 8)) & 0x1);
        uint8_t idx  = (uint8_t)(low2 | (hi1 << 2));

        dst[i] = (T)(TURBO3_CENTROIDS[idx] * norm);
    }
}

template <typename T, int ne>
static __dpct_inline__ void dequantize_V_turbo4_0(
    const void * __restrict__ vx, T * __restrict__ dst, const int64_t i0) {

    const block_turbo4_0 * blocks = (const block_turbo4_0 *)vx;

    for (int i = 0; i < ne; i++) {
        const int64_t elem = i0 + i;
        const int64_t ib = elem / 32;
        const int j = (int)(elem % 32);

        const block_turbo4_0 & blk = blocks[ib];
        const float norm = turbo_fp16_to_fp32(blk.norm);

        int byte_idx = j & 15;
        uint8_t packed = blk.qs[byte_idx];
        uint8_t idx = j < 16 ? (uint8_t)(packed & 0x0F) : (uint8_t)(packed >> 4);

        dst[i] = (T)(TURBO4_CENTROIDS[idx] * norm);
    }
}

#endif // GGML_SYCL_TURBO_FATTN_HPP
