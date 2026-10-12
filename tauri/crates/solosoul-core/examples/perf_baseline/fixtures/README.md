# Performance preview assets

`scanned.pdf` is a single raster page built from the existing public
`solosoul-core/tests/fixtures/ocr_test.png`. It contains the visible OCR corpus
and no PDF text layer. The original blank `tests/fixtures/scanned.pdf` remains
an independent legacy unit fixture.

The committed PDF uses ReportLab 4.4.9 with `invariant=1`, an uncompressed page
content stream, a 612 x 792 point page, and one RGB image at x=36 with width=540
and its original aspect ratio, vertically centered. Its decoded image bytes
must exactly equal the source PNG converted to RGB; pypdf 6.10.0 verified that
the page has no extracted text. Two independently generated files were byte
identical, and Poppler rendered the corpus without clipping.

The media manifest records this asset's exact SHA-256 and byte length. Generate
fresh media datasets after this asset change; retain historical datasets and
their matching verifier binaries. Do not rewrite historical markers or combine
measurements made with different asset hashes.
