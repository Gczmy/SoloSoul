#!/bin/bash
set -e

APP_NAME="SoloSoul"
DMG_NAME="${APP_NAME}-v1.0"
APP_PATH="flutter/build/macos/Build/Products/Release/${APP_NAME}.app"
OUTPUT_DMG="flutter/build/macos/${DMG_NAME}.dmg"

echo "🚀 Building SoloSoul Release..."
cd /Users/zzc/PycharmProjects/SoloSoul

# Always rebuild to ensure latest code is included
echo "📦 Building Flutter macOS app..."
cd flutter && flutter build macos --release && cd ..

echo "📦 Packaging DMG..."
rm -f "$OUTPUT_DMG"

create-dmg \
  --volname "${APP_NAME} Installer" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "${APP_NAME}.app" 175 190 \
  --hide-extension "${APP_NAME}.app" \
  --app-drop-link 425 190 \
  "$OUTPUT_DMG" \
  "$APP_PATH"

echo "✅ Build Complete: $OUTPUT_DMG"
