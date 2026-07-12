//! Travel Footprint — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 分析 Vault 中的旅行记录，生成到访国家统计和分类报告。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, list_objects, log_error, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 国家到大洲的简化映射表
fn country_to_continent(country: &str) -> &'static str {
    let c = country.trim().to_lowercase();
    match c.as_str() {
        // 亚洲
        "中国" | "china" | "cn" | "chn" |
        "日本" | "japan" | "jp" | "jpn" |
        "韩国" | "south korea" | "kr" | "kor" |
        "新加坡" | "singapore" | "sg" | "sgp" |
        "泰国" | "thailand" | "th" | "tha" |
        "马来西亚" | "malaysia" | "my" | "mys" |
        "印度尼西亚" | "indonesia" | "id" | "idn" |
        "越南" | "vietnam" | "vn" | "vnm" |
        "菲律宾" | "philippines" | "ph" | "phl" |
        "印度" | "india" | "in" | "ind" |
        "阿联酋" | "united arab emirates" | "ae" | "are" |
        "卡塔尔" | "qatar" | "qa" | "qat" |
        "沙特阿拉伯" | "saudi arabia" | "sa" | "sau" |
        "以色列" | "israel" | "il" | "isr" |
        "土耳其" | "turkey" | "tr" | "tur" |
        "哈萨克斯坦" | "kazakhstan" | "kz" | "kaz" => "亚洲",

        // 欧洲
        "英国" | "united kingdom" | "uk" | "gb" | "gbr" |
        "法国" | "france" | "fr" | "fra" |
        "德国" | "germany" | "de" | "deu" |
        "意大利" | "italy" | "it" | "ita" |
        "西班牙" | "spain" | "es" | "esp" |
        "荷兰" | "netherlands" | "nl" | "nld" |
        "比利时" | "belgium" | "be" | "bel" |
        "瑞士" | "switzerland" | "ch" | "che" |
        "奥地利" | "austria" | "at" | "aut" |
        "瑞典" | "sweden" | "se" | "swe" |
        "挪威" | "norway" | "no" | "nor" |
        "丹麦" | "denmark" | "dk" | "dnk" |
        "芬兰" | "finland" | "fi" | "fin" |
        "俄罗斯" | "russia" | "ru" | "rus" |
        "波兰" | "poland" | "pl" | "pol" |
        "捷克" | "czech republic" | "cz" | "cze" |
        "希腊" | "greece" | "gr" | "grc" |
        "葡萄牙" | "portugal" | "pt" | "prt" |
        "爱尔兰" | "ireland" | "ie" | "irl" => "欧洲",

        // 北美
        "美国" | "united states" | "usa" | "us" |
        "加拿大" | "canada" | "ca" | "can" |
        "墨西哥" | "mexico" | "mx" | "mex" => "北美洲",

        // 大洋洲
        "澳大利亚" | "australia" | "au" | "aus" |
        "新西兰" | "new zealand" | "nz" | "nzl" |
        "斐济" | "fiji" | "fj" | "fji" => "大洋洲",

        // 南美
        "巴西" | "brazil" | "br" | "bra" |
        "阿根廷" | "argentina" | "ar" | "arg" |
        "智利" | "chile" | "cl" | "chl" |
        "秘鲁" | "peru" | "pe" | "per" |
        "哥伦比亚" | "colombia" | "co" | "col" => "南美洲",

        // 非洲
        "埃及" | "egypt" | "eg" | "egy" |
        "南非" | "south africa" | "za" | "zaf" |
        "摩洛哥" | "morocco" | "ma" | "mar" |
        "肯尼亚" | "kenya" | "ke" | "ken" |
        "尼日利亚" | "nigeria" | "ng" | "nga" |
        "埃塞俄比亚" | "ethiopia" | "et" | "eth" => "非洲",

        _ => "其他",
    }
}

/// 解析访问国家列表（支持逗号/顿号/换行/分号分隔）
fn parse_countries(input: &str) -> Vec<String> {
    let mut normalized = input.to_string();
    for delim in ['、', '，', ';', '；', '\n'] {
        normalized = normalized.replace(delim, ",");
    }
    normalized
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 生成旅行统计报告
fn analyze_travel(countries_str: &str, nationality: &str, visa_count: &str) -> String {
    let countries = parse_countries(countries_str);
    let total = countries.len();

    let mut asia = Vec::new();
    let mut europe = Vec::new();
    let mut north_america = Vec::new();
    let mut oceania = Vec::new();
    let mut south_america = Vec::new();
    let mut africa = Vec::new();
    let mut other = Vec::new();

    for country in &countries {
        match country_to_continent(country) {
            "亚洲" => asia.push(country.clone()),
            "欧洲" => europe.push(country.clone()),
            "北美洲" => north_america.push(country.clone()),
            "大洋洲" => oceania.push(country.clone()),
            "南美洲" => south_america.push(country.clone()),
            "非洲" => africa.push(country.clone()),
            _ => other.push(country.clone()),
        }
    }

    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║         🌍 TRAVEL FOOTPRINT          ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());

    if !nationality.is_empty() {
        lines.push(format!("║ 国籍: {:<32} ║", truncate(nationality, 32)));
    }
    lines.push(format!("║ 到访国家数: {:<26} ║", total));

    if !visa_count.is_empty() {
        lines.push(format!("║ 签证数量: {:<28} ║", visa_count));
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    // 按大洲统计
    let regions = [
        ("亚洲", &asia),
        ("欧洲", &europe),
        ("北美洲", &north_america),
        ("大洋洲", &oceania),
        ("南美洲", &south_america),
        ("非洲", &africa),
        ("其他", &other),
    ];

    for (name, list) in &regions {
        if !list.is_empty() {
            let countries_str = list.join(", ");
            lines.push(format!("║ {} ({}):", name, list.len()));
            // 分行显示国家列表
            for chunk in chunk_string(&countries_str, 34) {
                lines.push(format!("║   {:<34} ║", chunk));
            }
        }
    }

    if total == 0 {
        lines.push("║ 暂无旅行记录。                       ║".to_string());
    }

    lines.push("╚══════════════════════════════════════╝".to_string());

    lines.join("\n")
}

/// 截断字符串


/// 将长字符串分块（按字符数）
fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|c| c.iter().collect())
        .collect()
}

/// 简单的 JSON 字符串转义


#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Travel Footprint 启动 — 分析旅行足迹");

    let countries = match get_field("travel.visitedCountries") {
        Ok(v) if !v.trim().is_empty() => v,
        Ok(_) => {
            log_error("旅行记录为空");
            let result_json = r#"{"type":"text","status":"empty_travel","action":"fill_visited_countries","content":"旅行记录为空。请在 Vault 的「出行记录」中填写 visitedCountries 字段（如：中国,日本,美国）。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
        Err(e) => {
            log_error(&format!("获取旅行记录失败: {:?}", e));
            let result_json = r#"{"type":"text","status":"no_travel","action":"create_travel_object","content":"未找到旅行记录。请在 Vault 中创建「出行记录」分区，并添加 visitedCountries 字段。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
    };

    let nationality = get_field("passport.nationality").unwrap_or_default();
    let visa_count = list_objects("visa")
        .ok()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(&j).ok())
        .map(|v| v.len().to_string())
        .unwrap_or_default();

    let report = analyze_travel(&countries, &nationality, &visa_count);
    for line in report.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let parsed = parse_countries(&countries);
    let total = parsed.len();

    let mut asia = Vec::new();
    let mut europe = Vec::new();
    let mut north_america = Vec::new();
    let mut oceania = Vec::new();
    let mut south_america = Vec::new();
    let mut africa = Vec::new();
    let mut other = Vec::new();

    for country in &parsed {
        match country_to_continent(country) {
            "亚洲" => asia.push(country.clone()),
            "欧洲" => europe.push(country.clone()),
            "北美洲" => north_america.push(country.clone()),
            "大洋洲" => oceania.push(country.clone()),
            "南美洲" => south_america.push(country.clone()),
            "非洲" => africa.push(country.clone()),
            _ => other.push(country.clone()),
        }
    }

    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !nationality.is_empty() { pairs.push(("国籍", nationality.clone())); }
    pairs.push(("到访国家数", total.to_string()));
    if !visa_count.is_empty() { pairs.push(("签证数量", visa_count.clone())); }
    if !asia.is_empty() { pairs.push(("亚洲", asia.join(", "))); }
    if !europe.is_empty() { pairs.push(("欧洲", europe.join(", "))); }
    if !north_america.is_empty() { pairs.push(("北美洲", north_america.join(", "))); }
    if !oceania.is_empty() { pairs.push(("大洋洲", oceania.join(", "))); }
    if !south_america.is_empty() { pairs.push(("南美洲", south_america.join(", "))); }
    if !africa.is_empty() { pairs.push(("非洲", africa.join(", "))); }
    if !other.is_empty() { pairs.push(("其他", other.join(", "))); }

    let pairs_json: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v)))
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"旅行足迹","pairs":[{}],"text":"{}"}}"#,
        pairs_json.join(","),
        escape_json(&report)
    );
    let _ = send_result_json(&result_json);

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_countries() {
        assert_eq!(parse_countries("中国,日本,韩国"), vec!["中国", "日本", "韩国"]);
        assert_eq!(parse_countries("中国、日本、韩国"), vec!["中国", "日本", "韩国"]);
        assert_eq!(
            parse_countries("中国\n日本\n韩国"),
            vec!["中国", "日本", "韩国"]
        );
        assert_eq!(parse_countries("  中国  ,  日本  "), vec!["中国", "日本"]);
    }

    #[test]
    fn test_country_to_continent() {
        assert_eq!(country_to_continent("中国"), "亚洲");
        assert_eq!(country_to_continent("China"), "亚洲");
        assert_eq!(country_to_continent("CN"), "亚洲");
        assert_eq!(country_to_continent("美国"), "北美洲");
        assert_eq!(country_to_continent("France"), "欧洲");
        assert_eq!(country_to_continent("澳大利亚"), "大洋洲");
        assert_eq!(country_to_continent("巴西"), "南美洲");
        assert_eq!(country_to_continent("埃及"), "非洲");
        assert_eq!(country_to_continent("未知国"), "其他");
    }

    #[test]
    fn test_analyze_travel() {
        let report = analyze_travel("中国,日本,美国,法国,澳大利亚", "中国", "3");
        assert!(report.contains("TRAVEL FOOTPRINT"));
        assert!(report.contains("5")); // 国家数
        assert!(report.contains("亚洲"));
        assert!(report.contains("北美洲"));
        assert!(report.contains("欧洲"));
        assert!(report.contains("大洋洲"));
    }

    #[test]
    fn test_chunk_string() {
        let chunks = chunk_string("这是一个很长的字符串用于测试分块功能", 10);
        assert_eq!(chunks.len(), 2); // 18 chars / 10 = 2 chunks
        assert_eq!(chunks[0].chars().count(), 10);
    }
}
