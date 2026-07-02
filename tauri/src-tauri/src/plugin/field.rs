//! 字段解析器重新导出
//!
//! 公共实现位于 `solosoul-plugin` crate，避免 `solo_soul` 与插件运行时各自维护一份
//! 相同逻辑。Tauri 侧直接复用插件 crate 的 `FieldResolver`。

pub use solosoul_plugin::FieldResolver;
