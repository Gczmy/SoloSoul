# 增量 IPC 契约生成

RF301 迁移只读命令 `get_app_info`，RF302 再迁移真实注册的 10 个 `object_*` 和 4 个 `snapshot_*` 命令。当前覆盖 15/220 个命令，205 个命令仍待迁移。Rust 命令签名及其 DTO 是参数及响应形状的来源；前端通过 `invokeTypedCommand` 复用现有 `ipcClient` 的鉴权、过期请求检查和错误处理。旧入口仍服务未迁移命令，不宣称全库调用已经类型化。生成类型只做编译期检查，不做运行时响应校验。

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

生成文件为 `src/lib/generated/ipcContracts.ts` 和 `ipcContractManifest.json`。Node 包装器使用锁文件中的 Prettier 及项目配置统一格式。输出顺序稳定，不含绝对路径、时间戳或机器信息；Git 属性将这两个生成文件固定为 LF，避免 Windows 检出制造换行漂移；检查逐字节比较结果，漂移必须由开发者重新生成并审查。清单将实际注册集合划分为已迁移和未迁移两组，ACL 检查要求分区无遗漏、重叠和多余项。当前事件选择为空；事件 payload 生成由非空测试夹具验证，不表示应用事件已迁移。

两份 CI 流程均有独立契约检查，执行真实 Rust 源码副本的参数漂移、TypeScript 编译负例、生成稳定性以及 ACL 集合比较。该任务只编译工具，不需要桌面窗口或用户账户。

## 工具选择和边界

工具版本为 `solosoul-ipc-contract-gen 0.1.0`，使用已锁定的 `syn 2.0.117`、`heck 0.5.0` 和 `toml 0.9.12+spec-1.1.0`，没有升级应用依赖。采用严格 AST 子集，避免把响应序列化形状与输入反序列化形状混为一谈，也避免为了读取类型启动 Host。未选用 Schema 导出或一次性全量迁移；以后扩展支持必须同时添加真实 serde 形状回归。

输入和输出分别生成：响应 `Option<T>` 是必需的 `T | null`，`skip_serializing_if = "Option::is_none"` 生成可选响应字段，`Vec::is_empty` 生成可选数组字段；输入 `Option` 和 serde 默认字段可省略，命名 DTO 的输入形式为 `FooInput`。Tauri 参数按其命令命名规则生成，DTO 按 serde 的字段和枚举规则生成。

支持普通命名 struct、受支持的 tagged/external enum、嵌套 DTO、`Option`、`Vec`、字符串键 `HashMap` 和基础 JSON 类型。`serde_json::Value` 生成递归 `JsonValue`/`JsonObject`，保留 scalar、array、object 和 null；不将它伪装成字段对象。精确支持范围及拒绝规则见工具 crate 文档与测试。宏生成 DTO、条件字段、泛型 DTO、别名、`flatten`、`untagged`、自定义序列化、分向 rename、未知注解等必须报错，不能退化成 `any`。TypeScript 的 `number` 不提供整数范围或大整数精度保证；现有 JSON 数字协议不因此改变。

## 对象与历史契约（RF302）

`object_list/get/field_suggestions/create/update/delete/sync_with_template/ignore_template_sync/list_deprecated_fields/trash_list` 与 `snapshot_count_batch/get_data/list/rollback` 的 25 处生产调用使用类型化入口。未注册的 `object_restore` 不在此列表，`trash_*` 和 `page_delete` 也不因文件相邻而迁入。`snapshot_list` 在 Host 使用四字段 `SnapshotEntry`；Vault 查询及其排序、50 条上限、计数和正文 API 保持原行为。`snapshot_get_data` 仍返回任意 JSON。

`ObjectSummary` 与 `TrashItemSummary` 来自真实 `solosoul-vault` crate。生成器读取显式 workspace 成员、Host 普通 path 依赖和库入口，核对 crate 身份及真实 `mod` 链后才解析；当前不支持重命名、optional、目标条件或 workspace 继承的外部依赖。`crate::` 按 DTO 所属 crate 解析，`crates` 登记不是任意源码白名单。生成清单包含用于核验的三份 Cargo manifest。

Store 使用 `request.invokeTyped`，由 `createTypedInvoker` 包装已有带会话检查的传输函数；普通页面使用 `invokeTypedCommand`。两者均由命令推导参数和返回值，不能由调用者指定响应类型。会话票据在鉴权前后及响应抵达后继续检查有效性，原始错误仍透传。

生成 DTO 是 wire 契约。`objectViewModel.ts` 只派生 UI 所需形状：nullable 元数据转为可选值，乐观新增摘要允许尚未读取的字段缺省，JSON 正文先确认对象形状再展示。字段标签中的显式 null、非法字符串和嵌套值保留原样，由共享敏感度策略保守回退。写入时 `toJsonObject` 校验编辑器的 unknown 值；对象 undefined 键省略、数组 undefined/空位及非有限数值转 null，不修改原值，拒绝循环、BigInt 和非 JSON 对象。

`history.ts`、`templateSync.ts` 与字段建议类型直接重导出生成类型。模板变更的 `kind/payload` 是关联联合，展示器按 `kind` 缩窄，无需响应强转。类型生成不新增运行时 wire 校验，也不代表所有页面竞态已解决。

## 后续逐项迁移

1. 将目标命令响应收敛为明确 Rust DTO，保持既有 wire 字段及行为。
2. 在选择配置登记命令和必要的 DTO 源，运行生成；遇到不支持语法先扩展工具及 serde 回归，不能填手写类型兜底。
3. 将该命令的前端调用迁到 `invokeTypedCommand`，加入参数、返回值及错误路径验证。
4. 运行生成检查、ACL、TypeScript 和相应前后端测试，再按对应任务单独提交。

旧 `invokeCommand` 仍接受字符串，生成器不会阻止调用者继续使用旧入口。严格性只适用于迁移后的类型化入口；事件选择同样需要后续逐个接入实际订阅点。
