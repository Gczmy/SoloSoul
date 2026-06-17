#!/bin/bash
# 下载对应平台的 PDFium 动态库并放置到 src-tauri/resources/pdfium/
# 用于 OCR 的 PDF 渲染功能。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEST_DIR="${PROJECT_ROOT}/src-tauri/resources/pdfium"

mkdir -p "${DEST_DIR}"

OS="$(uname -s)"
MACHINE="$(uname -m)"

case "${OS}" in
  Darwin*)
    if [ "${MACHINE}" = "arm64" ]; then
      ARCH="mac-arm64"
    else
      ARCH="mac-x64"
    fi
    FILENAME="libpdfium.dylib"
    ;;
  Linux*)
    ARCH="linux-x64"
    FILENAME="libpdfium.so"
    ;;
  MINGW*|CYGWIN*|MSYS*)
    ARCH="win-x64"
    FILENAME="pdfium.dll"
    ;;
  *)
    echo "Unsupported operating system: ${OS}"
    exit 1
    ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-${ARCH}.tgz"
TMP_DIR="$(mktemp -d)"

echo "Downloading PDFium for ${ARCH} from ${URL}..."
curl -L -o "${TMP_DIR}/pdfium.tgz" "${URL}"

echo "Extracting..."
tar xzf "${TMP_DIR}/pdfium.tgz" -C "${TMP_DIR}"

echo "Installing ${FILENAME} to ${DEST_DIR}..."
cp "${TMP_DIR}/lib/${FILENAME}" "${DEST_DIR}/${FILENAME}"

rm -rf "${TMP_DIR}"

echo "PDFium installed: ${DEST_DIR}/${FILENAME}"
