//! Packing List Generator — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 根据目的地和季节智能推荐旅行打包清单。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 行李项类别
struct PackingCategory {
    name: &'static str,
    icon: &'static str,
    items: Vec<String>,
}

/// 根据目的地推断气候特征
fn infer_climate(destination: &str) -> &'static str {
    let d = destination.to_lowercase();
    if d.contains("热带") || d.contains("tropical") || d.contains("thailand") || d.contains("bali")
        || d.contains("singapore") || d.contains("malaysia") || d.contains("三亚") || d.contains("泰国")
    {
        "tropical"
    } else if d.contains("寒带") || d.contains("cold") || d.contains("arctic") || d.contains("antarctica")
        || d.contains("iceland") || d.contains("norway") || d.contains("芬兰") || d.contains("冰岛")
        || d.contains("siberia") || d.contains("阿拉斯加")
    {
        "cold"
    } else if d.contains("沙漠") || d.contains("desert") || d.contains("dubai") || d.contains("迪拜")
        || d.contains("saudi") || d.contains("沙特") || d.contains("埃及") || d.contains("摩洛哥")
    {
        "desert"
    } else {
        "temperate"
    }
}

/// 根据季节调整
fn infer_season(season_input: &str) -> &'static str {
    let s = season_input.to_lowercase();
    if s.contains("夏") || s.contains("summer") {
        "summer"
    } else if s.contains("冬") || s.contains("winter") {
        "winter"
    } else if s.contains("春") || s.contains("spring") {
        "spring"
    } else if s.contains("秋") || s.contains("autumn") || s.contains("fall") {
        "autumn"
    } else {
        "any"
    }
}

/// 生成行李清单
fn generate_packing_list(destination: &str, duration: &str, season_input: &str, has_passport: bool, has_visa: bool) -> Vec<PackingCategory> {
    let climate = infer_climate(destination);
    let season = infer_season(season_input);

    let mut categories = Vec::new();

    // 证件
    let mut docs = vec!["身份证".to_string()];
    if has_passport { docs.push("护照".to_string()); }
    if has_visa { docs.push("签证".to_string()); }
    docs.push("机票/车票预订确认".to_string());
    docs.push("酒店预订确认".to_string());
    docs.push("旅行保险单".to_string());
    docs.push("紧急联系人信息".to_string());
    categories.push(PackingCategory { name: "证件与文件", icon: "📁", items: docs });

    // 衣物
    let mut clothes = Vec::new();
    match climate {
        "tropical" => {
            clothes.push("轻薄短袖/T恤 × 5".to_string());
            clothes.push("短裤/轻薄长裤 × 3".to_string());
            clothes.push("泳衣/泳裤".to_string());
            clothes.push("遮阳帽".to_string());
            clothes.push("太阳镜".to_string());
            clothes.push("凉鞋/拖鞋".to_string());
            clothes.push("轻便雨衣".to_string());
        }
        "cold" => {
            clothes.push("保暖内衣 × 3".to_string());
            clothes.push("毛衣/抓绒衣 × 2".to_string());
            clothes.push("羽绒服/厚外套".to_string());
            clothes.push("厚长裤 × 3".to_string());
            clothes.push("保暖手套".to_string());
            clothes.push("围巾".to_string());
            clothes.push("保暖帽".to_string());
            clothes.push("防水雪地靴".to_string());
        }
        "desert" => {
            clothes.push("长袖防晒衣 × 3".to_string());
            clothes.push("轻薄长裤 × 3".to_string());
            clothes.push("头巾/遮阳帽".to_string());
            clothes.push("太阳镜".to_string());
            clothes.push("高帮徒步鞋".to_string());
            clothes.push("防风沙口罩".to_string());
        }
        _ => {
            clothes.push("T恤/衬衫 × 4".to_string());
            clothes.push("长裤 × 2".to_string());
            clothes.push("外套/夹克".to_string());
            clothes.push("舒适步行鞋".to_string());
            if season == "winter" {
                clothes.push("毛衣".to_string());
                clothes.push("保暖外套".to_string());
            }
            if season == "summer" {
                clothes.push("短裤".to_string());
                clothes.push("凉鞋".to_string());
            }
        }
    }
    clothes.push("内衣裤 × 5".to_string());
    clothes.push("袜子 × 5".to_string());
    clothes.push("睡衣".to_string());
    categories.push(PackingCategory { name: "衣物", icon: "👕", items: clothes });

    // 洗漱用品
    categories.push(PackingCategory {
        name: "洗漱用品",
        icon: "🧴",
        items: vec![
            "牙刷/牙膏".to_string(),
            "洗发水/沐浴露".to_string(),
            "剃须刀".to_string(),
            "护肤品/防晒霜".to_string(),
            "毛巾".to_string(),
            "纸巾/湿巾".to_string(),
        ],
    });

    // 电子设备
    categories.push(PackingCategory {
        name: "电子设备",
        icon: "🔌",
        items: vec![
            "手机充电器".to_string(),
            "移动电源".to_string(),
            "转换插头".to_string(),
            "耳机".to_string(),
            "相机（可选）".to_string(),
        ],
    });

    // 药品
    categories.push(PackingCategory {
        name: "药品",
        icon: "💊",
        items: vec![
            "常用药品（感冒药/止泻药）".to_string(),
            "创可贴".to_string(),
            "个人处方药".to_string(),
            "防蚊液（热带地区）".to_string(),
        ],
    });

    // 其他
    let mut others = vec![
        "雨伞".to_string(),
        "背包/手提袋".to_string(),
        "水杯".to_string(),
        "零食".to_string(),
    ];
    if !duration.is_empty() {
        others.push(format!("预计行程: {}", duration));
    }
    categories.push(PackingCategory { name: "其他", icon: "🎒", items: others });

    categories
}

/// 格式化输出
fn format_output(destination: &str, categories: &[PackingCategory]) -> String {
    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║      🎒 PACKING LIST                 ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());
    lines.push(format!("║ 目的地: {:<29} ║", truncate(destination, 29)));
    lines.push("╠══════════════════════════════════════╣".to_string());

    for cat in categories {
        lines.push(format!("║ {} {}:", cat.icon, cat.name));
        for item in &cat.items {
            lines.push(format!("║   • {}", truncate(item, 32)));
        }
        lines.push("║".to_string());
    }

    lines.push("╚══════════════════════════════════════╝".to_string());
    lines.join("\n")
}

/// 简单的 JSON 字符串转义




#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Packing List 启动 — 生成行李清单");

    let destination = get_field("travel.destination").unwrap_or_default();
    if destination.trim().is_empty() {
        log_info("未设置目的地 (travel.destination)");
        return -1;
    }

    let duration = get_field("travel.duration").unwrap_or_default();
    let season = get_field("travel.season").unwrap_or_default();
    let has_passport = get_field("passport.number").map(|v| !v.trim().is_empty()).unwrap_or(false);
    let has_visa = get_field("visa.type").map(|v| !v.trim().is_empty()).unwrap_or(false);

    let categories = generate_packing_list(&destination, &duration, &season, has_passport, has_visa);
    let output = format_output(&destination, &categories);

    // Phase 2: 结构化结果
    let pairs_json: Vec<String> = categories
        .iter()
        .map(|cat| {
            let items_str = cat.items.join("、");
            format!(r#"{{"key":"{} {}","value":"{}"}}"#, escape_json(cat.icon), escape_json(cat.name), escape_json(&items_str))
        })
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"{} 行李清单","pairs":[{}],"text":"{}"}}"#,
        escape_json(&destination),
        pairs_json.join(","),
        escape_json(&output)
    );
    let _ = send_result_json(&result_json);

    // 同时保留日志输出
    for line in output.lines() {
        log_info(line);
    }

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_climate() {
        assert_eq!(infer_climate("泰国"), "tropical");
        assert_eq!(infer_climate("冰岛"), "cold");
        assert_eq!(infer_climate("迪拜"), "desert");
        assert_eq!(infer_climate("法国"), "temperate");
    }

    #[test]
    fn test_infer_season() {
        assert_eq!(infer_season("夏天"), "summer");
        assert_eq!(infer_season("冬季"), "winter");
        assert_eq!(infer_season("spring"), "spring");
        assert_eq!(infer_season(""), "any");
    }

    #[test]
    fn test_generate_packing_list() {
        let list = generate_packing_list("泰国", "7天", "夏天", true, true);
        assert!(!list.is_empty());
        // 检查热带衣物
        let clothes = &list.iter().find(|c| c.name == "衣物").unwrap().items;
        assert!(clothes.iter().any(|i| i.contains("泳")));
    }

    #[test]
    fn test_format_output() {
        let cats = vec![
            PackingCategory { name: "测试", icon: "🧪", items: vec!["物品1".to_string(), "物品2".to_string()] },
        ];
        let out = format_output("北京", &cats);
        assert!(out.contains("PACKING LIST"));
        assert!(out.contains("北京"));
        assert!(out.contains("物品1"));
    }
}
