# 增量 IPC 契约生成

RF301 迁移只读命令 `get_app_info`，RF302 再迁移真实注册的 10 个 `object_*` 和 4 个 `snapshot_*` 命令。RF306 继续迁移插件 15 个命令，RF303 迁移 8 个会话命令与普通流式发送。RF304 再迁移备份、导入导出和任务恢复的 16 个命令，RF305 迁移设备/云/SAF 同步与账户恢复的 33 个命令。RF908 加入实际指南检索命令，当前实际注册清单覆盖 89/225 个命令，136 个命令仍待迁移，并生成 11 个选定全局事件。Rust 命令签名及其 DTO 是参数及响应形状的来源；前端通过 `invokeTypedCommand` 复用现有 `ipcClient` 的鉴权、过期请求检查和错误处理。旧入口仍服务未迁移命令，不宣称全库调用已经类型化。生成类型只做编译期检查，不做运行时响应校验。RF307 的错误包另有运行时白名单投影，范围见后文。

## 生成与检查

在 `tauri/` 中运行：

```bash
npm run generate:contracts
npm run check:contracts
npm run test:contracts
npm run check:acl
python -m unittest discover -s scripts -p test_check_acl_consistency.py
```

首次安装使用仓库已有 `npm ci`，并需要 Rust 工具链。Node 版本遵从 CI 的 Node 22。命令以 `cargo run --locked` 编译独立工具 `solosoul-ipc-contract-gen`，不会链接或执行 Tauri Host、Vault、业务命令或 serde 默认值函数，也不读取用户账户数据。Cargo 可以写入编译缓存；`--check` 不改写任何生成契约文件。

工具只读取显式选定源码、真实 `mod` 链、`generate_handler!` 注册和 ACL。`src-tauri/ipc-contracts.json` 只登记命令名称和源码路径，以及附加 DTO 源文件、外部 crate 的 Cargo manifest、事件名称与 Rust payload 类型；禁止在配置中复制参数或响应定义。源码路径必须位于项目内。

生成文件为 `src/lib/generated/ipcContracts.ts` 和 `ipcContractManifest.json`。Node 包装器使用锁文件中的 Prettier 及项目配置统一格式。输出顺序稳定，不含绝对路径、时间戳或机器信息；Git 属性将这两个生成文件固定为 LF，避免 Windows 检出制造换行漂移；检查逐字节比较结果，漂移必须由开发者重新生成并审查。清单将实际注册集合划分为已迁移和未迁移两组，ACL 检查要求分区无遗漏、重叠和多余项。当前选定全局事件为真实 `llm-stream-chunk` 与 RF305 的 10 个同步/恢复事件；插件安装进度与运行事件通过真实命令参数的 Channel<T> 生成，不伪造全局事件名。其余全局事件尚未迁移。

两份 CI 流程均有独立契约检查，执行真实 Rust 源码副本的参数漂移、TypeScript 编译负例、生成稳定性以及 ACL 集合比较。该任务只编译工具，不需要桌面窗口或用户账户。

## 工具选择和边界

工具版本为 `solosoul-ipc-contract-gen 0.1.0`，使用已锁定的 `syn 2.0.117`、`heck 0.5.0` 和 `toml 0.9.12+spec-1.1.0`，没有升级应用依赖。采用严格 AST 子集，避免把响应序列化形状与输入反序列化形状混为一谈，也避免为了读取类型启动 Host。未选用 Schema 导出或一次性全量迁移；以后扩展支持必须同时添加真实 serde 形状回归。

输入和输出分别生成：响应 `Option<T>` 是必需的 `T | null`，`skip_serializing_if = "Option::is_none"` 生成可选响应字段，`Vec::is_empty` 生成可选数组字段；输入 `Option` 和 serde 默认字段可省略，命名 DTO 的输入形式为 `FooInput`。Tauri 参数按其命令命名规则生成，DTO 按 serde 的字段和枚举规则生成。

支持普通命名 struct、受支持的 tagged/external enum、嵌套 DTO、`Option`、`Vec`、字符串键 `HashMap` 和基础 JSON 类型。`serde_json::Value` 生成递归 `JsonValue`/`JsonObject`，保留 scalar、array、object 和 null；不将它伪装成字段对象。精确支持范围及拒绝规则见工具 crate 文档与测试。宏生成 DTO、条件字段、泛型 DTO、别名、Input 或非命名 struct 的 `flatten`、`untagged`、自定义序列化、分向 rename、未知注解等必须报错，不能退化成 `any`。TypeScript 的 `number` 不提供整数范围或大整数精度保证；现有 JSON 数字协议不因此改变。

## 对象与历史契约（RF302）

`object_list/get/field_suggestions/create/update/delete/sync_with_template/ignore_template_sync/list_deprecated_fields/trash_list` 与 `snapshot_count_batch/get_data/list/rollback` 的 25 处生产调用使用类型化入口。未注册的 `object_restore` 不在此列表，`trash_*` 和 `page_delete` 也不因文件相邻而迁入。`snapshot_list` 在 Host 使用四字段 `SnapshotEntry`；Vault 查询及其排序、50 条上限、计数和正文 API 保持原行为。`snapshot_get_data` 仍返回任意 JSON。

`ObjectSummary` 与 `TrashItemSummary` 来自真实 `solosoul-vault` crate。生成器读取显式 workspace 成员、Host 普通 path 依赖和库入口，核对 crate 身份及真实 `mod` 链后才解析；当前不支持重命名、optional、目标条件或 workspace 继承的外部依赖。`crate::` 按 DTO 所属 crate 解析，`crates` 登记不是任意源码白名单。生成清单包含用于核验的所登记的 Cargo manifest。

Store 使用 `request.invokeTyped`，由 `createTypedInvoker` 包装已有带会话检查的传输函数；普通页面使用 `invokeTypedCommand`。两者均由命令推导参数和返回值，不能由调用者指定响应类型。会话票据在鉴权前后及响应抵达后继续检查有效性，原始错误仍透传。

生成 DTO 是 wire 契约。`objectViewModel.ts` 只派生 UI 所需形状：nullable 元数据转为可选值，乐观新增摘要允许尚未读取的字段缺省，JSON 正文先确认对象形状再展示。字段标签中的显式 null、非法字符串和嵌套值保留原样，由共享敏感度策略保守回退。写入时 `toJsonObject` 校验编辑器的 unknown 值；对象 undefined 键省略、数组 undefined/空位及非有限数值转 null，不修改原值，拒绝循环、BigInt 和非 JSON 对象。

`history.ts`、`templateSync.ts` 与字段建议类型直接重导出生成类型。模板变更的 `kind/payload` 是关联联合，展示器按 `kind` 缩窄，无需响应强转。类型生成不新增运行时 wire 校验，也不代表所有页面竞态已解决。

## 插件与资源契约（RF306）

15 个真实注册插件命令及其 DTO 来自 Host `commands/plugin.rs` 与共享 `solosoul-plugin` 的 `manifest/event/install_progress/session`。共享 crate 使用真实库入口 `src/mod.rs`。前端查询、安装、更新、运行、授权回应、会话、审计、注册表刷新、附件列表和输出文件操作均使用类型化入口。`plugin_list_attachments` 仍返回 JSON 文本；会话键为 `sessionId`，时间为毫秒数字；安装响应保留 `installedAt`，审计变体字段保留 `exit_code/field_id`。可空字段和 serde 省略字段按实际序列化分别表达。

`tauri::Webview` 是命令注入参数；`tauri::ResourceId` 生成数字别名，前端交给 SDK `Resource` 管理。安装取消使用其 `close()`，没有额外的 Host `plugin_cancel` 命令。生成器仅在命令根参数接受精确 `tauri::ipc::Channel<T>`，使用真实 SDK `Channel` 的 type import，并按 **Output/Serialize** 解析 payload。Channel 不允许嵌入 JSON DTO、响应或全局事件，不能用普通对象、JSON 克隆或数字句柄替代；原实例直接进入原生传输层。数字别名不新增整数范围或资源归属的运行时检查，归属仍由当前 Webview 的资源表验证。

工具仅扩展单字段 tuple 的 `serde(transparent)` newtype；`PluginResultPayload` 因此是任意 JSON。合法的 `alias`、`deserialize_with` 仅在 Output 方向接受，不执行函数，也不把接受集合不确定的 Input 放宽。RF304 将 `default = "fn"` 的缺键规则扩展到 Input：缺键允许使用默认值，显式传入值仍受原字段类型约束，默认函数永不执行。自定义序列化、flatten、普通 tuple、错误命名空间、伪 Channel 与未支持属性继续拒绝。`#[default]` 仅允许 Default enum 的合法 unit 变体。

`plugin.ts` 重导出 wire 类型；`pluginViewModel.ts` 单独派生内建结果、日志和授权/对话框展示形状。未知 JSON 结果在展示边界过滤，授权事件的必需标识先检查非空；不把后端 String 事件种类伪装为封闭枚举。市场缺少有效最新版本时不能派发安装/更新。参数的 null 默认值转换为原有 boolean 的字符串 false 或其它类型的空字符串，保持运行 params 的字符串值契约；显式字符串默认值不变。安装 abort 仍只关闭一次资源，创建期间取消会在句柄返回后回收，调用等待原生结果及清理完成；运行请求继续使用现有会话票据过滤旧事件/迟到响应，200 条日志、50 条结果、终态通知去重保持。

取消安装和停止结果展示是不同的现有能力：Store 的 `stopPlugin` 使当前前端请求失效，不声称中断 Wasm worker。serde/ResourceTable 单元测试和真实 JavaScript SDK 配合原生 IPC 替身覆盖生命周期边界；这些测试不等同各平台 Webview 实机安装或市场联网测试，也不扩大本项沙箱和授权策略范围。

## 会话与聊天流契约（RF303）

本组登记 `llm_list_conversations/list_trash/get_conversation/save_conversation/soft_delete_conversation/restore_conversation/permanent_delete/rename_conversation` 和 `llm_send_message_stream`。注册指向已有公开实现模块；LLM 根模块用显式名称保留函数、类型与 Tauri 生成的命令宏。`commands/llm/contracts.rs` 集中现有 `ChatContextSelection`、`GuideChunk` 和 `LlmStreamPayload`，旧模块继续重导出；两个上下文字段改用显式 serde rename，实际 JSON 仍为 `objectIds/guideChunks`。Core 的会话、消息与摘要 DTO 保持原样。

`IpcEvents['llm-stream-chunk']` 必需的身份为 `accountId/sessionGeneration/conversationId/requestId`，以及 `chunk/isDone/error`；`error` 是必需 nullable 字段。Store 的真实监听器复用该生成类型，并保留既有身份/代次过滤，补充拒绝缺少 error 键的运行时事件。编译负例与运行时不完整事件用例同时覆盖身份字段；生成类型本身仍不验证运行时 JSON。

`isDone` 表示正文流结束，可以带尾正文，不能等同持久化成功。现有 `__LLM_PERSIST_FAILED__` 错误前缀仍单独更新 `persistFailed`，监听器等命令结算后才收尾，规范正文仍从 Host 的已存会话确认。此项不引入新的错误协议；结构化 LLM 错误由 RF317 继续处理。

普通发送按真实签名生成 `accountId/conversationId/providerId/messages`，不接受 API key；旧调用兼容的 `requestId/contextSelection` 仍允许缺省或 null。发送消息的 wire 是现有 `Vec<serde_json::Value>`，前端 builder 继续限制出站 user/assistant 文本，Host 继续运行既有验证。持久化 `ChatMessage.role` 保持 String，旧 system/tool 等角色可回放。会话输出缺省 `deletedAt` 会省略，输入可缺省或 null；不把所有 optional/nullable 混为一种形状。

会话列表、正文、回收站、修改、预保存和发送均使用已有会话或流请求守卫上的类型化入口。`llmChat.ts` 和消息气泡只派生展示模型，消息 `id/isError` 仍是 UI 字段；上下文选择直接来自生成 Input。RAG 查询、Provider 设置与统计命令没有因相邻而迁移；RAG 的账户参数问题由 RF908 承接。原通用传输兼容入口继续保留，不代表整个 LLM 命令组都已迁移。

共享 JSON fixture 由真实 Rust serde 和生成 TypeScript 同时验证，覆盖完整事件、保存失败与上游失败、旧角色/临时会话、摘要和上下文变体。真实源码副本的 request identity 改名会使 `check:contracts` 失败且不改写输出。完整前后端测试包含原有流隔离、双入口互斥、锁定/重解锁与保存失败通知回归。这些本地检查不代替各平台 Webview 或真实 LLM 服务验收。

## 备份与传输契约（RF304）

本组新增 4 个 `backup_*`、4 个加密导出/附件查询、3 个导入预览/执行、3 个持久导入任务查询/恢复，以及 2 个文档导出命令。备份的 `created_at/size_bytes/object_count` 保留 snake_case，其余传输 DTO 保留现有 camelCase。文档格式参数在 Host 仍是 String；前端格式选择器的封闭选项属于 UI，不冒充后端枚举。普通导出范围和恢复任务摘要保留真实 enum/null 字段，没有引入新包格式或错误协议；结构化传输错误由 RF318 承接。

`commands/export_import/contracts.rs` 集中原有 wire 定义，根模块保留公开类型、函数与 Tauri 命令宏的显式导出。注册指向实际实现模块；测试辅助符号只在其原测试范围导入。密码处理、路径授权、任务调度、会话提交检查及 Core 包读写保持现有执行逻辑。前端的备份页、提醒、预取、范围选择/估算、导出、导入和恢复重试入口均从命令推导参数和结果；带会话票据的调用继续使用 `request.invokeTyped`。

`AdvancedImportRequestInput` 的 `selections/selectedAttachmentIds` 允许缺省或 null，表示全量；空数组明确表示零选择。普通界面继续发送显式选择数组。导出范围开启附件但传空 ID 数组仍是 `Selected(empty)`，不能据此推断为全量导出。旧客户端缺少 `operationId/objectStrategies/locale` 时保留 Native 默认；locale 可省略，但不能显式为 null。普通导入请求与 Resume 参数分开，Resume 不接受新的选择/策略。

`ImportResult.status` 为 `complete/partial/notCommitted`，失败阶段、错误码和 operationId 在响应中是必需 nullable 字段。`attachmentFilesWritten` 与已关联附件计数分别表达，IPC resolve 不代表全部成功。当前前端继续使用原 `importOutcomeError` 和持久任务摘要处理部分提交，不丢弃未完成的输入或恢复入口。共享合成 JSON 经真实 Host serde、生成 TypeScript 编译及实际前端导入 Hook 验证；原有旧包兼容、会话切换、重试和附件回归继续运行。

生成器仅额外接受一个、且只有 `tauri::Runtime` bound 的命令泛型，并要求它用于根层 `AppHandle/Window/WebviewWindow/Webview` 注入；泛型不得进入 JSON 参数、State 内容或返回类型。未使用泛型、额外泛型/bound、where clause 及伪 Runtime/注入类型仍拒绝。serde default 函数只检查普通路径并投影缺键，不执行或猜测返回值；Input alias/custom deserialize 仍拒绝。

`types/exportImport.ts` 的范围树和解密预览仅派生展示摘要；完整生成的 Vault ObjectSummary 仍用于 wire，不因 UI 的局部字段而变可选。云同步和账户恢复其余命令不因使用同一结果类型而纳入本项。本地检查不等同各平台 Webview/SAF 实机或联网服务验收。

## 同步与恢复契约（RF305）

本组包含 16 个 `sync_*`、4 个 `recovery_*`、2 个发现命令、8 个 `cloud_sync_*` 及 3 个 SAF `vault_sync_*`。13 个原有 DTO 集中于 `sync/contracts.rs`，旧模块保留类型重导出。`sync/ipc.rs` 为 4 个跨平台重复函数及 3 个 SAF 命令提供唯一注册入口，转发原实现；旧实现不再重复声明 Tauri 命令属性。发现入口的参数统一为 `timeoutMs`，移动端恢复发现继续返回空列表。网络协议、账户/根目录准入、主机取消、停机等待与持久化导入执行保持原流程。

`SyncResult` 的 wire 只有 summary、计数、冲突和 per_table；`syncViewModel.ts` 从生成类型派生 UI 的时间戳、对端名、入站/失败标记。旧历史缺少展示元数据时继续可读，peer 展示模型中的可选元数据也由 wire 的 Pick/Partial 派生。Host 的 `strategy/clientType/phase` 等开放 String 不改成假设的封闭枚举。云配置读取实际返回 JSON，前端保留已有表单解释边界；保存/测试配置通过共享 `toJsonObject` 进入生成的 JSON 参数，不把 UI 配置接口冒充 wire DTO。

选定全局事件为 `sync-pairing-request/sync-completed/sync-conflicts-updated/sync-nsd-failed/sync-progress/device-sync-auto-status/cloud-sync-status/cloud-sync-incoming/recovery-progress/saf-auth-revoked`。所有实际发送点改用 Rust DTO，已订阅事件复用 `IpcEvents`。配对保留 nodeId、指纹、地址、设备名与 SAS；完成保留 peerNodeId 与双向条数；HLC 的 node_id 保持 hex 字符串。设备自动同步启动保留 snake_case 的 peer_count，终态 message 必需且 nullable。SAF 进度与恢复 operationId 仍按实际发送方省略缺键，不填 null；SAF 授权撤销的 unit payload 为 JSON null。同名原生插件回调不属于这些全局事件。

云事件保留 accountId/sessionGeneration，继续在原会话检查后发送；前端只据入站通知刷新当前列表，不直接采用可能迟到的 payload.files。设备 Store 的会话守卫、SAS 更新、5 秒完成事件合并、账户内历史及 NSD 失败后的开关恢复保持原行为。生成类型没有新增运行时载荷校验，也没有为旧设备事件虚构账户/代次字段。

恢复结果继续将 ImportResult 平铺并增加 accountId/accountName，complete/partial/notCommitted 的 operationId、failureStage、errorCode 及附件写入/关联计数均保留。生成器仅扩展 **Output 的具体命名 struct flatten**；递归、重复/保留键、map、Option、enum、tuple、额外字段选项和 Input flatten 均拒绝。另只忽略真实命令上的 `allow(clippy::too_many_arguments)`。实际 Host serde JSON、生成 TS 编译负例及源码漂移检查覆盖这些边界。本地 Windows 单元验证不替代 macOS/Android/iOS 配对、SAF 或云服务实机验收。

## 后续逐项迁移

1. 将目标命令响应收敛为明确 Rust DTO，保持既有 wire 字段及行为。
2. 在选择配置登记命令和必要的 DTO 源，运行生成；遇到不支持语法先扩展工具及 serde 回归，不能填手写类型兜底。
3. 将该命令的前端调用迁到 `invokeTypedCommand`，加入参数、返回值及错误路径验证。
4. 运行生成检查、ACL、TypeScript 和相应前后端测试，再按对应任务单独提交。

旧 `invokeCommand` 仍接受字符串，生成器不会阻止调用者继续使用旧入口。严格性只适用于迁移后的类型化入口；事件选择同样需要后续逐个接入实际订阅点。


## 对象结构化错误（RF307）

已迁移的 14 个对象/快照命令在 Rust 返回 `BackendError`，其 `code` 是稳定 enum，`safeDetails` 是必需 nullable，`retryable` 是必需 boolean。公开细节只包含 enum 阶段和名称/载荷两项固定上限，不包含对象/字段 ID、名称、值、密钥、路径、SQL 或自由文本 cause。对象保存失败与回滚失败不建议自动重试；锁定、维护或读取失败仍需由用户恢复条件后判断是否重试。成功返回及提交顺序不变，回滚保存后的历史/审计失败继续 best-effort 成功。

生成器从实际 `Result<T, E>` 同时投影成功 `T` 和拒绝 `E`，额外生成 `IpcCommandErrors` 及清单 `structuredErrorCommands`；不在配置中维护第二份错误类型。非 Result 命令的拒绝契约为 `never`（只表示业务签名，无此包）；其余 Result<T, String> 错误继续生成 `string`。错误类型仍须满足既有 Output serde 子集，自定义 Serialize、未知类型或泛型等照常拒绝。TypeScript Promise 的 catch 参数仍是 unknown，这份映射不声称语言可静态保证原生框架或传输失败也遵守业务错误包。

`backendErrorWire.ts` 只导入生成类型，不依赖 i18n、logger 或 Store。它在 IPC 边界复制白名单机器字段，去掉额外 message/cause 和不安全 details；14 个命令的未知/损坏或旧未识别错误统一回退 `INTERNAL_ERROR`。`BackendCommandError.message` 只保留 code，原始 cause 不随 Error 保存；原生框架的参数解码失败也经此安全回退。旧对象的已知 String、重名 ID 和动态字段组消息有独立兼容适配，新 Host 业务直接按失败阶段编码，不依赖英文正文。

`resolveBackendErrorMessage` 与已有 `translateRustError` 在展示层翻译 code；对象 Store 保存机器错误，语言切换后重新翻译。历史页面、快照预览与对象读取的错误提示/日志使用安全投影。IPC 日志对仍未迁移的错误只保留 `LEGACY_ERROR` 类别，但这些域的原拒绝值、旧前缀解析与展示行为继续保留，LLM/传输/同步/插件分别由 RF317～RF320 迁移。Host 诊断仅记录 code、阶段和静态 cause 类型，不保存自由文本 cause；Vault JSON 损坏日志也不记录对象名称。

Core 增加对象创建的阶段错误入口，保留原 `build_create_record` String API；CLI 调用仍取得原错误文本，记录构建与写入逻辑没有复制。合成 JSON fixture 同时经真实 Host serde、生成 TS 编译负例、运行时白名单投影与真实双语 i18next 校验；Vault 写入失败、归属错误与损坏快照仍在实际数据库上验证。


## 指南检索账户绑定（RF908）

`llm_search_guide_chunks` 的参数由实际 `rag.rs` 命令生成：`accountId/query/language` 必需，`topK` 可以缺省或为 null。查询结果使用实际 GuideChunk 输出；serde 允许非有限浮点值成为 null，builder 在再次作为发送 Input 时仅将该分数转换为 0，保留文本及其他字段。关键词回退、有结果/空结果和 Embedding 选择保持原实现。

普通聊天页面及快捷浮窗通过 core 传递发送开始时的账户和原 StreamRun。服务与 builder 在调用前后检查原票据，锁定、切换账户或请求失效后不会接纳旧片段；关闭自动上下文仍不检索指南。检索的普通失败保留空上下文回退，不将失效会话吞成成功。

回归使用共享合成请求 JSON，同时覆盖真实生产服务的 native invoke 参数、页面/浮窗发送链路及编译负例。Windows Host 测试用实际 `generate_handler!`、Tauri 参数解析和临时 Vault 检查缺参数拒绝、关键词命中、空结果及锁定拒绝。IPC 执行使用 Tauri MockRuntime，独立 Wry App 仅提供 AppHandle 路径 API，没有可见原生窗口或真实 Provider 请求；这不替代多端 Webview/联网 Embedding 验收。

## LLM 结构化错误（RF317）

26 个实际注册的 llm_* 命令返回 BackendError；provider 配置、会话读写、普通/流式发送、用量及 RAG 的类型均来自原 Rust 签名。guide_* 文档命令属于保留的独立旧域。llm_check_connection 的离线探测继续返回 false，关键词回退与用量写入的 best-effort 语义继续保留。读取 Profile 的实际 IO 失败不再被伪装成空配置。

LlmStreamPayload 保留必需 nullable 的 error 字段，新增可省略的 failure 错误包；无错误事件和旧 RF303 fixture 的字段形状保持不变。新 Host 的 error 仅含机器码，回复保存失败仍带 __LLM_PERSIST_FAILED__ 前缀；新客户端优先使用 failure.code，字符串前缀及旧 HTTP 文案只在集中兼容层读取。错误、Error.message、日志均不携带 URL、原始响应正文或数据库 cause，诊断仅保留 code、stage 和 cause 类型。

保存失败保留已生成回复并提示复制留存；若目录维护挡住终态通知，则同一包通过 invoke 拒绝返回，客户端仍按“回复已生成但未保存”处理。该路径不绕过 Root 所有权或原会话 gate。会话失效始终拒绝旧请求，不发布到新账户。前端无法读取确认正文时单列“尚未确认保存状态”，不会声称写入一定失败；两个聊天入口与后台通知共用一次提示领取。

Core 的新 typed provider resolver 提供固定类别，原 resolve_chat_provider String API 保留原文案给旧调用方/CLI。错误翻译仅发生在显示层，React 聊天投影使用当前 Hook 的翻译器；传输、Store 和生成契约不执行翻译。
