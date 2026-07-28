#!/bin/bash
# ============================================================
# SoloSoul APK Checksum Generator
# ============================================================
# 为 Android APK 生成 SHA-256 校验和文件，用于 P1 签名校验。
#
# 该文件应随 APK 一起上传到 GitHub Release，客户端在下载后
# 会自动校验 SHA-256 以确保 APK 完整性。
#
# 使用方式:
#   ./docs/compute-apk-checksum.sh path/to/SoloSoul_2.6.1_universal-release.apk
#
# 产物:
#   path/to/SoloSoul_2.6.1_universal-release.apk.sha256
#   内容格式: <64位hex>  <文件名>
# ============================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <path-to-apk>"
    echo ""
    echo "Example:"
    echo "  $0 ./SoloSoul-Releases/SoloSoul_2.6.1_universal-release.apk"
    exit 1
fi

APK_PATH="$1"

if [[ ! -f "$APK_PATH" ]]; then
    log_error "文件不存在: ${APK_PATH}"
    exit 1
fi

CHECKSUM_FILE="${APK_PATH}.sha256"
APK_BASENAME=$(basename "$APK_PATH")

# 计算 SHA-256
log_info "正在计算 SHA-256: ${APK_BASENAME}"

if command -v sha256sum &>/dev/null; then
    sha256sum "$APK_PATH" | awk '{print $1}' > "$CHECKSUM_FILE"
elif command -v shasum &>/dev/null; then
    shasum -a 256 "$APK_PATH" | awk '{print $1}' > "$CHECKSUM_FILE"
else
    # macOS 内置 shasum 的 fallback
    shasum -a 256 "$APK_PATH" | awk '{print $1}' > "$CHECKSUM_FILE"
fi

CHECKSUM=$(cat "$CHECKSUM_FILE")
CHECKSUM_SIZE=$(wc -c < "$CHECKSUM_FILE" | tr -d ' ')

log_info "SHA-256: ${CHECKSUM}"
log_info "已写入: ${CHECKSUM_FILE} (${CHECKSUM_SIZE}B)"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  请将以下文件上传到 GitHub Release:${NC}"
echo -e "${GREEN}========================================${NC}"
echo "  1. ${APK_BASENAME}"
echo "  2. ${APK_BASENAME}.sha256"
echo ""
echo "示例: gh release upload v2.6.1 ${APK_PATH} ${CHECKSUM_FILE}"
echo ""
