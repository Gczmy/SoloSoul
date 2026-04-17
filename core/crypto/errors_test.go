package crypto

import (
	"errors"
	"testing"
)

func TestCryptoErrors(t *testing.T) {
	tests := []struct {
		name     string
		err      error
		wantMsg  string
	}{
		{
			name:    "ErrInvalidSalt",
			err:     ErrInvalidSalt,
			wantMsg: "invalid salt: must be 32 bytes",
		},
		{
			name:    "ErrEmptyPassword",
			err:     ErrEmptyPassword,
			wantMsg: "password cannot be empty",
		},
		{
			name:    "ErrInvalidKey",
			err:     ErrInvalidKey,
			wantMsg: "invalid key: must be 32 bytes",
		},
		{
			name:    "ErrDecryptionFailed",
			err:     ErrDecryptionFailed,
			wantMsg: "decryption failed",
		},
		{
			name:    "ErrInvalidCiphertext",
			err:     ErrInvalidCiphertext,
			wantMsg: "invalid ciphertext",
		},
		{
			name:    "ErrVaultLocked",
			err:     ErrVaultLocked,
			wantMsg: "vault is locked",
		},
		{
			name:    "ErrVaultUninitialized",
			err:    ErrVaultUninitialized,
			wantMsg: "vault not initialized",
		},
		{
			name:    "ErrInvalidPassword",
			err:    ErrInvalidPassword,
			wantMsg: "invalid password",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.err == nil {
				t.Error("err is nil")
				return
			}
			if tt.err.Error() != tt.wantMsg {
				t.Errorf("Error() = %q, want %q", tt.err.Error(), tt.wantMsg)
			}
		})
	}
}

func TestErrorsAreDistinct(t *testing.T) {
	errs := []error{
		ErrInvalidSalt,
		ErrEmptyPassword,
		ErrInvalidKey,
		ErrDecryptionFailed,
		ErrInvalidCiphertext,
		ErrVaultLocked,
		ErrVaultUninitialized,
		ErrInvalidPassword,
	}

	// Check each pair is distinct
	for i := 0; i < len(errs); i++ {
		for j := i + 1; j < len(errs); j++ {
			if errors.Is(errs[i], errs[j]) || errors.Is(errs[j], errs[i]) {
				if errs[i] != errs[j] {
					continue // Different error instances, ok
				}
				t.Errorf("errs[%d] and errs[%d] should be distinct errors", i, j)
			}
		}
	}
}

func TestErrorsAreSentinel(t *testing.T) {
	// Verify these are sentinel errors (can be checked with errors.Is)
	if !errors.Is(ErrInvalidSalt, ErrInvalidSalt) {
		t.Error("ErrInvalidSalt should be self-equal")
	}
	if !errors.Is(ErrEmptyPassword, ErrEmptyPassword) {
		t.Error("ErrEmptyPassword should be self-equal")
	}
	if !errors.Is(ErrInvalidKey, ErrInvalidKey) {
		t.Error("ErrInvalidKey should be self-equal")
	}
	if !errors.Is(ErrDecryptionFailed, ErrDecryptionFailed) {
		t.Error("ErrDecryptionFailed should be self-equal")
	}
}
