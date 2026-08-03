#!/bin/bash
# ============================================================
# SoloSoul Release Artifact Signer
# ============================================================
# 在 macOS 上为 Release 产物生成 Tauri updater .sig 签名。
# 统一签名入口，避免在 Windows 构建机上暴露私钥。
#
# 注意：.sig 文件的内容会被 generate-latest-json.js 读入 latest.json，
#       但 .sig 文件本身不需要上传到 GitHub Release。
#
# 使用方式:
#   ./docs/sign_artifacts.sh [artifacts-dir]
#
# 默认产物目录: ./SoloSoul-Releases
# 私钥读取优先级:
#   1. TAURI_SIGNING_PRIVATE_KEY 环境变量
#   2. ~/SoloSoul/signing/tauri-updater/secret.key
# ============================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TAURI_DIR="${PROJECT_ROOT}/tauri"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC}  $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# --- 读取版本号 ---
if [[ ! -f "${TAURI_DIR}/package.json" ]]; then
    log_error "找不到 ${TAURI_DIR}/package.json"
    exit 1
fi

VERSION=$(grep '"version"' "${TAURI_DIR}/package.json" | head -1 | sed -E 's/.*"version": "([^"]+)".*/\1/')
if [[ -z "$VERSION" ]]; then
    log_error "无法从 package.json 解析版本号"
    exit 1
fi

# --- 产物目录 ---
ARTIFACTS_DIR="${1:-${PROJECT_ROOT}/SoloSoul-Releases}"
if [[ ! -d "$ARTIFACTS_DIR" ]]; then
    log_error "产物目录不存在: ${ARTIFACTS_DIR}"
    exit 1
fi

log_info "版本号: ${VERSION}"
log_info "产物目录: ${ARTIFACTS_DIR}"

# --- 解析签名私钥 ---
TAURI_KEY_FILE="${HOME}/SoloSoul/signing/tauri-updater/secret.key"
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -f "${TAURI_KEY_FILE}" ]]; then
    log_info "从 ${TAURI_KEY_FILE} 读取 Tauri 签名私钥"
    TAURI_SIGNING_PRIVATE_KEY=$(cat "${TAURI_KEY_FILE}")
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    log_error "未设置 TAURI_SIGNING_PRIVATE_KEY，且未找到 ${HOME}/SoloSoul/signing/tauri-updater/secret.key"
    log_error "请在 macOS 上配置私钥后重试。"
    exit 1
fi

# --- 签名函数 ---
sign_file() {
    local file="$1"
    local sig="${file}.sig"

    if [[ -f "$sig" ]]; then
        log_warn "已存在签名文件，跳过: ${sig}"
        return 0
    fi

    log_info "正在签名: $(basename "$file")"
    (
        cd "${TAURI_DIR}"
        npx tauri signer sign --password "" --private-key "$TAURI_SIGNING_PRIVATE_KEY" "$file"
    )
}

# --- 查找并签名产物 ---
shopt -s nullglob
signed_count=0
skipped_count=0

# 仅为 updater 实际使用的包生成 .sig：
# - macOS: .app.tar.gz
# - Windows: -setup.exe
# - Linux: .AppImage
# DMG 仅用于手动安装，不需要 updater 签名。
for pattern in \
    "${ARTIFACTS_DIR}/SoloSoul_${VERSION}_"*.app.tar.gz \
    "${ARTIFACTS_DIR}/SoloSoul_${VERSION}_"*-setup.exe \
    "${ARTIFACTS_DIR}/SoloSoul_${VERSION}.AppImage"
do
    for file in $pattern; do
        if [[ -f "$file" ]]; then
            if [[ -f "${file}.sig" ]]; then
                log_warn "已存在签名文件，跳过: $(basename "$file")"
                ((skipped_count++)) || true
            else
                sign_file "$file"
                ((signed_count++)) || true
            fi
        fi
    done
done
shopt -u nullglob

if [[ "$signed_count" -eq 0 && "$skipped_count" -eq 0 ]]; then
    log_warn "未找到需要签名的产物，请确认产物已放入 ${ARTIFACTS_DIR}"
    exit 0
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Signing Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "新签名: ${signed_count}"
echo -e "已跳过: ${skipped_count}"
echo ""
