package crypto

import (
	"bytes"
	"crypto/rand"
	"testing"
)

func TestDeriveKey(t *testing.T) {
	tests := []struct {
		name      string
		password  string
		salt      []byte
		wantErr   bool
		errType   error
	}{
		{
			name:     "valid password and salt",
			password: "testpassword123",
			salt:     make([]byte, Argon2SaltLen),
		},
		{
			name:     "empty password",
			password: "",
			salt:     make([]byte, Argon2SaltLen),
			wantErr:  true,
			errType:  ErrEmptyPassword,
		},
		{
			name:     "invalid salt length",
			password: "testpassword",
			salt:     make([]byte, 16), // should be 32
			wantErr:  true,
			errType:  ErrInvalidSalt,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Fill salt with random bytes for valid cases
			if !tt.wantErr {
				if _, err := rand.Read(tt.salt); err != nil {
					t.Fatalf("Failed to fill salt with random bytes: %v", err)
				}
			}

			got, err := DeriveKey(tt.password, tt.salt)
			if tt.wantErr {
				if err != tt.errType {
					t.Errorf("DeriveKey() error = %v, wantErrType %v", err, tt.errType)
				}
				return
			}
			if err != nil {
				t.Fatalf("DeriveKey() unexpected error: %v", err)
			}
			if len(got) != Argon2KeyLen {
				t.Errorf("DeriveKey() key length = %d, want %d", len(got), Argon2KeyLen)
			}
		})
	}
}

func TestGenerateSalt(t *testing.T) {
	salt1, err := GenerateSalt()
	if err != nil {
		t.Fatalf("GenerateSalt() failed: %v", err)
	}

	if len(salt1) != Argon2SaltLen {
		t.Errorf("GenerateSalt() length = %d, want %d", len(salt1), Argon2SaltLen)
	}

	// Generate another salt and ensure they're different (random)
	salt2, err := GenerateSalt()
	if err != nil {
		t.Fatalf("GenerateSalt() second call failed: %v", err)
	}

	if bytes.Equal(salt1, salt2) {
		t.Error("GenerateSalt() produced identical salts")
	}
}

func TestVerifyPassword(t *testing.T) {
	password := "correcthorsebatterystaple"
	salt, err := GenerateSalt()
	if err != nil {
		t.Fatalf("GenerateSalt() failed: %v", err)
	}

	expectedKey, err := DeriveKey(password, salt)
	if err != nil {
		t.Fatalf("DeriveKey() failed: %v", err)
	}

	tests := []struct {
		name       string
		password   string
		wantMatch  bool
		wantErr    bool
	}{
		{
			name:      "correct password",
			password:  password,
			wantMatch: true,
		},
		{
			name:      "wrong password",
			password:  "wrongpassword",
			wantMatch: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			match, err := VerifyPassword(tt.password, salt, expectedKey)
			if tt.wantErr && err == nil {
				t.Error("VerifyPassword() expected error, got nil")
			}
			if !tt.wantErr && err != nil {
				t.Errorf("VerifyPassword() unexpected error: %v", err)
			}
			if match != tt.wantMatch {
				t.Errorf("VerifyPassword() = %v, want %v", match, tt.wantMatch)
			}
		})
	}
}

func TestDeriveKey_Deterministic(t *testing.T) {
	password := "testpassword"
	salt := make([]byte, Argon2SaltLen)
	if _, err := rand.Read(salt); err != nil {
		t.Fatalf("Failed to fill salt with random bytes: %v", err)
	}

	key1, err := DeriveKey(password, salt)
	if err != nil {
		t.Fatalf("DeriveKey() first call failed: %v", err)
	}

	key2, err := DeriveKey(password, salt)
	if err != nil {
		t.Fatalf("DeriveKey() second call failed: %v", err)
	}

	if !bytes.Equal(key1, key2) {
		t.Error("DeriveKey() should be deterministic with same inputs")
	}
}

func TestDeriveKey_DifferentSaltsProduceDifferentKeys(t *testing.T) {
	password := "testpassword"
	salt1 := make([]byte, Argon2SaltLen)
	salt2 := make([]byte, Argon2SaltLen)

	if _, err := rand.Read(salt1); err != nil {
		t.Fatalf("Failed to fill salt1: %v", err)
	}
	if _, err := rand.Read(salt2); err != nil {
		t.Fatalf("Failed to fill salt2: %v", err)
	}

	key1, err := DeriveKey(password, salt1)
	if err != nil {
		t.Fatalf("DeriveKey() with salt1 failed: %v", err)
	}

	key2, err := DeriveKey(password, salt2)
	if err != nil {
		t.Fatalf("DeriveKey() with salt2 failed: %v", err)
	}

	if bytes.Equal(key1, key2) {
		t.Error("DeriveKey() should produce different keys for different salts")
	}
}

func TestDeriveKey_DifferentPasswordsProduceDifferentKeys(t *testing.T) {
	salt := make([]byte, Argon2SaltLen)
	if _, err := rand.Read(salt); err != nil {
		t.Fatalf("Failed to fill salt: %v", err)
	}

	key1, err := DeriveKey("password1", salt)
	if err != nil {
		t.Fatalf("DeriveKey() with password1 failed: %v", err)
	}

	key2, err := DeriveKey("password2", salt)
	if err != nil {
		t.Fatalf("DeriveKey() with password2 failed: %v", err)
	}

	if bytes.Equal(key1, key2) {
		t.Error("DeriveKey() should produce different keys for different passwords")
	}
}
