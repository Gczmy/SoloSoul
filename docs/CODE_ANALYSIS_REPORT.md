# 代码分析修复报告（云同步专项轮）

> 最后更新：2026-08-23 16:40:00
> 当前分支：`main`
> 修复轮次：1（Phase 2 云同步新代码专项审查）

## 审查范围

- `crates/solosoul-core/src/cloud_sync/`（mod.rs + webdav.rs，~700 行）
- `src-tauri/src/sync/cloud_auto_sync.rs`（~750 行）
- `src-tauri/src/commands/cloud_targets.rs`、`commands/settings.rs` 云同步命令段
- 前端 `CloudSyncPage.tsx`

## 基线检查

| 检查项 | 结果 |
|--------|------|
| `cargo fmt --check` | ✅（本轮已顺手修复 mod.rs 一处格式） |
| `cargo clippy --workspace -D warnings` | ✅ 零告警 |
| `npx tsc --noEmit` / `npm run lint` | ✅ |
| Rust workspace 测试 + E2E | ✅ 990 passed |
| 凭据日志泄漏扫描 | ✅ 无（日志仅含 source 标识与错误分类文本） |

## 问题清单

| ID    | 优先级 | 类别   | 文件位置                                        | 描述                                                                                     | 状态        |
|-------|--------|--------|-------------------------------------------------|------------------------------------------------------------------------------------------|-------------|
| N-001 | P1     | 潜在崩溃 | `crates/solosoul-core/src/cloud_sync/webdav.rs:46` | `WebDavConnector::new` 对用户输入的 `base_url` 执行 `Url::from_str(...).expect(...)`——Settings 表单输错 URL 即 panic 整个应用；同函数 `Client::builder().build().expect()` 同理 | `[x]` 已修复 |
| N-002 | P2     | 资源泄漏 | `src-tauri/src/sync/cloud_auto_sync.rs:336-363`  | 上传失败时临时快照残留在 `{data_dir}/cloud_sync_tmp/` 无清理，长期累积占用磁盘             | `[ ]` 待修复 |
| N-003 | P3     | 规范   | `webdav.rs:85,114,127,222`                      | `Method::from_bytes(b"PROPFIND").unwrap()` 重复 4 处（实际不可失败）；应提为常量           | `[ ]` 待修复 |

## 详细说明与修复方案

### N-001（P1）：用户输入 URL 触发 panic

- **场景**：用户在 CloudSyncPage「服务器地址」填入非法 URL（如漏掉 scheme、多余空格），保存后
  「连接测试」→ `create_connector` → `WebDavConnector::new` → `expect` → 进程 abort。
  Android Release 构建为 panic=abort，直接闪退。
- **方案**：`new` 签名改为 `Result<Self, CloudSyncError>`（URL 解析失败 → `ConfigMissing`
  或新增 `InvalidConfig` 变体；Client 构建失败 → `Internal`）。`create_connector` 已返回
  `CloudResult`，自然透传。调用方仅 create_connector 与测试。
- **验证**：单测断言非法 URL 返回 Err 而非 panic；既有测试改用 `.unwrap()` 于合法输入。

### N-002（P2）：上传失败残留临时快照

- **场景**：导出成功但上传失败（网络断开）时，`?` 在 `remove_file` 之前传播，
  `cloud_sync_tmp/snapshot_*.solosoul` 永久残留（可达数百 MB）。
- **方案**：①上传结果无论成败均清理本次临时文件；②每轮同步开始时清扫目录内
  超过 24h 的陈旧残留（覆盖历史崩溃场景）。

### N-003（P3）：重复 Method unwrap

- `Method::from_bytes(b"PROPFIND")` 编译期即可确定为合法方法名，unwrap 不可失败；
  但四处重复属噪音。提为 `const METHOD_PROPFIND/MKCOL: Method`（Method 为 Copy 类型可常量化
  ——若 const 不稳则用 `OnceLock` 或就地 `from_bytes` 提取为私有 helper 函数）。

- 已完成：1 / 3
- 当前处理：无

#### 修复说明 N-001

- `new` 签名改为 `CloudResult<Self>`：URL 解析失败 → `ConfigMissing`（含输入回显与解析错误详情）、
  scheme 非 http/https → `ConfigMissing`、Client 构建失败 → `Internal`。
- `create_connector` 经 `?` 透传至前端 toast；Android panic=abort 场景不再闪退。
- E2E 新增 t10 回归：5 类非法 URL（空串/非 URL/裸 host/ftp/含空格）断言不 panic 且返回
  `ConfigMissing`。E2E 8/8 通过，clippy 全绿。
