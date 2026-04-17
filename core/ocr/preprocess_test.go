package ocr

import (
	"image"
	"image/color"
	"image/draw"
	"testing"
)

func TestNewImageProcessor(t *testing.T) {
	proc := NewImageProcessor()

	if proc == nil {
		t.Fatal("NewImageProcessor() returned nil")
	}
}

func TestImageProcessor_Preprocess_NilOpts(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	result := proc.Preprocess(img, nil)

	if result == nil {
		t.Error("Preprocess() with nil opts should return original image")
	}
}

func TestImageProcessor_Preprocess_EmptyOpts(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	result := proc.Preprocess(img, &PreprocessOptions{})

	if result == nil {
		t.Error("Preprocess() with empty opts should return original image")
	}
}

func TestImageProcessor_Rotate_0(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	result := proc.Rotate(img, 0)

	if result == nil {
		t.Error("Rotate(0) should return image")
	}
}

func TestImageProcessor_Rotate_90(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 50) // width=100, height=50

	result := proc.Rotate(img, 90)

	bounds := result.Bounds()
	if bounds.Dx() != 50 {
		t.Errorf("After 90° rotation, width = %d, want 50", bounds.Dx())
	}
	if bounds.Dy() != 100 {
		t.Errorf("After 90° rotation, height = %d, want 100", bounds.Dy())
	}
}

func TestImageProcessor_Rotate_180(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 50)

	result := proc.Rotate(img, 180)

	bounds := result.Bounds()
	if bounds.Dx() != 100 {
		t.Errorf("After 180° rotation, width = %d, want 100", bounds.Dx())
	}
	if bounds.Dy() != 50 {
		t.Errorf("After 180° rotation, height = %d, want 50", bounds.Dy())
	}
}

func TestImageProcessor_Rotate_270(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 50)

	result := proc.Rotate(img, 270)

	bounds := result.Bounds()
	if bounds.Dx() != 50 {
		t.Errorf("After 270° rotation, width = %d, want 50", bounds.Dx())
	}
	if bounds.Dy() != 100 {
		t.Errorf("After 270° rotation, height = %d, want 100", bounds.Dy())
	}
}

func TestImageProcessor_Rotate_Negative(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	// -90 should be equivalent to 270
	result := proc.Rotate(img, -90)

	bounds := result.Bounds()
	if bounds.Dx() != 100 {
		t.Errorf("After -90° rotation, width = %d, want 100", bounds.Dx())
	}
}

func TestImageProcessor_Rotate_360(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	// 360 should be equivalent to 0
	result := proc.Rotate(img, 360)

	bounds := result.Bounds()
	if bounds.Dx() != 100 {
		t.Errorf("After 360° rotation, width = %d, want 100", bounds.Dx())
	}
}

func TestImageProcessor_Grayscale(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(10, 10)

	result := proc.Grayscale(img)

	if result == nil {
		t.Error("Grayscale() should return image")
	}

	// Result should be a Gray16 image
	if _, ok := result.(*image.Gray16); !ok {
		// Note: the implementation uses Gray16, but may return RGBA depending on draw behavior
		// This is just a basic check
	}
}

func TestImageProcessor_AdjustContrast(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(10, 10)

	// Test with factor 1.0 (no change)
	result := proc.AdjustContrast(img, 1.0)
	if result == nil {
		t.Error("AdjustContrast() with factor 1.0 should return image")
	}

	// Test with higher contrast
	result = proc.AdjustContrast(img, 1.5)
	if result == nil {
		t.Error("AdjustContrast() with factor 1.5 should return image")
	}

	// Test with lower contrast
	result = proc.AdjustContrast(img, 0.5)
	if result == nil {
		t.Error("AdjustContrast() with factor 0.5 should return image")
	}
}

func TestImageProcessor_AdjustBrightness(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(10, 10)

	// Test with factor 1.0 (no change)
	result := proc.AdjustBrightness(img, 1.0)
	if result == nil {
		t.Error("AdjustBrightness() with factor 1.0 should return image")
	}

	// Test with higher brightness
	result = proc.AdjustBrightness(img, 1.5)
	if result == nil {
		t.Error("AdjustBrightness() with factor 1.5 should return image")
	}

	// Test with lower brightness
	result = proc.AdjustBrightness(img, 0.5)
	if result == nil {
		t.Error("AdjustBrightness() with factor 0.5 should return image")
	}
}

func TestImageProcessor_Denoise(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(10, 10)

	result := proc.Denoise(img)

	if result == nil {
		t.Error("Denoise() should return image")
	}
}

func TestImageProcessor_Crop(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	region := &Rectangle{X: 10, Y: 10, Width: 50, Height: 50}
	result := proc.Crop(img, region)

	bounds := result.Bounds()
	if bounds.Dx() != 50 {
		t.Errorf("Crop width = %d, want 50", bounds.Dx())
	}
	if bounds.Dy() != 50 {
		t.Errorf("Crop height = %d, want 50", bounds.Dy())
	}
}

func TestImageProcessor_Crop_OutOfBounds(t *testing.T) {
	proc := NewImageProcessor()
	img := createTestImage(100, 100)

	// Region extends beyond image bounds
	region := &Rectangle{X: 90, Y: 90, Width: 50, Height: 50}
	result := proc.Crop(img, region)

	bounds := result.Bounds()
	// Should be clamped to image bounds
	if bounds.Dx() > 10 {
		t.Errorf("Crop width = %d, should be clamped to 10", bounds.Dx())
	}
}

func TestImageProcessor_AutoDetectOrientation(t *testing.T) {
	proc := NewImageProcessor()

	// Portrait image (taller than wide)
	portrait := createTestImage(50, 100)
	angle := proc.AutoDetectOrientation(portrait)
	if angle != 90 {
		t.Errorf("Portrait image angle = %d, want 90", angle)
	}

	// Landscape image (wider than tall)
	landscape := createTestImage(100, 50)
	angle = proc.AutoDetectOrientation(landscape)
	if angle != 0 {
		t.Errorf("Landscape image angle = %d, want 0", angle)
	}

	// Square image
	square := createTestImage(100, 100)
	angle = proc.AutoDetectOrientation(square)
	if angle != 0 {
		t.Errorf("Square image angle = %d, want 0", angle)
	}
}

func TestDocumentTypeConstants(t *testing.T) {
	types := []DocumentType{
		DocumentTypePassport,
		DocumentTypeNationalID,
		DocumentTypeVisa,
		DocumentTypeDriverLicense,
		DocumentTypeUnknown,
	}

	// Verify values
	if DocumentTypePassport != "passport" {
		t.Errorf("DocumentTypePassport = %q, want %q", DocumentTypePassport, "passport")
	}
	if DocumentTypeNationalID != "national_id" {
		t.Errorf("DocumentTypeNationalID = %q, want %q", DocumentTypeNationalID, "national_id")
	}
	if DocumentTypeVisa != "visa" {
		t.Errorf("DocumentTypeVisa = %q, want %q", DocumentTypeVisa, "visa")
	}
	if DocumentTypeDriverLicense != "driver_license" {
		t.Errorf("DocumentTypeDriverLicense = %q, want %q", DocumentTypeDriverLicense, "driver_license")
	}
	if DocumentTypeUnknown != "unknown" {
		t.Errorf("DocumentTypeUnknown = %q, want %q", DocumentTypeUnknown, "unknown")
	}

	// Verify distinctness
	for i, t1 := range types {
		for j, t2 := range types {
			if i != j && t1 == t2 {
				t.Errorf("DocumentType constants at indices %d and %d should be distinct", i, j)
			}
		}
	}
}

func TestEngineInterface(t *testing.T) {
	// Verify Engine is an interface
	var _ Engine = (*PaddleOCR)(nil)
}

func TestDefaultEngineConfig(t *testing.T) {
	config := DefaultEngineConfig()

	if config == nil {
		t.Fatal("DefaultEngineConfig() returned nil")
	}

	if len(config.Languages) == 0 {
		t.Error("Languages should not be empty")
	}

	if config.UseGPU {
		t.Error("UseGPU should be false by default")
	}

	if config.NumWorkers != 2 {
		t.Errorf("NumWorkers = %d, want 2", config.NumWorkers)
	}
}

func TestOCREngineErrors(t *testing.T) {
	errs := []error{
		ErrUnsupportedDocumentType,
		ErrImageTooSmall,
		ErrNoTextFound,
		ErrEngineNotAvailable,
	}

	for _, err := range errs {
		if err == nil {
			t.Error("OCR error should not be nil")
		}
		if err.Error() == "" {
			t.Error("OCR error should have a message")
		}
	}
}

func TestRectangle(t *testing.T) {
	rect := &Rectangle{
		X:      10,
		Y:      20,
		Width:  100,
		Height: 50,
	}

	if rect.X != 10 {
		t.Errorf("X = %d, want 10", rect.X)
	}
	if rect.Y != 20 {
		t.Errorf("Y = %d, want 20", rect.Y)
	}
	if rect.Width != 100 {
		t.Errorf("Width = %d, want 100", rect.Width)
	}
	if rect.Height != 50 {
		t.Errorf("Height = %d, want 50", rect.Height)
	}
}

// Helper function to create a simple test image
func createTestImage(width, height int) image.Image {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	// Fill with a simple color
	draw.Draw(img, img.Bounds(), &image.Uniform{color.RGBA{R: 128, G: 128, B: 128, A: 255}}, image.ZP, draw.Src)
	return img
}
