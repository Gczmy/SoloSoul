# 代码分析修复报告

> 最后更新：2026-06-02 18:26:53
> 当前分支：`master`（6e9979c）
> 修复轮次：1（初始分析）

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置                         | 描述                                           | 状态      |
|------|--------|------------|----------------------------------|------------------------------------------------|-----------|
| P001 | P0     | 编译错误   | `flutter/native/src/frb_generated.rs` | FRB 生成代码引用 `crate::plugin`，但 `plugin` 模块只在 `sandbox` feature 下启用，默认构建失败 | `[ ]` 待修复 |
| P002 | P0     | 漏洞       | `core/api/server.go:74-113`      | `authMiddleware` 已定义但**没有任何路由使用**，所有 API 端点公开可访问 | `[ ]` 待修复 |
| P003 | P0     | 编译错误   | `flutter/test/unit/presentation/utils/property_value_utils_test.dart` 等 | 测试引用已删除的 `wrapEveryNChars` 函数，Dart 测试编译失败 | `[ ]` 待修复 |
| P004 | P0     | 漏洞       | `core/ocr/paddle.go:138`         | `exec.CommandContext` 传入未经验证的 `pythonPath` 和外部文件路径，存在命令注入风险 | `[ ]` 待修复 |
| P005 | P1     | 安全       | `core/api/server.go:117`         | CORS 配置 `Access-Control-Allow-Origin: *` 过于宽松 | `[ ]` 待修复 |
| P006 | P1     | 安全       | `core/api/server.go:127-132`     | HTTP 服务器未设置 `MaxHeaderBytes` 和请求体限制，存在 DoS 风险 | `[ ]` 待修复 |
| P007 | P1     | 架构       | `lib/core/services/*`            | 多个服务层文件导入 `flutter/material.dart`，违反分层原则 | `[ ]` 待修复 |
| P008 | P1     | 架构       | `lib/core/services/operation_notification.dart:205` | 服务层中定义了 StatefulWidget `_NotificationWidget`，服务层与 UI 层严重耦合 | `[ ]` 待修复 |
| P009 | P1     | 功能缺陷   | `core/api/server.go:138-167`     | `StartUnix` 函数是未完成的存根，返回 nil 但未启动服务器 | `[ ]` 待修复 |
| P010 | P1     | 安全       | `core/ocr/paddle.go:263`         | 临时目录权限 `0755` 过于宽松，敏感图像数据应使用 `0700` | `[ ]` 待修复 |
| P011 | P1     | 安全       | `core/api/server.go:398-441`     | `handleChangePassword` 端点未进行密码强度验证和请求频率限制 | `[ ]` 待修复 |
| P012 | P2     | 代码质量   | `flutter/test/*`                 | 大量测试文件存在未使用变量、未使用导入、`const` 优化建议 | `[ ]` 待修复 |
| P013 | P2     | 代码质量   | `cmd/solosoul/main.go` 等        | 使用 `fmt.Println` 输出，应替换为结构化日志 | `[ ]` 待修复 |
| P014 | P2     | 代码质量   | `lib/presentation/pages/plugin_dashboard_page.dart` | 2581 行代码，文件过大，需要拆分 | `[ ]` 待修复 |
| P015 | P2     | 代码质量   | `core/api/server.go`             | 过度使用 `map[string]interface{}`，应定义具体结构体 | `[ ]` 待修复 |

## 修复进度

- 已完成：0 / 15
- 当前处理：无

## 详细问题描述与修复指引

### P001: FRB 编译错误（`sandbox` feature 未启用时 `plugin` 模块不可见）

**影响分析：**
`flutter/native/src/frb_generated.rs` 中有 117 处引用了 `crate::plugin`，但 `src/lib.rs` 中 `pub mod plugin` 被 `#[cfg(feature = "sandbox")]` 条件编译保护。默认情况下构建 Rust 库会失败。

**代码片段：**
```rust
// src/lib.rs:16-17
#[cfg(feature = "sandbox")]
pub mod plugin;
```

```rust
// src/frb_generated.rs:1044
crate::plugin::manager::PluginEvent,
```

**修复方案：**
- 方案A：在 `Cargo.toml` 的默认 features 中添加 `sandbox`（但可能影响 iOS 构建，因 wasmtime 依赖问题）
- 方案B：在 `frb_generated.rs` 中所有引用 `crate::plugin` 的地方添加 `#[cfg(feature = "sandbox")]` 条件编译
- 方案C：重新生成 FRB 绑定，确保不使用 sandbox-only 的类型

**推荐方案B**，因为 iOS 构建明确排除了 sandbox feature，FRB 生成代码应尊重这一约束。

---

### P002: `authMiddleware` 未应用到任何路由

**影响分析：**
`core/api/server.go:179-204` 定义了认证中间件，检查 `Authorization` Header 中的 Bearer Token。但 `Start()` 和 `StartUnix()` 中注册的所有路由都直接使用了 handler 函数，没有经过 `authMiddleware` 包装。

**代码片段：**
```go
// 定义了中间件，但从未使用
func (s *HTTPServer) authMiddleware(next http.HandlerFunc) http.HandlerFunc {
    // ...
}

// Start() 中直接注册，无认证保护
mux.HandleFunc("GET /api/accounts", s.handleAccountList)
mux.HandleFunc("DELETE /api/profile/{id}", s.handleProfileDelete)
// ... 所有路由均无认证
```

**修复方案：**
将需要保护的路由用 `authMiddleware` 包装：
```go
mux.HandleFunc("GET /api/accounts", s.authMiddleware(s.handleAccountList))
// 公共端点（如 /health、/api/auth/setup）除外
```

**注意：** 应先完成 P002 修复，因为它影响所有后续 API 安全修复的验证。

---

### P003: 测试引用已删除的函数

**影响分析：**
`lib/presentation/utils/property_value_utils.dart` 中的 `wrapEveryNChars` 函数已被删除（注释显示 "REMOVED"），但多个测试文件仍在引用该函数，导致 `dart analyze` 报 error。

**受影响的测试文件：**
- `flutter/test/unit/presentation/utils/property_value_utils_test.dart`
- `flutter/test/unit/property_value_utils_test.dart`
- `flutter/test/widget/presentation/widgets/object_card/object_card_properties_list_test.dart`

**修复方案：**
删除或更新这些测试，移除对 `wrapEveryNChars` 的引用。

---

### P004: OCR 引擎命令注入风险

**影响分析：**
`core/ocr/paddle.go:138` 使用 `exec.CommandContext` 执行 Python 脚本，`pythonPath` 来自构造函数参数且未经验证。如果 `pythonPath` 被恶意替换（如包含 `; rm -rf /`），可能导致命令注入。

**代码片段：**
```go
cmd := exec.CommandContext(ctx, p.pythonPath, "-c", script, imagePath, string(docType))
```

**修复方案：**
1. 对 `pythonPath` 进行白名单验证（只允许 `python3`、`python` 等已知路径）
2. 使用 `filepath.Clean` 清理 `imagePath`
3. 考虑使用 `exec.LookPath` 验证 `pythonPath` 存在性

---

### P005: CORS 配置过于宽松

**影响分析：**
`core/api/server.go:117` 设置 `Access-Control-Allow-Origin: *`，允许任意网站跨域访问 API。对于处理敏感个人数据的 SoloSoul 后端来说，这是不安全的。

**修复方案：**
将 `*` 替换为明确的允许来源列表，或根据环境变量配置：
```go
allowedOrigin := os.Getenv("SOLOSOUL_ALLOWED_ORIGIN")
if allowedOrigin == "" {
    allowedOrigin = "http://localhost:3000" // 默认开发环境
}
w.Header().Set("Access-Control-Allow-Origin", allowedOrigin)
```

---

### P006: HTTP 服务器缺少 DoS 防护

**影响分析：**
`http.Server` 未设置 `MaxHeaderBytes`（默认 1MB），也未限制请求体大小。恶意客户端可发送超大请求消耗服务器资源。

**修复方案：**
```go
s.server = &http.Server{
    Addr:           addr,
    Handler:        corsMux,
    ReadTimeout:    10 * time.Second,
    WriteTimeout:   10 * time.Second,
    MaxHeaderBytes: 1 << 20, // 1MB
}
```

同时各 Handler 应使用 `http.MaxBytesReader` 限制请求体：
```go
r.Body = http.MaxBytesReader(w, r.Body, 1<<20) // 1MB
```

---

### P007: 服务层导入 UI 库（分层架构违规）

**影响分析：**
以下服务层文件导入了 `flutter/material.dart`，违反了 Clean Architecture 的分层原则（服务层不应依赖 UI 层）：
- `lib/core/services/operation_notification.dart`
- `lib/core/services/attachment_download_service.dart`
- `lib/core/services/unified_object_service.dart`
- `lib/core/services/attachment_storage_service.dart`
- `lib/core/services/attachment_upload_service.dart`

**修复方案：**
- 移除服务层中对 `material.dart` 的依赖
- 将 UI 相关的逻辑（如显示 SnackBar、Dialog）提升到 Presentation 层
- 服务层应通过回调、Stream 或状态管理（Riverpod）与 UI 层通信

---

### P008: 服务层混入 UI Widget

**影响分析：**
`lib/core/services/operation_notification.dart:205` 定义了 `_NotificationWidget extends StatefulWidget`，直接在服务层中嵌入 UI 组件。

**修复方案：**
将 `_NotificationWidget` 迁移到 `lib/presentation/widgets/` 目录下，服务层只负责触发通知事件。

---

### P009: `StartUnix` 是未完成的存根

**影响分析：**
`core/api/server.go:138-167` 的 `StartUnix` 函数创建了路由但未启动服务器，直接返回 `nil`。调用者期望它启动一个 Unix Socket 服务器。

**代码片段：**
```go
func (s *HTTPServer) StartUnix(socketPath string) error {
    // ... 注册路由 ...
    s.server = &http.Server{Handler: mux}
    return nil // Let caller set up listener
}
```

**修复方案：**
完成函数实现，创建 Unix listener 并启动服务器：
```go
listener, err := net.Listen("unix", socketPath)
if err != nil { return err }
return s.server.Serve(listener)
```

---

### P010: 临时目录权限过于宽松

**影响分析：**
`core/ocr/paddle.go:263` 创建临时目录时使用 `0755` 权限，任何本地用户都可以读取 OCR 处理的敏感图像数据。

**修复方案：**
```go
os.MkdirAll(tmpDir, 0700) // 仅所有者可读写执行
```

---

### P011: 密码修改端点缺少后端验证

**影响分析：**
`handleChangePassword` 仅在前端检查密码长度（8 字符），后端没有二次验证。且没有请求频率限制，可能被暴力破解。

**修复方案：**
1. 后端添加密码强度验证（最小长度、复杂度）
2. 添加速率限制（如每分钟最多 5 次密码修改尝试）
3. 要求提供旧密码作为验证

---

### P012-P015: 代码质量优化项

详见上方问题清单，这些为 P2 优先级，可在 P0/P1 修复完成后处理。
