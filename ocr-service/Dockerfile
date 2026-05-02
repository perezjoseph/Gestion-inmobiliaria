FROM python:3.12-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    libgomp1 \
    libsm6 \
    libxext6 \
    libxrender1 \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Download PaddleOCR models and convert to ONNX for OpenVINO
RUN pip install --no-cache-dir paddleocr==3.5.0 paddlepaddle==3.2.2

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr
ENV PADDLE_PDX_MODEL_SOURCE=BOS

COPY convert_models.py .
RUN for i in 1 2 3; do \
    timeout 300 python convert_models.py \
    && break \
    || { echo "Attempt $i failed, retrying in 15s..."; sleep 15; }; \
    done \
    && ls -la /app/.paddleocr/official_models/

COPY main.py ocr_engine.py ./

FROM python:3.12-slim

WORKDIR /app

# Install system deps, Intel compute runtime (OpenCL for all Intel GPUs: HD 530+, UHD,
# Iris Xe, Arc) from GitHub releases, and create non-root user with GPU access.
# Uses legacy1 packages for Skylake/Kaby Lake support (Gen9).
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    libgomp1 \
    libsm6 \
    libxext6 \
    libxrender1 \
    curl \
    ocl-icd-libopencl1 \
    && mkdir -p /tmp/gpu \
    && curl -fsSL -o /tmp/gpu/intel-igc-core.deb \
    https://github.com/intel/intel-graphics-compiler/releases/download/igc-1.0.17537.20/intel-igc-core_1.0.17537.20_amd64.deb \
    && curl -fsSL -o /tmp/gpu/intel-igc-opencl.deb \
    https://github.com/intel/intel-graphics-compiler/releases/download/igc-1.0.17537.20/intel-igc-opencl_1.0.17537.20_amd64.deb \
    && curl -fsSL -o /tmp/gpu/libigdgmm12.deb \
    https://github.com/intel/compute-runtime/releases/download/24.35.30872.22/libigdgmm12_22.5.0_amd64.deb \
    && curl -fsSL -o /tmp/gpu/intel-opencl-icd.deb \
    https://github.com/intel/compute-runtime/releases/download/24.35.30872.22/intel-opencl-icd_24.35.30872.22_amd64.deb \
    && curl -fsSL -o /tmp/gpu/intel-opencl-icd-legacy1.deb \
    https://github.com/intel/compute-runtime/releases/download/24.35.30872.22/intel-opencl-icd-legacy1_24.35.30872.22_amd64.deb \
    && dpkg -i /tmp/gpu/*.deb || apt-get install -f -y --no-install-recommends \
    && rm -rf /tmp/gpu /var/lib/apt/lists/* \
    && groupadd -r ocruser \
    && groupadd -r render || true \
    && useradd -r -g ocruser -d /app -s /sbin/nologin ocruser \
    && usermod -aG video ocruser \
    && usermod -aG render ocruser

COPY --from=builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages
COPY --from=builder /usr/local/bin /usr/local/bin
COPY --from=builder /app/main.py /app/ocr_engine.py ./
COPY --from=builder /app/.paddleocr /app/.paddleocr

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr
ENV OPENVINO_DEVICE=GPU

RUN chown -R ocruser:ocruser /app

USER ocruser

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
