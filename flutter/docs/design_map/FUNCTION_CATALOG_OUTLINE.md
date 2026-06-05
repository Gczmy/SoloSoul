# SoloSoul 功能目录大纲（完整版）

> 本文档汇总 SoloSoul 当前技术栈（Flutter + Rust）下**所有已实现的功能**，按领域分层组织，用于 Tauri 迁移时的功能对照与细化文档拆分。
>
> **状态**：基于 2026-06-04 代码库快照  
> **覆盖范围**：Flutter 客户端（主项目）、Rust 原生核心

---

## 一、认证与安全体系

### 1.1 多账户管理
- **账户创建**：支持创建多个独立账户，每个账户拥有独立的 Vault、 Salt、验证令牌；账户名自定义；密码 ≥8 位；可选密码提示语
- **账户登录**：主密码验证解锁 Vault；密码错误提示；支持从已创建账户列表快速切换
- **账户列表管理**：展示所有本地账户（名称、最后访问时间）；支持删除账户（需密码确认）
- **默认账户**：标记默认账户，启动时自动选中；支持切换默认账户
- **遗留 Vault 迁移**：单账户旧格式自动迁移到多账户格式

### 1.2 Vault 加密体系
- **Vault 初始化**：首次使用时创建加密 Vault，生成 Salt，存储验证令牌（加密后的 `"SOLOSOUL_VAULT_V1"`）
- **Vault 解锁**：主密码 → Argon2id KDF → 派生密钥 → 验证令牌解密验证 → Vault 解锁
- **Vault 锁定**：手动锁定 / 自动锁定 / 系统睡眠前锁定；锁定后擦除内存中的敏感状态
- **修改密码**：在 Vault 解锁状态下，用旧密码验证后重新派生密钥并加密所有数据
- **Vault 统计**：Profile 数量、总大小、最后修改时间
- **Dart 端 Vault 服务**：`RustVaultService` — 通过 FRB 调用 Rust 后端，提供加密/解密 bytes、流式加密/解密文件（SOLO blob v3，1MB 分块，支持进度与取消）
- **Rust 端 Vault 服务**：`native/src/vault/` — SQLCipher 加密 SQLite 存储

### 1.3 生物识别认证
- **Touch ID / Face ID / 指纹**：基于 `local_auth` 插件；检测可用性；纯生物识别认证（无设备密码回退）
- **生物识别凭证管理**：启用/禁用生物识别解锁；存储/清除生物识别绑定的凭证（Keychain / 安全存储）
- **平台限制**：iOS Keychain handler 当前缺失，临时回退到文件存储

### 1.4 安全设置
- **自动锁定**：窗口失焦/后台后延迟锁定（1 / 5 / 15 / 30 分钟 / 从不）；记录用户交互时间戳，超时立即锁定
- **剪贴板自动清理**：复制敏感字段后 N 秒自动清空剪贴板（30 / 60 / 120 秒 / 从不）
- **隐私屏幕**：启用后禁止截屏/录屏
- **窗口失焦锁定**：App 进入 paused / inactive 状态时启动倒计时
- **系统睡眠前锁定**：macOS 监听系统睡眠通知，提前锁定 Vault 并擦除敏感密钥

### 1.5 敏感数据分级与访问控制
- **六级敏感度**：`public` / `internal` / `private` / `sensitive` / `restricted` / `critical`
- **自动掩码**：`internal` / `private` 及以上级别字段自动模糊显示（星号/遮罩）
- **重新验证**：`sensitive` / `restricted` / `critical` 字段查看/编辑前需重新输入密码（1 分钟缓存）
- **敏感度设置页面**：全局字段敏感度注册表浏览与搜索；按账户自定义字段敏感度覆盖；批量调整
- **敏感度标签组件**：`SensitivityTag` / `SensitiveValueWidget` / `SensitivityBlurredWidget` — 统一视觉标识与交互
- **字段历史敏感度**：字段历史记录同样继承当前字段的敏感度级别

### 1.6 密码学核心（Rust）
- **Argon2id KDF**：内存参数可配置（开发 8MiB/2iter/4par，生产 64MiB/3iter/4par），支持 Apple Silicon 优化
- **AES-256-GCM**：加密/解密 bytes、流式分块加密文件（v3 SOLO blob 格式）
- **安全内存擦除**：`SecureWipe` + `runtime.SetFinalizer`；Rust 端 `mlock` + `zeroize`
- **常数时间比较**：`subtle.ConstantTimeCompare` 防时序攻击
- **Rust 原生实现**：
  - `flutter/native/src/crypto/` — 供 Flutter FRB 调用（Argon2id + AES-GCM + 流式加密）
  - `native/src/crypto/` — 供 Flutter 通过 flutter_rust_bridge FFI 调用

---

## 二、数据模型与存储体系

### 2.1 Unified Object Model（统一对象模型）
- **核心概念**：所有数据以 `UnifiedObject` 为统一节点，替代传统固定 Schema
- **对象字段**：
  - `id`, `typeId`, `name`, `icon`, `parentId`, `order`
  - `properties`: `Map<String, PropertyValue>` — 实际存储值
  - `propertyLabels`: `Map<String, String>` — 显示标签
  - `semanticTypes`: `Map<String, String>` — 语义类型标识（如 `pet.name`）
  - `propertyOrder`: `List<String>` — 属性显示顺序
  - `attachments`: `List<Attachment>` — 附件元数据
  - `createdAt`, `updatedAt`, `isDeleted`, `deletedAt`
- **布局模式**：`document`（自由内容 + 嵌套子对象）/ `collection`（ primarily 组织者）
- **属性类型**：`text`, `number`, `date`, `checkbox`, `select`, `multiSelect`, `relation`, `url`
- **内置类型注册表**：`ObjectTypeRegistry` — 预定义类型（page, collection, note, task, contact, item 及各类 `__preset_*` 业务类型）
- **自定义类型**：用户可创建自定义对象类型，定义属性 Schema

### 2.2 默认页面与分区
- **四大默认页面**：Profile / Travel / Financial / Professional（固定 ID）
- **默认分区**：每个页面下预置固定分区（如 Identity, Contact, Passport, Bank Account 等）
- **页面编辑**：首页支持内联页面编辑器，增删改页面结构、调整顺序
- **快速操作**：首页 Dashboard 展示快捷入口、安全项提醒

### 2.3 Profile 数据模型（Go 端旧模型）
- **SuperProfile**：完整用户画像，包含 Identity / Travel / Financial / Professional / Preferences / Documents / Metadata
- **Dart Schema 定义**：`lib/core/models/` — Profile 数据模型，含 Passport、NationalID、Address、Contact 等子结构
- **验证器**：`lib/core/utils/validator.dart` — 正则验证邮箱、电话、URL 等

### 2.4 附件存储系统
- **小文件路径**（≤50MB）：内存一次性加密（SOLO blob v2），通过 `encryptBytes` / `decryptBytes`
- **大文件路径**（>50MB 或任意）：Rust 端流式分块加密（SOLO blob v3，1MB 分块），支持进度文件与取消标志文件
- **附件元数据**：`Attachment` 对象（id, fileId, fileName, mimeType, size, encryptedSize, createdAt）
- **附件预览**：小文件直接内存解密预览；大文件拒绝内存加载（>10MB）；PPTX 支持 macOS QuickLook 原生预览
- **附件下载/上传**：支持取消令牌、进度回调
- **附件池**：管理附件生命周期，自动清理孤立附件

### 2.5 数据持久化与迁移
- **Schema 版本**：当前 v6，支持从 v4/v5 自动迁移
  - v4→v5：内置分区 `typeId` 去页面化（如 `profile_identity` → `__preset_identity`）
  - v5→v6：清除内置类型英文 `propertyLabels`，走动态本地化
- **Profile 存储服务**：`ProfileStorageService` — LRU 缓存（3 条目），通过 `RustVaultService` 存取加密 JSON
- **版本检测**：`AppVersionTracker` — 检测 App 升级，标记待备份提醒

### 2.6 字段历史
- **历史记录追踪**：每个字段的值变更自动保存历史（值、时间戳、来源）
- **历史查看**：对象卡片内展开查看某字段的历史变更记录
- **历史恢复**：从历史记录中恢复旧值
- **敏感度继承**：历史记录同样受字段敏感度控制

---

## 三、核心业务页面功能

### 3.1 启动与引导
- **启动画面**：Logo 动画 + 加载指示器；后台并行初始化（Liquid Glass shader 预热、Rust FFI、OCR 引擎、用户指南索引、临时文件清理）
- **初始化失败处理**：友好错误页面，提示重启
- **生命周期监听**：App 状态变化（paused / inactive / resumed / detached / hidden）触发自动锁定逻辑

### 3.2 登录页面
- **密码输入**：主密码输入框（显隐切换）；错误状态提示
- **生物识别解锁**：一键 Touch ID / Face ID 解锁
- **创建账户表单**：账户名、密码、确认密码、密码提示语；密码强度提示
- **账户列表展开**：显示所有本地账户，点击切换
- **备份恢复提示**：若 Vault 为空但存在备份，提示恢复
- **法律文档**：首次使用需同意隐私政策与服务条款

### 3.3 首页（Home / Dashboard）
- **Liquid Glass AppBar**：标题 + 头部操作按钮组
- **快速操作区**：快捷入口瓷砖（可自定义增删改顺序）
- **安全项提醒**：即将过期（护照、签证等）的安全文档提醒
- **页面树编辑器**：内联编辑页面结构，支持添加/删除/重命名/排序页面
- **空状态**：引导用户添加第一个页面或对象

### 3.4 对象工作区（Object Workspace）
- **层级浏览**：根级对象列表 → 进入子对象 → 无限层级嵌套
- **对象卡片**：图标 + 名称 + 敏感度标签 + 操作菜单
- **排序与重排**：拖拽重排对象顺序
- **空状态引导**：提示创建第一个对象
- **预计算缓存**：`UnifiedObjectCache` — O(1) 读取对象、子对象、工作区子对象

### 3.5 对象编辑器（Object Editor）
- **创建/编辑对象**：通用编辑器，支持所有 UnifiedObject 类型
- **属性编辑**：动态表单，根据 `PropertyDefinition` 渲染对应输入控件（文本、数字、日期、选择器、多选、复选框、URL）
- **语义类型选择**：为字段指定语义类型（如 `person.birth_date`），供插件识别
- **图标选择器**：从分类图标网格中选择对象图标
- **附件管理**：添加/查看/删除附件；显示附件大小与加密后大小
- **字段历史**：展开查看某字段的历史变更
- **对象关系**：`relation` 类型指向其他对象
- **字符计数器**：文本字段实时字数统计

### 3.6 分类页面（Profile / Travel / Financial / Professional）
- **统一入口**：四个页面均通过 `ObjectCategoryPage` + `DefaultPageIds` 渲染对应默认页面
- **预置分区**：每个分类下固定分区（如 Profile 下 Identity、Contact、ID Card、Address）
- **分区模板**：`SectionTemplatePage` — 基于模板快速创建标准分区对象

### 3.7 搜索功能
- **全局搜索页面**：独立搜索页面，聚焦输入框
- **实时过滤**：按对象类型过滤（Profile、Travel、Financial、Professional、All）
- **搜索结果**：展示对象名称、类型、匹配字段预览；点击跳转对象编辑器
- **空状态**：无结果提示

### 3.8 回收站（Trash）
- **软删除**：对象删除后进入回收站（`isDeleted=true`），保留 30 天
- **回收站浏览**：按日期分组展示已删除对象；支持搜索过滤
- **恢复操作**：单个恢复 / 批量恢复；恢复后还原到原位置
- **永久删除**：手动永久删除（需密码验证）；30 天后自动永久清理
- **密码验证**：进入回收站需重新验证密码（敏感操作）

### 3.9 操作日志（Operation Log）
- **日志记录**：每次 CRUD 操作生成 `OperationEntry`，含操作类型、对象名、before→after 差异描述
- **日志浏览**：按时间倒序展示；支持按操作类型过滤（创建、更新、删除、恢复、备份等）
- **日志搜索**：关键词搜索操作记录
- **敏感访问控制**：进入操作日志页面需密码重新验证
- **持久化**：日志保存到本地文件，支持刷新加载

---

## 四、本地文件扫描与智能导入

### 4.1 本地文件搜索（Local Search）
- **扫描配置**：选择扫描路径（默认热路径：桌面、文档、下载）；选择扫描深度（文件名 / 指纹 / 全文解析）
- **目标文件类型**：PDF、图片、Office 文档（Word、Excel、PowerPoint）、文本文件等
- **分层搜索策略**：优先扫描高频目录，避免全磁盘扫描
- **文件大小限制**：按扩展名配置最大扫描大小
- **扫描缓存**：`ScanCacheService` — 记录已扫描文件的时间戳与大小，跳过未变更文件
- **取消机制**：`CancelToken` — 支持随时取消扫描
- **进度报告**：实时显示已扫描数、发现数、跳过数、当前路径

### 4.2 内容解析与分区检测
- **文件名匹配**：通过文件名关键词识别潜在个人信息文件
- **内容指纹**：文件内容特征匹配
- **全文解析**：提取文档文本，检测个人信息模式
- **分区检测器**：`ScanSectionDetector` — 根据文件名和内容自动推断对应 SoloSoul 分区
- **图像扫描**：`ScanImageScanner` — 扫描图片中的文字（OCR）

### 4.3 扫描预览与导入
- **候选列表**：扫描完成后展示所有发现的文件候选，显示匹配的分区与置信度
- **选择导入**：支持全选/取消全选/单个选择
- **冲突检测**：检测与现有数据的重复或冲突
- **AI 智能映射**：调用 LLM 分析文件内容，自动映射到 SoloSoul 的字段结构
- **导入执行**：将选中文件的数据导入 Vault，生成对应的 UnifiedObject
- **导入结果**：统计导入成功数、跳过数、警告列表
- **LLM 配置引导**：若 LLM 未配置，引导用户前往配置页面

### 4.4 扫描进度页面
- **实时进度**：扫描过程中的实时进度展示（进度条、统计数字、当前文件）
- **取消操作**：随时取消扫描

---

## 五、AI / LLM 功能

### 5.1 LLM 聊天对话
- **多会话管理**：左侧边栏展示所有会话列表；新建/删除/重命名会话
- **聊天面板**：消息气泡（用户/AI）；支持 Markdown 渲染
- **响应式布局**：宽屏（>800px）固定侧边栏 + 右侧面板；窄屏 Drawer 会话列表
- **空状态**：无会话时引导创建新会话

### 5.2 LLM 配置
- **后端类型选择**：本地（Ollama）/ 云端（OpenAI、Anthropic、Google 等）
- **API Key 管理**：加密存储各云服务商的 API Key
- **模型选择**：列出可用模型，支持自定义模型名称
- **Ollama 状态检测**：自动检测本地 Ollama 服务是否运行
- **参数配置**：温度、最大 token 数、系统提示词等

### 5.3 模型管理
- **模型列表**：展示已配置模型及其状态
- **模型下载/切换**：本地模型管理
- **用量统计**：Token 使用量、费用统计（按模型/按天/按周/按月）
- **统计卡片**：每日 Sparkline、模型使用饼图、Token 分解、账户统计网格

### 5.4 智能提取与映射
- **OCR + LLM 联动**：扫描文档 OCR 识别后，调用 LLM 提取结构化字段
- **字段映射解析**：`LlmFieldMappingParser` — 将 LLM 输出解析为 SoloSoul 字段映射
- **提取结果预览**：展示 LLM 提取的字段与对应值，用户确认后导入

---

## 六、插件系统

### 6.1 插件市场与生命周期
- **插件看板**：三标签页（全部 / 已安装 / 可更新）
- **插件搜索**：按名称/描述搜索插件
- **插件安装**：从插件市场下载并安装（.wasm 文件）
- **插件卸载**：移除插件及其数据
- **插件更新**：检测并更新到最新版本
- **插件信息展示**：名称、版本、描述、发布者、签名、权限需求

### 6.2 插件执行与沙盒
- **Wasmtime 沙盒**：Rust 后端通过 Wasmtime 运行插件（WASI 目标）
- **执行流程**：加载 manifest → 初始化 session → 执行 → 事件流返回
- **字段授权**：插件访问敏感字段前需用户显式授权（Consent）
- **授权管理**：查看/撤销已授权的字段访问权限
- **会话管理**：活跃 Session 列表；强制卸载插件；Session TTL 过期自动清理
- **iOS 限制**：iOS 不支持 Wasmtime JIT，插件系统不可用

### 6.3 插件数据交互
- **字段映射**：插件通过 `field_map` 访问用户数据，按语义类型匹配
- **初始参数**：执行时传入场景 ID 与字段列表（JSON 序列化）
- **事件流**：`PluginEvent` — ConsentRequest / Completed / Error

---

## 七、同步引擎

### 7.1 局域网设备发现
- **mDNS 广播**：`frbMdnsAdvertise` — 在本地网络广播设备名称与端口
- **mDNS 发现**：`frbMdnsDiscover` — 发现同局域网内的 SoloSoul 设备（3 秒超时）
- **设备列表展示**：显示发现的设备名称、地址

### 7.2 端到端加密同步
- **Noise_IK 协议**：所有通信经过加密
- **配对密钥**：基于 pairingKey + deviceSalt 的身份验证
- **发起方同步**：`frbSyncInitiator` — 连接远程设备，发送状态向量，接收差异，应用 CRDT
- **响应方同步**：`frbSyncResponder` — 监听 0.0.0.0:9900，接收连接，执行同步
- **CRDT 冲突解决**：`native/src/sync/crdt.rs` — 无冲突复制数据类型，自动合并

### 7.3 附件同步
- **文件传输**：CRDT 同步完成后，通过同一加密通道传输附件文件
- **增量同步**：仅传输新增/变更的附件
- **不完整标记**：记录未完整传输的文件，下次续传

### 7.4 同步页面 UI
- **本机信息**：展示本机设备名、同步密钥（显隐切换）
- **设备发现**：一键扫描局域网设备
- **手动连接**：输入远程地址与响应方密钥
- **同步日志**：展示同步历史记录（时间、方向、字节数、附件数）

---

## 八、OCR 引擎

### 8.1 引擎架构
- **Rust ONNX Runtime**：基于 `ort` + PP-OCRv4 模型（det + cls + rec 三模型）
- **模型内存加载**：从 Flutter asset bundle 读取 ONNX 模型，通过 FFI 传递给 Rust 初始化 Session
- **平台支持**：macOS / Linux / Android 完整支持；Windows 因链接冲突暂用 stub（返回友好错误）
- **预热机制**：App 启动后后台异步初始化，避免首次使用等待

### 8.2 MRZ 识别
- **机器可读区识别**：护照/身份证 MRZ 码定位与提取
- **MRZ 解析**：`mrz_parser.dart` + Rust `mrz_pipeline.rs` — 解析 TD1/TD2/TD3 格式，提取姓名、国籍、护照号、出生日期、有效期等
- **超时保护**：10 秒超时
- **置信度检查**：低置信度时抛出异常

### 8.3 通用 OCR
- **文本检测**（det）：定位图像中的文本区域
- **文本识别**（rec）：识别文本内容
- **方向分类**（cls）：检测文本方向并旋转校正
- **后处理**：文本行组装、坐标整理

### 8.4 图像预处理
- **预处理选项**：旋转（0/90/180/270）、去噪、对比度、亮度、裁剪、灰度化
- **Rust 端预处理**：`native/src/ocr/preprocess.rs` — 图像尺寸调整、二值化、边缘增强

### 8.5 OCR 扫描 UI
- **OCR 扫描按钮**：对象编辑器中的浮动扫描按钮
- **LLM 提取选项**：扫描后可选 LLM 智能提取字段
- **结果卡片**：展示识别到的字段与置信度，支持一键填入对象

---

## 九、备份与数据管理

### 9.1 加密备份
- **备份创建**：将当前 Vault 数据（Profile + 附件）加密导出到独立备份目录
- **自动备份**：App 升级后首次启动提示备份
- **备份保留策略**：最多保留 5 份常规备份 + 5 份特别备份；超出时自动清理最旧备份
- **特别备份**：用户可命名创建自定义备份（如"出国前备份"）
- **进度指示**：备份过程中的实时进度条
- **文件权限**：备份文件 `chmod 600`

### 9.2 备份恢复
- **备份列表**：展示所有备份（常规 + 特别），显示时间、大小
- **恢复执行**：选择备份后恢复，需 Vault 已解锁（使用相同密钥）
- **恢复进度**：实时进度展示
- **版本兼容**：备份文件名包含 App 版本号，便于追溯

### 9.3 数据导出/导入
- **导出**：将 Profile 数据导出为加密文件（JSON 格式，AES-256-GCM 加密）
- **导入**：从加密文件导入数据；预览导入内容（对象列表、冲突检测）
- **附件池清理**：清理 Vault 中不再被任何对象引用的孤立附件

### 9.4 Vault 信息管理
- **Vault 统计**：数据大小、附件大小、附件数量、总占用
- **数据迁移**：Schema 自动迁移提示
- **调试日志**：查看应用运行日志（用于诊断）

---

## 十、设置与系统功能

### 10.1 应用设置
- **账户设置**：当前账户信息查看、所有账户列表、账户切换、删除账户
- **访问设置**：语言切换（英文/中文）、生物识别设置
- **安全设置**：自动锁定、剪贴板清理、隐私屏幕、窗口失焦锁定
- **同步设置**：同步相关配置
- **下载设置**：附件下载相关配置
- **LLM 设置**：AI 配置快捷入口
- **插件设置**：插件管理快捷入口
- **广告设置**：广告相关配置
- **应用信息**：版本号、检查更新（GitHub Releases API）、法律文档、调试日志

### 10.2 国际化（i18n）
- **支持语言**：英文（en）、中文（zh）
- **ARB 文件管理**：`lib/l10n/` 下的 ARB 翻译文件
- **动态字段标签**：`translateFieldLabel` / `FieldLabelResolver` — 内置类型字段标签走 ARB 动态本地化，不存储静态英文
- **Locale 切换**：实时切换，无需重启

### 10.3 主题与 UI
- **Material 3 + Liquid Glass**：iOS 26 Liquid Glass 设计语言
- **明暗主题**：跟随系统 / 手动切换
- **Glass Theme**：`GlassTheme` 统一配置玻璃质感参数
- **自适应质量**：`LiquidGlassWidgets.wrap(adaptiveQuality: true)` — 根据设备性能自适应渲染质量

### 10.4 路由与导航
- **GoRouter**：声明式路由；支持 deep link
- **常驻侧边栏**：`AppSidebar` — 页面树导航、账户切换、设置入口
- **响应式布局**：桌面端固定侧边栏 + 内容区；移动端底部导航 / Drawer

### 10.5 原生集成
- **macOS 原生通道**：`NativeChannelService` — 菜单栏锁定回调、系统睡眠回调
- **macOS QuickLook**：PPTX/PPT 文件原生预览
- **Apple Vision OCR**：macOS/iOS 平台可选使用 Apple Vision 框架 OCR（备用方案）
- **剪贴板监控**：`ClipboardMonitorService` — 监控敏感数据复制，定时清理

---

## 十一、Rust 原生核心功能（Flutter 专用）

### 12.1 加密模块（`native/src/crypto/`）
- `aes.rs` — AES-256-GCM 加密/解密
- `argon2.rs` — Argon2id KDF
- `stream.rs` — 流式加密/解密（SOLO blob v3，1MB 分块）
- `utils.rs` — 密码学工具函数

### 12.2 Vault 模块（`native/src/vault/`）
- `store.rs` — SQLCipher 加密 SQLite 存储
- `profile.rs` — Profile 数据存取
- `processor.rs` — 数据处理器
- `migration.rs` — Vault 数据迁移

### 12.3 账户模块（`native/src/account/`）
- `manager.rs` — 多账户管理
- `mod.rs` — 账户类型定义

### 12.4 同步模块（`native/src/sync/`）
- `crdt.rs` — CRDT 冲突解决
- `engine.rs` — 同步引擎
- `protocol.rs` — 同步协议（Noise_IK + 状态向量）
- `transport.rs` — 网络传输层

### 12.5 插件模块（`native/src/plugin/`）
- `sandbox.rs` — Wasmtime 沙盒
- `host.rs` — 宿主函数（安全数据访问接口）
- `manager.rs` — 插件生命周期管理
- `manifest.rs` — 插件清单解析
- `session.rs` — Session 管理
- `store.rs` — 插件数据存储
- `field_map.rs` — 字段映射

### 12.6 OCR 模块（`native/src/ocr/`）
- `general_pipeline.rs` — 通用 OCR 流水线
- `mrz_pipeline.rs` — MRZ 识别流水线
- `inference.rs` — ONNX 推理
- `model.rs` — 模型加载与管理
- `preprocess.rs` — 图像预处理
- `postprocess.rs` — 结果后处理
- `windows_stub.rs` — Windows 平台 stub

### 12.7 设备发现（`native/src/discovery/`）
- `mdns.rs` — mDNS 广播与发现

### 12.8 安全存储（`native/src/safe_storage.rs`）
- 平台原生安全存储封装（Keychain / Windows Credential / Linux Secret Service）

### 12.9 FRB API 绑定（`native/src/api.rs`）
- 1500+ 行，暴露 50+ 个 FRB 函数给 Dart，覆盖：
  - 账户管理（创建、删除、列表、切换）
  - Vault 操作（解锁、锁定、状态、修改密码）
  - Profile CRUD（保存、加载、删除、列表）
  - 加密工具（加密/解密 bytes、文件）
  - 同步（mDNS 广播/发现、发起/响应同步）
  - 插件（列出、加载 manifest、执行、授权响应、强制卸载、Session 列表）
  - OCR（初始化、状态、识别文本、MRZ 提取）
  - 安全存储（读取、写入、删除）

---



## 十四、测试体系

### 14.1 Flutter 测试
- **单元测试** (`test/unit/`)：Provider 逻辑、迁移指纹、版本检测、Vault Service
- **Widget 测试** (`test/widget/`)：页面渲染、敏感标签组件、旅行页面交互
- **集成测试** (`integration_test/`)：应用启动导航、OCR 对话框、FFI 端到端

### 14.2 Rust 测试
- `flutter/native/`：`cargo test`

---

## 十五、CI/CD 与构建

### 15.1 GitHub Actions
- **`ci_cd.yml`**：Rust 测试 → Dart 单元测试 → Widget 测试 → 集成测试 → Release 构建 → DMG 打包 → Draft Pre-release
- **`pr_check.yml`**：`cargo fmt --check` + `cargo clippy` + `dart analyze` + 测试

### 15.2 构建脚本
- `build_rust.sh` — Rust 静态库编译（含交叉编译）
- `build_dmg.sh` — macOS DMG 打包
- `validate_ffi.sh` — FFI 签名一致性验证

---

## 十六、功能-技术栈对照矩阵

| 功能领域 | Flutter (Dart) | Rust 核心 |
|---------|---------------|----------|
| UI 渲染 | ✅ 主实现 | ❌ |
| 状态管理 | ✅ Riverpod | ❌ |
| Vault 加解密 | ✅ Dart 回退（Android） | ✅ 主实现（SQLCipher） |
| Argon2id KDF | ❌ | ✅ 原生 |
| 账户管理 | ✅ UI + 逻辑 | ✅ 原生 |
| Profile 存储 | ✅ 服务层 | ✅ SQLCipher |
| OCR 引擎 | ✅ Dart 封装 | ✅ ONNX 推理 |
| 插件系统 | ✅ UI + 服务 | ✅ Wasmtime |
| 同步引擎 | ✅ UI + 服务 | ✅ mDNS + CRDT |
| 本地扫描 | ✅ 完整实现 | ❌ |
| LLM 对话 | ✅ 完整实现 | ❌ |
| 备份恢复 | ✅ 完整实现 | ❌ |

---

## 后续细化方向

以上 16 个大类可进一步拆分为独立小文档，建议按以下维度拆分：

1. **按用户旅程**：登录 → 首页 → 对象管理 → 分类页面 → 搜索 → 设置 → 退出
2. **按技术层级**：UI 层 → 状态管理层 → 服务层 → 数据层 → 原生层
3. **按迁移优先级**：P0（核心数据 + 认证）→ P1（业务页面）→ P2（高级功能）→ P3（遗留兼容）

*本文档作为总纲，后续每个大类可独立成文，详细描述：功能场景、用户交互流程、数据结构、API 接口、测试要点、迁移注意事项。*
