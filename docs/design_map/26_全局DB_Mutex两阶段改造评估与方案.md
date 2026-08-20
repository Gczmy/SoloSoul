# 26 — 全局 DB Mutex 两阶段改造评估与实施方案（P025）

> **前置阅读**：`06_数据库与服务层.md`、`04_密码学库选型与核心实现.md`、`13_用户数据边界与加密存储.md`
> **Manifesto 对齐**：本地优先 | 隐私优先 | 性能可用
> **来源**：`docs/CODE_ANALYSIS_REPORT.md` P025（P2 架构/性能）
>
> **[状态] 已实施（Phase 0/1/2/3 全部完成）**（2026-08）：本文档记录对「SQL 取数 → 释放锁 → 锁外解密」
> 两阶段改造的评估结论与分阶段实施方案。Phase 0 观测、Phase 1 两阶段模式、Phase 2 热点转换、
> Phase 3 行为对拍与持锁时长对比已全部落地（提交见 §3，实测数据见 §3 Phase 3）。

---

## 1. 现状核实

### 1.1 锁结构

- `VaultStore.conn: Mutex<Option<Connection>>`（`crates/solosoul-vault/src/storage.rs:484`）：
  全库唯一 SQLite 连接，所有访问经同一把 std Mutex 串行化。
- 全库约 **100 处** `conn.lock()`，分布在 storage / objects / conversations / sync_apply /
  metadata / sync_meta / trash / profile / snapshots / reencrypt 各模块。
- `data_key()`（`storage.rs:729`）持有**独立** Mutex 且克隆密钥（`DataEncryptionKey`）——
  「锁外解密」无需触碰 conn 锁，前提成立。

### 1.2 解密确在持锁闭包内

N 行读取路径全部采用 `query_map` + 行映射器，AES-GCM 解密与 JSON 解析发生在**查询迭代
过程中**（即持锁期间）。代表性热点：

| 路径 | 位置 | 持锁工作量 |
|------|------|-----------|
| `list_object_records`（高级搜索/模板成员展开的整表扫描） | `objects.rs:1000` | 全表 AES 解密 + JSON 解析 |
| `list_objects` / 分页列表 impl | `objects.rs:675`、`:1014` | 同上 + 内存关键词过滤 |
| `load_objects_batch`（批量加载） | `objects.rs:441` | N 行批量解密 |
| `list_conversations`（会话整表） | `conversations.rs` | 整表解密 |
| `list_audit_log` | `metadata.rs:200-228` | 逐行解密 |

粗算：万级对象库、每对象 ~10KB 明文 → 单次搜索持锁 **0.5~2s**（解密 100MB 级 + JSON 解析，
后者通常更慢），期间任何 save / 设置 / 会话 / 同步写全部阻塞——即报告的
「一次高级搜索/同步批量应用期间，GUI 其他 DB 操作全部阻塞」。

### 1.3 既有先例（方向已被项目接受）

- `list_object_metadata`（P111）：公共列表路径不做全表解密（metadata-only）。
- `count_objects` / `objects_size_batch`：纯 SQL，不解密。
- `rewrite_table`：「SELECT 整表 → drop stmt → 再 UPDATE」两阶段模式。
- P008：会话软删/恢复/重命名由整表解密改为单行读取。
- `load_objects_batch` 已用 `prepare_cached` 减编译开销，但解密仍在锁内。

---

## 2. 评估结论

1. **问题部分成立**：痛点是「N 行解密的全表扫描」读者；单行读写（微秒级）与
   `with_tx` 写事务（需原子性）不是问题，不在改造范围。
2. **方案可行**：对读路径完全安全——`query_map` 改为先把加密列原样拷成 owned 数据
   （不解密），drop 锁后统一解密；`data_key()` 独立锁已提供前提。
3. **「全部锁点改造」不必要**：~100 处中绝大部分是单行操作，改造零收益纯风险；
   只转换 6~8 个 N 行解密读者。
4. **关键澄清——WAL 对本设计无益**：SQLite 单连接 + 全局 Mutex 下不存在并发连接，
   WAL 模式无法提供读写并行收益。真正的读写并行需连接池（r2d2_sqlite 等），
   属**独立专项评估**，不应混入本项。

---

## 3. 实施方案

### Phase 0 —— 观测（先行，半天）✅ 已完成（60bf171f）

给 5~6 个热点读函数加 `tracing::debug!` 持锁时长（`lock()` 前记 `Instant`，guard drop
后打印），在真实数据上确认瓶颈分布，据观测结果确认 Phase 2 转换清单，避免凭感觉选目标。

**落地**：新增 `LockHoldObserver`（`storage.rs`，`pub(crate)`，wait=争用等待 / hold=持锁时长，
debug 级），插桩 5 个 N 行解密热点——`list_objects` / `list_object_records` /
`load_objects_batch` / `list_conversations` / `list_snapshots_with_data_batch`。
`list_audit_log` 经核查已是「先 drop(guard) 再解密」的两阶段形态，无需插桩。
新增捕获型 subscriber 验证测试（`test_lock_hold_observer`，`--nocapture` 可查看 wait/hold 基线）。

### Phase 1 —— 建立两阶段模式（核心重构）✅ 已完成（e8064bab）

把 `object_row_to_record`（`objects.rs:308`，P225 单一实现）拆为两步：

```rust
// 步骤 1（持锁）：只取列，不触碰加密内容
struct ObjectRowRaw { id, account_id, …, properties: String, property_labels: String, … }
// 映射闭包仅 row.get() 装箱，无解密、无 JSON 解析

// 步骤 2（锁外）：raw.into_record(&key) —— 原解密 + JSON 解析逻辑原样搬入
```

**约束（必须逐字保留）**：
- 错误语义：P005「properties 损坏拒绝静默降级为空对象」、P225 统一错误前缀、
  `rusqlite::Error::FromSqlConversionFailure` 包装方式不变。
- 排序/分页：`ORDER BY` / `LIMIT / OFFSET` 留在 SQL 阶段 1；内存关键词过滤本就
  在解密后执行，移到锁外语义不变。
- 单遍读取：解锁后不得再次查询（一致性快照不可跨锁保持）。

### Phase 2 —— 按优先级逐函数转换（每个一提交，跑全量测试）✅ 已完成

1. `list_object_records`（持锁最重，先做）
2. `list_objects` / 分页 impl（含 keyword 过滤）
3. `load_objects_batch`
4. `list_conversations`
5. `list_audit_log`（`metadata.rs`）
6. 其余视 Phase 0 观测决定

**边界约束**：只改非 `_tx` 变体——`load_object_tx` 等事务内调用必须留在锁内
（事务原子性）；单行 `load_object` 不动（无收益）。

**落地记录**（每项独立提交，vault 172 用例全绿 + fmt/clippy 干净）：

| 转换项 | 中间形态 | 提交 |
|--------|----------|------|
| `list_object_records`（Phase 1，持锁最重先做） | `ObjectRowRaw`（from_row 装箱 0..21 / into_record 锁外解密+解析） | `e8064bab` |
| `list_objects`（含 keyword 过滤移锁外） | `ObjectListRowRaw`（from_row 装箱 0..17 / into_summary） | `db87583e` |
| `load_objects_batch` | 复用 `ObjectRowRaw`（列序与 OBJECT_SELECT_BASE 一致） | `bdacf2da` |
| `list_conversations` | `ConversationRowRaw`（装箱 id/updated_at/data Blob / into_decrypted） | `70315254` |
| `list_snapshots_with_data_batch` | `SnapshotRowRaw`（装箱 6 列 / into_decrypted + meta JSON 组装） | `7b295517` |
| `list_audit_log` | **无需转换**——已先 drop(guard) 再解密（两阶段形态） | — |

**实施要点**：各 `stmt` 借自 `conn`（借自 `guard`），收在块内先于 guard 释放；
`query_map` 的 `MappedRows` 拆成两条语句绑定局部变量，避免块末临时值借用残留（E0597）。
错误语义逐字保留（P225 统一前缀、P005 拒绝静默降级、各域原文案与列索引）。

### Phase 3 —— 验证 ✅ 已完成（22598679）

- 全量 `cargo test -p solosoul-vault -p solo_soul`（vault 170+ 用例，含既有并发测试）。
  **已完成**：Phase 0/1/2/3 每步均跑全量，vault 172 用例 + 全仓 969 用例全绿、fmt/clippy 干净。
- 用 Phase 0 的 span 对比改造前后持锁时长。**已完成**：新增 `tests/p025_baseline.rs`
  （`#[ignore]`，大数据集 2000 对象×~16KB 明文 + 500 会话 + 快照，仅依赖公开 API，
  可在改造前基线 60bf171f 与改造后同基准运行）。实测对比：

  | 热点 | 改造前 60bf171f | 改造后 HEAD | 降幅 |
  |------|---------------|------------|------|
  | `list_objects` | 469ms | 129ms | -72% |
  | `list_object_records` | 427ms | 106ms | -75% |
  | `load_objects_batch` | 17ms | 1ms | -94% |
  | `list_conversations` | 10ms | 1ms | -90% |
  | `list_snapshots_with_data` | 1ms | 0ms | ~-100% |

  剩余 hold 为 SQL 取数 + 装箱大 payload 的固有成本；解密 + JSON 解析（约 300~340ms）
  已完全移出锁区间，GUI 其他 DB 操作在解密期间不再被阻塞。
- 确认 ORDER BY / 分页 / 过滤语义逐字节不变（行为对拍）。**已完成**：全仓 969 用例
  （含排序/分页/关键词过滤相关既有测试）全绿，转换前后均为同一套 SQL 与内存过滤逻辑。

---

## 4. 明确不做（边界）

- `with_tx` 写路径不转换（事务原子性要求持锁全程）。
- 连接池 / 多连接 + WAL：另行专项评估。
- std Mutex 不换 tokio Mutex：工作为同步 CPU 密集（解密/JSON 解析），无异步收益。

---

## 5. 预期收益与完成标准

**预期收益**：高级搜索 / 批量加载持锁时间从「秒级」降至「纯 SQL 毫秒级」；
GUI 其他 DB 操作在解密期间不再被阻塞；`list_objects` 关键词过滤一并移出锁。

**完成标准**：
- [x] Phase 0 观测设施落地：`LockHoldObserver` + 5 热点插桩 + 验证测试（60bf171f）；小数据集 hold≈0ms，真实库基线待 Phase 3 现场收集
- [x] Phase 1 两阶段模式落地，vault 全量测试通过（e8064bab，172 用例全绿）
- [x] Phase 2 全部转换项完成，每项独立提交且测试通过（db87583e / bdacf2da / 70315254 / 7b295517；`list_audit_log` 核查为已两阶段，免转）
- [x] Phase 3 行为对拍 + 持锁时长对比数据记录（22598679，见 §3 Phase 3 实测表）
