import io
import re

import fitz
import numpy as np
from typing import Optional

from fastapi import FastAPI, File, Form, UploadFile
from fastapi.responses import JSONResponse
from paddleocr import PaddleOCR
from PIL import Image

app = FastAPI(title="OCR Service")

ocr_engine = PaddleOCR(
    use_textline_orientation=True,
    use_doc_orientation_classify=False,
    use_doc_unwarping=False,
    lang="es",
    device="cpu",
)


@app.get("/health")
async def health():
    return {"status": "ok"}


MAX_FILE_SIZE = 10 * 1024 * 1024  # 10 MB
ALLOWED_CONTENT_TYPES = {"image/jpeg", "image/png", "application/pdf"}


def _pdf_first_page_to_array(file_bytes: bytes) -> np.ndarray:
    """Convert the first page of a PDF to a numpy RGB array using PyMuPDF."""
    doc = fitz.open(stream=file_bytes, filetype="pdf")
    page = doc.load_page(0)
    pix = page.get_pixmap(dpi=300)
    img = Image.frombytes("RGB", (pix.width, pix.height), pix.samples)
    doc.close()
    return np.array(img)


def _image_bytes_to_array(file_bytes: bytes) -> np.ndarray:
    """Load JPEG/PNG bytes into a numpy RGB array."""
    img = Image.open(io.BytesIO(file_bytes)).convert("RGB")
    return np.array(img)


def _classify_document(lines: list[dict]) -> str:
    """Classify document type based on keyword heuristics in extracted text."""
    combined = " ".join(line["text"] for line in lines).upper()

    if any(kw in combined for kw in ("CEDULA", "IDENTIDAD", "ELECTORAL", "REPUBLICA DOMINICANA")):
        return "cedula"
    if any(kw in combined for kw in ("CONTRATO", "ARRENDAMIENTO", "ALQUILER")):
        return "contrato"
    if any(kw in combined for kw in ("DEPOSITO", "AHORROS", "BANCO")):
        return "deposito_bancario"
    if any(kw in combined for kw in ("FACTURA", "RECIBO", "COMPROBANTE")):
        return "recibo_gasto"
    return "unknown"


def _extract_structured_fields(lines: list[dict], document_type: str) -> dict:
    """Extract named fields from OCR lines based on document type."""
    if document_type == "unknown" or not lines:
        return {}

    combined = " ".join(line["text"] for line in lines)
    combined_upper = combined.upper()
    line_texts = [line["text"] for line in lines]
    line_texts_upper = [t.upper() for t in line_texts]

    fields: dict = {}

    if document_type == "deposito_bancario":
        # monto
        monto_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', combined)
        if monto_match:
            fields["monto"] = monto_match.group().strip()

        # moneda
        moneda_match = re.search(r'(RD\$|US\$)', combined)
        fields["moneda"] = moneda_match.group(1) if moneda_match else "RD$"

        # fecha
        fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', combined)
        if fecha_match:
            fields["fecha"] = fecha_match.group()

        # depositante
        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("DEPOSITANTE", "NOMBRE", "TITULAR")):
                if ":" in line_texts[i]:
                    after_colon = line_texts[i].split(":", 1)[1].strip()
                    if after_colon:
                        fields["depositante"] = after_colon
                        break
                if i + 1 < len(line_texts):
                    fields["depositante"] = line_texts[i + 1].strip()
                break

        # cuenta
        cuenta_match = re.search(r'\d{3}-\d{6,}-\d', combined)
        if cuenta_match:
            fields["cuenta"] = cuenta_match.group()

        # referencia
        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("REF", "REFERENCIA", "NO.", "NUMERO")):
                if ":" in line_texts[i]:
                    after_colon = line_texts[i].split(":", 1)[1].strip()
                    if after_colon:
                        fields["referencia"] = after_colon
                        break
                ref_match = re.search(r'[A-Za-z0-9-]+$', line_texts[i].strip())
                if ref_match and ref_match.group() != line_texts[i].strip():
                    fields["referencia"] = ref_match.group()
                    break
                if i + 1 < len(line_texts):
                    fields["referencia"] = line_texts[i + 1].strip()
                break

    elif document_type == "recibo_gasto":
        # proveedor
        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("PROVEEDOR", "EMPRESA", "RAZON SOCIAL")):
                if ":" in line_texts[i]:
                    after_colon = line_texts[i].split(":", 1)[1].strip()
                    if after_colon:
                        fields["proveedor"] = after_colon
                        break
                if i + 1 < len(line_texts):
                    fields["proveedor"] = line_texts[i + 1].strip()
                break
        if "proveedor" not in fields and line_texts:
            fields["proveedor"] = line_texts[0].strip()

        # monto
        monto_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', combined)
        if monto_match:
            fields["monto"] = monto_match.group().strip()

        # moneda
        moneda_match = re.search(r'(RD\$|US\$)', combined)
        fields["moneda"] = moneda_match.group(1) if moneda_match else "RD$"

        # fecha
        fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', combined)
        if fecha_match:
            fields["fecha"] = fecha_match.group()

        # numero_factura
        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("FACTURA", "NCF", "COMPROBANTE")):
                ncf_match = re.search(r'[A-Za-z0-9-]+$', line_texts[i].strip())
                if ncf_match and ncf_match.group() != line_texts[i].strip():
                    fields["numero_factura"] = ncf_match.group()
                    break
                if ":" in line_texts[i]:
                    after_colon = line_texts[i].split(":", 1)[1].strip()
                    if after_colon:
                        fields["numero_factura"] = after_colon
                        break
                if i + 1 < len(line_texts):
                    fields["numero_factura"] = line_texts[i + 1].strip()
                break

    elif document_type == "cedula":
        cedula_match = re.search(r'\d{3}-?\d{7}-?\d', combined)
        if cedula_match:
            raw = re.sub(r'[^0-9]', '', cedula_match.group())
            fields["cedula"] = f"{raw[:3]}-{raw[3:10]}-{raw[10]}"

        header_indices = []
        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("CEDULA", "IDENTIDAD", "ELECTORAL", "REPUBLICA DOMINICANA")):
                header_indices.append(i)

        if header_indices:
            start = max(header_indices) + 1
            remaining = [t.strip() for t in line_texts[start:] if t.strip()]
            alpha_lines = [
                t for t in remaining
                if re.search(r'[A-Za-zÁÉÍÓÚáéíóúÑñ]', t) and not re.match(r'^[\d\s.,$/-]+$', t)
            ]
            if len(alpha_lines) >= 2:
                fields["nombre"] = alpha_lines[0]
                fields["apellido"] = alpha_lines[1]
            elif len(alpha_lines) == 1:
                fields["nombre"] = alpha_lines[0]

    elif document_type == "contrato":
        moneda_match = re.search(r'(RD\$|US\$)', combined)
        fields["moneda"] = moneda_match.group(1) if moneda_match else "RD$"

        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("CANON", "RENTA", "MENSUAL", "ALQUILER")):
                monto_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', line_texts[i])
                if monto_match:
                    fields["monto_mensual"] = monto_match.group().strip()
                    break
                for j in range(max(0, i - 1), min(len(line_texts), i + 3)):
                    monto_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', line_texts[j])
                    if monto_match:
                        fields["monto_mensual"] = monto_match.group().strip()
                        break
                if "monto_mensual" in fields:
                    break

        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("DESDE", "INICIO", "VIGENCIA")):
                fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', line_texts[i])
                if fecha_match:
                    fields["fecha_inicio"] = fecha_match.group()
                    break
                for j in range(i, min(len(line_texts), i + 3)):
                    fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', line_texts[j])
                    if fecha_match:
                        fields["fecha_inicio"] = fecha_match.group()
                        break
                if "fecha_inicio" in fields:
                    break

        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("HASTA", "FIN", "VENCIMIENTO")):
                fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', line_texts[i])
                if fecha_match:
                    fields["fecha_fin"] = fecha_match.group()
                    break
                for j in range(i, min(len(line_texts), i + 3)):
                    fecha_match = re.search(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}', line_texts[j])
                    if fecha_match:
                        fields["fecha_fin"] = fecha_match.group()
                        break
                if "fecha_fin" in fields:
                    break

        for i, upper in enumerate(line_texts_upper):
            if any(kw in upper for kw in ("DEPOSITO", "GARANTIA")):
                dep_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', line_texts[i])
                if dep_match:
                    fields["deposito"] = dep_match.group().strip()
                    break
                for j in range(max(0, i - 1), min(len(line_texts), i + 3)):
                    dep_match = re.search(r'(?:RD\$|US\$)?\s*[\d,]+\.\d{2}', line_texts[j])
                    if dep_match:
                        fields["deposito"] = dep_match.group().strip()
                        break
                if "deposito" in fields:
                    break

    return fields


def _run_ocr(img_array: np.ndarray) -> list[dict]:
    """Run PaddleOCR on a numpy image array and return structured line dicts."""
    results = ocr_engine.predict(img_array)

    if not results:
        return []

    res = results[0]
    rec_texts = res["rec_texts"] if isinstance(res, dict) else res.rec_texts
    rec_scores = res["rec_scores"] if isinstance(res, dict) else res.rec_scores
    dt_polys = res["dt_polys"] if isinstance(res, dict) else res.dt_polys

    if not len(rec_texts):
        return []

    lines: list[dict] = []
    for i, text in enumerate(rec_texts):
        poly = dt_polys[i]
        flat_bbox = [float(coord) for point in poly for coord in point]
        lines.append({
            "text": str(text),
            "confidence": float(rec_scores[i]),
            "bbox": flat_bbox,
        })
    return lines


@app.post("/ocr/extract")
async def ocr_extract(
    image: UploadFile = File(...),
    document_type: Optional[str] = Form(None),
):
    if image.content_type not in ALLOWED_CONTENT_TYPES:
        return JSONResponse(
            status_code=422,
            content={"detail": "Formato no soportado. Use archivos JPEG, PNG o PDF"},
        )

    file_bytes = await image.read()

    if len(file_bytes) > MAX_FILE_SIZE:
        return JSONResponse(
            status_code=413,
            content={"detail": "El archivo excede el tamaño máximo de 10 MB"},
        )

    if image.content_type == "application/pdf":
        img_array = _pdf_first_page_to_array(file_bytes)
    else:
        img_array = _image_bytes_to_array(file_bytes)

    lines = _run_ocr(img_array)

    if document_type:
        doc_type = document_type
    else:
        doc_type = _classify_document(lines)

    structured_fields = _extract_structured_fields(lines, doc_type)

    return JSONResponse(content={
        "document_type": doc_type,
        "lines": lines,
        "structured_fields": structured_fields,
    })
