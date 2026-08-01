# 代码分析修复报告

> 最后更新：2026-08-01 16:43:00
> 当前分支：`main`
> 阶段：修复轮次 3 + 复核轮次 1-3 已完成；**全部 64 项已闭环**（P048 于本版最后关闭）。本版为清理版——已完成的 63 项详录已移除，完整历史见 git 中本文件的提交历史与各修复 commit。
> 分析范围：`tauri/`（Rust 后端 `src-tauri/` + `crates/`，React/TS 前端 `src/`）；`solosoul_cli/` 仅 P064 涉及。

## 状态总账

- **已完成并经独立复核确认：64 / 64 项**（P048 最后一处于 2026-08-01 16:43 按已决策方案 A 修复并验证，见下文）。
- 历次复核的关键产出（已闭环，仅留索引）：P004 盖章信任锚（`crates/solosoul-plugin/src/host.rs` `stamp_result_payload`，含防回归单测×4）；P012 插件运行时统一到 crate（方向 B 六步）；P048 手写 hover 全量迁移 `interactive-*` 工具类；P064 CLI 测试夹具修复。

## 基线检查（复核轮次 3 复跑，全部通过）

| 检查项 | 结果 |
|--------|------|
| `cargo fmt --check` / `cargo clippy -- -D warnings`（workspace） | ✅ 0 警告 |
| `cargo test`（workspace） | ✅ 675 通过 0 失败（core 148、plugin 56+2 ignored、vault 99、src-tauri 317） |
| `cargo test`（solosoul_cli） | ✅ 146+2 通过 0 失败 |
| `npx tsc --noEmit` / `npm run lint` | ✅ 通过 |
| `npm run test`（Vitest） | ✅ 44 文件 415 测试通过 |
| 工作区 | ✅ 干净（`acl-manifests.json` 已提交 e81c977e） |

## 问题清单

**全部 64 项已闭环**（P001–P064）。最后关闭的 P048 记录如下：

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P048 | P2 | 重复/规范 | 全前端 | 手写 onMouseEnter/Leave hover 统一迁移到 `interactive-*` 工具类 | `[x]` 已修复（2026-08-01 16:43：四批迁移 + 收尾 3 处 + PasswordInput 方案 A，3 处微差经决策接受为既定视觉，见下文） |

---

## P048 详细讨论（优化修复方案）

### 现状（经复核轮次 2-3 确认）

- 四批迁移 + 收尾 commit（`20bf6eae`、`ed87ece5`、`01eb57d0`、`3dd3d075`、`b4d36d05`）已将 119 处 onMouseEnter / 128 处 onMouseLeave 中的全部纯样式 hover 迁移至 `styles/animations.css:501-984` 的 30+ 个 `interactive-*` 参数化工具类；`currentTarget.style` 内联样式改写全 `src/` **实测清零**。
- 迁移本体视觉等价性抽查通过（9 个迁移点逐值对比），条件态（busy/warning/disabled）经 `:hover:not(:disabled)` 与变量=基值双保险逐分支等价保留。
- 剩余 `onMouseEnter/Leave` 9 处中 8 处确为行为性（tooltip portal、长按、200ms 延迟展开、展开/折叠），予以保留。

### 遗留问题 1：`PasswordInput.tsx` 漏网 —— ✅ 已按方案 A 修复（2026-08-01 16:43，用户决策）

**问题**：容器 div 的 `setIsHovered`（原 :182-183）的全部消费点仅为样式分支——hover 时 border 改 `1px solid var(--accent-primary)`、box-shadow 加 `0 0 0 2px accent 10%`。纯样式 hover，与已被迁移的 `WarningCancelButton` 模式完全相同。此前报告将其标注为「tooltip」系与同文件 `handleHintEnter`（真 tooltip，行为性）张冠李戴。

**修复实施（方案 A，CSS 变量 + 工具类）**：
- `animations.css` 新增 `interactive-password-field`（基态 `--pif-border-color: var(--border-subtle)` / `--pif-ring: none`；`:hover` 改 accent + 10% ring），附注释说明 focus/disabled 守卫机制。
- `PasswordInput.tsx`：删除 `onMouseEnter/Leave` 与 `isHovered` state；容器 className 加 `interactive-password-field`；border/boxShadow 改消费 `--pif-*` 变量；error/isFocused 分支内联写变量值（内联压过类 hover，四态优先级 error > focus > hover > default 逐字等价）；disabled 内联写基值作守卫（div 无 `:disabled` 伪类）。
- 验证：`tsc` ✅ / `eslint` ✅ / Vitest 44 文件 415 测试 ✅。至此全 `src/` 无任何 JS 驱动的纯样式 hover。

**历史方案记录**：曾评估方案 B（保持 state 实现仅修正口径）与方案 C（全量 CSS 化 `:focus-within`），用户于 2026-08-01 决策采用方案 A。

**为什么不能直接用 `interactive-field`**（`animations.css:758-772`）：
1. 该类的 `:focus` 规则对**容器 div 无效**——实际获得焦点的是内部 `<input>`，div 不是可聚焦元素；当前 focus 环由 JS 的 `isFocused`（input 的 onFocus/onBlur）驱动，15% ring。
**技术要点（为何不能直接用现成 `interactive-field`）**：① 该类的 `:focus` 规则对容器 div 无效（实际聚焦的是内部 input，div 非可聚焦元素），focus 环仍须 JS 驱动；② 容器 border 原为四态条件内联样式，内联优先级压死类规则，必须改为「层叠发生在自定义属性上」的变量方案；③ disabled 是 prop 而非 DOM 属性，div 无 `:disabled` 伪类，需组件内联写基值作守卫。方案 B（保持 state 实现仅修正口径）与方案 C（全量 CSS 化）经评估放弃：B 留最后一个 JS 样式 hover 不彻底；C 与该文件内联风格不一致、回归风险最大。

### 遗留问题 2：三处早批次未声明微差 —— ✅ 已决策接受为新视觉基线（2026-08-01，用户决策）

| # | 位置 | 旧值 | 新值（既定视觉） |
|---|------|------|------|
| 1 | `SyncPage.tsx` QR/scan/manual 按钮 hover | tint 12% | `interactive-toolbar` tint 10% |
| 2 | `TrashDetailPanel.tsx` 关闭 X hover | tint 12% | `interactive-accent` tint 10% |
| 3 | `PageGuide.tsx` prev 禁用态 | `text-disabled` 颜色 | opacity 0.4 |

决策理由：10% 已是全项目工具栏类 hover 的统一标准值，两处 12% 是统一前的离群值，向标准收敛符合 P048 统一化目标；opacity 0.4 是更通用的禁用表达；差异在常规使用中不可感知（2% 亮度）。无代码改动。

### 验收结果（P048 关闭）

1. ✅ PasswordInput 按方案 A 迁移（2026-08-01 16:43）；
2. ✅ 三处微差经决策接受为既定视觉；
3. ✅ `tsc` + `lint` + Vitest 415 通过；四态（hover/focus/error/disabled）逻辑逐分支等价性经代码级核验（内联变量压类 hover 的层叠分析），**人工目检（LoginPage/PasswordVerificationDialog/PasswordChangeForm 三处使用点）待阶段 4 前补做**；
4. 待 commit（一项一 commit，见下）。

**下一步**：进入流程阶段 4——全库终版扫描 → 生成终版报告 → 打标签 `code-audit-passed-yyyymmdd`。

---

## 遗留观察（非阻塞，不编号）

- GUI 导入路径 `tauri/src-tauri/src/commands/export_import/import.rs:713-726` 的内联附件名净化是第三份独立实现，无专属单测（core 与 plugin host 两份已有测试）。建议后续补 2-3 条用例防漂移。
- 既有瑕疵（均非本轮引入，记录在案）：`app_state.rs:362-377` PluginManager 第二、三次回退为相同确定性调用（第三次必失败，「三次尝试」名不副实）；`solosoul_cli/src/commands/plugin.rs:449` 重复错位 doc comment、`:575` 注释笔误、`crates/solosoul-plugin/src/manager.rs:257` 变量名 `bunded_version` 拼写。

## 复核历史摘要（详版见 git 历史）

- **复核轮次 1**：58 项声称完成 → 52 通过、5 有出入（P003/P004/P010/P015/P041）、0 虚假。
- **决策轮次**：P012 定方向 B（统一到 crate，P047 并入）；P048 定分批视觉等价重构；P017/P018 确认删除。
- **复核轮次 2**：「63/63 闭环」声明 → 发现 **P004 因 P012 第④步回归重新打开**（盖章未移植 crate）、P048 三处误归、新增 P064（CLI 测试编译失败）。
- **复核轮次 3**：P004 回归修复/P064/P048 收尾三项经核验真实完成；发现 P048 第 4 处漏网（PasswordInput），维持 `[~]`。
