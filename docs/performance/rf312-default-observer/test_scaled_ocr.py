import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image, ImageDraw

from scaled_ocr import create_half_png, remap_half_receipt


class ScaledOcr(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source, self.half = self.root / "source.png", self.root / "half.png"
        with Image.new("RGB", (201, 161), "white") as bitmap:
            ImageDraw.Draw(bitmap).rectangle((30, 50, 160, 80), fill="black")
            bitmap.save(self.source)
        create_half_png(self.source, self.half)
        self.source_receipt, self.half_receipt = self.root / "source.json", self.root / "half.json"
        self.original = self.receipt(self.source, "1 234567890")
        self.raw = self.receipt(self.half, "1234567890")
        self.write()

    def receipt(self, image, text):
        with Image.open(image) as bitmap:
            width, height = bitmap.size
        return dict(Engine="Windows.Media.Ocr", Language="en-GB", ImageSha256=hashlib.sha256(image.read_bytes()).hexdigest(),
                    PixelWidth=width, PixelHeight=height, Text=text,
                    Lines=[dict(Text=text, Words=[dict(Text=text, X=10, Y=20, Width=40, Height=10)])])

    def write(self):
        self.source_receipt.write_text(json.dumps(self.original), encoding="utf-8")
        self.half_receipt.write_text(json.dumps(self.raw), encoding="utf-8")

    def remap(self):
        return remap_half_receipt(self.source, self.source_receipt, self.half, self.half_receipt)

    def test_real_odd_dimensions_map_by_actual_ratio_and_preserve_text(self):
        original_files = [p.read_bytes() for p in (self.source, self.source_receipt, self.half, self.half_receipt)]
        mapped, proof = self.remap()
        word = mapped["Lines"][0]["Words"][0]
        self.assertEqual(word["Text"], "1234567890")
        self.assertAlmostEqual(word["X"], 10 * 201 / 100)
        self.assertAlmostEqual(word["Y"], 20 * 161 / 80)
        self.assertEqual(proof["halfSize"], [100, 80])
        self.assertEqual(mapped["ImageSha256"], self.original["ImageSha256"])
        self.assertEqual(original_files, [p.read_bytes() for p in (self.source, self.source_receipt, self.half, self.half_receipt)])

    def test_does_not_join_or_correct_split_or_wrong_digits(self):
        for text in ("1 234567890", "123456789O", "1234567891"):
            with self.subTest(text=text):
                self.raw["Lines"][0]["Words"][0]["Text"] = text
                self.write()
                mapped, _ = self.remap()
                self.assertEqual(mapped["Lines"][0]["Words"][0]["Text"], text)

    def test_reject_existing_output_without_overwrite(self):
        before = self.half.read_bytes()
        with self.assertRaises(ValueError):
            create_half_png(self.source, self.half)
        self.assertEqual(before, self.half.read_bytes())

    def test_reject_changed_source_or_scaled_receipt_binding(self):
        for target in (self.original, self.raw):
            with self.subTest(target=target["PixelWidth"]):
                before = target["ImageSha256"]
                target["ImageSha256"] = "0" * 64
                self.write()
                with self.assertRaises(ValueError):
                    self.remap()
                target["ImageSha256"] = before
        self.write()

    def test_reject_other_dimensions_even_with_matching_receipt(self):
        with Image.open(self.source) as bitmap:
            bitmap.resize((101, 80)).save(self.half)
        self.raw = self.receipt(self.half, "1234567890")
        self.write()
        with self.assertRaises(ValueError):
            self.remap()

    def test_reject_changed_pixels_even_with_updated_hash(self):
        with Image.open(self.half) as bitmap:
            bitmap.putpixel((0, 0), (0, 0, 0))
            bitmap.save(self.half)
        self.raw = self.receipt(self.half, "1234567890")
        self.write()
        with self.assertRaises(ValueError):
            self.remap()

    def test_reject_bad_word_geometry_and_nonfinite_numbers(self):
        baseline = copy.deepcopy(self.raw)
        for key, value in (("X", -1), ("Width", 0), ("X", True), ("Y", float("inf")), ("Height", 1000)):
            with self.subTest(key=key, value=value):
                self.raw = copy.deepcopy(baseline)
                self.raw["Lines"][0]["Words"][0][key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.remap()

    def test_reject_unexpected_engine_language_and_empty_lines(self):
        baseline = copy.deepcopy(self.raw)
        for key, value in (("Engine", "other"), ("Language", "other"), ("Lines", [])):
            with self.subTest(key=key):
                self.raw = copy.deepcopy(baseline)
                self.raw[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.remap()

    def test_reject_non_png_and_oversized_header(self):
        for data in (b"invalid", b"\x89PNG\r\n\x1a\n" + b"\0" * 8 + (8000).to_bytes(4, "big") * 2):
            self.source.write_bytes(data)
            with self.assertRaises(ValueError):
                create_half_png(self.source, self.root / "new.png")


if __name__ == "__main__":
    unittest.main()
