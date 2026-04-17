package crypto

import "errors"

var (
	ErrInvalidSalt          = errors.New("invalid salt: must be 32 bytes")
	ErrEmptyPassword        = errors.New("password cannot be empty")
	ErrInvalidKey           = errors.New("invalid key: must be 32 bytes")
	ErrDecryptionFailed     = errors.New("decryption failed")
	ErrInvalidCiphertext    = errors.New("invalid ciphertext")
	ErrVaultLocked          = errors.New("vault is locked")
	ErrVaultUninitialized   = errors.New("vault not initialized")
	ErrInvalidPassword      = errors.New("invalid password")
	ErrInvalidInput         = errors.New("invalid input: null pointer")
	ErrInvalidParams        = errors.New("invalid parameters")
	ErrKeyDerivationFailed  = errors.New("key derivation failed")
	ErrSaltGenerationFailed = errors.New("salt generation failed")
)
