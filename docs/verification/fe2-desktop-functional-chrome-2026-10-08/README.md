# FE2 功能文字、主题边界与桌面外壳调整

本次按用户追加要求实施：功能和主内容文字统一按浅深模式使用中性黑白；配色方案仅控制背景与边框，强调色和语义色分别管理。删除桌面导航及移动顶栏的 SoloSoul 品牌图标，保留导航图标、登录和关于标志。展开侧栏从232缩至186px，折叠宽度和交通灯安全区保持。

macOS浅色外壳覆盖率70%，导航底色混入14%黑色；深色外壳覆盖率65%。浅色登录底板改为背景色78%与黑色混合，避免正文中性色使底板变黑；登录外壳/底板90%、卡片80%仍保留。Windows导航增加浅深独立Mica色调与solid回退，仅有浏览器验证。

## 最终来源与验证

- `final-macos-source.json`：最新122项构建来源；`final-macos-report.json`：Debug构建退出0、九组保护输入恢复。此前6项Rust夹具测试来源未变，本次复用，不声称重新执行。
- `final-unit.log`：6文件82项通过；类型检查及涉及文件ESLint退出0。
- `final-browser-regression.log`：51项通过，覆盖文字、材质、登录回退、186px侧栏、菜单和标题栏几何。`final-browser/`的31张PNG全部来自浏览器，不是原生截图。
- `final-native-observation.json`：最新818ba4df隔离包真实深色首页、浅色登录/首页及深色恢复。`final-macos-native-provenance.json`：PID15190、0700合成根/0600绑定、测试Vault/日志/插件锁以及公开附件SOLC加密元数据。
- `theme-boundaries-before-compact-regression.log`：71项通过，属于最后186px/65%/浅色底板调整之前；`theme-boundaries-mobile-regression.log`：6项通过，同样保持阶段来源。

## 首次失败及历史范围

`browser-regression-first.log`保留39通过/2失败：删除折叠品牌行后工具菜单失去旧锚点。修正为导航实际顶部加paddingTop后41项通过，日志另存。`browser-regression-light-base-final.log`为中间16项结果，`browser/`9张PNG为正文中性色全局收敛前的阶段截图，不提升为最终来源。此前原生59ae178b、a99e2d78及e811观察保持冻结来源；原附件缩放采集缺失尚未关闭。

原生图像仅在CUA会话，不归档PNG或连续帧，也不证明实体性能、系统辅助功能、Windows原生或新移动端生产包。标准CUA启动不声称继承外部sandbox；lsof仅提供采样时点路径证据。完整目标仍14项、8完成、6待验证，未提交、推送或发布。
