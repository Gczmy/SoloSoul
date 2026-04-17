package crypto

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"io"
)

const (
	AES256KeyLen      = 32
	AES256GCMNonceLen = 12
)

// EncryptedBlob is the format stored on disk
type EncryptedBlob struct {
	Nonce      [AES256GCMNonceLen]byte
	Ciphertext []byte
}

// Encrypt encrypts plaintext with AES-256-GCM
// Key must be exactly 32 bytes
func Encrypt(key, plaintext []byte) (*EncryptedBlob, error) {
	if len(key) != AES256KeyLen {
		return nil, ErrInvalidKey
	}

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	nonce := make([]byte, AES256GCMNonceLen)
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}

	ciphertext := gcm.Seal(nil, nonce, plaintext, nil)

	blob := &EncryptedBlob{
		Ciphertext: ciphertext,
	}
	copy(blob.Nonce[:], nonce)

	return blob, nil
}

// Decrypt decrypts an EncryptedBlob
func Decrypt(key []byte, blob *EncryptedBlob) ([]byte, error) {
	if len(key) != AES256KeyLen {
		return nil, ErrInvalidKey
	}

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	plaintext, err := gcm.Open(nil, blob.Nonce[:], blob.Ciphertext, nil)
	if err != nil {
		return nil, ErrDecryptionFailed
	}

	return plaintext, nil
}

// EncryptToBytes encrypts and returns raw bytes (nonce || ciphertext)
// Useful for compact storage
func EncryptToBytes(key, plaintext []byte) ([]byte, error) {
	blob, err := Encrypt(key, plaintext)
	if err != nil {
		return nil, err
	}

	result := make([]byte, AES256GCMNonceLen+len(blob.Ciphertext))
	copy(result, blob.Nonce[:])
	copy(result[AES256GCMNonceLen:], blob.Ciphertext)

	return result, nil
}

// DecryptFromBytes decrypts raw bytes (nonce || ciphertext)
func DecryptFromBytes(key, data []byte) ([]byte, error) {
	if len(data) < AES256GCMNonceLen {
		return nil, ErrInvalidCiphertext
	}

	blob := &EncryptedBlob{
		Nonce:      [AES256GCMNonceLen]byte{},
		Ciphertext: data[AES256GCMNonceLen:],
	}
	copy(blob.Nonce[:], data[:AES256GCMNonceLen])

	return Decrypt(key, blob)
}
