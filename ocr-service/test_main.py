"""Tests for OCR sidecar FastAPI endpoints.

PaddleOCR is mocked via conftest.py (module-level patch) so tests
run without real model files.
"""
import io
from unittest.mock import MagicMock

import pytest
from PIL import Image
from starlette.testclient import TestClient

from main import app, ocr_engine, _classify_document, _extract_structured_fields


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


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _lines(*texts, confidence=0.95):
    return [{"text": t, "confidence": confidence, "bbox": [0, 0, 100, 0, 100, 30, 0, 30]} for t in texts]


# ---------------------------------------------------------------------------
# _classify_document tests
# ---------------------------------------------------------------------------

class TestClassifyDocument:
    def test_cedula_keywords(self):
        assert _classify_document(_lines("CEDULA DE IDENTIDAD")) == "cedula"
        assert _classify_document(_lines("JUNTA CENTRAL ELECTORAL")) == "cedula"
        assert _classify_document(_lines("REPUBLICA DOMINICANA")) == "cedula"

    def test_contrato_keywords(self):
        assert _classify_document(_lines("CONTRATO DE ARRENDAMIENTO")) == "contrato"
        assert _classify_document(_lines("ALQUILER MENSUAL")) == "contrato"

    def test_cedula_priority_over_contrato(self):
        lines = _lines("CEDULA DE IDENTIDAD", "CONTRATO DE ARRENDAMIENTO")
        assert _classify_document(lines) == "cedula"

    def test_cedula_priority_over_deposito(self):
        lines = _lines("REPUBLICA DOMINICANA", "BANCO POPULAR", "DEPOSITO AHORROS")
        assert _classify_document(lines) == "cedula"

    def test_contrato_priority_over_gasto(self):
        lines = _lines("CONTRATO DE ALQUILER", "FACTURA", "RECIBO")
        assert _classify_document(lines) == "contrato"

    def test_no_keywords_returns_unknown(self):
        assert _classify_document(_lines("HELLO WORLD", "RANDOM TEXT")) == "unknown"

    def test_empty_lines_returns_unknown(self):
        assert _classify_document([]) == "unknown"


# ---------------------------------------------------------------------------
# _extract_structured_fields tests — cedula
# ---------------------------------------------------------------------------

class TestExtractCedula:
    def test_extracts_cedula_with_dashes(self):
        lines = _lines("CEDULA DE IDENTIDAD", "001-1234567-8", "JUAN", "PEREZ")
        fields = _extract_structured_fields(lines, "cedula")
        assert fields["cedula"] == "001-1234567-8"

    def test_normalizes_cedula_without_dashes(self):
        lines = _lines("CEDULA DE IDENTIDAD", "00112345678", "MARIA", "GOMEZ")
        fields = _extract_structured_fields(lines, "cedula")
        assert fields["cedula"] == "001-1234567-8"

    def test_extracts_nombre_and_apellido(self):
        lines = _lines("REPUBLICA DOMINICANA", "CEDULA DE IDENTIDAD", "001-1234567-8", "JUAN CARLOS", "PEREZ MARTINEZ")
        fields = _extract_structured_fields(lines, "cedula")
        assert fields["nombre"] == "JUAN CARLOS"
        assert fields["apellido"] == "PEREZ MARTINEZ"

    def test_empty_lines_returns_empty_dict(self):
        fields = _extract_structured_fields([], "cedula")
        assert fields == {}


# ---------------------------------------------------------------------------
# _extract_structured_fields tests — contrato
# ---------------------------------------------------------------------------

class TestExtractContrato:
    def test_extracts_monto_mensual_with_canon_keyword(self):
        lines = _lines("CONTRATO DE ARRENDAMIENTO", "CANON MENSUAL RD$ 25,000.00")
        fields = _extract_structured_fields(lines, "contrato")
        assert "monto_mensual" in fields
        assert "25,000.00" in fields["monto_mensual"]

    def test_extracts_fecha_inicio_with_desde(self):
        lines = _lines("CONTRATO", "DESDE 01/01/2024", "HASTA 31/12/2024")
        fields = _extract_structured_fields(lines, "contrato")
        assert fields["fecha_inicio"] == "01/01/2024"

    def test_extracts_fecha_fin_with_hasta(self):
        lines = _lines("CONTRATO", "DESDE 01/01/2024", "HASTA 31/12/2024")
        fields = _extract_structured_fields(lines, "contrato")
        assert fields["fecha_fin"] == "31/12/2024"

    def test_extracts_deposito_with_keyword(self):
        lines = _lines("CONTRATO", "DEPOSITO DE GARANTIA RD$ 50,000.00")
        fields = _extract_structured_fields(lines, "contrato")
        assert "deposito" in fields
        assert "50,000.00" in fields["deposito"]

    def test_extracts_moneda_rd(self):
        lines = _lines("CONTRATO", "CANON RD$ 15,000.00")
        fields = _extract_structured_fields(lines, "contrato")
        assert fields["moneda"] == "RD$"

    def test_extracts_moneda_usd(self):
        lines = _lines("CONTRATO", "CANON US$ 1,500.00")
        fields = _extract_structured_fields(lines, "contrato")
        assert fields["moneda"] == "US$"

    def test_missing_fields_graceful_degradation(self):
        lines = _lines("CONTRATO DE ARRENDAMIENTO", "PARTES ACUERDAN LO SIGUIENTE")
        fields = _extract_structured_fields(lines, "contrato")
        assert "monto_mensual" not in fields
        assert "fecha_inicio" not in fields
        assert "fecha_fin" not in fields
        assert "deposito" not in fields

    def test_empty_lines_returns_empty_dict(self):
        fields = _extract_structured_fields([], "contrato")
        assert fields == {}


# ---------------------------------------------------------------------------
# /ocr/extract endpoint with document_type parameter
# ---------------------------------------------------------------------------

class TestExtractEndpointDocumentType:
    def test_provided_document_type_skips_classification(self, client):
        sample_ocr_result = [
            [
                ([[0, 0], [100, 0], [100, 30], [0, 30]], ("RANDOM TEXT", 0.90)),
            ]
        ]
        ocr_engine.ocr = MagicMock(return_value=sample_ocr_result)

        resp = client.post(
            "/ocr/extract",
            files={"image": ("doc.jpg", _make_jpeg(), "image/jpeg")},
            data={"document_type": "cedula"},
        )
        assert resp.status_code == 200
        body = resp.json()
        assert body["document_type"] == "cedula"

    def test_contrato_document_type_passthrough(self, client):
        sample_ocr_result = [
            [
                ([[0, 0], [100, 0], [100, 30], [0, 30]], ("CANON MENSUAL RD$ 20,000.00", 0.92)),
            ]
        ]
        ocr_engine.ocr = MagicMock(return_value=sample_ocr_result)

        resp = client.post(
            "/ocr/extract",
            files={"image": ("contract.jpg", _make_jpeg(), "image/jpeg")},
            data={"document_type": "contrato"},
        )
        assert resp.status_code == 200
        body = resp.json()
        assert body["document_type"] == "contrato"
        assert "structured_fields" in body

    def test_omitted_document_type_uses_auto_classification(self, client):
        sample_ocr_result = [
            [
                ([[0, 0], [100, 0], [100, 30], [0, 30]], ("CEDULA DE IDENTIDAD", 0.95)),
                ([[0, 40], [100, 40], [100, 70], [0, 70]], ("001-1234567-8", 0.90)),
            ]
        ]
        ocr_engine.ocr = MagicMock(return_value=sample_ocr_result)

        resp = client.post(
            "/ocr/extract",
            files={"image": ("id.jpg", _make_jpeg(), "image/jpeg")},
        )
        assert resp.status_code == 200
        body = resp.json()
        assert body["document_type"] == "cedula"
