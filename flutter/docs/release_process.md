# SoloSoul 发布流程（macOS + Windows）

> 同时发布 macOS DMG 和 Windows ZIP，版本号严格同步。

---

## 环境要求

- **macOS**：Xcode、Rust、Flutter、create-dmg
- **Windows**：Visual Studio 2022+、Rust、Flutter、Git Bash（用于运行 build_windows_zip.sh）

> 注：macOS 包必须在 Mac 上编译，Windows 包必须在 Windows 上编译，无法在一台机器上完成双平台。

---

## 阶段一：准备（在 Mac 上执行一次）

### 1. 检查私有库同步状态

确认 https://github.com/Gczmy/SoloSoul_code.git 与本地状态相同，如有未推送的本地更新，优先推送。

```bash
git status
git push origin master
```

### 2. 确认并统一版本号

修改 `pubspec.yaml` 的 `version` 字段为下一个版本号（遵循 [SemVer](https://semver.org/lang/zh-CN/)）：

```yaml
version: 1.1.0+1
```

> 版本号格式：`主版本.次版本.补丁+构建号`。macOS 和 Windows 使用完全相同的版本号。

### 3. 推送版本号更新

```bash
git add pubspec.yaml
git commit -m "chore: bump version to 1.1.0"
git push origin master
```

---

## 阶段二：分别编译（在两台机器上并行执行）

### 4a. macOS 构建（在 Mac 上执行）

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/flutter
./build_dmg.sh 1.1.0
```

产物：`build/macos/SoloSoul-v1.1.0.dmg`

### 4b. Windows 构建（在 Windows 上执行）

在 Windows PC（或 Parallels/VMware 虚拟机）上：

```powershell
# 1. 先拉取最新代码（确保 pubspec.yaml 版本号已更新）
cd D:\PycharmProject\SoloSoul_code\flutter
git pull origin master

# 2. 执行构建脚本（使用 Git Bash）
bash ./build_windows_zip.sh 1.1.0
```

产物：`build/windows/SoloSoul-v1.1.0-windows-x64.zip`

> 注：Windows 脚本在 Git Bash 中运行。如果未安装 Git Bash，可使用 WSL 或手动执行脚本中的命令。

---

## 阶段三：收集与发布（在 Mac 上执行）

### 5. 收集产物

将 Windows 产物传输到 Mac（如通过共享文件夹、云盘、U盘等），统一放到同一目录：

```
~/SoloSoul-Releases/
├── SoloSoul-v1.1.0.dmg
└── SoloSoul-v1.1.0-windows-x64.zip
```

### 6. 本地验证

#### macOS
- 双击 DMG 安装，确认应用能正常启动
- 验证 Vault 解锁、对象 CRUD、附件上传等基础功能

#### Windows
- 解压 `SoloSoul-v1.1.0-windows-x64.zip`
- 进入 `SoloSoul/` 文件夹，双击 `solosoul_flutter.exe` 启动
- 验证 Vault 解锁、对象 CRUD、附件上传等基础功能

### 7. GitHub Release 发布

在 https://github.com/Gczmy/SoloSoul.git 创建 Release：

1. 点击 "Draft a new release"
2. 选择或创建标签（如 `v1.1.0`）
3. 填写 Release 标题和说明
4. **上传两个附件**：
   - `SoloSoul-v1.1.0.dmg`
   - `SoloSoul-v1.1.0-windows-x64.zip`
5. 勾选 "Set as a pre-release"（如为预览版）或 "Publish release"

> 通过 GitHub Releases 上传，而不是通过 git 提交。GitHub Releases 允许上传附件，这些附件不存储在 git 仓库中。

### 8. 更新公开库 changelog（简洁版本）

在 https://github.com/Gczmy/SoloSoul.git 更新 CHANGELOG.md，包含从上次版本到本次版本的所有变更摘要。

### 9. 更新私有库 changelog（详细版本）

在 https://github.com/Gczmy/SoloSoul_code.git 更新 CHANGELOG.md，包含详细的变更列表（检查 commit 记录，不要遗漏）。

---

## CI/CD 自动发布（备选）

Push 到 `master` 分支后，GitHub Actions 会自动：

1. `build-macos` job：在 `macos-latest` runner 上构建 DMG
2. `build-windows` job：在 `windows-latest` runner 上构建 ZIP
3. `release` job：收集产物，统一创建 GitHub Release（Draft + Pre-release）

详见 `.github/workflows/ci_cd.yml`。
