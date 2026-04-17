#!/bin/bash
#
# build_rust.sh - Build the Rust crypto library for SoloSoul
#
# Usage:
#   ./build_rust.sh              # Build for current platform
#   ./build_rust.sh --all         # Build for all platforms
#   ./build_rust.sh --clean       # Clean and rebuild
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$SCRIPT_DIR/crypto-argon2"
OUTPUT_DIR="$CRATE_DIR/target/release"

echo "=== SoloSoul Rust Crypto Build ==="
echo "Crate directory: $CRATE_DIR"
echo ""

# Change to crate directory
cd "$CRATE_DIR"

# Handle arguments
case "${1:-}" in
    --all)
        echo "Building for all platforms..."
        echo ""
        echo "Note: Cross-compilation requires proper Rust toolchains."
        echo "Install with: rustup target add aarch64-apple-darwin x86_64-apple-darwin"
        echo ""

        # Check for macOS
        if [[ "$(uname -s)" == "Darwin" ]]; then
            echo ">>> Building for macOS (Apple Silicon)..."
            cargo build --release --target aarch64-apple-darwin

            echo ">>> Building for macOS (Intel)..."
            cargo build --release --target x86_64-apple-darwin

            echo ">>> Creating universal binary..."
            cargo-lipo --targets aarch64-apple-darwin,x86_64-apple-darwin 2>/dev/null || \
            lipo -create \
                "$OUTPUT_DIR/aarch64-apple-darwin/release/libsolosoul_crypto.a" \
                "$OUTPUT_DIR/x86_64-apple-darwin/release/libsolosoul_crypto.a" \
                -output "$OUTPUT_DIR/libsolosoul_crypto_universal.a"

            echo "Universal binary created: $OUTPUT_DIR/libsolosoul_crypto_universal.a"
        fi

        echo ""
        echo ">>> Building for Linux..."
        cargo build --release --target x86_64-unknown-linux-gnu

        echo ""
        echo ">>> Building for Windows..."
        cargo build --release --target x86_64-pc-windows-gnu
        ;;

    --clean)
        echo "Cleaning build artifacts..."
        cargo clean
        rm -f "$OUTPUT_DIR/libsolosoul_crypto"*.a
        ;;

    --test)
        echo "Running Rust tests..."
        cargo test
        ;;

    *)
        echo "Building for current platform..."
        cargo build --release

        LIB_NAME=""
        case "$(uname -s)" in
            Darwin)
                LIB_NAME="libsolosoul_crypto.a"
                ;;
            Linux)
                LIB_NAME="libsolosoul_crypto.a"
                ;;
            MINGW*|CYGWIN*)
                LIB_NAME="libsolosoul_crypto.a"
                ;;
        esac

        if [[ -f "$OUTPUT_DIR/$LIB_NAME" ]]; then
            echo ""
            echo "=== Build Complete ==="
            echo "Library: $OUTPUT_DIR/$LIB_NAME"
            ls -lh "$OUTPUT_DIR/$LIB_NAME"
        fi
        ;;
esac

echo ""
echo "Build script finished."
