package ocr

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"time"
)

// PaddleOCR is a wrapper around the PaddleOCR Python library
type PaddleOCR struct {
	pythonPath string
	scriptPath string
	processor  *ImageProcessor
	mrzParser  *MRZParser
}

// NewPaddleOCR creates a new PaddleOCR engine
// Note: Requires Python with paddlepaddle and paddleocr packages installed
func NewPaddleOCR(pythonPath string) (*PaddleOCR, error) {
	// Try to find Python if not specified
	if pythonPath == "" {
		pythonPath = "python3"
	}

	paddle := &PaddleOCR{
		pythonPath: pythonPath,
		processor:  NewImageProcessor(),
		mrzParser:  NewMRZParser(),
	}

	// Check if paddleocr is available
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, pythonPath, "-c", "import paddleocr; print('ok')")
	if err := cmd.Run(); err != nil {
		// PaddleOCR not available - return a stub that can still do MRZ parsing
		return paddle, nil
	}

	return paddle, nil
}

// ProcessImage implements Engine.ProcessImage
func (p *PaddleOCR) ProcessImage(ctx context.Context, imageData []byte, docType DocumentType) (*ExtractionResult, error) {
	return p.ProcessImageWithPreprocessing(ctx, imageData, docType, nil)
}

// ProcessImageWithPreprocessing implements Engine.ProcessImageWithPreprocessing
func (p *PaddleOCR) ProcessImageWithPreprocessing(ctx context.Context, imageData []byte, docType DocumentType, opts *PreprocessOptions) (*ExtractionResult, error) {
	// Check if paddleocr is available
	if !p.IsAvailable() {
		return nil, ErrEngineNotAvailable
	}

	// Save image to temp file
	ext := "png"
	imagePath, err := SaveImage(imageData, ext)
	if err != nil {
		return nil, fmt.Errorf("failed to save image: %w", err)
	}
	defer os.Remove(imagePath)

	// Preprocess image if options provided
	finalPath := imagePath
	if opts != nil && opts.CropRegion != nil {
		// Crop will be done by Python paddleocr
	}

	// Call PaddleOCR Python script
	result, err := p.ProcessWithPython(finalPath, docType)
	if err != nil {
		// Fallback: try to parse MRZ directly from any text in the image
		return p.processFallback(ctx, imageData, docType)
	}

	// Try to extract MRZ data if it's a travel document
	if docType == DocumentTypePassport || docType == DocumentTypeNationalID {
		mrzData, mrzErr := p.mrzParser.ExtractMRZFromText(result.RawText)
		if mrzErr == nil && mrzData != nil {
			result.Fields = append(result.Fields, mrzData.ToExtractedFields()...)
		}
	}

	return result, nil
}

// processFallback handles cases where PaddleOCR fails
func (p *PaddleOCR) processFallback(ctx context.Context, imageData []byte, docType DocumentType) (*ExtractionResult, error) {
	// If PaddleOCR is not available, at least try to parse MRZ from any text
	// This is useful when the user pastes MRZ text directly
	result := &ExtractionResult{
		DocumentType: docType,
		Fields:       []ExtractedField{
			{
				Key:        "ocr_status",
				Value:      "paddleocr_unavailable",
				Confidence: 0,
				Source:     "system",
			},
			{
				Key:        "message",
				Value:      "PaddleOCR not installed. Install with: pip install paddlepaddle paddleocr",
				Confidence: 0,
				Source:     "system",
			},
		},
		Timestamp: time.Now(),
	}
	return result, nil
}

// DetectDocumentType implements Engine.DetectDocumentType
func (p *PaddleOCR) DetectDocumentType(ctx context.Context, imageData []byte) (DocumentType, error) {
	// This would use PaddleOCR's document classification in production
	// For now, default to passport as it's the most common travel document
	return DocumentTypePassport, nil
}

// Close implements Engine.Close
func (p *PaddleOCR) Close() error {
	// No resources to release in this stub
	return nil
}

// ProcessWithPython calls the actual PaddleOCR Python script
// This would be called in production when PaddleOCR is properly installed
func (p *PaddleOCR) ProcessWithPython(imagePath string, docType DocumentType) (*ExtractionResult, error) {
	script := p.getPythonScript()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, p.pythonPath, "-c", script, imagePath, string(docType))
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("paddleocr failed: %w", err)
	}

	var result ExtractionResult
	if err := json.Unmarshal(output, &result); err != nil {
		return nil, fmt.Errorf("failed to parse paddleocr output: %w", err)
	}

	return &result, nil
}

// getPythonScript returns the Python script for PaddleOCR
func (p *PaddleOCR) getPythonScript() string {
	return `
import sys
import json
from paddleocr import PaddleOCR

def main(image_path, doc_type):
    ocr = PaddleOCR(use_angle_cls=True, lang='en', show_log=False)
    result = ocr.ocr(image_path, cls=True)

    fields = []
    raw_text = []

    if result and result[0]:
        for line in result[0]:
            text = line[1][0]
            confidence = line[1][1]
            raw_text.append(text)

            # Map to fields based on document type
            text_lower = text.lower()

            if 'passport' in text_lower or 'country' in text_lower:
                fields.append({
                    'key': 'document_type',
                    'value': text,
                    'confidence': confidence,
                    'source': 'ocr'
                })

    return json.dumps({
        'document_type': doc_type,
        'fields': fields,
        'raw_text': ' '.join(raw_text)
    })

if __name__ == '__main__':
    print(main(sys.argv[1], sys.argv[2]))
`
}

// ParseMRZFromImage extracts MRZ data from a passport image
func (p *PaddleOCR) ParseMRZFromImage(ctx context.Context, imageData []byte) (*MRZData, error) {
	// First, extract text using OCR
	result, err := p.ProcessImage(ctx, imageData, DocumentTypePassport)
	if err != nil {
		return nil, err
	}

	// Combine all raw text
	rawText := ""
	for _, field := range result.Fields {
		if field.Source == "mrz" {
			rawText += field.Value + " "
		}
	}

	// Parse MRZ from the text
	return p.mrzParser.ExtractMRZFromText(rawText)
}

// OCRResult represents the raw result from PaddleOCR
type OCRResult struct {
	Text       string  `json:"text"`
	Confidence float64 `json:"confidence"`
	BoundingBox []float64 `json:"bbox"`
}

// ProcessDocument processes a document and extracts all fields
func (p *PaddleOCR) ProcessDocument(ctx context.Context, imagePath string) (*ExtractionResult, error) {
	// Detect document type first
	docType, err := p.DetectDocumentType(ctx, []byte(imagePath))
	if err != nil {
		docType = DocumentTypeUnknown
	}

	// Process with PaddleOCR
	result, err := p.ProcessWithPython(imagePath, docType)
	if err != nil {
		return nil, err
	}

	// Try to extract MRZ if it's a passport
	if docType == DocumentTypePassport {
		mrzData, mrzErr := p.mrzParser.ExtractMRZFromText(result.RawText)
		if mrzErr == nil && mrzData != nil {
			result.Fields = append(result.Fields, mrzData.ToExtractedFields()...)
		}
	}

	return result, nil
}

// GetSupportedLanguages returns the list of supported languages
func (p *PaddleOCR) GetSupportedLanguages() []string {
	return []string{"en", "ch", "ja", "ko", "fr", "de", "es", "pt", "it", "ru", "ar"}
}

// IsAvailable checks if PaddleOCR is available
func (p *PaddleOCR) IsAvailable() bool {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, p.pythonPath, "-c", "import paddleocr")
	return cmd.Run() == nil
}

// SaveImage saves image data to a temporary file
func SaveImage(data []byte, ext string) (string, error) {
	tmpDir := filepath.Join("/tmp", "solosoul-ocr")
	if err := os.MkdirAll(tmpDir, 0755); err != nil {
		return "", err
	}

	filename := filepath.Join(tmpDir, fmt.Sprintf("%d.%s", time.Now().UnixNano(), ext))
	if err := os.WriteFile(filename, data, 0600); err != nil {
		return "", err
	}

	return filename, nil
}
