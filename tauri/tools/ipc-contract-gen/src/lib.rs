//! 只读 Rust 源码到 IPC TypeScript 契约的增量生成器。
//!
//! 不链接 Host/Vault、不执行 Rust 函数或 serde 默认值。支持普通命名 struct、
//! unit/external/adjacent enum，以及只有 unit/named variants 的 internal enum；
//! String/bool/普通 JSON 数值、Option、Vec、字符串键 HashMap、serde_json::Value、
//! 显式嵌套 DTO，以及经 workspace/Host path 依赖核验的库源码。输入和输出独立投影：
//! 输入 Option/default 可缺键；输出普通 Option 必需且 nullable；
//! Option::is_none / Vec::is_empty 仅控制输出省略，不能暗含输入 default。
//! 支持单字段 serde transparent tuple newtype；default 函数只控制输入缺键，永不执行。
//! Output 可忽略合法 alias/deserialize_with，Input 仍拒绝这些反序列化属性。
//! 命令仅支持一个 tauri::Runtime 泛型，且必须仅用于根层原生注入，不得进入 wire。
//! 精确 tauri::ResourceId 映射句柄别名；Channel<T> 仅限命令参数根层，T 按 Output 投影。
//! 仅支持 Output 的具体命名 struct flatten，拒绝碰撞、递归与 Input flatten。
//! 不支持宏生成 DTO、条件字段、泛型、type alias、其它 flatten/untagged、自定义 Serialize、
//! 分向 rename 及未知注解。此类输入必须报错，不能生成 any/未知占位。
//! 数值保持现有 JSON number 协议；浮点输出另含非有限值的 null，不声称 TS
//! 能表达整数范围或 u64 精度。空 DTO 用 Record<string, never> 限定对象形状。

mod attrs;
mod runtime;
mod source;
mod types;

use heck::{ToLowerCamelCase, ToSnakeCase};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::{FnArg, GenericArgument, Item, PathArguments, ReturnType, Type};
use types::{Catalog, Direction, Renderer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    commands: Vec<SelectedCommand>,
    types: Vec<String>,
    events: Vec<SelectedEvent>,
    #[serde(default)]
    crates: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectedCommand {
    name: String,
    source: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectedEvent {
    name: String,
    source: String,
    payload: String,
}

#[derive(Debug, Serialize)]
pub struct Generated {
    pub typescript: String,
    pub manifest: serde_json::Value,
}

pub fn generate(root: &Path) -> Result<Generated, String> {
    let mut sources = source::Sources::new(root)?;
    let library = sources.rust(source::LIB)?;
    sources.reject_namespace_shadowing(&library, source::LIB)?;
    let handlers = source::registrations(&library)?;
    let allowed = source::acl_commands(&sources.read(source::ACL)?)?;
    for name in handlers.keys() {
        if !allowed.contains(name) {
            return Err(format!("registered command missing ACL: {name}"));
        }
    }
    let selection: Selection = serde_json::from_str(&sources.read(source::SELECTION)?)
        .map_err(|error| format!("selection: {error}"))?;
    sources.register_crates(&selection.crates)?;
    let mut catalog = Catalog::default();
    let mut selected = BTreeMap::new();
    for command in selection.commands {
        if selected.contains_key(&command.name) {
            return Err(format!("duplicate selected command: {}", command.name));
        }
        let registered = handlers
            .get(&command.name)
            .ok_or_else(|| format!("selected command is not registered: {}", command.name))?;
        if registered.conditional {
            return Err(format!(
                "conditional migrated command is unsupported: {}",
                command.name
            ));
        }
        let module = &registered.path[..registered.path.len().saturating_sub(1)];
        let actual_source = sources.resolve_module(module)?;
        if actual_source != command.source {
            return Err(format!(
                "{}: selected source does not match handler module ({actual_source})",
                command.name
            ));
        }
        catalog.add(&mut sources, &command.source, module)?;
        let file = sources.rust(&command.source)?;
        let functions: Vec<_> = file
            .items
            .into_iter()
            .filter_map(|item| match item {
                Item::Fn(function) if attrs::ident(&function.sig.ident) == command.name => {
                    Some(function)
                }
                _ => None,
            })
            .collect();
        if functions.len() != 1 {
            return Err(format!(
                "{} must resolve to exactly one actual function",
                command.name
            ));
        }
        selected.insert(
            command.name,
            (
                module.join("::"),
                functions.into_iter().next().ok_or("missing function")?,
            ),
        );
    }
    let mut additional_sources = BTreeSet::new();
    for source in selection.types {
        if !additional_sources.insert(source.clone()) {
            return Err(format!("duplicate DTO source: {source}"));
        }
        let module = sources.module_for_source(&source)?;
        catalog.add(&mut sources, &source, &module)?;
    }
    let mut events = BTreeMap::new();
    for event in selection.events {
        if event.name.is_empty() || event.name.chars().any(char::is_control) {
            return Err("event name must be a non-empty printable string".into());
        }
        if events.contains_key(&event.name) {
            return Err(format!("duplicate event: {}", event.name));
        }
        let module = sources.module_for_source(&event.source)?;
        catalog.add(&mut sources, &event.source, &module)?;
        let payload: Type = syn::parse_str(&event.payload)
            .map_err(|error| format!("event {} payload: {error}", event.name))?;
        events.insert(event.name, (module.join("::"), payload));
    }
    let mut renderer = Renderer::new(&catalog);
    let mut command_members = Vec::new();
    let mut error_members = Vec::new();
    let mut structured_error_commands = Vec::new();
    for (name, (module, function)) in &selected {
        let (arguments, result, error) = command_contract(&mut renderer, module, function)
            .map_err(|error| format!("command {name}: {error}"))?;
        command_members.push(format!(
            "  {}: {{ args: {arguments}; result: {result}; }};",
            attrs::quote(name)
        ));
        error_members.push(format!("  {}: {error};", attrs::quote(name)));
        if error != "string" && error != "never" {
            structured_error_commands.push(name.clone());
        }
    }
    let mut event_members = Vec::new();
    for (name, (module, payload)) in &events {
        let ty = renderer
            .render(payload, module, Direction::Output)
            .map_err(|error| format!("event {name}: {error}"))?;
        event_members.push(format!("  {}: {ty};", attrs::quote(name)));
    }
    let mut typescript = "// Generated by solosoul-ipc-contract-gen 0.1.0. Do not edit.\n// Contracts cover only the registered migrated subset.\n\n".to_string();
    typescript.push_str(renderer.imports());
    let definitions = renderer.definitions();
    if !definitions.is_empty() {
        typescript.push_str(&definitions);
        typescript.push_str("\n\n");
    }
    typescript.push_str(&map_type("IpcCommands", &command_members));
    typescript.push_str("\n\n");
    typescript.push_str(&map_type("IpcCommandErrors", &error_members));
    typescript.push_str("\n\n");
    typescript.push_str(&map_type("IpcEvents", &event_members));
    typescript.push('\n');
    let command_names: Vec<_> = selected.keys().cloned().collect();
    let event_names: Vec<_> = events.keys().cloned().collect();
    let unmigrated: Vec<_> = handlers
        .keys()
        .filter(|name| !selected.contains_key(*name))
        .cloned()
        .collect();
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "generator": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") },
        "commands": command_names,
        "events": event_names,
        "structuredErrorCommands": structured_error_commands,
        "unmigratedCommands": unmigrated,
        "sources": sources.used.into_iter().collect::<Vec<_>>(),
    });
    Ok(Generated {
        typescript,
        manifest,
    })
}
fn map_type(name: &str, members: &[String]) -> String {
    if members.is_empty() {
        format!("export type {name} = Record<never, never>;")
    } else {
        format!("export type {name} = {{\n{}\n}};", members.join("\n"))
    }
}
fn command_contract(
    renderer: &mut Renderer<'_>,
    module: &str,
    function: &syn::ItemFn,
) -> Result<(String, String, String), String> {
    let runtime = runtime::parameter(renderer, module, &function.sig.generics)?;
    let mut runtime_used = false;
    if function.sig.unsafety.is_some()
        || function.sig.abi.is_some()
        || function.sig.variadic.is_some()
        || function.sig.constness.is_some()
    {
        return Err("unsafe/extern/variadic/const commands are unsupported".into());
    }
    let mut attribute_count = 0;
    let mut snake_case = false;
    for attribute in &function.attrs {
        if attribute.path().is_ident("doc") {
            continue;
        }
        if attribute.path().is_ident("allow") {
            let lint: syn::Path = attribute
                .parse_args()
                .map_err(|_| "unsupported command lint attribute")?;
            if source::path_text(&lint) != "clippy::too_many_arguments" {
                return Err("unsupported command lint attribute".into());
            }
            continue;
        }
        if source::path_text(attribute.path()) != "tauri::command" {
            return Err("unsupported command attribute".into());
        }
        attribute_count += 1;
        if let syn::Meta::List(_) = &attribute.meta {
            let mut seen = false;
            attribute
                .parse_nested_meta(|meta| {
                    if !meta.path.is_ident("rename_all") || seen {
                        return Err(meta.error("unsupported/duplicate tauri command option"));
                    }
                    seen = true;
                    let rule = meta.value()?.parse::<syn::LitStr>()?.value();
                    snake_case = match rule.as_str() {
                        "snake_case" => true,
                        "camelCase" => false,
                        _ => return Err(meta.error("unsupported command argument case")),
                    };
                    Ok(())
                })
                .map_err(|error| error.to_string())?;
        } else if !matches!(attribute.meta, syn::Meta::Path(_)) {
            return Err("unsupported tauri command attribute".into());
        }
    }
    if attribute_count != 1 {
        return Err("exactly one #[tauri::command] is required".into());
    }
    let mut names = BTreeSet::new();
    let mut parameters = Vec::new();
    for argument in &function.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            return Err("self command arguments are unsupported".into());
        };
        if !argument.attrs.is_empty() {
            return Err("attributed command arguments are unsupported".into());
        }
        let syn::Pat::Ident(pattern) = argument.pat.as_ref() else {
            return Err("command parameters must be identifiers".into());
        };
        if pattern.by_ref.is_some() || pattern.subpat.is_some() {
            return Err("destructured/ref command arguments are unsupported".into());
        }
        if let Some(parameter) = runtime.as_deref() {
            if runtime::contains(&argument.ty, parameter) {
                if !runtime::injection(renderer, module, &argument.ty, parameter)? {
                    return Err("Runtime parameter is allowed only in root Tauri injections".into());
                }
                runtime_used = true;
                continue;
            }
        }
        if injected(renderer, module, &argument.ty)? {
            continue;
        }
        let rust_name = attrs::ident(&pattern.ident);
        let name = if snake_case {
            rust_name.to_snake_case()
        } else {
            rust_name.to_lower_camel_case()
        };
        if !names.insert(name.clone()) {
            return Err(format!("duplicate serialized command parameter: {name}"));
        }
        let optional = renderer.option_inner(&argument.ty, module)?.is_some();
        let ty = renderer.render_argument(&argument.ty, module)?;
        parameters.push(format!(
            "{}{}: {ty};",
            attrs::quote(&name),
            if optional { "?" } else { "" }
        ));
    }
    if runtime.is_some() && !runtime_used {
        return Err("Runtime parameter requires a matching Tauri injection".into());
    }
    let arguments = if parameters.is_empty() {
        "undefined".into()
    } else {
        format!("{{ {} }}", parameters.join(" "))
    };
    let (result, error) = match &function.sig.output {
        ReturnType::Default => ("null".into(), "never".into()),
        ReturnType::Type(_, ty) => {
            if runtime
                .as_deref()
                .is_some_and(|parameter| runtime::contains(ty, parameter))
            {
                return Err("Runtime parameter cannot appear in a command result".into());
            }
            let (success, error) = result_types(renderer, module, ty)?;
            let result = renderer.render(success, module, Direction::Output)?;
            let error = error
                .map(|ty| renderer.render(ty, module, Direction::Output))
                .transpose()?
                .unwrap_or_else(|| "never".into());
            (result, error)
        }
    };
    Ok((arguments, result, error))
}
/// 实际 Result 的成功和拒绝载荷分别生成；只支持能按 Output serde 明确展开的类型。
fn result_types<'a>(
    renderer: &Renderer<'_>,
    module: &str,
    ty: &'a Type,
) -> Result<(&'a Type, Option<&'a Type>), String> {
    let Type::Path(path) = ty else {
        return Ok((ty, None));
    };
    let name = renderer.path(module, &path.path)?;
    if !matches!(
        name.as_str(),
        "Result" | "std::result::Result" | "core::result::Result"
    ) {
        return Ok((ty, None));
    }
    let PathArguments::AngleBracketed(arguments) = &path
        .path
        .segments
        .last()
        .ok_or("empty Result path")?
        .arguments
    else {
        return Err("Result requires success and error types".into());
    };
    if arguments.args.len() != 2 {
        return Err("Result requires two concrete type arguments".into());
    }
    let mut args = arguments.args.iter();
    let Some(GenericArgument::Type(success)) = args.next() else {
        return Err("unsupported Result success type".into());
    };
    let Some(GenericArgument::Type(error)) = args.next() else {
        return Err("unsupported Result error type".into());
    };
    Ok((success, Some(error)))
}
fn injected(renderer: &Renderer<'_>, module: &str, ty: &Type) -> Result<bool, String> {
    let Type::Path(path) = ty else {
        return Ok(false);
    };
    if path.qself.is_some() {
        return Err("qualified command types are unsupported".into());
    }
    let name = renderer.path(module, &path.path)?;
    // 只识别 Tauri 的明确注入类型，不能把任意未知参数当作 Host 状态忽略。
    if name == "tauri::State" {
        let PathArguments::AngleBracketed(arguments) = &path
            .path
            .segments
            .last()
            .ok_or("missing State path")?
            .arguments
        else {
            return Err("tauri::State requires a concrete type".into());
        };
        let types = arguments
            .args
            .iter()
            .filter(|argument| matches!(argument, GenericArgument::Type(_)))
            .count();
        if types != 1
            || arguments.args.iter().any(|argument| {
                !matches!(
                    argument,
                    GenericArgument::Type(_) | GenericArgument::Lifetime(_)
                )
            })
        {
            return Err("unsupported tauri::State arguments".into());
        }
        return Ok(true);
    }
    if matches!(
        name.as_str(),
        "tauri::AppHandle" | "tauri::Window" | "tauri::WebviewWindow" | "tauri::Webview"
    ) {
        if path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
        {
            return Err("generic Tauri injection types are unsupported".into());
        }
        return Ok(true);
    }
    Ok(false)
}
