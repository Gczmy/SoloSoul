# 3. Tauri 前端（插件看板与授权交互）

> 实现位置：`tauri/src/pages/ai/PluginDashboardPage.tsx`、`tauri/src/components/plugin/`、`tauri/src/stores/pluginStore.ts`。
> 取代原「06-flutter-ui / 07-plugin-dashboard」设计稿（客户端已从 Flutter 迁移至 Tauri/React）。

## 3.1 组件清单

| 组件 | 职责 |
|------|------|
| `pages/ai/PluginDashboardPage.tsx` | 插件看板：Tab 切换、Tier 筛选、市场/已安装加载、运行编排 |
| `components/plugin/PluginCard.tsx` | 插件卡片：状态徽章、版本、Tier、安装/更新/卸载/运行操作 |
| `components/plugin/PluginConsentDialog.tsx` | 字段授权弹窗（敏感度提示 + 允许/拒绝） |
| `components/plugin/PluginDialog.tsx` | 通用交互对话框（对应 Host `show_dialog`） |
| `components/plugin/PluginRunParamsDialog.tsx` | 运行参数填写弹窗（manifest `params` 声明） |
| `components/plugin/PluginResultPanel.tsx` | 结构化结果展示（文本 / 键值 / 表格 / Markdown） |
| `components/plugin/PluginQuickPanel.tsx` | 快捷面板入口 |
| `components/plugin/PluginQuickNotificationListener.tsx` | 运行完成通知监听 |
| `components/plugin/WatermarkPluginConfig.tsx` | 水印插件专属配置 |
| `stores/pluginStore.ts` | Zustand 状态：市场列表 / 已安装 / 运行中 / Consent 请求 |

## 3.2 看板（PluginDashboardPage）

- **Tab**：`all` / `installed` / `running` / `logs`（全部 / 已安装 / 运行中 / 日志）。
- **Tier 筛选**：`p0`–`p4`，默认启用 `p0/p1/p2`（`DEFAULT_ENABLED_TIERS`），插件按 `manifest.tier` 分批开放。
- **卡片状态机**（`PluginCard`）：

| 状态 | 徽章 | 操作 |
|------|------|------|
| 未安装 | Not Installed | 安装 |
| 已安装 · 最新 | Installed | 运行 / 卸载 |
| 已安装 · 有更新 | Update: x → y | 更新 / 卸载 |
| 运行中 | Running | 停止 |

- **结果与日志**：运行结束在结果面板展示结构化结果；`logs` Tab 展示插件审计日志（`plugin_audit_log`）。

## 3.3 授权与交互流

```
用户点击运行（可选填写 params）
  → plugin_run(plugin_id, params, channel)  →  宿主创建 Session 并执行
  → 宿主经 Channel 推送事件：
       consent_request（敏感字段）→ 弹 PluginConsentDialog → plugin_consent_response
       dialog_request         → 弹 PluginDialog        → plugin_dialog_response
       log / result / completed / error / custom_event → 实时渲染
  → completed 后展示结果面板 + 通知
```

**Consent 弹窗**展示：插件名、请求字段（i18n 标签）、敏感度级别（public / internal / sensitive / critical，对应前端 `SensitivityLevel`），用户可选择允许 / 拒绝；拒绝时 Host 返回 `-2`。

## 3.4 事件流（PluginEvent）

事件信封（`event.rs`）：`event_type` + `json_data` + 可选字段（`plugin_id` / `request_id` / `field_id` / `field_label` / `sensitivity_level` / `custom_type`）。

| event_type | 触发 | 前端处理 |
|------------|------|----------|
| `log` | 插件日志 | 日志面板 |
| `result` | 结构化结果 | 结果面板 |
| `consent_request` | 敏感字段授权 | PluginConsentDialog |
| `dialog_request` | 通用对话框 | PluginDialog |
| `completed` | 正常运行结束 | 结果展示 + 通知 |
| `error` | Trap / 执行失败 | 错误提示 |
| `custom_event` | 插件自定义事件 | 按 `custom_type` 分发 |

## 3.5 IPC 命令（Tauri Commands）

| 命令 | 说明 |
|------|------|
| `plugin_list_all` | 市场插件列表（bundled/远程注册表合并） |
| `plugin_list_installed` | 已安装插件列表 |
| `plugin_list_attachments` | 附件列表（供水印等插件） |
| `plugin_install(plugin_id, version)` | 安装指定版本 |
| `plugin_update(plugin_id)` | 更新到最新版本 |
| `plugin_uninstall(plugin_id)` | 卸载 |
| `plugin_run(plugin_id, params, channel)` | 运行（携带参数与事件 Channel） |
| `plugin_list_sessions` | 活跃会话列表 |
| `plugin_audit_log` | 插件审计日志 |
| `plugin_update_registry` | 手动刷新远程注册表 |
| `plugin_consent_response(request_id, approved, value)` | 授权响应 |
| `plugin_dialog_response(request_id, value)` | 对话框响应 |
