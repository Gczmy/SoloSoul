#!/bin/bash
# 启动两个 SoloSoul 开发实例进行设备同步冒烟测试。
# 用法：bash scripts/dev-two-instances.sh
#
# 环境变量：
#   SOLOSOUL_SMOKE_DIR - 测试数据根目录，默认 /tmp/solosoul-smoke
#
# 工作流程：
# 1. 如果 device-a 还没有账号，先单独启动 A，提示用户创建账号后关闭。
# 2. 将 A 的数据复制到 device-b，使两边拥有相同的 account_id 与密码。
# 3. 同时启动 A、B 两个 tauri dev 实例，使用不同的 Vite/HMR 端口与数据目录。

set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BASE_DIR="${SOLOSOUL_SMOKE_DIR:-/tmp/solosoul-smoke}"
A_DIR="$BASE_DIR/device-a"
B_DIR="$BASE_DIR/device-b"

mkdir -p "$BASE_DIR"

cleanup() {
  echo "Shutting down instances..."
  [ -n "${PID_A:-}" ] && kill "$PID_A" 2>/dev/null || true
  [ -n "${PID_B:-}" ] && kill "$PID_B" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# 1. 如果 A 没有账号，先引导用户创建。
if [ ! -f "$A_DIR/accounts.json" ]; then
  echo "========================================"
  echo "  SoloSoul 同步双实例联调脚本"
  echo "========================================"
  echo ""
  echo "第一次运行需要先在 device-a 创建一个账号。"
  echo "请在打开的 App 中完成注册/登录，然后关闭窗口并按回车继续。"
  echo ""

  SOLOSOUL_DATA_DIR="$A_DIR" \
    SOLOSOUL_VITE_PORT=1420 \
    SOLOSOUL_VITE_HMR_PORT=1421 \
    npm run tauri dev --prefix "$ROOT_DIR" &
  PID_A=$!

  read -r
  kill "$PID_A" 2>/dev/null || true
  wait "$PID_A" 2>/dev/null || true
  PID_A=""

  if [ ! -f "$A_DIR/accounts.json" ]; then
    echo "错误：未在 $A_DIR 检测到账号，退出。"
    exit 1
  fi
fi

# 2. 复制 A 的数据到 B，保证 account_id 一致。
echo "Copying account data from device-a to device-b..."
rm -rf "$B_DIR"
cp -R "$A_DIR" "$B_DIR"

# 3. 同时启动两个实例。
echo "Launching device A and device B..."
echo "  A: data=$A_DIR  vite=1420  hmr=1421"
echo "  B: data=$B_DIR  vite=1430  hmr=1431"
echo ""
echo "操作提示："
echo "  1. 在两个实例中分别用同一密码解锁账号。"
echo "  2. 在 SyncPage 启用同步，记录对方的 fingerprint。"
echo "  3. 互相信任后，在任一实例点击 Sync 验证双向同步。"
echo ""

SOLOSOUL_DATA_DIR="$A_DIR" \
  SOLOSOUL_VITE_PORT=1420 \
  SOLOSOUL_VITE_HMR_PORT=1421 \
  npm run tauri dev --prefix "$ROOT_DIR" &
PID_A=$!

SOLOSOUL_DATA_DIR="$B_DIR" \
  SOLOSOUL_VITE_PORT=1430 \
  SOLOSOUL_VITE_HMR_PORT=1431 \
  npm run tauri dev --prefix "$ROOT_DIR" &
PID_B=$!

wait
