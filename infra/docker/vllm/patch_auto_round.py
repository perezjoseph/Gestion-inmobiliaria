#!/usr/bin/env python3
"""
Patch vLLM's auto_round.py to fix MoE gate layer quantization on XPU.

Problem: Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound marks all MoE gate
layers as bits:16 in extra_config (they should remain fp16). On XPU, the
get_quant_method/apply_ipex_quant_layer code path sometimes fails to detect
these 16-bit layers and assigns GPTQLinearMethod, causing a
UR_RESULT_ERROR_DEVICE_LOST crash when creating quantized weight tensors.

Fix: Insert an explicit early-exit check at the top of apply_ipex_quant_layer
that reads extra_config and returns UnquantizedLinearMethod for layers with
bits >= 16. This is a belt-and-suspenders fix that ensures the gate layers
never reach the GPTQ/IPEX quantization code path.
"""

import sys
from pathlib import Path

# Find the auto_round.py file
search_paths = [
    Path("/usr/local/lib/python3.12/dist-packages/vllm/model_executor/layers/quantization/auto_round.py"),
    Path("/usr/local/lib/python3.11/dist-packages/vllm/model_executor/layers/quantization/auto_round.py"),
]

target = None
for p in search_paths:
    if p.exists():
        target = p
        break

if target is None:
    # Try to find it dynamically
    import glob
    matches = glob.glob("/usr/local/lib/python3.*/dist-packages/vllm/model_executor/layers/quantization/auto_round.py")
    if not matches:
        matches = glob.glob("/usr/lib/python3.*/dist-packages/vllm/model_executor/layers/quantization/auto_round.py")
    if matches:
        target = Path(matches[0])
    else:
        print("ERROR: Could not find auto_round.py", file=sys.stderr)
        sys.exit(1)

print(f"Patching {target}")

content = target.read_text()

# Patch 1: Add early exit to apply_ipex_quant_layer for 16-bit extra_config layers
OLD_APPLY_IPEX = "    def apply_ipex_quant_layer(self, layer, prefix: str):"
NEW_APPLY_IPEX = '''    def apply_ipex_quant_layer(self, layer, prefix: str):
        # [PATCH] Early exit for MoE gate layers marked as 16-bit in extra_config.
        # Prevents UR_RESULT_ERROR_DEVICE_LOST on XPU when GPTQLinearMethod
        # tries to create quantized weight tensors for fp16 layers.
        if self.extra_config and prefix:
            from vllm.model_executor.layers.linear import LinearBase
            from vllm.model_executor.layers.vocab_parallel_embedding import ParallelLMHead
            for cfg_key, cfg_val in self.extra_config.items():
                if cfg_key == prefix or cfg_key == f"model.{prefix}" or f"model.{cfg_key}" == prefix:
                    if isinstance(cfg_val, dict) and cfg_val.get("bits", self.weight_bits) >= 16:
                        if isinstance(layer, (LinearBase, ParallelLMHead)):
                            return UnquantizedLinearMethod()
                        return None'''

if OLD_APPLY_IPEX in content:
    content = content.replace(OLD_APPLY_IPEX, NEW_APPLY_IPEX, 1)
    print("  ✓ Patched apply_ipex_quant_layer")
else:
    print("  WARNING: Could not find apply_ipex_quant_layer signature to patch", file=sys.stderr)
    # Try alternative: patch apply_gptq_quant_layer as well
    pass

# Patch 2: Also add check to apply_gptq_quant_layer for non-XPU fallback
OLD_APPLY_GPTQ = "    def apply_gptq_quant_layer(self, layer, prefix: str, backend: str = \"auto\"):"
NEW_APPLY_GPTQ = '''    def apply_gptq_quant_layer(self, layer, prefix: str, backend: str = "auto"):
        # [PATCH] Early exit for layers marked as 16-bit in extra_config.
        if self.extra_config and prefix:
            from vllm.model_executor.layers.linear import LinearBase
            from vllm.model_executor.layers.vocab_parallel_embedding import ParallelLMHead
            for cfg_key, cfg_val in self.extra_config.items():
                if cfg_key == prefix or cfg_key == f"model.{prefix}" or f"model.{cfg_key}" == prefix:
                    if isinstance(cfg_val, dict) and cfg_val.get("bits", self.weight_bits) >= 16:
                        if isinstance(layer, (LinearBase, ParallelLMHead)):
                            return UnquantizedLinearMethod()
                        return None'''

if OLD_APPLY_GPTQ in content:
    content = content.replace(OLD_APPLY_GPTQ, NEW_APPLY_GPTQ, 1)
    print("  ✓ Patched apply_gptq_quant_layer")
else:
    print("  INFO: apply_gptq_quant_layer not found (may not exist in this version)")

target.write_text(content)
print("Patch applied successfully.")
