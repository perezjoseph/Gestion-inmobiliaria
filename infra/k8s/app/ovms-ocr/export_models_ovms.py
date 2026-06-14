"""Export OCR models from the ocr-service image into OVMS model repository layout.

OVMS expects:
  /models/<model_name>/1/inference.xml
  /models/<model_name>/1/inference.bin

Run this inside the ocr-service container (or during build) to produce the
OVMS-compatible directory structure from the converted IR models.
"""

import shutil
from pathlib import Path

MODEL_DIR = Path("/app/.paddleocr/official_models")
OUTPUT_DIR = Path("/models")

OVMS_MODEL_MAP = {
    "det": "pp-ocrv5-det",
    "rec": "pp-ocrv5-rec",
}


def export_for_ovms() -> None:
    for model_path in sorted(MODEL_DIR.iterdir()):
        if not model_path.is_dir():
            continue
        ir_xml = model_path / "inference.xml"
        ir_bin = model_path / "inference.bin"
        if not ir_xml.exists():
            continue

        name_lower = model_path.name.lower()
        ovms_name = None
        for key, mapped in OVMS_MODEL_MAP.items():
            if key in name_lower:
                ovms_name = mapped
                break

        if not ovms_name:
            continue

        dest = OUTPUT_DIR / ovms_name / "1"
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ir_xml, dest / "inference.xml")
        shutil.copy2(ir_bin, dest / "inference.bin")
        print(f"Exported {model_path.name} -> {dest}")


if __name__ == "__main__":
    export_for_ovms()
