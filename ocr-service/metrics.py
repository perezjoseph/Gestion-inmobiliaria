from prometheus_client import Counter, Histogram

ocr_documents_total = Counter(
    "ocr_documents_total",
    "OCR documents processed by classified type",
    ["doc_type"],
)

ocr_duration_seconds = Histogram(
    "ocr_duration_seconds",
    "End-to-end OCR extract request duration in seconds",
)

ocr_inference_seconds = Histogram(
    "ocr_inference_seconds",
    "OpenVINO inference duration in seconds by pipeline stage",
    ["stage"],
)
