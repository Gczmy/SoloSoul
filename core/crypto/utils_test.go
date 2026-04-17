package crypto

import (
	"bytes"
	"testing"
)

func TestGenerateRandomBytes(t *testing.T) {
	tests := []struct {
		name    string
		n       int
		wantErr bool
	}{
		{
			name:    "zero length",
			n:       0,
			wantErr: false,
		},
		{
			name:    "small size",
			n:       16,
			wantErr: false,
		},
		{
			name:    "medium size",
			n:       256,
			wantErr: false,
		},
		{
			name:    "large size",
			n:       1024,
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := generateRandomBytes(tt.n)
			if (err != nil) != tt.wantErr {
				t.Errorf("generateRandomBytes(%d) error = %v, wantErr %v", tt.n, err, tt.wantErr)
				return
			}
			if err == nil && len(got) != tt.n {
				t.Errorf("generateRandomBytes(%d) len = %d, want %d", tt.n, len(got), tt.n)
			}
		})
	}
}

func TestGenerateRandomBytes_Unique(t *testing.T) {
	// Generate multiple random byte slices and ensure they're different
	results := make([][]byte, 10)
	for i := 0; i < 10; i++ {
		result, err := generateRandomBytes(32)
		if err != nil {
			t.Fatalf("generateRandomBytes(32) failed: %v", err)
		}
		results[i] = result
	}

	// Check that each is unique
	for i := 0; i < len(results); i++ {
		for j := i + 1; j < len(results); j++ {
			if bytes.Equal(results[i], results[j]) {
				t.Errorf("generateRandomBytes produced identical output at iterations %d and %d", i, j)
			}
		}
	}
}

func TestGenerateRandomBytes_CryptoRandom(t *testing.T) {
	// The function should use crypto/rand, so results should be unpredictable
	// We can verify this indirectly by checking that consecutive calls
	// don't follow a pattern

	b1, err := generateRandomBytes(32)
	if err != nil {
		t.Fatalf("generateRandomBytes(32) failed: %v", err)
	}

	b2, err := generateRandomBytes(32)
	if err != nil {
		t.Fatalf("generateRandomBytes(32) second call failed: %v", err)
	}

	// If crypto/rand is used properly, these should be completely different
	if bytes.Equal(b1, b2) {
		t.Error("Two consecutive calls to generateRandomBytes produced identical output")
	}
}
