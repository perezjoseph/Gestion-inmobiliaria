"""Download PaddleOCR PP-OCRv5 mobile models and convert to ONNX for OpenVINO.

Run during Docker build to bake ONNX models into the image.
PaddleX models use inference.json (not .pdmodel), so we use paddlex's
built-in export to produce ONNX files.
"""

import os
import shutil
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
    """Convert PaddlePaddle inference models to ONNX format.

    PaddleX models store the model graph in inference.json (not .pdmodel).
    We rename inference.json -> inference.pdmodel temporarily so paddle2onnx
    can process them, then produce inference.onnx.
    """
    import paddle2onnx

    for model_name in MODELS:
        model_path = MODEL_DIR / model_name
        output_onnx = model_path / "inference.onnx"

        if output_onnx.exists():
            print(f"ONNX already exists for {model_name}, skipping.")
            continue

        json_file = model_path / "inference.json"
        pdmodel_file = model_path / "inference.pdmodel"
        pdiparams = model_path / "inference.pdiparams"

        if not pdiparams.exists():
            print(f"WARNING: {pdiparams} not found, skipping {model_name}")
            continue

        # PaddleX uses inference.json; paddle2onnx expects inference.pdmodel
        if json_file.exists() and not pdmodel_file.exists():
            shutil.copy2(json_file, pdmodel_file)

        if not pdmodel_file.exists():
            print(f"WARNING: no model file found for {model_name}, skipping")
            continue

        print(f"Converting {model_name} to ONNX...")
        model_content = pdmodel_file.read_bytes()
        params_content = pdiparams.read_bytes()

        onnx_model = paddle2onnx.export(
            model_content,
            params_content,
            opset_version=14,
        )
        output_onnx.write_bytes(onnx_model)
        print(f"  -> {output_onnx} ({output_onnx.stat().st_size} bytes)")


def main() -> None:
    download_models()
    convert_to_onnx()

    # Verify all ONNX files exist
    for model_name in MODELS:
        onnx_path = MODEL_DIR / model_name / "inference.onnx"
        if not onnx_path.exists():
            raise RuntimeError(f"ONNX conversion failed for {model_name}: {onnx_path}")
        print(f"Verified: {onnx_path} ({onnx_path.stat().st_size} bytes)")

    print("All models converted successfully.")


if __name__ == "__main__":
    main()
