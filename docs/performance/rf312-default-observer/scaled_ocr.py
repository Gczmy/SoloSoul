"""Bind supplemental half-size OCR to the unchanged native screenshot.

This module changes only OCR input pixels and coordinate units. It does not join,
replace, normalize, or infer recognized text. Callers retain the original receipt
for controls, the exact content verifier, and the original operation deadline.
"""

from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import struct

from PIL import Image


def _sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _ordinary(path):
    path = Path(path)
    if not path.is_file() or any(
        p.is_symlink() or getattr(p.stat(), "st_file_attributes", 0) & 0x400
        for p in (path, *path.parents)
    ):
        raise ValueError("Ordinary evidence file and parents required")
    return path


def _bitmap(path):
    path = _ordinary(path)
    data = path.read_bytes()
    if len(data) < 24 or not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("Bounded PNG evidence required")
    width, height = struct.unpack(">II", data[16:24])
    if not 32 <= width <= 4096 or not 32 <= height <= 4096 or width * height > 8_388_608:
        raise ValueError("Bounded screenshot dimensions required")
    with Image.open(path) as bitmap:
        if bitmap.size != (width, height) or getattr(bitmap, "n_frames", 1) != 1:
            raise ValueError("Single PNG dimensions must match")
        return bitmap.convert("RGB")


def create_half_png(source_image, output_image):
    """Create one deterministic derivative, refusing existing/reparse outputs."""
    source_image, output_image = Path(source_image), Path(output_image)
    if output_image.exists() or output_image.is_symlink():
        raise ValueError("Fresh half-size PNG output required")
    if any(p.is_symlink() or getattr(p.stat(), "st_file_attributes", 0) & 0x400
           for p in (output_image.parent, *output_image.parent.parents)):
        raise ValueError("Ordinary output parents required")
    before = _sha(_ordinary(source_image))
    with _bitmap(source_image) as bitmap:
        size = (bitmap.width // 2, bitmap.height // 2)
        if min(size) < 32:
            raise ValueError("Half-size screenshot too small")
        with bitmap.resize(size, Image.Resampling.LANCZOS) as resized:
            with output_image.open("xb") as stream:
                resized.save(stream, format="PNG")
    if _sha(source_image) != before:
        raise ValueError("Native screenshot changed during scaling")
    return output_image


def _receipt(path, image):
    path = _ordinary(path)
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    with _bitmap(image) as bitmap:
        size = bitmap.size
    if (value.get("Engine") != "Windows.Media.Ocr" or value.get("Language") != "en-GB"
            or value.get("ImageSha256") != _sha(_ordinary(image))
            or (value.get("PixelWidth"), value.get("PixelHeight")) != size):
        raise ValueError("Exact native OCR input binding required")
    lines = value.get("Lines")
    if not isinstance(lines, list) or not 1 <= len(lines) <= 256:
        raise ValueError("Bounded OCR lines required")
    for line in lines:
        words = line.get("Words")
        if not isinstance(words, list) or not 1 <= len(words) <= 32:
            raise ValueError("Bounded OCR words required")
        for word in words:
            text = word.get("Text")
            if not isinstance(text, str) or not 0 < len(text) <= 128:
                raise ValueError("Bounded recognized text required")
            coordinates = [word.get(key) for key in ("X", "Y", "Width", "Height")]
            if any(type(v) not in (int, float) or not math.isfinite(v) for v in coordinates):
                raise ValueError("Finite raw OCR coordinates required")
            x, y, width, height = coordinates
            if not (0 <= x < x + width <= size[0] and 0 <= y < y + height <= size[1]):
                raise ValueError("Raw OCR word outside its input")
    return value


def remap_half_receipt(source_image, source_receipt, half_image, half_receipt):
    """Verify derivative pixels and raw receipts, then map unchanged word boxes.

    The returned receipt belongs to source-image coordinates. Its provenance
    must be stored beside the accepted proof and independently reconstructed.
    The original content verifier must still accept its exact words.
    """
    source_image, source_receipt, half_image, half_receipt = map(
        Path, (source_image, source_receipt, half_image, half_receipt)
    )
    paths = [source_image, source_receipt, half_image, half_receipt]
    before = [_sha(_ordinary(path)) for path in paths]
    original = _receipt(source_receipt, source_image)
    raw = _receipt(half_receipt, half_image)
    with _bitmap(source_image) as source, _bitmap(half_image) as half:
        if half.size != (source.width // 2, source.height // 2):
            raise ValueError("Exact fixed half-size input required")
        with source.resize(half.size, Image.Resampling.LANCZOS) as expected:
            if half.tobytes() != expected.tobytes():
                raise ValueError("Derivative pixels do not match the native screenshot")
        x_ratio, y_ratio = source.width / half.width, source.height / half.height
        original_size, half_size = list(source.size), list(half.size)
    mapped = deepcopy(raw)
    for line in mapped["Lines"]:
        for word in line["Words"]:
            for key in ("X", "Width"):
                word[key] *= x_ratio
            for key in ("Y", "Height"):
                word[key] *= y_ratio
    mapped.update(ImageSha256=before[0], PixelWidth=original["PixelWidth"], PixelHeight=original["PixelHeight"])
    if [_sha(path) for path in paths] != before:
        raise ValueError("Evidence changed during coordinate mapping")
    provenance = dict(
        kind="fixed-half-lanczos", sourceImageSha256=before[0], sourceOcrSha256=before[1],
        halfImageSha256=before[2], halfOcrSha256=before[3], sourceSize=original_size,
        halfSize=half_size, exactDerivativePixelsVerified=True, recognizedTextUnchanged=True,
    )
    return mapped, provenance
