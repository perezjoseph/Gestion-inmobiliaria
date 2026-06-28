#!/bin/bash
# Apply SYCL TurboQuant kernel patch to TheTom/llama-cpp-turboquant
# Run from the repo root after cloning:
#   git clone --branch feature/turboquant-kv-cache https://github.com/TheTom/llama-cpp-turboquant.git
#   cd llama-cpp-turboquant
#   bash /path/to/apply-patch.sh

set -e

PATCH_DIR="$(cd "$(dirname "$0")" && pwd)"
SYCL_DIR="ggml/src/ggml-sycl"

echo "==> Copying new TurboQuant SYCL headers..."
cp "$PATCH_DIR/turbo_quant.hpp"   "$SYCL_DIR/turbo_quant.hpp"
cp "$PATCH_DIR/turbo_dequant.hpp" "$SYCL_DIR/turbo_dequant.hpp"
cp "$PATCH_DIR/turbo_fattn.hpp"   "$SYCL_DIR/turbo_fattn.hpp"

echo "==> Patching set_rows.cpp to add turbo3/turbo4 dispatch..."
if ! grep -q "GGML_TYPE_TURBO3_0" "$SYCL_DIR/set_rows.cpp"; then
    # Add include at top
    sed -i '1s/^/#include "turbo_quant.hpp"\n/' "$SYCL_DIR/set_rows.cpp"

    # Add turbo cases before the default: case in the switch
    sed -i '/default:/i \        case GGML_TYPE_TURBO3_0:\
            set_rows_sycl_turbo3<TIdx>(src0_d, src1_d, (block_turbo3_0 *)dst->data, ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13, nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13, nb1, nb2, nb3, stream);\
            break;\
        case GGML_TYPE_TURBO4_0:\
            set_rows_sycl_turbo4<TIdx>(src0_d, src1_d, (block_turbo4_0 *)dst->data, ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13, nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13, nb1, nb2, nb3, stream);\
            break;' "$SYCL_DIR/set_rows.cpp"
    echo "    set_rows.cpp patched."
else
    echo "    set_rows.cpp already patched, skipping."
fi

echo "==> Patching ggml-sycl.cpp to add supports_op for turbo types..."
MAIN_CPP="$SYCL_DIR/ggml-sycl.cpp"
if ! grep -q "GGML_TYPE_TURBO3_0" "$MAIN_CPP"; then
    # Find the SET_ROWS case in supports_op and add turbo types to the accepted list
    # Look for the pattern where Q8_0 is accepted and add turbo types
    sed -i 's/op->type == GGML_TYPE_Q8_0/op->type == GGML_TYPE_Q8_0 || op->type == GGML_TYPE_TURBO3_0 || op->type == GGML_TYPE_TURBO4_0/g' "$MAIN_CPP"
    echo "    ggml-sycl.cpp patched."
else
    echo "    ggml-sycl.cpp already patched, skipping."
fi

echo ""
echo "==> Patch applied. Build with:"
echo "    cmake -B build -DGGML_SYCL=ON -DGGML_SYCL_TARGET=INTEL \\"
echo "      -DCMAKE_C_COMPILER=icx -DCMAKE_CXX_COMPILER=icpx \\"
echo "      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF"
echo "    cmake --build build -j\$(nproc) --target llama-server"
echo ""
echo "    Then run with: --cache-type-k turbo4 --cache-type-v turbo3"
