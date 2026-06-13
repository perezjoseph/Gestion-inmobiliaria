# Intel Arc GPU Deployment (llm-scaler-vllm)

## Contents
- Docker container setup
- Bare metal environment
- Launching the serving service
- Quantization options (FP8 / INT4 / MXFP4)
- Data parallelism
- Embeddings and rerankers
- Multi-modal models
- Multi-node distributed deployment (Docker Swarm + Ray)
- Load balancer solution
- Supported models
- Performance tuning
- Troubleshooting

## Docker container setup

llm-scaler-vllm is Intel's optimized vLLM fork for Arc B60/A770 GPUs. It ships as a Docker image on Docker Hub.

**Pull the image** — use exact beta version, never `latest`:
```bash
# Find latest version at:
# https://github.com/intel/llm-scaler/blob/main/Releases.md/#latest-beta-release
docker pull intel/llm-scaler-vllm:<VERSION>

# Stable PV release: intel/llm-scaler-vllm:1.0
# For Arc A770: intelanalytics/multi-arc-serving:latest
```

**Run the container:**
```bash
sudo docker run -td \
    --privileged \
    --net=host \
    --device=/dev/dri \
    --name=lsv-container \
    -v /home/intel/LLM:/llm/models/ \
    -e no_proxy=localhost,127.0.0.1 \
    -e http_proxy=$http_proxy \
    -e https_proxy=$https_proxy \
    --shm-size="32g" \
    --entrypoint /bin/bash \
    intel/llm-scaler-vllm:<VERSION>

docker exec -it lsv-container bash
```

**Device permissions** (for non-sudo Docker):
```bash
sudo usermod -aG docker $USER
sudo usermod -aG render $USER
sudo usermod -aG video $USER
# Log out and back in
```

**Map a single GPU** (replace `--privileged --device=/dev/dri` with):
```bash
--device /dev/dri/renderD128:/dev/dri/renderD128 \
--mount type=bind,source="/dev/dri/by-path/pci-0000:18:00.0-card",target="/dev/dri/by-path/pci-0000:18:00.0-card" \
--mount type=bind,source="/dev/dri/by-path/pci-0000:18:00.0-render",target="/dev/dri/by-path/pci-0000:18:00.0-render" \
-v /dev/dri/card0:/dev/dri/card0
```

**oneAPI environment note:**
- Interactive shell (`docker exec -it bash`): `.bashrc` sources oneAPI automatically.
- Docker Compose / non-interactive: prepend `source /opt/intel/oneapi/setvars.sh --force &&` to your command.

---

## Bare metal environment

Required: Ubuntu 24.04 (fresh install, minor version 3 or 4).

**Install the offline installer:**
```bash
# Download from Intel RDC (no registration):
# https://cdrdv2.intel.com/v1/dl/getContent/919991/919992?filename=multi-arc-bmg-offline-installer-26.18.8.2-combo.tar.xz

sudo su -
cd the_path_of_multi-arc-bmg-offline-installer-x.x.x.x
./installer.sh
# Reboot after completion
```

**Post-reboot platform evaluation:**
```bash
cd /opt/intel/multi-arc
scripts/evaluation/platform_basic_evaluation.sh
# Results in results/, reference perf in results/reference_perf_b60.csv
```

**Collect system info for debugging:**
```bash
scripts/debug/collect_sysinfo.sh
```

---

## Launching the serving service

**Standard launch (local model):**
```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve \
    --model /llm/models/DeepSeek-R1-Distill-Qwen-7B \
    --served-model-name DeepSeek-R1-Distill-Qwen-7B \
    --dtype=float16 \
    --enforce-eager \
    --port 8000 \
    --host 0.0.0.0 \
    --trust-remote-code \
    --disable-sliding-window \
    --gpu-memory-util=0.9 \
    --max-num-batched-tokens=8192 \
    --disable-log-requests \
    --max-model-len=8192 \
    --block-size 64 \
    --quantization fp8 \
    -tp=1 \
    2>&1 | tee /llm/vllm.log > /proc/1/fd/1 &

tail -f /llm/vllm.log
```

**Launch from HuggingFace (if model not pre-downloaded):**
```bash
HF_TOKEN="<your_api_token>"
HF_HOME="/llm/models"
vllm serve deepseek-ai/DeepSeek-R1-Distill-Qwen-7B \
    --served-model-name DeepSeek-R1-Distill-Qwen-7B
```

**Key notes:**
- Prefix caching is enabled by default. Disable with `--no-enable-prefix-caching`.
- Add `--api-key xxx` for authentication.
- For tool calling: `--enable-auto-tool-choice --tool-call-parser qwen3_coder`

**Benchmarking:**
```bash
vllm bench serve \
    --model /llm/models/DeepSeek-R1-Distill-Qwen-7B \
    --dataset-name random \
    --served-model-name DeepSeek-R1-Distill-Qwen-7B \
    --random-input-len=1024 \
    --random-output-len=512 \
    --ignore-eos \
    --num-prompt 10 \
    --trust_remote_code \
    --request-rate inf \
    --backend vllm \
    --port=8000
```

---

## Quantization options

| Method | `--quantization` | Description | Applicable |
|--------|-----------------|-------------|------------|
| Online FP8 | `fp8` | Dynamic FP8 at runtime | All models |
| Online INT4 | `sym_int4` | Dynamic symmetric INT4 | All models |
| MXFP4 | `mxfp4` | Microscaling FP4 | gpt-oss-20b/120b only |
| Pre-quantized AWQ | (auto-detected) | From model config | AWQ models |
| Pre-quantized GPTQ | (auto-detected) | From model config | GPTQ models |

**INT4 example:**
```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve \
    --model /llm/models/DeepSeek-R1-Distill-Qwen-7B \
    --dtype=float16 --enforce-eager \
    --port 8000 --host 0.0.0.0 \
    --trust-remote-code --disable-sliding-window \
    --gpu-memory-util=0.9 --no-enable-prefix-caching \
    --max-num-batched-tokens=8192 --max-model-len=8192 \
    --block-size 64 --quantization sym_int4 -tp=1
```

**OOM during quantization:** `export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1`

**INT4 with shared library (for Qwen3.5/3.6 models):**
```bash
export VLLM_QUANTIZE_Q40_LIB="/usr/local/lib/python3.12/dist-packages/vllm_int4_for_multi_arc.so"
```

---

## Data parallelism

Near-linear scaling on Intel XPU:

| DP | Batch | Throughput |
|----|-------|-----------|
| 1 | 10 | 1x |
| 2 | 20 | 1.9x |
| 4 | 40 | 3.58x |

Enable: `--dp 2`

Alternative: load balancer solution (Section below) provides slightly better performance and supports periodic rotation for long-running services.

---

## Embeddings and rerankers

**Embedding service (bge-m3):**
```bash
VLLM_ALLOW_LONG_MAX_MODEL_LEN=1 \
VLLM_WORKER_MULTIPROC_METHOD=spawn \
vllm serve \
    --model /llm/models/bge-m3 \
    --served-model-name bge-m3 \
    --task embed \
    --dtype=float16 --enforce-eager \
    --port 8000 --host 0.0.0.0 \
    --trust-remote-code --disable-sliding-window \
    --gpu-memory-util=0.9 --no-enable-prefix-caching \
    --max-num-batched-tokens=2048 --max-model-len=2048 \
    --block-size 64 -tp=1
```

Query: `POST /v1/embeddings` with `{"input": ["text"], "model": "bge-m3", "encoding_format": "float"}`

**Reranker service (bge-reranker-base):**
```bash
vllm serve /llm/models/bge-reranker-base \
    --served-model-name bge-reranker-base \
    --task score \
    # ... same Intel flags as above
```

Query: `POST /v1/rerank` with `{"model": "bge-reranker-base", "query": "...", "documents": [...]}`

---

## Multi-modal models

**Vision (Qwen2.5-VL):**
```bash
vllm serve /llm/models/Qwen2.5-VL-7B-Instruct \
    --served-model-name Qwen2.5-VL-7B-Instruct \
    --allowed-local-media-path /llm/models/test \
    --dtype=float16 --enforce-eager \
    --port 8000 --host 0.0.0.0 --trust-remote-code \
    --gpu-memory-util=0.9 --no-enable-prefix-caching \
    --max-num-batched-tokens=5120 --max-model-len=5120 \
    --block-size 64 --quantization fp8 -tp=1
```

Image query uses standard OpenAI chat completions with `image_url` content type.
Local images: `"url": "file:/llm/models/test/1.jpg"`

**Omni (audio + vision — Qwen2.5-Omni-7B):**
```bash
pip install librosa soundfile  # Audio dependencies
vllm serve /llm/models/Qwen2.5-Omni-7B \
    --served-model-name Qwen2.5-Omni-7B \
    # ... same vision flags
```

**Audio transcription (whisper) — requires V0 engine:**
```bash
pip install transformers==4.52.4 librosa
VLLM_USE_V1=0 python3 -m vllm.entrypoints.openai.api_server \
    --model /llm/models/whisper-medium \
    --served-model-name whisper-medium \
    --allowed-local-media-path /llm/models/test \
    # ... same Intel flags, but --block-size 16
```

Query: `POST /v1/audio/transcriptions` (multipart form with audio file).

---

## Multi-node distributed deployment (Docker Swarm + Ray)

For models that exceed single-machine memory.

**1. Create Docker Swarm overlay:**
```bash
# Node-1 (head):
docker swarm init --advertise-addr <IP_A>
# Node-2:
docker swarm join --token <token> <IP_A>:2377
# Create network:
docker network create --driver overlay --attachable my-overlay
```

**2. Start containers on overlay network** (both nodes):
```bash
sudo docker run -td --privileged --network=my-overlay --device=/dev/dri \
    --name=node-1 -v /model_path:/llm/models/ --shm-size="32g" \
    --entrypoint /bin/bash intel/llm-scaler-vllm:<VERSION>
```

**3. Configure SSH** between containers (generate keys, `ssh-copy-id`).

**4. Start Ray cluster:**
```bash
# Node-1 (head):
export VLLM_HOST_IP=10.0.1.19
ray start --block --head --port=6379 --num-gpus=1 --node-ip-address=10.0.1.19

# Node-2 (worker):
export VLLM_HOST_IP=10.0.1.20
ray start --block --address=10.0.1.19:6379 --num-gpus=1
```

**5. Launch vLLM with Ray backend:**
```bash
export VLLM_HOST_IP=10.0.1.19
export CCL_ATL_TRANSPORT=ofi

vllm serve /llm/models/Qwen2.5-7B-Instruct \
    --dtype=float16 --enforce-eager \
    --port 8005 --host 0.0.0.0 --trust-remote-code \
    --disable-sliding-window --gpu-memory-util=0.9 \
    --no-enable-prefix-caching --max-num-batched-tokens=8192 \
    --max-model-len=20000 --block-size 64 \
    --served-model-name test \
    -tp=2 -pp=1 \
    --distributed-executor-backend ray
```

---

## Load balancer solution

Routes traffic to multiple vLLM instances with a single endpoint at `http://localhost:8000`.

**Drop-in DP alternative:**
```bash
cd vllm/docker-compose/load_balancer
docker compose up -d
docker compose logs -f
# Stop: docker compose down
```

**With periodic rotation (prevents degradation in long-running services):**
```bash
cd vllm/docker-compose/load_balancer
chmod +x vllm_bootstrap_and_rotate.sh
bash vllm_bootstrap_and_rotate.sh
# Stop: docker compose down && crontab -l | grep -v "vllm_bootstrap_and_rotate" | crontab -
```

---

## Supported models

Key models with quantization support on Intel Arc:

| Model | FP16 | FP8 | INT4 | Notes |
|-------|------|-----|------|-------|
| DeepSeek-R1-Distill-Qwen-1.5B/7B/14B/32B | ✅ | ✅ | ✅ | |
| DeepSeek-R1-Distill-Llama-8B/70B | ✅ | ✅ | ✅ | |
| DeepSeek-R1-0528-Qwen3-8B | ✅ | ✅ | ✅ | |
| DeepSeek-V2-Lite | ✅ | ✅ | | `VLLM_MLA_DISABLE=1` |
| Qwen3-8B/14B/32B | ✅ | ✅ | ✅ | |
| Qwen3-30B-A3B / Qwen3-Next-80B-A3B | ✅ | ✅ | ✅/— | |
| Qwen3-235B-A22B | | ✅ | | |
| Qwen3-Coder-30B-A3B-Instruct | ✅ | ✅ | ✅ | |
| Qwen3-Coder-Next | ✅ | ✅ | | |
| Qwen3.5/3.6-27B | ✅ | ✅ | ✅ | Use `0.14.0-b8.3.1` image |
| Qwen3.5/3.6-35B-A3B | ✅ | ✅ | ✅ | Use `0.14.0-b8.3.1` image |
| Qwen3.5-122B-A10B | | ✅ | ✅ | |
| QwQ-32B | ✅ | ✅ | ✅ | |
| Llama-3.1-8B/70B | ✅ | ✅ | ✅ | |
| Mixtral-8x7B-Instruct | ✅ | ✅ | ✅ | |
| Qwen2.5-VL-7B/32B/72B | ✅ | ✅ | ✅ | Multi-modal |
| Qwen3-VL-4B/8B/30B-A3B | ✅ | ✅ | ✅ | Multi-modal |
| Qwen2.5-Omni-7B / Qwen3-Omni-30B-A3B | ✅ | ✅ | ✅ | Audio+Vision |
| whisper-medium/large-v3 | ✅ | ✅ | ✅ | Transcription |
| BAAI/bge-m3, bge-large-en-v1.5 | ✅ | ✅ | ✅ | Embeddings |
| BAAI/bge-reranker-large/v2-m3 | ✅ | ✅ | ✅ | Reranker |
| Qwen3-Embedding-8B / Qwen3-Reranker-8B | ✅ | ✅ | ✅ | |
| GLM-4-9B/32B-0414 | | ✅ | | Use bfloat16 |
| tencent/Hunyuan-0.5B/7B-Instruct | ✅ | ✅ | ✅ | `pip install transformers==4.56.1` |

Full list: https://github.com/intel/llm-scaler/blob/main/vllm/README.md#3-supported-models

---

## Performance tuning

**CCL communication mode (multi-GPU):**
```bash
export CCL_TOPO_P2P_ACCESS=1  # P2P (~15% better for large batches)
export CCL_TOPO_P2P_ACCESS=0  # USM (fallback)
```

**BPE-Qwen tokenizer (faster for Qwen models):**
```bash
pip install bpe-qwen
vllm serve MODEL --tokenizer-mode bpe-qwen
```

**CPU affinity / NUMA binding:**
```bash
lscpu | grep NUMA  # Find cores for your GPU's NUMA node
numactl -C 0-17 vllm serve ...
```

**Finding maximum context length:**
The V1 engine logs max supported context on startup:
```
INFO GPU KV cache size: 114,432 tokens
```
If requested length exceeds this, vLLM errors with the maximum value — set `--max-model-len` accordingly.

**GPU affinity (multi-GPU):**
```bash
export ZE_AFFINITY_MASK=0,1  # Use first two Arc GPUs
```

---

## Troubleshooting

**`ModuleNotFoundError: No module named 'vllm._C'`**
Run from `/llm` root, not `/llm/vllm`:
```bash
cd /llm && python3 -m vllm.entrypoints.openai.api_server
```

**OOM during online quantization:**
```bash
export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1
```

**Service won't start with large context:**
Check the error for maximum supported value and set `--max-model-len` to that.

**oneAPI not loaded (Docker Compose / non-interactive):**
```yaml
entrypoint: source /opt/intel/oneapi/setvars.sh --force && vllm serve ...
```

**Network issues downloading models:**
Use [hf-mirror](https://hf-mirror.com/) or pre-download and volume-mount.

---

## Reference commands for Qwen3.5/3.6 models

Recommended image: `intel/llm-scaler-vllm:0.14.0-b8.3.1`

```bash
export VLLM_ALLOW_LONG_MAX_MODEL_LEN=1
export VLLM_WORKER_MULTIPROC_METHOD=spawn
export VLLM_OFFLOAD_WEIGHTS_BEFORE_QUANT=1
export ZE_AFFINITY_MASK=0,1

vllm serve --port 8000 --host 0.0.0.0 \
    --gpu-memory-util 0.9 --max-num-batched-tokens 8192 \
    --max-model-len 40000 --block-size 64 \
    --dtype float16 --model /llm/models/Qwen3.6-35B-A3B/ \
    --served-model-name Qwen3.6-35B-A3B \
    --tensor-parallel-size 2 --quantization fp8 \
    --enforce-eager --trust-remote-code --disable-log-requests
```

With prefix caching and tool calling:
```bash
--enable-prefix-caching --enable-auto-tool-choice --tool-call-parser qwen3_coder
```

Source: https://github.com/intel/llm-scaler/blob/main/vllm/README.md
