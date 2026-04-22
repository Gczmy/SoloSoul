#!/bin/bash
# ============================================================
# SoloSoul DMG Builder - Self-Signed Build Script
# ============================================================
# This script builds a self-signed macOS DMG for SoloSoul.
# No Apple Developer account required.
#
# Usage: ./build_dmg.sh
# Requirements: flutter, create-dmg (brew install create-dmg)
# ============================================================

set -e

VERSION=${1:-"1.0.0"}
APP_NAME="SoloSoul"
DMG_NAME="${APP_NAME}-v${VERSION}-macos"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul DMG Builder (Self-Signed)${NC}"
echo -e "${GREEN}========================================${NC}"

# Step 1: Clean and build Flutter app (always rebuild to ensure latest code)
RELEASE_DIR="build/macos/Build/Products/Release"
APP_PATH="$RELEASE_DIR/$APP_NAME.app"
DMG_STAGING_DIR="build/macos/dmg_staging"
DMG_OUTPUT="build/macos/${DMG_NAME}.dmg"

echo -e "${YELLOW}Cleaning and building Flutter app (always rebuild)...${NC}"
flutter build macos --release --obfuscate --split-debug-info=./debug_info/macos

# Step 2: Ad-hoc code sign (self-signed, no Apple account needed)
echo -e "${YELLOW}Signing app with ad-hoc certificate...${NC}"
codesign --force --deep --sign - "$APP_PATH"
echo -e "${GREEN}Code signing complete.${NC}"

# Step 3: Create clean staging directory (only .app, no intermediate files)
echo -e "${YELLOW}Preparing clean staging directory...${NC}"
rm -rf "$DMG_STAGING_DIR"
mkdir -p "$DMG_STAGING_DIR"

# Only copy the signed .app - nothing else
cp -R "$APP_PATH" "$DMG_STAGING_DIR/"

# Step 4: Create DMG with create-dmg
echo -e "${YELLOW}Creating DMG...${NC}"
rm -f "$DMG_OUTPUT"

create-dmg \
    --volname "${APP_NAME} v${VERSION}" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --icon "${APP_NAME}.app" 150 180 \
    --hide-extension "${APP_NAME}.app" \
    --app-drop-link 450 180 \
    "$DMG_OUTPUT" \
    "$DMG_STAGING_DIR/"  # Key: only package the staging dir content

# Step 5: Sign the DMG itself
echo -e "${YELLOW}Signing DMG...${NC}"
codesign --force --sign - "$DMG_OUTPUT"
echo -e "${GREEN}DMG signing complete.${NC}"

# Cleanup staging dir
rm -rf "$DMG_STAGING_DIR"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Build Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Output: ${DMG_OUTPUT}${NC}"
echo -e "${YELLOW}Note: On first run, right-click the app and select${NC}"
echo -e "${YELLOW}\"Open\" to bypass the \"cannot verify developer\" warning.${NC}"
echo -e "${GREEN}========================================${NC}"

# Print file size
ls -lh "$DMG_OUTPUT"
