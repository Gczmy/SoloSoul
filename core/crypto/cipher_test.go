package crypto

import (
	"bytes"
	"crypto/rand"
	"testing"
)

func TestEncryptDecrypt(t *testing.T) {
	key := make([]byte, AES256KeyLen)
	if _, err := rand.Read(key); err != nil {
		t.Fatalf("Failed to generate key: %v", err)
	}

	tests := []struct {
		name      string
		plaintext []byte
		wantErr   bool
	}{
		{
			name:      "simple text",
			plaintext: []byte("Hello, World!"),
		},
		{
			name:      "empty text",
			plaintext: []byte(""),
		},
		{
			name:      "binary data",
			plaintext: []byte{0x00, 0xFF, 0x42, 0x13, 0x99},
		},
		{
			name:      "long text",
			plaintext: bytes.Repeat([]byte("A"), 10000),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			blob, err := Encrypt(key, tt.plaintext)
			if err != nil {
				t.Fatalf("Encrypt() failed: %v", err)
			}

			decrypted, err := Decrypt(key, blob)
			if err != nil {
				t.Fatalf("Decrypt() failed: %v", err)
			}

			if !bytes.Equal(decrypted, tt.plaintext) {
				t.Errorf("Decrypt() = %v, want %v", decrypted, tt.plaintext)
			}
		})
	}
}

func TestEncrypt_InvalidKey(t *testing.T) {
	tests := []struct {
		name   string
		keyLen int
	}{
		{name: "too short", keyLen: 16},
		{name: "too long", keyLen: 64},
		{name: "empty", keyLen: 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			key := make([]byte, tt.keyLen)
			_, err := Encrypt(key, []byte("test"))
			if err != ErrInvalidKey {
				t.Errorf("Encrypt() error = %v, want %v", err, ErrInvalidKey)
			}
		})
	}
}

func TestDecrypt_InvalidKey(t *testing.T) {
	blob := &EncryptedBlob{
		Nonce:      [AES256GCMNonceLen]byte{},
		Ciphertext: []byte("test"),
	}

	tests := []struct {
		name   string
		keyLen int
	}{
		{name: "too short", keyLen: 16},
		{name: "too long", keyLen: 64},
		{name: "empty", keyLen: 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			key := make([]byte, tt.keyLen)
			_, err := Decrypt(key, blob)
			if err != ErrInvalidKey {
				t.Errorf("Decrypt() error = %v, want %v", err, ErrInvalidKey)
			}
		})
	}
}

func TestDecrypt_TamperedCiphertext(t *testing.T) {
	key := make([]byte, AES256KeyLen)
	if _, err := rand.Read(key); err != nil {
		t.Fatalf("Failed to generate key: %v", err)
	}

	plaintext := []byte("sensitive data")
	blob, err := Encrypt(key, plaintext)
	if err != nil {
		t.Fatalf("Encrypt() failed: %v", err)
	}

	// Tamper with the ciphertext
	if len(blob.Ciphertext) > 0 {
		blob.Ciphertext[0] ^= 0xFF
	}

	_, err = Decrypt(key, blob)
	if err != ErrDecryptionFailed {
		t.Errorf("Decrypt() error = %v, want %v", err, ErrDecryptionFailed)
	}
}

func TestEncryptToBytesDecryptFromBytes(t *testing.T) {
	key := make([]byte, AES256KeyLen)
	if _, err := rand.Read(key); err != nil {
		t.Fatalf("Failed to generate key: %v", err)
	}

	plaintext := []byte("Compact storage test")

	encrypted, err := EncryptToBytes(key, plaintext)
	if err != nil {
		t.Fatalf("EncryptToBytes() failed: %v", err)
	}

	// Verify the format: nonce (12 bytes) + ciphertext
	if len(encrypted) < AES256GCMNonceLen {
		t.Errorf("Encrypted data too short: %d bytes", len(encrypted))
	}

	decrypted, err := DecryptFromBytes(key, encrypted)
	if err != nil {
		t.Fatalf("DecryptFromBytes() failed: %v", err)
	}

	if !bytes.Equal(decrypted, plaintext) {
		t.Errorf("Decrypted = %v, want %v", decrypted, plaintext)
	}
}

func TestDecryptFromBytes_InvalidInput(t *testing.T) {
	key := make([]byte, AES256KeyLen)

	tests := []struct {
		name  string
		data  []byte
	}{
		{name: "too short", data: []byte("tooshort")},
		{name: "empty", data: []byte("")},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := DecryptFromBytes(key, tt.data)
			if err != ErrInvalidCiphertext {
				t.Errorf("DecryptFromBytes() error = %v, want %v", err, ErrInvalidCiphertext)
			}
		})
	}
}

func TestEncryptedBlobFormat(t *testing.T) {
	key := make([]byte, AES256KeyLen)
	if _, err := rand.Read(key); err != nil {
		t.Fatalf("Failed to generate key: %v", err)
	}

	plaintext := []byte("Format test")
	blob, err := Encrypt(key, plaintext)
	if err != nil {
		t.Fatalf("Encrypt() failed: %v", err)
	}

	// Verify nonce is set
	if blob.Nonce == [12]byte{} {
		t.Error("Nonce should not be zeroed")
	}

	// Verify ciphertext is not empty
	if len(blob.Ciphertext) == 0 {
		t.Error("Ciphertext should not be empty")
	}

	// Verify ciphertext is different from plaintext (encrypted)
	if bytes.Equal(blob.Ciphertext, plaintext) {
		t.Error("Ciphertext should differ from plaintext")
	}
}

func TestUniqueNonce(t *testing.T) {
	key := make([]byte, AES256KeyLen)
	if _, err := rand.Read(key); err != nil {
		t.Fatalf("Failed to generate key: %v", err)
	}

	plaintext := []byte("Same plaintext")

	blob1, err := Encrypt(key, plaintext)
	if err != nil {
		t.Fatalf("First Encrypt() failed: %v", err)
	}

	blob2, err := Encrypt(key, plaintext)
	if err != nil {
		t.Fatalf("Second Encrypt() failed: %v", err)
	}

	// Nonces should be different
	if blob1.Nonce == blob2.Nonce {
		t.Error("Nonces should be unique for same plaintext")
	}

	// Ciphertexts should be different due to different nonces
	if bytes.Equal(blob1.Ciphertext, blob2.Ciphertext) {
		t.Error("Ciphertexts should differ due to unique nonces")
	}
}
