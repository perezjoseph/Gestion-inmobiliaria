FROM python:3.11-alpine AS builder

WORKDIR /app

RUN apk add --no-cache \
    mesa-gl \
    glib \
    libgomp \
    libsm \
    libxext \
    libxrender \
    # Build deps for native Python packages
    gcc \
    g++ \
    musl-dev \
    linux-headers \
    mesa-dev \
    glib-dev \
    libffi-dev \
    openblas-dev

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr
# Use Baidu Object Storage as download source (better availability outside China)
ENV PADDLE_PDX_MODEL_SOURCE=BOS

RUN for i in 1 2 3 4 5; do \
    python -c "from paddleocr import PaddleOCR; PaddleOCR(use_textline_orientation=True, lang='es', device='cpu')" \
    && break \
    || { echo "Attempt $i failed, retrying in 15s..."; sleep 15; }; \
    done

FROM python:3.11-alpine

WORKDIR /app

RUN apk add --no-cache \
    mesa-gl \
    glib \
    libgomp \
    libsm \
    libxext \
    libxrender \
    libstdc++ \
    openblas \
    curl \
    && addgroup -S ocruser && adduser -S -G ocruser -h /app -s /sbin/nologin ocruser

COPY --from=builder /usr/local/lib/python3.11/site-packages /usr/local/lib/python3.11/site-packages
COPY --from=builder /usr/local/bin /usr/local/bin
COPY --from=builder /app/main.py .
COPY --from=builder /app/.paddleocr /app/.paddleocr

ENV PADDLE_PDX_CACHE_HOME=/app/.paddleocr

RUN chown -R ocruser:ocruser /app

USER ocruser

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
