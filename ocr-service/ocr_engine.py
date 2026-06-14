"""OpenVINO-based OCR engine for Intel GPU inference.

Implements the PP-OCRv5 mobile detection + recognition pipeline using
OpenVINO's GPU plugin, compatible with all Intel GPUs (HD 530+, UHD, Iris Xe, Arc).

The pipeline:
  1. Text detection  (PP-OCRv5_mobile_det)  — finds text regions
  2. Text line orientation (PP-LCNet_x0_25_textline_ori) — classifies orientation
  3. Text recognition (PP-OCRv5_mobile_rec) — reads text from each region
"""

import logging
import os
from pathlib import Path

import cv2
import numpy as np
import openvino as ov
import yaml

from metrics import ocr_inference_seconds

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
MODEL_DIR = Path(os.environ.get("PADDLE_PDX_CACHE_HOME", "/app/.paddleocr")) / "official_models"
DEVICE = os.environ.get("OPENVINO_DEVICE", "GPU")
IR_MODEL_FILE = "inference.xml"

# Detection config — resolved at runtime
DET_MODEL = None  # set by _resolve_model_names()
DET_THRESH = 0.3
DET_BOX_THRESH = 0.6
DET_UNCLIP_RATIO = 1.5
DET_MIN_SIZE = 3

# Recognition config — resolved at runtime
REC_MODEL = None  # set by _resolve_model_names()
REC_IMG_HEIGHT = 48
REC_IMG_WIDTH = 320

# Orientation config — resolved at runtime
ORI_MODEL = None  # set by _resolve_model_names()


def _resolve_model_names() -> tuple[str, str, str]:
    """Discover actual model directory names by pattern matching.

    PaddleOCR versions use different naming (e.g. PP-OCRv5_mobile_det vs
    PP-OCRv5_server_det, latin_PP-OCRv5_mobile_rec vs PP-OCRv5_mobile_rec).
    """
    det, rec, ori = None, None, None
    if not MODEL_DIR.exists():
        raise RuntimeError(f"Model directory not found: {MODEL_DIR}")

    for d in MODEL_DIR.iterdir():
        if not d.is_dir():
            continue
        name = d.name.lower()
        if "det" in name and (d / IR_MODEL_FILE).exists():
            det = d.name
        elif "rec" in name and (d / IR_MODEL_FILE).exists():
            rec = d.name
        elif "textline_ori" in name and (d / IR_MODEL_FILE).exists():
            ori = d.name

    if not det:
        raise RuntimeError(f"No detection model found in {MODEL_DIR}")
    if not rec:
        raise RuntimeError(f"No recognition model found in {MODEL_DIR}")

    return det, rec, ori


def _load_character_dict(model_name: str) -> list[str]:
    """Load character dictionary from the model's inference.yml."""
    yml_path = MODEL_DIR / model_name / "inference.yml"
    with open(yml_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)

    # The dict is under PostProcess -> character_dict or label_list
    post = config.get("PostProcess", config.get("postprocess", {}))
    char_list = post.get("character_dict", post.get("label_list", []))

    if not char_list:
        char_list = _search_nested_dict(config)

    return char_list


def _search_nested_dict(config: dict) -> list[str]:
    """Search nested config for a character dictionary list."""
    for section in config.values():
        if not isinstance(section, dict):
            continue
        for key, val in section.items():
            if ("dict" in key.lower() or "label" in key.lower()) and isinstance(val, list) and len(val) > 100:
                return val
    return []


# ---------------------------------------------------------------------------
# Pre-processing helpers
# ---------------------------------------------------------------------------

DET_INPUT_SIZE = int(os.environ.get("OCR_DET_INPUT_SIZE", "640"))


def _resize_for_det(img: np.ndarray, max_side: int = 0) -> tuple[np.ndarray, float]:
    """Resize image keeping aspect ratio, pad to multiple of 32."""
    if max_side == 0:
        max_side = DET_INPUT_SIZE
    h, w = img.shape[:2]
    ratio = min(max_side / h, max_side / w)
    new_h, new_w = int(h * ratio), int(w * ratio)
    new_h = ((new_h + 31) // 32) * 32
    new_w = ((new_w + 31) // 32) * 32
    resized = cv2.resize(img, (new_w, new_h))
    return resized, ratio


def _normalize(img: np.ndarray, mean: tuple = (0.485, 0.456, 0.406),
               std: tuple = (0.229, 0.224, 0.225)) -> np.ndarray:
    """Normalize image to [0,1] then apply ImageNet mean/std."""
    img = img.astype(np.float32) / 255.0
    img = (img - np.array(mean, dtype=np.float32)) / np.array(std, dtype=np.float32)
    return img


def _det_preprocess(img: np.ndarray) -> tuple[np.ndarray, float, tuple[int, int]]:
    """Preprocess image for detection model. Pre-allocates padding buffer."""
    orig_shape = img.shape[:2]
    resized, ratio = _resize_for_det(img)
    normalized = _normalize(resized)
    blob = normalized.transpose(2, 0, 1)[np.newaxis, ...]
    return blob, ratio, orig_shape


# ---------------------------------------------------------------------------
# Post-processing: detection
# ---------------------------------------------------------------------------

def _box_from_bitmap(pred: np.ndarray, bitmap: np.ndarray,
                     orig_h: int, orig_w: int, ratio: float) -> list[np.ndarray]:
    """Extract bounding boxes from detection bitmap using contours."""
    height, width = bitmap.shape
    contours, _ = cv2.findContours(
        (bitmap * 255).astype(np.uint8), cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE
    )

    boxes = []
    for contour in contours:
        if len(contour) < 4:
            continue
        rect = cv2.minAreaRect(contour)
        box = cv2.boxPoints(rect)

        # Filter small boxes
        w_box = np.linalg.norm(box[0] - box[1])
        h_box = np.linalg.norm(box[1] - box[2])
        if min(w_box, h_box) < DET_MIN_SIZE:
            continue

        # Score inside the contour
        mask = np.zeros((height, width), dtype=np.uint8)
        cv2.fillPoly(mask, [contour.astype(np.int32)], 1)
        score = cv2.mean(pred, mask)[0]
        if score < DET_BOX_THRESH:
            continue

        # Unclip
        box = _unclip(box, DET_UNCLIP_RATIO)

        # Scale back to original image coordinates
        box[:, 0] = np.clip(box[:, 0] / ratio, 0, orig_w)
        box[:, 1] = np.clip(box[:, 1] / ratio, 0, orig_h)

        # Order points: top-left, top-right, bottom-right, bottom-left
        box = _order_points(box)
        boxes.append(box)

    # Sort boxes top-to-bottom, left-to-right
    if boxes:
        boxes.sort(key=lambda b: (b[0][1], b[0][0]))

    return boxes


def _unclip(box: np.ndarray, unclip_ratio: float) -> np.ndarray:
    """Expand a box by the unclip ratio."""
    poly = cv2.minAreaRect(box.astype(np.float32))
    center, (w, h), angle = poly
    w *= unclip_ratio
    h *= unclip_ratio
    new_box = cv2.boxPoints((center, (w, h), angle))
    return new_box


def _order_points(pts: np.ndarray) -> np.ndarray:
    """Order 4 points as: top-left, top-right, bottom-right, bottom-left."""
    rect = np.zeros((4, 2), dtype=np.float32)
    s = pts.sum(axis=1)
    rect[0] = pts[np.argmin(s)]
    rect[2] = pts[np.argmax(s)]
    d = np.diff(pts, axis=1)
    rect[1] = pts[np.argmin(d)]
    rect[3] = pts[np.argmax(d)]
    return rect


# ---------------------------------------------------------------------------
# Pre-processing: recognition
# ---------------------------------------------------------------------------

def _crop_text_region(img: np.ndarray, box: np.ndarray) -> np.ndarray:
    """Crop and perspective-transform a text region from the image."""
    w = int(max(np.linalg.norm(box[0] - box[1]), np.linalg.norm(box[2] - box[3])))
    h = int(max(np.linalg.norm(box[0] - box[3]), np.linalg.norm(box[1] - box[2])))
    if w < 1 or h < 1:
        return np.zeros((REC_IMG_HEIGHT, REC_IMG_WIDTH, 3), dtype=np.uint8)

    dst = np.array([[0, 0], [w, 0], [w, h], [0, h]], dtype=np.float32)
    matrix = cv2.getPerspectiveTransform(box.astype(np.float32), dst)
    cropped = cv2.warpPerspective(img, matrix, (w, h))
    return cropped


def _rec_preprocess(crop: np.ndarray) -> np.ndarray:
    """Resize cropped text region and normalize for recognition model."""
    h, w = crop.shape[:2]
    ratio = REC_IMG_HEIGHT / h
    new_w = min(int(w * ratio), REC_IMG_WIDTH)
    resized = cv2.resize(crop, (new_w, REC_IMG_HEIGHT))

    # Pad to REC_IMG_WIDTH
    padded = np.zeros((REC_IMG_HEIGHT, REC_IMG_WIDTH, 3), dtype=np.uint8)
    padded[:, :new_w, :] = resized

    normalized = _normalize(padded)
    blob = normalized.transpose(2, 0, 1)[np.newaxis, ...]
    return blob


def _rec_preprocess_batch(crops: list[np.ndarray]) -> np.ndarray:
    """Vectorized batch preprocessing for recognition — avoids per-crop Python loop overhead."""
    n = len(crops)
    batch = np.zeros((n, 3, REC_IMG_HEIGHT, REC_IMG_WIDTH), dtype=np.float32)
    mean = np.array([0.485, 0.456, 0.406], dtype=np.float32)
    std = np.array([0.229, 0.224, 0.225], dtype=np.float32)

    for i, crop in enumerate(crops):
        h, w = crop.shape[:2]
        ratio = REC_IMG_HEIGHT / h
        new_w = min(int(w * ratio), REC_IMG_WIDTH)
        resized = cv2.resize(crop, (new_w, REC_IMG_HEIGHT))
        # normalize in-place on float view
        img_f = resized.astype(np.float32) / 255.0
        img_f = (img_f - mean) / std
        batch[i, :, :, :new_w] = img_f.transpose(2, 0, 1)

    return batch


# ---------------------------------------------------------------------------
# Post-processing: recognition
# ---------------------------------------------------------------------------

def _rec_postprocess(output: np.ndarray, char_dict: list[str]) -> tuple[str, float]:
    """Decode recognition model output using CTC greedy decoding."""
    # output shape: (1, seq_len, num_classes)
    preds = output[0]
    pred_indices = preds.argmax(axis=1)
    pred_scores = preds.max(axis=1)

    # CTC decode: collapse repeated chars and remove blank (index 0)
    text = []
    scores = []
    prev_idx = -1
    for i, idx in enumerate(pred_indices):
        if idx == 0 or idx == prev_idx:
            prev_idx = idx
            continue
        # char_dict doesn't include blank at index 0, so offset by 1
        char_idx = idx - 1
        if 0 <= char_idx < len(char_dict):
            text.append(str(char_dict[char_idx]))
            scores.append(float(pred_scores[i]))
        prev_idx = idx

    result_text = "".join(text)
    avg_score = float(np.mean(scores)) if scores else 0.0
    return result_text, avg_score


# ---------------------------------------------------------------------------
# Line grouping: merge word-level boxes into text lines
# ---------------------------------------------------------------------------

def _box_center_y(box: np.ndarray) -> float:
    """Get the vertical center of a 4-point box."""
    return float(np.mean(box[:, 1]))


def _box_height(box: np.ndarray) -> float:
    """Get the height of a 4-point box."""
    return float(max(np.linalg.norm(box[0] - box[3]), np.linalg.norm(box[1] - box[2])))


def _box_left(box: np.ndarray) -> float:
    """Get the leftmost x coordinate."""
    return float(np.min(box[:, 0]))


def _merge_boxes(boxes: list[np.ndarray]) -> np.ndarray:
    """Merge multiple boxes into one bounding box."""
    all_points = np.vstack(boxes)
    x_min, y_min = all_points.min(axis=0)
    x_max, y_max = all_points.max(axis=0)
    return np.array([
        [x_min, y_min], [x_max, y_min],
        [x_max, y_max], [x_min, y_max],
    ], dtype=np.float32)


def _group_boxes_into_lines(boxes: list[np.ndarray],
                            y_overlap_thresh: float = 0.5) -> list[list[int]]:
    """Group word-level boxes into lines based on vertical overlap.

    Two boxes are on the same line if their vertical overlap exceeds
    y_overlap_thresh relative to the shorter box height.
    Returns list of groups, each group is a list of box indices sorted left-to-right.
    """
    if not boxes:
        return []

    n = len(boxes)
    used = [False] * n
    groups: list[list[int]] = []

    # Sort by vertical center
    indices = sorted(range(n), key=lambda i: _box_center_y(boxes[i]))

    for idx in indices:
        if used[idx]:
            continue

        group = [idx]
        used[idx] = True
        cy = _box_center_y(boxes[idx])
        h = _box_height(boxes[idx])

        for other in indices:
            if used[other]:
                continue
            other_cy = _box_center_y(boxes[other])
            other_h = _box_height(boxes[other])
            min_h = max(min(h, other_h), 1.0)
            overlap = min_h - abs(cy - other_cy)
            if overlap / min_h >= y_overlap_thresh:
                group.append(other)
                used[other] = True

        # Sort group left-to-right
        group.sort(key=lambda i: _box_left(boxes[i]))
        groups.append(group)

    return groups


# ---------------------------------------------------------------------------
# Main OCR Engine
# ---------------------------------------------------------------------------

class OpenVINOOCREngine:
    """OCR engine using OpenVINO for inference on Intel GPUs."""

    # Max text regions to batch in one recognition call
    REC_BATCH_SIZE = int(os.environ.get("OCR_REC_BATCH_SIZE", "16"))

    def __init__(self, device: str = DEVICE):
        self.core = ov.Core()
        self.device = device

        available = self.core.available_devices
        logger.info("OpenVINO available devices: %s", available)

        if device not in available and device != "AUTO":
            logger.warning("Device %s not available, falling back to CPU", device)
            self.device = "CPU"

        global DET_MODEL, REC_MODEL, ORI_MODEL
        DET_MODEL, REC_MODEL, ORI_MODEL = _resolve_model_names()
        logger.info("Resolved models: det=%s, rec=%s, ori=%s", DET_MODEL, REC_MODEL, ORI_MODEL)

        self._load_models()
        self.char_dict = _load_character_dict(REC_MODEL)
        logger.info(
            "OCR engine initialized on %s with %d characters",
            self.device, len(self.char_dict),
        )

    def _load_models(self) -> None:
        """Compile detection and recognition models with optimized settings."""
        det_path = MODEL_DIR / DET_MODEL / IR_MODEL_FILE
        rec_path = MODEL_DIR / REC_MODEL / IR_MODEL_FILE

        cache_dir = os.environ.get("OPENVINO_CACHE_DIR", "/app/.cache")
        self.core.set_property({"CACHE_DIR": cache_dir})

        logger.info("Loading detection model from %s", det_path)
        det_model = self.core.read_model(str(det_path))
        det_model.reshape({0: [1, 3, DET_INPUT_SIZE, DET_INPUT_SIZE]})
        self.det_compiled = self.core.compile_model(det_model, self.device, {
            "PERFORMANCE_HINT": "LATENCY",
            "NUM_STREAMS": "1",
        })
        self._det_request = self.det_compiled.create_infer_request()

        logger.info("Loading recognition model from %s", rec_path)
        rec_model = self.core.read_model(str(rec_path))
        rec_model.reshape({0: [self.REC_BATCH_SIZE, 3, REC_IMG_HEIGHT, REC_IMG_WIDTH]})
        self.rec_compiled = self.core.compile_model(rec_model, self.device, {
            "PERFORMANCE_HINT": "THROUGHPUT",
            "NUM_STREAMS": "2",
        })
        self._rec_request = self.rec_compiled.create_infer_request()

    def predict(self, img: np.ndarray) -> list[dict]:
        """Run full OCR pipeline on an RGB image array.

        Returns a list of dicts with keys: text, confidence, bbox.
        Word-level detections are grouped into text lines.
        """
        if img.ndim != 3 or img.shape[2] != 3:
            return []

        # BGR for OpenCV operations
        img_bgr = cv2.cvtColor(img, cv2.COLOR_RGB2BGR)

        # 1. Detection (word-level boxes)
        boxes = self._detect(img_bgr)
        if not boxes:
            return []

        # 2. Batch recognition for all detected words
        crops = [_crop_text_region(img_bgr, box) for box in boxes]
        rec_results = self._recognize_batch(crops)

        word_results = []
        for i, (text, confidence) in enumerate(rec_results):
            if not text.strip():
                continue
            word_results.append({"text": text, "confidence": confidence, "box": boxes[i]})

        if not word_results:
            return []

        # 3. Group words into lines
        word_boxes = [w["box"] for w in word_results]
        line_groups = _group_boxes_into_lines(word_boxes)

        results = []
        for group in line_groups:
            texts = [word_results[i]["text"] for i in group]
            confs = [word_results[i]["confidence"] for i in group]
            group_boxes = [word_results[i]["box"] for i in group]

            line_text = " ".join(texts)
            line_conf = float(np.mean(confs)) if confs else 0.0
            merged_box = _merge_boxes(group_boxes)
            flat_bbox = [float(coord) for point in merged_box for coord in point]

            results.append({
                "text": line_text,
                "confidence": line_conf,
                "bbox": flat_bbox,
            })

        return results

    def _detect(self, img_bgr: np.ndarray) -> list[np.ndarray]:
        """Run text detection and return bounding boxes."""
        orig_h, orig_w = img_bgr.shape[:2]
        blob, ratio, _ = _det_preprocess(img_bgr)

        _, _, bh, bw = blob.shape
        if bh != DET_INPUT_SIZE or bw != DET_INPUT_SIZE:
            padded = np.zeros((1, 3, DET_INPUT_SIZE, DET_INPUT_SIZE), dtype=np.float32)
            padded[:, :, :bh, :bw] = blob
            blob = padded

        input_tensor = self._det_request.get_input_tensor(0)
        input_tensor.data[:] = blob
        with ocr_inference_seconds.labels(stage="detection").time():
            self._det_request.infer()
        output = self._det_request.get_output_tensor(0).data

        pred = output[0, 0, :bh, :bw]
        bitmap = (pred > DET_THRESH).astype(np.uint8)

        return _box_from_bitmap(pred, bitmap, orig_h, orig_w, ratio)

    def _recognize_batch(self, crops: list[np.ndarray]) -> list[tuple[str, float]]:
        """Run batched text recognition on all cropped text regions."""
        results: list[tuple[str, float]] = []
        batch_size = self.REC_BATCH_SIZE
        n = len(crops)

        input_tensor = self._rec_request.get_input_tensor(0)

        full_batches = n // batch_size
        for b in range(full_batches):
            start = b * batch_size
            batch_blob = _rec_preprocess_batch(crops[start:start + batch_size])
            input_tensor.data[:] = batch_blob
            with ocr_inference_seconds.labels(stage="recognition").time():
                self._rec_request.infer()
            output = self._rec_request.get_output_tensor(0).data
            for i in range(batch_size):
                results.append(_rec_postprocess(output[i:i+1], self.char_dict))

        remainder = n % batch_size
        if remainder:
            remaining_crops = crops[n - remainder:]
            padded_crops = remaining_crops + [
                np.zeros((REC_IMG_HEIGHT, REC_IMG_WIDTH, 3), dtype=np.uint8)
                for _ in range(batch_size - remainder)
            ]
            batch_blob = _rec_preprocess_batch(padded_crops)
            input_tensor.data[:] = batch_blob
            with ocr_inference_seconds.labels(stage="recognition").time():
                self._rec_request.infer()
            output = self._rec_request.get_output_tensor(0).data
            for i in range(remainder):
                results.append(_rec_postprocess(output[i:i+1], self.char_dict))

        return results