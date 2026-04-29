# SoloSoul 数据存储架构分析与数据库方案评估

> 分析日期: 2026-04-28

---

## 一、当前数据存储架构全景

### 1.1 Flutter 客户端（主项目）

| 平台 | 存储介质 | 数据库 | 加密方式 | 当前状态 |
|------|---------|--------|---------|---------|
| iOS/macOS | vault.db (SQLite via rusqlite) | 有 SQLite | Dart AES-256-GCM -> Rust 存密文 BLOB | 生产环境主路径 |
| Android/Windows | JSON 文件 | 无 | Dart AES-256-GCM -> base64 存 JSON | 回退路径 |

关键发现：
- Cargo.toml: rusqlite = { version = "0.31", features = ["bundled"] } —— SQLCipher 因 Android 交叉编译问题已被移除
- store.rs:49-51: "Skipping PRAGMA key since bundled rusqlite does not include SQLCipher"
- 项目采用"应用层 AES-256-GCM + 明文 SQLite 存密文 BLOB"的折中方案

### 1.2 Go 后端（维护模式）

| 组件 | 存储方式 | 数据库 | 加密 |
|------|---------|--------|------|
| Vault | 文件系统 | 无（JSON 索引） | Go AES-256-GCM，每字段独立 .enc 文件 |
| 账户元数据 | JSON 文件 | 无 | 明文（仅存 salt 和 verify hash） |

---

## 二、Drift + SQLCipher 方案评估

优点：
- 跨平台统一（消除 Android/Windows JSON 回退）
- 响应式查询（Stream 替代 setState）
- 类型安全的 SQL

冲突点：
- 双重加密：当前已是 Dart AES-256-GCM + 密文存 SQLite，再加 SQLCipher = 加密套娃
- FFI 层级竞争：已有 flutter_rust_bridge，Drift 引入第二层 FFI
- 密钥管理冲突：Argon2id 在 Dart 端，SQLCipher 需连接层设密钥
- Android 交叉编译：项目放弃 SQLCipher 的历史原因
- 包冲突：sqlcipher_flutter_libs 与 sqlite3_flutter_libs 互斥

优化价值：
- 安全性：低（当前已足够）
- 跨平台一致性：高（唯一高价值点）
- 迁移成本：极高（需重写 3291 行的 profile_storage_service）

结论：净优化价值有限。安全模型平行而非互补。

---

## 三、纯 Rust 加密数据库方案评估

优点（更契合当前架构）：
- 架构归一：加密逻辑下移到 Rust，Dart 只负责业务
- 单一 FFI：保持 flutter_rust_bridge 为唯一边界
- 消除平台碎片：Rust 统一处理全平台
- 内存安全：zeroize 管理密钥
- rusqlite 直接支持 bundled-sqlcipher

推荐路径：
- Cargo.toml: rusqlite = { version = "0.31", features = ["bundled-sqlcipher"] }
- store.rs: conn.execute_batch("PRAGMA key = ...")

关键障碍：
- Android NDK 上 OpenSSL/sqlcipher 交叉编译仍是最大风险
- 密钥派生（Argon2id）需从 Dart 整体下移到 Rust

优化价值：
- 安全性：高（密钥完全在 Rust 层）
- 性能：高（消除 Dart<->Rust 密文传输往返）
- 跨平台一致性：高（全平台统一 Rust FFI）
- 迁移成本：高（需重写 Vault 服务、store.rs、统一密钥派生）

结论：更合理的技术演进方向，但需优先解决 Android NDK 构建问题。

---

## 四、综合建议

短期：
- 保持当前"应用层 AES-256-GCM + 明文 SQLite 存密文"架构
- 安全性已足够，迁移风险远大于收益

中期：
- 评估 rusqlite + bundled-sqlcipher 在 Android NDK 上的可行性
- 如果可行：将 Argon2id 下移到 Rust，启用 PRAGMA key，删除 JSON 回退
- 这是最简洁、高性能的演进路径

长期：
- 不引入 Drift。Drift 的 orm + code generation + 第二层 FFI 对当前项目的价值无法抵消其引入的复杂度
