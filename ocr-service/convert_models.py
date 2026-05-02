"""Download PaddleOCR PP-OCRv5 mobile models and convert to ONNX for OpenVINO.

Run during Docker build to bake ONNX models into the image.
"""

import os
import subprocess
import sys
from pathlib import Path

CACHE_HOME = Path(os.environ.get("PADDLE_PDX_CACHE_HOME", "/app/.paddleocr"))
MODEL_DIR = CACHE_HOME / "official_models"

# Models used by the OCR pipeline
MODELS = [
    "PP-OCRv5_mobile_det",
    "PP-OCRv5_mobile_rec",
    "PP-LCNet_x0_25_textline_ori",
]


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


def convert_to_onnx() -> None:
    """Convert PaddlePaddle inference models to ONNX format."""
    for model_name in MODELS:
        model_path = MODEL_DIR / model_name
        pdiparams = model_path / "inference.pdiparams"
        output_onnx = model_path / "inference.onnx"

        if output_onnx.exists():
            print(f"ONNX already exists for {model_name}, skipping.")
            continue

        if not pdiparams.exists():
            print(f"WARNING: {pdiparams} not found, skipping {model_name}")
            continue

        # inference.json contains the model structure for PaddleX models
        # paddle2onnx expects the .pdmodel file — derive from .pdiparams
        pdmodel = model_path / "inference.json"
        if not pdmodel.exists():
            print(f"WARNING: {pdmodel} not found, skipping {model_name}")
            continue

        print(f"Converting {model_name} to ONNX...")
        cmd = [
            sys.executable, "-m", "paddle2onnx",
            "--model_dir", str(model_path),
            "--model_filename", "inference.json",
            "--params_filename", "inference.pdiparams",
            "--save_file", str(output_onnx),
            "--opset_version", "14",
            "--enable_onnx_checker", "True",
        ]
        subprocess.check_call(cmd)
        print(f"  -> {output_onnx}")


def main() -> None:
    download_models()
    convert_to_onnx()
    print("All models converted successfully.")


if __name__ == "__main__":
    main()
