# 插件兼容模板设计 · TODO 状态文档

> **目的**:让任何重启 Session 在打开本文件后,能立即知道 SoloSoul 插件兼容模板设计当前落在哪一阶段、已完成哪些事、还剩什么没做、关键 commit / 文件在哪。
>
> **维护约定**:每个新 stage 完成后,先(1) push 提交,再(2)更新本文件「着陆点」和「已完成」段落,然后(3)重排「待办」顺序。

---

## 1. 着陆点 (Authoritative State)

| 项 | 值 |
|----|----|
| 当前阶段 | **Stage 1 + Stage 2 + Stage 3 全部已落 origin/master (PR #8 + PR #9 合并后)** |
| 工作分支 | `master` (PR #9 branch `feat/plugin-template-stage3-v17-idempotency` merge 后可清理;PR #8 branch 同样) |
| Stage 1 commit (origin/master) | **`382f0cc5`** — `feat(plugin-template): stage 1 schema + v17 migration`(已 push origin/master;后续 v2.3.3 bump 在它之上) |
| Stage 2 commits (origin/master) | **`cf12e7ff`**(PR #8 主 commit)<br>**`e5320dcd`**(doc-sync)<br>**`c6eac2ef`**(Stage 2 boundary tests + cargo fmt)<br>**`3a9c992f`**(Merge pull request #8 from Gczmy/feat/plugin-template-stage2-select-widening) |
| Stage 3 commits (PR #9 / origin/master) | **`b89e221c`**(主 commit)<br>**`fe2c7436`**(doc-sync)<br>**`23fe4d2a`**(review blocker)<br>**`dc8493b5`**(review blocker — Stage 1 遗留 rustfmt 收 lib.rs)<br>**`78c61ecf`**(audit-trail refresh — CHANGELOG + TODO 着陆点,merge 前最末 commit) |
| 推送状态 | ✅ Stage 1/2/3 全部在 `origin/master` (PR #8 + PR #9 合并后);PR #9 branch 可清理 |
| 未提交工作区残留 | §4.4 原 line `tauri/crates/solosoul-core/src/llm/service.rs` / `tauri/crates/solosoul-crypto/src/cipher.rs` (rustfmt-only) 残留 — §4.4 `chore(fmt)` commit 待做 |
| 最近无历史 stage | 前 11 条相邻 commit (`78c61ecf`/`23fe4d2a`/`dc8493b5`/`fe2c7436`/`b89e221c`/`3a9c992f`/`c6eac2ef`/`e5320dcd`/`cf12e7ff`/`382f0cc5`) 同属 Phase 1 plugin-template story line,已全部落 `origin/master` |

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
| **合计 (Stage 1 baseline)** | **197 / 197 tests passed** |

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

> **重要**:重启 Session 第一次起手前必须先**核对**本节是否已有进展!

### 4.1 Stage 2 — SELECT Widening · 必须

> ✅ **[done]** — 落地于 PR #8 (`cf12e7ff` SELECT widening 主体 + `c6eac2ef` boundary tests + `e5320dcd` doc-sync + `3a9c992f` merge)。重启后本项不再需要重做。
> 选择策略:`contract_type_id` 放在 `template_type` 之后(语义上与「模板相关」字段贴合),
> 不是末尾追加。这迫使所有后续 `row.get(N)?` 的 N≥12 都需 shift +1,迁移文档保留作为 Stage 3+ 维护参考。

完成点:

- 5 个 SQL 路径都加了 `contract_type_id` 列(SQL + 闭包双处透传):
  - `VaultStore::save_object` 的 `INSERT INTO objects` 现在含 20 列,`params!`
    含 `obj.contract_type_id.clone()`;`ON CONFLICT` UPDATE 写
    `contract_type_id = excluded.contract_type_id`,兼容 `Some` / `None` 两状态。
  - `load_object` / `list_object_changes_since` / `search_objects` 三个 ObjectRecord
    SELECT closure 现都从 `row.get(16)?` 读 `contract_type_id`;`created_at` /
    `updated_at` / `version` 索引从 16/17/18 顺推到 17/18/19。
  - `list_objects`(返回 `Vec<ObjectSummary>`)的 SELECT 现以 `row.get(12)?`
    取 `contract_type_id`;`icon_name` 索引从 12 顺推到 13。
- 验收测试 3 条 (`cf12e7ff` 主 roundtrip + `c6eac2ef` 2 边界):
  - `test_contract_type_id_roundtrip` (`cf12e7ff`) —
    `save_object(...Some("com.test.plugin/v1")) → load_object(...)` 断言
    `contract_type_id == Some(...)`;同测调用 `list_objects(...)` 断言
    `icon_name == "doc"`(防止 `icon_name` 列 shift +1 误修改)。
  - `test_contract_type_id_none_roundtrip` (`c6eac2ef`) — 边界:save 一个
    `contract_type_id = None` 的 object,断言 `load_object` + `list_objects`
    也返回 `None`(防止"空合约"被静默改成空串)。
  - `test_contract_type_id_default_roundtrip` (`c6eac2ef`) — 边界:save 一个
    通过 default INSERT 路径的 object,断言默认值不被合约列 INSERT 覆盖。
- 验证: `cargo check --workspace --all-targets` 0 errors;
  `cargo test --package solosoul-vault --lib` **90 / 90 passed**(87 baseline + 1 roundtrip + 2 boundary tests)。

注意:

- PR #8 boundary tests 后,`storage.rs::tests` 累计 3 项 Roundtrip family 测试(Stage 2 主 roundtrip 1 项 + Stage 2 boundary 2 项)落地,
  后续 v17 migration 测试 (Stage 3) 不依赖 `storage.rs` 列位调整列索引。
- 后续 Stage 3 code 变动仍必须 走同一个「挨着 `template_type`」列位,避免重做一遍
  row.get 索引映射。

### 4.2 v17 幂等性 · 部分 DB 状态测试 · 必须

> ✅ **[done]** — 落地于 PR #9 (`b89e221c` Stage 3 主 commit + `fe2c7436` doc-sync + `23fe4d2a` review blocker + `dc8493b5` review blocker + `78c61ecf` audit-trail refresh)。重启后本项不再需要重做。

完成点:

- `test_migration_v17_idempotent_run_twice`(走 `setup_conn()` 从 v1 经过 v2–v16 全部落到 v17):
  - 验证两次 `run_migrations` 后 `schema_migrations` 中 `version=17` 仅 1 条
    (证实 gate `current < 17` 第二次正确 skip)。
  - 两表 `contract_type_id` 均 `notnull=0` 且 `(dflt_value IS NULL OR dflt_value = '')` 返回 1
    (Option B 合约 — NULL 与空字符串都是「无合约」合法表述)。
  - `data_version` 第二次调用后仍为 17。
- `test_migration_v17_partial_state`(4 个阶梯 mod 内 for-loop):
  - (false, false) — 双列均无,fresh-install 路径走 2 个 ALTER。
  - (true,  false) — 仅 `user_templates` 已有,`objects` 补列。
  - (false, true)  — 仅 `objects` 已有,`user_templates` 补列。
  - (true,  true)  — 双列已存在,空 `sql_parts` 走 `INSERT OR IGNORE` no-op 路径。
  - 4 个阶梯结尾都验证:两表都有 `contract_type_id` 列 + `schema_migrations.version=17` 仅 1 行 + `data_version == 17`。
- 辅助基础设施(同步落在同一 commit):
  - `setup_v16_partial_state(has_utpl_ctid, has_objects_ctid) -> (Connection, TempDir)`
    手动限定 v16 状阶梯 (skip v1–v15)。
  - `HELPERS_PARTIAL_V16_SQL: &str` 常量内嵌 `/*UTPL_CTID*/` / `/*OBJECTS_CTID*/` 立场标记,
    `setup_v16_partial_state` 以 `.replace()` 条件输送「`contract_type_id TEXT,`」行。

验证门 (commit `b89e221c` + PR #9 整体):

| 命令 | 结果 |
|------|------|
| `cargo check --workspace --all-targets` (cd `tauri`) | **0 errors** |
| `cargo test --package solosoul-vault --lib` | **92 / 92 passed** = 87 baseline + 3 PR #8 (`cf12e7ff` 1 roundtrip + `c6eac2ef` 2 boundary) + 2 PR #9 (idempotency) |
| `cargo test --package solosoul-core --lib` | 72 / 72 ✅ (unchanged) |
| `cargo test --package solosoul-sync --lib` | 22 / 22 ✅ (unchanged) |
| `cargo test --package solosoul-plugin --lib` | 16 / 16 ✅ (unchanged) |
| **合计 (Phase 1 收尾)** | **202 / 202 tests passed** |
| `cargo fmt --all --check` (cd `tauri`) | **clean** (0 diff) |
| `cargo clippy --package solosoul-vault --all-targets -- -D warnings` | **0 warnings** |

Stage 3 review blocker fix (`23fe4d2a` + `dc8493b5`):

- `23fe4d2a` — `migration.rs` `let mut conn` → `let conn`,`cargo fmt --all` 收 `migration.rs` Stage 3 测试区空格。修复 `cargo clippy --workspace --all-targets -- -D warnings` 在 vault 子包的 `unused_mut` 警告 与 `cargo fmt --check` 漂移。
- `dc8493b5` — `cargo fmt --all` 收 `lib.rs` 头 4 处 `#[serde(rename = "contractTypeId", default, skip_serializing_if = "Option::is_none")]` 多行展开,Stage 1 rustfmt 漂移到此收尾。

设计上限 (未覆盖):

- `if current <= 17` gate 退化**绕过完整测试体** — `schema_migrations.version INTEGER PRIMARY KEY` + `INSERT OR IGNORE`
  静默吸收重试插入,行 `description` 与 `applied_at` 都不变。本 test 区分不了「被 skip」与「走 no-op 路径被舍弃」。
  **gate 任务交程序员 code review 维护。**
- 4 阶梯均假定列 default 为 NULL 或空字符串(Option B 严格合规表述)。若 legacy DB 的 default 为 ` ` (单空格)
  或其他非空字符串,本 test 仍会侦为「违约」(`(dflt_value IS NULL OR dflt_value = '')` 返回 0)。
- 不覆盖 `pragma_table_info` 表格收集所隐含的 v8/v9 列(`template_id` / `template_type` / `category`) —
  这些已在 helper DDL 中加进模拟 v16 状,避免 v18+ 倒退误在。
- `setup_v16_partial_state` 与 `VaultStore::init_schema` 的 real schema 在隐式约束上可能有细微差异
  (例如 `init_schema` 在 user_templates `properties_json` 内嵌入 v7 inlined `contract_type_id`,helper 走 v17 ALTER)。
  这对 v17 本身可达性并无影响 (v17 ALTER 屏蔽差异),但 v18+ 倒退误测的可能需要另面分析。

### 4.3 文档回填 · 必做 before next merge

- **`docs/CHANGELOG.md`**:把 `[Unreleased]` Stage 3 条目 (Added/Changed/Fixed 3 条) 合并成 Stable 2.4.0 条目,记录 schema bump 16 → 17、`contractTypeId` / `contractField` serde rename、Stage 2 roundtrip + boundary test additions、Stage 3 idempotency tests。
- **`docs/WORKLOG.md`**:记录本次 Phase 1 的时间线、commit SHA 列表 (Stage 1/2/3 fan-out + Stage 2 boundary tests)。
- **`docs/design_map/30_插件兼容模板设计.md`**:补"Stage 1 + Stage 2 + Stage 3 + Option B 契约"段落,明确告诉插件作者:
  > 任何插件若在裸 SQL `INSERT INTO objects/user_templates` 时写入 `contract_type_id`,**在 Stage 2 SELECT widening 落地之前,值会被 Rust 静默丢弃**(读取永远为 `None`)。Stage 1+2+3 期间请改走 Rust 序列化路径或沿用 Stage 2 SELECT widening roundtrip (`row.get(16)?` 路径)。Stage 3 进一步强制`contract_type_id DEFAULT` 必须是 NULL 或空字符串 (Option B 合约),否则 Stage 3 partial-state tests 会在 PR 校验阶段失败。
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
78c61ecf docs(plugin-template,changelog): stage 3 audit-trail refresh on branch       ← PR #9 merge 前末端
23fe4d2a fix(plugin-template): drop unused_mut on conn + apply rustfmt               ← Stage 3 review blocker
dc8493b5 fix(plugin-template): rustfmt vault lib serde attributes (Stage 1 leftover)  ← Stage 3 review blocker
fe2c7436 docs(plugin-template): mark Stage 3 done and update 着陆点 in TODO tracking    ← Stage 3 doc-sync
b89e221c feat(plugin-template): stage 3 v17 idempotency + partial-state tests         ← Stage 3 (PR #9 主 commit)
3a9c992f Merge pull request #8 from Gczmy/feat/plugin-template-stage2-select-widening ← PR #8 merge
c6eac2ef fix(plugin-template): stage 2 boundary tests + cargo fmt                     ← Stage 2 boundary tests
e5320dcd docs(plugin-template): mark Stage 2 done and update 着陆点 in TODO tracking   ← Stage 2 doc-sync
cf12e7ff feat(plugin-template): stage 2 SELECT widening — roundtrip contract_type_id  ← Stage 2 (PR #8 主 commit)
f3e55664 docs: update CHANGELOG.md for v2.3.3                                          ↑ Stage 1 + v2.3.3 bump
8b18965a chore: bump version to 2.3.3                                                  ← v2.3.3 bump
382f0cc5 feat(plugin-template): stage 1 schema + v17 migration                         ← Stage 1 (已 push origin/master)
cc9f5eb1 feat(updater): enable silent Windows auto-updates and sync Cargo.lock version
a4f49d11 feat(cli): settings phase — replace raw /setting card with SettingsMenu
1c233ddf feat(cli): add /sync /ocr /embed_model; --ocr scan --mrz; CLI release artifacts
```

**(注)** 实测两条 commit 头补插是 `docs: update CHANGELOG.md for v2.3.3` 与 `chore: bump version to 2.3.3`
(原本从 v2.3.2 → v2.3.3,这些是 Stage 1 推送后某次会话动过的)。Stage 2 分支 创建于
`f3e55664`(= `master`) 之上,原因了 v2.3.3 与 Stage 1"打包同推" 被压在一块。

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

> **Stage 1 + Stage 2 + Stage 3 全部已落 `origin/master` (PR #8 + PR #9 合并后完成 Phase 1):
> cross-crate **202 / 202 tests passing** (vault 92/92 = 87 baseline + 3 PR #8 roundtrip family + 2 PR #9 idempotency + core 72/72 + sync 22/22 + plugin 16/16),
> 0 errors / 0 warnings / 0 rustfmt 漂移。Phase 1 plugin-template compatibility story line 整体交付完成。
> 下一步走 §4.3 (`CHANGELOG.md` 滚动 `[Unreleased]` → Stable 2.4.0 + `design_map/30` Stage 3 reach-out 补段) /
> §4.4 (`chore(fmt)` `service.rs` + `cipher.rs` 收尾) / §4.5 (Optional serde roundtrip)。
> 重启 Session 时读本文件 + `git log` 即可恢复上下文;`feat/plugin-template-stage3-v17-idempotency` branch merge 后可 `git branch -d` 清理。**
