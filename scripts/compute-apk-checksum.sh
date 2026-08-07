#!/bin/bash
# ============================================================
# SoloSoul APK Checksum Generator (+ minisign signature)
# ============================================================
# 为 Android APK 生成 SHA-256 校验和文件，并对其签名（P003）。
#
# 校验和文件应随 APK 一起上传到 GitHub Release；客户端在下载后
# 会先验签（.sha256.minisig）再校验 SHA-256，校验和不再与 APK 同通道
# 无条件信任（P003 闭环）。
#
# 签名密钥：**embed 注册表专用密钥对**（与 embedding registry.json 同款，
# 客户端公钥已编译进 embed_model.rs / update.rs）。私钥默认从
#   ${HOME}/SoloSoul/signing/embed-registry/embed-registry.key
# 读取，也可用环境变量 SOLOSOUL_EMBED_PRIVATE_KEY 覆盖。
#
# 使用方式:
#   ./scripts/compute-apk-checksum.sh path/to/SoloSoul_2.6.1_universal-release.apk
#
# 产物:
#   path/to/SoloSoul_2.6.1_universal-release.apk.sha256
#   path/to/SoloSoul_2.6.1_universal-release.apk.sha256.minisig
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

APK_PATH="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"

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

# ── P003: 对校验和文件签名（tauri signer，输出为 base64 包裹的 minisign 明文） ──
EMBED_KEY="${SOLOSOUL_EMBED_PRIVATE_KEY:-${HOME}/SoloSoul/signing/embed-registry/embed-registry.key}"
SIGNATURE_FILE="${CHECKSUM_FILE}.minisig"

if [[ ! -f "$EMBED_KEY" ]]; then
    log_error "未找到 embed 签名私钥: ${EMBED_KEY}"
    log_error "客户端将拒绝未签名校验和（P003 硬失败），必须先生成签名。"
    exit 1
fi

log_info "正在签名校验和: $(basename "$CHECKSUM_FILE")"
TAURI_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../tauri" && pwd)"
# set +u: 规避 macOS bash 3.2 在子 shell 中对含中文注释脚本的变量解析误报
if ! (set +u; cd "$TAURI_DIR" && npx tauri signer sign --password "" --private-key-path "$EMBED_KEY" "$CHECKSUM_FILE" >/dev/null 2>&1); then
    log_error "签名失败，请检查 embed 私钥（${EMBED_KEY}）"
    exit 1
fi
if [[ -f "${CHECKSUM_FILE}.sig" ]]; then
    cp "${CHECKSUM_FILE}.sig" "$SIGNATURE_FILE"
    log_info "已写入签名: $(basename "$SIGNATURE_FILE")"
else
    log_error "签名产物缺失: ${CHECKSUM_FILE}.sig"
    exit 1
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  请将以下文件上传到 GitHub Release:${NC}"
echo -e "${GREEN}========================================${NC}"
echo "  1. ${APK_BASENAME}"
echo "  2. ${APK_BASENAME}.sha256"
echo "  3. ${APK_BASENAME}.sha256.minisig"
echo ""
echo "示例: gh release upload v2.6.1 ${APK_PATH} ${CHECKSUM_FILE} ${SIGNATURE_FILE}"
echo ""
