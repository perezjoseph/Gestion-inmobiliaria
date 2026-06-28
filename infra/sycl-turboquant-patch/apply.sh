#!/bin/bash
# Patches ggml-sycl for TurboQuant support
# Usage: bash apply.sh <path-to-ggml-sycl-dir>
set -e
SYCL_DIR="$1"

echo "  Patching set_rows.cpp..."
# Add include at top
sed -i '1i #include "turbo_quant.hpp"' "$SYCL_DIR/set_rows.cpp"

# Insert turbo cases BEFORE the GGML_ABORT using awk
awk '/GGML_ABORT\("Unsupported tensor type!"\)/ && !done {
    print "        case GGML_TYPE_TURBO3_0:"
    print "            set_rows_sycl_turbo3<TIdx>(src0_d, src1_d, (block_turbo3_0 *)dst->data, ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13, nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13, nb1, nb2, nb3, stream);"
    print "            break;"
    print "        case GGML_TYPE_TURBO4_0:"
    print "            set_rows_sycl_turbo4<TIdx>(src0_d, src1_d, (block_turbo4_0 *)dst->data, ne00, ne01, ne02, ne03, ne10, ne11, ne12, ne13, nb00, nb01, nb02, nb03, nb10, nb11, nb12, nb13, nb1, nb2, nb3, stream);"
    print "            break;"
    done=1
}
{print}' "$SYCL_DIR/set_rows.cpp" > /tmp/set_rows_patched.cpp
mv /tmp/set_rows_patched.cpp "$SYCL_DIR/set_rows.cpp"

echo "  Patching ggml-sycl.cpp..."

# 1. supports_op: add turbo types to the SET_ROWS accepted list
# Find the SET_ROWS case in do_ggml_backend_sycl_device_supports_op
sed -i '0,/op->type == GGML_TYPE_Q8_0/{s/op->type == GGML_TYPE_Q8_0/op->type == GGML_TYPE_Q8_0 || op->type == GGML_TYPE_TURBO3_0 || op->type == GGML_TYPE_TURBO4_0/}' "$SYCL_DIR/ggml-sycl.cpp"

# 2. ggml_sycl_op_f32 SET_ROWS element dispatch:
#    The standard code computes ne = ggml_nelements(src0) / QK then launches.
#    For turbo types, each block covers 128 elements (QK_TURBO3/4=128).
#    We need to ensure ne is divided by 128 (not smaller QK values).
#    The turbo set_rows kernels compute their own grid sizes internally, so
#    we just need the dispatch to not crash. Patch: add ne /= 128 guard for turbo.
awk '
/case GGML_OP_SET_ROWS:/ && !done_setrows {
    in_setrows = 1
}
in_setrows && /ne =.*ggml_nelements/ {
    print $0
    print "            if (dst->type == GGML_TYPE_TURBO3_0 || dst->type == GGML_TYPE_TURBO4_0) { ne = ne / 128; }"
    in_setrows = 0
    done_setrows = 1
    next
}
{print}
' "$SYCL_DIR/ggml-sycl.cpp" > /tmp/ggml_sycl_patched.cpp
mv /tmp/ggml_sycl_patched.cpp "$SYCL_DIR/ggml-sycl.cpp"

echo "  SYCL TurboQuant patch applied."
