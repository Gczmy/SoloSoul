use std::collections::BTreeSet;
use syn::parse::Parser;
use syn::{Attribute, Token};

#[derive(Clone, Default)]
pub(crate) struct SerdeAttrs {
    pub rename: Option<String>,
    pub rename_all: Option<String>,
    pub default: bool,
    pub skip_none: bool,
    pub skip_empty_vec: bool,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub derives: BTreeSet<String>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Location {
    Struct,
    Enum,
    Variant,
    Field,
}

pub(crate) fn read(attrs: &[Attribute], location: Location) -> Result<SerdeAttrs, String> {
    let mut result = SerdeAttrs::default();
    let mut seen = BTreeSet::new();
    for attribute in attrs {
        if attribute.path().is_ident("doc") {
            continue;
        }
        if attribute.path().is_ident("derive")
            && matches!(location, Location::Struct | Location::Enum)
        {
            let parser = syn::punctuated::Punctuated::<syn::Path, Token![,]>::parse_terminated;
            let paths = parser
                .parse2(
                    attribute
                        .meta
                        .require_list()
                        .map_err(|error| error.to_string())?
                        .tokens
                        .clone(),
                )
                .map_err(|error| error.to_string())?;
            for path in paths {
                let name = super::source::path_text(&path);
                if !matches!(
                    name.as_str(),
                    "Debug"
                        | "Clone"
                        | "Copy"
                        | "Default"
                        | "PartialEq"
                        | "Eq"
                        | "Hash"
                        | "PartialOrd"
                        | "Ord"
                        | "Serialize"
                        | "Deserialize"
                        | "serde::Serialize"
                        | "serde::Deserialize"
                ) {
                    return Err(format!("unsupported derive: {name}"));
                }
                result
                    .derives
                    .insert(name.rsplit("::").next().unwrap_or(&name).to_string());
            }
            continue;
        }
        if !attribute.path().is_ident("serde") {
            return Err(format!(
                "unsupported DTO attribute: {}",
                super::source::path_text(attribute.path())
            ));
        }
        attribute
            .parse_nested_meta(|meta| {
                let key = meta
                    .path
                    .get_ident()
                    .ok_or_else(|| meta.error("serde key must be an identifier"))?
                    .to_string();
                if !seen.insert(key.clone()) {
                    return Err(meta.error(format!("duplicate serde attribute: {key}")));
                }
                match key.as_str() {
                    "rename" => result.rename = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
                    "rename_all" if location != Location::Field => {
                        let rule = meta.value()?.parse::<syn::LitStr>()?.value();
                        if !matches!(
                            rule.as_str(),
                            "lowercase"
                                | "UPPERCASE"
                                | "PascalCase"
                                | "camelCase"
                                | "snake_case"
                                | "SCREAMING_SNAKE_CASE"
                                | "kebab-case"
                                | "SCREAMING-KEBAB-CASE"
                        ) {
                            return Err(meta.error("unsupported rename_all rule"));
                        }
                        result.rename_all = Some(rule);
                    }
                    "default" if matches!(location, Location::Struct | Location::Field) => {
                        if meta.input.peek(Token![=]) || meta.input.peek(syn::token::Paren) {
                            return Err(meta.error("default functions are unsupported"));
                        }
                        result.default = true;
                    }
                    "skip_serializing_if" if location == Location::Field => {
                        let predicate = meta.value()?.parse::<syn::LitStr>()?.value();
                        match predicate.as_str() {
                            "Option::is_none" => result.skip_none = true,
                            "Vec::is_empty" => result.skip_empty_vec = true,
                            _ => {
                                return Err(meta
                                    .error("only Option::is_none and Vec::is_empty are supported"))
                            }
                        }
                    }
                    "tag" if location == Location::Enum => {
                        result.tag = Some(meta.value()?.parse::<syn::LitStr>()?.value())
                    }
                    "content" if location == Location::Enum => {
                        result.content = Some(meta.value()?.parse::<syn::LitStr>()?.value())
                    }
                    _ => return Err(meta.error(format!("unsupported serde attribute: {key}"))),
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
    }
    if result.content.is_some() && result.tag.is_none() {
        return Err("serde content requires tag".into());
    }
    if result.tag.is_some() && result.tag == result.content {
        return Err("serde tag and content must differ".into());
    }
    Ok(result)
}

// 依据已锁 serde_derive_internals 0.29.1 的 field/variant 命名规则实现。
// Tauri 命令参数另用 heck；两者对缩写与连续大写并不等价。
pub(crate) fn rename(name: &str, rule: Option<&str>, variant: bool) -> Result<String, String> {
    if !name.is_ascii() {
        return Err("non-ASCII Rust identifiers are not supported".into());
    }
    let pascal = || {
        let mut result = String::new();
        let mut upper = true;
        for ch in name.chars() {
            if ch == '_' {
                upper = true;
            } else if upper {
                result.push(ch.to_ascii_uppercase());
                upper = false;
            } else {
                result.push(ch);
            }
        }
        result
    };
    let snake = || {
        let mut result = String::new();
        for (index, ch) in name.chars().enumerate() {
            if index > 0 && ch.is_ascii_uppercase() {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        }
        result
    };
    let result = match (variant, rule.unwrap_or("")) {
        (_, "") | (true, "PascalCase") | (false, "lowercase" | "snake_case") => name.to_string(),
        (true, "lowercase") => name.to_ascii_lowercase(),
        (_, "UPPERCASE") => name.to_ascii_uppercase(),
        (false, "PascalCase") => pascal(),
        (_, "camelCase") => {
            let mut text = if variant { name.to_owned() } else { pascal() };
            if let Some(first) = text.get_mut(..1) {
                first.make_ascii_lowercase();
            }
            text
        }
        (true, "snake_case") => snake(),
        (_, "SCREAMING_SNAKE_CASE") => {
            if variant {
                snake().to_ascii_uppercase()
            } else {
                name.to_ascii_uppercase()
            }
        }
        (_, "kebab-case") => {
            if variant {
                snake().replace('_', "-")
            } else {
                name.replace('_', "-")
            }
        }
        (_, "SCREAMING-KEBAB-CASE") => {
            if variant {
                snake().to_ascii_uppercase().replace('_', "-")
            } else {
                name.to_ascii_uppercase().replace('_', "-")
            }
        }
        _ => return Err("unsupported naming rule".into()),
    };
    Ok(result)
}
pub(crate) fn ident(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_string()
}
pub(crate) fn quote(text: &str) -> String {
    serde_json::to_string(text).expect("strings always serialize")
}
