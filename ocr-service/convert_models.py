"""Download PaddleOCR PP-OCRv5 models and convert to ONNX for OpenVINO.

Run during Docker build to bake ONNX models into the image.
paddle2onnx.export() accepts file paths (including .json model files)
and produces ONNX output.
"""

import os
from pathlib import Path

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


def convert_to_onnx() -> None:
    """Convert PaddleX models to ONNX using paddle2onnx."""
    import paddle2onnx

    models = discover_models()
    if not models:
        raise RuntimeError(f"No models found in {MODEL_DIR}")

    print(f"Found {len(models)} models to convert:")
    for m in models:
        print(f"  - {m.name}")

    for model_path in models:
        output_onnx = model_path / "inference.onnx"

        if output_onnx.exists():
            print(f"ONNX already exists for {model_path.name}, skipping.")
            continue

        model_file = str(model_path / "inference.json")
        params_file = str(model_path / "inference.pdiparams")

        if not Path(model_file).exists():
            print(f"WARNING: no inference.json for {model_path.name}, skipping")
            continue

        print(f"Converting {model_path.name} to ONNX...")
        paddle2onnx.export(
            model_file,
            params_file,
            str(output_onnx),
            opset_version=14,
        )
        print(f"  -> {output_onnx} ({output_onnx.stat().st_size} bytes)")


def main() -> None:
    download_models()
    convert_to_onnx()

    converted = [
        d.name for d in MODEL_DIR.iterdir()
        if d.is_dir() and (d / "inference.onnx").exists()
    ]
    print(f"Converted models: {converted}")
    if not converted:
        raise RuntimeError("No models were converted to ONNX")

    print("All models converted successfully.")


if __name__ == "__main__":
    main()
