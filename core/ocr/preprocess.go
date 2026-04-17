package ocr

import (
	"image"
	"image/color"
	"image/draw"
	"math"
)

// ImageProcessor handles image preprocessing for OCR
type ImageProcessor struct{}

// NewImageProcessor creates a new image processor
func NewImageProcessor() *ImageProcessor {
	return &ImageProcessor{}
}

// Preprocess applies preprocessing options to an image
func (p *ImageProcessor) Preprocess(img image.Image, opts *PreprocessOptions) image.Image {
	if opts == nil {
		return img
	}

	result := img

	// Apply rotation
	if opts.Rotate != 0 {
		result = p.Rotate(result, opts.Rotate)
	}

	// Apply grayscale
	if opts.Grayscale {
		result = p.Grayscale(result)
	}

	// Apply contrast adjustment
	if opts.Contrast != 0 && opts.Contrast != 1.0 {
		result = p.AdjustContrast(result, opts.Contrast)
	}

	// Apply brightness adjustment
	if opts.Brightness != 0 && opts.Brightness != 1.0 {
		result = p.AdjustBrightness(result, opts.Brightness)
	}

	// Apply denoise
	if opts.Denoise {
		result = p.Denoise(result)
	}

	// Apply crop
	if opts.CropRegion != nil {
		result = p.Crop(result, opts.CropRegion)
	}

	return result
}

// Rotate rotates an image by the given angle (0, 90, 180, 270)
func (p *ImageProcessor) Rotate(img image.Image, angle int) image.Image {
	// Normalize angle
	angle = angle % 360
	if angle < 0 {
		angle += 360
	}

	if angle == 0 {
		return img
	}

	bounds := img.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()

	var newWidth, newHeight int
	switch angle {
	case 90, 270:
		newWidth = height
		newHeight = width
	case 180:
		newWidth = width
		newHeight = height
	default:
		return img
	}

	result := image.NewRGBA(image.Rect(0, 0, newWidth, newHeight))

	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			r, g, b, a := img.At(x+bounds.Min.X, y+bounds.Min.Y).RGBA()
			var newX, newY int
			switch angle {
			case 90:
				newX = height - 1 - y
				newY = x
			case 180:
				newX = width - 1 - x
				newY = height - 1 - y
			case 270:
				newX = y
				newY = width - 1 - x
			}
			result.Set(newX, newY, color.RGBA{
				R: uint8(r >> 8),
				G: uint8(g >> 8),
				B: uint8(b >> 8),
				A: uint8(a >> 8),
			})
		}
	}

	return result
}

// Grayscale converts an image to grayscale
func (p *ImageProcessor) Grayscale(img image.Image) image.Image {
	bounds := img.Bounds()
	result := image.NewGray16(bounds)
	draw.Draw(result, bounds, img, bounds.Min, draw.Src)
	return result
}

// AdjustContrast adjusts the contrast of an image
func (p *ImageProcessor) AdjustContrast(img image.Image, factor float32) image.Image {
	bounds := img.Bounds()
	result := image.NewRGBA(bounds)

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			r, g, b, a := img.At(x, y).RGBA()
			rf := float32(r>>8) / 255.0
			gf := float32(g>>8) / 255.0
			bf := float32(b>>8) / 255.0

			// Apply contrast formula: (value - 0.5) * factor + 0.5
			rf = (rf - 0.5) * float32(factor) + 0.5
			gf = (gf - 0.5) * float32(factor) + 0.5
			bf = (bf - 0.5) * float32(factor) + 0.5

			// Clamp to [0, 1]
			rf = float32(math.Max(0, math.Min(1, float64(rf))))
			gf = float32(math.Max(0, math.Min(1, float64(gf))))
			bf = float32(math.Max(0, math.Min(1, float64(bf))))

			result.Set(x, y, color.RGBA{
				R: uint8(rf * 255),
				G: uint8(gf * 255),
				B: uint8(bf * 255),
				A: uint8(a >> 8),
			})
		}
	}

	return result
}

// AdjustBrightness adjusts the brightness of an image
func (p *ImageProcessor) AdjustBrightness(img image.Image, factor float32) image.Image {
	bounds := img.Bounds()
	result := image.NewRGBA(bounds)

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			r, g, b, a := img.At(x, y).RGBA()
			rf := float32(r>>8) / 255.0
			gf := float32(g>>8) / 255.0
			bf := float32(b>>8) / 255.0

			rf = rf * float32(factor)
			gf = gf * float32(factor)
			bf = bf * float32(factor)

			rf = float32(math.Max(0, math.Min(1, float64(rf))))
			gf = float32(math.Max(0, math.Min(1, float64(gf))))
			bf = float32(math.Max(0, math.Min(1, float64(bf))))

			result.Set(x, y, color.RGBA{
				R: uint8(rf * 255),
				G: uint8(gf * 255),
				B: uint8(bf * 255),
				A: uint8(a >> 8),
			})
		}
	}

	return result
}

// Denoise applies a simple box blur to reduce noise
func (p *ImageProcessor) Denoise(img image.Image) image.Image {
	bounds := img.Bounds()
	result := image.NewRGBA(bounds)

	kernelSize := 3
	offset := kernelSize / 2

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			var sumR, sumG, sumB, count float32

			for ky := -offset; ky <= offset; ky++ {
				for kx := -offset; kx <= offset; kx++ {
					px := x + kx
					py := y + ky
					if px >= bounds.Min.X && px < bounds.Max.X && py >= bounds.Min.Y && py < bounds.Max.Y {
						r, g, b, _ := img.At(px, py).RGBA()
						sumR += float32(r >> 8)
						sumG += float32(g >> 8)
						sumB += float32(b >> 8)
						count++
					}
				}
			}

			result.Set(x, y, color.RGBA{
				R: uint8(sumR / count),
				G: uint8(sumG / count),
				B: uint8(sumB / count),
				A: 255,
			})
		}
	}

	return result
}

// Crop crops an image to the specified region
func (p *ImageProcessor) Crop(img image.Image, region *Rectangle) image.Image {
	bounds := img.Bounds()

	// Ensure region is within bounds
	x := region.X
	if x < bounds.Min.X {
		x = bounds.Min.X
	}
	y := region.Y
	if y < bounds.Min.Y {
		y = bounds.Min.Y
	}
	width := region.Width
	if x+width > bounds.Max.X {
		width = bounds.Max.X - x
	}
	height := region.Height
	if y+height > bounds.Max.Y {
		height = bounds.Max.Y - y
	}

	cropRect := image.Rect(x, y, x+width, y+height)
	result := image.NewRGBA(cropRect)
	draw.Draw(result, cropRect, img, image.Point{X: x, Y: y}, draw.Src)
	return result
}

// AutoDetectOrientation attempts to detect the orientation of text in an image
// Returns the recommended rotation angle (0, 90, 180, or 270)
func (p *ImageProcessor) AutoDetectOrientation(img image.Image) int {
	bounds := img.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()

	// Simple heuristic: check if image is wider than tall (typically landscape)
	// and if height > width, it might need 90 degree rotation
	if float64(height) > float64(width)*1.5 {
		return 90
	}
	if float64(width) > float64(height)*1.5 {
		return 0
	}

	// Default to 0 (no rotation)
	return 0
}
