//! export_docx 子模块 —— markdown（P047 拆分）

use super::fields::{attachment_lines, flatten_object_fields};

/// Markdown 转义：仅转义会破坏结构或影响直接复制的字符。
///
/// 字段值来自用户数据，过度转义（`.` `()` `*` `-` `+` `#` 等）会让源码充满反斜杠、
/// 无法直接复制使用。只保留真正危险的：`\` 自身、反引号（代码块）、`[]`（链接）、
/// `|`（表格）、`<>`（HTML/自动链接）。
pub(crate) fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '`' | '[' | ']' | '|' | '<' | '>' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// 渲染为可点击的 Markdown 链接（`[text](url)`）。
///
/// label 取原文转义（保证所见即所得，避免长 URL 挤占版面）。
/// 链接目标按 CommonMark 规则处理：目标含 `[`/`]` 或需转义的括号/空格时
/// 用尖括号包裹（`<url>`），避免 `\(` 转义在部分解析器中被截断。
pub(crate) fn markdown_link(text: &str, url: &str) -> String {
    let escaped_text = escape_markdown(text);
    if escaped_text.contains('[') || escaped_text.contains(']') {
        // 文本含括号时回退为自动链接形式，保证所见即所得
        format!("<{}>", escape_markdown(url))
    } else {
        let escaped_url = escape_markdown(url);
        let needs_angle =
            escaped_url.contains(' ') || escaped_url.contains('(') || escaped_url.contains(')');
        let dest = if needs_angle {
            format!("<{}>", escaped_url)
        } else {
            escaped_url
        };
        format!("[{}]({})", escaped_text, dest)
    }
}

/// 从 byte 位置起匹配一个邮箱（不含空白，常见格式），返回 (start, end)。
fn find_email_at(value: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = value.as_bytes();
    if from >= bytes.len() {
        return None;
    }
    // 邮箱起点：字母数字/._%+-（至少一个），后跟 @
    let mut i = from;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || b"._%+-".contains(&bytes[i])) {
        i += 1;
    }
    if i == from || i >= bytes.len() || bytes[i] != b'@' {
        return None;
    }
    // @ 后域名：字母数字/.-（至少一个字符），最后一段至少 2 个字母
    let mut j = i + 1;
    let mut last_dot: Option<usize> = None;
    while j < bytes.len()
        && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'.' || bytes[j] == b'-')
    {
        if bytes[j] == b'.' {
            last_dot = Some(j);
        }
        j += 1;
    }
    let tld = last_dot? + 1;
    if j <= tld || j - tld < 2 {
        return None;
    }
    let tld_ok = bytes[tld..j].iter().all(|b| b.is_ascii_alphabetic());
    if !tld_ok {
        return None;
    }
    Some((from, j))
}

/// 从 byte 位置起匹配一个 http(s) URL，返回 (start, end)。
fn find_url_at(value: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = value.as_bytes();
    let rest = &value[from..];
    let lower = rest.to_ascii_lowercase();
    let scheme_len = if lower.starts_with("https://") {
        8
    } else if lower.starts_with("http://") {
        7
    } else {
        return None;
    };
    let mut j = from + scheme_len;
    while j < bytes.len() {
        let b = bytes[j];
        // URL 内不允许的字符：空白与 Markdown/HTML 结构字符
        if b.is_ascii_whitespace()
            || matches!(
                b,
                b'<' | b'>' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'\\' | b'"' | b'\''
            )
        {
            break;
        }
        j += 1;
    }
    // 去掉结尾的标点（.,;:!?）
    while j > from + scheme_len {
        let b = bytes[j - 1];
        if matches!(b, b'.' | b',' | b';' | b':' | b'!' | b'?') {
            j -= 1;
        } else {
            break;
        }
    }
    if j <= from + scheme_len {
        return None;
    }
    Some((from, j))
}

/// 把值文本中的链接实体（email / url）自动转换为可点击链接；其余保持原文。
/// 逐字符扫描（零依赖），已链接段不再重复处理。
/// 行首结构字符防护：仅在行首转义 `#`/`-`/`>` 及有序列表前缀数字（`1.`/`1)`）。
/// 其余位置保持原样（源码可复制），避免多行值续行被渲染成标题/列表/引用。
/// 注意：`+` 不是列表标记（CommonMark 列表仅 `-`/`*`/数字），且电话 `+86…`
/// 行首常见，故不纳入防护。
fn escape_line_leading(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let first = bytes[0];
    // 首字符宽度：ASCII 1 字节，中文等 2-4 字节（`first as char` 会把首字节当
    // 独立码点导致中文首字节损坏，必须按 UTF-8 长度切片保留原始字符）
    let first_char = &value[..utf8_len(first)];
    let mut escaped = String::new();
    if matches!(first, b'#' | b'-' | b'>') {
        escaped.push('\\');
        escaped.push_str(first_char);
    } else if first.is_ascii_digit() {
        // 有序列表前缀：数字后跟 `.` 或 `)`（如 `1. 条目` / `1) 条目`）
        let next = bytes.get(1).copied();
        if matches!(next, Some(b'.') | Some(b')')) {
            escaped.push('\\');
            escaped.push_str(first_char);
        } else {
            escaped.push_str(first_char);
        }
    } else {
        escaped.push_str(first_char);
    }
    escaped.push_str(&value[utf8_len(first)..]);
    escaped
}

fn linkify_markdown_text(value: &str) -> String {
    let mut out = String::new();
    let mut pos = 0usize;
    let bytes = value.as_bytes();
    while pos < bytes.len() {
        let b = bytes[pos];
        if b.is_ascii_alphanumeric() || b == b'@' || b == b'+' || b == b'-' || b == b'.' {
            // 尝试邮箱（@ 起头或字母数字开头）；find_email_at 的起点即当前 pos
            if let Some((_, e)) = find_email_at(value, pos) {
                out.push_str(&markdown_link(
                    &value[pos..e],
                    &format!("mailto:{}", &value[pos..e]),
                ));
                pos = e;
                continue;
            }
            // 尝试 URL（h 起头且为 http(s)）；find_url_at 的起点即当前 pos
            if b == b'h' {
                if let Some((_, e)) = find_url_at(value, pos) {
                    out.push_str(&markdown_link(&value[pos..e], &value[pos..e]));
                    pos = e;
                    continue;
                }
            }
        }
        // 未命中：逐字符转义后追加（保持后续扫描位置）
        let ch_len = utf8_len(b);
        out.push_str(&escape_markdown(&value[pos..pos + ch_len]));
        pos += ch_len;
    }
    out
}

/// UTF-8 首字节的字符长度。
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// 按字段类型渲染 markdown 值：
/// - url → 自动链接（`[url](url)`）；
/// - email → `[addr](mailto:addr)`；
/// - phone → `[号码](tel:号码)`；
/// - date/datetime → 原样（markdown 无日期语义）；
/// - 其余 → 链接实体自动转换。
///
/// 多行值：行间用空行分隔（Markdown 段落分隔），渲染时真换行。
pub(crate) fn markdown_field_value(ftype: &str, value: &str) -> String {
    let lines: Vec<&str> = value.split('\n').collect();
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n"); // 空行分隔（段落即换行）
        }
        let line = escape_line_leading(line); // 行首结构字符防护（仅行首，其余保持原样）
        let rendered = match ftype {
            "url" => {
                let line = line.trim();
                if line.is_empty() {
                    String::new()
                } else {
                    markdown_link(line, line)
                }
            }
            "email" => {
                let line = line.trim();
                if line.is_empty() {
                    String::new()
                } else {
                    markdown_link(line, &format!("mailto:{}", line))
                }
            }
            "phone" => {
                let line = line.trim();
                if line.is_empty() {
                    String::new()
                } else {
                    markdown_link(line, &format!("tel:{}", line))
                }
            }
            "date" | "datetime" => escape_markdown(&line),
            _ => linkify_markdown_text(&line),
        };
        out.push_str(&rendered);
    }
    out
}

/// 构造 Markdown 文档（UTF-8）：封面段 → 每对象（`#` 标题 / 元信息 / 字段列表 / 附件）。
/// 对象间以 `---` 分隔（连续排版）；字段值与标签均做 Markdown 转义。
pub(crate) fn build_markdown_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
    account_name: &str,
    account_id: &str,
) -> String {
    let mut out = String::new();
    out.push_str("# SoloSoul\n\n");
    out.push_str(&format!(
        "账户名：{}（{}）\n\n",
        escape_markdown(account_name),
        escape_markdown(account_id)
    ));
    out.push_str(&format!(
        "{} · 导出 {} 个对象\n\n",
        escape_markdown(export_time),
        records.len()
    ));

    for (idx, rec) in records.iter().enumerate() {
        if idx > 0 {
            out.push_str("---\n\n");
        }
        // 对象名用 H3（H1 留给封面标题；H1/H2 默认带下划线与 --- 分隔线冲突，且字级过大）
        out.push_str(&format!("### 对象名称：{}\n\n", escape_markdown(&rec.name)));

        // 元信息段
        let tpl_name = rec
            .template_id
            .as_ref()
            .and_then(|tid| template_names.get(tid))
            .cloned()
            .unwrap_or_default();
        let mut meta_lines = Vec::new();
        if !tpl_name.is_empty() {
            meta_lines.push(format!("模板：{}", escape_markdown(&tpl_name)));
        }
        meta_lines.push(format!("创建时间：{}", escape_markdown(&rec.created_at)));
        meta_lines.push(format!("更新时间：{}", escape_markdown(&rec.updated_at)));
        if !rec.tags_json.is_empty() {
            meta_lines.push(format!(
                "标签：{}",
                rec.tags_json
                    .iter()
                    .map(|t| escape_markdown(t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for line in &meta_lines {
            out.push_str("- ");
            out.push_str(line);
            out.push('\n');
        }

        // 字段：`**标签**：值`——按字段类型格式化（url/email/phone 链接化）；
        // 字段间空行分隔（Markdown 段落即换行，避免相邻字段挤在一行）；
        // 多行值内部同样空行分隔。
        let fields = flatten_object_fields(rec);
        if !fields.is_empty() {
            out.push('\n');
            for (label, value, ftype) in &fields {
                let value = markdown_field_value(ftype, value);
                out.push_str(&format!("**{}**：{}\n\n", escape_markdown(label), value));
            }
        }

        // 附件清单（主行 + 可选描述/标签子行，嵌套列表）
        let attachments = attachment_lines(rec);
        if !attachments.is_empty() {
            out.push_str("\n附件清单：\n");
            for entry in &attachments {
                out.push_str("- ");
                out.push_str(&escape_markdown(&entry.main));
                out.push('\n');
                if let Some(desc) = &entry.description {
                    out.push_str("  - 描述：");
                    out.push_str(&escape_markdown(desc));
                    out.push('\n');
                }
                if !entry.tags.is_empty() {
                    out.push_str("  - 标签：");
                    out.push_str(&escape_markdown(&entry.tags.join("、")));
                    out.push('\n');
                }
            }
        }
        out.push('\n');
    }
    out
}
