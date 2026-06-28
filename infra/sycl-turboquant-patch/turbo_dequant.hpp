#ifndef GGML_SYCL_TURBO_DEQUANT_HPP
#define GGML_SYCL_TURBO_DEQUANT_HPP

#include "common.hpp"
#include "turbo_quant.hpp"

// Dequantization functions for turbo3_0 and turbo4_0.
// Used by get_rows and flash-attention V path.
// Returns values in ROTATED space (Q is pre-rotated by the model graph).

// turbo3 dequantize: unpack 3-bit index from block, multiply by centroid * norm
// Follows the same signature pattern as dequantize_q4_0 in dequantize.hpp
static __dpct_inline__ void dequantize_turbo3_0(
    const void * __restrict__ vx, const int64_t ib, const int iqs, dfloat2 & v) {

    const block_turbo3_0 * x = (const block_turbo3_0 *)vx;
    const block_turbo3_0 & blk = x[ib];

    const float norm = turbo_fp16_to_fp32(blk.norm);

    // Element iqs*2 and iqs*2+1 (dfloat2 returns two consecutive values)
    const int j0 = iqs * 2;
    const int j1 = j0 + 1;

    // Unpack index for j0
    uint8_t low2_0 = (uint8_t)((blk.qs[j0 / 4] >> ((j0 % 4) * 2)) & 0x3);
    uint8_t hi1_0  = (uint8_t)((blk.signs[j0 / 8] >> (j0 % 8)) & 0x1);
    uint8_t idx0   = (uint8_t)(low2_0 | (hi1_0 << 2));

    // Unpack index for j1
    uint8_t low2_1 = (uint8_t)((blk.qs[j1 / 4] >> ((j1 % 4) * 2)) & 0x3);
    uint8_t hi1_1  = (uint8_t)((blk.signs[j1 / 8] >> (j1 % 8)) & 0x1);
    uint8_t idx1   = (uint8_t)(low2_1 | (hi1_1 << 2));

    v.x() = TURBO3_CENTROIDS[idx0] * norm;
    v.y() = TURBO3_CENTROIDS[idx1] * norm;
}

// turbo4 dequantize: unpack 4-bit index from packed layout
static __dpct_inline__ void dequantize_turbo4_0(
    const void * __restrict__ vx, const int64_t ib, const int iqs, dfloat2 & v) {

    const block_turbo4_0 * x = (const block_turbo4_0 *)vx;
    const block_turbo4_0 & blk = x[ib];

    const float norm = turbo_fp16_to_fp32(blk.norm);

    const int j0 = iqs * 2;
    const int j1 = j0 + 1;

    // turbo4 packing: first 16 elements in low nibbles, last 16 in high nibbles
    auto get_idx = [&](int j) -> uint8_t {
        int byte_idx = j & 15;
        uint8_t packed = blk.qs[byte_idx];
        return j < 16 ? (uint8_t)(packed & 0x0F) : (uint8_t)(packed >> 4);
    };

    v.x() = TURBO4_CENTROIDS[get_idx(j0)] * norm;
    v.y() = TURBO4_CENTROIDS[get_idx(j1)] * norm;
}

// Single-element dequantize (for flash-attention element access)
static inline float dequantize_turbo3_0_single(const block_turbo3_0 & blk, int j) {
    const float norm = turbo_fp16_to_fp32(blk.norm);
    uint8_t low2 = (uint8_t)((blk.qs[j / 4] >> ((j % 4) * 2)) & 0x3);
    uint8_t hi1  = (uint8_t)((blk.signs[j / 8] >> (j % 8)) & 0x1);
    uint8_t idx  = (uint8_t)(low2 | (hi1 << 2));
    return TURBO3_CENTROIDS[idx] * norm;
}

static inline float dequantize_turbo4_0_single(const block_turbo4_0 & blk, int j) {
    const float norm = turbo_fp16_to_fp32(blk.norm);
    int byte_idx = j & 15;
    uint8_t packed = blk.qs[byte_idx];
    uint8_t idx = j < 16 ? (uint8_t)(packed & 0x0F) : (uint8_t)(packed >> 4);
    return TURBO4_CENTROIDS[idx] * norm;
}

// Dequantize full block (32 elements) into float buffer
// Used for get_rows and non-fused flash attention paths
static inline void dequantize_turbo3_0_block(const block_turbo3_0 & blk, float * dst) {
    const float norm = turbo_fp16_to_fp32(blk.norm);
    for (int j = 0; j < 32; j++) {
        uint8_t low2 = (uint8_t)((blk.qs[j / 4] >> ((j % 4) * 2)) & 0x3);
        uint8_t hi1  = (uint8_t)((blk.signs[j / 8] >> (j % 8)) & 0x1);
        uint8_t idx  = (uint8_t)(low2 | (hi1 << 2));
        dst[j] = TURBO3_CENTROIDS[idx] * norm;
    }
}

static inline void dequantize_turbo4_0_block(const block_turbo4_0 & blk, float * dst) {
    const float norm = turbo_fp16_to_fp32(blk.norm);
    for (int j = 0; j < 32; j++) {
        int byte_idx = j & 15;
        uint8_t packed = blk.qs[byte_idx];
        uint8_t idx = j < 16 ? (uint8_t)(packed & 0x0F) : (uint8_t)(packed >> 4);
        dst[j] = TURBO4_CENTROIDS[idx] * norm;
    }
}

#endif // GGML_SYCL_TURBO_DEQUANT_HPP
