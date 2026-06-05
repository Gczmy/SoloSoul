# 15 — 设备同步功能重构路线图

> **前置阅读**：`05_密码学库选型与核心实现.md`、`06_数据库_服务层_Repository迁移.md`
> **Manifesto 对齐**：本地优先 | 隐私优先 | 用户主权
> **源文档**：`tauri_refactor/同步功能重构路线图.md`

---

## 1. 当前架构诊断

### 已知严重问题（从源码分析）

| # | 问题 | 根因 | 影响 |
|---|------|------|------|
| 1 | CRDT 合并失效，回退到 LWW | Section 级 JSON 字符串存入 Yrs，同 section 不同字段冲突时整段被覆盖 | 字段级修改丢失 |
| 2 | 每次传输携带完整 `profile_json` | SyncMessage 中 profile_json 字段 | 数 MB 冗余传输 |
| 3 | 同步粒度太粗 | Section 级而非字段/对象级 | 修改护照号码和姓名视为同一修改 |
| 4 | 请求-响应模式，无持续连接 | 每次同步需完整握手流程 | 无法后台静默同步 |
| 5 | 离线无队列 | 离线编辑不触发同步 | 重新联网后不自动同步 |
| 6 | 附件串行传输 | for 循环逐个传输 | 效率低下 |
| 7 | Noise_IK 双方使用相同 Keypair | 测试和生产代码共用 | 缺乏设备身份认证 |

### 决策：不在当前架构上修复，在 Tauri 重构中重写

---

## 2. CRDT 引擎选型：Loro

| 维度 | yrs（当前） | Loro |
|------|-----------|------|
| 数据模型 | Map/Text/Array | Map/List/Text/**MovableTree** |
| 树结构 | [错误] 不支持 | [正确] 原生 MovableTree |
| Rust 原生 | [正确] | [正确] |
| 性能 | 文本极快，JSON 一般 | 与 Yjs/Automerge 同级 |
| 推荐度 | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ **首选** |

**Loro 的匹配优势**：
- MovableTree：对象在不同 page/category 间移动 → Loro 正确处理
- 统一数据模型：一个 `LoroDoc` 包含 Map（对象属性）、List（附件）、Text（备注）、Tree（层级）
- 字段级粒度：不需要序列化为 JSON 字符串

```rust
// [正确] Loro：字段级 CRDT
let identity = doc.get_map("identity");
identity.insert("full_name", "Alice Smith");
identity.insert("nationality", "US");
// 另一设备并发修改 passport_number → 自动合并，互不覆盖
```

---

## 3. 重构四阶段计划

### Phase 1：基础修复（MVP，2-3 周）— 不改架构

- 修复 CRDT 粒度：section 级 JSON → 字段级存储
- 移除 `profile_json` 冗余传输
- 修复 Noise 密钥派生（每台设备独立 keypair）
- 附件并发传输
- 持久化同步日志（写入 SQLite）

**继续使用 `yrs`**，仅修改映射策略。

### Phase 2：架构升级（核心重构，4-6 周）

- 引入 `loro` 替换 `yrs`
- 设计 Loro Schema：Map（对象）、Tree（层级）、List（附件）、Text（备注）
- 后台同步服务：长连接 + 心跳 + 自动重连 + 静默同步
- 离线编辑队列（Outbox）：本地修改先写队列，联网后自动批量推送
- QUIC 传输层（`quinn` crate）补充 TCP

### Phase 3：Any-Sync 风格 DAG（4-6 周）

- 每个编辑 = 加密的 Loro diff + Ed25519 签名 + parent CID 列表
- 设备独立身份（Ed25519 密钥对）
- DAG 存储：SQLite `sync_changes` 表
- 多 head 自动合并 + Snapshot 机制
- **可选备份节点协议**（用户自建，E2EE）

> **审批决策（2026-06-05）**：备份节点为**用户自建、完全可选**的功能。
> - 节点仅存储加密后的 change 数据，节点运营者（包括用户自己选择的 VPS 提供商）无法解密
> - 非强制功能：不配置备份节点时，系统仅通过局域网 P2P 同步
> - 备份节点解决"闭盖笔记本问题"（设备关闭时无法同步）

### Phase 4：网络增强（中长期）

- WebRTC 打洞（跨 NAT P2P）
- 带宽自适应 + 增量附件同步
- 冲突 UI（自动合并歧义时用户审查）

---

## 4. 冲突解决策略演进

| 阶段 | 同对象不同字段 | 同字段同时修改 | 文本并发编辑 |
|------|-------------|-------------|------------|
| Phase 1 | [正确] 字段级自动合并 | LWW（updated_at 较新者胜） | N/A |
| Phase 2 | [正确] Loro 自动合并 | Loro Register LWW | [正确] Loro Text CRDT |
| Phase 3 | [正确] 自动合并 | 自动合并 + 保留历史 | [正确] 同 Phase 2 |

---

## 5. 关键依赖库

| 库 | 用途 | 引入阶段 |
|----|------|---------|
| `loro` | CRDT 引擎 | Phase 2 |
| `ed25519-dalek` | 设备身份签名 | Phase 3 |
| `quinn` | QUIC 传输 | Phase 2 |
| `snow` | Noise 协议（保留） | 已有 |
| `x25519-dalek` | 密钥交换（保留） | 已有 |

---

## 6. 风险与缓解

| 风险 | 缓解 |
|------|------|
| `loro` 生态尚不成熟 | Phase 1 先用 `yrs` 修复粒度，Phase 2 再迁移 |
| 后台同步耗电 | 移动端 WiFi 全量、蜂窝仅关键数据 |
| DAG 历史膨胀 | Snapshot + 定期 GC（保留 90 天） |
| 多设备密钥管理 | 密钥派生自账户主密钥 + 设备标识 |

---

## 7. 完成标准

### Phase 1
- [ ] 字段级 CRDT 粒度（不再整段 JSON 覆盖）
- [ ] 同步消息不携带完整 profile_json
- [ ] 每台设备有独立 Noise keypair
- [ ] 同步日志可持久化查询

### Phase 2
- [ ] Loro 替换 yrs 完成
- [ ] 后台持续同步正常工作
- [ ] 离线编辑队列（Outbox）自动推送
- [ ] QUIC 传输层可用

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 4（设备同步）*
