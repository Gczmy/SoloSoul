# Flutter 第三轮重构完成报告 (Round 3)

> 更新时间：2026-04-23
> 范围：`flutter/` 目录
> 依据：[audit_round3_report.md](./audit_round3_report.md)

---

## 一、执行摘要

本轮修复基于 `audit_round3_report.md`，聚焦**长期可维护性**。

| 维度 | 状态 |
|------|------|
| P0: 架构决策（Repository层/代码生成） | 进行中 |
| P1: Pilot 全面推广 | 进行中 |
| P2: 状态管理现代化 | 进行中 |
| P3: 开发体验与可观测性 | 进行中 |

---

## 二、P0: 架构决策

### R3-P0-1: Repository 层决策
- 状态: ⏳ 进行中
- 决策: 删除 12 个空 Repository 类，更新架构图

### R3-P0-2: 代码生成启用 (json_serializable Pilot)
- 状态: ⏳ 进行中
- 目标: 为 AccountInfo/DeviceInfo 启用 json_serializable

---

## 三、P1: Pilot 全面推广

### R3-P1-3: ProfileSectionState mixin 全面推广
- 状态: ⏳ 进行中
- 目标: 将 47 处 extends ConsumerState 迁移到 ProfileSectionState

### R3-P1-4: UnifiedFormSection handleDelete 全面推广
- 状态: ⏳ 进行中
- 目标: 所有 Section 使用 handleDelete() + onDidDelete/onDeleteFailed

### R3-P1-5: Professional 页面 SensitivityLevel 完全动态化
- 状态: ⏳ 进行中
- 目标: 修复剩余 6 处硬编码 SensitivityLevel.public

---

## 四、P2: 状态管理现代化

### R3-P2-6: Riverpod v2 迁移 Pilot
- 状态: ⏳ 进行中
- 目标: accountsProvider FutureProvider hack → AsyncNotifier

### R3-P2-7: 减少 ref.read 在 build 中的使用
- 状态: ⏳ 进行中
- 目标: 将关键路径改为 ref.watch

---

## 五、P3: 开发体验与可观测性

### R3-P3-8: 启用关键 lint 规则
- 状态: ⏳ 进行中
- 目标: avoid_catches_without_on_clauses, unawaited_futures 等

### R3-P3-9: 添加性能基准测试
- 状态: ⏳ 进行中
- 目标: crypto_benchmark.dart, storage_benchmark.dart

### R3-P3-10: 添加 TODO 标记已知技术债务
- 状态: ⏳ 进行中
- 目标: 空 Repository、预留模块添加 TODO

---

## 六、提交历史

| Commit | 描述 |
|--------|------|
| (pending) | ... |
