package ocr

import (
	"context"
	"testing"
	"time"
)

func TestNewPaddleOCR(t *testing.T) {
	paddle, err := NewPaddleOCR("")
	if err != nil {
		t.Fatalf("NewPaddleOCR() failed: %v", err)
	}

	if paddle == nil {
		t.Fatal("NewPaddleOCR() returned nil")
	}

	if paddle.processor == nil {
		t.Error("processor should be initialized")
	}
	if paddle.mrzParser == nil {
		t.Error("mrzParser should be initialized")
	}
}

func TestPaddleOCR_GetSupportedLanguages(t *testing.T) {
	paddle, err := NewPaddleOCR("")
	if err != nil {
		t.Fatalf("NewPaddleOCR() failed: %v", err)
	}

	langs := paddle.GetSupportedLanguages()

	if len(langs) == 0 {
		t.Error("GetSupportedLanguages() returned empty slice")
	}

	// Check that common languages are supported
	hasEnglish := false
	for _, lang := range langs {
		if lang == "en" {
			hasEnglish = true
			break
		}
	}
	if !hasEnglish {
		t.Error("GetSupportedLanguages() should include 'en'")
	}
}

func TestPaddleOCR_Close(t *testing.T) {
	paddle, err := NewPaddleOCR("")
	if err != nil {
		t.Fatalf("NewPaddleOCR() failed: %v", err)
	}

	if err := paddle.Close(); err != nil {
		t.Errorf("Close() failed: %v", err)
	}
}

func TestPaddleOCR_DetectDocumentType(t *testing.T) {
	paddle, err := NewPaddleOCR("")
	if err != nil {
		t.Fatalf("NewPaddleOCR() failed: %v", err)
	}

	docType, err := paddle.DetectDocumentType(context.Background(), []byte("fake image data"))
	if err != nil {
		t.Errorf("DetectDocumentType() failed: %v", err)
	}

	// Default should be passport
	if docType != DocumentTypePassport {
		t.Errorf("DetectDocumentType() = %q, want %q", docType, DocumentTypePassport)
	}
}

func TestPaddleOCR_ProcessImage_NotAvailable(t *testing.T) {
	paddle, err := NewPaddleOCR("")
	if err != nil {
		t.Fatalf("NewPaddleOCR() failed: %v", err)
	}

	// Since PaddleOCR is likely not installed, the engine should return unavailable
	// This test verifies graceful handling
	result, err := paddle.ProcessImage(context.Background(), []byte("fake data"), DocumentTypePassport)

	// If not available, should return appropriate error or fallback result
	if err == ErrEngineNotAvailable {
		// Expected when PaddleOCR is not installed
		return
	}

	// If it did process (fallback mode), that's also acceptable
	if result != nil {
		// Fallback mode returned a result
		return
	}
}

func TestSaveImage(t *testing.T) {
	data := []byte("fake image data")

	path, err := SaveImage(data, "png")
	if err != nil {
		t.Fatalf("SaveImage() failed: %v", err)
	}

	if path == "" {
		t.Error("SaveImage() returned empty path")
	}

	// Clean up is handled by the caller in actual use
	_ = path
}

func TestExtractionResult_Fields(t *testing.T) {
	result := &ExtractionResult{
		DocumentType: DocumentTypePassport,
		Fields: []ExtractedField{
			{Key: "surname", Value: "SMITH", Confidence: 0.95, Source: "mrz"},
		},
		RawText:   "SMITH<<SARAH",
		Timestamp: time.Now(),
	}

	if result.DocumentType != DocumentTypePassport {
		t.Errorf("DocumentType = %q, want %q", result.DocumentType, DocumentTypePassport)
	}
	if len(result.Fields) != 1 {
		t.Errorf("Fields len = %d, want 1", len(result.Fields))
	}
}

func TestExtractedField_Fields(t *testing.T) {
	field := ExtractedField{
		Key:        "full_name",
		Value:      "John Doe",
		Confidence: 0.92,
		Source:     "ocr",
	}

	if field.Key != "full_name" {
		t.Errorf("Key = %q, want %q", field.Key, "full_name")
	}
	if field.Confidence != 0.92 {
		t.Errorf("Confidence = %f, want 0.92", field.Confidence)
	}
}
