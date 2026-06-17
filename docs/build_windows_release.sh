#!/bin/bash

# ============================================================
# SoloSoul Windows Release Builder — Tauri v2 Edition
# ============================================================
# 一键构建 SoloSoul Windows Release 版本（NSIS）。
#
# 使用方式:
#   ./build_windows_release.sh                    # 默认构建
#   ./build_windows_release.sh --verbose          # 详细输出
#
# 重要说明:
#   - 本脚本需在 Windows 环境（Git Bash / MSYS2 / WSL）中运行
#   - 确保已安装 Node.js >= 22、Rust (stable)、npm
#   - Windows 代码签名需另行购买证书并使用 signtool，当前未在脚本中实现
#   - 产物为 NSIS 安装包（.exe）
# ============================================================

set -euo pipefail

# --- 配置区 ---
APP_NAME="SoloSoul"
TAURI_DIR="tauri"
BUNDLE_BASE="${TAURI_DIR}/target/release/bundle"

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

# 架构标识（Windows 固定 x64）
ARCH="x64"
NSIS_NAME="${APP_NAME}_${VERSION}_${ARCH}-setup.exe"

# --- 横幅 ---
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SoloSoul Windows Release Builder${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${CYAN}Version:${NC}    ${VERSION}"
echo -e "${CYAN}Platform:${NC}   Windows ${ARCH}"
echo -e "${CYAN}Output:${NC}     ${BUNDLE_BASE}/nsis/${NSIS_NAME}"
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

# 安装/检查 Python Pillow（生成 NSIS 安装程序位图资源所需）
log_step "Installing Python dependencies..."
python3 -m pip install Pillow --quiet 2>/dev/null || python -m pip install Pillow --quiet 2>/dev/null || log_warn "Could not install Pillow; build will use existing NSIS assets if available"

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

# 检查本地模型文件是否存在（tauri.conf.json 中声明为打包资源，但模型文件被 gitignore）
MODELS_DIR="${TAURI_DIR}/src-tauri/resources/models"
if [[ ! -d "${MODELS_DIR}/all-MiniLM-L6-v2" || ! -d "${MODELS_DIR}/pp-ocr-v6-small" ]]; then
    log_error "找不到本地模型文件: ${MODELS_DIR}"
    log_error "模型文件未提交到 Git，构建前请确保已下载或从其他机器复制。"
    log_error "本次发布已准备的模型包: SoloSoul-Releases/models-v2.3.0.zip"
    exit 1
fi

# 解析 Tauri 自动更新器签名密钥
TAURI_KEY_FILE="${HOME}/.tauri/secret.key"
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -f "${TAURI_KEY_FILE}" ]]; then
    log_info "从 ${TAURI_KEY_FILE} 读取 Tauri 签名私钥"
    TAURI_SIGNING_PRIVATE_KEY=$(cat "${TAURI_KEY_FILE}")
    export TAURI_SIGNING_PRIVATE_KEY
fi

# --- 清理旧产物 ---
log_step "Cleaning previous build artifacts..."
rm -rf "${BUNDLE_BASE}"
mkdir -p "${BUNDLE_BASE}/nsis"

# --- 安装依赖 ---
log_step "Installing Node dependencies..."
cd "${TAURI_DIR}"
npm ci

# --- 构建 Tauri ---
log_step "Building Tauri release (target: nsis)..."
npm run tauri build

cd ".."

NSIS_PATH="${BUNDLE_BASE}/nsis/${NSIS_NAME}"

if [[ ! -f "$NSIS_PATH" ]]; then
    log_error "构建失败: 找不到 ${NSIS_PATH}"
    exit 1
fi

# 生成 Tauri 自动更新器签名文件（.sig）
log_step "Signing updater package..."
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    log_error "未设置 TAURI_SIGNING_PRIVATE_KEY，且未找到 ~/.tauri/secret.key"
    log_error "请设置环境变量：export TAURI_SIGNING_PRIVATE_KEY='...'"
    log_error "或生成并保存密钥到 ~/.tauri/secret.key："
    log_error "  cd tauri && npx tauri signer generate -w ~/.tauri/secret.key"
    log_error "然后将公钥更新到 tauri/src-tauri/tauri.conf.json 的 updater.pubkey"
    exit 1
fi
# 注意：TAURI_DIR 下运行 npx tauri，但 NSIS_PATH 是相对于项目根目录的路径，
# 因此传入绝对路径，避免文件找不到。
NSIS_PATH_ABS="$(cd "${TAURI_DIR}/.." && pwd)/${NSIS_PATH}"
(
    cd "${TAURI_DIR}"
    npx tauri signer sign --password "" --private-key "$TAURI_SIGNING_PRIVATE_KEY" "$NSIS_PATH_ABS"
)

NSIS_SIZE=$(du -sh "$NSIS_PATH" 2>/dev/null | cut -f1)

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Build Complete! 🚀${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${BLUE}Version:${NC}      ${VERSION}"
echo -e "${BLUE}NSIS:${NC}         ${NSIS_PATH} (${NSIS_SIZE})"
echo ""
echo -e "${YELLOW}Notes:${NC}"
echo -e "  • Windows 代码签名未启用，如需签名请使用 signtool 对 MSI/EXE 进行签名"
echo -e "  • 对外分发前建议购买代码签名证书以消除 SmartScreen 警告"
echo ""
