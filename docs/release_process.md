# SoloSoul 发布流程（macOS + Windows）

> 同时发布 macOS DMG 和 Windows NSIS 安装包，版本号严格同步。
> 当前基于 **Tauri v2** 架构。

---

## 环境要求

- **macOS**：Node.js >= 22、Rust (stable)、npm、create-dmg（可选，用于美观 DMG）
- **Windows**：Node.js >= 22、Rust (stable)、npm、Visual Studio 2022+（提供 MSVC 工具链）

> 注：macOS 包必须在 Mac 上编译，Windows 包必须在 Windows 上编译，无法在一台机器上完成双平台。

### 本地模型文件

`tauri/src-tauri/resources/models/` 目录下的模型文件（`all-MiniLM-L6-v2/`、`pp-ocr-v6-small/`）被 `.gitignore` 排除，不会随仓库克隆自动出现，但 `tauri.conf.json` 将其声明为打包资源。构建前必须确保该目录存在且完整，否则 Tauri 打包会失败。

若从干净仓库开始构建，请从已准备好的构建机复制，或解压预打包的模型文件：

```bash
# 在 Mac/Windows 构建前检查
ls tauri/src-tauri/resources/models/all-MiniLM-L6-v2
ls tauri/src-tauri/resources/models/pp-ocr-v6-small
```

### Tauri 自动更新器签名密钥

应用内「检查更新」依赖 Tauri Updater，要求 Release 包附带 Ed25519 签名文件（`.sig`）以及 `latest.json`。构建前必须配置私钥：

```bash
export TAURI_SIGNING_PRIVATE_KEY="-----BEGIN OPENSSH PRIVATE KEY-----\n...\n-----END OPENSSH PRIVATE KEY-----\n"
```

> 私钥 **绝对不要** 提交到 Git。建议存储在密码管理器或 CI Secrets 中。

若首次设置签名密钥，使用 Tauri CLI 生成：

```bash
cd tauri
npx tauri signer generate -w ~/.tauri/solo_soul.key
```

生成后会输出公钥，将其更新到 `tauri/src-tauri/tauri.conf.json`：

```json
"plugins": {
  "updater": {
    "pubkey": "替换为生成命令输出的公钥"
  }
}
```

> 如果修改了 `pubkey`，旧版本客户端将无法再通过自动更新接收新版本，需要重新安装。仅在旧私钥丢失或泄露时更换密钥。

---

## 阶段一：准备（在 Mac 上执行一次）

### 1. 检查私有库同步状态

确认 https://github.com/Gczmy/SoloSoul_code.git 与本地状态相同，如有未推送的本地更新，优先推送。

```bash
git status
git push origin master
```

### 2. 确认并统一版本号

Tauri 版本号分散在 **3 个文件**中，必须保持严格一致：

| 文件 | 字段 | 示例 |
|------|------|------|
| `tauri/package.json` | `"version": "2.0.0"` | `"version": "2.1.0"` |
| `tauri/src-tauri/tauri.conf.json` | `"version": "2.0.0"` | `"version": "2.1.0"` |
| `tauri/Cargo.toml` | `workspace.package.version` | `version = "2.1.0"` |

修改以上三个文件，将版本号更新为下一个版本（遵循 [SemVer](https://semver.org/lang/zh-CN/)）。

> 版本号格式：`主版本.次版本.补丁`。macOS 和 Windows 使用完全相同的版本号。
> Tauri 不支持 `+buildNumber` 后缀，请使用纯 SemVer 格式。

### 3. 推送版本号更新到私有库

```bash
git add tauri/package.json tauri/src-tauri/tauri.conf.json tauri/Cargo.toml
git commit -m "chore: bump version to 2.1.0"
git push origin master
```

---

## 阶段二：分别编译（在两台机器上并行执行）

### 4a. macOS 构建（在 Mac 上执行）

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code
./docs/build_macos_release.sh
```

脚本自动从 `tauri/package.json` 读取版本号（如 `2.1.0`），产物：

```
tauri/src-tauri/target/release/bundle/
├── macos/SoloSoul.app
├── dmg/SoloSoul_2.1.0_arm64.dmg
└── dmg/SoloSoul_2.1.0_arm64.dmg.sig
```

> 如需覆盖版本号，可传入参数：`VERSION="2.2.0" ./docs/build_macos_release.sh`
> （注意：传入参数不会修改源文件中的版本号，仅影响产物命名）

> 构建脚本会自动调用 `npx tauri signer sign` 生成 `.sig`，需要提前设置 `TAURI_SIGNING_PRIVATE_KEY`。

#### 签名说明

- 默认使用 **ad-hoc 签名**（`codesign --sign -`），**无需 Apple Developer 账户**
- 首次在另一台 Mac 上运行时，需在 系统设置 > 隐私与安全性 中手动允许
- 如需使用 Apple Development 证书：
  ```bash
  APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name" ./docs/build_macos_release.sh
  ```
- **对外公开分发**前，需获取 Apple Developer ID 账户并添加公证（Notarization）步骤

### 4b. Windows 构建（在 Windows 上执行）

在 Windows PC（或 Parallels/VMware 虚拟机）的 **Git Bash**（或 MSYS2 / WSL）中：

```bash
# 1. 先拉取最新代码（确保版本号已更新）
cd /d/PycharmProject/SoloSoul_code
git pull origin master

# 2. 运行一键构建脚本
./docs/build_windows_release.sh
```

脚本会自动安装依赖并构建，产物：

```
tauri/src-tauri/target/release/bundle/
└── nsis/SoloSoul_2.1.0_x64-setup.exe
└── nsis/SoloSoul_2.1.0_x64-setup.exe.sig
```

> 如需覆盖版本号，可传入参数：`VERSION="2.2.0" ./docs/build_windows_release.sh`

> 构建脚本会自动调用 `npx tauri signer sign` 生成 `.sig`，需要提前设置 `TAURI_SIGNING_PRIVATE_KEY`。

> Windows 代码签名需另行购买证书并使用 `signtool` 签名，当前未在脚本中实现。

---

## 阶段三：收集与发布（在 Mac 上执行）

### 5. 收集产物

将 Windows 产物传输到 Mac（如通过共享文件夹、云盘、U盘等），统一放到同一目录，并确保 `.sig` 文件与安装包同名且放在一起：

```
/Users/zzc/PycharmProjects/SoloSoul_code/SoloSoul-Releases
├── SoloSoul_2.1.0_arm64.dmg         # macOS (Apple Silicon)
├── SoloSoul_2.1.0_arm64.dmg.sig     # macOS 更新签名
├── SoloSoul_2.1.0_x64-setup.exe     # Windows (NSIS 安装包)
└── SoloSoul_2.1.0_x64-setup.exe.sig # Windows 更新签名
```

### 6. 本地验证

#### macOS
- 双击 DMG 安装，将 `SoloSoul.app` 拖入 Applications
- 首次启动若提示「无法打开，因为无法验证开发者」，前往 系统设置 > 隐私与安全性 > 安全性，点击「仍要打开」
- 验证 Vault 解锁、对象 CRUD、设置页面等基础功能

#### Windows
- 双击 `.exe` 安装包完成安装
- 从开始菜单或桌面快捷方式启动 SoloSoul
- 验证 Vault 解锁、对象 CRUD、设置页面等基础功能

### 7. 生成 latest.json

在 Mac 上执行：

```bash
cd tauri
node scripts/generate-latest-json.js \
  "$(node -p "require('./src-tauri/tauri.conf.json').version")" \
  ../SoloSoul-Releases \
  ../SoloSoul-Releases/latest.json
```

生成的 `latest.json` 包含各平台安装包下载地址与 Ed25519 签名，供应用内更新器读取。

### 8. GitHub Release 发布

在 **公开库** https://github.com/Gczmy/SoloSoul.git 创建 Release：

1. 点击 "Draft a new release"
2. 选择或创建标签（如 `v2.1.0`）
3. 填写 Release 标题和说明
4. **上传以下 5 个附件**（应用内更新器依赖 `.sig` 与 `latest.json`）：
   - `SoloSoul_2.1.0_arm64.dmg`
   - `SoloSoul_2.1.0_arm64.dmg.sig`
   - `SoloSoul_2.1.0_x64-setup.exe`
   - `SoloSoul_2.1.0_x64-setup.exe.sig`
   - `latest.json`
5. 点击 "Publish release"

> 通过 GitHub Releases 上传，而不是通过 git 提交。GitHub Releases 允许上传附件，这些附件不存储在 git 仓库中。

### 9. 更新公开库 changelog（简洁版本）

在 https://github.com/Gczmy/SoloSoul.git 更新 `CHANGELOG.md`，包含从上次版本到本次版本的所有变更摘要。

### 10. 更新私有库 changelog（详细版本）

在 https://github.com/Gczmy/SoloSoul_code.git 更新 `CHANGELOG.md`，包含详细的变更列表（检查 commit 记录，不要遗漏）。

---

## CI/CD 自动发布（备选）

Push 到 `master` 分支后，GitHub Actions 会自动：

1. `frontend-check` job：TypeScript 类型检查、Lint、单元测试
2. `rust-test` job：Rust 格式化检查、Clippy、单元测试
3. `build-macos` job：在 `macos-latest` runner 上构建 DMG（仅 master push）
4. `build-windows` job：在 `windows-latest` runner 上构建 NSIS（仅 master push）
5. `release` job：收集产物，统一创建并发布 GitHub Release（非 Draft、非 Pre-release），使 `releases/latest/download/latest.json` 立即对客户端可见。

详见 `.github/workflows/ci_cd.yml`。

---

## 附录：版本号速查

| 组件 | 文件路径 | 字段 |
|------|----------|------|
| Node.js / npm | `tauri/package.json` | `"version": "x.y.z"` |
| Tauri 配置 | `tauri/src-tauri/tauri.conf.json` | `"version": "x.y.z"` |
| Rust / Cargo | `tauri/Cargo.toml` | `[workspace.package] version = "x.y.z"` |
