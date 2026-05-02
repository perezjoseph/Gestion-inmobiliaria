"""Shared pytest fixtures for OCR sidecar tests.

Patches OpenVINOOCREngine at import time so main.py never tries to load real models.
"""
import sys
from unittest.mock import MagicMock

# Patch openvino before main.py is imported so the module-level
# `ocr_engine = OpenVINOOCREngine()` call uses the mock.
_mock_openvino = MagicMock()
sys.modules.setdefault("openvino", _mock_openvino)

_mock_cv2 = MagicMock()
sys.modules.setdefault("cv2", _mock_cv2)

_mock_yaml = MagicMock()
sys.modules.setdefault("yaml", _mock_yaml)

# Mock the ocr_engine module itself so OpenVINOOCREngine.__init__ is a no-op
_mock_ocr_engine_module = MagicMock()
sys.modules.setdefault("ocr_engine", _mock_ocr_engine_module)
