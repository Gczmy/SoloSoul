//! MRZ Encoder — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 将 Vault 中的护照信息编码为 ICAO Doc 9303 TD3 标准机读区格式。

use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, escape_json};

/// MRZ 编码结果
struct MrzResult {
    line1: String,
    line2: String,
}

/// 字符到 MRZ 数值映射
/// 0-9 → 0-9, A-Z → 10-35, < → 0
fn char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='Z' => c as u32 - 'A' as u32 + 10,
        '<' => 0,
        _ => 0, // 非法字符视为填充符
    }
}

/// 计算 MRZ 校验位
/// 权重循环 [7, 3, 1]
fn compute_check_digit(data: &str) -> char {
    let weights = [7, 3, 1];
    let mut sum = 0u32;
    for (i, c) in data.chars().enumerate() {
        sum += char_value(c) * weights[i % 3];
    }
    let digit = (sum % 10) as u8;
    (b'0' + digit) as char
}

/// 格式化姓名字段
/// 姓氏 << 名字，空格替换为 <，不足用 < 填充，超出截断
fn format_name(surname: &str, given_names: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(max_len);

    // 姓氏：空格/逗号替换为 <
    for c in surname.to_uppercase().chars() {
        if c == ' ' || c == ',' || c == '-' {
            result.push('<');
        } else if c.is_ascii_alphanumeric() {
            result.push(c);
        }
    }

    result.push('<');
    result.push('<');

    // 名字
    for c in given_names.to_uppercase().chars() {
        if c == ' ' || c == ',' || c == '-' {
            result.push('<');
        } else if c.is_ascii_alphanumeric() {
            result.push(c);
        }
    }

    // 截断或填充
    if result.len() > max_len {
        result.truncate(max_len);
    } else {
        while result.len() < max_len {
            result.push('<');
        }
    }

    result
}

/// 将日期从 YYYY-MM-DD 格式转换为 YYMMDD
fn to_mrz_date(date_str: &str) -> Result<String, &'static str> {
    let s = date_str.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        Ok(format!("{}{}{}", &s[2..4], &s[5..7], &s[8..10]))
    } else if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        // 已经是 YYMMDD 格式
        Ok(s.to_string())
    } else {
        Err("日期格式必须为 YYYY-MM-DD 或 YYMMDD")
    }
}

/// 格式化护照号码（9位，不足用 < 填充）
fn format_doc_number(doc_no: &str) -> String {
    let upper = doc_no.to_uppercase();
    let mut result = upper.chars().take(9).collect::<String>();
    while result.len() < 9 {
        result.push('<');
    }
    result
}

/// 格式化个人号码（14位，不足用 < 填充）
fn format_personal_number(pn: &str) -> String {
    let upper = pn.to_uppercase();
    let mut result = upper.chars().take(14).collect::<String>();
    while result.len() < 14 {
        result.push('<');
    }
    result
}

/// 编码 TD3（护照）MRZ
fn encode_td3(
    issuing_country: &str,
    surname: &str,
    given_names: &str,
    doc_number: &str,
    nationality: &str,
    dob: &str,
    sex: &str,
    expiry: &str,
    personal_number: &str,
) -> Result<MrzResult, &'static str> {
    // Line 1: P< + issuing_country(3) + name(39) = 44
    let country = issuing_country.to_uppercase();
    if country.len() != 3 {
        return Err("签发国代码必须为 3 位字母（如 CHN）");
    }

    let name_field = format_name(surname, given_names, 39);
    let line1 = format!("P<{}{}", country, name_field);
    debug_assert_eq!(line1.len(), 44, "Line 1 长度必须为 44");

    // Line 2
    let doc_num = format_doc_number(doc_number);
    let doc_check = compute_check_digit(&doc_num);

    let nat = nationality.to_uppercase();
    if nat.len() != 3 {
        return Err("国籍代码必须为 3 位字母（如 CHN）");
    }

    let dob_mrz = to_mrz_date(dob)?;
    let dob_check = compute_check_digit(&dob_mrz);

    let sex_mrz = match sex.to_uppercase().as_str() {
        "M" | "MALE" => "M",
        "F" | "FEMALE" => "F",
        _ => "<",
    };

    let exp_mrz = to_mrz_date(expiry)?;
    let exp_check = compute_check_digit(&exp_mrz);

    let pers_num = format_personal_number(personal_number);
    let pers_check = compute_check_digit(&pers_num);

    // Composite check digit over:
    // doc_num(9) + doc_check(1) + dob(6) + dob_check(1) + expiry(6) + exp_check(1) + pers_num(14) + pers_check(1) = 39 chars
    let composite_data = format!(
        "{}{}{}{}{}{}{}{}",
        doc_num, doc_check, dob_mrz, dob_check, exp_mrz, exp_check, pers_num, pers_check
    );
    let composite_check = compute_check_digit(&composite_data);

    let line2 = format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        doc_num, doc_check, nat, dob_mrz, dob_check, sex_mrz, exp_mrz, exp_check, pers_num, pers_check, composite_check
    );

    // 长度校验
    if line2.len() != 44 {
        return Err("Line 2 长度异常");
    }

    Ok(MrzResult { line1, line2 })
}

/// 脱敏显示 MRZ（隐藏敏感字符）
fn mask_mrz(result: &MrzResult) -> String {
    format!(
        "{}\n{}{}",
        &result.line1[..6],
        &result.line2[..9],
        "*".repeat(result.line2.len() - 9)
    )
}

/// 简单的 JSON 字符串转义


/// 插件入口
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("MRZ Encoder 启动 — 编码护照 MRZ");

    // 读取所有必需字段
    let fields = [
        ("passport.number", "护照号码"),
        ("passport.nationality", "国籍"),
        ("passport.surname", "姓氏"),
        ("passport.givenNames", "名字"),
        ("passport.dateOfBirth", "出生日期"),
        ("passport.sex", "性别"),
        ("passport.expiryDate", "有效期"),
    ];

    let mut values: Vec<Option<String>> = Vec::with_capacity(fields.len());
    let mut missing = Vec::new();

    for (path, name) in &fields {
        match get_field(path) {
            Ok(v) if !v.trim().is_empty() => values.push(Some(v)),
            _ => {
                values.push(None);
                missing.push(*name);
            }
        }
    }

    if !missing.is_empty() {
        let msg = format!(
            "缺少以下必需字段：{}\n\n请在 passport 对象中补充这些字段后重试。",
            missing.join("、")
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"字段缺失","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -1;
    }

    // 提取并清理值
    let doc_number = values[0].as_ref().unwrap().trim();
    let nationality = values[1].as_ref().unwrap().trim();
    let surname = values[2].as_ref().unwrap().trim();
    let given_names = values[3].as_ref().unwrap().trim();
    let dob = values[4].as_ref().unwrap().trim();
    let sex = values[5].as_ref().unwrap().trim();
    let expiry = values[6].as_ref().unwrap().trim();

    // 检查空值字段（字段存在但内容为空）
    let mut empty_fields: Vec<(&str, &str)> = Vec::new();
    if doc_number.is_empty() { empty_fields.push(("passport.number", "护照号码")); }
    if nationality.is_empty() { empty_fields.push(("passport.nationality", "国籍")); }
    if surname.is_empty() { empty_fields.push(("passport.surname", "姓氏")); }
    if given_names.is_empty() { empty_fields.push(("passport.givenNames", "名字")); }
    if dob.is_empty() { empty_fields.push(("passport.dateOfBirth", "出生日期")); }
    if sex.is_empty() { empty_fields.push(("passport.sex", "性别")); }
    if expiry.is_empty() { empty_fields.push(("passport.expiryDate", "有效期")); }

    if !empty_fields.is_empty() {
        let names: Vec<&str> = empty_fields.iter().map(|(_, name)| *name).collect();
        let paths: Vec<&str> = empty_fields.iter().map(|(path, _)| *path).collect();
        let msg = format!(
            "以下字段内容为空：{}\n\n字段路径：{}\n\n请检查 passport 中这些字段是否已填写。",
            names.join("、"),
            paths.join("、")
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"字段为空","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -1;
    }

    // ── 字段格式预验证 ──────────────────────────────────────────────

    // 1. 护照号码验证
    if doc_number.len() > 9 {
        let msg = format!(
            "护照号码(number)过长。\n\n当前值：'{}'（{}位）\nMRZ 格式要求护照号码最多 9 位，超出部分将被截断。\n\n建议修改为不超过 9 位的号码，例如：E12345678",
            doc_number,
            doc_number.len()
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"护照号码过长","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -2;
    }

    // 2. 国籍验证（必须为3位 ISO 代码）
    if nationality.len() != 3 || !nationality.chars().all(|c| c.is_ascii_alphabetic()) {
        let msg = format!(
            "国籍(nationality)格式错误。\n\n当前值：'{}'\n无法识别为有效的 ISO 3166-1 alpha-3 三位字母代码。\n\n请在 passport 中将 nationality 字段修改为 ISO 代码，例如：\n- CHN（中国）\n- USA（美国）\n- GBR（英国）\n- JPN（日本）\n- KOR（韩国）\n- CAN（加拿大）\n- AUS（澳大利亚）\n\n如果您输入的是中文国家名称（如「中国」），SoloSoul 会在宿主层尝试自动转换，但如果转换失败，请手动修改为 ISO 代码。",
            nationality
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"国籍格式错误","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -2;
    }

    // 3. 出生日期验证
    if let Err(e) = to_mrz_date(dob) {
        let msg = format!(
            "出生日期(dateOfBirth)格式错误。\n\n当前值：'{}'\n错误原因：{}\n\n支持的格式：\n- YYYY-MM-DD（推荐），例如：1990-05-23\n- YYMMDD，例如：900523\n\n请检查 passport 中的 dateOfBirth 字段格式。",
            dob, e
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"日期格式错误","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -2;
    }

    // 4. 有效期验证
    if let Err(e) = to_mrz_date(expiry) {
        let msg = format!(
            "有效期(expiryDate)格式错误。\n\n当前值：'{}'\n错误原因：{}\n\n支持的格式：\n- YYYY-MM-DD（推荐），例如：2031-07-14\n- YYMMDD，例如：310714\n\n请检查 passport 中的 expiryDate 字段格式。",
            expiry, e
        );
        log_error(&msg);
        let err_json = format!(
            r#"{{"type":"error","title":"日期格式错误","message":"{}"}}"#,
            escape_json(&msg)
        );
        let _ = send_result_json(&err_json);
        return -2;
    }

    // 签发国默认使用国籍（大多数情况）
    let issuing_country = nationality;

    // 可选字段：个人号码
    let personal_number = get_field("passport.personalNumber")
        .unwrap_or_default();

    // ── MRZ 编码 ────────────────────────────────────────────────────
    match encode_td3(
        issuing_country,
        surname,
        given_names,
        doc_number,
        nationality,
        dob,
        sex,
        expiry,
        &personal_number,
    ) {
        Ok(result) => {
            log_info("MRZ 编码成功:");
            log_info(&result.line1);
            log_info(&result.line2);
            log_info(&format!("脱敏预览:\n{}", mask_mrz(&result)));

            // Phase 2: 结构化结果
            let pairs_json = vec![
                format!(r#"{{"key":"MRZ 行 1","value":"{}"}}"#, escape_json(&result.line1)),
                format!(r#"{{"key":"MRZ 行 2","value":"{}"}}"#, escape_json(&result.line2)),
                format!(r#"{{"key":"脱敏预览","value":"{}"}}"#, escape_json(&mask_mrz(&result))),
            ];
            let result_json = format!(r#"{{"type":"key_value","title":"MRZ 编码","pairs":[{}]}}"#, pairs_json.join(","));
            let _ = send_result_json(&result_json);

            0
        }
        Err(e) => {
            let msg = format!(
                "MRZ 编码失败：{}\n\n所有字段格式已通过初步验证，但在最终编码阶段仍出现错误。\n请检查护照信息是否符合 ICAO Doc 9303 标准。\n\n常见问题：\n- 护照号码包含非法字符（仅支持字母数字）\n- 姓名包含 MRZ 不支持的字符\n- 日期计算异常",
                e
            );
            log_error(&msg);
            let err_json = format!(
                r#"{{"type":"error","title":"MRZ 编码失败","message":"{}"}}"#,
                escape_json(&msg)
            );
            let _ = send_result_json(&err_json);
            -2
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_value() {
        assert_eq!(char_value('0'), 0);
        assert_eq!(char_value('9'), 9);
        assert_eq!(char_value('A'), 10);
        assert_eq!(char_value('Z'), 35);
        assert_eq!(char_value('<'), 0);
    }

    #[test]
    fn test_compute_check_digit() {
        // ICAO 9303 校验位算法验证
        // "123456789": 1*7+2*3+3*1+4*7+5*3+6*1+7*7+8*3+9*1 = 147 → 7
        assert_eq!(compute_check_digit("123456789"), '7');
        // "ABCD": 10*7+11*3+12*1+13*7 = 206 → 6
        assert_eq!(compute_check_digit("ABCD"), '6');
        // "<<": 0*7+0*3 = 0 → 0
        assert_eq!(compute_check_digit("<<"), '0');
        // "": sum=0 → 0
        assert_eq!(compute_check_digit(""), '0');
    }

    #[test]
    fn test_format_name() {
        assert_eq!(
            format_name("ZHANG", "SAN", 39),
            "ZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
        );
        assert_eq!(
            format_name("VAN DER", "JOHN PAUL", 39),
            "VAN<DER<<JOHN<PAUL<<<<<<<<<<<<<<<<<<<<<"
        );
    }

    #[test]
    fn test_to_mrz_date() {
        assert_eq!(to_mrz_date("1990-05-23").unwrap(), "900523");
        assert_eq!(to_mrz_date("2000-01-01").unwrap(), "000101");
        assert_eq!(to_mrz_date("900523").unwrap(), "900523");
        assert!(to_mrz_date("invalid").is_err());
    }

    #[test]
    fn test_format_doc_number() {
        assert_eq!(format_doc_number("E12345678"), "E12345678");
        assert_eq!(format_doc_number("ABC"), "ABC<<<<<<");
    }

    #[test]
    fn test_encode_td3_basic() {
        let result = encode_td3(
            "CHN", "ZHANG", "SAN",
            "E12345678", "CHN", "1990-05-23", "M", "2025-12-31", ""
        ).unwrap();

        assert_eq!(result.line1.len(), 44);
        assert!(result.line1.starts_with("P<CHNZHANG<<SAN"));

        assert_eq!(result.line2.len(), 44);
        assert!(result.line2.starts_with("E12345678"));
    }

    #[test]
    fn test_encode_td3_with_personal_number() {
        let result = encode_td3(
            "CHN", "LI", "SI",
            "G87654321", "CHN", "1985-10-10", "F", "2030-06-15", "ID123456"
        ).unwrap();

        assert_eq!(result.line1.len(), 44);
        assert_eq!(result.line2.len(), 44);
    }

    #[test]
    fn test_mask_mrz() {
        let result = MrzResult {
            line1: "P<CHNZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<".to_string(),
            line2: "E123456780CHN9005237M2512317<<<<<<<<<<04".to_string(),
        };
        let masked = mask_mrz(&result);
        assert!(masked.contains("P<CHNZ"));
        assert!(masked.contains("E12345678"));
        assert!(masked.contains("*********"));
    }
}
