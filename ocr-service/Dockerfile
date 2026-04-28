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

COPY main.py .

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr
# Use Baidu Object Storage as download source (better availability outside China)
ENV PADDLE_PDX_MODEL_SOURCE=BOS

RUN for i in 1 2 3; do \
    timeout 300 python -c \
      "from paddleocr import PaddleOCR; PaddleOCR(use_textline_orientation=True, use_doc_orientation_classify=False, use_doc_unwarping=False, lang='es', device='cpu')" \
    && break \
    || { echo "Attempt $i failed, retrying in 15s..."; sleep 15; }; \
    done

FROM python:3.12-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    libgomp1 \
    libsm6 \
    libxext6 \
    libxrender1 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r ocruser && useradd -r -g ocruser -d /app -s /sbin/nologin ocruser

COPY --from=builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages
COPY --from=builder /usr/local/bin /usr/local/bin
COPY --from=builder /app/main.py .
COPY --from=builder /app/.paddleocr /app/.paddleocr

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr

RUN chown -R ocruser:ocruser /app

USER ocruser

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
