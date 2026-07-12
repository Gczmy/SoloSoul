//! ID Validator — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 读取 Vault 中的证件号码，校验其格式与校验位合法性。

use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, escape_json};

/// 中国居民身份证 18 位校验
///
/// 算法（GB 11643-1999）：
/// 1. 前 17 位分别乘以权重 [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]
/// 2. 求和对 11 取模
/// 3. 余数映射到校验码 [1,0,'X',9,8,7,6,5,4,3,2]
fn validate_cn_id(id: &str) -> Result<bool, &'static str> {
    if id.len() != 18 {
        return Err("中国身份证必须为 18 位");
    }

    // 检查前 17 位是否为数字
    let prefix = &id[..17];
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return Err("前 17 位必须全为数字");
    }

    // 检查第 18 位：数字或 X/x
    let check_char = id.as_bytes()[17] as char;
    if !check_char.is_ascii_digit() && !matches!(check_char, 'X' | 'x') {
        return Err("第 18 位必须为数字或 X");
    }

    const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    const CHECK_CODES: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

    let mut sum: u32 = 0;
    for (i, ch) in prefix.chars().enumerate() {
        let digit = (ch as u8 - b'0') as u32;
        sum += digit * WEIGHTS[i];
    }

    let expected = CHECK_CODES[(sum % 11) as usize];
    let actual = check_char.to_ascii_uppercase();

    Ok(expected == actual)
}

/// 护照号码基础格式校验
///
/// 规则：允许字母和数字，长度 6-12 位（覆盖多国护照常见范围）
/// 中国护照：E + 8 位数字 或 9 位字母数字
fn validate_passport_number(num: &str) -> &'static str {
    if num.len() < 6 || num.len() > 12 {
        return "长度异常（应为 6-12 位）";
    }
    if !num.chars().all(|c| c.is_ascii_alphanumeric()) {
        return "包含非法字符（仅允许字母和数字）";
    }
    ""
}

/// 简单的 JSON 字符串转义


#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("ID Validator 启动");

    // 1. 读取身份证字段（容错：错误视为空值，不提前退出）
    let id_card = match get_field("idCard.number") {
        Ok(v) if !v.trim().is_empty() => {
            log_info(&format!("读取到 idCard.number: {}", mask_id(&v)));
            v.trim().to_string()
        }
        Ok(_) => {
            log_info("idCard.number 为空，跳过身份证校验");
            String::new()
        }
        Err(e) => {
            log_info(&format!("idCard.number 不可用（可能无身份证数据）: {:?}", e));
            String::new()
        }
    };

    // 2. 校验身份证
    let mut id_status = "skipped";
    if !id_card.is_empty() {
        match validate_cn_id(&id_card) {
            Ok(true) => {
                log_info("✅ 身份证校验通过");
                id_status = "✅ 通过";
            }
            Ok(false) => {
                log_error("❌ 身份证校验失败：校验位不匹配");
                id_status = "❌ 校验位不匹配";
            }
            Err(msg) => {
                log_error(&format!("❌ 身份证格式错误: {}", msg));
                id_status = "❌ 格式错误";
            }
        }
    }

    // 3. 读取护照字段（容错：错误视为空值）
    let passport = match get_field("passport.number") {
        Ok(v) if !v.trim().is_empty() => {
            log_info(&format!("读取到 passport.number: {}", mask_id(&v)));
            v.trim().to_string()
        }
        Ok(_) => {
            log_info("passport.number 为空，跳过护照校验");
            String::new()
        }
        Err(e) => {
            log_info(&format!("passport.number 不可用（可能无护照数据）: {:?}", e));
            String::new()
        }
    };

    // 4. 校验护照格式
    let mut passport_status = "skipped";
    if !passport.is_empty() {
        let err = validate_passport_number(&passport);
        if err.is_empty() {
            log_info("✅ 护照号码格式正常");
            passport_status = "✅ 格式正常";
        } else {
            log_error(&format!("⚠️ 护照号码格式异常: {}", err));
            passport_status = "⚠️ 格式异常";
        }
    }

    log_info("ID Validator 执行完毕");

    // Phase 2: 结构化结果（始终返回，包含所有字段状态）
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !id_card.is_empty() {
        pairs.push(("证件类型", "身份证".to_string()));
        pairs.push(("脱敏号码", mask_id(&id_card)));
        pairs.push(("校验结果", id_status.to_string()));
    } else {
        pairs.push(("身份证", "无数据".to_string()));
    }
    if !passport.is_empty() {
        pairs.push(("护照号码", mask_id(&passport)));
        pairs.push(("护照格式", passport_status.to_string()));
    } else {
        pairs.push(("护照", "无数据".to_string()));
    }
    let pairs_json: Vec<String> = pairs.iter().map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"证件校验","pairs":[{}]}}"#, pairs_json.join(","));
    let _ = send_result_json(&result_json);

    0
}

/// 脱敏显示：保留前 3 位和后 2 位，中间用 * 替换
fn mask_id(id: &str) -> String {
    if id.len() <= 5 {
        return "***".to_string();
    }
    let prefix = &id[..3];
    let suffix = &id[id.len() - 2..];
    format!("{}****{}", prefix, suffix)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cn_id() {
        // 这是一个结构有效的身份证号码（仅用于校验位算法测试）
        // 注意：此号码为随机生成，不对应任何真实个人
        let valid_id = "110101199001011237";
        assert!(validate_cn_id(valid_id).unwrap());
    }

    #[test]
    fn test_invalid_cn_id_wrong_check() {
        let invalid_id = "110101199001011230"; // 最后一位应为 X
        assert!(!validate_cn_id(invalid_id).unwrap());
    }

    #[test]
    fn test_invalid_cn_id_length() {
        assert!(validate_cn_id("123456").is_err());
        assert!(validate_cn_id("1234567890123456789").is_err());
    }

    #[test]
    fn test_invalid_cn_id_non_digit() {
        assert!(validate_cn_id("abcdefghijklmnopqr").is_err());
    }

    #[test]
    fn test_passport_validation() {
        assert!(validate_passport_number("E12345678").is_empty());
        assert!(validate_passport_number("ABC123").is_empty());
        assert!(!validate_passport_number("ABC").is_empty());      // 太短
        assert!(!validate_passport_number("ABC1234567890").is_empty()); // 太长
        assert!(!validate_passport_number("ABC-123").is_empty());   // 含非法字符
    }

    #[test]
    fn test_mask_id() {
        assert_eq!(mask_id("123456789012345678"), "123****78");
        assert_eq!(mask_id("abc"), "***");
    }
}
