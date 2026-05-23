## 14. 插件生命周期管理

### 14.1 安装（与主软件分离）

```dart
Future<void> installFromMarket(String pluginId) async {
  // 1. 获取远程 registry
  // 2. 版本兼容性检查（plugin_api_version + min/max_app_version）
  // 3. 下载 wasm + manifest 到临时目录
  // 4. SHA-256 校验
  // 5. 安装到 ~/.solosoul/plugins/{plugin_id}/（与 Vault 同级）
  // 6. 更新 installed.json
  // 7. 清理临时文件
}
```

### 14.2 更新（插件独立更新，不随 App 发布）

- **策略**：新版本安装后旧版本 Session 立即失效，下次运行强制加载新版 wasm。
- **版本共存**：不允许。同一 `plugin_id` 仅保留最新通过白名单校验的版本。
- **回滚**：若新版运行失败，自动回退到上一个已知的有效版本（前提是旧版 wasm 仍存在于本地缓存目录 `.backup/` 中）。

### 14.3 卸载（彻底分离，不影响主软件）

```dart
Future<void> uninstallPlugin(String pluginId) async {
  // 1. 立即撤销所有活跃 Session（Rust 侧）
  await rustApi.pluginForceUnload(pluginId);

  // 2. 删除插件目录（wasm + manifest + config + cache）
  final pluginDir = Directory('${_pluginDir.path}/$pluginId');
  if (await pluginDir.exists()) {
    await pluginDir.delete(recursive: true);
  }

  // 3. 保留审计日志（审计日志属于主软件，不在插件目录中）
  // 4. 更新 installed.json（标记为已卸载）
  await _updateInstalledIndex(pluginId, null, 'uninstalled');

  // 5. 触发 UI 刷新
  ref.invalidate(installedPluginsProvider);
}
```

### 14.4 运行状态管理

Rust 侧 `PluginSessionManager` 负责跟踪所有活跃 Session：

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct PluginSessionManager {
    /// plugin_id -> SessionInfo
    active: Arc<Mutex<HashMap<String, SessionInfo>>>,
}

pub struct SessionInfo {
    pub session_id: String,
    pub plugin_id: String,
    pub started_at: Instant,
    pub expires_at: Instant,
}

impl PluginSessionManager {
    pub fn list_active(&self) -> Vec<SessionInfo> {
        let guard = self.active.lock().unwrap();
        guard.values().cloned().collect()
    }

    pub fn is_running(&self, plugin_id: &str) -> bool {
        let guard = self.active.lock().unwrap();
        guard.contains_key(plugin_id)
    }

    pub fn revoke(&self, plugin_id: &str) {
        let mut guard = self.active.lock().unwrap();
        if let Some(session) = guard.remove(plugin_id) {
            // 通知对应的 Store 终止执行
            // TODO: 实现 Store 级别的 cancel token
        }
    }
}
```

Dart 侧通过 `rustApi.pluginListActiveSessions()` 获取运行中列表，用于看板"运行中"状态判定。

**卸载后保证**：
- Vault 数据不受影响
- 主软件配置不受影响
- 审计日志保留（记录 `PluginUninstalled` 事件）
- 插件内存通过 Store drop 彻底清零
- 插件目录完全删除，无残留文件
