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

COPY convert_models.py .
RUN for i in 1 2 3; do \
    timeout 300 python convert_models.py \
    && break \
    || { echo "Attempt $i failed, retrying in 15s..."; sleep 15; }; \
    done

COPY main.py ocr_engine.py ./

FROM python:3.12-slim

WORKDIR /app

# Install system deps, Intel compute runtime (OpenCL for all Intel GPUs: HD 530+, UHD,
# Iris Xe, Arc) from Debian non-free repos, and create non-root user with GPU access
RUN echo "deb http://deb.debian.org/debian bookworm non-free non-free-firmware" \
    >> /etc/apt/sources.list.d/non-free.list \
    && apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    libgomp1 \
    libsm6 \
    libxext6 \
    libxrender1 \
    curl \
    intel-opencl-icd \
    ocl-icd-libopencl1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r ocruser && useradd -r -g ocruser -d /app -s /sbin/nologin ocruser \
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
