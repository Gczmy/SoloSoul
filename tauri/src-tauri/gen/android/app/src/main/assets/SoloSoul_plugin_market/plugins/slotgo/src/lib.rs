//! SlotGo — UK Visa 预约时间查询助手
//!
//! 官方插件示例，展示 SoloSoul 插件系统的完整能力：
//! - 请求敏感字段（护照号码）→ 触发 Consent 弹窗
//! - 请求可选字段（姓名、邮箱）
//! - 代理 HTTP POST 请求（域名白名单限制）
//! - 降级容错：网络不可用返回模拟结果
//!
//! 编译：
//! ```bash
//! cargo build --target wasm32-wasip1 --release
//! ```
//! 产物：`target/wasm32-wasip1/release/slotgo.wasm`
//! 重命名：`cp target/wasm32-wasip1/release/slotgo.wasm plugin.wasm`

use solosoul_plugin_sdk::{get_field, get_timestamp, log_error, log_info, post_json, PluginError};

/// 插件入口函数
///
/// SoloSoul Host 通过 `run` 符号调用此函数。
/// 返回值 `0` 表示成功，非零表示插件自定义错误码。
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("SlotGo: 开始查询 UK Visa 可选预约时间");

    // 1. 获取护照号码（required field，会触发 Consent 弹窗）
    let passport = match get_field("travel.primary_passport.number") {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("获取护照号码失败: {:?}", e));
            return 1;
        }
    };
    let masked = if passport.len() > 4 {
        format!("...{}", &passport[passport.len() - 4..])
    } else {
        passport.clone()
    };
    log_info(&format!("护照号码已获取（末4位: {}）", masked));

    // 2. 获取可选字段
    let name = get_field("identity.full_name").unwrap_or_default();
    let email = get_field("identity.contact.emails").unwrap_or_default();
    if !name.is_empty() {
        log_info(&format!("姓名: {}", name));
    }

    // 3. 构造查询请求体
    let body = format!(
        r#"{{"passport":"{}","name":"{}","email":"{}","timestamp":{}}}"#,
        passport,
        name,
        email,
        get_timestamp()
    );

    // 4. 发送 POST 请求到预约查询服务
    match post_json("https://api.solosoul.io/slotgo/query", &body) {
        Ok(resp) => {
            log_info(&format!("预约查询成功: {}", resp));
            0
        }
        Err(PluginError::DomainNotAllowed) => {
            log_error("域名未授权，请在 manifest 中配置 allowed_domains");
            // 降级：返回模拟结果供演示
            log_info("降级模拟: 2026-06-15 09:00, 2026-06-15 14:00, 2026-06-16 09:00");
            0
        }
        Err(PluginError::NetworkTimeout) => {
            log_error("网络超时，请检查网络连接");
            // 降级：返回模拟结果供演示
            log_info("降级模拟: 2026-06-15 09:00, 2026-06-15 14:00, 2026-06-16 09:00");
            0
        }
        Err(e) => {
            log_error(&format!("预约查询失败: {:?}，降级为本地模拟", e));
            // 降级：返回模拟结果供演示
            log_info("降级模拟: 2026-06-15 09:00, 2026-06-15 14:00, 2026-06-16 09:00");
            0
        }
    }
}
