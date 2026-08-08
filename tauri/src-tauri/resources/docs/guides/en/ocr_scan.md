# OCR & Scan

The OCR (Optical Character Recognition) feature lets you extract text from images or PDFs and import it as objects. All recognition runs **locally** — images are never uploaded to any external service.

## Using OCR

1. Enter **OCR Scan** from the sidebar or home page
2. Choose an input method:
   - **Select File**: pick an image or PDF from your device
   - **Take Photo**: mobile — capture directly with the camera
3. Pick the scan mode and model tier (see below)
4. The extracted text is displayed
5. Review the result and correct it if necessary
6. Click **Import as Object**, name the object, and save

## Scan Modes

- **General recognition**: extracts all text from an image or PDF
- **MRZ recognition**: recognizes the Machine Readable Zone of passports, visas, and ID cards, extracting structured fields such as document type, issuing country, document number, nationality, date of birth, sex, and expiry date

<!--STEPPER Scan a passport info page-->
1. Go to the **OCR Scan** page and switch the mode to **MRZ recognition**
2. Click **Select File** and upload a passport info page photo
3. Wait for recognition to complete
4. Review the recognized document fields
5. Click **Import as Object** to save
<!--/STEPPER-->

<!--TIP-->
If no MRZ is detected (e.g., you photographed an ordinary document), the app automatically falls back to general recognition — nothing is lost.
<!--/TIP-->

## Supported Input Formats

- General recognition: PNG, JPG, JPEG, WEBP, BMP, TIFF, PDF
- MRZ recognition: PNG, JPG, JPEG, WEBP, BMP, TIFF (document photos)

## Model Tiers

You can switch the model tier on the **Settings → OCR** page or at the top of the scan page. Larger tiers are more accurate but slower and consume more storage:

| Tier | Description |
|------|-------------|
| tiny | Lightweight model (~1.5MB), fast, good for simple text |
| small | Balanced tier (~30MB), the default recommendation |
| medium | High accuracy (~132MB), good for complex layouts |
| vision | Built-in system engine (macOS Vision, cannot be removed) |

Models are downloaded and installed on demand; bundled tiers ship with the app and can be installed without downloading.

<!--TIP-->
For best recognition results, ensure the image is clear, the text area occupies the main part of the frame, and lighting is even.
<!--/TIP-->

## Recognition Results

- Results are shown as text and can be copied or edited before import
- Importing creates an object (type `document`) with the recognized text saved in an **OCR Text** field (internal level)

## Related Docs

<!--CARDS-->
- [Object Management](objects.md) — Save scan results as objects
- [Object Templates](templates.md) — Passport and ID templates
- [Attachment Management](attachments.md) — Scanned image attachments
<!--/CARDS-->
