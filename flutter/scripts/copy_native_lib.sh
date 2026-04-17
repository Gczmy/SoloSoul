#!/bin/bash
# Copy Rust dylib to macOS app bundle

set -e

FLUTTER_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$FLUTTER_DIR/build/macos/Build/Products/Release"
APP_BUNDLE="$BUILD_DIR/solosoul_flutter.app"
RUST_DIR="$FLUTTER_DIR/native"
RUST_TARGET="$RUST_DIR/target/aarch64-apple-darwin/release"

echo "=== SoloSoul Native Library Builder ==="

# Build Rust library if needed
if [ ! -f "$RUST_TARGET/libsolosoul_core.dylib" ]; then
    echo "Building Rust library..."
    cd "$RUST_DIR"
    /opt/homebrew/bin/cargo build --release
    cd "$FLUTTER_DIR"
fi

# Copy to app bundle
if [ -d "$APP_BUNDLE/Contents/Frameworks" ]; then
    echo "Copying libsolosoul_core.dylib to app bundle..."
    cp "$RUST_TARGET/libsolosoul_core.dylib" "$APP_BUNDLE/Contents/Frameworks/"
    echo "Done!"
else
    echo "Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi
