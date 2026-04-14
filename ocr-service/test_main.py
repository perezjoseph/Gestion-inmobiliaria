"""Tests for OCR sidecar FastAPI endpoints.

PaddleOCR is mocked via conftest.py (module-level patch) so tests
run without real model files.
"""
import io
from unittest.mock import MagicMock

import pytest
from PIL import Image
from starlette.testclient import TestClient

from main import app, ocr_engine


@pytest.fixture()
def client():
    return TestClient(app)


def _make_jpeg(width: int = 1, height: int = 1) -> bytes:
    buf = io.BytesIO()
    Image.new("RGB", (width, height), color="white").save(buf, format="JPEG")
    return buf.getvalue()


def test_health_returns_200(client):
    resp = client.get("/health")
    assert resp.status_code == 200
    assert resp.json() == {"status": "ok"}


def test_unsupported_format_returns_422(client):
    resp = client.post(
        "/ocr/extract",
        files={"image": ("notes.txt", b"plain text", "text/plain")},
    )
    assert resp.status_code == 422
    assert "Formato no soportado" in resp.json()["detail"]


def test_oversized_file_returns_413(client):
    oversized = b"\xff\xd8\xff\xe0" + b"\x00" * (10 * 1024 * 1024 + 1)
    resp = client.post(
        "/ocr/extract",
        files={"image": ("big.jpg", oversized, "image/jpeg")},
    )
    assert resp.status_code == 413
    assert "tamaño máximo" in resp.json()["detail"]


def test_valid_image_returns_200(client):
    sample_ocr_result = [
        [
            (
                [[10, 20], [300, 20], [300, 50], [10, 50]],
                ("BANCO POPULAR DOMINICANO", 0.97),
            ),
            (
                [[10, 60], [200, 60], [200, 90], [10, 90]],
                ("DEPOSITO AHORROS", 0.95),
            ),
        ]
    ]
    ocr_engine.ocr = MagicMock(return_value=sample_ocr_result)

    resp = client.post(
        "/ocr/extract",
        files={"image": ("receipt.jpg", _make_jpeg(), "image/jpeg")},
    )
    assert resp.status_code == 200

    body = resp.json()
    assert "document_type" in body
    assert "lines" in body
    assert "structured_fields" in body
    assert isinstance(body["lines"], list)
    assert len(body["lines"]) == 2
    assert body["lines"][0]["text"] == "BANCO POPULAR DOMINICANO"
    assert body["document_type"] == "deposito_bancario"
