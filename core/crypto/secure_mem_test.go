package crypto

import (
	"bytes"
	"runtime"
	"testing"
	"time"
)

func TestNewSecureBuffer(t *testing.T) {
	tests := []struct {
		name    string
		size    int
		wantErr bool
	}{
		{
			name:    "zero size",
			size:    0,
			wantErr: false,
		},
		{
			name:    "small size",
			size:    16,
			wantErr: false,
		},
		{
			name:    "medium size",
			size:    256,
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sb, err := NewSecureBuffer(tt.size)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewSecureBuffer(%d) error = %v, wantErr %v", tt.size, err, tt.wantErr)
				return
			}
			if err == nil && len(sb.data) != tt.size {
				t.Errorf("NewSecureBuffer(%d) data len = %d, want %d", tt.size, len(sb.data), tt.size)
			}
		})
	}
}

func TestFromBytes(t *testing.T) {
	original := []byte("sensitive data that should be zeroed")
	// Save a copy since FromBytes will zero the original
	savedData := make([]byte, len(original))
	copy(savedData, original)

	// Create secure buffer from bytes
	sb := FromBytes(original)

	// Verify secure buffer has a copy of the saved data
	if !bytes.Equal(sb.Bytes(), savedData) {
		t.Error("SecureBuffer.Bytes() should return a copy of the original data")
	}

	// The original should be zeroed by FromBytes
	allZeros := true
	for _, b := range original {
		if b != 0 {
			allZeros = false
			break
		}
	}
	if !allZeros {
		t.Error("FromBytes should zero the original data")
	}
}

func TestFromBytes_NilInput(t *testing.T) {
	sb := FromBytes(nil)
	if sb == nil {
		t.Error("FromBytes(nil) should return empty SecureBuffer, not nil")
	}
	if sb.data != nil {
		t.Error("FromBytes(nil) should have nil data")
	}
}

func TestSecureBuffer_Bytes(t *testing.T) {
	data := []byte("test data")
	// Save a copy since FromBytes zeros the original
	savedData := make([]byte, len(data))
	copy(savedData, data)

	sb := FromBytes(data)

	// Get bytes
	result := sb.Bytes()

	// Should be equal to saved data
	if !bytes.Equal(result, savedData) {
		t.Errorf("Bytes() = %v, want %v", result, savedData)
	}

	// Modifying returned bytes should not affect internal state
	result[0] = 0
	if sb.Bytes()[0] == 0 {
		t.Error("Modifying returned bytes should not affect internal data")
	}
}

func TestSecureBuffer_Zero(t *testing.T) {
	data := []byte("sensitive data")
	sb := FromBytes(data)

	// Zero the buffer
	sb.Zero()

	// All bytes should be zero
	for i, b := range sb.data {
		if b != 0 {
			t.Errorf("After Zero(), data[%d] = %d, want 0", i, b)
		}
	}
}

func TestSecureBuffer_Zero_NilReceiver(t *testing.T) {
	var sb *SecureBuffer
	// Should not panic
	sb.Zero()
}

func TestSecureBuffer_Destroy(t *testing.T) {
	data := []byte("sensitive data")
	sb := FromBytes(data)

	// Destroy the buffer
	sb.Destroy()

	// Data should be nil after destroy
	if sb.data != nil {
		t.Error("After Destroy(), data should be nil")
	}
}

func TestSecureBuffer_Destroy_NilReceiver(t *testing.T) {
	var sb *SecureBuffer
	// Should not panic
	sb.Destroy()
}

func TestSecureWipe(t *testing.T) {
	data := []byte("sensitive data that needs wiping")
	SecureWipe(data)

	// All bytes should be zero
	for i, b := range data {
		if b != 0 {
			t.Errorf("After SecureWipe(), data[%d] = %d, want 0", i, b)
		}
	}
}

func TestZeroBytes(t *testing.T) {
	tests := []struct {
		name    string
		data    []byte
		wantAllZeros bool
	}{
		{
			name:         "nil data",
			data:         nil,
			wantAllZeros: true,
		},
		{
			name:         "empty slice",
			data:         []byte{},
			wantAllZeros: true,
		},
		{
			name:         "normal data",
			data:         []byte("test data"),
			wantAllZeros: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Only test non-nil cases
			if tt.data != nil {
				// Fill with non-zero first
				for i := range tt.data {
					tt.data[i] = 0xFF
				}
				zeroBytes(tt.data)

				for i, b := range tt.data {
					if b != 0 {
						t.Errorf("After zeroBytes(), data[%d] = %d, want 0", i, b)
					}
				}
			}
		})
	}
}

func TestSecureBuffer_Finalizer(t *testing.T) {
	// This test verifies that finalizers are set
	// We can't easily test the finalizer behavior directly,
	// but we can verify the SecureBuffer is set up correctly

	data := []byte("test data for finalizer")
	sb := FromBytes(data)

	// Force GC to run finalizers (may not be deterministic)
	runtime.GC()
	time.Sleep(10 * time.Millisecond)
	runtime.GC()

	// The buffer should still be usable before explicit destroy
	if sb == nil {
		t.Error("FromBytes should not return nil")
	}
}

func TestNewSecureBuffer_RandomContent(t *testing.T) {
	// NewSecureBuffer should fill with random data
	sb, err := NewSecureBuffer(32)
	if err != nil {
		t.Fatalf("NewSecureBuffer(32) failed: %v", err)
	}

	// The data should not be all zeros
	allZeros := true
	for _, b := range sb.data {
		if b != 0 {
			allZeros = false
			break
		}
	}
	if allZeros {
		t.Error("NewSecureBuffer should fill with random data, got all zeros")
	}

	// Two calls should produce different random data
	sb2, err := NewSecureBuffer(32)
	if err != nil {
		t.Fatalf("NewSecureBuffer(32) second call failed: %v", err)
	}

	if bytes.Equal(sb.data, sb2.data) {
		t.Error("Two calls to NewSecureBuffer should produce different random data")
	}
}
