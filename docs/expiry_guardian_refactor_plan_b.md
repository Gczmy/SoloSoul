# Expiry Guardian 插件全量重构方案（方案 B）

> 目标：将 `com.solosoul.official.expiry-guardian` 从旧版硬编码字段路径模式，升级为基于 **Typed Contract** 的现代插件架构，与当前 Tauri / `solosoul-plugin` host 能力完全对齐。

---

## 一、重构前后对比

| 维度 | 重构前（旧版） | 重构后（方案 B） |
|------|--------------|----------------|
| 字段访问 | 硬编码 `get_field("passport.expiryDate")` | 通过 `contracts` + `roles` 声明，Host 自动按 `contract_type_id` 匹配对象 |
| 对象扫描 | 一次只取每类第一个对象 | 使用 `list_objects` / `get_data_structure_tree` 扫描所有匹配对象 |
| 日期计算 | 手搓 Julian-day | 使用 `chrono` 或 SDK 共享 helper |
| 结果输出 | `format!` 拼接 `"key_value"` JSON | 强类型 `serde` 结构 + 自定义 `custom_ui` |
| 国际化 | 硬编码中文 emoji 标签 | 返回 i18n key，由前端或 manifest 本地化 |
| 契约 | `field_bindings` 仅声明字段 | 完整 `PluginContractBinding` + `roles` |
| 数据访问授权 | 依赖 `required_fields`/`optional_fields` 字符串匹配 | 走 Stage 4-B `resolve_typed` 路径，按 contract gate |

---

## 二、核心设计

### 2.1 契约定义

插件 manifest 中声明一个正式契约：

```json
{
  "contracts": [
    {
      "typeId": "com.solosoul.expiry/guardian/v1",
      "version": 1,
      "displayName": { "zh": "到期提醒", "en": "Expiry Guardian" },
      "strictContractGate": true,
      "roles": [
        { "roleId": "document", "label": { "zh": "证件", "en": "Document" }, "required": true, "defaultPropertyId": "__name__" },
        { "roleId": "expiryDate", "label": { "zh": "到期日", "en": "Expiry Date" }, "required": true, "defaultPropertyId": "expiryDate" }
      ]
    }
  ],
  "field_bindings": [
    { "contractTypeId": "com.solosoul.expiry/guardian/v1", "propertyId": "expiryDate", "abiName": "expiryDate" }
  ]
}
```

说明：
- `document` role 默认映射到对象名（`__name__`），插件用它显示"护照-01"、"签证-美国"等。
- `expiryDate` role 映射到实际存储到期日的属性。
- `strictContractGate: true` 表示只有 `contract_field` 被标记为 `expiryDate` 的属性才会被放行。

### 2.2 Host 侧的 Stage 4-B 路径

当前 `tauri/crates/solosoul-plugin/src/field.rs` 已支持：

- `resolve_typed(field_id)` — 通过 `contract_type_id` 反查 `UserTemplate` 与对象，再按 role 找到真实 property id。
- `field_metadata_typed(field_id)` — 返回字段标签与敏感度。
- `build_structure_tree()` — 返回类型/属性元数据树。
- `list_objects(type_id)` — 当插件声明 contracts 时，会先用 `parse_typed_field("{alias}.dummy")` 得到 `ctid`，再按 `contract_type_id` / `template_id` / `collection_type` 过滤对象。

因此方案 B **不需要修改 host Rust 代码**，只需让插件正确使用现有 typed API。

### 2.3 插件侧数据结构

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryItem {
    object_id: String,
    object_name: String,
    kind: String,          // 来自 contract_type_id 别名，如 "passport"
    expiry_date: String,   // 原始日期字符串
    days_remaining: i64,
    urgency: UrgencyLevel, // expired/critical/warning/notice/safe
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum UrgencyLevel {
    Expired,
    Critical,
    Warning,
    Notice,
    Safe,
}

impl UrgencyLevel {
    fn from_days(days: i64) -> Self {
        match days {
            d if d < 0 => UrgencyLevel::Expired,
            d if d <= 30 => UrgencyLevel::Critical,
            d if d <= 60 => UrgencyLevel::Warning,
            d if d <= 90 => UrgencyLevel::Notice,
            _ => UrgencyLevel::Safe,
        }
    }

    fn i18n_key(self) -> &'static str {
        match self {
            UrgencyLevel::Expired => "expired",
            UrgencyLevel::Critical => "critical",
            UrgencyLevel::Warning => "warning",
            UrgencyLevel::Notice => "notice",
            UrgencyLevel::Safe => "safe",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryResult {
    #[serde(rename = "type")]
    result_type: String,
    title: String,
    items: Vec<ExpiryItem>,
    summary: ExpirySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpirySummary {
    total: usize,
    expired: usize,
    critical: usize,
    warning: usize,
    notice: usize,
    safe: usize,
}
```

---

## 三、具体实现步骤

### 3.1 更新 `Cargo.toml`

```toml
[package]
name = "expiry-guardian"
version = "1.1.0"
edition = "2021"
authors = ["SoloSoul Official"]
description = "Expiry Guardian — Scan all documents and warn about upcoming expirations via typed contract"

[dependencies]
solosoul-plugin-sdk = { path = "../../SDK/rust" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[lib]
crate-type = ["cdylib"]
```

### 3.2 更新 `manifest.json`

```json
{
  "plugin_id": "com.solosoul.official.expiry-guardian",
  "name": "Expiry Guardian",
  "version": "1.1.0",
  "plugin_api_version": "2.0",
  "min_app_version": "2.5.0",
  "max_app_version": "999.999.999",
  "description": "证件到期卫士 — 基于契约扫描所有证件有效期并分级预警",
  "publisher": "SoloSoul Official",
  "homepage": "https://github.com/Gczmy/SoloSoul_plugin_market/tree/main/plugins/com.solosoul.official.expiry-guardian",
  "network_policy": {
    "block_all_outbound": true
  },
  "data_ttl_seconds": 60,
  "require_user_confirmation": false,
  "contracts": [
    {
      "typeId": "com.solosoul.expiry/guardian/v1",
      "version": 1,
      "displayName": {
        "zh": "到期提醒",
        "en": "Expiry Guardian"
      },
      "strictContractGate": true,
      "roles": [
        {
          "roleId": "document",
          "label": { "zh": "证件", "en": "Document" },
          "required": true,
          "defaultPropertyId": "__name__"
        },
        {
          "roleId": "expiryDate",
          "label": { "zh": "到期日", "en": "Expiry Date" },
          "required": true,
          "defaultPropertyId": "expiryDate"
        }
      ]
    }
  ],
  "field_bindings": [
    {
      "contractTypeId": "com.solosoul.expiry/guardian/v1",
      "propertyId": "expiryDate",
      "abiName": "expiryDate"
    }
  ],
  "i18n": {
    "zh": {
      "name": "到期卫士",
      "description": "基于契约扫描 Vault 中证件有效期，按 30/60/90/180 天分级提醒。"
    },
    "en": {
      "name": "Expiry Guardian",
      "description": "Scan all documents via typed contract for expiration dates and sort by urgency (30/60/90/180 days)."
    }
  },
  "tier": "p0",
  "category": "reminder",
  "custom_ui": "expiry_guardian"
}
```

注意：
- `plugin_api_version` 升级到 `2.0`，因为使用了 typed contract 和 `custom_ui`。
- `required_fields` / `optional_fields` 被 `contracts` + `roles` 取代，可删除。
- 如果 host 仍需要兼容旧版字段声明，可保留 `required_fields` 作为降级提示。

### 3.3 重写 `src/lib.rs`

#### 3.3.1 依赖与类型

```rust
//! Expiry Guardian — SoloSoul Official Plugin (Typed Contract Edition)
//!
//! 基于 Stage 4-B typed contract 扫描 Vault 中所有带 expiryDate 角色的对象，
//! 计算剩余天数并输出结构化结果。

use serde::{Deserialize, Serialize};
use solosoul_plugin_sdk::{
    get_data_structure_tree, get_field, get_locale, get_timestamp, list_objects, log_error,
    log_info, send_result_json,
};

///  urgency 分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Urgency {
    Expired,
    Critical,
    Warning,
    Notice,
    Safe,
}

impl Urgency {
    fn from_days(days: i64) -> Self {
        match days {
            d if d < 0 => Urgency::Expired,
            d if d <= 30 => Urgency::Critical,
            d if d <= 60 => Urgency::Warning,
            d if d <= 90 => Urgency::Notice,
            _ => Urgency::Safe,
        }
    }

    fn i18n_key(self) -> &'static str {
        match self {
            Urgency::Expired => "expired",
            Urgency::Critical => "critical",
            Urgency::Warning => "warning",
            Urgency::Notice => "notice",
            Urgency::Safe => "safe",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryItem {
    object_id: String,
    object_name: String,
    kind: String,
    expiry_date: String,
    days_remaining: i64,
    urgency: Urgency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpirySummary {
    total: usize,
    expired: usize,
    critical: usize,
    warning: usize,
    notice: usize,
    safe: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryResult {
    #[serde(rename = "type")]
    result_type: String,
    title: String,
    locale: String,
    items: Vec<ExpiryItem>,
    summary: ExpirySummary,
}
```

#### 3.3.2 日期解析与计算

方案 B 推荐使用 `chrono`（WASI 可编译）。如果希望零依赖，可使用 SDK 共享 helper；这里给出 `chrono` 版本：

```rust
use chrono::{Datelike, NaiveDate, Utc};

/// 解析 ISO "2025-12-31" 或 MRZ "251231" 格式
fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        return NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        let yy: i32 = s[..2].parse().ok()?;
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        let month: u32 = s[2..4].parse().ok()?;
        let day: u32 = s[4..6].parse().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    None
}

fn days_until(expiry: NaiveDate) -> i64 {
    let today = Utc::now().date_naive();
    (expiry - today).num_days()
}
```

如果坚持零依赖，保留原 Julian-day 实现，但抽出到 SDK：

```rust
// SDK/rust/src/lib.rs
pub fn parse_date_yyyymmdd_or_iso(date_str: &str) -> Option<(i32, u32, u32)> { ... }
pub fn days_until_ymd(year: i32, month: u32, day: u32) -> Option<i64> { ... }
```

并在插件中调用：

```rust
let (y, m, d) = solosoul_plugin_sdk::parse_date_yyyymmdd_or_iso(&raw_date)?;
let days = solosoul_plugin_sdk::days_until_ymd(y, m, d)?;
```

#### 3.3.3 扫描逻辑

```rust
/// 从数据结构树找出所有声明了 expiryDate 角色的类型别名。
fn discover_expiry_types() -> Vec<(String, String)> {
    let mut result = Vec::new();
    let tree_json = match get_data_structure_tree() {
        Ok(json) => json,
        Err(e) => {
            log_error(&format!("无法读取数据结构树: {:?}", e));
            return result;
        }
    };

    let tree: serde_json::Value = match serde_json::from_str(&tree_json) {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("数据结构树 JSON 解析失败: {}", e));
            return result;
        }
    };

    let types = tree.get("types").and_then(|v| v.as_array()).unwrap_or(&vec![]);
    for t in types {
        let alias = t.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let props = t.get("properties").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let has_expiry = props.iter().any(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .map(|id| id == "expiryDate")
                .unwrap_or(false)
        });
        if has_expiry {
            result.push((alias, name));
        }
    }
    result
}

/// 扫描单个类型的所有对象，读取 document name + expiryDate。
fn scan_type(alias: &str, type_name: &str) -> Vec<ExpiryItem> {
    let mut items = Vec::new();
    let json = match list_objects(alias) {
        Ok(j) => j,
        Err(e) => {
            log_error(&format!("list_objects({}) 失败: {:?}", alias, e));
            return items;
        }
    };

    let objects: Vec<serde_json::Value> = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("解析 {} 对象列表失败: {}", alias, e));
            return items;
        }
    };

    for obj in objects {
        let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let props = obj.get("properties").cloned().unwrap_or(serde_json::Value::Null);

        // 通过 typed field 路径读取 expiryDate 角色字段
        let field_path = format!("{}.expiryDate", alias);
        let raw_date = match get_field(&field_path) {
            Ok(v) => v,
            Err(_) => {
                // 回退：从 properties 直接取（兼容无 contract gate 场景）
                props
                    .get("expiryDate")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
        };

        if raw_date.is_empty() {
            log_info(&format!("{}: 未填写到期日", name));
            continue;
        }

        match parse_date(&raw_date) {
            Some(expiry) => {
                let days = days_until(expiry);
                let urgency = Urgency::from_days(days);
                items.push(ExpiryItem {
                    object_id: id,
                    object_name: name,
                    kind: type_name.to_string(),
                    expiry_date: raw_date,
                    days_remaining: days,
                    urgency,
                });
            }
            None => {
                log_error(&format!("{}: 无法解析日期 '{}'", name, raw_date));
            }
        }
    }

    items
}
```

#### 3.3.4 入口函数

```rust
#[no_mangle]
pub extern "C" fn run() -> i32 {
    let locale = get_locale().unwrap_or_else(|_| "en".to_string());
    log_info("Expiry Guardian 启动 — 基于契约扫描证件有效期");

    let mut items = Vec::new();
    for (alias, type_name) in discover_expiry_types() {
        log_info(&format!("扫描类型: {} ({})", type_name, alias));
        items.extend(scan_type(&alias, &type_name));
    }

    // 按 urgency 升序、days_remaining 升序排列
    items.sort_by_key(|i| (i.urgency, i.days_remaining));

    let summary = ExpirySummary {
        total: items.len(),
        expired: items.iter().filter(|i| i.urgency == Urgency::Expired).count(),
        critical: items.iter().filter(|i| i.urgency == Urgency::Critical).count(),
        warning: items.iter().filter(|i| i.urgency == Urgency::Warning).count(),
        notice: items.iter().filter(|i| i.urgency == Urgency::Notice).count(),
        safe: items.iter().filter(|i| i.urgency == Urgency::Safe).count(),
    };

    let title = if locale.starts_with("zh") {
        "证件到期预警"
    } else {
        "Document Expiry Alerts"
    };

    let result = ExpiryResult {
        result_type: "expiry_guardian".to_string(),
        title: title.to_string(),
        locale,
        items,
        summary,
    };

    match serde_json::to_string(&result) {
        Ok(json) => {
            let _ = send_result_json(&json);
        }
        Err(e) => {
            log_error(&format!("结果序列化失败: {}", e));
            return -1;
        }
    }

    0
}
```

### 3.4 可选：扩展 SDK

如果希望所有官方插件共享日期解析，在 `SoloSoul_plugin_market/SDK/rust/src/lib.rs` 增加：

```rust
/// 解析 ISO "YYYY-MM-DD" 或 MRZ "YYMMDD" 日期。
pub fn parse_date_yyyymmdd_or_iso(date_str: &str) -> Option<(i32, u32, u32)> {
    let s = date_str.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let year = s[..4].parse().ok()?;
        let month = s[5..7].parse().ok()?;
        let day = s[8..10].parse().ok()?;
        return Some((year, month, day));
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        let yy: i32 = s[..2].parse().ok()?;
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        let month: u32 = s[2..4].parse().ok()?;
        let day: u32 = s[4..6].parse().ok()?;
        return Some((year, month, day));
    }
    None
}

/// 计算目标日期距离今天的天数（基于 Unix 时间戳，零依赖）。
pub fn days_until_ymd(year: i32, month: u32, day: u32) -> Option<i64> {
    fn ordinal(y: i32, m: u32, d: u32) -> i64 {
        let a = (14 - m as i32) / 12;
        let y_adjusted = y + 4800 - a;
        let m_adjusted = m as i32 + 12 * a - 3;
        let jd = d as i64
            + ((153 * m_adjusted + 2) / 5) as i64
            + 365 * y_adjusted as i64
            + y_adjusted as i64 / 4
            - y_adjusted as i64 / 100
            + y_adjusted as i64 / 400
            - 32045;
        jd
    }

    let now_ms = get_timestamp();
    let today_ordinal = ordinal(1970, 1, 1) + now_ms / 86400000;
    let target_ordinal = ordinal(year, month, day);
    Some(target_ordinal - today_ordinal)
}
```

这样 `expiry-guardian` 和 `calendar-events` 都可以删除本地重复实现。

---

## 四、前端绑定（custom_ui）

### 4.1 新增 React 组件

在 `tauri/src/components/plugin-views/ExpiryGuardianView.tsx` 创建：

```tsx
import React from 'react';
import styles from './ExpiryGuardianView.module.css';

interface ExpiryItem {
  objectId: string;
  objectName: string;
  kind: string;
  expiryDate: string;
  daysRemaining: number;
  urgency: 'expired' | 'critical' | 'warning' | 'notice' | 'safe';
}

interface ExpirySummary {
  total: number;
  expired: number;
  critical: number;
  warning: number;
  notice: number;
  safe: number;
}

interface Props {
  payload: {
    title: string;
    locale: string;
    items: ExpiryItem[];
    summary: ExpirySummary;
  };
}

const urgencyLabels: Record<string, { zh: string; en: string }> = {
  expired: { zh: '已过期', en: 'Expired' },
  critical: { zh: '紧急', en: 'Critical' },
  warning: { zh: '警告', en: 'Warning' },
  notice: { zh: '注意', en: 'Notice' },
  safe: { zh: '安全', en: 'Safe' },
};

function urgencyClass(u: string): string {
  switch (u) {
    case 'expired': return styles.urgencyExpired;
    case 'critical': return styles.urgencyCritical;
    case 'warning': return styles.urgencyWarning;
    case 'notice': return styles.urgencyNotice;
    default: return styles.urgencySafe;
  }
}

export const ExpiryGuardianView: React.FC<Props> = ({ payload }) => {
  const isZh = payload.locale.startsWith('zh');
  const t = (obj: { zh: string; en: string }) => (isZh ? obj.zh : obj.en);

  return (
    <div className={styles.container}>
      <h3>{payload.title}</h3>
      <div className={styles.summary}>
        {Object.entries(payload.summary).map(([key, count]) => (
          <div key={key} className={styles.summaryItem}>
            <span>{key}</span>
            <strong>{count}</strong>
          </div>
        ))}
      </div>
      <ul className={styles.list}>
        {payload.items.map((item) => (
          <li key={item.objectId} className={`${styles.item} ${urgencyClass(item.urgency)}`}>
            <div className={styles.itemHeader}>
              <span className={styles.kind}>{item.kind}</span>
              <span className={styles.name}>{item.objectName}</span>
              <span className={styles.badge}>{t(urgencyLabels[item.urgency])}</span>
            </div>
            <div className={styles.itemMeta}>
              {item.expiryDate} · {isZh ? '剩余' : ''} {item.daysRemaining} {isZh ? '天' : 'days remaining'}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
};
```

### 4.2 注册 custom_ui 映射

在插件结果渲染器（如 `tauri/src/components/plugin/PluginResultRenderer.tsx`）中添加：

```tsx
import { ExpiryGuardianView } from '@/components/plugin-views/ExpiryGuardianView';

function renderResult(result: PluginResultPayload) {
  if (result.type === 'expiry_guardian') {
    return <ExpiryGuardianView payload={result} />;
  }
  // ...existing renderers
}
```

### 4.3 CSS 模块（最小示例）

```css
/* ExpiryGuardianView.module.css */
.container { padding: 16px; }
.summary { display: flex; gap: 12px; margin-bottom: 16px; }
.summaryItem { display: flex; flex-direction: column; align-items: center; }
.list { list-style: none; padding: 0; }
.item { border-radius: 8px; padding: 12px; margin-bottom: 8px; }
.urgencyExpired { background: #ffe5e5; border-left: 4px solid #ff4d4f; }
.urgencyCritical { background: #fff2e8; border-left: 4px solid #fa8c16; }
.urgencyWarning { background: #fffbe6; border-left: 4px solid #fadb14; }
.urgencyNotice { background: #e6f7ff; border-left: 4px solid #1890ff; }
.urgencySafe { background: #f6ffed; border-left: 4px solid #52c41a; }
```

---

## 五、验证步骤

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/SoloSoul_plugin_market

# 1. 如果扩展了 SDK，先构建 SDK
cd SDK/rust && cargo build && cd ../..

# 2. 构建插件
cd plugins/com.solosoul.official.expiry-guardian
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/expiry_guardian.wasm plugin.wasm

# 3. 运行测试
cargo test -p expiry-guardian

# 4. 检查 manifest 和 wasm 一致性
python3 scripts/generate_registry.py
cd ../..
python3 scripts/validate_registry.py  # 如果有这个脚本

# 5. 前端类型检查
cd /Users/zzc/PycharmProjects/SoloSoul_code/tauri
npm run check-all
```

---

## 六、风险与回退

| 风险 | 缓解措施 |
|------|---------|
| Host 侧 `strictContractGate` 未正确放行 `list_objects` | 先在 `manifest.json` 中设置 `strictContractGate: false`，验证通过后再开启 |
| 用户未将 expiryDate 字段标记为 contract_field | 插件保留从 `properties.expiryDate` 回退读取 |
| 前端缺少 `ExpiryGuardianView` 注册 | 插件仍可输出 `"type":"key_value"` 作为 fallback，直到前端实现 custom_ui |
| `plugin_api_version: "2.0"` 不被旧版 host 识别 | 提升 `min_app_version` 到支持 Stage 4-B 的版本（如 2.5.0） |
| `chrono` 在 wasm32-wasip1 上编译失败 | 改用 SDK 的零依赖 `parse_date_yyyymmdd_or_iso` / `days_until_ymd` helper |

---

## 七、文件变更清单

| 文件 | 变更 |
|------|------|
| `SoloSoul_plugin_market/plugins/com.solosoul.official.expiry-guardian/Cargo.toml` | 加 `serde`/`serde_json`，版本 `1.1.0`，描述更新 |
| `SoloSoul_plugin_market/plugins/com.solosoul.official.expiry-guardian/manifest.json` | 版本 `1.1.0`，`plugin_api_version` 升级到 `2.0`，添加完整 `contracts` + `roles`，删除 `required_fields`/`optional_fields`，加 `custom_ui` |
| `SoloSoul_plugin_market/plugins/com.solosoul.official.expiry-guardian/src/lib.rs` | 全量重写：typed contract 扫描、`serde` 强类型结果、`chrono`/SDK 日期计算、i18n key |
| `SoloSoul_plugin_market/SDK/rust/src/lib.rs` | 可选：增加 `parse_date_yyyymmdd_or_iso` / `days_until_ymd` 共享 helper |
| `SoloSoul_plugin_market/registry.json` | 重新生成 |
| `tauri/src/components/plugin-views/ExpiryGuardianView.tsx` | 新增 custom UI 组件 |
| `tauri/src/components/plugin-views/ExpiryGuardianView.module.css` | 新增样式 |
| `tauri/src/components/plugin/PluginResultRenderer.tsx` | 注册 `expiry_guardian` 类型渲染 |

---

## 八、建议的提交粒度

1. `feat(plugin): typed contract manifest for expiry-guardian` — manifest + Cargo.toml
2. `refactor(plugin): rewrite expiry-guardian with typed contract and serde result` — lib.rs
3. `feat(sdk): shared date helpers for plugins` — SDK（如果采用）
4. `feat(ui): ExpiryGuardian custom result view` — 前端组件
5. `chore(plugin): regenerate registry.json` — registry

---

## 九、备注

- 如果希望**完全不改动前端**，可以先不实现 `custom_ui`，把 `result_type` 继续输出为 `"key_value"` 或 `"table"`，利用 SDK 已有渲染器展示结果。
- 本方案假设当前 host 已完整支持 Stage 4-B typed lookup。如果实际运行中发现 `resolve_typed` 对 `__name__` role 支持不完整，需先小修 host 侧 `find_property_for_role` 对系统字段的处理。
