# OCR & Scan

The OCR (Optical Character Recognition) feature lets you extract text from images and create objects.

## Using OCR

1. Enter **OCR Scan** from the sidebar or home page
2. Click **Select Image** and upload an image containing text
3. The system displays extracted text and confidence scores
4. Review results and correct if necessary
5. Click **Import as Object** and select the target section

<!--STEPPER Scan a passport info page-->
1. Go to the **OCR Scan** page
2. Click **Select Image** and upload a passport info page photo
3. Wait for recognition to complete
4. Review recognized fields (name, passport number, nationality, etc.)
5. Click **Import as Object** → select **Travel** section
<!--/STEPPER-->

## Supported Image Formats

- PNG, JPG, JPEG, WEBP, BMP, TIFF

## Recognized Fields

The OCR engine attempts to automatically recognize common fields:

- Names, dates, and number-type fields
- Results are presented as key-value pairs
- Fields with confidence below 80% are specially marked

<!--TIP-->
For best recognition results, ensure the image is clear, the text area occupies the main part of the frame, and lighting is even.
<!--/TIP-->

## Local File Import

In addition to OCR, you can directly import local files as objects:

1. Go to the **Local Import** page
2. Select files or folders
3. The system scans supported file types
4. Check the files you want to import
5. Choose the import method (create object or attachment)

## Privacy Notes

- OCR processing is done locally; images are not uploaded to external services
- Recognition results are saved only in your vault
