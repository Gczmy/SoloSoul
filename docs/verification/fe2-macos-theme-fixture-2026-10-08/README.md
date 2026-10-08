# macOS 深色夹具修正与原生导航补验

2026-10-08。发现原隔离夹具把 `defaultLightTheme`、`defaultDarkTheme` 都设成 `warm-stone`，导致深色单选项已保存、画面却仍浅色。正式默认值及 `settingsThemeConfig` → `applyTheme` 的映射正确，因此只修正 macOS Debug 验收夹具为 `warm-stone-dark`，没有改生产主题解析或偏好链路。夹具账户/路径/权限拒绝规则保持。

原6410…包在当前合成账户中经真实「更多外观 → 暖石深」切为深色，随后实际固定浅色恢复。深色左右/上下导航均取得稳定画面；右/上/下工具真实展开；上/下添加页面弹层在菜单收起和真实拖动图标滚动条后边界保持，取消实际关闭，未创建测试页面。见 [前包原生观察](native-cua-observation.json)。这些是当前会话工具输出的人工作用范围，不是自动测试全通过，也不升级前包构建来源。

首轮普通Cargo夹具测试因 Tauri manifest / macOS private-api 配置校验失败，尚未运行单测，保留 [原始失败](initial-config-rejection/report.json)。第二轮仅对不创建窗口的Cargo测试使用已有规范 `TAURI_CONFIG.app.macOSPrivateApi=false`；**6测试通过，0失败，测试退出0**。随后的Tauri CLI实际图形构建保留正常macOS配置，**构建退出0**。9组受保护平台输入恢复，当前106份构建/执行来源无差异。见 [最终构建报告](final-build/report.json)、[测试原文](final-build/fixture-test.log)。

新冻结Debug包二进制 `e811729b523619ef2b5ca6b0d2ce7f5e0e2f5dbc822af48d4eef1671e023ae27`。实际prepare/preflight均0，新0700合成根与0600显式绑定、浅暖石/深暖石深配置已核对。旧PID49150通过正常Quit后只读确认已不存在；新包由CUA标准启动，实际PID83926/冻结执行路径及合成账户ID与marker核对一致。实际密码解锁进入首页、打开外观页后，直接选「深色 · 暖石深」即取得真实深色顶栏、侧栏、正文和卡片，磁盘偏好回读dark/warm-stone-dark。见 [新包观察](current-native-observation.json)。完整app与私有根/绑定/备份仅/tmp。

CUA阶段保留暂态捕捉失败、后台台前调度缩略图及一次窗口坐标不可达；同一实例重新绑定/窗口Raise后继续，没有因此重启或停止其他应用。1200x800原生图像仅在会话工具输出，未导出PNG、没有图像hash；捕捉标识遮住交通灯，因此未宣称该区域完整像素通过。尝试继续锁定材质时系统再次锁屏，应用锁定动作未取得结果，不能写成深色登录通过。

本轮只有夹具源码修正，不影响移动端二进制；保留它们原有冻结来源，不重建无关目标。完整macOS预览/登录方法/窄矮窗/系统辅助功能/连续恢复与实体性能仍待验。14任务、8完成、6待验证，目标继续；未提交、推送、发布。
