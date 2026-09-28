use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::visit::Visit;
use syn::{Attribute, Item, Token};

pub(crate) const LIB: &str = "src-tauri/src/lib.rs";
pub(crate) const ACL: &str = "src-tauri/permissions/solo-soul/default.toml";
pub(crate) const SELECTION: &str = "src-tauri/ipc-contracts.json";

pub(crate) struct Sources {
    root: PathBuf,
    pub used: BTreeSet<String>,
    files: BTreeMap<String, syn::File>,
}
impl Sources {
    pub fn new(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("root: {error}"))?;
        if !root.is_dir() {
            return Err("root must be a directory".into());
        }
        Ok(Self {
            root,
            used: BTreeSet::new(),
            files: BTreeMap::new(),
        })
    }
    fn checked(&self, relative: &str) -> Result<PathBuf, String> {
        if relative.is_empty()
            || relative.contains('\\')
            || Path::new(relative)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!("invalid relative source path: {relative}"));
        }
        let path = self
            .root
            .join(relative)
            .canonicalize()
            .map_err(|error| format!("{relative}: {error}"))?;
        if !path.starts_with(&self.root) || !path.is_file() {
            return Err(format!("source escapes root or is not a file: {relative}"));
        }
        Ok(path)
    }
    pub fn read(&mut self, relative: &str) -> Result<String, String> {
        let path = self.checked(relative)?;
        let source =
            std::fs::read_to_string(path).map_err(|error| format!("{relative}: {error}"))?;
        self.used.insert(relative.to_string());
        Ok(source)
    }
    pub fn rust(&mut self, relative: &str) -> Result<syn::File, String> {
        if !relative.ends_with(".rs") {
            return Err(format!("Rust source must end in .rs: {relative}"));
        }
        if let Some(file) = self.files.get(relative) {
            return Ok(file.clone());
        }
        let file = syn::parse_file(&self.read(relative)?)
            .map_err(|error| format!("{relative}: {error}"))?;
        self.files.insert(relative.to_string(), file.clone());
        Ok(file)
    }
    // 解析真实 mod 声明而非仅拼目录，避免登记一个同名但未被 Host 使用的函数。
    pub fn resolve_module(&mut self, module: &[String]) -> Result<String, String> {
        let mut source = LIB.to_string();
        let mut directory = "src-tauri/src".to_string();
        for name in module {
            let file = self.rust(&source)?;
            let matches: Vec<_> = file
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Mod(item) if item.ident == name => Some(item),
                    _ => None,
                })
                .collect();
            if matches.len() != 1 {
                return Err(format!(
                    "{source}: module {name} must have exactly one declaration"
                ));
            }
            let declaration = matches[0];
            if declaration.content.is_some()
                || declaration
                    .attrs
                    .iter()
                    .any(|attr| !attr.path().is_ident("doc"))
            {
                return Err(format!("{source}: inline/attributed module {name} is not supported for migrated contracts"));
            }
            let direct = format!("{directory}/{name}.rs");
            let nested = format!("{directory}/{name}/mod.rs");
            let direct_exists = self.root.join(&direct).exists();
            let nested_exists = self.root.join(&nested).exists();
            source = match (direct_exists, nested_exists) {
                (true, false) => direct,
                (false, true) => nested,
                _ => return Err(format!("module {name} must have exactly one source file")),
            };
            self.checked(&source)?;
            directory = format!("{directory}/{name}");
        }
        Ok(source)
    }
    pub fn module_for_source(&mut self, source: &str) -> Result<Vec<String>, String> {
        self.checked(source)?;
        let suffix = source
            .strip_prefix("src-tauri/src/")
            .and_then(|value| value.strip_suffix(".rs"))
            .ok_or_else(|| {
                format!("DTO source must be under src-tauri/src and end in .rs: {source}")
            })?;
        let mut module: Vec<String> = suffix.split('/').map(str::to_owned).collect();
        if module.last().is_some_and(|name| name == "mod") {
            module.pop();
        }
        if source == LIB {
            module.clear();
        }
        for name in &module {
            syn::parse_str::<syn::Ident>(name)
                .map_err(|_| format!("invalid module path: {source}"))?;
        }
        if self.resolve_module(&module)? != source {
            return Err(format!(
                "source does not match the actual Rust module: {source}"
            ));
        }
        Ok(module)
    }
}

#[derive(Clone)]
pub(crate) struct Registration {
    pub path: Vec<String>,
    pub conditional: bool,
}
struct HandlerEntry {
    attrs: Vec<Attribute>,
    path: syn::Path,
}
impl Parse for HandlerEntry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            path: input.parse()?,
        })
    }
}
struct HandlerEntries(Vec<HandlerEntry>);
impl Parse for HandlerEntries {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let entries =
            syn::punctuated::Punctuated::<HandlerEntry, Token![,]>::parse_terminated(input)?;
        Ok(Self(entries.into_iter().collect()))
    }
}
#[derive(Default)]
struct Handlers {
    registrations: BTreeMap<String, Registration>,
    errors: Vec<String>,
    macros: usize,
    conditional_depth: usize,
}
impl<'ast> Visit<'ast> for Handlers {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        // 测试模块中的示例注册不是应用命令；其余条件命令保留全平台集合。
        if item.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test"))
        }) {
            return;
        }
        let conditional = item.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        });
        self.conditional_depth += usize::from(conditional);
        syn::visit::visit_item_mod(self, item);
        self.conditional_depth -= usize::from(conditional);
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if item.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test"))
        }) {
            return;
        }
        let conditional = item.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        });
        self.conditional_depth += usize::from(conditional);
        syn::visit::visit_item_fn(self, item);
        self.conditional_depth -= usize::from(conditional);
    }
    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        if path_text(&item.path) != "tauri::generate_handler" {
            return;
        }
        self.macros += 1;
        match syn::parse2::<HandlerEntries>(item.tokens.clone()) {
            Err(error) => self.errors.push(format!("generate_handler: {error}")),
            Ok(entries) => {
                for entry in entries.0 {
                    if entry.path.leading_colon.is_some()
                        || entry
                            .path
                            .segments
                            .iter()
                            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
                    {
                        self.errors
                            .push("handler path must be an ordinary Rust path".into());
                        continue;
                    }
                    if entry.attrs.iter().any(|attr| !attr.path().is_ident("cfg")) {
                        self.errors
                            .push("unsupported handler entry attribute".into());
                        continue;
                    }
                    let mut path: Vec<String> = entry
                        .path
                        .segments
                        .iter()
                        .map(|segment| segment.ident.to_string())
                        .collect();
                    if path.first().is_some_and(|name| name == "crate") {
                        path.remove(0);
                    }
                    let Some(name) = path.last().cloned() else {
                        continue;
                    };
                    if self
                        .registrations
                        .insert(
                            name.clone(),
                            Registration {
                                path,
                                conditional: !entry.attrs.is_empty() || self.conditional_depth > 0,
                            },
                        )
                        .is_some()
                    {
                        self.errors
                            .push(format!("duplicate handler command: {name}"));
                    }
                }
            }
        }
    }
}
pub(crate) fn registrations(file: &syn::File) -> Result<BTreeMap<String, Registration>, String> {
    let mut handlers = Handlers::default();
    handlers.visit_file(file);
    if handlers.macros == 0 {
        return Err("no tauri::generate_handler macro found".into());
    }
    if !handlers.errors.is_empty() {
        return Err(handlers.errors.join("; "));
    }
    Ok(handlers.registrations)
}
pub(crate) fn acl_commands(text: &str) -> Result<BTreeSet<String>, String> {
    let document: toml::Value = toml::from_str(text).map_err(|error| format!("ACL: {error}"))?;
    let permissions = document
        .get("permission")
        .and_then(toml::Value::as_array)
        .ok_or("ACL missing [[permission]]")?;
    let selected: Vec<_> = permissions
        .iter()
        .filter(|permission| {
            permission.get("identifier").and_then(toml::Value::as_str) == Some("allow-app-commands")
        })
        .collect();
    if selected.len() != 1 {
        return Err("ACL must define allow-app-commands exactly once".into());
    }
    let allowed = selected[0]
        .get("commands")
        .and_then(|value| value.get("allow"))
        .and_then(toml::Value::as_array)
        .ok_or("ACL missing commands.allow")?;
    let mut commands = BTreeSet::new();
    for command in allowed {
        let name = command.as_str().ok_or("ACL command must be a string")?;
        if !commands.insert(name.to_string()) {
            return Err(format!("duplicate ACL command: {name}"));
        }
    }
    Ok(commands)
}
pub(crate) fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
