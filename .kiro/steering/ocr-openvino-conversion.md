---
inclusion: manual
---

# OCR Service: PaddleOCR to OpenVINO via ONNX

## Why ONNX Is Required (Not Optional)

PaddleOCR PP-OCRv5 (v3.5+) downloads models in **PaddleX format**:
- `inference.json` — JSON-serialized model graph (NOT protobuf `.pdmodel`)
- `inference.pdiparams` — binary weights

OpenVINO's `ov.convert_model()` only reads **protobuf `.pdmodel`** files. It cannot parse PaddleX's JSON format. Attempting to rename `.json` → `.pdmodel` or feed it directly causes `Model can't be parsed` errors.

Therefore the conversion pipeline is:
```
PaddleOCR download → inference.json + .pdiparams
                   → paddle2onnx.export() → inference.onnx
                   → OpenVINO read_model() at runtime on GPU
```

`paddle2onnx` natively handles both `.pdmodel` (protobuf) and `.json` (PaddleX) formats. This was verified locally on 2026-05-02.

## Model Names (lang="es", PaddleOCR 3.5)

PaddleOCR downloads different models depending on version and language:
- **Detection**: `PP-OCRv5_server_det` (not `mobile`)
- **Recognition**: `latin_PP-OCRv5_mobile_rec` (latin prefix for Spanish)
- **Orientation**: `PP-LCNet_x1_0_textline_ori` (not `x0_25`)

The `ocr_engine.py` discovers models dynamically by pattern matching (`det`, `rec`, `textline_ori`) rather than hardcoding names.

## Build-Time Conversion

`convert_models.py` runs during Docker build:
1. `download_models()` — uses PaddleOCR to trigger model download
2. `discover_models()` — finds all dirs with `inference.pdiparams`
3. `convert_to_onnx()` — calls `paddle2onnx.export(model_file, params_file, save_file, opset_version=14)` with file paths

## Runtime Loading

`ocr_engine.py` uses OpenVINO to load ONNX on Intel GPU:
```python
core = ov.Core()
model = core.read_model("inference.onnx")
compiled = core.compile_model(model, "GPU")
```

Model caching (`CACHE_DIR`) stores compiled GPU kernels for faster subsequent startups.
