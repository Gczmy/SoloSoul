package crypto

// This file is always compiled - it provides the base types and selection logic
// Actual implementation is selected at runtime based on build tags and environment

import (
	"crypto/subtle"
	"os"
)

// Argon2 parameters - can be overridden via environment for dev/prod tradeoffs
var (
	// Argon2Secure enables production-grade security (64MiB, 3 iterations, 4 parallelism)
	// Set SOLOSOUL_SECURE=1 in production for full security
	Argon2Secure = os.Getenv("SOLOSOUL_SECURE") == "1"

	// UseRust indicates whether to use Rust implementation for better performance
	// Set SOLOSOUL_USE_RUST=1 to enable Rust (requires building crypto-argon2 first)
	UseRust = os.Getenv("SOLOSOUL_USE_RUST") == "1"

	Argon2Memory      uint32 = getMemorySetting()
	Argon2Iterations  uint32 = getIterationsSetting()
	Argon2Parallelism uint32 = getParallelismSetting()
	Argon2SaltLen            = 32
	Argon2KeyLen             = 32 // 256 bits for AES-256
)

func getMemorySetting() uint32 {
	if Argon2Secure {
		return 64 * 1024 // 64 MiB - production grade (OWASP recommended)
	}
	return 8 * 1024 // 8 MiB - balanced for Apple Silicon
}

func getIterationsSetting() uint32 {
	if Argon2Secure {
		return 3 // OWASP recommended
	}
	return 2 // Balanced for speed
}

func getParallelismSetting() uint32 {
	if Argon2Secure {
		return 4 // Match CPU cores
	}
	return 4
}

// DeriveKey derives a 256-bit key from master password using Argon2id
// Implementation is selected at runtime based on available build
func DeriveKey(password string, salt []byte) ([]byte, error) {
	if len(salt) != Argon2SaltLen {
		return nil, ErrInvalidSalt
	}
	if len(password) == 0 {
		return nil, ErrEmptyPassword
	}

	return deriveKeyImpl([]byte(password), salt)
}

// GenerateSalt generates a random salt for key derivation
func GenerateSalt() ([]byte, error) {
	return generateSaltImpl()
}

// VerifyPassword checks if a password matches the expected key derived from a salt
// Uses constant-time comparison to prevent timing attacks
func VerifyPassword(password string, salt, expectedKey []byte) (bool, error) {
	derivedKey, err := DeriveKey(password, salt)
	if err != nil {
		return false, err
	}

	if subtle.ConstantTimeCompare(derivedKey, expectedKey) == 1 {
		return true, nil
	}
	return false, nil
}

// GetSecurityLevel returns current security level description
func GetSecurityLevel() string {
	impl := getImplementationName()

	if Argon2Secure {
		return impl + " - production (64MiB, 3 iterations, 4 parallelism)"
	}
	return impl + " - development (8MiB, 2 iterations, 4 parallelism)"
}
