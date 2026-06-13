# SoloSoul JS SDK

SoloSoul 的 JavaScript 客户端 SDK 占位实现，用于在 Tauri 前端或外部 Web 页面中与 SoloSoul 核心交互。

> 当前为 **P4 占位版本**，仅提供类型定义与 IPC 调用封装。完整实现需配合 Tauri v2 运行环境。

## 安装

```bash
npm install @solosoul/sdk
```

作为 Tauri 插件使用时，需要同时安装 `@tauri-apps/api`：

```bash
npm install @tauri-apps/api
```

## 快速开始

```ts
import { SoloSoulClient } from '@solosoul/sdk';

const client = new SoloSoulClient();

// 解锁 Vault
await client.unlockVault('your-master-password');

// 列出已安装插件
const plugins = await client.listPlugins();
console.log(plugins);

// 运行插件
const result = await client.runPlugin('hello-world', { name: 'SoloSoul' });
console.log(result);
```

## 设计说明

- 所有方法均为异步，底层通过 Tauri `invoke` 调用 Rust Commands。
- 当前已实现命令名与参数类型占位，具体返回值以后端 Commands 为准。
- 非 Tauri 环境下调用会抛出 `SoloSoulError`。

## 支持的命令

| 方法 | 对应 Tauri Command | 说明 |
|------|-------------------|------|
| `unlockVault(password)` | `auth_unlock_vault` | 解锁本地 Vault |
| `lockVault()` | `auth_lock_vault` | 锁定 Vault |
| `listPlugins()` | `plugin_list_installed` | 列出已安装插件 |
| `installPlugin(pluginId)` | `plugin_install` | 安装市场插件 |
| `runPlugin(pluginId, params?)` | `plugin_run` | 运行插件 |

## 开发

```bash
cd sdk/js
npm install
npm run build
```

## 已知限制

- 当前版本未包含错误重试、离线队列、事件订阅等高级功能。
- 需要在 Tauri Webview 或具有 `window.__TAURI__` 的环境中运行。
