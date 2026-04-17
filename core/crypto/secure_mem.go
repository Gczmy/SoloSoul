package crypto

import (
	"runtime"
)

// SecureBuffer is a byte slice that zeroizes memory when destroyed
type SecureBuffer struct {
	data []byte
}

// NewSecureBuffer creates a new secure buffer of the given size filled with random data
func NewSecureBuffer(size int) (*SecureBuffer, error) {
	data, err := generateRandomBytes(size)
	if err != nil {
		return nil, err
	}

	return &SecureBuffer{data: data}, nil
}

// FromBytes creates a SecureBuffer from existing bytes and zeros the source
func FromBytes(data []byte) *SecureBuffer {
	if data == nil {
		return &SecureBuffer{}
	}

	sb := &SecureBuffer{
		data: make([]byte, len(data)),
	}
	copy(sb.data, data)

	// Zero the source bytes
	zeroBytes(data)

	// Set finalizer to zero on GC
	runtime.SetFinalizer(sb, func(s *SecureBuffer) {
		s.Zero()
	})

	return sb
}

// Bytes returns a copy of the data (caller is responsible for zeroing if needed)
func (sb *SecureBuffer) Bytes() []byte {
	if sb == nil || sb.data == nil {
		return nil
	}
	result := make([]byte, len(sb.data))
	copy(result, sb.data)
	return result
}

// Zero zeroes the underlying memory
func (sb *SecureBuffer) Zero() {
	if sb == nil || sb.data == nil {
		return
	}
	zeroBytes(sb.data)
}

// Destroy zeroes and frees the secure buffer
func (sb *SecureBuffer) Destroy() {
	if sb == nil {
		return
	}
	sb.Zero()
	sb.data = nil
}

// SecureWipe securely wipes a byte slice
func SecureWipe(data []byte) {
	zeroBytes(data)
}

func zeroBytes(data []byte) {
	if data == nil {
		return
	}
	// volatile write prevents compiler from optimizing away the zeroing
	for i := range data {
		data[i] = 0
	}
}

