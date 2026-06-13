"""SoloSoul Python SDK —— 客户端占位实现。

该 SDK 提供与 SoloSoul 核心交互的异步接口。当前为 P4 占位版本，
所有方法均抛出 NotImplementedError，等待本地 IPC 协议确定后实现。
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


class SoloSoulError(Exception):
    """SoloSoul SDK 基础异常。"""

    def __init__(self, message: str, code: str | None = None) -> None:
        super().__init__(message)
        self.code = code


@dataclass
class PluginInfo:
    """已安装插件元数据。"""

    id: str
    name: str
    version: str
    description: str
    tier: str
    installed: bool


@dataclass
class PluginRunResult:
    """插件运行结果。"""

    success: bool
    exit_code: int
    results: list[Any]
    logs: list[dict[str, Any]]


class SoloSoulClient:
    """SoloSoul Python 客户端占位实现。"""

    def __init__(self, base_url: str | None = None) -> None:
        """初始化客户端。

        Args:
            base_url: 可选的本地服务地址。None 表示使用默认 IPC 通道。
        """
        self.base_url = base_url

    async def unlock_vault(self, password: str) -> None:
        """解锁本地 Vault。"""
        raise NotImplementedError("unlock_vault 尚未实现，等待本地 IPC 协议确定")

    async def lock_vault(self) -> None:
        """锁定 Vault。"""
        raise NotImplementedError("lock_vault 尚未实现，等待本地 IPC 协议确定")

    async def is_bootstrapped(self) -> bool:
        """检查是否已完成初始化。"""
        raise NotImplementedError("is_bootstrapped 尚未实现，等待本地 IPC 协议确定")

    async def list_plugins(self) -> list[PluginInfo]:
        """列出已安装插件。"""
        raise NotImplementedError("list_plugins 尚未实现，等待本地 IPC 协议确定")

    async def install_plugin(self, plugin_id: str) -> None:
        """安装市场插件。"""
        raise NotImplementedError("install_plugin 尚未实现，等待本地 IPC 协议确定")

    async def run_plugin(
        self,
        plugin_id: str,
        params: dict[str, str] | None = None,
    ) -> PluginRunResult:
        """运行插件。"""
        raise NotImplementedError("run_plugin 尚未实现，等待本地 IPC 协议确定")
