#!/bin/bash

# ============================================================
# SoloSoul DMG Builder - Fixed Rpath & Identity Signed
# ============================================================
set -e

VERSION=${1:-"1.0.0"}
APP_NAME="SoloSoul"
DMG_NAME="${APP_NAME}"
BUNDLE_ID="com.solosoul.solosoulFlutter" 

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' 

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul DMG Builder (Fixing Libraries)${NC}"
echo -e "${GREEN}========================================${NC}"

# Step 1: Build
RELEASE_DIR="build/macos/Build/Products/Release"
APP_PATH="$RELEASE_DIR/$APP_NAME.app"
DMG_STAGING_DIR="build/macos/dmg_staging"
DMG_OUTPUT="build/macos/${DMG_NAME}.dmg"

# 清理之前的编译产物，避免 hdiutil convert 因文件已存在而失败
echo -e "${YELLOW}Cleaning previous build artifacts...${NC}"
rm -f "build/macos/${DMG_NAME}.dmg"
rm -f build/macos/rw.*."${DMG_NAME}.dmg"
rm -rf "${DMG_STAGING_DIR}"

echo -e "${YELLOW}Building Flutter app...${NC}"
flutter build macos --release

# --- 关键修复：修正动态库路径 (RPATH) ---
echo -e "${YELLOW}Fixing Dynamic Library Paths (Self-Containment)...${NC}"

# 1. 确保 Frameworks 目录存在并将 Rust 库拷进去
mkdir -p "$APP_PATH/Contents/Frameworks"
cp "native/target/release/libsolosoul_core.dylib" "$APP_PATH/Contents/Frameworks/"

# 2. 修改主程序，使其去相对路径查找 dylib
TARGET_EXEC="$APP_PATH/Contents/MacOS/SoloSoul"
# 找到当前指向你用户目录的那个错误路径
OLD_PATH=$(otool -L "$TARGET_EXEC" | grep libsolosoul_core | awk '{print $1}')

if [ ! -z "$OLD_PATH" ]; then
    echo "Updating link from: $OLD_PATH"
    install_name_tool -change "$OLD_PATH" \
                      "@executable_path/../Frameworks/libsolosoul_core.dylib" \
                      "$TARGET_EXEC"
fi

# 3. 同时也修正 dylib 自己的 ID 路径
install_name_tool -id "@executable_path/../Frameworks/libsolosoul_core.dylib" \
                  "$APP_PATH/Contents/Frameworks/libsolosoul_core.dylib"

# --- 注入正式签名 ---
echo -e "${YELLOW}Injecting Identity Signature...${NC}"
IDENTITY="A432EC36C0EF2CD554D9E9679CDAC754F414C072"

TEMP_ENT="/tmp/solosoul_release.entitlements"
printf '<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <false/>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>' > "$TEMP_ENT"

xattr -cr "$APP_PATH"

# 签名内部库 (必须先签库，再签主程序)
echo -e "${YELLOW}Signing Frameworks...${NC}"
find "$APP_PATH/Contents/Frameworks" -name "*.dylib" | xargs codesign --force --sign "$IDENTITY" --timestamp=none

# 签名主程序
echo -e "${YELLOW}Signing Main Executable...${NC}"
codesign --force --sign "$IDENTITY" \
         --identifier "$BUNDLE_ID" \
         --entitlements "$TEMP_ENT" \
         --timestamp=none "$APP_PATH"

rm "$TEMP_ENT"

# Step 3 & 4: 打包 DMG (逻辑保持不变)
echo -e "${YELLOW}Creating DMG...${NC}"
rm -rf "$DMG_STAGING_DIR" && mkdir -p "$DMG_STAGING_DIR"
cp -R "$APP_PATH" "$DMG_STAGING_DIR/"

if command -v create-dmg >/dev/null 2>&1; then
    create-dmg --volname "${APP_NAME}" --window-pos 200 120 --window-size 600 400 --icon-size 100 \
               --icon "${APP_NAME}.app" 150 180 --hide-extension "${APP_NAME}.app" --app-drop-link 450 180 \
               "$DMG_OUTPUT" "$DMG_STAGING_DIR/"
else
    hdiutil create -format UDZO -srcfolder "$DMG_STAGING_DIR" "$DMG_OUTPUT"
fi

codesign --force --sign "$IDENTITY" "$DMG_OUTPUT"
rm -rf "$DMG_STAGING_DIR"

echo -e "${GREEN}Build Complete! 🚀${NC}"
