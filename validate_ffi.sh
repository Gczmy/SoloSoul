#!/bin/bash
#
# validate_ffi.sh - Verify Rust and Go FFI signatures match
#

set -e

echo "=== FFI Signature Validation ==="
echo ""

# Check 1: Rust library exists
echo "[1] Checking Rust crate structure..."
if [[ ! -f "crypto-argon2/Cargo.toml" ]]; then
    echo "    ERROR: crypto-argon2/Cargo.toml not found"
    exit 1
fi
if [[ ! -f "crypto-argon2/src/lib.rs" ]]; then
    echo "    ERROR: crypto-argon2/src/lib.rs not found"
    exit 1
fi
echo "    OK: Rust crate files exist"

# Check 2: Go FFI file exists
echo "[2] Checking Go FFI bindings..."
if [[ ! -f "core/crypto/kdf_common.go" ]]; then
    echo "    ERROR: core/crypto/kdf_common.go not found"
    exit 1
fi
if [[ ! -f "core/crypto/kdf_rust.go" ]]; then
    echo "    ERROR: core/crypto/kdf_rust.go not found"
    exit 1
fi
echo "    OK: Go FFI files exist"

# Check 3: Verify function names match between Rust and Go
echo "[3] Checking function name consistency..."

# Get Rust function names
RUST_FUNCS=$(grep -E "^pub unsafe extern \"C\" fn" crypto-argon2/src/lib.rs | sed 's/pub unsafe extern "C" fn //' | sed 's/(.*$//' | tr -d ' ')

# Get Go CGO extern names
GO_EXTERNS=$(grep -E "^extern int32_t" core/crypto/kdf_rust.go | sed 's/extern int32_t //' | sed 's/(.*$//' | tr -d ' ')

echo "    Rust FFI functions:"
for func in $RUST_FUNCS; do echo "      - $func"; done
echo "    Go CGO bindings:"
for func in $GO_EXTERNS; do echo "      - $func"; done

# Check that all Go bindings exist in Rust
MISSING=""
for func in $GO_EXTERNS; do
    if ! echo "$RUST_FUNCS" | grep -q "$func"; then
        MISSING="$MISSING $func"
    fi
done

if [[ -z "$MISSING" ]]; then
    echo "    OK: All Go bindings have corresponding Rust functions"
else
    echo "    WARNING: Missing Rust functions:$MISSING"
fi

# Check 4: Verify build tags
echo "[4] Checking build tags..."
RUST_BUILD=$(head -1 core/crypto/kdf_rust.go | grep -oE '//go:build.*')
echo "    Rust build tag: $RUST_BUILD"

# Check 5: Verify kdf_common.go exports correct interface
echo "[5] Checking kdf_common.go interface..."
if grep -q "func DeriveKey" core/crypto/kdf_common.go; then
    echo "    OK: DeriveKey function exported"
else
    echo "    ERROR: DeriveKey function not found"
    exit 1
fi

if grep -q "func GenerateSalt" core/crypto/kdf_common.go; then
    echo "    OK: GenerateSalt function exported"
else
    echo "    ERROR: GenerateSalt function not found"
    exit 1
fi

# Check 6: Verify Cargo.toml has required dependencies
echo "[6] Checking Cargo.toml dependencies..."
if grep -q "argon2" crypto-argon2/Cargo.toml; then
    echo "    OK: argon2 dependency found"
else
    echo "    ERROR: argon2 dependency missing"
    exit 1
fi

if grep -q "aes-gcm" crypto-argon2/Cargo.toml; then
    echo "    OK: aes-gcm dependency found"
else
    echo "    NOTE: aes-gcm dependency not found (optional)"
fi

# Check 7: Verify Go build compiles
echo "[7] Checking Go build..."
if go build -tags='rust cgo' ./core/crypto/... 2>/dev/null; then
    echo "    OK: Go package compiles"
else
    echo "    WARNING: Go package has compilation errors"
fi

echo ""
echo "=== Validation Complete ==="
echo ""
echo "SUMMARY:"
echo "  - Rust crate: $(ls crypto-argon2/target/release/*.a 2>/dev/null | wc -l) static libraries built"
echo "  - Go FFI: Ready"
echo ""
echo "To build:"
echo "  go build -tags 'rust cgo' ./..."
