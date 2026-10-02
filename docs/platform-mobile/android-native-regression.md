# Android 原生回归设备入口

RF-310 使用 `tauri/scripts/android-native-regression.mjs`，要求显式提供专用模拟器 serial 和 AVD 名称；不会自动选择首个设备或连接用户手机。只使用 Debug APK，不读取发布签名密钥。不覆盖正式 macOS 客户端或真实账户。

## 构建与执行

在 `tauri/` 中使用已配置的 Node、rustup、Android SDK/NDK、JDK。ARM64 示例：

```bash
npx tauri android build --debug --target aarch64 --split-per-abi --apk --ci
cd src-tauri/gen/android
./gradlew tasks --all
./gradlew :app:assembleArm64DebugAndroidTest -x rustBuildArm64Debug
cd ../../..
node scripts/android-native-regression.mjs \
  --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5586 --avd SoloSoul_RF201 --mode supported \
  --apk src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk \
  --test-apk src-tauri/gen/android/app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk \
  --output /tmp/solosoul-native-supported
```

再以 `--mode fallback` 和新的输出目录执行一次。输出目录必须不存在，避免覆盖证据。最低 API 31，ABI 为 ARM64 或 x86_64；本次 CI 固定 API34 / Google APIs / x86_64，支持与回退分成两个矩阵任务。x86_64 对应 `--target x86_64`、`assembleX86_64DebugAndroidTest` 和 x86_64 APK 路径。

## 原生断言与失败判定

- `supported`：实际系统 blur 必须可用；验证原生菜单选择、取消、暂停恢复、系统临时禁用 blur 时的可读性。
- `fallback`：设置系统 `disable_window_blurs=1`，实际能力必须不可用；验证插件返回 `unavailable`，不留下原生菜单，交由现有前端回退。此测试不把原生层拒绝打开误称为网页回退视觉验收。
- 两种环境均执行真实系统主题 light/dark 切换、显式 light/dark 偏好回归，以及资源安装和异步就绪回归。支持模式 11 项、回退模式 9 项，分别包含 1 项环境断言；不运行应跳过的另一模式用例。

每个方法独立 instrumentation 进程，并关闭框架自动清理 Activity；获得最终报告后由驱动 force-stop 测试应用，避免 WebView/Tauri 收尾使成功报告丢失。严格要求指定方法恰好 1 项通过；跳过、零测试、进程崩溃、设备不存在/离线、身份不符均 exit1。

`report.json` 记录设备 serial、AVD、API、ABI、APK SHA256、逐项耗时和结果；`junit.xml` 将设备初始化和收尾失败也表示为失败。保存 instrumentation 原文和失败截图，并拉取测试产生的玻璃/主题截图与 JSON。执行结束恢复原系统 blur 设置；主题用例自行恢复夜间模式和应用主题偏好。仅允许专用测试设备，测试中会修改该测试应用的资源和主题。

## CI 边界

`.github/workflows/build-android.yml` 提供 `android-native` 独立矩阵任务及手动入口，结果无论成功失败都上传。镜像由 [android-emulator-runner 官方配置](https://github.com/ReactiveCircus/android-emulator-runner) 启动，保留动画，Ubuntu 使用 KVM。完整工作流目前被维护者禁用；增加入口不会自动启用或触发远端任务。本地设备结果和远端 CI 结果分别记录，不将配置存在当作实际远端通过。

## RF-121 Card 原生补证（2026-10-02）

两组各新增独立 Debug WebView Card 检查：生产 Card/CardGrid、Android token 和平台 CSS，真实帧后检查浅深色溢出并在原生截图核对内部背景像素。专用 Activity 不初始化账户、禁用网络；Debug Gradle 自动生成内联资源，Release manifest 与任务图排除该入口。API34 ARM64 支持11/回退9均通过，10项Kotlin单元检查通过。[完整证据](../verification/rf121-android-native-checkpoint-2026-10-02.json)。默认对比度与正常动画不代替辅助功能、完整文字像素或多版本验收。
