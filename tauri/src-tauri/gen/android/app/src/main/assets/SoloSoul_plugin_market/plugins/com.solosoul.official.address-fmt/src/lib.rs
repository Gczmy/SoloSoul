//! Address Formatter — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 通过 SDK `list_objects()` 批量获取 Vault 中的所有地址对象，
//! 在插件内部完成计数和属性提取。不再使用 .count 字段。

use solosoul_plugin_sdk::{get_param, list_objects, log_error, log_info, send_result_json, escape_json};

/// 国际化文本表
struct I18n {
    is_zh: bool,
}

impl I18n {
    fn new() -> Self {
        let locale = get_param("locale")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "en".to_string());
        Self {
            is_zh: locale.starts_with("zh"),
        }
    }

    fn log_start(&self) -> String {
        if self.is_zh {
            "Address Formatter 启动 — 格式化所有地址".to_string()
        } else {
            "Address Formatter started — formatting all addresses".to_string()
        }
    }

    fn log_list_ok(&self, len: usize) -> String {
        format!("list_objects OK, json length={}", len)
    }

    fn log_count(&self, count: usize) -> String {
        format!("address count={}", count)
    }

    fn log_found(&self, count: usize) -> String {
        if self.is_zh {
            format!("发现 {} 条地址", count)
        } else {
            format!("Found {} address{}", count, if count == 1 { "" } else { "es" })
        }
    }

    fn log_processing(&self, i: usize) -> String {
        if self.is_zh {
            format!("--- 处理地址[{}] ---", i)
        } else {
            format!("--- Processing address[{}] ---", i)
        }
    }

    fn log_address_fields(&self, i: usize, street: &str, city: &str, state: &str, postal: &str, country: &str, district: &str) -> String {
        if self.is_zh {
            format!(
                "地址[{}] street='{}' city='{}' state='{}' postal='{}' country='{}' district='{}'",
                i, street, city, state, postal, country, district
            )
        } else {
            format!(
                "address[{}] street='{}' city='{}' state='{}' postal='{}' country='{}' district='{}'",
                i, street, city, state, postal, country, district
            )
        }
    }

    fn log_country_recognized(&self, i: usize, country: &str, label: &str) -> String {
        if self.is_zh {
            format!("地址[{}] 国家识别: {} → {}", i, country, label)
        } else {
            format!("address[{}] country recognized: {} → {}", i, country, label)
        }
    }

    fn log_formatted(&self, i: usize, display_label: &str, formatted: &str) -> String {
        if display_label.is_empty() {
            if self.is_zh {
                format!("地址[{}] 格式化结果: {}", i, formatted)
            } else {
                format!("address[{}] formatted result: {}", i, formatted)
            }
        } else {
            if self.is_zh {
                format!("地址[{}] 格式化结果: {} | {}", i, display_label, formatted)
            } else {
                format!("address[{}] formatted result: {} | {}", i, display_label, formatted)
            }
        }
    }

    fn log_missing_fields(&self, i: usize, street: &str, city: &str, country: &str) -> String {
        if self.is_zh {
            format!("地址[{}] 缺少必需字段: street='{}' city='{}' country='{}'", i, street, city, country)
        } else {
            format!("address[{}] missing required fields: street='{}' city='{}' country='{}'", i, street, city, country)
        }
    }

    fn log_no_addresses(&self) -> String {
        if self.is_zh {
            "未找到任何地址记录，请先添加至少一条地址记录".to_string()
        } else {
            "No address records found. Please add at least one address.".to_string()
        }
    }

    fn log_complete(&self, success: usize, total: usize) -> String {
        if self.is_zh {
            format!("Address Formatter 完成 — 成功格式化 {} / {} 条地址", success, total)
        } else {
            format!("Address Formatter completed — successfully formatted {} / {} addresses", success, total)
        }
    }

    fn log_parse_error(&self, e: &serde_json::Error) -> String {
        if self.is_zh {
            format!("解析地址列表 JSON 失败: {:?}", e)
        } else {
            format!("Failed to parse address list JSON: {:?}", e)
        }
    }

    fn log_list_error(&self, e: &solosoul_plugin_sdk::PluginError) -> String {
        format!("list_objects('address') FAILED: {:?}", e)
    }

    fn log_send_result_failed(&self, code: i32) -> String {
        format!("solosoul_result send failed, error code: {}", code)
    }

    fn result_title(&self) -> &'static str {
        if self.is_zh {
            "地址格式化结果"
        } else {
            "Address Formatting Results"
        }
    }

    fn result_label(&self, i: usize, display_label: &str) -> String {
        if display_label.is_empty() {
            if self.is_zh {
                format!("地址 {}", i + 1)
            } else {
                format!("Address {}", i + 1)
            }
        } else {
            display_label.to_string()
        }
    }
}

/// 地址组件
struct Address {
    street: String,
    city: String,
    state: String,
    postal_code: String,
    country: String,
    district: String,
}

/// 国家代码标准化（2字母或3字母转内部标识）
fn normalize_country(code: &str) -> &'static str {
    let upper = code.to_uppercase();
    match upper.as_str() {
        "CN" | "CHN" | "中国" | "CHINA" => "CN",
        "US" | "USA" | "美国" | "UNITED STATES" | "AMERICA" => "US",
        "GB" | "GBR" | "UK" | "英国" | "UNITED KINGDOM" | "BRITAIN" => "GB",
        "JP" | "JPN" | "日本" | "JAPAN" => "JP",
        "DE" | "DEU" | "德国" | "GERMANY" => "DE",
        "FR" | "FRA" | "法国" | "FRANCE" => "FR",
        "CA" | "CAN" | "加拿大" | "CANADA" => "CA",
        "AU" | "AUS" | "澳大利亚" | "AUSTRALIA" => "AU",
        "SG" | "SGP" | "新加坡" | "SINGAPORE" => "SG",
        "KR" | "KOR" | "韩国" | "SOUTH KOREA" => "KR",
        _ => "DEFAULT",
    }
}

/// 按国家格式化地址
fn format_address(addr: &Address) -> String {
    match normalize_country(&addr.country) {
        "CN" => format_china(addr),
        "US" => format_us(addr),
        "GB" => format_gb(addr),
        "JP" => format_jp(addr),
        "DE" => format_de(addr),
        "FR" => format_fr(addr),
        "CA" => format_ca(addr),
        "AU" => format_au(addr),
        "SG" => format_sg(addr),
        "KR" => format_kr(addr),
        _ => format_default(addr),
    }
}

/// 中国格式：省 + 市 + 区 + 街道 + 邮编
fn format_china(addr: &Address) -> String {
    let mut parts = Vec::new();
    if !addr.state.is_empty() { parts.push(addr.state.clone()); }
    if !addr.city.is_empty() { parts.push(addr.city.clone()); }
    if !addr.district.is_empty() { parts.push(addr.district.clone()); }
    if !addr.street.is_empty() { parts.push(addr.street.clone()); }

    let mut result = parts.join("");
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" ({})", addr.postal_code));
    }
    result
}

/// 美国格式：Street, City, State ZIP
fn format_us(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 英国格式：Street, City, County POSTCODE
fn format_gb(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 日本格式：〒ZIP + 都道府县 + 市区町村 + 番地
fn format_jp(addr: &Address) -> String {
    let mut result = String::new();
    if !addr.postal_code.is_empty() {
        result.push_str(&format!("〒{}", addr.postal_code));
    }
    if !addr.state.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.state);
    }
    if !addr.city.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.city);
    }
    if !addr.street.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.street);
    }
    result
}

/// 德国格式：Street, ZIP City
fn format_de(addr: &Address) -> String {
    let mut result = addr.street.clone();
    let city_part = if !addr.postal_code.is_empty() && !addr.city.is_empty() {
        format!(", {} {}", addr.postal_code, addr.city)
    } else if !addr.city.is_empty() {
        format!(", {}", addr.city)
    } else {
        String::new()
    };
    result.push_str(&city_part);
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    result
}

/// 法国格式：Street, ZIP City
fn format_fr(addr: &Address) -> String {
    format_de(addr) // 法国与德国格式相同
}

/// 加拿大格式：Street, City, Province POSTCODE
fn format_ca(addr: &Address) -> String {
    format_us(addr) // 加拿大与美国格式相同
}

/// 澳大利亚格式：Street, City, State POSTCODE
fn format_au(addr: &Address) -> String {
    format_us(addr) // 澳大利亚与美国格式相同
}

/// 新加坡格式：Street, City POSTCODE
fn format_sg(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 韩国格式：省 + 市 + 区 + 街道 + 邮编
fn format_kr(addr: &Address) -> String {
    format_china(addr) // 韩国与中国格式逻辑相同
}

/// 默认格式（无国家匹配时）
fn format_default(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    if !addr.country.is_empty() {
        result.push_str(&format!(", {}", addr.country));
    }
    result
}

/// 插件入口
///
/// 通过 SDK list_objects() 批量获取所有地址对象，在插件内部完成计数和属性提取。
#[no_mangle]
pub extern "C" fn run() -> i32 {
    let i18n = I18n::new();

    log_info(&i18n.log_start());

    // 批量获取所有地址对象（Phase 5：替代 .count + 逐字段 N+1 读取）
    let json = match list_objects("address") {
        Ok(j) => j,
        Err(e) => {
            log_error(&i18n.log_list_error(&e));
            return -1;
        }
    };
    log_info(&i18n.log_list_ok(json.len()));

    // 本地解析 JSON，插件自行完成计数
    let objects: Vec<serde_json::Value> = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            log_error(&i18n.log_parse_error(&e));
            return -1;
        }
    };

    let count = objects.len();
    log_info(&i18n.log_count(count));

    if count == 0 {
        log_error(&i18n.log_no_addresses());
        return -1;
    }

    log_info(&i18n.log_found(count));
    let mut success_count = 0;
    let mut result_pairs: Vec<String> = Vec::new();

    for (i, obj) in objects.iter().enumerate() {
        let props = &obj["properties"];
        log_info(&i18n.log_processing(i));

        let street = props["street"].as_str().unwrap_or("").to_string();
        let city = props["city"].as_str().unwrap_or("").to_string();
        let district = props["district"].as_str().unwrap_or("").to_string();
        let state = props["state"].as_str().unwrap_or("").to_string();
        let postal_code = props["postalCode"].as_str().unwrap_or("").to_string();
        let country = props["country"].as_str().unwrap_or("").to_string();

        log_info(&i18n.log_address_fields(i, &street, &city, &state, &postal_code, &country, &district));

        if street.is_empty() || city.is_empty() || country.is_empty() {
            log_error(&i18n.log_missing_fields(i, &street, &city, &country));
            continue;
        }

        let addr = Address {
            street,
            city,
            state,
            postal_code,
            country: country.clone(),
            district,
        };

        let formatted = format_address(&addr);
        let country_label = normalize_country(&country);

        // name 来自 list_objects 返回的 JSON name 字段，替代 .__name__
        let display_label = obj["name"].as_str().unwrap_or("").to_string();

        log_info(&i18n.log_country_recognized(i, &country, country_label));
        log_info(&i18n.log_formatted(i, &display_label, &formatted));
        success_count += 1;

        // Phase 2: 收集结构化结果数据
        let label = i18n.result_label(i, &display_label);
        // 存储：label\tformatted\tcountry\tcountry_code
        result_pairs.push(format!("{}\t{}\t{}\t{}", label, formatted, country, country_label));
    }

    // Phase 2: 发送结构化结果（key_value 卡片，带 tag 元数据）
    if !result_pairs.is_empty() {
        let pairs_json: Vec<String> = result_pairs
            .iter()
            .map(|v| {
                let parts: Vec<&str> = v.splitn(4, '\t').collect();
                let key = parts.get(0).unwrap_or(&"Address");
                let value = parts.get(1).unwrap_or(&"");
                let country_name = parts.get(2).unwrap_or(&"");
                let country_code = parts.get(3).unwrap_or(&"");
                let tag_json = if !country_name.is_empty() {
                    format!(r#","tag":"{}","tagCode":"{}""#, escape_json(country_name), escape_json(country_code))
                } else {
                    String::new()
                };
                format!(r#"{{"key":"{}","value":"{}"{}}}"#, escape_json(key), escape_json(value), tag_json)
            })
            .collect();
        let json = format!(
            r#"{{"type":"key_value","title":"{}","pairs":[{}]}}"#,
            i18n.result_title(),
            pairs_json.join(",")
        );
        let ret = send_result_json(&json);
        if let Err(code) = ret {
            log_error(&i18n.log_send_result_failed(code));
            // 降级：用日志输出
            for v in &result_pairs {
                log_info(v);
            }
        }
    }

    log_info(&i18n.log_complete(success_count, count));

    if success_count == 0 {
        -1
    } else {
        0
    }
}

/// JSON 字符串转义（处理所有标准转义字符）


// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(street: &str, city: &str, state: &str, postal: &str, country: &str, district: &str) -> Address {
        Address {
            street: street.to_string(),
            city: city.to_string(),
            state: state.to_string(),
            postal_code: postal.to_string(),
            country: country.to_string(),
            district: district.to_string(),
        }
    }

    #[test]
    fn test_normalize_country() {
        assert_eq!(normalize_country("CN"), "CN");
        assert_eq!(normalize_country("chn"), "CN");
        assert_eq!(normalize_country("中国"), "CN");
        assert_eq!(normalize_country("US"), "US");
        assert_eq!(normalize_country("USA"), "US");
        assert_eq!(normalize_country("美国"), "US");
        assert_eq!(normalize_country("GB"), "GB");
        assert_eq!(normalize_country("UK"), "GB");
        assert_eq!(normalize_country("JP"), "JP");
        assert_eq!(normalize_country("日本"), "JP");
        assert_eq!(normalize_country("Unknown"), "DEFAULT");
    }

    #[test]
    fn test_format_china() {
        let a = addr("中关村大街1号", "北京市", "", "100080", "CN", "海淀区");
        assert_eq!(format_address(&a), "北京市海淀区中关村大街1号 (100080)");
    }

    #[test]
    fn test_format_us() {
        let a = addr("1600 Pennsylvania Avenue NW", "Washington", "DC", "20500", "US", "");
        assert_eq!(format_address(&a), "1600 Pennsylvania Avenue NW, Washington, DC 20500");
    }

    #[test]
    fn test_format_gb() {
        let a = addr("10 Downing Street", "London", "", "SW1A 2AA", "GB", "");
        assert_eq!(format_address(&a), "10 Downing Street, London SW1A 2AA");
    }

    #[test]
    fn test_format_jp() {
        let a = addr("1-1-2 丸の内", "千代田区", "東京都", "100-0001", "JP", "");
        assert_eq!(format_address(&a), "〒100-0001 東京都 千代田区 1-1-2 丸の内");
    }

    #[test]
    fn test_format_de() {
        let a = addr("Unter den Linden 1", "Berlin", "", "10117", "DE", "");
        assert_eq!(format_address(&a), "Unter den Linden 1, 10117 Berlin");
    }

    #[test]
    fn test_format_sg() {
        let a = addr("1 Raffles Place", "Singapore", "", "048616", "SG", "");
        assert_eq!(format_address(&a), "1 Raffles Place, Singapore 048616");
    }

    #[test]
    fn test_format_default() {
        let a = addr("Unknown St", "Mystery City", "", "", "XX", "");
        assert!(format_address(&a).contains("Unknown St"));
        assert!(format_address(&a).contains("XX"));
    }

    #[test]
    fn test_format_missing_optional() {
        let a = addr("Main St", "Springfield", "", "", "US", "");
        assert_eq!(format_address(&a), "Main St, Springfield");
    }
}
