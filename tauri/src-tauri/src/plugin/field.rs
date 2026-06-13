//! 字段解析器占位实现
//!
//! Phase 2 仅保留基础设施，实际字段值解析将在后续接入 Vault 数据层后实现。

use super::PluginError;

/// 字段解析器
#[derive(Debug, Default, Clone)]
pub struct FieldResolver;

impl FieldResolver {
    /// 创建字段解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析字段值
    ///
    /// 当前为占位实现，始终返回空字符串，保证测试与最小链路可运行。
    pub fn resolve(&self, _field_id: &str) -> Result<String, PluginError> {
        Ok(String::new())
    }
}
