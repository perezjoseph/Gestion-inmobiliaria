# Custom vLLM image for Intel XPU with MoE gate layer fix.
#
# Problem: vLLM's auto_round.py quantization code path on XPU fails to skip
# MoE gate layers marked as bits:16 in extra_config, causing
# UR_RESULT_ERROR_DEVICE_LOST when GPTQLinearMethod.create_weights tries to
# create quantized weight tensors for what should be fp16 layers.
#
# Fix: Patch apply_ipex_quant_layer and apply_gptq_quant_layer to add an
# explicit early-exit check for 16-bit layers from extra_config.

FROM docker.io/intel/vllm:0.14.1-xpu

COPY patch_auto_round.py /tmp/patch_auto_round.py

USER root
RUN python3 /tmp/patch_auto_round.py && rm /tmp/patch_auto_round.py
USER 1000
