"""Download PaddleOCR PP-OCRv5 mobile models and convert to OpenVINO IR format.

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


def convert_to_openvino_ir() -> None:
    """Convert PaddlePaddle models to OpenVINO IR format (.xml + .bin).

    PaddleX stores the model graph in inference.json (same format as .pdmodel).
    We copy it to inference.pdmodel so OpenVINO's PaddlePaddle frontend can read it.
    """
    for model_name in MODELS:
        model_path = MODEL_DIR / model_name
        output_xml = model_path / "inference.xml"

        if output_xml.exists():
            print(f"OpenVINO IR already exists for {model_name}, skipping.")
            continue

        json_file = model_path / "inference.json"
        pdmodel_file = model_path / "inference.pdmodel"

        # PaddleX uses inference.json; OpenVINO expects .pdmodel extension
        if json_file.exists() and not pdmodel_file.exists():
            shutil.copy2(json_file, pdmodel_file)

        if not pdmodel_file.exists():
            print(f"WARNING: no model file found for {model_name}, skipping")
            continue

        print(f"Converting {model_name} to OpenVINO IR...")
        ov_model = ov.convert_model(str(pdmodel_file))
        ov.save_model(ov_model, str(output_xml))
        print(f"  -> {output_xml} ({output_xml.stat().st_size} bytes)")


def main() -> None:
    download_models()
    convert_to_openvino_ir()

    # Verify all IR files exist
    for model_name in MODELS:
        xml_path = MODEL_DIR / model_name / "inference.xml"
        bin_path = MODEL_DIR / model_name / "inference.bin"
        if not xml_path.exists() or not bin_path.exists():
            raise RuntimeError(f"OpenVINO IR conversion failed for {model_name}")
        print(f"Verified: {xml_path} ({xml_path.stat().st_size} bytes)")

    print("All models converted successfully.")


if __name__ == "__main__":
    main()
