# SoloSoul Python SDK

SoloSoul 的 Python 客户端 SDK 占位实现，用于在 Python 脚本或自动化工作流中与 SoloSoul 核心交互。

> 当前为 **P4 占位版本**，仅提供接口定义与类型提示。完整实现需要设计本地 IPC 通道（如 Unix Domain Socket / Named Pipe / HTTP）。

## 安装

```bash
pip install -e sdk/python
```

## 快速开始

```python
import asyncio
from solosoul_sdk import SoloSoulClient

async def main():
    client = SoloSoulClient()

    # 解锁 Vault
    await client.unlock_vault("your-master-password")

    # 列出已安装插件
    plugins = await client.list_plugins()
    print(plugins)

    # 运行插件
    result = await client.run_plugin("hello-world", {"name": "SoloSoul"})
    print(result)

asyncio.run(main())
```

## 设计说明

- 所有方法均为 `async`，便于后续集成异步 IPC。
- 当前实现统一抛出 `NotImplementedError`，等待后端暴露本地通信协议。
- 类型提示完整，可直接作为接口契约参考。

## 支持的命令

| 方法 | 说明 |
|------|------|
| `unlock_vault(password)` | 解锁本地 Vault |
| `lock_vault()` | 锁定 Vault |
| `is_bootstrapped()` | 检查是否已完成初始化 |
| `list_plugins()` | 列出已安装插件 |
| `install_plugin(plugin_id)` | 安装市场插件 |
| `run_plugin(plugin_id, params=None)` | 运行插件 |

## 开发

```bash
cd sdk/python
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
```

## 已知限制

- 当前版本未实现真实 IPC，所有方法直接抛出 `NotImplementedError`。
- 后续可通过 Tauri Sidecar、本地 HTTP 服务或 STDIO 通道与核心通信。
