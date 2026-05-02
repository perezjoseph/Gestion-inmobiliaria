"""Download PaddleOCR PP-OCRv5 models and convert to OpenVINO IR format.

Run during Docker build to bake pre-converted IR models into the image.
OpenVINO can read PaddlePaddle .pdmodel files directly via ov.convert_model(),
then we serialize to IR (.xml + .bin) for faster runtime loading.
"""

import os
import shutil
from pathlib import Path

import openvino as ov

CACHE_HOME = Path(os.environ.get("PADDLE_PDX_CACHE_HOME", "/app/.paddleocr"))
MODEL_DIR = CACHE_HOME / "official_models"


def download_models() -> None:
    """Use PaddleOCR to trigger model download."""
    from paddleocr import PaddleOCR

    print("Downloading PaddleOCR models...")
    PaddleOCR(
        use_textline_orientation=True,
        use_doc_orientation_classify=False,
        use_doc_unwarping=False,
        lang="es",
        device="cpu",
    )
    print("Models downloaded successfully.")


def discover_models() -> list[Path]:
    """Find all downloaded model directories that have inference params."""
    models = []
    if not MODEL_DIR.exists():
        return models
    for model_dir in sorted(MODEL_DIR.iterdir()):
        if not model_dir.is_dir():
            continue
        if (model_dir / "inference.pdiparams").exists():
            models.append(model_dir)
    return models


def convert_to_openvino_ir() -> None:
    """Convert all PaddlePaddle models to OpenVINO IR format (.xml + .bin).

    PaddleX stores the model graph in inference.json (same format as .pdmodel).
    We copy it to inference.pdmodel so OpenVINO's PaddlePaddle frontend can read it.
    """
    models = discover_models()
    if not models:
        raise RuntimeError(f"No models found in {MODEL_DIR}")

    print(f"Found {len(models)} models to convert:")
    for m in models:
        print(f"  - {m.name}")

    for model_path in models:
        output_xml = model_path / "inference.xml"

        if output_xml.exists():
            print(f"OpenVINO IR already exists for {model_path.name}, skipping.")
            continue

        json_file = model_path / "inference.json"
        pdmodel_file = model_path / "inference.pdmodel"

        # PaddleX uses inference.json; OpenVINO expects .pdmodel extension
        if json_file.exists() and not pdmodel_file.exists():
            shutil.copy2(json_file, pdmodel_file)

        if not pdmodel_file.exists():
            print(f"WARNING: no .pdmodel for {model_path.name}, skipping")
            continue

        print(f"Converting {model_path.name} to OpenVINO IR...")
        ov_model = ov.convert_model(str(pdmodel_file))
        ov.save_model(ov_model, str(output_xml))
        print(f"  -> {output_xml} ({output_xml.stat().st_size} bytes)")


def main() -> None:
    download_models()
    convert_to_openvino_ir()

    # Verify at least the key models converted
    converted = [
        d.name for d in MODEL_DIR.iterdir()
        if d.is_dir() and (d / "inference.xml").exists()
    ]
    print(f"Converted models: {converted}")
    if not converted:
        raise RuntimeError("No models were converted to OpenVINO IR")

    print("All models converted successfully.")


if __name__ == "__main__":
    main()
