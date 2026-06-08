FROM python:3.12-slim AS builder

WORKDIR /app

# Upgrade base packages to pick up latest Debian security patches before
# installing our own dependencies.
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    libgomp1 \
    libsm6 \
    libxext6 \
    libxrender1 \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
# Upgrade pip before installing to pick up CVE fixes (CVE-2025-8869, CVE-2026-6357, CVE-2026-3219)
RUN pip install --no-cache-dir --upgrade pip \
    && pip install --no-cache-dir -r requirements.txt

# Download PaddleOCR models and convert to OpenVINO IR format
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
# compute-runtime 26.09+ bundles legacy platform support (Gen9) in the main package.
# apt-get upgrade pulls the latest Debian security patches for the base image
# (libgnutls30t64, libssh2-1t64, libcurl4t64, libxml2, libc6, krb5, util-linux,
# sed, etc.) so Trivy image scans don't flag the base OS packages.
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends \
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
    https://github.com/intel/intel-graphics-compiler/releases/download/v2.30.1/intel-igc-core-2_2.30.1+20950_amd64.deb \
    && curl -fsSL -o /tmp/gpu/intel-igc-opencl.deb \
    https://github.com/intel/intel-graphics-compiler/releases/download/v2.30.1/intel-igc-opencl-2_2.30.1+20950_amd64.deb \
    && curl -fsSL -o /tmp/gpu/libigdgmm12.deb \
    https://github.com/intel/compute-runtime/releases/download/26.09.37435.1/libigdgmm12_22.9.0_amd64.deb \
    && curl -fsSL -o /tmp/gpu/intel-opencl-icd.deb \
    https://github.com/intel/compute-runtime/releases/download/26.09.37435.1/intel-opencl-icd_26.09.37435.1-0_amd64.deb \
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
