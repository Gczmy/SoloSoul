#!/bin/bash

# ============================================================
# SoloSoul Release Builder — Tauri v2 Edition
# ============================================================
# 一键构建 SoloSoul macOS Release 版本，包含代码签名与 DMG 打包。
#
# 使用方式:
#   ./build_release.sh                    # 默认构建
#   ./build_release.sh --verbose          # 详细输出
#   APPLE_SIGNING_IDENTITY="XXX" ./build_release.sh  # 使用指定证书签名
#
# 重要说明:
#   - 当前版本使用 ad-hoc 签名（默认，无需 Apple Developer 账户）
#   - 如需使用 Apple Development 证书，请设置 APPLE_SIGNING_IDENTITY 环境变量
#   - 本脚本不包含公证（Notarization），因当前无 Developer ID 账户
#   - 对外分发前需自行完成公证，或使用 CI 流水线处理
#
# 前置要求:
#   - Node.js >= 22
#   - Rust / Cargo (stable)
#   - npm
#   - macOS (脚本针对 macOS 设计，其他平台需调整)
# ============================================================

set -euo pipefail

# --- 配置区 ---
APP_NAME="SoloSoul"
BUNDLE_ID="com.solosoul.app"
TAURI_DIR="tauri"
BUNDLE_BASE="${TAURI_DIR}/target/release/bundle"

# 签名身份: 默认 ad-hoc ("-")，可通过环境变量覆盖
SIGN_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# 参数解析
VERBOSE=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose|-v) VERBOSE=1; shift ;;
        --help|-h)
            echo "Usage: $0 [--verbose] [--help]"
            echo ""
            echo "Environment variables:"
            echo "  APPLE_SIGNING_IDENTITY   Apple signing identity (default: ad-hoc '-')"
            exit 0
            ;;
        *) echo -e "${RED}Unknown option: $1${NC}"; exit 1 ;;
    esac
done

if [[ "$VERBOSE" -eq 1 ]]; then
    set -x
fi

# --- 工具函数 ---
log_info()  { echo -e "${GREEN}[INFO]${NC}  $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step()  { echo -e "${CYAN}[STEP]${NC}  $1"; }

# --- 读取版本号 ---
if [[ ! -f "${TAURI_DIR}/package.json" ]]; then
    log_error "找不到 ${TAURI_DIR}/package.json，请在项目根目录运行此脚本"
    exit 1
fi

# 支持通过 VERSION 环境变量覆盖，否则从 package.json 读取
if [[ -z "${VERSION:-}" ]]; then
    VERSION=$(grep '"version"' "${TAURI_DIR}/package.json" | head -1 | sed -E 's/.*"version": "([^"]+)".*/\1/')
fi
if [[ -z "$VERSION" ]]; then
    log_error "无法从 package.json 解析版本号"
    exit 1
fi

# 架构标识
ARCH=$(uname -m)
DMG_NAME="${APP_NAME}_${VERSION}_${ARCH}.dmg"

# --- 横幅 ---
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul Release Builder (Tauri v2)${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${CYAN}Version:${NC}    ${VERSION}"
echo -e "${CYAN}Platform:${NC}   macOS ${ARCH}"
echo -e "${CYAN}Identity:${NC}   ${SIGN_IDENTITY}"
echo -e "${CYAN}Output:${NC}     ${BUNDLE_BASE}/dmg/${DMG_NAME}"
echo ""

# --- 前置检查 ---
log_step "Checking prerequisites..."

check_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        log_error "$1 is required but not installed"
        exit 1
    fi
}

check_cmd node
check_cmd npm
check_cmd cargo

# 检查 Node 版本
NODE_MAJOR=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [[ "$NODE_MAJOR" -lt 22 ]]; then
    log_warn "Node.js >= 22 recommended (found $(node -v))"
fi

# 检查是否在项目根目录
if [[ ! -d ".git" || ! -d "${TAURI_DIR}" ]]; then
    log_error "请在项目根目录运行此脚本"
    exit 1
fi

# --- 清理旧产物 ---
log_step "Cleaning previous build artifacts..."
rm -rf "${BUNDLE_BASE}"
mkdir -p "${BUNDLE_BASE}/dmg"

# --- 安装依赖 ---
log_step "Installing Node dependencies..."
cd "${TAURI_DIR}"
npm ci

# --- 构建 Tauri (仅生成 .app，DMG 后面手动打包以确保签名生效) ---
log_step "Building Tauri release (target: app bundle only)..."
npm run tauri build -- --bundles app

cd ".."

APP_PATH="${BUNDLE_BASE}/macos/${APP_NAME}.app"
if [[ ! -d "$APP_PATH" ]]; then
    log_error "构建失败: 找不到 ${APP_PATH}"
    exit 1
fi

# --- 代码签名 ---
log_step "Code signing app bundle..."

# 清除扩展属性，避免签名干扰
xattr -cr "$APP_PATH"

# 1. 签名所有嵌入的 .dylib（从深到浅，先签内部依赖）
DYLIB_COUNT=$(find "$APP_PATH/Contents" -name "*.dylib" 2>/dev/null | wc -l | tr -d ' ')
if [[ "$DYLIB_COUNT" -gt 0 ]]; then
    log_info "Signing ${DYLIB_COUNT} embedded dylibs..."
    find "$APP_PATH/Contents" -name "*.dylib" -exec \
        codesign --force --sign "${SIGN_IDENTITY}" --timestamp=none {} \;
fi

# 2. 签名所有 .framework（从深到浅）
FW_COUNT=$(find "$APP_PATH/Contents" -name "*.framework" -type d 2>/dev/null | wc -l | tr -d ' ')
if [[ "$FW_COUNT" -gt 0 ]]; then
    log_info "Signing ${FW_COUNT} embedded frameworks..."
    find "$APP_PATH/Contents" -name "*.framework" -type d -print0 | \
    while IFS= read -r -d '' fw; do
        binary="$fw/$(basename "$fw" .framework)"
        if [[ -f "$binary" ]]; then
            codesign --force --sign "${SIGN_IDENTITY}" --timestamp=none "$binary" 2>/dev/null || true
        fi
        codesign --force --sign "${SIGN_IDENTITY}" --timestamp=none "$fw" 2>/dev/null || true
    done
fi

# 3. 签名主程序包
log_info "Signing main app bundle..."
codesign --force --deep --sign "${SIGN_IDENTITY}" \
         --identifier "${BUNDLE_ID}" \
         --timestamp=none \
         "$APP_PATH"

# 4. 验证签名
log_step "Verifying signature..."
if codesign --verify --verbose "$APP_PATH" 2>/dev/null; then
    log_info "Signature verification passed"
else
    log_warn "Signature verification returned warnings (ad-hoc signing is normal)"
fi

# --- DMG 打包 ---
log_step "Creating DMG..."

DMG_STAGING_DIR="${BUNDLE_BASE}/dmg_staging"
DMG_OUTPUT="${BUNDLE_BASE}/dmg/${DMG_NAME}"

rm -rf "$DMG_STAGING_DIR"
mkdir -p "$DMG_STAGING_DIR"
cp -R "$APP_PATH" "$DMG_STAGING_DIR/"

# 优先使用 create-dmg（更美观），否则 fallback 到 hdiutil
if command -v create-dmg >/dev/null 2>&1; then
    log_info "Using create-dmg for branded DMG..."
    create-dmg \
        --volname "${APP_NAME}" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "${APP_NAME}.app" 150 180 \
        --hide-extension "${APP_NAME}.app" \
        --app-drop-link 450 180 \
        --no-internet-enable \
        "$DMG_OUTPUT" \
        "$DMG_STAGING_DIR/" \
        2>/dev/null || {
            log_warn "create-dmg failed, falling back to hdiutil"
            hdiutil create -format UDZO -srcfolder "$DMG_STAGING_DIR" "$DMG_OUTPUT"
        }
else
    log_info "Using hdiutil (install create-dmg for branded DMG)"
    hdiutil create -format UDZO -srcfolder "$DMG_STAGING_DIR" "$DMG_OUTPUT"
fi

# 对 DMG 签名
codesign --force --sign "${SIGN_IDENTITY}" --timestamp=none "$DMG_OUTPUT" 2>/dev/null || true

# 清理 staging
rm -rf "$DMG_STAGING_DIR"

# --- 输出结果 ---
APP_SIZE=$(du -sh "$APP_PATH" 2>/dev/null | cut -f1)
DMG_SIZE=$(du -sh "$DMG_OUTPUT" 2>/dev/null | cut -f1)

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Build Complete! 🚀${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${BLUE}Version:${NC}      ${VERSION}"
echo -e "${BLUE}App Bundle:${NC}   ${APP_PATH} (${APP_SIZE})"
echo -e "${BLUE}DMG:${NC}          ${DMG_OUTPUT} (${DMG_SIZE})"
echo ""
echo -e "${YELLOW}Notes:${NC}"
echo -e "  • 当前签名方式: ${SIGN_IDENTITY}"
echo -e "  • ad-hoc 签名 (-) 允许本地运行，但首次打开可能需在 系统设置 > 隐私与安全性 中允许"
echo -e "  • 对外分发需使用 Apple Developer ID 证书 + 公证 (Notarization)"
echo -e "  • 使用 Apple 证书: APPLE_SIGNING_IDENTITY='Developer ID Application: XXX' $0"
echo ""
