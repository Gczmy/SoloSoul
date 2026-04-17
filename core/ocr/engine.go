package ocr

import (
	"context"
	"errors"
	"time"
)

// DocumentType represents the type of document being scanned
type DocumentType string

const (
	DocumentTypePassport  DocumentType = "passport"
	DocumentTypeNationalID DocumentType = "national_id"
	DocumentTypeVisa      DocumentType = "visa"
	DocumentTypeDriverLicense DocumentType = "driver_license"
	DocumentTypeUnknown   DocumentType = "unknown"
)

// ExtractedField represents a single extracted field from a document
type ExtractedField struct {
	Key        string    `json:"key"`
	Value     string    `json:"value"`
	Confidence float32   `json:"confidence"` // 0.0 to 1.0
	Source    string    `json:"source"`     // Which part of document this came from
}

// ExtractionResult contains all extracted fields from a document
type ExtractionResult struct {
	DocumentType DocumentType   `json:"document_type"`
	Fields      []ExtractedField `json:"fields"`
	RawText     string          `json:"raw_text,omitempty"`
	ImageID     string          `json:"image_id,omitempty"`
	Timestamp   time.Time       `json:"timestamp"`
}

// OCRJob represents a pending OCR job
type OCRJob struct {
	ID          string       `json:"id"`
	Status      JobStatus    `json:"status"`
	DocumentType DocumentType `json:"document_type"`
	CreatedAt   time.Time    `json:"created_at"`
	CompletedAt *time.Time   `json:"completed_at,omitempty"`
	Result      *ExtractionResult `json:"result,omitempty"`
	Error       string       `json:"error,omitempty"`
}

// JobStatus represents the status of an OCR job
type JobStatus string

const (
	JobStatusPending   JobStatus = "pending"
	JobStatusProcessing JobStatus = "processing"
	JobStatusCompleted JobStatus = "completed"
	JobStatusFailed    JobStatus = "failed"
)

// Engine is the OCR engine interface
type Engine interface {
	// ProcessImage processes an image and extracts fields based on document type
	ProcessImage(ctx context.Context, imageData []byte, docType DocumentType) (*ExtractionResult, error)

	// ProcessImageWithPreprocessing processes an image with preprocessing options
	ProcessImageWithPreprocessing(ctx context.Context, imageData []byte, docType DocumentType, opts *PreprocessOptions) (*ExtractionResult, error)

	// DetectDocumentType attempts to detect the document type from an image
	DetectDocumentType(ctx context.Context, imageData []byte) (DocumentType, error)

	// Close releases any resources held by the engine
	Close() error
}

// PreprocessOptions contains options for image preprocessing
type PreprocessOptions struct {
	Rotate        int      `json:"rotate"`         // 0, 90, 180, 270 degrees
	Denoise      bool     `json:"denoise"`
	Contrast     float32  `json:"contrast"`       // 0.0 to 2.0, 1.0 = no change
	Brightness   float32  `json:"brightness"`     // 0.0 to 2.0, 1.0 = no change
	CropRegion    *Rectangle `json:"crop_region"`  // Optional crop region
	Grayscale    bool     `json:"grayscale"`
}

// Rectangle represents a rectangular region
type Rectangle struct {
	X      int `json:"x"`
	Y      int `json:"y"`
	Width  int `json:"width"`
	Height int `json:"height"`
}

// Common OCR errors
var (
	ErrUnsupportedDocumentType = errors.New("unsupported document type")
	ErrImageTooSmall          = errors.New("image too small for OCR")
	ErrNoTextFound           = errors.New("no text found in image")
	ErrEngineNotAvailable    = errors.New("OCR engine not available")
)

// EngineConfig contains configuration for the OCR engine
type EngineConfig struct {
	// Language models to use (e.g., "en", "zh", "ja")
	Languages []string
	// UseGPU enables GPU acceleration if available
	UseGPU bool
	// NumWorkers sets the number of parallel workers
	NumWorkers int
}

// DefaultEngineConfig returns the default engine configuration
func DefaultEngineConfig() *EngineConfig {
	return &EngineConfig{
		Languages: []string{"en"},
		UseGPU:    false,
		NumWorkers: 2,
	}
}
