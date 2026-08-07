# Changelog

All notable changes to SoloSoul are documented in this file.

## [2.8.6] - 2026-08-07

### Added

- **设备同步「同步界面偏好」勾选框** — 同步页新增勾选「同步界面偏好」（主题、主题色、窗口大小等 UI 外观偏好），由用户决定是否随设备同步：勾选 → 发送侧剥离本机 UI 外观偏好、接收侧保留本机偏好（主题等不改动本机）；不勾选 → 偏好不参与同步。偏好写入 VaultService 热替换后开关自动重应用。
- **已发现设备卡片增强** — 已发现设备卡片显示与客户端类型对应的设备图标（手机/笔记本），并支持点击打开详情弹窗（与已知设备一致，同一设备详情一致）。
- **云端 LLM 隐私提示完整方案（P035）** — ① provider 编辑表单在 baseUrl 非本地（localhost/127.0.0.1）时显示橙色提示「对话内容将发送至该第三方服务，请确认其隐私政策」；② 首次启用云端 provider 前弹确认框（说明数据将发送至第三方服务、不经 SoloSoul 服务器中转），确认后同一设备不再重复拦截；本地 LLM 服务器（Ollama/LM Studio）不拦截。
- **R2 代码审计新增 35 项修复闭环（P000-P035）** — 安全（P001 导出落盘路径白名单、P002 删除 legacy XOR 迁移路径、P003 APK 校验和独立签名、P012 密码验证浮层收敛、P015 导入/导出/恢复密码 Zeroizing、P031 LLM SSRF 内网段防护、P032 主密码解锁阶梯限流、P033 CSP 收紧、P034 前端密码 state 提交后置空）+ 性能（P004 插件 FieldResolver 惰性缓存、P005 导出附件批量命令、P007 附件复制/下载 spawn_blocking、P022 WASM 编译进程级缓存、P023/P024 重 IO spawn_blocking、P025 reqwest Client 共享、P026/P028 配置/字体缓存、P030 LLM 120s 超时）+ 重构（P016-P021、P036-P044 前端掩码/类型/对话框/吞错收敛）。

### Fixed

- **LLM 配置页内容约 1 秒后消失** — 后端 `llm_get_embed_models` 以 `#[serde(flatten)]` 序列化为扁平 snake_case 形状，前端类型误写为嵌套 camelCase 且渲染 `m.info.id`：真实数据（需网络拉取注册表约 1s）到达后重渲染抛 TypeError → 无 ErrorBoundary → React 整树卸载。前端对齐后端真实形状（扁平 + snake_case）+ 冒烟测试改用真实形状数据（负向验证确定性复现原崩溃），成为防回归测试。
- **设备同步单向发现** — Android 能发现 macOS 但 macOS 看不到 Android：桌面发现层对无账户信息的 mDNS 服务放行展示（会话层仍严格校验 account_id，SAS 验证码兑底安全不受影响），安卓端广播补发 `account_hash`（TXT 可达时按哈希过滤）。
- **同步完成 toast 去重** — 同一 peer 短窗口合并/丢弃 0 条 toast，macOS 端不再连续弹出 4 条「对端同步完成：检查 0 条」；响应方展示完整交换条数（发回条数由发起方汇总）。
- **同步握手失败 detail i18n** — 「与设备握手失败：vault is locked」等后端英文错误不再原样透传，纳入 i18n 翻译。
- **首页自定义卡片标题溢出** — 全数字长名称达到最大长度时轻微溢出卡片宽度，改为单行截断 + ellipsis。
- **账户来源浮层「返回」改「返回登录」** — 创建新账户页的账户来源决策卡片「返回」按钮与「不，创建新账户」功能重叠，改为「返回登录」直接回登录页（首启无账户场景特殊处理：仅关闭浮层）。
- **已发现/已知设备卡片细线边框** — 与卡片背景区分，视觉层级更清晰。
- **同步状态卡设备信息紧凑排列** — 设备名/指纹/监听地址三行行间距过大，合并为单容器紧凑布局。

### Performance

- **插件 FieldResolver 惰性缓存（P004）** — 消除每字段全表解密放大。
- **导出附件勾选批量命令（P005）** — N+1 逐对象查询 → `export_get_attachments_batch` 单次批量。
- **附件复制/下载 spawn_blocking（P007）** — 大文件拷贝不再阻塞 tokio worker。
- **WASM 编译进程级缓存（P022）** — 内容哈希键 + `Arc<Module>`，重复编译归零。
- **重 IO 移出 async runtime（P023/P024）** — log_export、fs_scan_directory 移入 spawn_blocking。
- **reqwest::Client 进程级共享（P025）** — OnceLock 单例 + 请求级 timeout。
- **PDF 水印字体 / OCR 配置缓存（P026/P028）** — 消除每次 10-50MB TTC 读盘与每次扫描重读配置。

### Refactor

- **auto_sync_core 泛型调度内核（P017）** — SAF/设备双自动同步状态机合并。
- **超长函数 7 处拆分（P018）** — 含 session 双端共用 helper 去重、AppState::new 拆分。
- **前端掩码规则统一（P036）** — 三处收敛到 `lib/masking.ts` 单一规则源。
- **前端镜像类型 10 组收敛（P037）** — 4 个新单源类型文件 + ObjectSummary 语义重命名。
- **确认对话框 5 处收敛（P040）** — 薄封装统一到 `ui/ConfirmDialog`（portal/滚动锁定/Escape/动画）。
- **AppRoutes 职责拆分（P041）** — 提取 useAppUpdate + useOcrFirstInstall。

### Chores

- 版本号同步升级到 2.8.6（versionCode 2008006）。
- 59 个 commit 自 v2.8.5（`c73b119`）到 v2.8.6。

## [2.8.5] - 2026-08-06

### Added

- **设备同步 9 项改进（P0#1-P0#5 + P1#7-P1#10）** — ① 自动同步开关持久化（重启后恢复，消除「用户以为开着其实已悄悄关闭」的静默失效）；② 「离线」文案修正为「未在局域网发现」并附最近同步时间，补齐缺失的 sync_last_seen i18n；③ 设备列表停留期间每 15s 自动刷新（告别过期快照）；④ 同步历史持久化（重启保留最近 10 条）；⑤ 在线状态心跳化（sync_peers.last_addr schema v25，周期同步触达刷新在线状态）；⑥ 离线 peer 2s TCP 探测快速跳过（告别 10s 连接超时拖慢整轮）；⑦ 移动端 conflicts 回传对齐桌面端。
- **「返回账户来源选择」独立浮层** — 创建新账户页返回账户来源选择不再重挂整个引导向导（此前会故意把向导跳到末步、点「返回」后露出停在末步的引导卡片且无关闭出口）；改为独立轻量浮层，仅含决策卡片 + 恢复对话框，「返回」= 关闭浮层露出创建账户表单。

### Changed

- **CLI 成功/信息语义收敛（R2-W2/X2）** — 新增中性 info_message overlay（标题「信息」、非红色、Esc 关闭）；「同步成功」改 success_message toast；plugin 市场空态/会话/已装/审计列表、导入预览、模型已装等 9 处从红色 error overlay 迁移，全库语义 grep 验收残留 0。
- **CLI 解锁样板收敛（R2-W3/X3）** — 8 处「require_unlocked + get_vault_store」双行样板收敛为 `require_unlocked_with_vault` 单行；`/purge` 补解锁门禁（锁定用户不再得到原始 eyre）；helper 错误消息统一中文「Vault 未打开」。

### Fixed

- **Windows 密码管理器自动填充泄漏明文** — 登录页密码框 `autoComplete="current-password"` 触发系统密码管理器填充历史密码并显示明文；登录/密码验证/PIN 设置/生物识别 5 处验证场景全部改为 `off`，恢复创建账户的新密码框补 `new-password`。
- **安卓自动填充后输入框白底** — `-webkit-autofill` 深色模式下自动填充后输入框底色变白风格不匹配，补主题适配覆盖。
- **回收站批量恢复失败不再吞错（R2-V3）** — 内层 catch 重新抛出，对话框保持打开可重试（已恢复项幂等）。
- **CLI 按键错误不再退出 TUI（R2-V1/V7）** — 按键 handler 错误统一捕获为 overlay；`/list` 截断提示 + shared_runtime() 优雅报错。
- **附件路径 symlink 旁路硬化（R2-V8a/W1/X1）** — src_raw 字面路径仅在 canonicalize 失败时参与判定；`attachment_copy_to_vault` 真修复（非 canonical base 比较 + 路径判定纯函数 `path_within_base` + 3 条防回归测试）。

### Performance

- **CLI 列表导航消除每按键克隆（R2-V5）** — 5 处列表导航 handler 消除 items Vec 每按键深拷贝。
- **known_peers 去冗余 clone（审查反馈）**。

### Chores

- 版本号同步升级到 2.8.5（versionCode 2008005）。
- 37 个 commit 自 v2.8.4（`9014fd3d`）到 v2.8.5。

## [2.8.4] - 2026-08-06

### Added

- **R2 全库代码分析修复** — 新一轮全库分析（28 项）全部闭环：P0 附件路径校验移除字符串前缀回退（R2-01）、prompt 敏感输入改 `Zeroizing<String>` 承载（R2-04）、成功消息改用 success_message toast（R2-26）等 27 项修复完成，>400 行大文件重构列入长期候选随功能迭代顺带处理。

### Changed

- **设备同步双向发现修复** — ① 安卓端「已发现设备」显示名由 `node_<uuid>` 改为可读设备名 `SoloSoul-<指纹前缀>`（优先指纹派生，回退 NSD 服务名/节点名，与桌面端规则一致）；② macOS 端发现不到安卓端：Android NSD 广播的 TXT 属性（account_hash/account_id）存在不传播到标准 mDNS 客户端的已知互操作限制，桌面发现层对无账户信息的服务改为放行展示（会话层仍严格校验 account_id，配对有 SAS 验证码兑底，安全不受影响），同时安卓端广播补发 `account_hash`（SHA-256 前 16 字节 hex，与桌面端算法逐位一致，实测 Java `%02x` 对 Byte 输出与 Rust `hex::encode` 一致），TXT 可达时直接按哈希过滤。
- **双向同步完成通知** — 响应方（被连接端）成功完成同步会话后，通过 `sync-completed` 事件全局提示「同步完成 + 具体条数」（与发起方 toast 对称），用户不在同步页也能收到；结果同步写入「与设备同步」结果行，并刷新对端列表与冲突计数。
- **CLI 成功消息改用 success_message toast（R2-26）** — 导出/导入成功、密码修改/提示更新、生物识别启用/禁用等 22 处成功消息原先复用红色错误 overlay，现改写入 success_message toast，语义与样式分离。
- **CLI 解锁样板收敛（R2-28）** — 6 处「require_unlocked + get_vault_store().ok_or_else」双行样板合并为 `require_unlocked_with_vault` 单行。
- **t(key)||兜底 死兜底改 defaultValue（R2-18）** — i18next 缺 key 返回 key 本身（truthy），100 余处 `t('key') || '文本'` 的 `||` 右侧实为死代码，统一改为 `t('key', { defaultValue })`，缺 key 时兜底文案真实生效。
- **useExportScope 附件加载三处重复收敛（R2-17）** — togglePage / loadSelectedAttachments / bulkSelect 近乎逐字重复的 N+1 附件加载块收敛为单一共享 helper。

### Fixed

- **版本更新横幅字节数顺序错乱** — 下载进度 `27.0 MB / 44.2 MB` 被 bidi 重排显示成 `MB / 44.2 MB 27.0`：为防抖动右对齐误加了 `direction: rtl`，RTL 方向对纯 LTR 的「数字+单位」文本做了 bidi 翻转。移除 `direction: rtl`（右对齐由 `text-align: right` 承担，防抖动由 `min-width` + `tabular-nums` 承担），字节数恢复「数字 单位 / 数字 单位」顺序，并加防回归单测（断言 DOM 顺序与 direction 非 rtl）。
- **附件路径校验移除字符串前缀回退（R2-01，P0）** — 附件路径白名单校验此前在匹配失败时回退到「字符串前缀」宽松判定，可被 `..` 与同前缀目录绕过，现移除回退仅保留规范化后的严格路径判定。
- **CLI 命令错误改显示 overlay 不再退出 TUI（R2-03）** — 未知命令/参数错误原先直接退出终端界面，改为在状态栏显示错误 overlay。
- **CLI 裸输 /plugin_run 不再越界 panic（R2-02）** — 无参数调用时原先按空参数数组越界读取，现做参数数量守卫并提示用法。
- **回收站确认操作失败不再 unhandled rejection（R2-09）** — 永久删除/恢复失败时对话框关闭且无提示，现加 submitting 防重复提交 + 失败 toast + 保持对话框可重试。
- **重命名失败回滚乐观更新（R2-19）** — AttachmentViewer 重命名失败后界面显示新名、后端仍是旧名（状态不一致），现失败回滚原名 + toast 提示。
- **purge_trash 底层删除失败不再产生孤儿对象（R2-07）** — 回收站清空时若底层删除失败，本地状态同步移除导致后端残留不可见对象，现失败时回滚本地列表。
- **导入偏好保存失败不再被吞没（R2-06）** — 导入完成后偏好写入失败原先静默，现报错提示。
- **TTC 字体改内存加载（R2-05）** — 水印渲染临时字体文件改用内存加载，消除临时文件生命周期隐患。
- **CLI 导出审计日志文件权限收紧 0600（R2-24）** — 审计日志导出文件不再跟随 umask，显式 0600。
- **CLI 日志白名单过滤 + 按日轮转（R2-25）** — CLI 日志落盘前白名单过滤敏感字段，按日期轮转避免无限增长。

### Performance

- **同步附件清单收集消除 N+1 全量解密（R2-08）** — 附件清单收集由逐对象全量解密改为批量加载，同步性能提升。
- **trashStore permanentDelete 并发上限 8（R2-20）** — 清空大量回收站条目时不再瞬间发起数百 IPC，改并发 worker 池限流。
- **CLI phase 借用匹配消除每次按键整 phase 深拷贝（R2-10）** — 渲染路径改为借用匹配，消除按键时全状态深拷贝。
- **CLI Theme 缓存 + /list 200 截断（R2-22）** — 主题读取缓存、对象列表超 200 条截断提示，避免大库卡顿。
- **CLI tokio 运行时收敛为进程级单例（R2-23）** — 消除每次命令初始化运行时的开销。
- **Rust 轻项四连（R2-15）** — 流式导入逐步提交、TTC 字体缓存、迁移失败错误归因、示例密码随机化。

### Chores

- **死代码清理** — 删除 LLM 上下文 build_context 死代码子树（R2-11）、needs_rebuild 与 PluginRegistry::from_path（R2-12）、CLI render_empty 与 CliError（R2-27）、前端多余 export baseSteps/resolveFieldLabel/formatTimeValue（R2-21）。
- 提取共享 allowed_fs_bases 白名单 helper（R2-14）。
- 版本号同步升级到 2.8.4（versionCode 2008004）。
- 34 个 commit 自 v2.8.3（`0fc3ab15`）到 v2.8.4。

## [2.8.3] - 2026-08-05

### Added

- **设备同步配对 SAS 验证码** — 双侧确认配对时，两端各自从 Noise 握手派生同一 6 位验证码（大号 3-3 分块展示）供目视比对，替代无法相互对照的 32 字符指纹；无验证码场景（手动配对/旧客户端）回退显示指纹，旧客户端流程完全兼容。

### Changed

- **同步冲突对话框布局重构** — 恢复常规整窗滚动（外部滚动条）；冲突列表不再被裁切，顶部显示「共 N 条冲突」计数提示。
- **同步冲突位置导航** — 详情头部显示「冲突 X / N」+ 上一条/下一条切换按钮，无需滚动列表即可逐个查看全部冲突。
- **同步冲突「只看差异」切换** — 顶栏开关开启后仅保留有差异字段行并显示「共 N 处差异」计数，无差异时显示明确空态。
- **冲突底部操作按钮悬浮固定** — 「保留本地/保留远程/忽略」在滚动时始终悬浮贴卡片底边可见，无需滚到底即可快速处理。
- **冲突 diff 差异行高亮** — 对象/数组字段按叶子逐行展开配对，有差异的具体行（含单侧缺失）以浅红背景高亮，一眼定位差异值。
- **冲突 diff 全面 i18n** — 表名（objects → 对象）、字段名（35+）、属性内部元数据键（__templateName → 模板名称）、属性类型代码（date → 日期）、模板/对象类型与图标值、敏感度等级（渲染为彩色徽章）均不再显示原始代码/键，未知项标题化兑底。
- **冲突 diff 去除紧凑 JSON** — 嵌套对象逐行「字段: 值」、数组逐行「- 项」；`__fields` 字段定义无差异时折叠为一行摘要，仅模板结构真正变化才展开。
- **冲突 diff 省略不可感知元数据** — 对象指纹、空子对象/父对象、簿记字段（版本/修改时间）不再展示，「共 N 处差异」只计真实内容差异；时间字段精确到秒。
- **假冲突自动消解** — 两台设备内容已收敛、仅版本/修改时间不同时不再产生冲突（保留较新一方，无数据丢失）；删除墓碑为真实决策仍照常记录冲突。
- **设备名展示优化** — 已发现设备名统一为「SoloSoul-<指纹前缀>」（替代 node_<uuid>），不再溢出屏幕；macOS 设备图标改为笔记本图标，与客户端类型一一对应。
- **移除 OCR「当前模型」冗余文案** — 侧边栏扫描卡片与扫描页的模型系列标题删除。

### Fixed

- **macOS Vision 扫描报错** — 修复「Vision CLI 异常退出：Cannot load image at --」：Rust 侧误传 `--` 参数分隔符导致 Swift 端把 `--` 当图像路径，现直接传真实路径并防御性跳过前置 `--`。
- **已知设备详情卡片操作不刷新** — 点击「撤销信任/信任并配对」后卡片立即更新，无需重新进入详情。
- **冲突 diff 属性名换行** — 属性名标签在窄列下不再断行，长值在标签后正常换行。

### Chores

- 版本号同步升级到 2.8.3（versionCode 2008003）。
- 30 个 commit 自 v2.8.2（`df8a6e5e`）到 v2.8.3。

## [2.8.2] - 2026-08-05

### Added

- **覆盖恢复进度条** — 「从其他设备恢复」覆盖流程显示实时进度（下载 0-40% / 覆盖 45% / 创建 50% / 导入 50-95% / 完成 100%），导入阶段按对象与附件单调推进；完成后弹确认框，用户确认后返回登录页并展示刚恢复的账户。

### Fixed

- **Android SAF 目录删除后重启无提醒（回归修复）** — 自动更新安装后丢失 SAF 授权失效检测状态，重新注册检测逻辑，每次启动都提示「SAF 目录访问已失效」横幅（不再只提醒一次后静默）。

### Changed

- **全库确定进度条统一渐变** — 恢复进度、版本更新横幅（UpdateBanner/OcrInstallBanner/UpdateInfoCard/MandatoryUpdateOverlay）、SAF 目录加载、Embedding 下载、拖拽上传等全部确定进度条统一为「主题色→暖黄」渐变（原生 `<progress>` 替换为自绘 div，消除默认绿色），随 accent 预设联动。
- **版本更新横幅字节数防抖动** — `tabular-nums` 等宽数字 + 固定最小宽度 + RTL 右对齐，下载中数字位数变化不再推动进度条与文字左右晃动。
- **已知设备卡片悬停动画** — 设备同步已知设备项加入 `interactive-card-lift` 悬停效果（上浮 + 主题色 ring + 阴影），与 workspace 对象卡片同源交互语言。

### Security

- **npm 供应链漏洞修复（`bb19e77c`）** — react-router 7.17.0→7.18.2（生产依赖，open redirect CVE-2025-68470 绕过/XSS/构造函数注入/路由 DoS）、undici 7.27.1→7.29.0、postcss→8.5.25、brace-expansion→5.0.9。keyv/cacheable 命名空间供应链攻击确认不受影响；剩余 2 个 high（RSC Mode CSRF）为纯客户端 SPA 不可利用面，暂缓 v8 升级。

### Chores

- 版本号同步升级到 2.8.2（versionCode 2008002）。
- 9 个 commit 自 v2.8.1（`b5af6cc2`）到 v2.8.2。

## [2.8.1] - 2026-08-05

### Fixed

- **搜索卡片跳转闪烁根治** — `BrowserRouter useTransitions={false}` 关闭 react-router v7 默认的导航 `startTransition`（其更新无法被 `flushSync` 强制冲刷），导航与弹层关闭同帧提交，搜索卡片跳转不再先闪出底层页面；同类弹层跳转（AI 聊天进入完整页等）一并修复。
- **全局搜索不再命中内部元数据键** — `_dynamic_group_`（搜「动态字段组」可命中）与 `__templateHash`（模板指纹哈希）键与值均不参与搜索，消除「字段名：__templateHash」噪声结果。
- **导出附件展开图标** — 无附件对象不再显示展开箭头（后端 `has_attachments` 逐对象提供）；修复该字段 snake_case 序列化导致的回归（有附件对象图标也消失），加防回归单测。
- **桌面端生物识别解锁行恢复解锁方式说明** — 图标右侧「使用 X」说明在开启后恢复显示（安卓端保持隐藏）。
- **PIN 解锁加载动画位置** — 移至 PIN 码框下方、创建新账户按钮上方，不再遮挡输入框。
- **Android 指纹锁定期误报修复** — 失败次数过多触发系统锁定时不再显示「当前设备未设置或不支持生物识别」，改走锁定提示。
- **恢复流程时序调整** — 扫描完成后即检查账户 ID 冲突并提示，用户确认「覆盖恢复」后再输入密码；取消返回扫码卡片页。
- **同步结果统计摘要 i18n** — 「结果：examined=…」不再直显后端英文串。
- **Android SAF 目录 5 项问题** — 失效 Toast 去重、文案指向「设置 > 数据管理」、迁移成功后横幅正确消失、重选目录后不再误入创建账户页、内外目录切换不再要求重复创建账户。
- **CSS 变量批量修复** — 强制更新全屏卡片透明悬浮 + 7 个历史遗留未定义别名（`--bg-elevated-hover` 等）。
- **CLI 编译修复** — `SyncPeerInfo` 补全 v24 迁移新增字段。

### Changed

- **设备同步页 6 项 UI 升级** — 已知设备可点击展开详情卡片（设备名/信任徽章/指纹/最后信任与同步时间/客户端类型图标/host:port）；设备名过长裁剪；「你的设备名」标注；冲突计数显示具体值；冲突内容字段级 diff 可读化（非 JSON）；「保留本地/保留远程」按钮样式统一。
- **已知设备元数据扩展（v24 迁移 + 协议扩展）** — 同步对端携带 `client_type`/`trusted_at`/`last_seen_ts`，设备详情展示客户端类型与最近活动时间。

### Docs

- **AGENTS.md 新增「Rust 增量编译缓存损坏」陷阱与处置 SOP** — 链接报 undefined-symbol（内部单态化符号）时先定向 `cargo clean -p` 再排查代码。

### Chores

- 版本号同步升级到 2.8.1（versionCode 2008001）。
- 22 个 commit 自 v2.8.0（`8a8a831d`）到 v2.8.1。

## [2.8.0] - 2026-08-04

### Added

- **macOS Vision Framework 原生 OCR（P133）** — 将 `macos_vision.rs` 死模块接入为 macOS 端默认 OCR 引擎，设置页仅 macOS 端显示「Apple Vision」选项，其他端不显示；与 PP-OCRv6 双引擎可切换。
- **Embedding 注册表真实公钥注入（P207/N-10）** — 使用维护者持有的 minisign 公钥注入 `EMBED_REGISTRY_PUBKEY_B64`，注册表 JSON 与模型下载 sha256 同通道下发不再可信，硬校验签名。
- **config 原子写（P135）** — `safe_storage::write_atomic`（.tmp + rename）反向接入，`change_password`/`unlock_with_kdf_upgrade`/`create_account` 三条关键路径全部切换，消除「写一半」损坏风险。
- **P209 迁移窗口诊断日志** — 启动时扫描 legacy XOR 生物识别凭证存量并输出计数（trace 级），为迁移窗口关闭提供量化依据。

### Security

- **同步握手身份绑定（P001）** — 对端指纹以 Noise 握手认证值为准，双角色握手后强制比对，不再被加密通道内自报消息覆盖。
- **Windows 生物识别 DPAPI（P002）** — Windows 生产路径改用真 DPAPI（CryptProtectData）保护主密钥凭证文件，魔数检测 + 原子迁移旧凭证。
- **KDF 生产参数默认启用（P003）** — release 构建默认 64MiB/3iter（OWASP），仅 debug 用开发档；旧账户解锁成功后透明升级参数并重加密 verify token。
- **显式 command allowlist（P101）** — 188 条显式命令白名单替代 `allow-all-custom-commands`，前端全部 invoke 比对零缺失，IPC 攻击面收敛。
- **LLM base_url 网络出口校验（P102）** — 仅允许已登记 provider 目标；`llm_save_provider` 对未登记新 URL 强制系统级原生确认对话框（XSS 无法程序化点击），embedding 通道发送前同样校验（N-4）。
- **OCR 模型下载加固（P104）** — URL 校验 + 重定向白名单 + 流式双限 + 原子 rename；内置 sha256 清单扩至三档 12 文件（tiny/medium 取自官方 HF 仓库并交叉验证，N-5）。
- **分块头部纳入 GCM 认证（P105/P106）** — SOLC v2 头部作为 AAD 参与每个 chunk 认证，篡改 `chunk_count` 即解密失败；v3 分块头部一致性校验杜绝头部驱动巨额分配 DoS。
- **fs 面收敛（P107/P108）** — 基目录收窄至 Desktop/Documents/Downloads + Vault 附件目录；copy-file/stat 作用域仅保留 `$APPCACHE`/`$TEMP`。
- **删除 crypto oracle 命令组（P205）** — 移除 encrypt/decrypt/derive_key 三个命令，消除密码/密钥经 IPC 进 JS 堆的命令面；生物识别会话密钥改 Zeroizing 持有（P204）。
- **CSP 收紧（P206）** — `frame-src` 移除死授权 `data:`，新增 `object-src data:` 恢复桌面端 PDF 附件预览。
- **Embedding 注册表签名校验（P207）** — minisign 公钥硬校验 `registry.json.minisig`，7 条防回归单测。
- **插件 WASI stdio 黑洞（P208）** — 移除 `inherit_stdio()`，杜绝插件向宿主日志注入伪造内容。
- **同步永久停滞修复（N-1）** — keyset 分页替代 OFFSET：回退行 SQL 精确过滤（(wall,counter,node) 三元组 + id 决胜全序）、等值组尾部允许通过、会话层节点编码对齐，删除/批量创建 >limit 行同 ms 场景不再停滞。
- **reencrypt 事务化（N-2）** — `reencrypt_all` 全有或全无；改密/KDF 升级 config 前置备份 + 写失败自动回滚（config.json.pending 两阶段交换，R-4①/N-12）。
- **llmStore 明文清理（N-3）** — `streamBuffer` 纳入 vault-locked 清理链，流式进行中锁定不再残留 LLM 输出明文。
- **同步墓碑传播（N-13）** — objects/trash 硬删补写 sync_tombstones，变更清单合并墓碑，应用端识别 `deleted && data.is_null()` 删除本地行——永久删除可正确同步到对端。
- **墓碑生命周期清理（N-14）** — `delete_peer` 联动删 sync_watermarks；`cleanup_expired_tombstones()` 水位老化（存续 peer 水位 MIN ≥ 墓碑 HLC）+ 纯单机 365 天时间兜底，同步会话完成后触发。
- **legacy XOR 凭证保留（P209）** — 用户决策保留迁移路径（迁移窗口未关闭），配启动扫描诊断日志。

### Performance

- **同步分页下推 SQL（P109/P110）** — watermark 下推 + 有效 HLC 排序 + LIMIT/OFFSET（后修正为 keyset）；HLC 一次批量 JOIN 消除逐对象 SELECT。
- **list_objects metadata-only（P111）** — 新增 `list_object_metadata` 免全表解密，主列表/page_delete/attachment_list_all/llm_context 公共路径受益。
- **attachment_list_all 解密收敛（P112）** — 复用已解密 summary.properties + parent_id 预分组，4 轮全量解密降为单轮。
- **重路径 spawn_blocking（P113/P114）** — OCR 推理（image + MRZ，N-6）与全部 vault async command 统一移入阻塞线程池，不再阻塞 tokio runtime。
- **apply_sync_records 批量化（P115）** — 单事务 + 零克隆借用视图，消除每条记录 4 条 auto-commit SQL。
- **前端渲染优化（P116-P119）** — ChatMessageList 消息项 memo（流式仅最后一条重渲染）；useLlmChatCore 字段级选择器去整店订阅；WorkspaceObjectCard 修复 memo 击穿；回收站分页 + filtered useMemo + memo 卡片。
- **关键词过滤递归值树（P210）** — 消除整值 `to_string()` 往返。
- **page_delete 批量加载（P211）** — 单事务 + metadata 预筛，消除 N 次解密与 N×2 auto-commit。
- **import_vault 借用迭代（P212）** — metadata 预查 + 单事务批量写入。
- **SQL 常量化 + prepare_cached（P213）** — 热点语句缓存编译，消除 `format!` 分配。
- **llm_context public 预筛（P214）** — Section 3 改 metadata 预筛 + 仅 public 子集批量解密。
- **整店订阅分字段化（P215-P218）** — sync/plugin/ocr/ui store 字段级选择器 + 插件日志环形截断；onScroll 折叠瞬间持久化；附件三级列表 memo；审计日志卡片 memo + useMemo + 加载更多。
- **trash_items keyset 分页（R-1/R-2）** — 回收站变更清单 SQL 级 keyset 分页，消除 P110 同构停滞；修复秒/毫秒错配（不再放大 1000×）。

### Refactor

- **统一 HLC 三阶段（方案 B）** — objects（阶段 1）、trash/profile/user_template（阶段 2）本地写统一落 HLC，v23 迁移存量回填 + 回退兜底保留（阶段 3），同步变更清单大幅简化（R-3 窗口关闭）。
- **页游标并入 peer watermark（R-3）** — 会话中断后等值 HLC 组可续传。
- **storage.rs 八域拆分（P223-②）** — objects/trash/snapshots/sync_meta/sync_changes/sync_apply/metadata/profile 抽子模块，7922→4433 行。
- **host.rs 六簇分簇（P223-①）** — `register_host_functions` 923→7 行调度器 + 6 簇，`check_rate` 助手收敛 7 处重复检查。
- **lib.rs Builder 收尾（P223-③）** — Builder 链按插件组分簇（649→982 行）。
- **前端巨型组件拆分（P224）** — TrashDetailPanel（1282→313）、SyncPage（848→276）、TemplateManagerPage（810→328）、AboutPage（738→195）、OcrPage（738→385）均抽纯展示子组件/hook/面板。
- **统一 IPC 调用层（P131）** — 61 文件全量迁移裸调 invoke 到 `invokeCommand`，命令失败统一日志。
- **死命令/死代码清理（P132/P136/P219-P222）** — 删除 8 个死 Tauri 命令、LlmService 死方法簇（保留 CLI 依赖）、6 处前端死导出、13 项 Rust 死函数、25 处可见性收敛。
- **重复代码收敛（P137-P142/P225-P226）** — LLM 数据结构统一复用 core 定义；sync.rs 11 对 cfg 重复合并；同步管理器三处收敛 shared.rs；OCR 模型管理抽 hook；搜索逻辑抽 searchShared；导航卡片按钮收敛 hook；行解密闭包/unlock 前缀/PIN 凭证/附件源解析四大簇。
- **循环依赖断链（P228）** — accountId 注入 + 共享类型抽离 types/。

### Fixed

- **错误吞没批量修复（P006/P007/P120-P127/P227/P231）** — 删除对象失败、对话保存失败、导出范围加载、回收站详情、批量附件操作、PIN 验证、unlock 异常、诊断包导出、模板加载、自定义页面加载、低危静默降级等全部补 toast/日志/错误态；P120/P122 新增失败占位 + 重试 UI（N-11）。
- **fs 目录外附件降级（P107 附带）** — 目录外选附件 sizeBytes=0 降级、遗留外部 vaultPath 预览优雅报错。
- **P004/P005/P230 锁定后明文残留** — trashStore/searchCache/ocrScanStore 全部接入 vault-locked 清理链。
- **AboutPage window.open 兜底（P231）** — 移除无效兜底，shell 打开失败改应用内 toast。
- **ipc.test.ts 陈旧 mock（N-9）** — 移除针对已删 crypto 命令的测试。
- **P129/N-8 写入点收敛** — App/index.tsx 与 notification.ts 直写③收敛到导出的 `syncPlaintextPref`。
- **P128 主题缓存第 5 写入点消除** — 交回 settingsStore 唯一写入。

### Chores

- 版本号同步升级到 2.8.0。
- 签名密钥迁入 `~/SoloSoul/signing/` 并更新全部路径引用（`9837a81f`）。
- 172 个 commit 自 v2.7.1（`9807324d`）到 v2.8.0。

## [2.7.1] - 2026-08-01

### Fixed

- **跨设备恢复同名账户误判** — `create_account_with_id` 移除大小写不敏感的账户名唯一性检查（恢复场景允许同名，身份以 `account_id` 为准）；相同 `account_id` 不再直接报错，恢复对话框提供「覆盖恢复」（删除本端账户并用旧设备数据替换，可重设本端密码）与「取消」选择，配套二次确认弹窗与错误 i18n。
- **恢复包模板与历史快照丢失** — 恢复包（include_all）携带全部模板与对象历史快照（base64 + 原时间戳），解决预置模板丢失与历史记录变 1；`save_snapshot_at` 按原时间戳恢复，旧包向后兼容；Overwrite 策略快照去重防叠加；恢复保留原模板 ID；导出体积按实际字节估算。
- **Android SAF 目录删除后启动闪退** — `migrate_vault_data` 新增 `clear_dst` 参数，SAF 降级迁移（src 位于 dst 内部）不再清空目标目录，避免删除源数据与应用级目录导致 PluginManager 初始化失败闪退；迁移成功后清理临时缓存目录。
- **Android PDF 附件黑色块** — `PdfPreviewActivity` 首屏渲染延迟到布局完成，避免 1×1 位图拉伸成黑色块。
- **Android 导出/导入页溢出** — `ObjectSelectionTree` 长页面名/文件名 flex 截断防溢出卡片。
- **onboarding 账户来源卡片交互** — 点击「是的，从其他设备恢复账户」后直接跳转扫码页；创建新账户页补回「已在其他设备上有账户」返回入口。
- **设备同步错误 i18n 与发现修复** — 桌面 addresses 统一为 `ip:port` 形状；裸 IP 回退匹配；NSD 权限命令名对齐（Android 可正常发现设备）；守卫/握手/spawn_blocking join 错误全部纳入 `__SYNC_ERR__` 前缀机制。
- **同步二维码弹窗闪烁** — `SyncShowQrDialog`/`SyncScanQrDialog` 卡片进场淡入 + 固定高度加载占位，消除高度突变闪烁；Card 冗余 maxWidth 清理；`should_show_device` 去多余分配。
- **恢复流程英文错误 i18n** — `friendlyConnectError` 新增 PIN 不匹配/MITM/握手中断/传输失败/包超限/任务失败 6 类诊断文案；主机端错误走 `translateRustError` + `resolveBackendErrorMessage` 双级兜底。

### Refactor

- **`has_account` 公共方法** — 恢复覆盖检查改用 `has_account(id)`，减少文件 IO。

### Chore

- **兜底插件目录复用** — `solosoul_plugin_fallback` 固定目录名复用，避免 pid 后缀目录残留堆积。
- **构建产物解跟踪** — `gen/schemas` 取消 git 跟踪；Cargo.lock 同步。
- 版本号同步升级到 2.7.1。
- 17 个 commit 自 v2.7.0（`46c4ea08`）到 v2.7.1。

## [2.7.0] - 2026-08-01

### Added

- **首页快捷入口重排** — 快捷入口卡片调整为：设置 → 回收站 → 搜索 → 模板管理 → 附件管理 → 插件 → OCR → 导入导出 → 设备同步 → 帮助 → AI 对话，并新增「模板管理」「附件管理」两张入口卡片；帮助文案同步更新。

### Security

- **恢复凭证不再经 mDNS 明文广播（P001）** — 恢复会话的 PIN/nonce 不再写入 mDNS TXT 广播，仅经二维码/手动输入带外传递，局域网攻击者无法再凭广播信息直接劫持恢复会话。
- **导入包附件路径遍历修复（P002/P003）** — 导入附件时校验对象/附件 ID 字符集并净化文件名校验，杜绝 Windows 下经构造 payload 任意目录写；附件元数据写回净化后的 `safe_name`，插件工作区拷贝前增加末段净化兜底。
- **插件输出路径校验信任锚闭环（P004）** — 插件返回的路径在 host 侧以真实输出目录盖章后才允许打开/拷贝，前端不再直接打开未经验证的任意本地路径。
- **PIN 离线爆破加固（P005）** — PIN 凭证派生强制生产级 KDF 参数，不随开发模式降级。
- **capabilities 授权面收敛（P030）** — 收缩 fs/shell 授权到数据目录与用户常用目录，自定义命令按模块拆分权限。
- **GUI 登录/建库密码 Zeroizing 对齐（P031）** — 登录与建库密码以 `Zeroizing<String>` 经 IPC 传递，失败后立即安全擦除。
- **shell open 正则收紧（P032）** — 移除 `file://` 与绝对路径的宽松匹配，本地文件预览改走路径白名单。

### Performance

- **搜索计数免全量解密（P006）** — 分页计数改为 `COUNT(*)` 查询，不再逐行 AES 解密取长度。
- **模板命中复用已解密记录（P007）** — 高级搜索消除第二次全表解密扫描。
- **指南分块提升出循环（P008）** — RAG top_k 循环不再重复读取指南文件。
- **OCR 引擎按档位缓存（P009）** — 每次 OCR 命令不再重新加载 ONNX 引擎。
- **工作区整店订阅分字段化（P010/P055）** — 首页/工作区/编辑器等页面避免 store 任意变化触发整页重渲染。
- **附件管理器分页（P011）** — 顶层页面列表支持「加载更多」。
- **embedding 重建批量单事务（P051）** — 消除逐条独立事务+fsync。
- **回收站批量删除/附件批量下载并发化（P052/P054）** — 消除串行 IPC。
- **loadCustomPages 消除 N+1 IPC（P053）**。

### Refactor

- **插件运行时双份实现收敛（P012）** — 删除 `src-tauri` 侧平行实现，统一使用 `solosoul-plugin` crate 薄封装；wasmtime 依赖收敛，watermark 注册闭包去重（P047）。
- **mDNS 双 daemon 收敛（P013）** — 进程内仅保留单一 ServiceDaemon。
- **认证状态双 store 收敛（P014/P015）** — logout 状态重置移至 `finally`，解锁写入路径收敛为 `authStore.completeUnlock`。
- **大函数/大组件拆分（P023-P027、P038-P041、P043/P044）** — import 执行、迁移脚本、setup 闭包、工作区/恢复/引导/附件/登录/关于等 750+ 行大组件全部按职责拆分。
- **profile preferences 写入块收敛（P028）** — 7 处重复块抽为共享函数。
- **手写 hover 迁移（P048）** — 全项目 109 处手写 `onMouseEnter/Leave` 统一迁移到共享 `interactive-*` 工具类，视觉等价。
- **FilterChipGroup 收敛（P049）** — 5 处筛选 chip 块抽取共享组件。
- **校验 switch 表驱动化（P050）** — 对象编辑器动态组校验改查表循环。
- **对象 store trash 死切片删除（P056）** — 与 trashStore 双轨操作同一后端数据的冗余实现移除。
- **updateObject 同步列表（P057）** — 成功后同步更新对象摘要列表，消除潜伏性不一致。
- **死代码清理（P017-P022、P033-P037、P058）** — 删除 liquid-glass 死样式、死 barrel、死导出符号、18 个死 Tauri Commands、未用 npm 依赖等，缩小 IPC 攻击面。

### Fixed

- **三处裸调 plugin-dialog（P016）** — 改用 `openWithPause` 封装，避免文件选择器触发自动锁定误锁。
- **invoke 链缺 `.catch`（P059）** — 历史页/快照查看/导航栏三处补齐错误处理。
- **插件下载静默吞错（P060）** — 下载下沉 Rust 命令并增加前端错误提示。
- **恢复 PIN 非常数时间比较（P029）** — 改为常数时间字节比较。
- **CLI 测试恢复（P064）** — 修复夹具字段缺失与 fmt 漂移，恢复 CLI cargo test。

### Chore

- **acl-manifests.json 同步** — 补登 `plugin_copy_output_file` 命令，ACL 一致性检查 197 命令全部登记。
- **依赖同步** — Cargo.lock 同步（zeroize 依赖进 solo_soul 包）；wasmtime async feature 确认不可移除（wasmtime 45 default features 与 wasmtime-wasi p1 均传递依赖）。

## [2.6.8] - 2026-07-31

### Added

- **恢复流程三输入框** — 「从其他设备恢复」账户卡片新增三个输入框：新主密码、二次输入新主密码、主密码提示词，与创建账户流程体验对齐。
- **桌面端关于页面更新详情补全** — 桌面端关于页面补全 GitHub Release 完整更新详情（与 Android 端对齐），新增 `desktop_check_update` 命令从 GitHub Releases API 拉取最新版本 notes。
- **二维码配对整合** — 设备同步「显示二维码」卡片内新增同步/恢复二维码模式切换，删除独立的「显示恢复二维码」按钮；`SyncScanQrDialog` 同时支持两种二维码格式识别。

### Fixed

- **设备同步页按钮卡死** — 多次快速点击启用/禁用时偶发禁用失败且页面所有按钮失效的问题修复。
- **更新横幅被 AppBar 遮挡** — 登录解锁后更新提示横幅 z-index 提升至 AppBar 之上。
- **移动端扫码默认后置摄像头** — 扫码启动时优先选择后置摄像头，避免默认打开前置。
- **创建账户页空字段校验** — 空表单直接提交时按优先级（账户名 > 主密码 > 确认密码）抖动输入框 + 红边框 + 红色提示文字，不再静默返回登录页。
- **恢复连接超时友好诊断** — 恢复连接超时/被拒/网络不可达时给出诊断引导（同一网络检查 + macOS 防火墙放行路径），新增 `recovery_connect_timeout` / `recovery_connect_refused` / `recovery_connect_unreachable` 文案。
- **恢复密码框空字段校验** — 恢复流程密码输入框加入空字段校验（主密码 > 确认密码优先级，提示词可选），与创建账户页一致的抖动/红边框交互。
- **扫码双框视觉修复** — 移除 html5-qrcode `qrbox` 配置，消除「灰遮罩大框 + 透明小框」双框视觉，扫描区域覆盖整个视频画面。
- **数据管理页冗余项清理** — 删除移动端「当前存储类型卡片」上方冗余的保险库目录标题行。
- **安卓顶栏按钮尺寸统一** — 对象页添加对象、模板页模板示例/新建模板按钮统一为与指南按钮一致的 32×32 紧凑尺寸。
- **显示二维码卡片滚动保护** — `SyncShowQrDialog` 恢复二维码展开手动模式后内容超出视口时卡片内可滚动，修复 tab 切换/关闭/取消按钮不可达问题。

### Chores

- 版本号同步升级到 2.6.8。
- 13 个功能 commit 自 v2.6.7（`38be0c52`）到 v2.6.8（`c604e07b`）。

## [2.6.7] - 2026-07-31

### Added

- **恢复流程统一为「扫码优先 + 账户卡片」** — 重构「从其他设备恢复」流程：新设备默认使用摄像头扫描旧设备二维码，旧设备在登录解锁后于「设置 → 设备同步」页展示二维码；主密码输入时机统一为扫码/连接成功后弹出的账户卡片（显示账户名与账户 ID），手动模式与扫码走同一条恢复链路（`recovery_restore_from_host`），结果完全一致。
- **摄像头能力自适应默认 Tab** — 应用启动时通过 `enumerateDevices()` 非侵入检测摄像头能力（不触发权限弹窗）：支持 → 默认「扫描二维码」Tab；不支持 → 默认「手动输入」Tab；无摄像头设备手动切到扫码 Tab 时显示「本设备不支持扫描二维码功能，请使用手动输入模式」提示。新增 `lib/cameraCapability.ts` + `hooks/useCameraCapability.ts`。
- **SyncScanQrDialog 摄像头兜底** — 设备同步扫码对话框接入摄像头能力检测：无摄像头时显示提示并引导回页面使用设备发现/手动输入；扫码启动失败（权限被拒）时显示同样的手动兜底入口。
- **macOS 摄像头权限声明** — Info.plist 新增 `NSCameraUsageDescription`，使 macOS 恢复扫码可正常弹出系统授权框并登记至 系统设置 → 隐私与安全性 → 摄像头。

### Fixed

- **扫码崩溃修复（页面消失）** — `html5-qrcode` 的 `stop()` 在扫描器未启动时**同步 throw**（`"Cannot stop, scanner is not running or paused."`），`.catch()` 无法捕获同步异常，导致异常逃逸 React 卸载流程、整页崩溃。`RecoveryQrScanner` cleanup 与 `start()` 均以 try/catch 包裹，并以回调 ref 消除父组件重渲染导致的扫描器反复 stop/restart。
- **Android 同步页按钮锁死** — 为设备同步页操作增加超时保护，防止启用/禁用等操作异常时页面所有按钮不可用。
- **恢复扫码错误文案细分** — `recovery_qr_no_camera` 拆分为「无摄像头设备」与「权限被拒」两种场景文案。
- **ACL 权限补全** — 新增 `recovery_discover_hosts` 命令白名单。

### Chores

- 版本号同步升级到 2.6.7。
- 9 个 commit 自 v2.6.6（`3a8be1f0`）到 v2.6.7（`38be0c52`）。

## [2.6.6] - 2026-07-31

### Added

- **账户恢复 QR 配对（反向恢复）** — 新增反向恢复流程，新设备扫码旧设备的恢复二维码完成传输，支持密码 + 二维码两种配对方式。
- **恢复发送端账户名传递** — 恢复时从发送端传递账户名，接收端自动填入，减少手动输入。
- **非 QR 手动连接恢复** — 恢复接收对话框新增手动输入标签页，支持输入主机地址和端口手动连接，适用于无摄像头设备的恢复场景。
- **mDNS 局域网恢复发现** — 新设备在恢复页面自动发现局域网内的主机设备，无需手动输入 IP 地址。
- **iOS 生物识别 Keychain 迁移** — iOS 生物识别凭证存储从 `FileBiometricStorage` 迁移到系统 Keychain（`kSecAccessControlUserPresence`），安全性从文件权限 0o600 提升至硬件 Keychain。
- **Android platform_storage 清理** — Android 的 `platform_storage()` 改为 `StubBiometricStorage`（实际凭证操作由 `KeystorePluginHandle` 处理）。
- **设备同步 QR 配对** — 设备同步页面新增 QR 码配对功能，支持扫码类型自动检测。
- **登录页恢复入口增强** — 新增「返回登录」和「从其他设备恢复」入口，新设备用户引导询问是否已有现有账户。

### Fixed

- **P0/P1 代码分析安全/稳定性修复（P001-P008）**：
  - P001：macOS Vision OCR 外部二进制路径加固（SHA-256 哈希校验 + 确定性缓存目录）
  - P002：Windows `icacls` 参数注入修复（链式 `.arg()` 替代 `format!` 拼接）
  - P003/P004：同步/设备同步事件 `.unwrap()` → match 优雅处理
  - P005：`storage.rs` 16+ 处静默吞没错误 → 向上传播
  - P006：SQL IN 子句 `format!` 拼接 → `repeat_n("?")` + 参数绑定
  - P007：AI 助手插件上下文注入（`build_section5_plugins()` 实际实现）
  - P008：iOS Keychain 迁移 + Android platform_storage 清理
- **Android 构建修复** — 补全缺失的 `Manager` trait 导入和死 `BiometricManager` 实例化移除。
- **Mobile mDNS 状态注册** — 为 Android/iOS 注册 `SharedDaemon` 状态，修复 mDNS 命令在移动端的运行。
- **恢复对话框错误处理** — 抑制预期的取消错误，补全缺失的命令权限和加载状态修复。
- **Invoke payload key 统一** — 统一为 camelCase 以匹配 Tauri 默认命令命名。
- **Android 更新横幅状态管理** — 在安装权限缺失时保持下载状态。

### Code Quality

- **P009: Clippy unwrap/expect 生产代码清零** — 消除 `aes.rs`、`hlc.rs`、`manager.rs`、`attachments.rs`、`field.rs`、`object/mod.rs`、`build.rs` 中所有 `unwrap_used`/`expect_used` 警告。
- **P011: React useMemo 审计** — `SettingsPage.tsx`（`settingGroups` → `useMemo`）、`DataManagementPage.tsx`（`breakdownItems`/`pieSlices` → `useMemo`，`PieChartSvg` → `memo()`）。
- **P012: unsafe FFI 测试补充** — `window.rs` 提取 `calculate_luminance` 纯函数（6 个单元测试）；`biometric/mod.rs` 新增 6 个 FFI 边界测试（CString、平台错误传播、Send+Sync 等）。
- **P013: 死代码扫描（cargo-machete + knip）** — 移除 10 个 Rust 未使用依赖（6 个 Cargo.toml）、清理 13 个前端死代码导出（10 个文件）。

### Chores

- 版本号同步升级到 2.6.6。
- 更新 CODE_ANALYSIS_REPORT.md 为全部 13 项完成状态。
- 重新生成 ACL manifests。

## [2.6.5] - 2026-07-30

### Fixed

- **Android 同步 NSD 编译修复** — 补全 `sync.rs` 中缺失的 `Manager` trait 导入，并将 `AppState` 克隆后再移入 `tokio::spawn` 后台任务，修复 Android release 构建失败。

### Chores

- 版本号同步升级到 2.6.5。

## [2.6.4] - 2026-07-29

### Added

- **Android 全局更新横幅** — 新增应用内全局更新横幅，支持 Markdown 渲染 release notes，提升更新提示体验。

### Fixed

- **改密后 SAF 远程同步** — 修改主密码后，自动将重新加密的 `vault.db` 同步到远程 SAF 存储，避免下次解锁时旧远程副本覆盖本地数据导致解密失败。
- **审计日志解密容错** — 审计日志列表对单字段解密失败进行容错，显示占位文本而不是整体查询失败。
- **对象删除错误提示** — `object_delete` 增加明确的对象不存在和解密异常处理分支。

### Chores

- 版本号同步升级到 2.6.4。
- 移除 `sync.rs` 中未使用的 `Manager` import 并重新生成 ACL manifests。

## [2.6.2] - 2026-07-29

### Added

- **Android 应用内自更新机制** — 实现 Android 应用内自更新（GitHub API + APK 下载 + 系统安装器），支持后台下载 APK 并通过系统安装器静默更新。
- **APK 断点续传** — 下载 APK 时支持 Range 请求头 + append 模式，网络中断后自动从中断处继续下载。
- **Android 强制更新策略** — 支持 [MANDATORY] 标记，关键安全更新可强制用户立即升级后方可使用。
- **Android 更新权限引导** — 引导用户开启「安装未知应用」权限，确保更新可正常安装。
- **APK 完整性校验** — 下载完成后自动计算 SHA-256 哈希并与预期值比对，防止文件损坏或篡改。

### Changed

- **发布流程更新** — 流程新增 APK checksum 生成步骤，以及 [MANDATORY] 强制更新标记说明。

### Fixed

- **Android 更新功能 i18n 补全** — 补全 `need_install_unknown_apps`、`mandatory_update_title`、`mandatory_update_desc` 三个翻译 key。

### Chores

- 版本号同步升级到 2.6.2。
- 5 个 commit（`27babb67` → `2cf3f8af`）到 v2.6.2（`36a9ee93`）。

## [2.6.3] - 2026-07-29

### Added

- **同步冲突事件通知** — 新增 `sync-conflicts-updated` 事件通知前端新冲突，支持徽章通知。
- **协议版本协商** — 新增 Hello/HelloAck 协议版本协商字段，实现向后兼容的客户端互操作。

### Fixed

- **mDNS 账户隐私保护** — 哈希 account_id 在 mDNS TXT 记录中，防止局域网上明文账户标识泄露。
- **HLC 时钟回跳检测** — 检测显著 HLC 时钟回跳并发出警告，防止 watermark 异常。
- **离线 LAN 回退** — local_ip_address 在 UDP 探测 8.8.8.8 失败时自动降级为本地回环地址。
- **mDNS 对端内存泄漏** — 定期清理过期 mDNS 发现的对端记录，防止 HashMap 无限增长。
- **恢复重试修复** — 将 mark_served() 移到文件传输完成后执行，中断后可重试恢复。
- **slowloris 攻击防护** — 添加 5 分钟会话级总超时，防止慢速攻击。
- **Vault 写入保护** — 等待活跃同步会话完成后才停止同步服务，防止 Vault 写入中断。
- **移动端 OOM 修复** — 附件分块流式写入临时文件，替代全内存缓冲，防止移动端内存溢出。
- **跨账户同步防护** — 验证 initiator session 中的 responder account_id，防止跨账户数据同步。
- **Android 构建修复** — 补全 sync.rs 中缺失的 `Manager` trait 导入，修复 Android 交叉编译错误。

### Chores

- 版本号同步升级到 2.6.3。
- 12 个 commit 自 v2.6.2（`36a9ee93`）到 v2.6.3（`44a80b50`）。

## [2.6.1] - 2026-07-28

### Fixed

- **设备同步页面严重卡顿** — 修复 SyncPage.tsx 中 useCallback 依赖整个 zustand store 导致的无限重渲染循环。改用 `useSyncStore.getState()` + 空依赖数组 + `Promise.all` 并行化 4 个 IPC 调用。
- **ACL 权限缺失补全** — 补全同步/恢复相关命令（`sync_discover`、`sync_listen_port` 等）到 `allow-all-custom-commands` 白名单。

### Chores

- 版本号同步升级到 2.6.1。
- 名称统一：代码/文档中的 `master` → `main`，`SoloSoul_code` → `SoloSoul`。
- 同步插件市场子模块指针至最新版本。

## [2.6.0] - 2026-07-28

### Added

- **账户恢复与同步加固** — 完成 P2 账户恢复加固与 P3 设备自动同步触发；同步+恢复相关保险库存储层统一重构。
- **回收站过期自动清理** — 启动时自动清理过期回收站项目，改为 tokio 后台任务执行，并限制并发防止重复任务。
- **回收站级联恢复** — 支持级联恢复页面及其子对象，自动重建页面桩，修复恢复后列表不刷新与 `original_parent_id` 字段填充问题。
- **Android 客户端 MVP** — 基于 Tauri Mobile 将 SoloSoul 桌面端扩展至 Android，完成核心页面响应式布局、底部导航、安全区适配、content:// URI 文件中转与系统文件对话框适配。
- **Android 锁屏安全** — 锁屏状态 Rust 插件集成；切后台检测与自动锁定兜底；锁屏遮罩残留修复；锁屏状态通过原生事件推送前端。
- **SAF 外部目录支持** — Android 引导流程选择外部 SAF 目录并持久化；`AutoSyncManager` 自动同步、写入防抖、后台 WorkManager 兜底、同步进度可视化；SAF 授权失效检测与自动降级到本地。
- **设备同步闭环** — 同步发现 NSD 广播、设备列表、监听端口暴露；移动端设备同步实现。
- **OCR/MRZ 导入体验增强** — OCR/MRZ 导入为对象前弹出名称输入框；默认名称精确到时分秒；扫描导入字段改为多行文本并使用 i18n 字段名；Android 增加拍照入口、运行时权限申请与 content URI 导入修复。
- **移动端生物识别** — Android Keystore 凭证存储；独立 Touch ID / Face ID（Class 2 弱人脸）开关及安全提示；生物识别诊断入口与权限申请优化。
- **本地通知与提醒** — 自动锁定提醒与备份提醒，支持跨重启去重，持久化 `lastBackupReminderAt`。
- **操作日志国际化** — 导入详情、级联恢复、OCR 扫描、回收站内容预览等操作日志详情支持中英文。
- **数据管理页面** — 将保险库目录项移入数据管理页面，显示路径并支持「恢复为本地目录」二次确认。
- **插件与模板增强** — 字段类型图标显示、插件结果区/徽章/UI 统一、模板图标统一、审计日志时间戳秒级显示。
- **首页与引导** — 欢迎卡片显示当前账户名；新增内置「文档」集合页；`PageGuide` 新手引导动画与滑动导航；首页指南按钮与自定义页面说明长度限制。
- **构建与发布** — Android AAB 构建、`versionCode` 叠加、签名配置、CI release 构建上传、ACL 一致性检查脚本。

### Changed

- 对象详情/历史记录卡片字段值支持完整行宽度、长文本自动换行与智能对齐；回收站快照字段值换行显示。
- `reqwest` 改为 `rustls-tls`，避免 Android 交叉编译依赖 OpenSSL。
- 系统文件对话框统一暂停自动锁定（`openWithPause`/`saveWithPause`），防止文件选择期间 Vault 被误锁定。
- 导出前显式展示将被导出的模板清单。
- 登录页默认选中上次登录的账户；登录后检测之前启用过的快捷解锁方式并引导重新设置。
- 插件/日志 UI 与移动端对齐：标签页、徽章、按钮尺寸、图标对齐。
- 移动端设置页隐藏插件、OCR、设备同步入口；关于页跳过桌面端自动更新检查。

### Fixed

- 锁屏遮罩残留、切后台未锁定等 Android 生命周期问题。
- 引导流程外部目录选择在前/后导航时丢失。
- 新建页面卡片空名称高亮提示；`AddPageButton` 弹窗适配正常模式、小窗模式与安全区，避免被 AppBar 遮挡。
- `PostLoginSetupGuide` 重复提示设置的问题。
- SAF 外部目录重装后登录失败、同步子目录误用 `getTreeDocumentId`、后台锁屏时无法自动锁定、首次同步 OOM/竞态等。
- 后台锁屏后 Vault 未自动锁定，解锁回 App 闪现旧内容。
- Android 构建问题：Kotlin 类型不匹配、status-bar 参数、AppBar 点击、资源复制路径、help 索引读取等。
- 移动端附件下载、批量下载、下载重命名、content URI 导出导入路径校验。
- 操作日志 `page_restore` 标签国际化；实体筛选器补充 `file` 与 `ocr_model`。
- 模板误报「已更新」提示条问题。
- 插件审计日志时间戳到整秒、插件结果区 UI 对齐。
- 回收站批量操作栏内嵌化、级联恢复后列表刷新、`original_parent_id` 字段纠正。

### Performance

- Android APK/AAB 体积优化：ProGuard/R8 精简、native 库瘦身、Rust release profile 优化。
- 桌面端包体积优化。

### Security

- Android SAF 路径规范化，移除 `./` 防止 `ENAMETOOLONG`。
- 应用后台时锁屏自动锁定。
- ACL 白名单补充 6 个缺失命令。
- 生物识别 Keystore 安全加固：`CryptoObject` 绑定与线程安全。
- 通知权限系统对话框每次安装最多弹一次。

### Chores

- 版本号同步升级到 2.6.0。
## [2.5.12] - 2026-07-12

### Added

- **模板动态字段组（dynamic_group）支持** — 模板编辑器新增动态字段组类型，允许在对象中按需添加/删除子字段；支持区域高度限制与滚动、子字段类型图标与敏感度徽章、类型选项国际化。
- **模板更新后对象手动同步** — 模板变更后对象详情页显示同步提示条，支持查看字段差异、一键同步或跳过；提示条状态持久化到 sessionStorage，仅在模板再次变更时重新显示。
- **数据管理页面交互优化** — 存储明细 breakdown 按钮增加悬停动画，提升可视化反馈。

### Fixed

- **模板同步指纹一致性** — 修复前后端模板指纹字段名、序列化 key、字段顺序不一致导致同步后提示条仍显示的问题；无字段差异时同步直接刷新 template_hash，避免重启后重复提示。
- **回收站内容预览与恢复** — 回收站内容预览改为显示字段值而非字段类型，支持动态字段组子字段树状展开、字段类型图标与敏感度徽章；快照敏感度优先使用快照自身定义；恢复对象时正确读取 camelCase 数据键，启动时自动修复旧版恢复对象的数据异常并补齐缺失的字段敏感度。
- **历史记录显示修复** — `__dynamic_group__` 在历史记录中正确国际化，动态字段组子项敏感度继承父字段快照定义，字段类型图标正确显示。
- **审计日志国际化** — 同步对象模板日志显示模板名并国际化，动态字段组关键字段查看日志显示具体子字段名并国际化。
- **UI 细节修复** — 模板管理器与回收站页面切换时 loading 闪烁修复；密码验证卡片置顶；动态字段组子字段类型选项国际化；模板管理与回收站详情卡片徽章顺序统一为 [所属页面][模板][插件绑定]。

### Changed

- **动态字段组作为模板级开关** — 重构动态字段组配置方式，统一前后端数据模型与 UI 渲染逻辑。
- **同步弹窗交互优化** — 点击"否"增加二次确认弹窗，关闭同步详情弹窗时不隐藏提示条。

### Code Quality

- **H001** — 将生产环境 `unwrap` 替换为 `expect`/if-let，消除潜在 panic。
- **H002** — 移除冗余 clone 并简化错误传播，减少不必要内存分配。
- **F001-F005** — 前端死代码清理：移除未使用的 helper、import、eslint-disable、i18n 解构与 useEffect 依赖缺失。
- **R001-R004** — 测试代码清理：同步新字段到测试、替换无用 `vec!`、移除未使用 import、清理 registry 测试。
- **C001** — 移除不必要的 `mut` 声明。

### Chores

- 版本号同步升级到 2.5.12。
- 72 个 commit 自 v2.5.11 到 v2.5.12。

## [2.5.11] - 2026-07-05

### Fixed

- **侧边栏悬停展开逻辑修复（水平模式）** — 水平模式（上/下）下功能区与 AddPageButton 之间的 4px flex gap 导致鼠标穿过间隙时功能区误折叠。改用 hoverZone 外层包裹 AddPageButton + 折叠功能区，handleMouseLeave 移到外层，彻底消除 gap 引起的事件检测失败。涉及 TopFunctionBar.tsx、SecondaryActionBar.tsx、AddPageButton.tsx。
- **AI 对话卡片弹出位置修复** — 垂直模式（左/右）下 AI 对话卡片弹出位置偏上的问题修复。useAiQuickChat/useOcrQuickScan/usePluginQuickPanel 三处定位逻辑从居中改为顶部对齐（rect.top）；SecondaryActionBar.tsx 中 AI chat 使用本地 state 导致 quickChatPos 始终为 null，手动触发 updateQuickChatPos + 附加 scroll/resize 监听器。
- **水平模式添加页面卡片右侧溢出修复** — 功能区折叠时 AddPageButton 靠近窗口右边缘，弹出卡片右半部分超出视口。添加 horizontalPopoverLeft 检测右侧溢出并自动左移，保证卡片完整显示。
- **Dialog 模态框居中修正** — 修复 Dialog 组件居中样式（Dialog.module.css）。
- **Dialog 标题主题色修正** — 修复 Dialog 标题颜色正确跟随主题。

### Changed

- **恢复自定义 DatePicker + DropdownSelect** — 用自定义日期选择器和下拉选择组件替换原生 input[type=date]，恢复统一的视觉风格和交互体验。

### Chores

- 版本号同步升级到 2.5.11。
- 5 个 commit 自 v2.5.10（f4fbb380）到 v2.5.11（c9fd00d0）。

## [2.5.10] - 2026-07-05

### Added

- **模板字段级契约角色绑定** — 新增字段级 `contractField` 标记与 `contractBindings` 声明，支持将字段绑定到插件的 typed contract role。后端新增 Schema v18 migration（`object_templates.contract_bindings`、`user_templates.contract_bindings`、`objects.contract_bindings`），前端 `TemplateFieldInput` 新增契约绑定 UI 面板。
- **插件自动注册合约绑定** — 插件安装时自动扫描 manifest 中 `contracts[].roles[].defaultPropertyId`，自动写入系统模板和用户模板的字段级 `contract_bindings`。删除插件时自动清除对应绑定。
- **导入 KeepBoth 策略** — 模板/对象冲突时支持 KeepBoth 策略，自动附加本地化后缀（如 `（已恢复）`/` (restored)）并重写 ID，保留双方数据不丢失。
- **导入模板内容哈希去重** — 导入时按模板 properties 的内容哈希自动去重，避免重复导入相同模板。
- **导入三级树预览** — 导入预览改为「页面 → 对象 → 附件」三级树形结构，后端增加附件级选择过滤。
- **导入继承模板字段敏感度** — 导入对象时模板字段的敏感度设置自动继承到导入结果。
- **导入创建初始 Snapshot** — 导入对象时自动创建初始历史快照，修复历史记录显示为 0 的问题。
- **导出全选/取消全选** — 导出区新增全选/取消全选按钮，搭配 `useExportScope` bulkSelect 钩子。
- **expiry-guardian 插件重写（方案 B）** — 使用 typed contract + 自定义 UI 完全重写 expiry-guardian 插件，持多语言过期标签渲染与到期提醒。
- **系统模板绑定 expiry-guardian** — Passport、Visa、ID Card、Bank Card 等系统模板增加 `expiryDate` 字段并绑定 expiry-guardian 插件。
- **expiry-guardian 自定义 UI 集成** — `PluginResultPanel` 新增 `expiry_guardian` case 渲染 `ExpiryGuardianView` 组件，支持多语言过期标签与摘要统计。
- **契约绑定元数据增强** — `PluginResultPayload` type 增加 `ExpiryGuardianPayload` variant，`deriveContractBindings` 运行时推导函数从 installed plugins manifest 自动匹配绑定。
- **Tauri 窗口状态持久化** — 用 `@tauri-apps/plugin-window-state` 替换自定义窗口大小 hook，跨重启自动恢复窗口位置和尺寸。

### Changed

- **内置 Dialog 替换为原生 `<dialog>`** — 移除自定义 Dialog CSS 实现，改用原生 HTML `<dialog>` 元素（-93 行）。
- **内置 ExpandableSection 替换为原生 `<details>/<summary>`** — 移除 `ExpandableSection` 自定义实现，改用原生折叠元素（-137 行）。
- **内置 DatePicker + DropdownSelect 替换为原生 `input[type=date]`** — 移除 509 行自定义日期选择器和下拉选择组件。
- **合并 `commands/rag.rs` 到 `commands/llm/rag.rs`** — 消除重复的 RAG 模块（-384 行）。
- **删除废弃的 `db/` 模块** — 移除已不再使用的旧式数据库模块（-221 行）。
- **统一 AES 错误类型** — 用 `CipherError` 枚举替换所有 `String` 类型 AES 错误，统一返回类型。
- **插件徽章 UI 统一** — 未安装合约时显示灰色徽章，已安装时使用主题色；`PluginBadge` 在所有组件中样式统一。
- **expiry-guardian UI 深色/浅色模式适配** — 移除 emoji，改用 Lucide 图标。
- **Windows 导出/导入密码校验移除** — 导出密码不再强制 8 位字母+数字组合，用户自行负责密码强度。
- **导出警告防重复触发** — `skipRef` 在导出成功后重置，防止重复弹窗。
- **PIN 图标 grip 调整** — 侧边栏 PIN 配置图标从 grip 改为更直观的图标。

### Fixed

- **P0.5 架构审计全部 85 项完成** — 移除 ~3,120 行代码，减少 9 个依赖，完成 CLI wrapper 内联、VaultService 参数合并、dead code 清理等全部 P0/P1/P2 项。
- **6 个 ESLint `no-explicit-any` 错误** — PluginResultPanel、OcrPage.test、OcrSettingsPage.test 中移除所有 `as any` 断言，改用 `Record<string, unknown>` 类型。
- **2 个 React Hooks 顺序回归** — Dialog.tsx 和 PluginBadge.tsx 中 early return 移到 Hooks 调用之后，符合 React Hooks 规则。
- **ESLint + React Hooks 回归修复** — 解决 P0-3c 重构中引入的 Dialog early return 在 `useEffect` 之前的违规。
- **插件契约角色反序列化** — `PluginContractRole.label` 和 `displayName` 支持 String 或 object 两种格式的兼容反序列化。
- **插件 typed-lookup 跨模板污染** — 通过 `contract_type_id` 隔离，防止不同模板的字段查找互相干扰。
- **导入对象 snapshot 缺失** — 导入时自动创建初始 snapshot，修复历史记录版本号显示问题。
- **模板 contract_type_id 导入导出** — contractTypeId 在导入导出中正确持久化。
- **AiFeatures 硬编码值折叠** — 将 `smartFill/commandGen/naturalLanguageSearch: false` 提取为常量。
- **template.rs 包装函数移除** — 移除 `load_trash_retention`/`retention_ms` 私有包装函数，直接委托到 `snapshot.rs`。

### Refactored

- **P0.5 Phase 1-4 全部完成**：
  - Phase 1：清理死代码、废弃模块和依赖（`rag.rs`合并、`db/`删除、`CustomDropdown` 替换）
  - Phase 2：CLI 和前端去重（泛型列表导航、`vault_write`/`attachment`/`export_import` 下沉到 `solosoul-core`）
  - Phase 3：ESM 兼容性、统一 `escape_json`/`truncate` SDK helper、`plugin-window-state` 集成
  - Phase 4：代码库最终清理（CLI `plugin_sink.rs` 删除、`DisposeModule`/`AppLifecycle`/`StreamSink` 残留清理）
- **`FieldResolver` 下沉到 `solosoul-plugin`** — 运行时推导旧插件的 `effective_roles`，提升插件兼容性。
- **P2 前端重构** — `useCancellable` → `AbortController` 迁移、死代码清理、内联优化。
- **CLI 包装器内联去重** — 通用列表导航 key handler、`settings_language_select`/`settings_theme_select` 委托、`Theme::with_level` 私有化。

### Chores

- 版本号同步升级到 2.5.10。
- 60 个 commit 自 v2.5.9（`3f57ca4f`）到 v2.5.10。
- expiry-guardian 子模块指针更新到最终修复版本。
- i18n 补全 `editor:templates.idCard` 中英文翻译。

## [2.5.9] - 2026-07-01

### Added

- **SafeMarkdown 安全组件** — 新增 `SafeMarkdown` 封装组件，统一配置 `disallowedElements={['script', 'style', 'iframe', 'object', 'embed']}`，替换 GuideRenderer/ChatMessageList/ChatMessageBubble 三处 ReactMarkdown 调用，加固 XSS 防护。

### Changed

- **PIN 码审计日志分类** — PIN 码查看关键信息的操作日志从 `critical_field_login`（密码登录）改为独立的 `critical_field_pin`（PIN 解锁），entityType 从 `biometric`（生物识别）改为 `auth`（认证），与主密码分类一致。
- **Windows Hello 审计日志一致性** — 补全 Windows Hello 在所有审计日志路径中的映射，确保 `windows_hello_unlock` → `windowsHello` 类型映射、`writeCriticalAccessLog` 分支覆盖。
- **关键数据访问对话框宽度** — `PasswordVerificationDialog` 的 `maxWidth` 从 480px 缩小至 360px，与登录页卡片宽度一致。Dialog 组件新增可选 `dialogStyle` prop。

### Performance

- **PIN 解密性能优化** — `pin_kdf_config()` 改用 `KdfConfig::from_env()`，开发模式下 PIN 解锁从 ~1472ms 降至 ~445ms（降幅 70%）。

### Fixed

- **PIN 解锁无法展开关键字段** — 修复 PIN 解锁成功后关键字段仍被遮挡的问题，新增全局 keydown 事件捕获确保口令对话框正常弹出。
- **Windows COM 初始化错误类型** — 修复 `ensure_mta()` 中 `?` 操作符无法将 `()` 转换为 `BiometricError` 的编译错误，改用 `map_err` 显式转换。
- **代码审计 P001–P005 修复**：
  - P001：`cargo fmt` 修复 9 个 Rust 文件格式
  - P002：ESLint 10 个 warning 清零（`useEffect` 依赖补全、未使用变量/导入清理、`console.log` → `console.warn`）
  - P003：Prettier 178 个文件格式化
  - P004：legacy XOR key 添加 SECURITY 注释，标记为已知保留
  - P005：创建 SafeMarkdown 组件替换三处 ReactMarkdown

### Security

- **SafeMarkdown XSS 加固** — `disallowedElements` 配置禁用 `script`/`style`/`iframe`/`object`/`embed` 标签，消除 ReactMarkdown 未来升级可能引入的风险。
- **Legacy XOR key 文档化** — `legacy.rs` 中 `LEGACY_XOR_KEY` 添加安全注释说明：仅用于旧版文件单向解密迁移，解密后自动迁移到 per-account HKDF 派生 AES-256-GCM 新格式。

### Chores

- 版本号同步升级到 2.5.9。
- 12 个 commit 自 v2.5.8（`642cad0a`）到 v2.5.9。
- 提交 CHANGELOG 摘要：`08401af7`、`b7c44657`、`e25241c3`、`3df4fa20`、`52655f29`、`b1311812`、`3018af4b`、`d779b5c3`、`fa96d87e`、`3f57ca4f` 等。

## [2.5.8] - 2026-07-01

### Added

- **Windows Hello 生物识别** — 新增 Windows Hello 生物识别凭证存储与用户验证支持。使用 `UserConsentVerifier` 触发 Windows Hello 弹窗验证用户身份，凭证存储复用本地加密文件方案，支持可用性检测、验证弹窗（PIN 作为系统回退）、诊断日志与审计事件记录。
- **PIN 快速解锁** — 已登录状态支持 PIN 码快速解锁 vault，减少频繁输入主密码的摩擦。
- **PIN 锁定重置** — 强因子认证（主密码/Face ID/Touch ID）成功后自动重置 PIN 锁定状态，避免用户因 PIN 锁定无法登录。
- **新手引导多页引导（PageGuide）** — 新增 PageGuide 组件，支持多步引导式帮助文档；增加拖拽上传教程页面。

### Changed

- **登录优先级逻辑** — 登录方式按 Face ID > Touch ID > Windows Hello > PIN > 主密码优先级自动选择。用单个 `loginMethod` 状态替代多个布尔标志，仅显示一种登录界面，不再叠加。切换账户时重置检测状态重新计算优先级。
- **导出密码限制取消** — 导出时任何非空密码均可使用（>=1 字符），不再限制复杂度（字母+数字+8位等），由用户自行决定密码强度。
- **PIN 输入组件重写** — PinInput 从受控 input 重构为隐藏真实输入框 + 纯视觉盒子（visual boxes）架构，消除光标偏移、输入冲突等问题。验证动画使用渐变流 + 斜纹光斑叠加效果，验证横线高度从 8px 减半至 4px。

### Fixed

- **锁定账户登录页面闪烁** — 全面消除 vault 锁定后重新挂载 LoginPage 的闪烁问题：
  - 模块级 `_cachedLoginMethod` 缓存登录方式跨卸载持久化，初始化时直接从缓存取值
  - 登录卡片 `minHeight: 420` 预留完整高度，账户选择器 `minHeight: 50` 预留空间
  - 移除加载转圈，`!selectedAccountId` 时不再提前设置 `bioChecked`/`pinChecked` 保护缓存值
  - 最终效果：锁定后重新挂载，TouchID/FaceID 按钮立即出现，零视觉过渡
- **PIN 并发竞态条件** — 修复 `spawn_blocking` 中的 Rust panic 问题，修复设置页 PIN 配置逻辑中的 race condition。
- **PIN 初次显示 bug** — 修复登录页面 PIN 默认显示问题，清理 `clear_credential` 逻辑中的警告。
- **Vault 锁定导航卡死** — 修复 vault 锁定后页面卡在「正在连接」界面无法跳转到登录页的问题。
- **settings.json 白屏崩溃** — 移除 settings.json 中残留的 JSON 注释（`//` 注释），修复深色/浅色模式切换时的白屏崩溃。
- **macOS 生物识别检测修复** — 修复 macOS 生物识别可用性检测逻辑，修正 `canEvaluatePolicy` 返回码判定，强化 strict 策略执行。
- **Windows COM 初始化修复** — `ensure_mta()` 中 `?` 操作符无法将 `()` 转换为 `BiometricError`，改用 `map_err` 显式转换为 `BiometricError::Other`。
- **Windows Hello 诊断日志** — 添加 Windows Hello 完整诊断与审计事件日志，记录可用性检查与验证结果。
- **代码批量修复 （P001-P005/P009/P013-P014）** — 批量修复代码审计问题。
- **水印插件生产构建启用** — 水印插件在 production 模式下可正常显示与使用。

### Refactored

- **PinInput 架构重写** — 从受控 `<input>` 组件重构为隐藏 input + 视觉盒子（visual boxes）架构，消除光标偏移、系统密码管理器拦截等问题，提升各浏览器兼容性。

### Docs

- **生物识别状态表更新** — 更新生物识别文档中的状态表格，修复 Windows Hello 的 save reason 参数与 unlock 审计日志记录描述。

### Chores

- 版本号同步升级到 2.5.8。
- 24 个 commit 自 v2.5.7（`ff2ad14b`）到 v2.5.8。
- 提交 CHANGELOG 摘要：`a3825bd7`、`27e2ccdf`、`5d35244d`、`71686c2a`、`d633b1d1`、`08401af7` 等。

## [2.5.7] - 2026-06-29

### Added

- **附件水印插件（P0）** — 新增图片/PDF 附件水印添加插件，支持文字水印（自定义文本、字号、颜色、透明度、旋转角度）、平铺/居中/四角定位、自定义输出目录与实时预览。插件自定义 UI 包含水印配置面板与结果区批量操作（全选/下载/清除）。
- **搜索敏感字段脱敏（P1-018/019/020）** — 搜索结果中敏感/关键级别的字段值不再显示具体内容，采用三层脱敏策略：对象级（sensitive/critical 整体跳过）、字段级（property_labels + __fields UUID 交叉引用）、模板级（回退到模板定义的属性敏感度）。
- **模板名称搜索** — 搜索框输入模板名称时，使用该模板的所有对象也会被包含在结果中，并在结果卡片中标注模板匹配信息。
- **搜索结果缓存** — 搜索结果增加 30 秒内存缓存，减少重复请求。
- **搜索页面入口** — 设置页「系统」分组新增搜索页面入口。
- **帮助搜索增强** — 帮助文档搜索支持全文内容匹配（基于预构建搜索索引），搜索结果排序优化（标题匹配权重更高），30 秒内存缓存。
- **新手引导动画增强** — Onboarding 教程卡片的「跳过」和「返回」按钮增加悬停动画。
- **侧边栏页面模式导航修复** — 搜索/OCR/插件按钮在页面模式下导航正确跳转。

### Security

- **P0 安全修复批次** — attachment_download 路径校验增强、Vision CLI 使用随机临时目录、CLI 不再使用 /tmp 作为回退路径。
- **P1 安全修复批次** — Windows icacls 用户名校验、导出路径禁用 /tmp 回退、导入 salt decode 错误修复。
- **OCR 扫描路径校验（P104/P206）** — ocr_scan_image 和 mrz 路径加固，template.rs 遗留警告清理。
- **附件存储安全（P103/P106/P107/P108/P109）** — 附件路径限制、inspect_backup OOM 修复、backup base64 清理、SHA-256 流式处理。
- **状态管理安全（P113/P114）** — ocrScanStore 退出时清理 scanHistory、authStore logout 设置 hasAccount=null。
- **附件重命名字符消毒（P207/P208）** — 附件文件名特殊字符消毒，对话消息限制 500 条。

### Fixed

- **P2xx 修复批次** — React hooks deps、localStorage 整合、CliError 迁移、CLI import 二次解密修复、密码修改回调展平。
- **P112/P116/P117** — RAG batch embedding 修复、.catch 静默错误修复、CLI N+1 查询消除。
- **P216/P228** — ESLint console 配置修正、CLI 测试密码改为常量。
- **P213** — 恢复附件 PDF 预览（通过 data URL 的 iframe 预览）。
- **P214/P219/P213/P209/P223/P227/P231/P226/P225** — 统计去重、useCallback、asset:// data URL、模板合并、死代码标签、println→error、字符串拼接、collapsible_match、unwrap 处理。
- **P217/P220/P221/P218/P222** — TrashDetailPanel 未使用变量、ChatMessageList key、react-markdown URL 白名单、ScanLocalPage 并发导入、CLI STREAMING_THRESHOLD 死代码。
- **P201** — 删除残留的 service.rs.bak 文件。
- **搜索结果修复** — 修复 6 项搜索 bug（系统页面名翻译、collectionType 筛选、模板搜索权限、关键级对象排除等）。
- **ConfirmDialog 国际化** — 确认/取消按钮使用国际化文本。
- **导出导入模板快照隔离** — 按 template property id 排序以保证确定性哈希。
- **水印插件修复** — 权限拦截修复（fs.copy_file、shell open scope）、workspace 归属判断、路径 symlink 解析、附件源路径拷贝、UI 按钮统一样式、注册表 custom_ui 序列化兼容。

### Changed

- **水印插件 Rewrite** — 从自定义 PDF 处理迁移到 pdfium-render + PDFium 库，支持更稳定的 PDF 水印添加。
- **水印插件国际化** — 全量支持中英文：配置面板标签、折叠栏标题、结果区按钮、配置摘要。

### Performance

- **LLM 流式输出优化（P111）** — emit_typing_effect 改为 20 字符批次发送。
- **附件 N+1 查询消除（P110）** — 新增 load_objects_batch VaultStore API，消除 attachment_count_batch 和 build_attachment_tree_pages 中的 N+1 查询。
- **RAG batch embedding（P112）** — 批量嵌入处理提升性能。

### Refactored

- **水印配置面板重构** — 配置摘要实时显示、全选/批量操作行合并、紧凑型按钮适配侧边栏卡片。

### Chores

- 版本号同步升级到 2.5.7。
- Clippy/Lint/ESLint 清理（P1/P2 代码质量修复）。
- 代码分析报告更新。

## [2.5.6] - 2026-06-27

### Security

- **流式加密导出/导入（P1-023/024）** — 大文件加解密改为分块流式处理，避免将完整密文加载到内存中，降低内存占用与侧信道风险。
- **HKDF-SHA256 密码验证（P2-010）** — 密码验证哈希从轻量级 Argon2id 迁移至 HKDF-SHA256，提升验证性能同时保持安全性。
- **Windows ACL 文件权限（P1-002）** — 通过 icacls 限制 Vault 数据目录的 Windows 文件权限，防止其他进程越权访问。
- **移除硬编码密钥（P1-003）** — 移除代码中硬编码的 BIO_FILE_KEY_SECRET，改为从密钥派生函数动态生成。
- **路径遍历防护（P2-012）** — 加固 OCR 与附件处理中的用户可控路径校验。
- **OCR Swift 安全加固（P2-014）** — 修复 OCR 模块中 Swift 代码的安全问题。
- **tauri.conf.json 架构修正（P1-007）** — schema 指向官方 tauri-apps/tauri。
- **插件运行时双重写入消除（P1-009）** — 合并 src-tauri/plugin/ 到 solosoul-plugin crate，消除同份数据写入两次的竞态条件。
- **全局 currentObject 单例替换（P1-017/018）** — 用 per-objectId 缓存替代全局单例，消除多对象并发编辑的竞态条件。
- **KDF 参数可配置** — 新增 SOLOSOUL_SECURE=1 环境变量切换生产模式 KDF 参数（64 MiB/3 iter），开发模式保持 8 MiB/2 iter。

### Fixed

- **回收站对象详情卡片"原位置"显示** — 自定义页面的 UUID 改为显示页面名称（使用 resolveCollectionLabel + useSettingsStore 解析），同时修复国际化。
- **回收站页面时间筛选切换闪烁** — 移除加载覆盖层（Loader2），旧数据在加载期间保持不变，新数据到达后无缝替换。
- **切换页面历史/附件 badge 数字闪烁** — 移除 snapshot counts 0 初始化，加载期间 badge 不显示，数据到达后统一显示实际数字。
- **快照区域横线调整** — 折叠时 toggle 下方显示横线，展开时由最后一个字段的 borderBottom 自然作底部边界。
- **ConfirmDialog 事件冒泡** — 修复覆盖层点击 stopPropagation()，防止点击 dismiss 时意外关闭附件卡片。
- **ObjectDetailModal/HistoryViewer/ExportSection 闪烁** — 全面引入懒加载 + fade-in 动画。
- **GlobalAttachmentManager 工具栏闪烁** — tab pills 改用 Button 组件，刷新按钮统一 variant=secondary。
- **附件预览修复** — 非图片附件无法预览修复；图片附件支持滚动、缩放与底部缩放工具栏。
- **复制按钮重影背景** — 消除在对话框背景上的白色重影，hover 时改为声明式样式 + 发光动画。
- **附件文件名 .* 后缀 bug** — 移除 save() 对话框的 filters 配置，避免自动追加 .*。
- **附件拖拽穿透与事件重复** — 修复多层元素穿透、计数刷新不及时、Tauri 重复事件去重。
- **地址格式化器默认国家徽章** — DEFAULT 显示"默认"/"Default"而非 raw code。
- **模板删除弹性修复** — 修复模板删除后 property_labels、fields、templateName、contractTypeId 残留导致的崩溃。
- **模板名称删除线统一** — 编辑器、工作区卡片、详情弹窗三处模板名称删除线统一检测 page category。
- **ESLint 警告清零（P1-022）** — 修复未使用变量、缺失依赖、死代码等所有 ESLint 警告。
- **Clippy/Fmt 修复（P0-002/003/004）** — 修复主项目的 clippy 警告与 cargo fmt。
- **前端性能优化（P2-019~025）** — 包括 ObjectWorkspacePage memo、搜索防抖、减少重渲染。
- **搜索防抖与 OCR 文档修复（P0-009/P2-008/P2-011）** — 搜索结果节流、OCR 文档路径修正、ort 版本兼容。

### Changed

- **ObjectWorkspacePage 重构（P1-013）** — 拆分为 useWorkspacePasswordGuard hook、ConfirmDeleteDialog、WorkspaceCategoryTabs 三个模块。
- **P1 组件提取大重构** — 提取 BadgeIconButton、DeleteButton、SelectCheckbox 等共享组件；消除所有 miniBtn/pgBtn 遗留用法；LLM 配置去重、附件工具函数抽取。
- **全站按钮样式统一** — 新增 danger-outline 变体（常态浅红底 + 亮红文字）；提取共享 DeleteButton 组件。
- **全站字体大小统一为语义化 token** — 新增 --text-body、--text-caption、--text-badge 等 token；591 处写死 fontSize 映射为 token。
- **全站页面布局标准化** — 新增 PageContainer、CardGrid 与 tokens.css 布局 token。
- **全站卡片 gap 统一** — 使用 --card-grid-gap token 布局，与字号比例对齐。
- **回收站恢复按钮无边框** — variant=secondary + accent 边框 -> variant=tertiary（无边框，hover 浅色底）。
- **回收站底部按钮右对齐** — 恢复和永久删除按钮容器添加 justifyContent: flex-end，垂直位置不变。
- **ConfirmDialog 字体/按钮对齐** — 标题/正文字号与对象删除确认框对齐；删除确认按钮改为 danger-outline。
- **附件 UI 统一** — TrashPage 附件标签按钮改为 pill 样式；图标和格式标签颜色统一为 var(--text-tertiary)。
- **附件管理页面字体统一** — GlobalAttachmentManager 内写死 fontSize 替换为排版 token。
- **附件侧边栏集成** — 附件管理入口加入侧边栏，导出/导入 tab UI 优化。
- **SelectCheckbox 增强** — 加入 accent 边框提升可见性，支持 indeterminate 状态。
- **插件复制按钮增强** — 统一添加主题色边框与发光动画反馈；复制成功文本国际化。
- **按钮 hover 状态统一** — 全站按钮采用声明式 hover 样式，消除内联 onMouseEnter/Leave。

### Added

- **附件下载功能** — AttachmentViewer 和 GlobalAttachmentManager 支持单个附件下载（系统 save 对话框）和批量下载（目录选择器）。新增 i18n key：download_result、download_failed、batch_download_result、select_download_directory。
- **附件批量选择 UI** — 提取 SelectCheckbox 组件，支持全选/取消全选/tri-state 不确定状态，批量工具栏始终可见。
- **CI/CD 工作流完善（P1-004/005）** — 新增 CLI 检查、macOS/Windows 构建、Release draft 作业。
- **vitest 配置迁移（P1-012）** — --passWithNoTests 标志迁移到 vitest 配置文件。
- **tauri-apps/cli 移入 devDependencies（P1-008）** — 减少生产依赖体积。

### Refactored

- **ObjectWorkspacePage 拆分** — 提取密码保护 hook、确认删除对话框、分类标签页。
- **BadgeIconButton 组件提取** — 消除 4 处重复的 badge+icon+button 组合实现。
- **DeleteButton 组件提取** — 附件管理、回收站等场景统一使用 danger-outline 按钮。
- **Button 变体统一** — tertiary（无边框）、danger-outline（危险操作）等变体标准化。
- **全局 currentObject 缓存重构** — 从全局单例改为 per-objectId 缓存 Map。
- **插件 crate 合并** — src-tauri/plugin/ 合并到 solosoul-plugin crate。
- **Cargo.toml workspace 对齐（P2-004/005）** — 统一 workspace 内各 crate 的依赖版本。
- **LLM 配置去重** — 抽取通用 LLM 配置逻辑，消除重复代码。
- **附件工具函数抽取** — useDragToAttach 等 hooks 提取为独立模块。

### i18n

- **81 条新 i18n key** — 包括 fs_is_dir、文件夹拖拽过滤、cannot_open_file、附件下载相关等。
- **HistoryPage 和 GlobalAttachmentManager 国际化** — Toast 消息全面支持中英文。
- **地址格式化器国家徽章本地化** — DEFAULT 国家代码显示"默认"/"Default"，国家名根据 locale 显示。
- **插件复制按钮文本国际化** — Copied/已复制 等反馈文本支持中英文。

### Docs

- **design_map IPC 规范更新** — IPC 命令从 66 个扩容到 155 个，新增完整命令列表与参数描述。
- **design_map Zustand Store 文档** — 根据实际 16 个 store 实现更新状态管理文档。
- **design_map 索引修复** — 解决文档索引冲突，补充实现细节。
- **CODE_ANALYSIS_REPORT 状态更新** — 标记已修复的审计项目。

### Chores

- 版本号同步升级到 2.5.6。
- cargo fmt 格式化及代码清理。
- 修复 workspace Cargo.toml 版本对齐。

## [2.5.5] - 2026-06-24

### Fixed

- **附件预览无法打开文件** — 新增 Rust 命令 `attachment_open`，绕过 `tauri-plugin-shell` 默认仅允许 `http(s)/mailto/tel` 的 open 校验，直接从 Vault attachments 目录调用系统默认应用打开非图片附件，并校验路径必须在 Vault 存储范围内。
- **图片附件预览无法滚动和缩放** — `AttachmentPreviewOverlay` 改为可滚动容器，支持按原始尺寸滚动浏览；新增底部缩放工具栏（放大 / 缩小 / 适应窗口）以及 Ctrl/Cmd + 滚轮缩放。
- **地址格式化器国家徽章国际化** — `PluginResultPanel` 将插件返回的 `DEFAULT` 国家代码识别为“未检测到国家”，中文显示 `默认`，英文显示 `Default`。

### Changed

- **全站页面布局标准化** — 新增 `PageContainer` 共享容器组件、`CardGrid` 卡片网格组件与 `tokens.css` 布局/排版 token（`--page-max-width*`、`--section-gap`、`--card-grid-*`、`--text-page-title` 等）。首页保持 720px 作为内容/列表页标准，设置/回收站/搜索统一 600px，编辑器/同步/LLM 配置统一 560px，表单/设置详情页统一 480px。20+ 页面从写死 `maxWidth` 改为引用 token，以后改一处即可统一全站宽度、间距与字号。
- **附件管理页面字体统一** — `GlobalAttachmentManager` 内所有写死 `fontSize`（10/11/12/13px）替换为排版 token（`--text-2xs`、`--text-xs`、`--text-body-sm`），与全站 token 保持一致。
- **全站字体大小统一为语义化 token** — 新增 `--text-body`、`--text-caption`、`--text-badge` 等语义字号 token；通过 codemod 将 125 个文件、约 591 处写死 `fontSize` 像素值按“首页层次”映射为 token（24→`--text-xl`、20→`--text-page-title`、18→`--text-md`、16→`--text-section-title`、15→`--text-card-title`、14→`--text-body`、13→`--text-body-sm`、12→`--text-caption`、11/10/9→`--text-badge`）。

### Added

- **附件下载功能** — AttachmentViewer 和 GlobalAttachmentManager 支持单个附件下载（系统 `save` 对话框）和批量下载（目录选择器），点击后复制 Vault 文件到用户选择的目标路径。新增 i18n key：`download_result`、`download_failed`、`batch_download_result`、`select_download_directory`。
- **附件下载文件名 bug 修复** — 移除 `save()` 对话框的 `filters` 配置，避免系统对话框自动给文件名追加 `.*` 后缀。

### Fixed

- **附件文件名 `.*` 后缀 bug** — 移除 `save()` 对话框的 `filters: [{name:'All Files', extensions:['*']}]`，避免系统对话框自动给文件名追加 `.*`。
- **附件拖拽穿透与事件重复** — 修复拖拽上传时多层元素穿透、计数刷新不及时、Tauri 重复事件去重问题。
- **回收站附件按钮顺序** — 附件卡片回收站批量工具栏恢复按钮与永久删除按钮对调。
- **回收站附件删除线** — 软删除附件行仅文件名有删除线，图标、大小日期、格式标签正常显示。
- **代码审查修复** — useDragToAttach 错误处理、清除过期 console.error。

### Changed

- **附件 UI 统一** — TrashPage 附件标签按钮改为 pill 样式；附件图标和格式标签颜色统一为 `var(--text-tertiary)`，消除 PDF 红色、图片强调色的差异。

### i18n

- **81 条新 i18n key** — 包括 `fs_is_dir`、文件夹拖拽过滤、`cannot_open_file` 等。
- **HistoryPage 和 GlobalAttachmentManager 国际化** — Toast 消息全面支持中英文。
- **`dialog_subtitle` 修复** — 从空字符串改为正确的占位符文本。

### Chores

- 版本号同步升级到 2.5.5。
- `cargo fmt` 格式化及代码清理。


## [2.5.4] - 2026-06-24

### Added

- **附件批量操作** — 附件管理页面支持批量选择、批量删除与恢复，抽离 `useBatchSelect`/`useAttachmentPageSort` hooks，支持附件重命名。
- **插件自适应 Key 截断** — 插件结果区 Key 不再硬编码 3em max-width，改为根据内容自适应截断，避免长 Key 过早换行。
- **插件国家徽章本地化** — 地址格式化器结果中的国家徽章根据界面 locale 显示本地化名称（中文环境显示「中国」，英文显示代码「CN」）。
- **侧边栏插件结果 UI 统一** — `QuickRunningInfo` 改用 `PluginResultPanel`，与插件页面共享同一套结果渲染组件。
- **OCR MRZ 模板匹配 Strategy C** — 新增 MRZ 模板匹配策略 C，改进 `split_mrz_lines` 方法，提升 MRZ 识别准确率。

### Fixed

- **附件/快照徽章溢出** — 数量超过 99 时显示 `99+`，避免徽章宽度溢出。
- **插件 UI 修复集** — 修复徽章布局（已安装始终可见、Completed 使用绿色）、Key/Value 截断与换行、侧边栏结果区 UI 与插件页面对齐、日志默认展开、Toast 重复防止等 10+ 项 UI 问题。
- **ESLint 问题修复** — 解决 E001-E007 共 7 个 ESLint 警告。
- **Clippy 警告修复** — 解决 P009-P010 共 11 个 OCR 代码中的 clippy 警告。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.4`。

### Refactored

- **插件共享可折叠组件提取** — 提取统一的可折叠日志/结果组件，消除插件页面与侧边栏的重复实现。
- **OCR 函数重命名** — `locate_mrz_region_flutter` 重命名为 `locate_mrz_region`，清除 Flutter 遗留引用。

### Chores

- 移除遗留的 `CODE_ANALYSIS_REPORT.md`。
- 更新过时注释，清理旧死代码。

## [2.5.3] - 2026-06-21

### Added

- **侧边栏功能按钮区折叠展开（SecondaryActionBar）** — 垂直模式下 hover 折叠区域箭头展开/离开自动折叠，支持 200px 可滚动按钮区，隐藏滚动条。展开过渡期间 `pointer-events: none` 防止 tooltip 闪烁。
- **侧边栏功能按钮区折叠展开（TopFunctionBar）** — 水平模式（top/bottom）同样支持 hover 展开/折叠，使用 ChevronRight 箭头指示方向，按钮区水平滚动。
- **Lock/Settings 按钮固定在折叠区外部** — 锁定账户和设置按钮始终固定在侧边栏底部（垂直模式）或功能条右侧（水平模式），不受折叠影响。
- **Zustand store 持久化展开/滚动状态** — `useSidebarHoverStore` 跨页面导航持久化 `isHovering`、滚动条位置（`verticalScrollTop`/`horizontalScrollLeft`），消除路由切换时状态丢失。
- **`document.documentElement` mouseleave 兜底** — 鼠标离开窗口时自动折叠功能区域。

### Fixed

- **折叠区域箭头方向** — 水平模式（top/bottom）箭头方向翻转：折叠时显示 ChevronLeft（←），展开时显示 ChevronRight（→），符合用户直觉。
- **底部（bottom）侧边栏不显示水平折叠** — `AppShell.tsx` 将 `{isTop ? <TopFunctionBar /> : <SideNavigation />}` 改为 `{isHorizontal ? ...}`，bottom 模式正确渲染 TopFunctionBar 并添加 `paddingBottom`。
- **底部 bar 位置错误** — TopFunctionBar CSS `position: fixed; top: 0` 覆盖了 flex 布局，新增动态 `bottom: 0` + `borderTop`。
- **展开后按钮无法点击** — `onTransitionEnd` 在 React 重渲染时可能不触发导致 `pointer-events: none` 永久生效，改为 `useEffect` + `setTimeout`（200ms）兜底。
- **卡片/页面按钮导航后折叠** — 关闭卡片或点击页面按钮后，如果鼠标仍在功能区域内保持展开。
- **跨页面导航滚动条闪烁** — `useLayoutEffect` 替代 `useEffect` 消除 `scrollTop`/`scrollLeft` 恢复时的视觉闪烁。
- **Settings 按钮无高亮状态** — 添加 `isActive={location.pathname.startsWith('/settings')}`。
- **LoginPage 生物识别闪烁** — 移除早期返回 bare `<div>`，始终渲染完整卡片布局，bio check 期间显示 Loader2 加载动画。
- **展开过渡期间 name card 闪烁** — 按钮容器在过渡期间 `pointer-events: none` 防止 tooltip 随移动按钮闪烁。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.3`。
- **按钮顺序调整** — 功能按钮区顺序为：搜索→回收站→模板管理→插件→OCR→导入导出→帮助文档→AI 对话。
- **底部按钮自定义配置移除** — `sidebar_bottom_actions` 相关配置项删除，按钮改为固定 8 个功能按钮 + Lock + Settings。

## [2.5.2] - 2026-06-21

### Added

- **侧边栏插件快捷面板** — 侧边栏点击插件按钮弹出浮动卡片，支持「全部/已安装/运行中」三个标签页，内联安装/卸载/运行操作，日志和结果内联展示，关闭卡片后后台运行不受影响。
- **侧边栏按钮卡片/页面模式切换** — 为 OCR 扫描、插件、AI 对话、搜索 4 个按钮提供可配置的卡片/页面两种模式，用户可在「主题与外观」设置中自由切换，默认均为卡片模式。

### Fixed

- **Auth 页面错误条件顺序** — 修复 BootstrapPage 和 LoginPage 中后端返回密码长度错误时被错误归类为「密码不正确」的问题，'8 characters' 检查现在优先于 'password' 通用检查。
- **插件注册表/安装降级修复** — 修复注册表更新和安装回退场景下的 2 个 bug。

### Changed

- **版本号统一** — 全平台版本号同步升级到 2.5.2。

## [2.5.1] - 2026-06-20

### Added

- **插件注册表后台自动刷新** — 应用启动时后台自动调用 `update_registry`，带 1 小时速率限制，失败不阻塞启动。
- **插件手动刷新按钮** — 插件仪表盘页新增手动刷新注册表按钮，带加载旋转动画。
- **远程插件安装** — `install_from_registry` 现在从远程下载最新的 manifest + WASM，使插件更新与软件版本真正分离。
- **注册表缓存分离** — `registry.json` 写入可写应用数据目录（`~/.solosoul/`），而非只读资源目录。
- **地址格式化器国家徽章** — 插件结果中每条地址左侧显示国家徽章（中文环境显示国家名如「中国」，英文显示代码如「CN」）。
- **图标分类国际化** — 侧边栏新建页面、重命名页面、新建模板的图标选择器分类名支持中英文切换。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.1`。
- **已安装插件存储分离** — WASM 文件存储路径从 `PluginStore` 独立为应用数据目录，bundled 资源仅作离线回退。
- **默认侧边栏按钮** — 新用户默认侧边栏底部按钮调整为「搜索」「模板管理」「帮助文档」；检测到旧默认值自动迁移。
- **地址格式化器 v1.0.5** — 日志与结果支持国际化（通过 `locale` 参数）。

### Fixed

- **模板详情卡片图标** — `TemplateDetailModal`、`SampleTemplateDetail`、`TemplateEditor` 的字段类型图标统一使用 `FieldTypeIcon` 组件（Lucide 图标），与对象编辑器保持一致。
- **PluginCard 国际化** — 日志级别、JSON/Markdown 标签、复制按钮支持中英文。
- **pluginStore Toast 错误处理** — 插件运行失败时显示红色错误 Toast。
- **BootstrapPage 密码提示** — 两次密码不一致时显示错误提示。
- **BootstrapPage 密码不匹配** — 修复 BootstrapPage 中密码验证不一致的问题。

## [2.5.0] - 2026-06-20

### Added

- **插件系统正式开放** — 插件系统对发布版本开放，已内置"地址格式化器"插件，支持10个国家/地区的地址规范格式化输出。更多插件将在后续版本中发布。
- **首页插件快捷入口** — 在首页「快速入口」区域新增插件卡片，一键跳转插件管理页面。
- **插件结果区折叠展开** — 插件结果区与日志区统一支持折叠/展开，默认展开，方便管理多插件输出。
- **Toast 国际化支持** — 插件运行完成的 Toast 通知支持中英文切换。
- **复制按钮反馈优化** — 所有复制按钮（日志、JSON、Markdown、逐条地址）统一添加主题色边框与柔光效果反馈。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.0`。
- **插件 UI 对齐** — 「运行中」tab 的结果卡片样式与「已安装」tab 统一，移除旧式分离输出卡片。
- **发布版本插件过滤** — 生产环境仅显示地址格式化器，开发/调试模式始终显示全部插件。
- **Sidebar 底部按钮 i18n** — 英文设置下"Position N"改为"Button N"，侧边栏底部"Plugins"选项正确显示国际化文本。

### Fixed

- **PluginGatePage 阻挡** — 移除了生产环境下的"插件系统开发中"门禁页，插件页面始终可用。
- **「全部」tab 运行插件跳转** — 在「全部」tab 点击运行后自动切换到「已安装」tab，避免出现旧样式结果卡片。
- **导航翻译缺失** — 补充 `navigation:plugins` 翻译键，侧边栏底部按钮选择器中的 Plugins 选项正确显示中英文。

## [2.4.1] - 2026-06-19

### Added

- **插件模板系统 Stage 3 — v17 幂等性 + 部分 DB 状态测试** — 为 Stage 1 落地的 v17 migration 在 `solosoul-vault/src/migration.rs::tests` 内新增两条验收测试:
  - `test_migration_v17_idempotent_run_twice` —— 两次顺序执行 `run_migrations()` 后断言 `schema_migrations.version = 17` 仅 1 行,证实 gate `current < 17` 在第二次调用中正确 skip;且两表 `contract_type_id` 均 `notnull = 0` 且 `(dflt_value IS NULL OR dflt_value = '')` (Option B 合约 — NULL 与空串都是「无合约」合法表述)。
  - `test_migration_v17_partial_state` —— 4 个阶梯 mod 内 for-loop 覆盖 `(user_templates.contract_type_id, objects.contract_type_id)` 四种「有/无」组合: 双列均无 (fresh install), 仅 `user_templates` 有, 仅 `objects` 有, 双列已有 (no-op path)。每个阶梯结尾都验证两表都有列 + `schema_migrations.version=17` 仅 1 行 + `data_version == 17`。
  - 新增 `setup_v16_partial_state(has_utpl_ctid, has_objects_ctid) -> (Connection, TempDir)` 辅助,内嵌 `HELPERS_PARTIAL_V16_SQL` 常量与 `/*UTPL_CTID*/` / `/*OBJECTS_CTID*/` 站位标记,以 `.replace()` 条件投送 `contract_type_id TEXT,` 行为。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.4.1`。
- **大组件拆分 Round 2 (P056–P059)** — 拆分 4 个大型前端组件为独立子组件，降低维护成本：
  - `SideNavigation.tsx` (1026 行) → `PrimaryNavZone` / `SecondaryActionBar` / `RenameableNavButton` / `AddPageButton`
  - `AiQuickChatPopover.tsx` (1009 行) → `ChatMessageList` / `ChatInputBar` / `ConversationHistory` / `UnconfiguredHint`
  - `LlmConfigPage.tsx` (929 行) → `AiFeaturesCard` / `SystemPromptCard` / `ProviderManagerPanel` / `LocalEmbeddingsPanel` / `KnowledgeBaseCard` / `RiskAcceptanceDialog`
  - `OcrQuickScanPopover.tsx` (789 行) → `OcrPopoverHeader` / `OcrHistoryTrashDropdown` / `OcrScanControls` / `OcrResultPanel`
- **大组件拆分 Round 1 (P052–P055)** — 拆分 `TrashPage`、`LlmChatPage`、`TemplateManagerPage`、`ExportImportPage` 为独立子组件与自定义 hook。
- **代码审计 P060–P062** — 修复 `lib.rs:491` `.expect()` 为优雅退出；提取 `TemplateTypeSelect`、`TemplatePageSelect`；提取 `useExportEstimate` hook 替代复杂 `useMemo`。`CODE_ANALYSIS_REPORT.md` 全部 20 项 P2 问题已修复。
- **代码格式化清理** — 对 `migration.rs` 中 Stage 3 测试区块与 Stage 1 落库时 `lib.rs` 中 `#[serde(...)]` 属性的 rustfmt 漂移进行规范化 (`cargo fmt --all`)。

### Fixed

- **`cargo clippy --workspace --all-targets -- -D warnings` 警告** — 修复 `migration.rs::tests::setup_v16_partial_state` 中 `let mut conn = Connection::open(...)` 的 `unused_mut` 警告 (`Connection::open` / `execute_batch` / `set_schema_version` 均只需 `&self`)。

## [2.3.3] - 2026-06-18

### Added

- **插件模板系统 Stage 1** — 新增插件兼容模板 Schema 与数据库 v17 migration，为插件市场模板化能力奠定基础。
- **Windows 静默自动更新** — 启用 Windows 端静默下载与安装更新，减少用户手动干预。
- **CLI 设置阶段重构** — `/setting` 命令从原始卡片替换为 `SettingsMenu`，提升终端设置体验。
- **CLI 扩展命令** — 新增 `/sync`、`/ocr`、`/embed_model` 命令；CLI 支持 `--ocr scan --mrz` 等参数，并补齐 CLI Release 产物构建。
- **模板库过滤优化** — 过滤时保持卡片高度稳定，并将可见卡片优先排列。
- **OCR 历史记录交互** — 点击 OCR 历史条目可直接加载扫描结果；补充 `active_model_series` 国际化并显示模型系列名称。
- **侧边栏搜索激活态** — 搜索弹窗打开时，侧边栏搜索导航按钮显示为激活状态。

### Changed

- **日志降噪** — 在 `npm run tauri dev` 期间静默 ONNX Runtime（`ort` crate）的 INFO 级别日志 spam。
- **代码格式化清理** — 对 `llm/service.rs`、`cipher.rs` 等文件进行 rustfmt 规范化整理。

## [2.3.2] - 2026-06-17

### Added

- **首页快捷入口** — 为回收站、OCR、搜索、AI 聊天添加快速访问卡片，优化高频功能入口。
- **自动更新下载进度** — 更新横幅显示下载进度，支持后台下载更新。
- **macOS 更新包生成** — Release 构建脚本生成 `.app.tar.gz` 供 Tauri updater 使用，替代 DMG 作为更新载体。
- **统一签名脚本** — 新增 `docs/sign_artifacts.sh`，在 macOS 上为 Windows `.exe` 和 macOS `.app.tar.gz` 统一生成 updater 签名，避免在 Windows 构建机暴露私钥。
- **PDFium 自动下载** — Release 构建脚本在缺失 PDFium 动态库时自动下载对应平台库。

### Changed

- **帮助入口调整** — 将帮助卡片从数据区移至快捷入口区。
- **快捷入口位置** — 将快捷入口区域调整到数据区下方，优化信息层级。
- **Release 签名策略** — `.sig` 签名文件不再上传到 GitHub Release，仅嵌入 `latest.json`。

### Fixed

- **帮助页面国际化** — 修正帮助页面标题的 i18n key。
- **快捷入口返回行为** — 修复快捷访问目标页面的返回按钮无法正确回到首页的问题。
- **代码审计 P048-P051** — 修复 `llm/service.rs` 中的 `.expect()`、`decrypt_chunked_from_bytes` 中的 `.unwrap()`、`OcrQuickScanPopover` 死代码，以及前端 `react-hooks/exhaustive-deps` 警告；使用 `editingTemplateId` memo 避免 `TemplateManagerPage` 过度重渲染。
- **更新后重启** — 注册 `tauri-plugin-process` 并授予 `process:default` ACL，允许更新完成后自动重启应用。
- **macOS 更新包元数据** — 禁用 `tar` 生成 `._*` AppleDouble 文件，避免 Tauri updater 解压报错。
- **PDFium 下载路径** — 修正 Windows DLL 的下载源路径。

## [2.3.1] - 2026-06-17

### Added

- **OCR PDF/MRZ 扩展** — 支持扫描/解析 PDF 文档与机读区（MRZ），识别结果可导入为对象。
- **自动更新签名集成** — Release 构建脚本自动生成 `.sig` 签名文件，并生成 Tauri updater 所需的 `latest.json`。
- **本地模型文件前置检查** — macOS/Windows 构建脚本在编译前检查 OCR 与 Embedding 模型是否完整，避免打包失败。

### Changed

- **国际化补全** — NSIS 安装程序欢迎/完成页、OCR 模型选项、帮助文档标题支持多语言。
- **发布脚本增强** — 自动检测 `~/.tauri/secret.key`，按版本号过滤产物生成 `latest.json`，补充签名密钥管理文档。

### Fixed

- **OCR 侧边栏快速扫描** — 修复快速扫描卡片阻塞性 bug 与相关体验问题。
- **NSIS 模板语法** — 修复内联 i18n 注释中的 Handlebars 转义问题，避免安装包构建异常。
- **签名路径** — 修复 macOS/Windows 构建脚本在 `tauri` 子目录中调用 `tauri signer` 时相对路径错误的 bug。

## [2.3.0] - 2026-06-17

### Added

- **本地 OCR 引擎（PP-OCRv6）** — 集成 ONNX Runtime 本地文字识别，支持 tiny/small/medium 三档模型；扫描图片/附件后可导入为对象，并补充测试、文档与审计日志。
- **OCR 首次启动静默安装** — 新账户首次启动时自动在后台下载并安装 small 模型，避免用户首次扫描时无模型可用。
- **macOS 生物识别改进** — 支持 Touch ID 凭证迁移到 Keychain UserPresence；修改主密码后自动更新生物识别凭证，无需重新启用。
- **SoloSoul CLI Phase 5 插件系统** — 终端完整支持插件生命周期：`/plugin_install`、`/plugin_update`、`/plugin_uninstall`、`/plugin_run`（支持 `key=value` 运行时参数，后台线程异步运行 Wasm 插件）、`/plugin_search`、`/plugin_sessions`，以及插件列表内联过滤、批量快捷键与详情屏幕。
- **SoloSoul CLI Phase 5 LLM 能力** — 终端新增 LLM 配置、对话、聊天与统计命令，核心能力下沉到 `solosoul-core`。
- **生物识别实现规范** — 新增 `docs/biometric-spec.md`，记录 macOS Touch ID 与 Windows Hello 实现约束及回退策略。
- **同步服务跨二进制复用** — 将 `SyncService` 移入 `solosoul-sync` crate，供 GUI 与 CLI 共享。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.3.0`。
- **扫描页模型描述** — 模型系列改为变量管理，描述不再硬编码模型名，便于后续扩展。
- **OCR 无模型提示** — 未安装模型时触发扫描，改为弹出国际化提示引导用户下载。
- **macOS 标题栏与 UI 修复** — 优化安全警告、国际化文案、NSIS 资源与 macOS 标题栏行为。
- **Updater ACL 权限** — 授予 `updater:default` ACL 权限，允许客户端正常下载并安装更新。

### Fixed

- **生物识别 Touch ID 解锁失败** — 修复修改密码或锁定后 Touch ID 无法解锁的问题。
- **生物识别 Keychain 回退** — 开发模式下 Keychain `-34018` 或 `not_found` 时自动回退到本地文件存储，避免反复弹出钥匙串输入框。
- **生物识别凭证迁移** — 修复旧账户生物识别迁移与关闭失败、本地凭证文件误删等问题。
- **生物识别取消提示** — 修复生物识别测试取消时错误使用国际化消息的问题。
- **Windows 构建失败** — 将 `macos_keychain` 模块限制为仅 macOS 编译，修复 Windows 构建时找不到 `core_foundation` / `security_framework` 的问题。
- **NSIS i18n.nsh 打包失败** — 将 `i18n.nsh` 内容内联到 `installer.nsi`，避免 Windows 构建时因未复制该文件导致 `could not find: "i18n.nsh"` 错误。

## [2.2.2] - 2026-06-16

### Added

- **Windows 启动崩溃排查基础设施** — 增加文件日志、panic 捕获、启动前检查（WebView2 / 数据目录 / 资源目录），便于定位安装后双击闪退。
- **插件市场缺失优雅降级** — 当打包资源中 `SoloSoul_plugin_market` 缺失时不再 panic，记录警告后继续启动。
- **SoloSoul CLI Phase 4 M1+M2 增强能力** — 终端 UI 与命令体系进一步增强。
- **代码审计清零** — `CODE_ANALYSIS_REPORT.md` 全部 47 项问题已修复或确认关闭。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.2.2`。
- **大组件拆分** — `LlmChatPage` 提取 `ChatMessageBubble`，`SideNavigation` 提取 `AiQuickChatPopover` 独立文件。
- **共享工具函数** — `formatTimestamp` / `formatRelative` 提取到 `src/lib/time.ts`，复制反馈时长与 debounce 延迟提取到 `src/lib/constants.ts`。

### Fixed

- **Windows 安装后闪退** — 修复在部分 Windows 环境下安装包安装成功但双击启动即崩溃的问题。
- **代码审计剩余 16 项** — 完成 P004、P019-P022、P029、P030、P033、P034、P036、P041-P046。
- **原生对话框替换** — 替换 4 处原生 `alert()` / `confirm()` 为项目统一的 `ConfirmDialog`。
- **生产日志清理** — 移除 `App.tsx` onboarding 路径的 `console.warn`。
- **Clippy 警告** — 修复 `lib.rs` 中的 `redundant_closure` 警告。

## [2.2.1] - 2026-06-15

### Added

- **SoloSoul CLI Phase 0–5 完整交付** — 独立终端 TUI 客户端达到与 GUI 一致的核心能力：账户创建/登录/自动锁定、Vault 只读与写入命令、对象向导、附件/备份/导入导出/设置/安全子命令。
- **CLI 核心库抽取** — 新增 `solosoul-core` crate，迁移 `VaultService`、模板、生物识别、密码验证核心逻辑，供 GUI 与 CLI 共享。
- **CLI 终端 UI** — 基于 `ratatui` 0.30.1 实现全屏 TUI，包含命令输入框、屏幕渲染、自动补全、字段编辑器、模态提示与密码输入框。
- **CLI 命令体系** — 支持 `/unlock`、`/lock`、`/list`、`/open`、`/size`、`/search`、`/history`、`/rollback`、`/newpage`、`/newobject`、`/edit`、`/delete`、`/trash`、`/restore`、`/purge`、`/operation_log`、`/export_log`、`/attach`、`/backup`、`/export`、`/import`、`/language`、`/theme`、`/setting`、`/security`、`/debug_log`、`/about`、`/help`。
- **CLI 品牌视觉** — 登录页/首页采用暖色仪表盘风格，新增 Codebuff 风格 Unicode Logo、FIGlet banner、扫光动画与 GUI 品牌蓝色主题。

### Changed

- **CLI 登录体验** — 账号名/选择器始终置顶，生物识别/密码选项改为可点击卡片，禁用 emoji 并加入鼠标悬停动画。
- **CLI 命令补全** — 根据当前阶段动态过滤命令，未解锁时隐藏登录后命令，向导内仅提供安全命令。
- **CLI 日志与锁定** — 日志写入 `{data_dir}/logs/cli.log`，5 分钟无操作自动锁定 Vault，模态打开期间暂停锁定计时。

### Fixed

- **GUI 窗口大小恢复** — 修复权限问题与同步缓存覆盖导致的启动后尺寸无法恢复。
- **GUI 关键字段日志** — 修复关键字段操作日志中的对象名、页面与属性名记录。
- **GUI 侧边栏滚动** — 修复各模式下滚动条显示与图标偏移。
- **GUI 搜索闪烁** — 修复搜索弹窗与结果高亮的闪烁问题。
- **GUI 主题悬停色** — 统一深色/浅色模式下的悬停颜色。

## [2.2.0] - 2026-06-14

### Added

- **端到端设备同步** — 实现 HLC / Noise XX / CRDT 的本地设备同步 P2/P3 完整链路，并在设置页增加「设备同步」入口。
- **插件系统 Phase 1-4** — 恢复并实现完整插件生命周期：真实 Vault 字段读取、网络代理、阻塞式 Consent、结果导出为 JSON / Markdown、在线注册表更新与 Minisign 签名验证、插件运行状态持久化与 Wasm 崩溃隔离。
- **插件开发资源** — 新增 JS / Python SDK 占位目录与 Wasm 插件开发指南。
- **模板管理增强** — 模板页面新增筛选与搜索功能；系统示例模板拆分为中文 / 英文独立子页面。
- **帮助文档** — 重写并更名帮助文档中的「同步」章节为「设备同步」。

### Changed

- **Windows 安装包品牌化** — NSIS 安装程序 UI 与品牌视觉一致，资源生成脚本对缺失 Pillow 更健壮。
- **macOS 应用标识** — 修正 macOS bundle 配置键，正确设置 Dock 名称与应用名。
- **登录页交互** — 账号名/选择器始终显示在所有登录方式上方；生物识别按钮动效与悬停样式统一。
- **搜索与布局** — 搜索弹窗自适应、结果卡片布局与高亮优化；主内容区增加 `scrollbar-gutter: stable` 防止滚动条抖动。
- **侧边栏滚动条** — 各模式侧边栏滚动条完全隐藏，避免 Windows 下图标偏移。

### Fixed

- **代码审计 P0/P1 问题** — 落地 R001–R032 与 F001–F032 中可修复的安全与稳定性问题。
- **Vault 加密** — 敏感字段使用 AES-256-GCM 加密存储；VaultService IO 移出异步锁路径，大文件加解密改为流式处理以限制内存占用。
- **生物识别安全** — 生物识别主密钥从混淆文件迁移至系统 Keychain。
- **路径遍历防护** — 清理附件导入/导出、备份文件名、嵌入模型 ID 等用户可控路径。
- **账户创建竞态** — 序列化 `create_account` 并拒绝重复 active ID 的对象创建。
- **登录性能** — 密码登录 KDF 与账户列表查询移至阻塞线程池。
- **异步竞态与内存泄漏** — 引入 `useCancellable` 保护关键 effect，清理剩余 copy/search timeout 监听器。

### Security

- 修复审计报告中的敏感数据明文落盘、硬编码 XOR 混淆、路径遍历、命令越权访问等问题。
- 限制 `fs` 类命令可访问路径，禁止目录穿越。
- 备份与附件操作改为精确 ID 匹配，避免前缀匹配误删/误读。

## [2.1.0] - 2026-06-12

### Added

- **新手引导** — 首次启动时显示可跳过的新手教程卡片，完成状态持久化到 `ui_preferences.json`；若已存在账号则自动跳过。
- **应用内更新检测** — About 页面自动检测 GitHub Releases 新版本并显示更新横幅，支持一键下载安装。
- **NSIS 安装程序品牌化** — Windows 安装包支持语言选择（简体中文/英文），并显示自定义欢迎与完成文案。
- **模板示例库** — 模板管理页新增 8 个系统推荐模板示例，一键以此模板创建对象。
- **关键字段默认敏感度** — 系统推荐模板中的身份证、护照、银行卡等关键字段默认标记为 `critical`。
- **图标系统扩展** — 侧边栏新建页面卡片可直接选择图标，支持更多图标选项。
- **审计日志增强** — 密码登录、Touch ID 解锁、Face ID 解锁以及关键字段查看均写入操作日志，关键字段日志包含对象名、所属页面和属性名。
- **窗口大小持久化** — 窗口尺寸同时保存到明文 UI 偏好（登录前可用）和加密账户偏好（账户级隔离），启动时自动恢复。

### Changed

- **搜索结果卡片** — 敏感度标签按 `公开/内部/敏感/关键` 顺序排列，命中的字段名/值使用主题色高亮。
- **搜索交互** — 搜索结果点击对象后打开详情模态框（而非编辑器），支持附件查看。
- **生物识别实现** — macOS 生物识别改用 objc2 FFI，移除运行时 `swiftc` 编译依赖，不再要求安装 Xcode Command Line Tools。
- **Windows 标题栏** — Windows 系统标题栏颜色跟随应用主题。
- **侧边栏底部按钮** — 支持自定义 3 个底部快捷按钮位置。
- **登录页视觉** — 按钮加入浮动动画与边框高亮。
- **安全设置** — 自动锁定时长绑定状态并加入说明，新增主密码警告文案。
- **操作日志** — 页面创建日志国际化并按类型区分颜色，生物识别解锁显示具体类型（Touch ID / Face ID）。

### Fixed

- **主内容区纯色背景** — 修复页面变成纯色内容、仅显示侧边栏和 AppBar 的问题。
- **侧边栏滚动条** — 左/右/上/下各模式均完全隐藏滚动条，避免 Windows 图标偏移。
- **滚动条占位** — 主内容区增加 `scrollbar-gutter: stable`，防止滚动条出现/消失时内容左右偏移。
- **窗口大小恢复** — 修复启动时磁盘旧值覆盖同步缓存导致无法恢复上次尺寸的问题。
- **NSIS 卸载流程** — 维护模式卸载成功后直接退出安装程序，不再重新进入安装流程。
- **macOS 关闭按钮** — 修复 `onCloseRequested` 监听器导致交通灯关闭按钮无法关闭窗口的问题。
- **Windows 编译** — 修复 `windows` crate 版本对齐与 HWND 类型转换问题。
- **LLM 统计** — 修复重置统计按钮在深色模式下的显示。
- **启动页面** — Bootstrap 密码提示词同步修复。

## [2.0.1] - 2026-06-12

### Fixed

- **macOS Bootstrap Page Input (v2)** — Password fields on first-run wizard no longer blocked by WKWebView password manager. Replaced `<input type="password">` with `type="text"` + CSS `-webkit-text-security: disc` to bypass WebKit keyboard interception on forms with multiple password fields. Also applied to login page password input.
- **Password Change Command** — Fixed `Command vault_change_password not found` error on Settings page. Frontend was calling `vault_change_password` but Rust command was registered as `change_password`.
- **Password Hint After Password Change** — When updating password + hint simultaneously, the hint update now uses the new password (instead of the old one, which failed because the password had already changed).

## [2.0.0] - 2026-06-12

### Changed (Major)

- **Complete Rewrite: Flutter → Tauri v2** — SoloSoul now uses a Rust/Tauri backend with React/TypeScript frontend, providing better performance, smaller binary size (~50MB vs ~200MB), and more native feel. The Flutter codebase in the `flutter/` directory is no longer maintained.
- **Crate Name** — `solosoul-app` → `solo_soul` (Rust snake_case convention).
- **Bundle Identifier** — Updated to `com.solosoul.app`.
- **Package Manager** — Flutter → npm (Node.js ≥ 22, React 19, Vite, TypeScript strict mode).
- **State Management** — Riverpod → Zustand.
- **UI Framework** — Flutter Widgets → React + CSS Modules (no Tailwind).
- **Layout** — Removed custom title bar, search changed to popover, support top/bottom function bars.
- **Theme System** — Full CSS custom properties theme engine with 20+ preset themes, accent color customization, and light/dark modes.

### Added

- **App Icon** — Custom shield icon with embedded S letter rendered via SVG path, not relying on system fonts.
- **Object CRUD** — Complete object workspace with card-based display, property editing, inline add/remove.
- **Template System** — User-defined object templates with 8+ field types (text, number, date, checkbox, select, multi-select, URL, email, phone).
- **Select/Multi-Select Field Options** — Template editor now supports defining options for select-type fields with an options editor overlay.
- **Field Format Hints** — Show format examples (e.g. `https://example.com`, `name@example.com`) below field names in the object editor.
- **URL Validation Relaxation** — Auto-prepend `https://` before validation to reduce user friction.
- **History System** — Object snapshot-based history with rollback, diff summary, and version browser.
- **Trash/Recycle Bin** — Soft-delete, restore, permanent delete, conflict detection on restore, batch operations.
- **Search** — Full-text search across objects with category/tag/type filtering.
- **AI Chat** — Multi-provider LLM support (OpenAI, Anthropic, Ollama, etc.) with streaming chat, conversation management, and AI-powered object creation.
- **Attachment System** — Upload/download/preview (images, PDF, text), rename, soft-delete, restore, permanent delete, vault encryption.
- **OCR Scanning** — Image-based OCR with MRZ parsing for passports/IDs.
- **Local Scan** — Filesystem scanning and indexing for local file search.
- **Template Manager** — Full template CRUD with field options editing and property reordering.
- **Export/Import** — Object export with encryption, tag filtering, attachment inclusion.
- **Password Hint** — Welcome page now includes a password hint input field with clear/undo support.
- **Biometric Unlock** — Touch ID / Face ID support.
- **Privacy Policy & Terms of Service** — Bilingual (EN/ZH) documents with language-based auto-routing.
- **Search Index** — Built-in user guide search with 2685 indexed words across 23 guides.
- **Operation Log** — Structured audit logging with full i18n support, all CRUD actions tracked.
- **Orphaned Attachment Cleanup** — One-click cleanup of unreferenced attachment disk files.
- **Vault Stats Dashboard** — Pie chart visualization of storage breakdown (objects, attachments, metadata).

### Fixed

- **macOS Bootstrap Page Input** — Password fields on first-run wizard no longer blocked by WKWebView password manager. Replaced `<input type="password">` with `type="text"` + CSS `-webkit-text-security: disc` to bypass WebKit keyboard interception on forms with multiple password fields.
- **Windows Language Detection** — Use `GetUserDefaultUILanguage` Win32 API to correctly detect display language (was always showing English on first launch). Final 5-layer detection chain: Rust eval → localStorage → IPC get_system_locale → navigator.language → eval override.
- **Windows Resource Paths** — Fixed `resource_path()` resolution for docs on Windows (was incorrectly using macOS `../Resources` path).
- **Windows User Data Path** — Use `%USERPROFILE%` instead of `$HOME` for data storage.
- **Windows Password Reveal Overlap** — Hidden native WebView2 password reveal button via CSS (`::-ms-reveal`) that overlapped the custom eye icon.
- **Multi-Select Order** — Field values now display in template option order, not selection order.
- **Multi-Select Display** — Fixed multi-select values not showing in cards, detail modals, and history panels.
- **Page Restore** — Restoring a deleted page from trash now immediately refreshes the sidebar via `loadCustomPages()`.
- **Theme Sync** — Selecting a card theme now automatically syncs the light/dark appearance option.
- **Password Validation** — Proper error message for short passwords (was incorrectly showing "incorrect password").
- **Password Error Matching** — Added detection for backend "8 characters" message to show localized `password_too_short`.
- **Validation i18n** — URL/email/phone/date/number validation errors no longer display raw localization keys.
- **Toast Messages** — Improved error toast from redundant "验证失败: 验证失败" to descriptive messages.
- **About Page** — Platform shows OS name only (no architecture); logo uses consistent brand gradient.
- **Privacy Policy & Terms Links** — Fixed 404 links to correct docs path, added bilingual documents.
- **Login Theme Flash** — Inline script + CSS dark color fallback prevents white flash on dark-themed first load.
- **Object Editor Data Pollution** — Switching objects no longer shows stale previous object data.
- **Object Editor Jump Flash** — Loading placeholder prevents blank form flash during data fetch.
- **Attachment Preview CSP** — Added `frame-src 'self' data:` for PDF iframe preview.
- **Attachment Double-Save Prevention** — StopPropagation on preview close to prevent accidental card dismissal.
- **Attachment Preview on Windows** — File path + data URI dual-path fallback for preview across platforms.
- **Vault Stats Field Names** — Fixed camelCase/snake_case mismatch between Rust backend and TypeScript frontend.
- **Orphaned Attachment Stats** — `get_vault_stats` now only counts referenced attachments.
- **Restore Conflict Detection** — Fixed to check same-name + same-page instead of ID existence.
- **Restored Suffix i18n** — Now generated in Rust backend per user's language.
- **Trash Detail Display** — All property keys use i18n, internal fields like `__attachments` filtered out.
- **History Snapshot Layout** — Fixed layout jumps during page transitions, unified badge styling.
- **Diff Summary** — 'Created' now shows i18n "初始版本" instead of raw text.
- **Crate `--bundles app` only** — Avoids generating unnecessary AppImage on macOS.
- **Sidebar Rename** — Double-click to rename custom pages.
- **Theme Persistence** — IPC-driven theme loading before React render prevents theme flicker.
- **FAB → Inline Add Button** — Replaced floating "+" with dashed-border placeholder cards.
- **Attachment Preview** — Data URI + file path dual-path strategy bypasses CSP limitations.
- **Auth Page Redirects** — Fixed closure capture bug using `getState()` pattern.
- **Set Vault Path Index Loading** — Fixed missing `loadIndex()` call on account switch.
- **Account Deletion Lock** — Delete now properly locks and redirects to login.
- **Change Password Flow** — Wrong current password returns proper error instead of hanging.

### Architecture

- **§4 Layered Encryption** — `ui_preferences.json` stored in plaintext; theme/language loaded before React render to prevent flash.
- **§5 Snapshot System** — `object_snapshots` table with auto-save on modify, version browser, rollback.
- **§24 Flat Model** — Removed collection layer; tags replace hierarchical grouping.
- **§25 Section/Type Separation** — `section_type` column separates UI grouping from `type_id` semantics.
- **§26 Multi-Provider LLM** — 5 preset providers + custom endpoint, API key separation, connection test, risk disclosure.
- **§6 Attachment System** — Full spec: create/rename/soft-delete/permanent-delete/preview/restore with dedicated trash tab.

### i18n

- **Complete i18n Foundation** — i18next + react-i18next with en-US/zh-CN locales across all pages (editor, auth, settings, navigation, layout, object workspace, AI chat, etc.).
- **Object field keys** — All template field keys and validation messages localized.
- **Operation log** — Actions, entities, performed-by, conflict status, section names all localized.
- **Trash page** — Time labels, type labels, expiration text, relative times all localized.
- **History** — Version labels, triggers, loading/no-history states all localized.
- **Attachment cards** — All buttons, confirmations, empty states, preview errors localized.
- **Template editor** — Field types, option editor, delete confirmations all localized.
- **User guides** — 23 system guides in both languages.
- **Bootstrap page** — Password hints, validation rules, account names all localized.
- **Settings** — All tabs, descriptions, backup labels, security settings fully localized.
- **Layout** — Sidebar, search popover, app shell, theme selector all localized.
- **About page** — Version info, platform, legal links all localized.

### Chores

- MSI bundle removed from build targets; Windows generates only NSIS (.exe).
- Cleaned up all language detection debug logging, retained production-ready 5-layer detection.
- iOS, Android, tvOS build targets removed from Cargo workspace (Tauri desktop-only).
- Removed `create-dmg` AppleScript dependency documentation (fallback to `hdiutil` fine).

## [1.8.0] - 2026-06-03

### Fixed

- **Windows PDF Preview** — Migrated from `pdfx` to `pdfrx` (^2.4.3), fixing PDF preview on Windows. `pdfx` was incompatible with CMake 4.x due to deprecated `DownloadProject.cmake` syntax.
- **Cross-Platform PDF Engine Unification** — Both macOS and Windows now use the same PDF rendering engine (PDFium via `pdfrx`), ensuring consistent behavior.
- **i18n Hardcoded English** — Replaced 20+ hardcoded English strings in biometric settings with localized ARB keys (`biometricTypeNotAvailable`, `biometricTypeAuthFailed`, etc.).
- **Ollama Exception Localization** — `modelNotFound` errors now display localized messages instead of raw English.
- **Windows CJK Font Fallback** — Added `_cjkFontFallback` (`Microsoft YaHei`, `PingFang SC`, etc.) to `AppTheme`, ensuring Chinese characters render with correct glyphs on Windows instead of falling back to Japanese fonts.
- **Settings Version Icon** — Windows platform icon in version sheet was incorrectly showing `phone_android`; now correctly shows `desktop_windows`.
- **Recent Device Platform Detection** — `getDeviceName()` now appends `(Windows)` / `(Linux)` to hostnames that don't contain platform keywords, so `getDevicePlatformLabel()` can correctly identify them in the recent devices list.

### Added

- **Login Page Account Short ID** — Account list items now display a short ID prefix (e.g. `acc_550e84`) next to the account name, allowing users to distinguish accounts with identical names copied from different machines.
- **Password Input Page Account Short ID** — The account header in the password input section also shows the short ID for quick verification before entering the master password.

### Changed

- **PDF Rendering Service** — Rewrote `PdfRenderService` using `pdfrx` API. Pages are rendered to BGRA then converted to PNG for OCR consumption.
- **Attachment PDF Preview** — Replaced custom `PdfController` + `PdfView` + navigation bar with `pdfrx`'s built-in `PdfViewer.file()` widget, which includes zoom, scroll, and page navigation out of the box.
- **Build Scripts** — Added `dart run pdfrx:remove_wasm_modules` to all build scripts (DMG, Windows ZIP) to strip unused Web WASM assets and reduce bundle size.
- **Windows CMake** — Removed `pdfx` auto-removal hacks from `windows/CMakeLists.txt`; `local_auth_windows` removal is retained.

## [1.7.1] - 2026-06-03

### Added

- **Windows Release Packaging**
  - `build_windows_zip.ps1` / `build_windows_zip.sh` — PowerShell and Bash scripts for Windows ZIP packaging, auto-reading version from `pubspec.yaml`.
  - CI/CD three-stage pipeline — `build-macos` + `build-windows` → unified `release` job uploading both `.dmg` and `.zip` to GitHub Releases.

- **Windows Compilation Hardening**
  - CMake auto-fix removes incompatible `pdfx` and `local_auth_windows` plugins from generated registrants.
  - `Cargo.toml` conditional dependency: `ort` excluded on Windows; OCR module uses stub implementation on Windows.
  - Enforced 900×600 minimum window size on Windows via Win32 `WM_GETMINMAXINFO`.
  - C++ source comments switched to English to prevent VS 2026 C4819 encoding errors under `/WX`.

### Fixed

- **Plugin Routing** — `backRoute` corrected from `'/'` to `AppRoutes.home`; missing `AppRoutes` import added.
- **Backup Manifest** — Ensures sidecar directory exists before writing `manifest.json`.
- **PDF Render on Windows** — `PdfRenderService` now returns `null` safely on Windows instead of crashing.
- **Rust Dependency** — Removed duplicate `rustls-tls` key in `reqwest` features.

### Changed

- **Build Scripts** — `build_dmg.sh`, `build_windows_zip.sh`, and `build_windows_zip.ps1` now auto-read version from `pubspec.yaml`; manual version argument is optional.
- **Operation Log** — Extended coverage to attachment, backup, and plugin lifecycle operations.
- **Localization** — Backup list summary text fully internationalized.
- **Settings UI** — Removed help & guides section (redundant with home quick-actions).

### Docs

- Updated Windows known issues in `docs/TODO.md`.
- Rewrote trash/recycle bin guides (Chinese & English), clarifying distinction between general trash and AI conversation trash.
- Fixed sensitivity level naming in object guides.

### Internal

- `.gitattributes` added to enforce LF for shell scripts.
- Removed redundant AttachmentPool "already exists" debug log.

## [1.7.0] - 2026-06-02

### Added

- **Attachment Storage & Statistics**
  - Data management page now correctly counts and displays attachment size alongside vault data size.
  - `AttachmentStorageService` gained `getTotalAttachmentSize`, `getAttachmentCount`, `getAttachmentFileIds`, and `cleanupPartialFiles` APIs.

- **Attachment Sync Support**
  - Rust `SyncEngine` extended with a dedicated attachment sync phase: manifest exchange → serial chunked transfer → temp file `.solo.part` + checksum rename.
  - `StateVectorRequest/Response` added `supports_attachments` backward-compatible flag.
  - Dart `SyncService` and `SyncPage` adapted to report attachment transfer progress.

- **Backup with Attachments**
  - `BackupService` rewritten to use a sidecar directory `{backupFile}.backup.attachments/` for each backup.
  - `createBackup` copies account attachments to the sidecar; `restoreBackup` restores them; `deleteBackup` cleans up the sidecar.

- **Attachment Reference Pool**
  - New `AttachmentPoolService` manages a global attachment pool (`attachments_pool/`) where each `fileId` is stored only once.
  - Backups share pool files via `manifest.json` references instead of duplicating attachments.
  - Deleting a backup triggers lazy reference counting: scans all remaining manifests and removes unreferenced pool files.
  - Old-format sidecar directories (containing raw `.solo` files) are automatically migrated to the pool + manifest format on restore.
  - Backup list UI now shows "X backups · Attachment Pool Y" with an explanatory subtitle about shared storage.

- **Attachment Management & PPTX Preview**
  - Full attachment lifecycle: upload, download, preview (PDF via `pdfx`, images with zoom/pan), soft-delete, restore, and permanent deletion.
  - PPTX thumbnail extraction for quick preview.

- **Windows Rust FFI Support**
  - Added Windows platform compilation and linking support for the Rust FFI native library.

- **AI Context & Chat Enhancements**
  - AI context now injects installed plugin information for richer responses.
  - Multi-conversation windows, trash/recycle bin for deleted conversations, and message timestamps in LLM chat.

- **User Guides System**
  - New plugin feature guide documents.
  - Home page quick-actions section now includes help & guides entry.
  - Settings page guide items include descriptive subtitles.
  - Notion-style rendering for guide documents via `flutter_markdown_plus`.

- **Test Infrastructure**
  - Added 34 new test files; test coverage increased from 15.9% to 23.5% (+7.6%).
  - New `attachment_pool_service_test.dart` (9 tests) and `backup_service_manifest_test.dart` (9 tests).

### Fixed

- **Code Security & Quality (P001-P011)**
  - Eliminated ~166 force unwraps (`!`) across 34 files.
  - Fixed 3 `use_build_context_synchronously` warnings.
  - Added path injection hardening for `Process.run` calls.
  - Replaced bare `catch (e)` with typed `on Exception catch (e)`.
  - Fixed memory leaks in `llm_chat_session_provider` (`StreamSubscription`/`Timer` cleanup).

- **P014** — Fixed `dart:convert` import in `plugin_card.dart`.
- **P007 Supplement** — Replaced `ValueChanged` with native `Function` types.

### Refactored

- **P007-B** — Extracted UI dependencies from `attachment_upload_service.dart` into the Presentation layer.
- **P007-A** — Decoupled `IconData` dependency from `unified_object_service.dart`.
- **P013** — `solosould` daemon migrated to structured logging (`log/slog`).
- **P015** — Replaced all `map[string]interface{}` in `server.go` with concrete structs.
- **P012** — Batch fixed code quality issues across test files.

### Docs

- Generated final code analysis report (`CODE_ANALYSIS_REPORT_FINAL.md`) — all 15 issues resolved.
- Added deferred items explanation (`CODE_ANALYSIS_DEFERRED_ITEMS.md`).
- Updated code analysis report with final status.

## [1.6.8] - 2026-05-31

### Added

- **Attachment Soft-Delete & Trash** (`dabb7e4`)
  - Added `isDeleted` + `deletedAt` fields to `Attachment` model with full `copyWith`/`toJson`/`fromJson` support.
  - `UnifiedObjectNotifier` gained `softDeleteAttachment`, `restoreAttachment`, and `permanentlyDeleteAttachment` methods.
  - `AttachmentListSheet` UI rebuilt to support 3-state lifecycle: active / soft-deleted (visible in trash tab) / permanently removed.
  - `updateObject` diff logic automatically filters out soft-deleted attachments from the active list while preserving them for restoration.

- **Attachment Download Service** (`dabb7e4`, `cc2c666`)
  - New `AttachmentDownloadService` class encapsulates decryption → file write → platform share sheet flow.
  - Downloads target a configurable directory (default: system `Downloads`). Settings page added `settings_page_download_section.dart` with custom path picker and "Restore Default" button.
  - Filename collision handling: `report.pdf` → `report (1).pdf` → `report (2).pdf` via `_getUniqueFilePath`.
  - Write-permission validation before every download; on macOS sandbox path invalidation, auto-fallback to default Downloads with a SnackBar notification.
  - Debounce guard (`_activeDownloads` `Set<String>`) prevents concurrent duplicate downloads from rapid button taps.

- **Attachment Preview Enhancement** (`dabb7e4`)
  - PDF preview powered by `pdfx` package with page navigation and zoom gestures.
  - Image preview supports full-screen zoom, pan, and tap-to-dismiss via `InteractiveViewer` + `GestureDetector`.
  - Preview dialog close button repositioned to top-right for consistency with macOS window chrome.

- **Inline Add Section Placeholder** (`78b1a4f`)
  - New reusable `AddSectionPlaceholder` widget: dashed border, centered "+ Add Section" text, tap gesture.
  - Replaces the FAB "+" button in `ObjectWorkspacePage` and `ObjectCategoryPage`.
  - Appears at the end of every list (pages, root objects, default category pages), eliminating empty-state dead ends.
  - AppBar "+" icon removed; all add flows now route through the inline placeholder for visual consistency.

- **Attachment Upload Service Extraction** (`cc2c666`)
  - New `AttachmentUploadService` unifies the file-pick → sensitivity-check → encrypt → store flow.
  - Replaces duplicated upload logic in `ObjectCardItemTile` and `EntryCardWidget` (~100 lines removed per widget).
  - Single call site: `AttachmentUploadService.upload(context, objectId, fileName, bytes, sensitivity)`.

### Fixed

- **Account Duplicate-Name Logic Bug** (`d319bd0`) — `SecureAccountStorage.createAccount` now rejects accounts with existing names when `accountId == null`. Previously all existing accounts were treated as "stale" and silently deleted, causing data loss.
- **JSON Serialization Key Mismatch** (`54cf246`) — Added `@JsonKey(name: '__propertyLabels')` and `@JsonKey(name: '__semanticTypes')` to `UnifiedObject` to align `json_serializable`-generated `fromJson()` with hand-written `toMap()`.
- **Orphan File Prevention** (`cc2c666`) — `permanentlyDeleteAttachment` throws `StateError` when `accountId` is null, preventing encrypted file orphans on disk.
- **macOS Sandbox Download Fallback** (`cc2c666`) — When the user-configured download directory becomes unreachable (sandbox token expiration after restart), the service automatically falls back to `~/Downloads` instead of crashing.
- **TypeId Migration Test Sync** (`9c40ee9`) — Updated 6 test files to match the `profile_*` → `__preset_*` typeId migration, fixing 14 pre-existing failures.

### Refactored

- **Plugin Event Handler Decomposition** (`b919a84`) — Split `_onRun()` (285 lines, 7-level nesting) in `plugin_dashboard_page.dart` into `_PluginRunSession` state class + 6 handler methods (`_handleDialogConsent`, `_handleFieldConsent`, `_handlePluginResult`, `_handlePluginLog`, `_handlePluginCompleted`, `_handlePluginError`) + `_showRunResult`.
- **Object Editor Validation Extraction** (`b919a84`) — Split `_saveObject()` into `_validateSaveInput()` (early-return validation) and `_buildProperties()` + `_PropertyBuildResult` (property map construction).
- **Password Dialog Deduplication** (`e0fc878`) — Extracted `_PasswordDialogBaseMixin` containing shared `TextEditingController`, `FocusNode`, error state, `initState`/`dispose`, and focus/text-change handlers. Two dialog variants now only implement `_verify()` and `build()`.
- **OCR Processing Pipeline** (`0b0f0ca`) — Extracted `_processOcrBytes()` unifying OCR → MRZ → field extraction. `_pickImage` and `_pickDocument` now only acquire bytes and delegate to the shared pipeline. ~80 lines of duplicate code removed.
- **Default Structure Factory** (`e5e12e7`) — Extracted `_buildPage()` / `_buildSection()` factory methods in `unified_object_notifier.dart` for reuse between `_createDefaultStructure` and `_migrateDefaultSectionSchemas`.
- **LLM Config Deduplication** (`cfb668e`) — Extracted `_activeProfile()` helper in `llm_config_service`, eliminating repeated `ref.read(currentProfileProvider)` lookups across 5 getters.
- **Icon Lookup Optimization** (`cfb668e`) — Replaced 114-line `switch` in `unified_object_service.getIconFromName` with a `Map<String, String>` constant table. ~70 lines removed.
- **Parallel I/O** (`b75808e`) — `updateObject`, `permanentlyDeleteObject`, and `permanentlyDeleteMultiple` now delete attachments in parallel via `Future.wait` instead of sequential `await`.
- **O(n) → O(1) Lookups** (`b75808e`) — `semantic_type_registry.getType` pre-builds a `Map<String, ObjectTypeDefinition>`; `recommend()` uses a `Set<String>` for constant-time contains checks.
- **Dead Code & Proxy Removal** (`b75808e`) — Removed unused `locale` variable in `plugin_dashboard_page`, unreachable `case 'map'` branch, and proxy methods `_logSectionForTypeId` / `_typeColor` (inlined at call sites).
- **Context Safety** (`b75808e`) — Fixed 3 `use_build_context_synchronously` info warnings in `plugin_dashboard_page.dart` by adding `if (!context.mounted) break;` at the top of the `await for` loop.

### Test Infrastructure

- **Injectable FFI Wrappers** (`d319bd0`) — Added `_saltGenerator` / `_keyDeriver` function pointers to `SecureAccountStorage` with `setFfiWrappersForTest()` API. All direct `frb.frbGenerateSalt` / `frb.frbDeriveKey` calls replaced with injectable wrappers.
- **Full Test Suite Green** — All 902 unit tests pass (0 failures). Previously 16 tests failed due to stale `typeId` formats and missing Rust FFI initialization.

## [1.6.7] - 2026-05-31

### Fixed

- **Force Unwrap Elimination** — Eliminated ~166 `!` force unwraps across 34 files. Replaced with local variable null-checks, pattern matching (`case final x?`), and `whereType<T>()` to prevent runtime crashes.
- **Context Safety** — Fixed 3 `use_build_context_synchronously` warnings in `plugin_dashboard_page.dart` by adding `mounted` checks before using `BuildContext` after async gaps.
- **Path Injection Hardening** — Added `_isSafePath()` validation to `Process.run` calls in `content_parser_service.dart` to prevent command injection via malicious file paths.
- **Typed Exception Catching** — Replaced 4 bare `catch (e)` clauses with `on Exception catch (e)` to avoid swallowing `Error` subclasses.
- **Memory Leak Fix** — Added `onDispose` cleanup for `StreamSubscription` and `Timer` in `llm_chat_session_provider.dart` to prevent leaks on provider disposal.

### Refactored

- **RadioGroup Migration** — Migrated deprecated `RadioListTile.groupValue` / `onChanged` to `RadioGroup` ancestor pattern in `plugin_radio_list_dialog.dart` and `plugin_sensitivity_override_dialog.dart`. Eliminates all 4 dart analyzer info warnings.
- **Widget Extraction** — Split 4 oversized build methods (>200 lines each) into focused sub-methods:
  - `_onRun()` (307 lines) → `_prepareInitialParams()` + `_showExecutionResult()`
  - `app_sidebar build()` (237 lines) → `_buildSidebarContent` / `_buildPagesSection` / `_buildBottomActions` / `_buildResizeHandle`
  - `_PluginResultDialog build()` (224 lines) → `_buildTitle` / `_buildLogsSection` / `_buildResultsSection` / `_buildErrorBanner`
  - `trash_card build()` (214 lines) → `_buildHeader` / `_buildActionBar`

### Code Quality

- **Static Analysis Zero-Warnings** — `dart analyze` now reports 0 errors, 0 warnings, 0 info across the entire codebase.
- **Dead Code Removal** — Removed unused imports, variables, and parameters across 5 files.

## [1.6.6] - 2026-05-25

### Added

- **Sidebar Plugin Entry** — Added Plugins shortcut to the app sidebar above the Lock Vault button for quick access to the plugin dashboard.

### Fixed

- **Identity Section Title Bug** — Fixed all identity-related sections (Passport, Travel, Education, Employment, Skill, Language, Article) showing empty or incorrect titles. `ObjectTypeRegistry` now uses `Title` as the title key for all predefined types. `ObjectCard._template` deduplicates redundant `Title` fields when a custom title key exists, and `ObjectEditorPage` correctly recognizes type-specific title keys.
- **Missing Title in Predefined Sections** — `DynamicSectionCard` now falls back to `ObjectTypeRegistry.buildPropertiesFromType()` when a section's properties lack a `Title`. All 8 predefined types include `Title` in their schema.
- **Title Sensitivity Tag** — `ObjectCardEditField` now displays the `SensitivityTag` (e.g., "公开") for title fields, not just regular fields.
- **Skill/Language Schema** — Removed redundant `name` field from `professional_skill` and `professional_language` templates; now use `Title` + `level`/`proficiency` only.
- **HardwareKeyboard Error Spam** — `main.dart` now silently filters out `HardwareKeyboard` assertion errors on macOS, preventing debug console noise.
- **Address Formatter Error Messages** — Removed `[address-fmt]` prefix from user-facing error logs in the address formatting plugin.

## [1.6.5] - 2026-05-25

### Added

- **Plugin Runtime v2** — Semantic type system, access review dialog, and address formatter plugin. Plugins now declare semantic types (e.g., `pet.name`) and request field-level consent before execution.
- **Plugin Market Expansion** — 20 plugins available via GitHub-as-market with multi-source support and CDN fallback:
  - TOTP Generator, Emergency Card, Address Formatter, MRZ Encoder
  - Expiry Guardian, ID Validator, Phone Formatter, Contact Exporter
  - Packing List, Calendar Events, Digital Will, Identity Timeline
  - Form Prefiller, Tax Profile, Data Completeness, Namecard Generator
  - Doc Checklist, Travel Footprint, Resume Builder
- **Plugin Home Quick Action** — Added Plugins shortcut to home page quick actions grid.
- **Schema propertyLabels** — Storage key, display label, and semantic type are now three independent layers. `propertyLabels` map allows editable field display names without changing storage keys.
- **Drag-to-Reorder Fields** — Section editor supports drag-to-reorder property fields via `ReorderableListView`. Order persists via `propertyOrder` on `UnifiedObject`.
- **Parent Page Selector** — Section editor now includes a "所属页面" dropdown to move sections between pages. Sections are prepended to the front of the new page's children.
- **Xcode Auto-Build Rust** — Debug builds now auto-compile `cargo build --features sandbox` when the native library is missing, eliminating manual Rust build steps.

### Fixed

- **Section Move-Delete Cascade** — `updateObject` no longer silently overwrites `parentId` with `null` when `parentId` is not explicitly passed. Fixed by introducing `UnifiedObject.kNullSentinel` in `_service.updateObject` and `notifier.updateObject`. Also fixed `restoreObject` unable to clear `deletedAt`.
- **Account Delete-Recreate Conflict** — `SecureAccountStorage.deleteAccount` now verifies removal and retries on stale data. `createAccount` auto-cleans stale Keychain records before creation. `AccountManager.deleteAccount` no longer silently ignores Keychain failures.
- **ObjectCard UI Polish Cycle** — Collapse threshold reduced from 3 to 1 items; persistent full-width "add item" button at bottom; edit/delete buttons grouped next to section name with add button pushed to far right.
- **Title Key Normalization** — All built-in type definitions now use `id: 'Title'` (capitalized). Legacy `'title'` is normalized at load time and in card templates to prevent duplication.
- **Title Order in Items** — Title is now included in `propertyOrder` so it stays at the top of item cards instead of falling to the bottom.
- **Empty Key Save Fallback** — New fields with empty `key` now use `displayLabel` as the fallback property key on save.
- **Editor Drag Handle** — `ReorderableDragStartListener` now uses the ReorderableListView's local index instead of the global `_propertyFields` index, fixing "non-visible item" exceptions.
- **Localized Field Display** — Restored localized display labels in the object editor schema view.
- **Registry Offline Fallback** — Bundled `registry.json` ships with the app so plugin market works on first launch without internet.
- **P001–P018 Code Quality** — Comprehensive static-analysis cleanup: replaced deprecated `withOpacity` with `withValues()`, added `on Exception` to catch clauses, removed dead code, extracted oversized widgets, fixed unawaited futures, and resolved 160+ analyzer warnings across 41 files.

### Refactored

- **Widget Extraction** — Extracted `_buildAccountTile` and glass settings methods from `app_sidebar` to reduce nesting and file size.
- **Git Cleanup** — Removed 33.7MB prebuilt `libsolosoul_core.a` from git tracking; updated `.gitignore` for Flutter ephemeral artifacts.

### Tests

- Added `PluginManager`, MRZ parser, and Vault path tests.
- Fixed failing tests from second wave test coverage push.

## [1.6.4] - 2026-05-23

### Added

- **Icon Library Expansion to 96 Icons** — `icon_picker_sheet.dart` now supports 96 icons across 12 categories (travel, finance, identity, education, technology, health, media, objects, nature, symbols, arrows, misc). Added search filtering, categorized grid display, and collapsible filter bar.
- **Default Page Custom Sections** — Profile/Travel/Financial/Professional pages now support adding custom sections via "+" button alongside the sensitivity mode button. Reuses `AddSectionDialog` and `custom_sections_widget.dart`. New sections appear at the bottom and support rename, delete, and property editing.
- **Article Section Template** — New predefined template for articles and notes with Title, Author, Source, URL, and Content fields.
- **Password Hint-Only Changes** — `change_password_dialog.dart` now supports updating only the password hint without changing the master password. Added `updatePasswordHintOnly` flow in `rust_vault_service.dart`.
- **Global Backoff Protection** — Password verification dialog now enforces a 30-second global cooldown after 5 failed attempts. Backoff state is persisted via `SharedPreferences` and survives dialog close/reopen across the entire app.
- **Unified Password Verification Dialog** — Extracted shared `password_verification_dialog.dart` with `showPasswordVerificationDialog()` API. Replaced duplicated password dialogs on search page, settings page, and all protected pages.

### Fixed

- **macOS Hot Restart Compilation Error** — `packages/liquid_glass_widgets/lib/src/renderer/shaders.dart` had `const String _shadersRoot = !kIsWeb && isTestEnvironment ? ...` which failed on IO builds because `isTestEnvironment` is `final` (not `const`) in `_env_io.dart`. Changed `_shadersRoot` and `ShaderKeys` fields to `final` with `ignore: prefer_const_declarations` to prevent analyzer false-positives.
- **Default Page Alignment (Phase 3)** — Custom sections now visually align with predefined sections on Profile/Travel/Financial/Professional pages. Removed hardcoded padding differences.
- **Old Account Auto-Migration** — Accounts created before schema v2 now automatically get missing default pages and sections (Identity, Contact, Address, ID Card, Passport, Visa, etc.) on next unlock.
- **macOS Sandbox Data Isolation** — Fixed `path_provider` data directory resolution for sandboxed release builds. Data now correctly stores in `~/Library/Containers/...` when sandbox is enabled.
- **URL Property Type** — `UrlProperty` now has proper validation regex and displays clickable links in `ObjectCard`.
- **Sidebar Alignment** — Fixed vertical alignment of sidebar items with icons of varying widths. Added `IntrinsicWidth` wrapper for consistent label positioning.
- **Filter Bar Collapse** — Filter sections on operation log, search, and trash pages now properly collapse/expand without layout jumps.
- **Delete Account Flow** — Improved confirmation dialog with warning text and 3-second delay before allowing deletion.
- **New Account Default Pages** — Fixed missing default pages when creating a brand new account after app reinstall.

### Refactored

- **Code Quality Audit (Round 1)** — Comprehensive static analysis and automated fix cycle:
  - Fixed 4 P0 test compilation errors: `llm_query_enhancer_test.dart` (dead file removed), `local_search_service_test.dart` (wrong class reference), `property_value_utils_test.dart` (missing import), `sensitivity_tag_test.dart` (removed `getSensitivityLabel` restored)
  - Fixed P1 warnings: unused variables/imports in `scan_import_service.dart`, `sensitivity_settings_page.dart`, `predefined_object_section.dart`
  - Fixed P1 potential bugs: `use_build_context_synchronously` in `llm_config_page.dart`, `unawaited_futures` in `account_style_provider.dart`
  - Fixed P1 deprecated API usages: `dangling_library_doc_comments` in `mrz_date_utils.dart`, missing `fake_async` dependency
  - Bulk P2 fixes via `dart fix --apply`: 160 fixes across 41 files (`prefer_const_constructors`, `prefer_const_declarations`, `no_leading_underscores_for_local_identifiers`, `unnecessary_import`, `unnecessary_to_list_in_spreads`, etc.)
  - Fixed `build_dmg.sh` entitlements path: `Runner/Release.entitlements` → `macos/Runner/Release.entitlements`
- **Filter Sections Unification** — Extracted shared `FilterSection` widget pattern across operation log, search, and trash pages. Eliminated duplicated filter logic.

### Internal

- Added `currentObjects` public getter to `UnifiedObjectNotifier` to avoid external access to protected `state` property.
- Restored `getSensitivityLabel(SensitivityLevel)` top-level helper in `sensitivity_tag.dart` for test compatibility.
- Generated `CODE_ANALYSIS_REPORT.md` and `CODE_ANALYSIS_REPORT_FINAL.md` documenting 20 identified issues and 17 resolutions.

## [1.6.3] - 2026-05-10

### Added

- **Custom Sections on Default Pages** — Added "+" button to each default page's right side (alongside sensitivity mode button), reusing `AddSectionDialog` to add custom sections to Profile/Travel/Financial/Professional pages. New sections appear at the bottom of the page and support all standard section operations (rename, delete, add/edit properties).
  - `custom_sections_widget.dart`: New shared widget wrapping `ObjectCard` list with add/edit/delete controls
  - `add_section_dialog.dart`: New dialog for naming and creating a custom section on any page
  - Updated all 4 default pages: `profile_page.dart`, `travel_page.dart`, `financial_page.dart`, `professional_page.dart`
- **OCR Scan: Save Original File** — `saveOriginalFile` checkbox in scan document sheet allows saving the scanned image as an encrypted attachment via `saveAttachment()`. Attachment is encrypted with vault key and stored in `UnifiedObject.attachments` map.
  - `scan_document_button.dart`: Added checkbox for save-original-file with translated label
  - `object_card_fields_sheet.dart`: Attachment UI displays saved filename with open/open-location actions
  - `unified_object_model.dart`: Added `attachments` field (`Map<String, String>`) to `UnifiedObject`
  - `base_models.dart`: Added `attachments` persistence field to `UnifiedObjectData`
  - `operation_logger.dart`: Added `logCustomSection()` for property-level audit logging
- **MRZ Scan Section Selector** — When importing from MRZ scan, user can override the default section via dropdown menu in the preview dialog. Validated against existing page sections and dynamically created sections.
  - `mrz_preview_card.dart`: Section selector dropdown + validation
  - `predefined_object_section_helpers.dart`: Added `findSectionByName` + `suggestSectionForType` for smart section routing
  - `entry_card_widget.dart`: Added `currentPageSections` parameter support
  - `travel_page.dart`: Passes page sections to MRZ preview
- **Operation Log i18n** — All action labels, time labels, device labels, and section names in operation log page now localized:
  - Filter: `'Action:'` → `'${l10n.operationLabelAction}:'`, `'Device:'` → `'${l10n.operationLabelDevice}:'`
  - Filter chips: `'macOS'` → `l10n.operationPlatformMacos`, `'iOS'` → `l10n.operationPlatformIos`
  - Tile badges: `_actionLabel` (hardcoded 'Created'/'Updated'/'Deleted'/'Restored'/'Purged') → `_actionLabel(l10n)` using `l10n.operationAction*`
  - Time labels: `_formatTime` (hardcoded 'Just now'/'Xm ago'/'Xh ago'/'Xd ago') → reuses `l10n.trashJustNow`/`trashMinutesAgo`/`trashHoursAgo`/`trashDaysAgo`
  - Device tags: `_getDeviceLabel` (hardcoded 'macOS'/'iOS'/'Android'/'Windows'/'Linux'/'Web') → `l10n.operationPlatform*`
  - Section display: `entry.section.toUpperCase()` → `_sectionLabel(l10n)` mapping via `logSection*` l10n keys
  - Detail dialog: All labels (`_actionLabel`, `_sectionLabel`, `_getDeviceLabel`) use l10n
- **New ARB Keys**: Added 23 new keys to both `app_en.arb` and `app_zh.arb`:
  - `operationPlatformMacos`, `operationPlatformIos`, `operationPlatformWindows`, `operationPlatformLinux`
  - `logSectionIdentity` through `logSectionCustom` (19 section labels)

### Fixed

- **MRZ Visa Routing + Double-Save Guard** — `import_result_page.dart` now correctly routes visa items to the travel page section instead of creating orphan objects. Added prevention for multiple MRZ scans of the same document path creating duplicate entries.
- **Section Deletion Notification** — Fixed snackbar disappearing after section soft-delete. Root cause: ObjectCard gets removed from widget tree after `deleteObject` updates provider state, making `context.mounted == false`. Fix: capture `Overlay.of(context)` before any `await`, pass via `forOverlay` parameter to `showOverlaySnackBar`, and add `BuildContext?` / `OverlayState?` dual-path API in `app_theme.dart`.
- **Trash Children Sorting** — `deletedChildrenProvider` now sorts by `deletedAt` descending (matching `trashRootDeletedObjectsProvider` pattern), ensuring newest deletions appear first.
- **Custom Section Title i18n** — Added `'Title'` case to `translateFieldLabel` switch in `format_field_label.dart` (case-sensitive: custom sections use capitalized `'Title'` key).
- **Trash Detail Dialog Empty Properties** — Removed `l10n.commonEmpty` text for empty property values in `unified_object_trash_card.dart`; labels and sensitivity tags remain visible.
- **ObjectCardPropertiesList i18n** — Changed from `formatFieldLabel(key)` (algorithmic Title Case) to `translateFieldLabel(key, l10n)` (i18n-aware) with proper imports.

## [1.6.2] - 2026-05-10

### Added

- **Section Template Browser** — 15 predefined section templates (passport, visa, bank account China/UK/US, ID card, driver's license, contact, education, employment, skills, languages, awards, identity, task) with localized names/descriptions
- **Trash Empty Placeholder** — Trash card shows "Title: (empty)" in gray when name is blank; detail dialog shows "(empty)" in gray italic for blank property values
- **Operation Log Windows/Linux Filter** — Added Windows and Linux device platform filter chips
- **I18n for Section Templates** — All 15 template names, descriptions, and field keys fully localized in English and Chinese
- **Field Key Translation** — Added snake_case variants for all translated field keys (full_name, given_name, date_of_birth, etc.) to support section template field display
- **Debug Logger Buffering** — Logger now always records sanitized entries to a circular buffer from app start; activation prints buffered logs for pre-bug capture

### Fixed

- **Per-Section Schema Independence** — Items now read schema properties from their parent section instead of a shared `ObjectTypeDefinition`, preventing cross-section property leakage. Each new section starts with only `Title`. New sections no longer inherit properties from previously-edited sections
- **Section Editor Deprecated Toggle Removed** — Deleting a property in a section editor truly removes it (no deprecated toggle); deprecated toggle only appears in the item editor for properties removed from parent schema
- **Section Delete/Re-create** — Removing then re-creating a property in a section no longer triggers "duplicate property" error
- **Settings Page Account Count i18n** — Hardcoded English replaced with `l10n.settingsAccountCount`
- **Settings Page Data Management i18n** — Backup summaries, special backup labels localized
- **Settings Page Current Account i18n** — Operation descriptions, device names, platform labels fully localized
- **Settings Page Version Display** — Platform now shows actual OS name (macOS/Windows/Android/iOS) instead of "Unknown"
- **Device Name Resolution** — Now uses actual system hostname (`Platform.localHostname`) so multiple Macs are distinguishable; adds platform label prefix ([macOS], [iOS], etc.) to device names
- **Template Field i18n** — ObjectCard add-item form shows localized field labels instead of raw camelCase keys
- **Redundant Title Fields Removed** — Removed duplicate Title property field in section editors
- **Duplicate ARB Keys** — Removed duplicate ARB keys causing gen-l10n warnings
- **Redundant Label Above Field Input** — Removed duplicate localized label above template field input in item editor
- **Privacy Mode Timeout** — Fixed privacy mode not timing out correctly
- **Sidebar Child Page Drag** — Fixed drag-and-drop glitch when reordering sidebar child pages
- **Verbose Metadata Update Logs** — Removed verbose debug logs from metadata update operations

### Changed

- **Debug Logger Always Recording** — Logger switched from gated (only records when activated) to always-recording with circular buffer; activation prints buffered entries live
- **liquid_glass_widgets Vendored** — Moved from pub.dev dependency to local `packages/liquid_glass_widgets` path dependency; removed verbose init logs

## [1.6.1] - 2026-05-09

### Added

- **Trash Filter Section** — Collapsible filter panel with time-based filters (10 days, 1 day, 6 hours, 1 hour) and type filters (Page, Section, Item)
- **Trash Hierarchy Expansion** — Pages show "Show sections", sections show "Show items" with nested expand/collapse
- **Trash Card Color Coding** — Icon backgrounds colored by type (blue=page, green=section, orange=item)

### Fixed

- **Delete Dialog i18n** — Hardcoded English replaced with localized strings in delete confirmation dialogs
- **Delete Snackbars** — Changed from `ScaffoldMessenger` to `showOverlaySnackBar` for consistent styling
- **MediaQuery Error** — Fixed null check error in `showOverlaySnackBar` when called from dialog context
- **Item Type Filter** — Fixed "Item" filter not matching predefined section items (travel_passport, profile_identity, etc.)
- **Trash Item Labels** — Removed hardcoded "collection" text from child rows, proper type labels shown
- **Double "前" Bug** — Fixed Chinese localization "在 X 前前删除" to "X 前删除"

### Changes

- **Sub-page Creation Disabled** — Add button now directly shows "Add section" dialog, bypassing sub-page creation to avoid bugs
- **Parent Dropdown Hidden** — Parent selector removed from page editor to prevent accidental parent changes

## [1.5.1] - 2026-05-08

### Fixed

- **Password Hint Persistence** — Account creation now saves hint to Rust vault via `updatePasswordHint` in both normal and fallback paths. Previously the hint was only in Keychain; if Keychain was unavailable, the hint was lost on re-login.
- **Display Card Field Labels** — Added `toFormattedStringLocalized(l10n)` to `FormattableEntry` mixin using `translateFieldLabel`. Updated all 13 `formatAllFields` callbacks across travel, profile, financial, and professional pages.
- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to DMG signing entitlements (v1.5.0).

## [1.5.0] - 2026-05-08

### Fixed

- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to `build_dmg.sh` entitlements template. These were missing, preventing `file_picker` and `image_picker` from working in release DMG builds.

## [1.4.9] - 2026-05-08

### Fixed

- **LLM Config Page** — Fixed redirecting to home page in release mode (removed from `debugOnlyRoutes` guard)
- **Local Search & Scan Routes** — Opened to production builds

## [1.4.8] - 2026-05-08

### Added

- **Auto Language Detection** — OS locale auto-detection on first launch (Chinese OS → zh, all others → en)
- **Version Auto-Injection** — Version auto-injected into DMG builds, with update notification linking to GitHub Releases
- **935 ARB Keys** — Comprehensive i18n completion across all pages and widgets with 0 hardcoded strings remaining

### Fixed

- Date picker localization, password dialog width consistency, search empty state colors, untranslated "Vault" references

## [1.4.7] - 2026-05-06

### Added

- **Liquid Glass UI Overhaul** — Complete cross-platform UI redesign using liquid glass material design. All 20+ protected pages, AppBars, cards, buttons, dialogs, and sidebar now use glass-morphism effects with Notion+Anytype bright color palette
- **Login UI Refresh** — Redesigned login page with gradient background, decorative orbs, vertical centering, and hover effects on all interactive elements
- **Back Navigation** — SoloGlassAppBar now supports `backRoute` for proper back button behavior on deep-linked pages

### Fixed

- **Sensitivity Lock Enforcement** — Locking sensitive access now simultaneously enforces data masking and collapses all expanded history records
- **Sidebar Rename Bug** — Editing a custom page name no longer persists when navigating away; double-tap renamed to long-press for faster click response
- **Sidebar Drag Performance** — Cached descendant lookups during drag-and-drop and simplified drag placeholder to reduce jank
- **LLM Stats Persistence** — Skips LLM usage statistics persistence when vault is locked to prevent errors
- **Object Editor Character Counter** — Fixed character counter showing literal text instead of actual number
- **History Timestamp Alignment** — Full timestamps in history records are now right-aligned for consistency

### Refactored

- **Sensitivity Model Consolidation** — `sensitivity_models.dart` moved to `core/models/` for cleaner architecture
- **Scan Service Refactoring** — `local_search_service` now uses `FieldRegistry` as the single source of truth for field sensitivity levels

## [1.4.6] - 2026-05-06

### Added

- **LLM Integration (P0)** — Full AI chat interface with streaming responses, smart field mapping for object creation, and encrypted usage statistics persistence. Supports multiple providers with per-model configuration, usage tracking, and sparkline chart visualization in settings
- **Local Search Import (P0)** — Import objects from local search results with automatic schema field mapping, batch validation, and CancelToken support to interrupt underlying scan I/O
- **Multi-Device Sync** — FRB-based sync engine featuring CRDT data structures for conflict-free replicated data types, Noise protocol encryption for handshake security, and TCP transport layer. Includes dedicated sync UI for device pairing and connection management
- **Rust Core Engine (Phases 1-5)** — Complete migration from Dart crypto fallback to unified Rust implementation:
  - Phase 1: Unified encryption layer with Argon2id + AES-256-GCM
  - Phase 2: Unified account management with UUID account IDs
  - Phase 3: Eliminated Dart fallback, added KdfPreset configuration
  - Phase 4: Typed FRB bindings replacing JSON relay pattern
  - Phase 5: CRDT sync engine with Noise encryption and TCP transport
- **Anytype-Inspired macOS Features** — Redesigned object workspace and editor interactions following Anytype UX patterns for spatial navigation and relation editing
- **Full Operation Recording** — All user CRUD actions are logged via `OperationLogService` with before/after property snapshots, sensitivity levels, and proper `logSectionForTypeId` mapping for complete audit trails
- **Comprehensive Test Coverage (Phases 1-11)** — Added extensive unit tests for LLM service, local import, auth notifiers, vault operations, sync engine, and widget behavior
- **Structured Sensitive Debug Logging (P028)** — `DebugLogger` now tags sensitive data with structured sensitivity levels for safer diagnostic output
- **Sync UI** — New settings section for managing multi-device synchronization with real-time connection status

### Fixed

- **AI Privacy Protection (P001)** — Smart mapping now blocks critical and restricted sensitivity data from being transmitted to cloud LLM APIs, preventing privacy leakage
- **Startup Black Screen (P0)** — Fixed native library loading race condition caused by incorrect `dlopen` path resolution on macOS app launch
- **Unlock Flow Hangs** — Restructured async unlock sequence to prevent UI hangs caused by `verify_hash` encoding mismatches between old and new account formats. Added automatic `verify_hash` repair for corrupted Keychain entries
- **Account Switch Security** — Password verification is now mandatory when switching accounts from settings, preventing unauthorized access via cached session tokens
- **Password Dialog Error Icon Color** — When verifying identity for critical/sensitive fields, entering an incorrect password now correctly turns the hint (`help_outline`) and visibility toggle (`visibility_outlined`/`visibility_off_outlined`) icons red to match the error text, instead of leaving them white/default
- **Security Audit (S001-S015)** — Path traversal validation on profile IDs, minimum PBKDF2 iteration enforcement, secure key material wipe after account creation, backup file permissions (0600), debug mode security hardening, and constant-time comparison migrated to Rust FFI
- **Performance Audit (PF001-PF010)** — Batch delete for empty trash, O(n²) elimination in list operations, save/log debounce, TextEditingController leak fixes, and timer no-op prevention
- **Code Quality Audit (D001-D011, P001-P055)** — Dead code cleanup, concrete exception types in catch clauses, mounted guards, duplicate code extraction, and removed empty setState calls
- **LLM Stability (P002-P007, P016)** — Proper `http.Client` disposal, input field clearing, stream controller leak fixes, print-to-SoloLog migration, and max 5-file limit for AI mapping to prevent request storms
- **LLM Type Safety (P004, P008-P011, P015)** — Debounced stream rebuilds, type-safe API key handling, proper comment alignment, and API key clearing on model switch
- **Import Integrity** — Ensures imported objects contain all schema-defined fields with correct defaults; prevents stats loss when Vault is locked at startup

### Refactored

- **Widget Extraction (P010-P015, P023-P025, P034-P039, P043, P057-P059)** — Extracted 26+ widget classes from 8 oversized files:
  - Settings page: 427 → 35 lines
  - Profile page: 327 → ~50 lines
  - Editor header, bottom save bar, property field rows, contact forms as standalone widgets
- **LLM Service Modularization (P020)** — Split monolithic `llm_service.dart` into focused files: `llm_config_service.dart`, `llm_chat_service.dart`, `llm_mapping_service.dart`, `llm_stats_service.dart`
- **Shared Utilities (Phase 0)** — Extracted `_verifyPassword`, `_postLoginSetup`, dialog overlay helpers, and page templates to eliminate duplication
- **Login Flow (P045)** — Unified `_handleUnlock` and `_handleCreateAccount` post-login setup into shared `_postLoginSetup`
- **Settings Dialogs (P031-P032)** — Extracted `_DebugActivationDialog` reducing dialog builder from 196 → 58 lines

## [1.4.5] - 2026-04-30

### Added

- **Operation Log Search** — Added search bar to operation log page filtering by description, section, and action; shows live result count and supports clear action
- **Trash Property Snapshot** — Purge actions in trash page now capture full property values and sensitivity levels via `OperationLogger.logCustomSection()` for complete audit trail
- **Object Card Title Key Config** — `ObjectCard` now accepts `titlePropertyKey` parameter (default `'Title'`) to support schemas using different title fields (e.g., `fullName` in Identity). Title input controller initialization and save logic now reference this configurable key instead of hard-coded `'Title'`

### Fixed

- **Label Formatting Consistency** — Extracted shared `formatFieldLabel()` utility in `presentation/utils/format_field_label.dart`; applied to `FieldHistoryView` and `OperationTile` so history records and operation log property snapshots display human-readable labels like "Given Name" instead of raw camelCase keys like "givenName", matching the display card formatting

## [1.4.4] - 2026-04-30

### Fixed

- **Trash Purge Snackbar Silent Failure** — `_confirmPurgeUnifiedObject()` previously accepted a `BuildContext` parameter from `ListView.itemBuilder`, which becomes unmounted after `permanentlyDeleteObject()` removes the item from the list. `showOverlaySnackBar()` checks `context.mounted` and silently returns if false. Removed the `BuildContext` parameter from both `_confirmPurgeUnifiedObject()` and `_confirmRestoreUnifiedObject()`; methods now use `_TrashPageState`'s stable `this.context`. Also removed the ineffective `WidgetsBinding.instance.addPostFrameCallback` workaround
- **Trash Card Action Overflow** — Reduced button padding from `EdgeInsets.symmetric(horizontal: 12)` to `6` and wrapped timestamp text in `Flexible` to prevent 13px overflow on medium-width screens
- **Trash Button Alignment** — Replaced `Flexible + Spacer` with `Expanded` so action buttons occupy the full card width consistently
- **Trash Responsive Actions** — Added `LayoutBuilder` with 420px threshold: narrow screens show icon-only buttons with tooltips; wide screens show labeled text buttons
- **Trash History Button State** — Empty history now shows gray icon with "0" count and tap-to-show "No history available" tooltip; non-empty history shows purple icon with count badge
- **Trash Detail/History Dialogs** — Added Details dialog showing fields + sensitivity tags + deletion time; added History dialog reusing `FieldHistoryDialog` with proper field prefix mapping
- **Trash Operation Logging** — Purge, restore, and empty-trash actions now write to `OperationLogService` via `_logSectionForTypeId()` mapping
- **Trash "Untitled" Display** — `PredefinedObjectSection` name resolution now includes `fullName` key in the lookup list
- **flutter_animate Crash** — Removed `.animate().fadeIn()` from `_UnifiedObjectTrashCard` which caused `FractionalTranslation` hit-test assertion during widget removal
- **Operation Log Sensitivity Colors** — Filter chips, `OperationTile`, and detail dialog now use `SensitivityTag` colors: Critical=red.shade900, Internal=blue, Public=green, Sensitive=orange
- **Identity Operation Logging** — `PredefinedObjectSection.onSave()` and `onDidDelete()` now log create/update/delete actions; undo restore also logs
- **Object Editor Sensitivity Dropdown** — `PopupMenuButton` child now shows `Row(SensitivityTag + Icon(Icons.keyboard_arrow_down))` for clearer affordance

## [1.4.3] - 2026-04-29

### Fixed

- **Vault Initialization Race (Android/Windows)** — `NativeVaultService._initialize()` now stores the async init future in `_initFuture`; all Android/Windows public async methods (`createAccountAsync`, `unlockVaultAsync`, `unlockVaultWithKeyAsync`, `deleteAccountAsync`, `listAccountsAsync`, `getAccountConfigAsync`) await `_ensureInitialized()` before accessing `_fallbackSecureStorage` or `_profilesDir`. Sync fallback `_androidRequest()` now returns `{'success': false, 'error': 'Vault not initialized'}` if called before initialization completes, preventing null-dereference crashes
- **Property Editor Controller Leaks** — `_TextEditor`, `_NumberEditor`, `_RelationEditor`, `_UrlEditor` in `property_editor_factory.dart` were `StatelessWidget`s creating `TextEditingController` on every `build()` without disposal. Converted all four to `StatefulWidget` with `dispose()` calling `controller.dispose()`
- **Object Editor Fire-and-Forget** — `_saveObject()` in `object_editor_page.dart` was `void _saveObject() async`, meaning exceptions from internal `await`s were silently swallowed. Changed to `Future<void> _saveObject() async` with full `try/catch`, logging errors via `DebugLogger` and showing `SnackBar` to user on failure
- **Mounted Checks** — Added `if (mounted)` guards after all `await showModalBottomSheet` / `await showDialog` / `await saveObject()` calls in `object_editor_page.dart` to prevent `setState()` on disposed widgets
- **Search Result Tile Rebuild Logic** — `SearchResultTile` previously watched `accountStyleProvider.select((s) => s.value?.displayMode)`, which does not change when a field is revealed. Changed to watch `accountStyleProvider.select((s) => s.value?.revealedFields)` and `isSensitiveAccessGrantedProvider`, ensuring tiles rebuild correctly when users click "Reveal"

### Performance

- **Sensitivity Settings Cache** — `SensitivitySettingsPage._buildSettingsView()` previously re-sorted all fields (O(n log n)) and performed 4 sensitivity-level filters + search filter on every rebuild. Added `_getEffectiveFields()` and `_getFilteredSections()` to `_SensitivitySettingsPageState` with memoization via `_cachedEffectiveFields`, `_cachedSections`, `_cachedRegistryHash`, `_cachedAccountStyleHash`, and `_cachedSearchQuery`
- **Trash Provider Aggregation** — `TrashPage._buildTrashContent()` previously called `ref.watch(effectiveSensitivityProvider(fieldId))` inside a `for` loop over 12 item types on every rebuild, causing 12 individual provider watches. Added `trashItemSensitivityMapProvider` in `sensitivity_provider.dart` which aggregates all 12 sensitivities into a single `Map<String, SensitivityLevel>`; `trash_page.dart` now watches only this one provider. Also added `_getFilteredTrash()` to cache filtered deleted items/unified objects by search query
- **Predefined Object Section Cache** — `PredefinedObjectSection` was a `ConsumerWidget` that rebuilt `fieldDefs` (schema property → `FormFieldDef` mapping with `FieldRegistry` sensitivity lookup) on every provider change. Converted to `ConsumerStatefulWidget` with `_cachedFieldDefs` and `_cachedTypeDef`, eliminating redundant O(m × n) registry traversals
- **Effective Field Level Select** — `effectiveFieldLevelProvider` in `sensitivity_based_visibility_widget.dart` previously watched the entire `accountStyleProvider` AsyncValue. Narrowed to `accountStyleProvider.select((s) => s.value)`, reducing rebuilds when only the AsyncValue wrapper state (loading/error) changes

### Changed

- **Test Warnings** — `biometric_credential_service_test.dart`: replaced deprecated `setMockMethodCallHandler` with `TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler`; removed unused `dart:typed_data` import. `profile_data_test.dart` and `profile_provider_test.dart`: added `const` to `ProfileData()` constructor calls

## [1.4.2] - 2026-04-29

### Fixed

- **Trash Emptying Completeness** — `TrashManager._calculateEmptyTrash()` and `getDeletedItems()` now correctly include `awards` in the professional section
- **Memory Leaks** — Fixed TextEditingController leaks in `ObjectEditorPage` (property field removal) and `ObjectCard` (dummy controller created on every build)
- **Null Safety** — Replaced unsafe `!` operators in `unified_object_provider.dart` children lookups with defensive `whereType<UnifiedObject>()`
- **Error Visibility** — Added error logging to 5 previously silent catch blocks in `native_vault_service.dart` and `native_crypto_service.dart`
- **State Consistency** — `purgeOldDeletedItems()` now returns a new immutable `ProfileData` via `copyWith` instead of mutating the input parameter
- **Operation Log Reliability** — `OperationLogService.addEntry()` now `await`s disk persistence before notifying listeners, preventing log loss on crash
- **Unlock Robustness** — `_handleUnlock()` in login page now wraps `unlockVault()` in try-catch to ensure loading spinner resets on unexpected errors
- **ProfileSectionEditor Safety** — `_markDeletedProfile` and `_markRestoredProfile` now use `identity.copyWith()` instead of fragile manual field reconstruction

### Performance

- **Provider Select Optimization** — `home_page.dart`, `object_editor_page.dart`, and `AccountsVersion` provider now use `.select()` for precise state watching, reducing unnecessary rebuilds

### Removed

- **Dead Code Cleanup** — Removed `UnifiedObjectDataExtension`, `verifyPasswordForRestrictedField`, `animatedSection` helper, `ProfileFieldHistories` typedef, and stale lint suppressions

## [1.4.1] - 2026-04-29

### Fixed

- **Critical Privacy Fix: Cross-Account Data Leakage** — Fixed a severe vulnerability where creating a new account after locking a previous account could display the previous account's data:
  - `main.dart`: Auth state transitions to `locked` now trigger `_wipeSensitiveState()` for both manual and auto-lock paths, ensuring all in-memory sensitive state is cleared
  - `_wipeSensitiveState()`: Now includes `unifiedObjectProvider.reset()` to clear UnifiedObject data previously missed
  - `UnifiedObjectNotifier`: `loadFromProfile()` now resets state to empty when `profile == null` (new account without data)
  - `UnifiedObjectNotifier`: `ref.listen(profileNotifierProvider)` now resets state when profile is cleared to `null` (lock/account switch)

### Performance

- **Isolate Offload for Search** — `SearchProvider._performSearch()` now offloads string matching across all identity/travel/financial/professional/unified fields to a background isolate via `Isolate.run()`, eliminating main-thread jank during search
- **Isolate Offload for Data I/O** — JSON encode/decode and `ProfileData.fromJson()` offloaded to isolates for:
  - `ProfileStorageService.loadProfile()` / `saveProfile()`
  - `BackupService.createBackup()` / `createSpecialBackup()` / `restoreBackup()`
- **Lazy List Rendering** — `AppSidebar`, `TrashPage`, and `ObjectCard` lists converted from eager `ListView` (spread operator) to `ListView.builder`, eliminating pre-build of off-screen children
- **Fine-Grained Object Card Rebuilds** — `_ObjectCardItemTile` now uses `fieldHistoriesProvider.select((h) => h.getHistory(item.id, 'unified'))` so each tile rebuilds independently only when its own history changes

### Fixed

- **Operation Log Live Updates** — `OperationLogProvider` architecture fixed from dead `NotifierProvider<OperationLogServiceNotifier, OperationLogService>` to a version-counter notifier, so entries correctly respond to `addEntry` / `refreshFromDisk` / `clearEntries`

### Changed

- **Widget Decomposition (Code Quality)** — Extracted widgets from god files to improve maintainability (no user-facing changes):
  - `settings_page.dart` (1957 → 1105 lines): `CurrentAccountSheet`, `AllAccountsSheet`, `VersionSheet`, `DebugLogSheet`
  - `profile_storage_service.dart` (3500 → 1865 lines): all 22 model classes moved to `core/models/profile_data.dart` with `@JsonSerializable` codegen
  - `home_page.dart` (1253 → 756 lines): `PageEditor`, `IconPicker`, `DashedPlaceholder`
  - `object_card.dart`: `_ObjectCardHeader`, `_ObjectCardPropertiesList`, `_ObjectCardHistorySection`

## [1.4.0] - 2026-04-29

### Added

- **Startup Data Integrity Validation** — `ProfileStorageService.loadProfile()` now runs `_validateAndRepairProfile()` immediately after migration. Automatically repairs:
  - Duplicate `UnifiedObject` IDs (keeps first occurrence)
  - Invalid `childrenIds` references (removes IDs pointing to non-existent objects)
  - Invalid `parentId` references (sets to `null` if parent no longer exists)
  - Repairs are persisted automatically so they don't re-occur on next load
- **Complete Trash Purge Coverage** — `purgeOldDeletedItemsIfNeeded()` and `purgeOldDeletedItems()` now cover all legacy sections:
  - `travel.travelHistory`
  - `professional.skills`, `professional.languages`, `professional.awards`
  - `identity.idCards`, `identity.addresses`, `identity.contact.entries`
  - `unifiedObjects.objects`
- **Field History Orphan Cleanup** — `FieldHistoryService.cleanupOrphanHistories()` removes history entries for permanently deleted items. Wired into `ProfilePersistenceService.loadProfile()` to run automatically on startup.
- **ProfileData.collectAllItemIds()** — Centralized method collecting all item IDs across legacy sections and unified objects, used for cross-section integrity checks.

### Fixed

- **`_calculateEmptyTrash()` completeness** — Now includes `professional.awards` and `unifiedObjects.objects` in permanent deletion.
- **FormHistory unbounded growth** — History entries for deleted items no longer accumulate indefinitely.

---

## [1.3.0] - 2026-04-29

### Added

- **Encrypted Backup & Restore** (`BackupService`) — Full-screen Data Management page. All backups are encrypted with the vault's AES-256-GCM key. Regular backups auto-rotate (max 5) with version-timestamp filenames.
- **Special Backups** — Up to 5 user-named backups stored outside the rotation cycle. Support rename, restore, and delete. Can be created from current state or promoted from any regular backup.
- **Auto-Backup on Unlock** — `AuthNotifier` fires non-blocking backup creation after every successful vault unlock.
- **Auto-Backup on App Upgrade** — `AppVersionTracker` detects version changes and triggers a versioned backup on the first unlock after upgrade.
- **Backup Recovery Prompt** — `LoginPage` detects empty vault + existing backups and offers a restore dialog before creating default items.
- **Account Data Isolation** — `UnifiedObjectNotifier.loadFromProfile()` now resets state to empty when the new account's `unifiedObjects` is null, preventing old account data from leaking into the new account.
- **Default Page Deletion Protection** — `deleteObject()` blocks soft-deletion of `DefaultPageIds` (profile/travel/financial/professional).
- **Default Page Sidebar Filtering** — `AppSidebar` custom pages list now excludes the four built-in default pages.
- **Operation Notification Overlay** — Backup actions use `OperationNotification.show()` (top-floating overlay) instead of `ScaffoldMessenger` SnackBar.

### Changed

- **Data Management** — Moved from BottomSheet (`settings_page.dart`) to standalone page (`data_management_page.dart`) with `AppBar`, `RefreshIndicator`, and full-screen layout.
- **Restore Backup Order** — `restoreBackup()` now reads the target backup file into memory *before* calling `createBackup()` for the protective backup, preventing cleanup from deleting the file being restored.

### Fixed

- **Restore Oldest Backup Failed** — When 5 regular backups existed, the protective backup's cleanup would delete the oldest backup before it could be read. Fixed by reordering read-before-protect.
- **Date Masking Leak** — `_maskedValue()` threshold was 8 chars, causing `1997-08-19` (10 chars) to show `1997••••••••8-19`. Threshold raised to 12 chars for full masking of dates and short IDs.
- **Object Workspace Pop Crash** — `build()` auto-navigate and `_deleteCurrentObject()` both called `context.pop()`, causing double-pop `GoError`. Removed pop from delete, let build handle navigation.
- **Migration `StateError` Crash** — `_migrateProfileDataToUnified._sens()` used `firstWhere` with `on Exception catch`, but `StateError` (from missing field in `FieldRegistry.defaultFields`) is an `Error`, not `Exception`. Fixed to `catch (_)` and added `FormFieldRegistry.getField()` fallback.
- **Account Switch Data Leak** — New account login would display previous account's custom pages because `UnifiedObjectNotifier` state was never cleared when `profile.unifiedObjects == null`.

---

## [1.2.0] - 2026-04-27

### Added
- **Unified Object Model** - Everything is a `UnifiedObject` with `parentId`/`childrenIds` tree structure. Replaces legacy `FlexibleSection`/`FlexibleItem` models.
- **Persistent Sidebar (`AppSidebar`)** - Drag-resizable (180–400px), collapse/expand, with tree-structured custom pages (expand/collapse for nested sub-pages)
- **Object Workspace Page** - Page-centric UI showing children as cards with inline property editing; non-page children shown as list tiles
- **Object Editor Page** - Generic editor for creating/editing any `UnifiedObject` with icon picker, type selection (create-only), and parent assignment
- **Icon Picker Sheet** - Shared bottom-sheet component for selecting from 26 predefined Material icons
- **Lock Vault Confirmation Dialog** - Unified confirmation dialog before locking, with cancel/confirm buttons and tap-outside-to-dismiss
- **Data Size Display** - Settings page Account section now shows total vault data size (B/KB/MB/GB)
- **Property Editor Factory** - Type-aware inline property editors (text, number, date, checkbox, select, multi-select, relation, URL)
- **CHANGELOG.md** - Version history documentation
- **Rust FFI for Argon2id** (`crypto-argon2/`) - High-performance Argon2id key derivation using Rust SIMD optimizations for Apple Silicon
- **Change Password API** - `POST /api/auth/password` endpoint with full data re-encryption
- **Shared Header Component** - Left sidebar with navigation, account badge, and lock button shared across all auth pages
- **Request Timeout** - 10-second timeout on all API client requests to prevent hanging

### Changed
- **Schema Version** - Bumped to v3; `ProfileData` now uses `unifiedObjects` field for all object storage
- **Home Page** - Simplified to main dashboard only; quick actions fixed at 90×90; inline page editor moved to object workspace
- **Custom Page Trash** - Page-type children no longer display in parent workspace (hierarchy visible only in sidebar tree)
- **Object Editor Save** - Save button moved from AppBar to bottom-centered outline button
- **Multi-account Session Persistence** - `sessionToken` and `currentAccount` are now persisted to localStorage
- **Session Validation** - Fixed closure capture bug in auth page redirects using `getState()` pattern
- **Settings Page** - Change Master Password section is now collapsed by default
- **Dashboard** - Simplified toolbar, account info moved to shared Header
- **Vault ChangePassword** - Now properly re-encrypts all profile data with new key

### Fixed
- **Data Persistence on Login** - `UnifiedObjectNotifier` now auto-loads from `ProfileData` via `ref.listen` and explicit `loadFromProfile()` calls on login
- **Lock Button Stucking** - Added try-finally in settings page to ensure button state is restored on error
- **Change Password Flow** - Wrong current password now returns proper error message instead of hanging
- **SetVaultPath Index Loading** - Fixed missing `loadIndex()` call when switching account vaults
- **Account Deletion Lock** - Delete account now properly locks and redirects to login

### Security
- **Argon2id Performance** - Rust SIMD implementation provides ~3x speedup on Apple M-series chips
- **Password Change Re-encryption** - All vault data is now properly re-encrypted when password changes

---

## [1.1.0] - 2026-04-24

### Added

- **Riverpod 3.0 Upgrade** — Upgraded from Riverpod 2.6.1 to 3.0.3
- **Disable Debug Mode Button** — Added power button in debug log sheet to exit debug mode

### Bug Fixes

- Fixed address save not persisting new entries (missing list update logic)
- Fixed soft delete confirmation dialog not showing (alreadyConfirmed flag was wrong)
- Fixed debug mode being lost on page navigation (provider now uses keepAlive)
- Fixed debug mode password dialog dismissing on outside tap (barrierDismissible: false)

### Technical

- StateNotifier → Notifier migration (4 classes)
- ChangeNotifierProvider → NotifierProvider migration
- Auto-retry disabled in ProviderScope
- All generated code regenerated for Riverpod 3.0 compatibility

---

## [1.0.0] - 2026-04-24

### Added

- **macOS DMG Installer** — Official v1.0.0 release with drag-and-drop installation
- **Debug Mode** — Hidden debug log sheet (tap version 5 times to reveal) with colored log levels
- **Improved Keychain Handling** — Better fallback mechanism for non-notarized distribution
- **Biometric Authentication** — Face ID unlock support with password verification fallback
- **Debug Logger Colors** — Color-coded log levels (INFO: cyan, WARN: yellow, ERROR: red, DEBUG: gray)

### Bug Fixes

- Fixed macOS Keychain probe false-negative issues
- Fixed debug log copy button functionality
- Fixed biometric toggle requiring password verification
- Fixed password dialog ghost overlay when cancelled
- Fixed duplicate hint button in message boxes

### Build

- Non-notarized distribution support (sandbox + identity signing disabled)
- DMG packaging with create-dmg tool

---

## [1.0.0-pre.1] - 2026-04-22

### Added

- **Flutter macOS Application** — Native macOS client with full feature set
- **Zero-Knowledge Security Architecture** — Master password never stored
- **Rust FFI Crypto Core** — High-performance Argon2id + AES-256-GCM via native FFI
- **Profile Management** — Identity, travel, financial, and professional data
- **OCR Scanning** — Auto-extract data from passports, IDs, and visas
- **Four-Tier Sensitivity System** — Public / Private / Sensitive / Critical
- **Operation History** — Full audit trail of all changes including sensitivity settings
- **Multi-Account Support** — Each account with independently encrypted storage
- **Local Storage Only** — All data in `~/.solosoul/`, no cloud sync

### Features

- **Profile Editor** — Intuitive tab-based interface for managing all profile data
- **Travel Module** — Passports, visas, travel history management
- **Financial Module** — Bank accounts, card information
- **Professional Module** — Education, employment, skills, languages
- **Sensitivity Settings** — Per-field sensitivity level configuration
- **Operation Log Page** — Searchable history of all profile and settings changes
- **Password Verification Dialog** — Re-authentication for sensitive operations

### Security

- **Argon2id Key Derivation** — Memory-hard KDF (64MB, 3 iterations)
- **AES-256-GCM Encryption** — Military-grade symmetric encryption
- **Secure Memory Handling** — Sensitive values zeroed after use
- **24-Hour Session Tokens** — Automatic expiry for plugin sessions
- **Plugin Consent System** — Per-field authorization for third-party access

### Technical Stack

| Component | Technology |
|-----------|------------|
| Frontend | Flutter, Riverpod |
| Crypto Core | Rust, Argon2id, AES-256-GCM |
| Backend | Go, Gin |
| Storage | Local encrypted files |

### Known Issues

- macOS only (other platforms coming soon)
- Touch ID not yet functional

---

## [0.1.0] - 2026-04-09

### Added
- Core crypto: Argon2id KDF, AES-256-GCM encryption, secure memory
- Vault storage system with file-based implementation
- CLI tool: init, unlock, lock, status, profile commands
- gRPC API server for vault operations
- Plugin management system with consent flow
- OCR module with MRZ parsing and PaddleOCR adapter
- Next.js web UI with login, dashboard, profile editor, vault, OCR, plugins, settings pages
- Multi-account support with independent vault directories
- Comprehensive test suite

[Unreleased]: https://github.com/Gczmy/SoloSoul/compare/v2.6.3...HEAD
[2.6.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.6.3
[2.6.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.6.2
[2.5.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.6
[2.5.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.5
[2.5.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.4
[2.5.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.3
[2.5.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.2
[2.5.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.1
[2.5.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.0
[2.4.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.4.1
[2.3.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.3
[2.3.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.2
[2.3.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.1
[2.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.0
[2.2.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.2
[2.2.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.1
[2.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.0
[2.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.1.0
[2.0.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.0.1
[2.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.0.0
[1.8.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.8.0
[1.7.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.7.1
[1.7.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.7.0
[1.6.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.6
[1.6.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.5
[1.6.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.4
[1.6.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.3
[1.6.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.2
[1.6.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.1
[1.5.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.5.1
[1.5.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.5.0
[1.4.9]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.9
[1.4.8]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.8
[1.4.7]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.7
[1.4.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.6
[1.4.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.5
[1.4.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.4
[1.4.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.3
[1.4.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.2
[1.4.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.1
[1.4.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.0
[1.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.3.0
[1.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.2.0
[1.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.1.0
[1.0.0-pre.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0-pre.1
[1.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0
[0.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v0.1.0
