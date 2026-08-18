#!/bin/bash
# ============================================================
# SoloSoul 发布签名一致性自检（防 v2.11.0 事故复发）
# ============================================================
# 背景：v2.11.0 曾因误用第二把 Android keystore（证书 160c4a08...）构建 APK，
# 导致已安装用户升级报 INSTALL_FAILED_UPDATE_INCOMPATIBLE (-7)。
# 本脚本在发布前校验「客户端信任锚 ↔ 实际签名」的一致性，任一项不通过即非零退出：
#
#   1. Android APK 签名证书 == 历次发布固定证书（keystore 轮换须显式更新下方常量）
#   2. macOS/Windows updater 公钥（tauri.conf.json 内置）== 本机签名私钥对应公钥
#   3. macOS/Windows .sig 的 minisign keynum == updater 公钥 keynum
#   4. APK 校验和 .minisig 的 keynum == 客户端编译公钥 APK_CHECKSUM_PUBKEY
#   5. APK 校验和公钥（embed-registry）== 客户端编译公钥 APK_CHECKSUM_PUBKEY
#
# 用法:
#   ./scripts/verify-release-signatures.sh                        # 默认检查 ./SoloSoul-Releases/
#   ./scripts/verify-release-signatures.sh <产物目录>
#
# 依赖:
#   - apksigner（Android SDK build-tools，自动探测 ~/Library/Android/sdk）
#   - Python 3（纯标准库，无第三方依赖）
# ============================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_ok()   { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

# 产物目录（默认 SoloSoul-Releases）
ART_DIR="${1:-SoloSoul-Releases}"
ART_DIR_ABS="$(cd "$ART_DIR" && pwd)"

# ⚠️ 历次发布的 Android 正式签名证书（v2.10.2 及更早均为此证书）。
# 对应 keystore：~/SoloSoul/solosoul-upload.jks（密码见 ~/SoloSoul/info.txt，别名 solosoul-upload）。
# 若未来合法轮换 keystore，必须同步更新此值（旧客户端将无法覆盖安装，需重新分发）。
APK_EXPECTED_CERT_SHA256="270fb489d218b02bc12fbb3489c8131fcabe723a5b2580dc7c1bc23be1e5f86c"

# 密钥/配置文件路径
TAURI_UPDATE_PUB="$HOME/SoloSoul/signing/tauri-updater/secret.key.pub"
EMBED_PUB="$HOME/SoloSoul/signing/embed-registry/embed-registry.key.pub"
TAURI_CONF="tauri/src-tauri/tauri.conf.json"
UPDATE_RS="tauri/src-tauri/src/commands/update.rs"

FAILED=0

# 自动探测 apksigner
APKSIGNER=""
if command -v apksigner >/dev/null 2>&1; then
    APKSIGNER="$(command -v apksigner)"
elif [ -d "$HOME/Library/Android/sdk/build-tools" ]; then
    APKSIGNER="$(ls "$HOME"/Library/Android/sdk/build-tools/*/apksigner 2>/dev/null | sort -V | tail -1)"
fi

echo "=========================================="
echo " SoloSoul 发布签名一致性自检"
echo " 产物目录: $ART_DIR_ABS"
echo "=========================================="

# ── 1. Android APK 签名证书一致性 ─────────────────────────────
if [ -n "$APKSIGNER" ]; then
    APK=$(ls "$ART_DIR"/SoloSoul_*_universal-release.apk 2>/dev/null | head -1 || true)
    if [ -z "$APK" ]; then
        log_info "未找到 APK（${ART_DIR}/SoloSoul_*_universal-release.apk），跳过证书检查"
    else
        CERT=$("$APKSIGNER" verify --print-certs "$APK" 2>/dev/null | grep "SHA-256 digest" | awk '{print $NF}')
        if [ "$CERT" = "$APK_EXPECTED_CERT_SHA256" ]; then
            log_ok "APK 签名证书与历次发布一致（${CERT}）"
        else
            log_fail "APK 签名证书不一致！"
            log_fail "  实际:      $CERT"
            log_fail "  应为此值:  $APK_EXPECTED_CERT_SHA256"
            log_fail "  说明: keystore 用错或证书已轮换。若轮换需显式更新本脚本常量并重新分发。"
            FAILED=1
        fi
    fi
else
    log_info "未找到 apksigner，跳过 APK 证书检查（安装 Android SDK build-tools 后重试）"
fi

# ── 2-5. minisign 密钥体系一致性（纯 Python） ─────────────────
python3 - "$ART_DIR_ABS" "$TAURI_UPDATE_PUB" "$EMBED_PUB" "$TAURI_CONF" "$UPDATE_RS" <<'PYEOF'
import base64, json, re, sys

art_dir, update_pub_path, embed_pub_path, conf_path, update_rs_path = sys.argv[1:6]
failures = []

def b64decode_wrapped(blob: bytes) -> bytes:
    """tauri 的 .pub/.sig 文件 = base64(标准 minisign 文本)"""
    return base64.b64decode(blob.strip())

def keynum_from_wrapped_pubkey(blob: bytes) -> str:
    text = b64decode_wrapped(blob).decode()
    key_line = [l.strip() for l in text.split('\n') if l.strip()][-1]
    raw = base64.b64decode(key_line)
    return raw[2:10].hex().upper()

def keynum_from_wrapped_sig(blob: bytes) -> str:
    text = b64decode_wrapped(blob).decode()
    lines = [l.strip() for l in text.split('\n') if l.strip()]
    sig_line = [l for l in lines if not l.startswith('untrusted') and not l.startswith('trusted')][0]
    raw = base64.b64decode(sig_line)
    return raw[2:10].hex().upper()

def check(name, cond, detail):
    if cond:
        print(f"  [PASS] {name} ({detail})")
    else:
        print(f"  [FAIL] {name} ({detail})")
        failures.append(name)

# 2. updater 公钥：tauri.conf.json 内置 == 本机签名私钥对应公钥
try:
    conf_pub = json.load(open(conf_path))['plugins']['updater']['pubkey']
    pub_file = open(update_pub_path).read().strip()
    check("updater 公钥一致（tauri.conf.json == secret.key.pub）",
          conf_pub.strip() == pub_file, f"keynum={keynum_from_wrapped_pubkey(pub_file.encode())}")
except FileNotFoundError as e:
    print(f"  [SKIP] {e}")
except Exception as e:
    print(f"  [FAIL] updater 公钥检查异常: {e}")
    failures.append("updater 公钥检查异常")

# 3. macOS/Windows .sig keynum == updater 公钥 keynum
try:
    pub_k = keynum_from_wrapped_pubkey(open(update_pub_path, 'rb').read())
    for sig in ['SoloSoul_*_arm64.app.tar.gz.sig', 'SoloSoul_*_x64-setup.exe.sig']:
        import glob
        for path in glob.glob(f"{art_dir}/{sig}"):
            sig_k = keynum_from_wrapped_sig(open(path, 'rb').read())
            check(f"{path.split('/')[-1]} 与 updater 公钥匹配", sig_k == pub_k, f"keynum={sig_k}")
except Exception as e:
    print(f"  [FAIL] updater .sig keynum 检查异常: {e}")
    failures.append("updater .sig keynum 检查异常")

# 4/5. APK 校验和：minisig keynum 与公钥均须匹配客户端编译常量
try:
    m = re.search(r'const APK_CHECKSUM_PUBKEY: &str = "([^"]+)"', open(update_rs_path).read())
    compiled_pub = m.group(1)
    compiled_k = base64.b64decode(compiled_pub)[2:10].hex().upper()
    embed_k = keynum_from_wrapped_pubkey(open(embed_pub_path, 'rb').read())
    check("APK 校验和公钥一致（embed-registry == 客户端编译常量）",
          embed_k == compiled_k, f"keynum={embed_k}")
    import glob
    for path in glob.glob(f"{art_dir}/*.sha256.minisig"):
        sig_k = keynum_from_wrapped_sig(open(path, 'rb').read())
        check(f"{path.split('/')[-1]} 与客户端编译公钥匹配", sig_k == compiled_k, f"keynum={sig_k}")
except Exception as e:
    print(f"  [FAIL] APK 校验和检查异常: {e}")
    failures.append("APK 校验和检查异常")

if failures:
    print("\n  ❌ 存在失败项:", ", ".join(failures))
    sys.exit(1)
print("\n  ✅ 全部密钥一致性检查通过")
PYEOF

echo ""
if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}==========================================${NC}"
    echo -e "${GREEN} 签名一致性自检全部通过 ✅${NC}"
    echo -e "${GREEN}==========================================${NC}"
else
    echo -e "${RED}==========================================${NC}"
    echo -e "${RED} 签名一致性自检存在失败项，禁止发布！${NC}"
    echo -e "${RED}==========================================${NC}"
    exit 1
fi
