# Quantization Guide

## Contents
- Quantization methods comparison
- AWQ setup and usage
- GPTQ setup and usage
- FP8 quantization (H100 / Intel Arc)
- INT4 quantization (Intel Arc)
- MXFP4 quantization (Intel Arc)
- Model preparation
- Accuracy vs compression trade-offs

## Quantization methods comparison

| Method | Compression | Accuracy Loss | Speed | Best For |
|--------|-------------|---------------|-------|----------|
| **AWQ** | 4-bit (75%) | <1% | Fast | NVIDIA, 70B models, production |
| **GPTQ** | 4-bit (75%) | 1-2% | Fast | NVIDIA, wide model support |
| **FP8** | 8-bit (50%) | <0.5% | Fastest | H100 (NVIDIA) or Intel Arc |
| **INT4 (sym_int4)** | 4-bit (75%) | ~1% | Fast | Intel Arc, all models |
| **MXFP4** | 4-bit (75%) | ~1% | Fast | Intel Arc, gpt-oss-20b/120b only |
| **SqueezeLLM** | 3-4 bit (75-80%) | 2-3% | Medium | Extreme compression |

**Recommendation**:
- **NVIDIA production**: Use AWQ for 70B models
- **H100 GPUs**: Use FP8 for best speed
- **Intel Arc**: Use FP8 (best speed) or sym_int4 (more compression)
- **Maximum compatibility**: Use GPTQ
- **Extreme compression**: Use SqueezeLLM

## AWQ setup and usage

**AWQ** (Activation-aware Weight Quantization) achieves best accuracy at 4-bit.

**Step 1: Find pre-quantized model**

Search HuggingFace for AWQ models:
```bash
# Example: TheBloke/Llama-2-70B-AWQ
# Example: TheBloke/Mixtral-8x7B-Instruct-v0.1-AWQ
```

**Step 2: Launch with AWQ**

```bash
vllm serve TheBloke/Llama-2-70B-AWQ \
  --quantization awq \
  --tensor-parallel-size 1 \
  --gpu-memory-utilization 0.95
```

**Memory savings**:
```
Llama 2 70B fp16: 140GB VRAM (4x A100 needed)
Llama 2 70B AWQ: 35GB VRAM (1x A100 40GB)
= 4x memory reduction
```

**Step 3: Verify performance**

Test that outputs are acceptable:
```python
from openai import OpenAI

client = OpenAI(base_url="http://localhost:8000/v1", api_key="EMPTY")

# Test complex reasoning
response = client.chat.completions.create(
    model="TheBloke/Llama-2-70B-AWQ",
    messages=[{"role": "user", "content": "Explain quantum entanglement"}]
)

print(response.choices[0].message.content)
# Verify quality matches your requirements
```

**Quantize your own model** (requires GPU with 80GB+ VRAM):

```python
from awq import AutoAWQForCausalLM
from transformers import AutoTokenizer

model_path = "meta-llama/Llama-2-70b-hf"
quant_path = "llama-2-70b-awq"

# Load model
model = AutoAWQForCausalLM.from_pretrained(model_path)
tokenizer = AutoTokenizer.from_pretrained(model_path)

# Quantize
quant_config = {"zero_point": True, "q_group_size": 128, "w_bit": 4}
model.quantize(tokenizer, quant_config=quant_config)

# Save
model.save_quantized(quant_path)
tokenizer.save_pretrained(quant_path)
```

## GPTQ setup and usage

**GPTQ** has widest model support and good compression.

**Step 1: Find GPTQ model**

```bash
# Example: TheBloke/Llama-2-13B-GPTQ
# Example: TheBloke/CodeLlama-34B-GPTQ
```

**Step 2: Launch with GPTQ**

```bash
vllm serve TheBloke/Llama-2-13B-GPTQ \
  --quantization gptq \
  --dtype float16
```

**GPTQ configuration options**:
```bash
# Specify GPTQ parameters if needed
vllm serve MODEL \
  --quantization gptq \
  --gptq-act-order \  # Activation ordering
  --dtype float16
```

**Quantize your own model**:

```python
from auto_gptq import AutoGPTQForCausalLM, BaseQuantizeConfig
from transformers import AutoTokenizer

model_name = "meta-llama/Llama-2-13b-hf"
quantized_name = "llama-2-13b-gptq"

# Load model
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoGPTQForCausalLM.from_pretrained(model_name, quantize_config)

# Prepare calibration data
calib_data = [...]  # List of sample texts

# Quantize
quantize_config = BaseQuantizeConfig(
    bits=4,
    group_size=128,
    desc_act=True
)
model.quantize(calib_data)

# Save
model.save_quantized(quantized_name)
```

## FP8 quantization (H100 / Intel Arc)

**FP8** (8-bit floating point) offers best speed with minimal accuracy loss. Works on both NVIDIA H100 and Intel Arc GPUs.

**Requirements**:
- NVIDIA: H100 or H800 GPU, CUDA 12.3+
- Intel: Arc B60/A770 via llm-scaler-vllm Docker

**NVIDIA — Enable FP8:**
```bash
vllm serve meta-llama/Llama-3-70B-Instruct \
  --quantization fp8 \
  --tensor-parallel-size 2
```

**Intel Arc — Enable FP8:**
```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve /llm/models/DeepSeek-R1-Distill-Qwen-7B \
  --quantization fp8 \
  --dtype float16 --enforce-eager \
  --block-size 64 --disable-sliding-window \
  -tp 1
```

For large models that OOM during quantization:
```bash
export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1
```

**Performance gains**:
```
H100: fp16 → FP8 = ~1.8x speedup
Intel Arc: fp16 → FP8 = significant memory reduction + speed gain
```

**Dynamic FP8 quantization** (no pre-quantized model needed):
```bash
# vLLM quantizes at runtime — no model preparation required
vllm serve MODEL --quantization fp8
```

## INT4 quantization (Intel Arc)

**sym_int4** provides dynamic symmetric INT4 quantization at runtime on Intel Arc GPUs.

```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve /llm/models/DeepSeek-R1-Distill-Qwen-7B \
  --quantization sym_int4 \
  --dtype float16 --enforce-eager \
  --block-size 64 --disable-sliding-window \
  -tp 1
```

For Qwen3.5/3.6 models, set the shared library path:
```bash
export VLLM_QUANTIZE_Q40_LIB="/usr/local/lib/python3.12/dist-packages/vllm_int4_for_multi_arc.so"
```

## MXFP4 quantization (Intel Arc)

**mxfp4** (Microscaling FP4) is only available for `gpt-oss-20b` and `gpt-oss-120b` models:

```bash
vllm serve /llm/models/gpt-oss-20b --quantization mxfp4
```

## Model preparation

**Pre-quantized models (easiest)**:

1. Search HuggingFace: `[model name] AWQ` or `[model name] GPTQ`
2. Download or use directly: `TheBloke/[Model]-AWQ`
3. Launch with appropriate `--quantization` flag

**Quantize your own model**:

**AWQ**:
```bash
# Install AutoAWQ
pip install autoawq

# Run quantization script
python quantize_awq.py --model MODEL --output OUTPUT
```

**GPTQ**:
```bash
# Install AutoGPTQ
pip install auto-gptq

# Run quantization script
python quantize_gptq.py --model MODEL --output OUTPUT
```

**Calibration data**:
- Use 128-512 diverse examples from target domain
- Representative of production inputs
- Higher quality calibration = better accuracy

## Accuracy vs compression trade-offs

**Empirical results** (Llama 2 70B on MMLU benchmark):

| Quantization | Accuracy | Memory | Speed | Production-Ready |
|--------------|----------|--------|-------|------------------|
| FP16 (baseline) | 100% | 140GB | 1.0x | ✅ (if memory available) |
| FP8 | 99.5% | 70GB | 1.8x | ✅ (H100 only) |
| AWQ 4-bit | 99.0% | 35GB | 1.5x | ✅ (best for 70B) |
| GPTQ 4-bit | 98.5% | 35GB | 1.5x | ✅ (good compatibility) |
| SqueezeLLM 3-bit | 96.0% | 26GB | 1.3x | ⚠️ (check accuracy) |

**When to use each**:

**No quantization (FP16)**:
- Have sufficient GPU memory
- Need absolute best accuracy
- Model <13B parameters

**FP8**:
- Using H100/H800 GPUs
- Need best speed with minimal accuracy loss
- Production deployment

**AWQ 4-bit**:
- Need to fit 70B model in 40GB GPU
- Production deployment
- <1% accuracy loss acceptable

**GPTQ 4-bit**:
- Wide model support needed
- Not on H100 (use FP8 instead)
- 1-2% accuracy loss acceptable

**Testing strategy**:

1. **Baseline**: Measure FP16 accuracy on your evaluation set
2. **Quantize**: Create quantized version
3. **Evaluate**: Compare quantized vs baseline on same tasks
4. **Decide**: Accept if degradation < threshold (typically 1-2%)

**Example evaluation**:
```python
from evaluate import load_evaluation_suite

# Run on FP16 baseline
baseline_score = evaluate(model_fp16, eval_suite)

# Run on quantized
quant_score = evaluate(model_awq, eval_suite)

# Compare
degradation = (baseline_score - quant_score) / baseline_score * 100
print(f"Accuracy degradation: {degradation:.2f}%")

# Decision
if degradation < 1.0:
    print("✅ Quantization acceptable for production")
else:
    print("⚠️ Review accuracy loss")
```
