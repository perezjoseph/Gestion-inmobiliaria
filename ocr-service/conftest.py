"""Shared pytest fixtures for OCR sidecar tests.

Patches PaddleOCR at import time so main.py never tries to load real models.
"""
import sys
from unittest.mock import MagicMock

import pytest

# Patch paddleocr before main.py is imported so the module-level
# `ocr_engine = PaddleOCR(...)` call uses the mock.
_mock_paddleocr_module = MagicMock()
sys.modules.setdefault("paddleocr", _mock_paddleocr_module)
