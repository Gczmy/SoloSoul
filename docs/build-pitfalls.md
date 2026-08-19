# 构建排坑记录（v2.11.1 发布实战）

> 本文记录 v2.11.1（2026-08-19）发布过程中踩到的全部坑：现象、根因、解法与预防。
> 发布主流程见 [release_process.md](release_process.md)。

---

## 坑 1：macOS 与 Android 并行构建互相踩踏（npm ci vs tsc）

**现象**：macOS 构建与 Android 构建同时启动，Android 侧报
`sh: tsc: command not found`，`beforeBuildCommand` 失败（exit 127）。

**根因**：macOS 构建脚本先执行 `npm ci`——它会**删除并重装** `tauri/node_modules`。
Android 构建的 `beforeBuildCommand`（`npm run build` → `tsc --noEmit`）恰好在这个
窗口期执行，`node_modules/.bin/tsc` 已被删除 → 找不到命令。

**解法**：改为**严格串行**构建——macOS 全部完成（含签名）后再启动 Android。
Android 重跑时 Rust 增量缓存命中，只走 Gradle 阶段，耗时大幅缩短。

**预防**：任何两个构建任务**不得并行**。构建机资源充足也不代表安全——`npm ci`
是破坏性操作，会清空 node_modules。若确需并行，至少先各自完成 `npm ci` + 前端构建
（只读 dist 输出），再并行 Rust 编译（cargo target 锁会串行等待，不损坏但会卡住）。

---

## 坑 2：后台进程被工具环境清理，构建「没起来」

**现象**：用 `nohup ./scripts/build_macos_release.sh > log 2>&1 &` 启动后台构建，
shell 返回了 PID，但数分钟后检查：**进程已不存在，日志文件甚至是几天前的旧内容**
（重定向未生效，文件时间戳未更新）。

**根因**：执行工具在每条命令返回后会清理派生的后台进程组，`nohup` 只能防 SIGHUP，
挡不住进程组清理；且首次启动时重定向都没来得及执行进程就已消失。

**解法**：改用 **tmux 独立会话**承载构建：

```bash
tmux new-session -d -s solosoul-build \
  "cd /Users/zzc/PycharmProjects/SoloSoul && ./scripts/build_macos_release.sh > /tmp/build.log 2>&1; echo EXIT_CODE=\$? >> /tmp/build.log"

# 轮询：会话退出 = 构建结束
tmux has-session -t solosoul-build && echo "RUNNING" || echo "DONE"
tail -f /tmp/build.log
```

tmux 会话是独立进程，不随执行工具清理，且自带退出状态回传（`EXIT_CODE=$?`）。

**预防**：长任务（Rust release 编译 / 交叉编译）一律进 tmux 会话，禁止裸 nohup。
每次启动用**新的日志文件名**（如 `*-v2111.log`），避免与历史残留同名文件混淆。

---

## 坑 3：keystore 密码提取失败（中文格式 + 行尾空格）

**现象**：Android Gradle 组装 APK 失败：
`KeytoolException: Failed to read key solosoul-upload ... keystore password was incorrect`。
Rust 25 个 crate 全部编译完成，白等了 12 分钟。

**根因**（两层）：
1. `~/SoloSoul/info.txt` 是**中文格式**（`• keystore 密码：xxx`），用
   `grep 'password'` 提取匹配不到 → 密码为空 → 读 keystore 失败；
2. 修正为按 `keystore 密码` 匹配后，提取值**含行尾对齐空格**（info.txt 用空格
   填充对齐，`sed` 提取保留行尾空白），密码变成 103 字符（实际 20）→ 依然失败。

**解法**：提取后必须 **trim 全部空白**：

```bash
SOLOSOUL_KEYSTORE_PASSWORD=$(
  grep 'keystore 密码' "$HOME/SoloSoul/info.txt" \
  | sed -E 's/.*密码[：:][[:space:]]*//' \
  | tr -d '[:space:]'
)
```

**预防**：提取后先验证长度（`${#VAR}` 应为 20 字符左右）再启动构建；发布流程文档
应固化上述中文兼容 + trim 的提取方式。

---

## 坑 4：产物路径找错（target 双层目录）

**现象**：macOS 构建日志显示 `Build Complete`，但
`ls tauri/src-tauri/target/release/bundle/` 报不存在，一度以为构建失败。

**根因**：`build_macos_release.sh` 的 `TAURI_DIR="tauri"`、
`BUNDLE_BASE="${TAURI_DIR}/target/release/bundle"` = **`tauri/target/release/bundle`**
（不是 `tauri/src-tauri/target/...`）。src-tauri 下是 cargo 工作区 target，bundle
产物在 tauri 根 target 下。

**预防**：查产物路径以**脚本内定义的 BUNDLE_BASE 为准**，不要凭经验猜测；
`find tauri -name "*.dmg" -newer <标记文件>` 可快速定位。

---

## 坑 5：旧日志文件残留导致误判

**现象**：第一次后台启动失败后，`head` 日志看到的是**几天前**（8/15）的旧内容
（v2.10.2 版本号、旧时间戳），差点误判为本次构建产物错误。

**根因**：`/tmp/solosoul-macos-build.log` 等通用文件名被历史构建占用，后台进程
没起来时文件未被截断，读到的是旧数据。

**预防**：每次构建用**版本号后缀文件名**（`*-v2111.log`）；判断构建真实状态先看
文件 mtime 与进程是否存活，再读内容。

---

## 坑 6（历史教训，本次已验证规避）：发布 keystore 误用

v2.11.0 曾误用另一把本地 keystore（`~/.solosoul/keys/solosoul-upload.jks`，
证书 OU=Engineering/Beijing）构建 APK，导致已安装用户升级报
INSTALL_FAILED_UPDATE_INCOMPATIBLE (-7)「签名不同」。**v2.11.1 已固化预防**：

1. keystore 唯一来源：`~/SoloSoul/solosoul-upload.jks`（别名 `solosoul-upload`）；
2. 构建后**立即验证证书指纹**：
   ```bash
   apksigner verify --print-certs <apk> | grep "SHA-256"
   # 必须等于 270fb489d218b02bc12fbb3489c8131fcabe723a5b2580dc7c1bc23be1e5f86c
   ```
3. 发布前跑 `bash scripts/verify-release-signatures.sh SoloSoul-Releases` 全量自检。

---

## 坑 7（v2.11.3）：Cargo.lock 的 sed 版本同步误改 tauri 框架 crate

```bash
sed -i '' 's/^version = "2.11.2"$/version = "2.11.3"/' Cargo.lock
```

这个「版本号同步」把 **tauri 框架 crate 自身**也连带改了：`tauri` /
`tauri-runtime` / `tauri-runtime-wry` 的版本号与应用版本号恰好同号
（如 2.11.2），sed 无从区分。tauri 2.11.3 要求 `tauri-build ^2.6.3`，
与锁定的 2.6.2 冲突 → macOS 构建在依赖解析阶段立即失败
（`failed to select a version for tauri-build`）。前两次发布（2.11.1/2.11.2）
同样误改过，只是恰好兼容没暴露。

**正确做法（只改 6 个工作区 crate）**：

```bash
# 按包块解析，只 bump 工作区 crate，框架三件套回退原版本
python3 - <<'EOF'
import re
with open('Cargo.lock') as f:
    blocks = re.split(r'(?m)^(?=name = )', f.read())
for i, b in enumerate(blocks):
    m = re.match(r'name = "([^"]+)"', b)
    if m and m.group(1) not in ('solo_soul','solosoul-core','solosoul-crypto','solosoul-plugin','solosoul-sync','solosoul-vault'):
        continue
    blocks[i] = re.sub(r'(?m)^version = "2\.11\.2"$', 'version = "2.11.3"', b, count=1)
open('Cargo.lock','w').write(''.join(blocks))
EOF
# 改完先验证解析：cargo check -p solo_soul
```

或更稳妥：**不手改 Cargo.lock**，bump Cargo.toml 后直接跑一次
`cargo check`/构建，让 cargo 自动把工作区 crate 版本对齐（框架依赖保持锁定）。

---

## 最佳实践速查（本版本教训的固化）

| 场景 | 做法 |
|------|------|
| 多平台构建 | **严格串行**；并行仅限 Rust 编译阶段（cargo 锁兜底） |
| 长任务后台 | **tmux 会话** + 退出码回传 + 版本号后缀日志名 |
| keystore 密码 | 从 info.txt 按中文键提取 + **trim 空白** + 验证长度 |
| APK 签名 | 构建后立即 apksigner 验证书指纹（270fb489...） |
| 产物路径 | 以构建脚本 `BUNDLE_BASE` 定义为准 |
| 日志判断 | 先看 mtime/进程，再读内容；文件名带版本号 |
| 发布前 | `verify-release-signatures.sh` 全量 PASS 才允许发版 |
