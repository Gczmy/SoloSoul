# macOS 完整客户端隔离验收（FE2）

使用正式 React 前端、Tauri IPC、Vault 解锁和 AppKit 窗口实现；不是 Card 演示窗口。仅在 macOS **debug** 显式启用 `macos-ui-regression`，不能用于发布或与 Windows `native-perf` 混用。

## 构建与准备

在 `tauri/` 目录执行：

```bash
npx tauri build --debug --features macos-ui-regression \
  --config src-tauri/tauri.macos-ui-regression.conf.json --bundles app

# 仅创建新临时目录及一个合成账户，不运行窗口 / Tauri 插件。
target/debug/bundle/macos/SoloSoulFE2Mac.app/Contents/MacOS/solo_soul --fe2-macos-prepare

# 启动时额外禁止读取 / 写入正式默认 Vault、日志、缓存、WebKit 与偏好路径。
python3 native-regression/macos-client/run.py --root <上一步输出的规范化路径>

# 如需由电脑控制工具按标准 macOS 应用启动，先绑定独立测试 bundle。
python3 native-regression/macos-client/run.py --bind --root <上一步输出的规范化路径>
```

准备命令输出规范化临时根目录，名称为 `solosoul-fe2-macos-<UUID>`。直接启动时将该路径赋给 `SOLOSOUL_FE2_MACOS_ROOT`。标准 LaunchServices 启动不继承该环境变量，绑定命令先执行无窗口 preflight，再以 0600 / create_new 写入独立测试 bundle 的 `Contents/Resources/fe2-macos-root.txt`；已有绑定不能静默覆盖。没有环境变量或有效绑定文件时不启动。不要改动 `HOME`、拷贝正式账户或使用正式客户端的 bundle 标识。

合成账户：`FE2 macOS 合成验收账户`；公开测试密码：`FE2-Mac-Synthetic-2026!`。密码只用于新建测试数据，不能用于真实账户。登录需走实际界面的密码验证，不注入解锁状态或 mock IPC。

## 隔离与边界

- `<root>/vault`：合成账户、加密 profile 与桌面 UI 偏好。
- `<root>/app-data`：日志、启动清理及注册表刷新时间。
- `<root>/plugins`：独立插件 owner、安装状态和审计记录。失败直接停止，不回退到用户目录 / 共享临时插件目录。
- WKWebView 使用非持久 `WKWebsiteDataStore`，不复用正式 localStorage / cookies。
- 编译配置与 bundle 标识是 `com.solosoul.fe2.macos`；运行时 Tauri 标识再追加该次 UUID，窗口状态、更新缓存和导入缓存不会指向正式标识。
- 启动拒绝规范化临时根之外的目录、软链接、开放给其他用户的目录、marker 不匹配、非夹具账户和孤儿账户目录。设置了外部 `SOLOSOUL_DATA_DIR` 也会拒绝启动。

启动器先通过合成目录验证 sandbox 对读写的拒绝及自身输出的正向控制；不能把 sandbox 进程本身启动失败当作保护通过。正式 OCR 默认缓存目录也被禁止，当前不验 OCR。该本地保护不是生产 App Sandbox。

上述额外 sandbox 只适用于 `run.py --root` 直接启动的进程。电脑控制工具的标准应用启动不继承该外部 sandbox，依靠 `macos-ui-regression` 的同一套路径隔离与启动拒绝校验。记录中必须区分两种启动方式，不能将沙箱检查结果套用到标准启动。

该入口不隔离所有操作系统服务：Touch ID / Keychain、系统文件选择器和用户的辅助功能设置仍属于当前 macOS 用户。当前测试只使用合成账户的主密码和应用内 UI，不配置生物识别、不导出到真实用户目录、不修改系统偏好。未执行的这些场景不能因隔离入口可用而标记通过。

## 需要记录的实机证据

记录当前前端与二进制 hash、macOS 版本、启动路径、实际测试步骤和退出状态；保存登录 / 解锁 / 锁定、完整顶栏、侧栏、浮层和主题的截图。Dock / 台前调度恢复的闪黑须保留能覆盖恢复过程的帧，单张稳定截图不能证明没有闪黑。

四种导航、工具菜单与新建页面定位、预览 / 通知、窄矮窗口、拖拽边界、减少透明度 / 高对比 / 减少动态效果分别验收。浏览器或独立材质例程的结果不能代替完整客户端证据。

此文档描述验收入口与要求，本身不是通过记录；实际结果写入第二轮前端执行报告与 checkpoint。
