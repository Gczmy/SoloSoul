# 插件兼容模板设计 · TODO 状态文档

> **目的**:让任何重启 Session 在打开本文件后,能立即知道 SoloSoul 插件兼容模板设计当前落在哪一阶段、已完成哪些事、还剩什么没做、关键 commit / 文件在哪。
>
> **维护约定**:每个新 stage 完成后,先(1) push 提交,再(2)更新本文件「着陆点」和「已完成」段落,然后(3)重排「待办」顺序。

---

## 1. 着陆点 (Authoritative State)

| 项 | 值 |
|----|----|
| 当前阶段 | **Stage 1 完成 · Stage 2 未启动** |
| 工作分支 | `master` (latest = `origin/master`) |
| Stage 1 commit | **`382f0cc5`** — `feat(plugin-template): stage 1 schema + v17 migration` |
| 推送状态 | ✅ 已推送,本地与 `origin/master` 完全同步 |
| 未提交工作区残留 | `tauri/crates/solosoul-core/src/llm/service.rs` (rustfmt-only) + `tauri/crates/solosoul-crypto/src/cipher.rs` (rustfmt-only) — 与 Stage 无关,留作独立 `chore(fmt)` commit |
| 最近无历史 stage | 本仓库为本功能首次落库,此前无相关历史 commit |

> **重启指示**:重启时优先读 `§2 已完成` 确认不要重做已做的事,然后跳到 `§4 待办` 选下一个 stage。

---

## 2. 已完成 (Stage 1 · Data-Layer Foundation)

### 2.1 Schema 字段追加 (`tauri/crates/solosoul-vault/src/lib.rs`)

| 类型 | 追加字段 | 类型 | Serde |
|------|----------|------|-------|
| `ObjectRecord` | `contract_type_id` | `Option<String>` | `rename="contractTypeId"` + `default` + `skip_serializing_if="Option::is_none"` |
| `ObjectSummary` | `contract_type_id` | `Option<String>` | 同上 |
| `UserTemplate` | `contract_type_id` | `Option<String>` | 同上 |
| `TemplateProperty` | `contract_field` | `Option<bool>` | `rename="contractField"` + `default` + `skip_serializing_if="Option::is_none"` |

### 2.2 数据库迁移 v16 → v17 (`tauri/crates/solosoul-vault/src/migration.rs`)

- `CURRENT_SCHEMA_VERSION: u32 = 17`。
- v7 历史 SQL 中 `user_templates` 的 `CREATE TABLE` 内联 `contract_type_id TEXT` 列(冷启路径与 v17 ALTER 路径布局一致)。
- v16 → v17 新迁移块:
  - 两条相互独立的 `pragma_table_info('user_templates')` / `pragma_table_info('objects')` 布尔;
  - 仅当对应表缺列时才 push `ALTER TABLE … ADD COLUMN` 进 `sql_parts`;
  - **无论 `sql_parts` 是否为空,事务都会 commit audit log** (no-op 或实际迁移两种路径都有记录)。
- 整段迁移包在单一 transaction 里,失败回滚一应俱全。

### 2.3 Storage SQL (`tauri/crates/solosoul-vault/src/storage.rs`)

- `init_schema()` 的 `objects` CREATE TABLE 新增 `contract_type_id TEXT` 列 (置于 `tags_json` 与 `created_at` 之间,与 v7 内嵌位置对齐)。
- 测试辅助 `init_schema` 同步生效(独立测试 DB 路径也具备该列)。

### 2.4 字段注入 (Stage 1 · Idempotent)

> 选择 **Option B**:Stage 1 **不**读 `contract_type_id` 列。Stage 1 注入的 `None` 是占位,留待 Stage 2 SELECT widening 时只在 **4 个 SELECT closure 处** 取下并替换为 `row.get(N)?`。

13 个文件共 **86** 处字面量注入,通过确定性正则 heredoc 在多次 commit / push 之间幂等:

| 文件 | 注入数 |
|------|-------|
| `tauri/crates/solosoul-vault/src/storage.rs` | 33 |
| `tauri/crates/solosoul-core/src/template_service.rs` | 7 |
| `tauri/crates/solosoul-sync/src/manager.rs` | 1 |
| `tauri/crates/solosoul-plugin/src/field.rs` | 7 |
| `tauri/src-tauri/src/commands/object.rs` | 17 |
| `tauri/src-tauri/src/commands/template.rs` | 5 (含 1 处 Rust field shorthand 定点 str_replace) |
| `tauri/src-tauri/src/commands/attachment.rs` | 3 |
| `tauri/src-tauri/src/commands/export_import.rs` | 2 |
| `tauri/src-tauri/src/services/llm_context.rs` | 1 |
| `tauri/src-tauri/src/plugin/field.rs` | 7 |
| `tauri/src-tauri/tests/plugin_address_fmt.rs` | 1 |
| **合计** | **86** |

注入逻辑 (供 Stage 2 复用 / 修正时参考):
- 行级扫描,正则锚定 `\b(ObjectRecord|ObjectSummary|UserTemplate|TemplateProperty)\s*\{$`。
- 排除 `impl TypeName {` (regexp 命中 `\bimpl\b` 时跳过) 与 `-> TypeName {` (字符串里 `->` 跳过)。
- 取开口**之后第一行** field-pattern (`name: value,`) 视为目标插入点。
- 直接读开口行后第一段 body indent 作为新行缩进。
- 幂等检测:目标字段已存在于 body 内**时不再**插入(故重跑 0 增量)。
- **未涵盖**:Rust field shorthand (`{ id, name, .. }` 形态)。这次 Stage 1 由定点 str_replace 兜底 (`commands/template.rs:70`),Stage 2 若仍有发现需单独补。

### 2.5 .gitignore 规整 (`tauri/.gitignore`)

- 追加 `*.rlib` 与 `librust_out.rlib` 两条,清空 cargo run 时的中间产物污染。

### 2.6 验证门 (已 Green)

| 命令 | 结果 |
|------|------|
| `cargo check --workspace --all-targets` (cd `tauri`) | **0 errors** |
| `cargo test --package solosoul-vault --lib` | 87 / 87 ✅ |
| `cargo test --package solosoul-core --lib` | 72 / 72 ✅ |
| `cargo test --package solosoul-sync --lib` | 22 / 22 ✅ |
| `cargo test --package solosoul-plugin --lib` | 16 / 16 ✅ |
| **合计** | **197 / 197 tests passed** |

### 2.7 已 push diff stat (commit `382f0cc5`)

14 files changed, +154 / -1。详细见上文 §2.4。

---

## 3. Stage 2 设计意图 (摘自 `docs/design_map/30_插件兼容模板设计.md` · Stage 1 暂停时未涉及)

> 计划中的 Stage 2 目标(非完全定稿,需重启时与 `30_插件兼容模板设计.md` 重新对齐):

- 插件可在保存自定义模板时**显式声明**哪些 `TemplateProperty` 是合约驱动的(`contract_field = true`)、哪些 `UserTemplate` 与 `ObjectRecord` 是「合约对象」(`contract_type_id = "com.foo.bar/whatever/1"`)。
- 加载时 Rust 端根据 `contract_type_id`/`contract_field` 调用插件 host 暴露的字段值接口,执行字段级校验 / 派生 / 加密。
- 由此以来,**任何后续 Login flow / 自动填充 / 同步合并**,都可在不修改 schema 的前提下让插件介入。

---

## 4. 待办 (Next-up · 按优先级排)

> **重要**:重启 Session 第一次起手前必须先**核对本节是否已有进展**!

### 4.1 Stage 2 — SELECT Widening · 必须

> 优先级最高。Stage 2 必做,没有它 `contract_type_id` 列实际是死列。

- 选定**单一列位置**(建议 `template_type` 之后),所有涉及 `objects` 表的 SELECT 均在该位引入 `contract_type_id`。
- 4 个目标 closure(在 `tauri/crates/solosoul-vault/src/storage.rs`):
  - `load_object` (单条读)
  - `search_objects` (列表搜索)
  - `list_objects` / `load_objects_or_all` (页/全量)
  - `list_object_changes_since` (增量同步)
- 仅在该 4 处**移除** `contract_type_id: None` 并**保留 `column_type_id: row.get(N)?`** 的列读取(其它 82 处注入位保留 `None` 不动)。
- 加 roundtrip 测试:`vault.save_object(&obj_with_contract) → vault.load_object(...)`,断言 `contract_type_id == Some("...")`。

### 4.2 v17 幂等性 · 部分 DB 状态测试 · 必须

> 现有 `migration.rs::tests` 缺两条:

- `test_migration_v17_idempotent_run_twice`:连调 `run_migrations(&mut conn)` 两次,断言:
  - `migration_history` 中 v17 行仅 **1** 条(不重复)。
  - `audit_log` 中 v17 entry 仅 **1** 条(关键:验证 no-op 分支也只在第一次 commit)。
- `test_migration_v17_partial_state`:手工 `ALTER TABLE … DROP COLUMN contract_type_id`(仅 `user_templates`),再调 migrations,断言仅缺失表被补列;`objects` 表保持 `pragma_table_info=1` 不被二次 ALTER。

副带核查:`run_migrations` 主循环是否**对已 apply 的版本跳过事务**(防止 fresh-DB 启动每次都 commit 一条 no-op audit,长期审计表膨胀)。

### 4.3 文档回填 · 必做 before next merge

- **`docs/CHANGELOG.md`**:新增 v2.4.0 条目,记录 schema bump 16 → 17、`contractTypeId` / `contractField` serde rename。
- **`docs/WORKLOG.md`**:记录本次 Phase 1 的时间线、commit SHA。
- **`docs/design_map/30_插件兼容模板设计.md`**:补"Stage 1 + Option B 契约"段落,明确告诉插件作者:
  > 任何插件若在裸 SQL `INSERT INTO objects/user_templates` 时写入 `contract_type_id`,**在 Stage 2 SELECT widening 落地之前,值会被 Rust 静默丢弃**(读取永远为 `None`)。Stage 1 期间请改走 Rust 序列化路径或暂勿持久化该列。
- **`wasm-plugin-development-guide.md`**:同上 Option B 契约镜像。

### 4.4 工作区整理 · 顺手做

- 把 `tauri/crates/solosoul-core/src/llm/service.rs` + `tauri/crates/solosoul-crypto/src/cipher.rs` 两个 rustfmt-only 修改合一个独立 commit `chore(fmt): rustfmt solosoul-core/llm + solosoul-crypto` 再 push,清空工作区。
- 先跑 `cargo fmt --check` workspace 一次确认**无其他 rustfmt 漂移**再 stage,避免污染。

### 4.5 Optional Unit Test · 健康度

> 对 `contract_type_id` / `contract_field` 加一组 serde roundtrip 单元测试,在 `vault` / `core` 里挑合适位置。这样 Stage 2 的 SELECT widening 即使改动大,Schema 侧的 rename / default 语义也不会回归。

---

## 5. 关键引用 (导航)

### 5.1 代码侧

- Schema 定义:`tauri/crates/solosoul-vault/src/lib.rs` (搜索 `contract_type_id`、`contract_field`)
- 迁移:同上 crate 的 `migration.rs`
- Storage SQL / closures:同上 crate 的 `storage.rs`
- Field-injection 起点(Stage 1 idempotent 脚本输出位置参考):见上 §2.4 的 13 个文件清单

### 5.2 设计文档

| 文件 | 内容 |
|------|------|
| `docs/design_map/30_插件兼容模板设计.md` | Plugin 模板设计原稿 — Stage 1 / Option B / Stage 2 来源 |
| `docs/design_map/29_模板系统重构方案.md` | 模板系统重构大方案 — 上下文 |
| `docs/design_map/09_对象规范.md` | ObjectRecord / ObjectSummary 的定义与字段语境 |
| `docs/design_map/08_IPC命令接口完整规范.md` | 哪些 IPC 命令透传 object/template,迁移影响的接口面 |
| `docs/design_map/24_测试方案与质量保障.md` | 提交 schema 改动时必跑的测试集 |

### 5.3 工程常数

- Rust 2021 edition,Argon2id + AES-256-GCM。
- `cargo test --workspace --all-targets` 是主验证;通常以 `cd tauri &&` 起手。
- AGENTS.md 提的 7 项核心 crate:`solosoul-vault` / `solosoul-core` / `solosoul-crypto` / `solosoul-sync` / `solosoul-plugin` + `src-tauri` + `solosoul_cli`。

### 5.4 最近 commit 历史(围绕插件模板)

```
382f0cc5 feat(plugin-template): stage 1 schema + v17 migration   ← Stage 1 (本次)
cc9f5eb1 feat(updater): enable silent Windows auto-updates and sync Cargo.lock version
a4f49d11 feat(cli): settings phase — replace raw /setting card with SettingsMenu
1c233ddf feat(cli): add /sync /ocr /embed_model; --ocr scan --mrz; CLI release artifacts
```

---

## 6. 反模式 (Anti-Patterns · 不要再走错)

1. **❌ 用裸 regex heredoc 删除既有行**:
   Stage 1 中途多次 heredoc bug 导致 bracker opener 行被错删。规则:**additive-only**,只 `insert`,永不 `replace` / `delete`。要清理时用 `git checkout HEAD -- <file>` 重置。
2. **❌ 把 `contract_type_id: None,` 视为临时占位然后想紧接着补**:
   Stage 1 选了 Option B,这一阶段 82 处 `None` 都是合法的 skip-Read 占位。**不要**在 Stage 1 阶段手改成 `row.get(N)?`,那会引入 Stage 1 不需要的列读取,白做。
3. **❌ "useless loop" 审计 log 噪声**:
   `run_migrations` 路径若对已 apply 版本仍 commit audit,长期 fresh-DB 用户的 `audit_log` 会膨胀 —— 4.2 核查事项不可延后。
4. **❌ 提交 rustfmt 漂移混入 stage commit**:
   已存在的 `service.rs` / `cipher.rs` rustfmt diff 是另一个 story line,不要合进 Stage 1 commit(Stage 1 commit 已 push,合并是过去式,但下一条 commit 仍需注意)。
5. **❌ 改 schema 时忘了 `serde` forward-compat**:
   `Option<T>` 的 `default + skip_serializing_if` 组合**must**保留,不要改成裸 `bool`/`String` —— 否则旧数据反序列化失败。

---

## 7. 一句话 TL;DR

> **Stage 1 schema + v17 migration 已落库 (`382f0cc5`,origin/master,197/197 tests pass);Stage 2 (SELECT widening + roundtrip测试) 是首要重启任务。**
