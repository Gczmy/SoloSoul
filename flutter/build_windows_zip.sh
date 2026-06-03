#!/bin/bash

# ============================================================
# SoloSoul Windows ZIP Builder
# ============================================================
set -e

# Auto-read version from pubspec.yaml if not provided
PUBSPEC_VERSION=$(grep "^version:" pubspec.yaml | sed 's/version: //;s/+.*//')
VERSION=${1:-"$PUBSPEC_VERSION"}
APP_NAME="SoloSoul"
ZIP_NAME="${APP_NAME}-v${VERSION}-windows-x64"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul Windows ZIP Builder${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${YELLOW}Building version: ${VERSION}${NC}"

RELEASE_DIR="build/windows/x64/runner/Release"
ZIP_OUTPUT="build/windows/${ZIP_NAME}.zip"
STAGING_DIR="build/windows/zip_staging"

# Clean previous build artifacts
echo -e "${YELLOW}Cleaning previous build artifacts...${NC}"
rm -f "${ZIP_OUTPUT}"
rm -rf "${STAGING_DIR}"

echo -e "${YELLOW}Building Flutter app for Windows...${NC}"
flutter pub get
dart run pdfrx:remove_wasm_modules
flutter build windows --release --obfuscate --split-debug-info=./debug_info/windows

# Verify build output
if [ ! -f "${RELEASE_DIR}/solosoul_flutter.exe" ]; then
    echo -e "${RED}Error: Build output not found at ${RELEASE_DIR}/solosoul_flutter.exe${NC}"
    exit 1
fi

# Stage files for ZIP
echo -e "${YELLOW}Staging files for ZIP...${NC}"
mkdir -p "${STAGING_DIR}/${APP_NAME}"
cp -r "${RELEASE_DIR}/"* "${STAGING_DIR}/${APP_NAME}/"

# Create ZIP
echo -e "${YELLOW}Creating ZIP archive...${NC}"
cd "${STAGING_DIR}"
if command -v zip >/dev/null 2>&1; then
    zip -r "../${ZIP_NAME}.zip" "${APP_NAME}/"
else
    echo -e "${RED}Error: 'zip' command not found. Please install zip or use PowerShell.${NC}"
    exit 1
fi
cd - >/dev/null

# Clean staging
rm -rf "${STAGING_DIR}"

echo -e "${GREEN}Build Complete! 🚀${NC}"
echo -e "${GREEN}Output: ${ZIP_OUTPUT}${NC}"
