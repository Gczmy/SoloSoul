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

# Step 1: Build Flutter app if not already built
if [ ! -d "build/macos/Build/Products/Release/SoloSoul.app" ]; then
    echo -e "${YELLOW}Building Flutter app...${NC}"
    flutter build macos --release --obfuscate --split-debug-info=./debug_info/macos
else
    echo -e "${GREEN}App already built, skipping build.${NC}"
fi

APP_PATH="build/macos/Build/Products/Release/SoloSoul.app"
DMG_TEMP="/tmp/${DMG_NAME}-temp.dmg"
DMG_OUTPUT="build/macos/${DMG_NAME}.dmg"

# Step 2: Ad-hoc code sign (self-signed, no Apple account needed)
echo -e "${YELLOW}Signing app with ad-hoc certificate...${NC}"
codesign --force --deep --sign - "$APP_PATH"
echo -e "${GREEN}Code signing complete.${NC}"

# Step 3: Create DMG with create-dmg
echo -e "${YELLOW}Creating DMG...${NC}"

# Remove old DMG if exists
rm -f "$DMG_OUTPUT" "$DMG_TEMP"

# Use create-dmg for a professional installer
create-dmg \
    --volname "${APP_NAME} Installer" \
    --window-pos 200 120 \
    --window-size 800 400 \
    --icon-size 100 \
    --icon "$APP_NAME.app" 200 190 \
    --hide-extension "$APP_NAME.app" \
    --app-drop-link 600 185 \
    --eula ~/.claude/LICENSE \
    "$DMG_TEMP" \
    "$APP_PATH/.." \
    2>/dev/null || {
    # Fallback: simple DMG if create-dmg fails
    echo -e "${YELLOW}create-dmg failed, using hdiutil fallback${NC}"
    hdiutil create -volname "${APP_NAME} v${VERSION}" -srcfolder "$APP_PATH/.." -ov -format UDZO "$DMG_TEMP"
}

# Move to final location
mv "$DMG_TEMP" "$DMG_OUTPUT"

# Step 4: Sign the DMG itself
echo -e "${YELLOW}Signing DMG...${NC}"
codesign --force --sign - "$DMG_OUTPUT"
echo -e "${GREEN}DMG signing complete.${NC}"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Build Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Output: ${DMG_OUTPUT}${NC}"
echo -e "${YELLOW}Note: On first run, right-click the app and select${NC}"
echo -e "${YELLOW}\"Open\" to bypass the \"cannot verify developer\" warning.${NC}"
echo -e "${GREEN}========================================${NC}"

# Print file size
ls -lh "$DMG_OUTPUT"
