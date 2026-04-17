//go:build rust && cgo

package crypto

/*
#cgo darwin  LDFLAGS: -L${SRCDIR}/../../crypto-argon2/target/release -lsolosoul_crypto -lm
#cgo linux   LDFLAGS: -L${SRCDIR}/../../crypto-argon2/target/release -lsolosoul_crypto -lm
#cgo windows LDFLAGS: -L${SRCDIR}/../../crypto-argon2/target/release -lsolosoul_crypto -lm

#include <stdint.h>

extern int32_t argon2_derive_key(
    const uint8_t* password,
    size_t password_len,
    const uint8_t* salt,
    size_t salt_len,
    uint32_t memory_kib,
    uint32_t iterations,
    uint32_t parallelism,
    uint8_t* output,
    size_t output_len
);

extern int32_t argon2_generate_salt(
    uint8_t* salt,
    size_t len
);
*/
import "C"

const (
	argon2ResultOK           = 0
	argon2ResultNullPtr      = -1
	argon2ResultInvalidLen   = -2
	argon2ResultInvalidParams = -3
	argon2ResultHashFailed    = -4
)

// deriveKeyImpl uses the Rust argon2 implementation for high-performance cross-platform support
func deriveKeyImpl(password []byte, salt []byte) ([]byte, error) {
	output := make([]byte, Argon2KeyLen)

	ret := C.argon2_derive_key(
		(*C.uchar)(&password[0]),
		C.size_t(len(password)),
		(*C.uchar)(&salt[0]),
		C.size_t(len(salt)),
		C.uint32_t(Argon2Memory),
		C.uint32_t(Argon2Iterations),
		C.uint32_t(Argon2Parallelism),
		(*C.uchar)(&output[0]),
		C.size_t(len(output)),
	)

	switch ret {
	case argon2ResultOK:
		return output, nil
	case argon2ResultNullPtr:
		return nil, ErrInvalidInput
	case argon2ResultInvalidLen:
		return nil, ErrInvalidSalt
	case argon2ResultInvalidParams:
		return nil, ErrInvalidParams
	default:
		return nil, ErrKeyDerivationFailed
	}
}

// generateSaltImpl generates a 32-byte salt using Rust CSPRNG
func generateSaltImpl() ([]byte, error) {
	salt := make([]byte, Argon2SaltLen)

	ret := C.argon2_generate_salt(
		(*C.uchar)(&salt[0]),
		C.size_t(len(salt)),
	)

	switch ret {
	case argon2ResultOK:
		return salt, nil
	default:
		return nil, ErrSaltGenerationFailed
	}
}

// getImplementationName returns the name of the current implementation
func getImplementationName() string {
	return "Rust (SIMD optimized)"
}
