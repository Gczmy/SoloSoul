#!/bin/bash

# ============================================================
# SoloSoul DMG Builder - Identity-Signed (Non-Sandbox Version)
# ============================================================
set -e

VERSION=${1:-"1.0.0"}
APP_NAME="SoloSoul"
DMG_NAME="${APP_NAME}"
BUNDLE_ID="SoloSoul" 

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' 

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul DMG Builder (Compatibility Mode)${NC}"
echo -e "${GREEN}========================================${NC}"

# Step 1: Clean and build Flutter app
RELEASE_DIR="build/macos/Build/Products/Release"
APP_PATH="$RELEASE_DIR/$APP_NAME.app"
DMG_STAGING_DIR="build/macos/dmg_staging"
DMG_OUTPUT="build/macos/${DMG_NAME}.dmg"

echo -e "${YELLOW}Cleaning and building Flutter app...${NC}"
flutter build macos --release --obfuscate --split-debug-info=./debug_info/macos

# --- 注入正式签名 ---
echo -e "${YELLOW}Injecting Identity Signature...${NC}"

# 1. 自动获取证书 (使用 SHA1 确保唯一性)
IDENTITY="A432EC36C0EF2CD554D9E9679CDAC754F414C072"

# 2. 创建“非沙盒”权限文件 (解决 -34018 关键点)
# 彻底移除 keychain-access-groups，并将 sandbox 设为 false
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

# 3. 彻底清除扩展属性 (防止缓存导致验证失败)
sudo xattr -cr "$APP_PATH"

# 4. 递归签名内部所有组件
echo -e "${YELLOW}Signing internal components...${NC}"
find "$APP_PATH" -depth -name "*.framework" -or -name "*.dylib" -or -name "*.appex" | xargs codesign --force --sign "$IDENTITY" --timestamp=none

# 5. 签名主程序并注入权限
echo -e "${YELLOW}Signing main executable...${NC}"
codesign --force --sign "$IDENTITY" \
         --identifier "$BUNDLE_ID" \
         --entitlements "$TEMP_ENT" \
         --timestamp=none "$APP_PATH"

# 清理临时文件
rm "$TEMP_ENT"

echo -e "${GREEN}Identity signing complete.${NC}"
# --- 注入结束 ---

# Step 3: Create clean staging directory
echo -e "${YELLOW}Preparing clean staging directory...${NC}"
rm -rf "$DMG_STAGING_DIR"
mkdir -p "$DMG_STAGING_DIR"
cp -R "$APP_PATH" "$DMG_STAGING_DIR/"

# Step 4: Create DMG
echo -e "${YELLOW}Creating DMG...${NC}"
rm -f "$DMG_OUTPUT"

# 注意：如果环境中没安装 create-dmg，脚本会报错
if command -v create-dmg >/dev/null 2>&1; then
    create-dmg \
        --volname "${APP_NAME}" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "${APP_NAME}.app" 150 180 \
        --hide-extension "${APP_NAME}.app" \
        --app-drop-link 450 180 \
        "$DMG_OUTPUT" \
        "$DMG_STAGING_DIR/"
else
    echo -e "${YELLOW}Warning: create-dmg not found, using diskutil as fallback...${NC}"
    hdiutil create -format UDZO -srcfolder "$DMG_STAGING_DIR" "$DMG_OUTPUT"
fi

# Step 5: Sign the DMG itself
echo -e "${YELLOW}Signing DMG...${NC}"
codesign --force --sign "$IDENTITY" "$DMG_OUTPUT"
echo -e "${GREEN}DMG signing complete.${NC}"

rm -rf "$DMG_STAGING_DIR"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Build Complete! (Stable DMG Version)${NC}"
echo -e "${GREEN}========================================${NC}"
ls -lh "$DMG_OUTPUT"
