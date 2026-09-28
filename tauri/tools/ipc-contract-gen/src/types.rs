use super::attrs::{self, Location};
use super::source::{path_text, Sources};
use std::collections::{BTreeMap, BTreeSet};
use syn::{GenericArgument, Item, PathArguments, Type, UseTree};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Direction {
    Input,
    Output,
}
#[derive(Clone)]
enum Definition {
    Struct(syn::ItemStruct),
    Enum(syn::ItemEnum),
    Alias,
}
#[derive(Clone)]
struct Decl {
    module: String,
    source: String,
    definition: Definition,
    manual_serde: bool,
}
#[derive(Default)]
pub(crate) struct Catalog {
    declarations: BTreeMap<String, Decl>,
    imports: BTreeMap<String, BTreeMap<String, String>>,
    crate_roots: BTreeMap<String, String>,
}
impl Catalog {
    pub fn add(
        &mut self,
        sources: &mut Sources,
        source: &str,
        module: &[String],
    ) -> Result<(), String> {
        let module = module.join("::");
        if self.imports.contains_key(&module) {
            return Ok(());
        }
        self.crate_roots
            .insert(module.clone(), sources.crate_namespace(source).to_string());
        let file = sources.rust(source)?;
        sources.reject_namespace_shadowing(&file, source)?;
        if file.attrs.iter().any(|attribute| {
            !attribute.path().is_ident("doc") && !attribute.path().is_ident("allow")
        }) {
            return Err(format!("{source}: unsupported source-level attribute"));
        }
        let mut imports = BTreeMap::new();
        for item in &file.items {
            if let Item::Use(item) = item {
                if item
                    .attrs
                    .iter()
                    .any(|attribute| !attribute.path().is_ident("doc"))
                {
                    return Err(format!(
                        "{source}: conditional/attributed use is unsupported"
                    ));
                }
                collect_use(&item.tree, String::new(), &mut imports)?;
            }
        }
        let manual: BTreeSet<_> = file
            .items
            .iter()
            .filter_map(|item| {
                let Item::Impl(item) = item else {
                    return None;
                };
                let (_, trait_path, _) = item.trait_.as_ref()?;
                if !trait_path.segments.last().is_some_and(|segment| {
                    segment.ident == "Serialize" || segment.ident == "Deserialize"
                }) {
                    return None;
                }
                let Type::Path(ty) = item.self_ty.as_ref() else {
                    return None;
                };
                ty.path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string())
            })
            .collect();
        for item in file.items {
            let (name, definition) = match item {
                Item::Struct(item) => (attrs::ident(&item.ident), Definition::Struct(item)),
                Item::Enum(item) => (attrs::ident(&item.ident), Definition::Enum(item)),
                Item::Type(item) => (attrs::ident(&item.ident), Definition::Alias),
                _ => continue,
            };
            let qualified = qualify(&module, &name);
            if self
                .declarations
                .insert(
                    qualified.clone(),
                    Decl {
                        module: module.clone(),
                        source: source.to_string(),
                        definition,
                        manual_serde: manual.contains(&name),
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate type declaration: {qualified}"));
            }
        }
        self.imports.insert(module, imports);
        Ok(())
    }
    pub fn expand_path(&self, module: &str, path: &syn::Path) -> Result<String, String> {
        if path.leading_colon.is_some() {
            return Err("absolute :: type paths are unsupported".into());
        }
        let raw = path_text(path);
        let mut parts = raw.split("::");
        let first = parts.next().ok_or("empty type path")?;
        let tail = parts.collect::<Vec<_>>().join("::");
        let imported = self
            .imports
            .get(module)
            .and_then(|imports| imports.get(first));
        let base = if let Some(imported) = imported {
            if tail.is_empty() {
                imported.clone()
            } else {
                format!("{imported}::{tail}")
            }
        } else {
            raw
        };
        if let Some(rest) = base.strip_prefix("crate::") {
            let root = self.crate_roots.get(module).map_or("", String::as_str);
            if root.is_empty()
                && self.crate_roots.values().any(|external| {
                    !external.is_empty() && rest.split("::").next() == Some(external.as_str())
                })
            {
                return Err("crate:: cannot refer to an external dependency".into());
            }
            return Ok(qualify(root, rest));
        }
        if let Some(rest) = base.strip_prefix("self::") {
            return Ok(qualify(module, rest));
        }
        if base.starts_with("super::") {
            let mut parent: Vec<_> = module.split("::").filter(|part| !part.is_empty()).collect();
            let mut rest = base.as_str();
            while let Some(next) = rest.strip_prefix("super::") {
                let minimum = usize::from(
                    self.crate_roots
                        .get(module)
                        .is_some_and(|root| !root.is_empty()),
                );
                if parent.len() <= minimum {
                    return Err("super path escapes crate".into());
                }
                parent.pop();
                rest = next;
            }
            return Ok(qualify(&parent.join("::"), rest));
        }
        if !base.contains("::") && self.declarations.contains_key(&qualify(module, &base)) {
            return Ok(qualify(module, &base));
        }
        Ok(base)
    }
}
fn qualify(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}::{name}")
    }
}
fn collect_use(
    tree: &UseTree,
    prefix: String,
    result: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    match tree {
        UseTree::Path(item) => collect_use(
            &item.tree,
            qualify(&prefix, &item.ident.to_string()),
            result,
        ),
        UseTree::Name(item) => {
            let name = item.ident.to_string();
            if name == "self" {
                return Err("use self imports are unsupported".into());
            }
            if result
                .insert(name.clone(), qualify(&prefix, &name))
                .is_some()
            {
                return Err(format!("duplicate import: {name}"));
            }
            Ok(())
        }
        UseTree::Rename(item) => {
            let name = item.rename.to_string();
            if result
                .insert(name.clone(), qualify(&prefix, &item.ident.to_string()))
                .is_some()
            {
                return Err(format!("duplicate import: {name}"));
            }
            Ok(())
        }
        UseTree::Group(item) => {
            for child in &item.items {
                collect_use(child, prefix.clone(), result)?;
            }
            Ok(())
        }
        UseTree::Glob(_) => Err("glob imports are unsupported in migrated contract sources".into()),
    }
}

#[derive(Default)]
struct FieldOptions<'a> {
    rename_all: Option<&'a str>,
    container_default: bool,
    reserved: Option<&'a str>,
}

pub(crate) struct Renderer<'a> {
    catalog: &'a Catalog,
    definitions: BTreeMap<String, String>,
    owners: BTreeMap<String, (String, Direction)>,
}
impl<'a> Renderer<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self {
            catalog,
            definitions: BTreeMap::new(),
            owners: BTreeMap::new(),
        }
    }
    pub fn definitions(&self) -> String {
        self.definitions
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    pub fn path(&self, module: &str, path: &syn::Path) -> Result<String, String> {
        self.catalog.expand_path(module, path)
    }
    pub fn option_inner<'b>(&self, ty: &'b Type, module: &str) -> Result<Option<&'b Type>, String> {
        let Type::Path(path) = ty else {
            return Ok(None);
        };
        let name = self.path(module, &path.path)?;
        if matches!(
            name.as_str(),
            "Option" | "std::option::Option" | "core::option::Option"
        ) {
            return Ok(Some(one_argument(path)?));
        }
        Ok(None)
    }
    pub fn render(
        &mut self,
        ty: &Type,
        module: &str,
        direction: Direction,
    ) -> Result<String, String> {
        if let Type::Tuple(tuple) = ty {
            if tuple.elems.is_empty() {
                return Ok("null".into());
            }
            return Err("tuple types outside enum payloads are unsupported".into());
        }
        let Type::Path(path) = ty else {
            return Err("unsupported Rust type (references, arrays, functions, macros and traits are not wire DTOs)".into());
        };
        if path.qself.is_some() {
            return Err("qualified associated types are unsupported".into());
        }
        let name = self.path(module, &path.path)?;
        if matches!(
            name.as_str(),
            "Option" | "std::option::Option" | "core::option::Option"
        ) {
            return Ok(format!(
                "({}) | null",
                self.render(one_argument(path)?, module, direction)?
            ));
        }
        if matches!(name.as_str(), "Vec" | "std::vec::Vec" | "alloc::vec::Vec") {
            return Ok(format!(
                "Array<{}>",
                self.render(one_argument(path)?, module, direction)?
            ));
        }
        if name == "std::collections::HashMap" {
            let arguments = type_arguments(path, 2)?;
            let Type::Path(key) = arguments[0] else {
                return Err("HashMap keys must be String".into());
            };
            let key_name = self.path(module, &key.path)?;
            if key.qself.is_some()
                || key
                    .path
                    .segments
                    .iter()
                    .any(|segment| !matches!(segment.arguments, PathArguments::None))
                || !matches!(
                    key_name.as_str(),
                    "String" | "std::string::String" | "alloc::string::String"
                )
            {
                return Err("HashMap keys must be String".into());
            }
            return Ok(format!(
                "Record<string, {}>",
                self.render(arguments[1], module, direction)?
            ));
        }
        if path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
        {
            return Err(format!("unsupported generic type: {name}"));
        }
        if name == "serde_json::Value" {
            self.definitions.insert(
                "JsonObject".into(),
                "export type JsonObject = { [key: string]: JsonValue };".into(),
            );
            self.definitions.insert("JsonValue".into(), "export type JsonValue = null | boolean | number | string | Array<JsonValue> | JsonObject;".into());
            return Ok("JsonValue".into());
        }
        match name.as_str() {
            "String" | "std::string::String" | "alloc::string::String" | "char" => {
                return Ok("string".into())
            }
            "bool" => return Ok("boolean".into()),
            "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
                return Ok("number".into())
            }
            "f32" | "f64" => {
                return Ok(if direction == Direction::Input {
                    "number"
                } else {
                    "number | null"
                }
                .into())
            }
            _ => {}
        }
        self.named(&name, direction)
    }
    fn named(&mut self, name: &str, direction: Direction) -> Result<String, String> {
        let declaration = self
            .catalog
            .declarations
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown or unregistered wire type: {name}"))?;
        if declaration.manual_serde {
            return Err(format!(
                "{name}: hand-written Serialize/Deserialize is unsupported"
            ));
        }
        let base = name.rsplit("::").next().ok_or("empty type name")?;
        let export = if direction == Direction::Input {
            format!("{base}Input")
        } else {
            base.to_string()
        };
        if !export.is_ascii()
            || export.starts_with("r#")
            || matches!(
                export.as_str(),
                "IpcCommands"
                    | "IpcEvents"
                    | "Array"
                    | "Record"
                    | "JsonValue"
                    | "JsonObject"
                    | "type"
                    | "interface"
                    | "class"
                    | "function"
                    | "default"
                    | "import"
                    | "export"
                    | "string"
                    | "number"
                    | "boolean"
                    | "null"
                    | "undefined"
                    | "never"
                    | "any"
                    | "unknown"
            )
        {
            return Err(format!("unsupported TypeScript export name: {export}"));
        }
        if let Some(owner) = self.owners.get(&export) {
            if owner != &(name.to_string(), direction) {
                return Err(format!("colliding DTO export name: {export}"));
            }
            return Ok(export);
        }
        self.owners
            .insert(export.clone(), (name.to_string(), direction));
        // 注册后再递归，允许显式名称引用的递归 DTO，不展开为 any。
        let body = match declaration.definition {
            Definition::Struct(item) => {
                no_generics(&item.generics)?;
                let metadata = attrs::read(&item.attrs, Location::Struct)?;
                self.derive(&metadata, direction, &declaration.module, name)?;
                match item.fields {
                    syn::Fields::Named(fields) => self.fields(
                        &fields,
                        &declaration.module,
                        direction,
                        FieldOptions {
                            rename_all: metadata.rename_all.as_deref(),
                            container_default: metadata.default,
                            ..FieldOptions::default()
                        },
                    )?,
                    _ => return Err(format!("{name}: tuple/unit structs are unsupported")),
                }
            }
            Definition::Enum(item) => {
                no_generics(&item.generics)?;
                let metadata = attrs::read(&item.attrs, Location::Enum)?;
                self.derive(&metadata, direction, &declaration.module, name)?;
                let mut variants = Vec::new();
                let mut names = BTreeSet::new();
                for variant in item.variants {
                    if variant.discriminant.is_some() {
                        return Err("enum discriminants are unsupported".into());
                    }
                    let variant_meta = attrs::read(&variant.attrs, Location::Variant)?;
                    let variant_name = variant_meta.rename.clone().unwrap_or(attrs::rename(
                        &attrs::ident(&variant.ident),
                        metadata.rename_all.as_deref(),
                        true,
                    )?);
                    if !names.insert(variant_name.clone()) {
                        return Err(format!("duplicate serialized enum variant: {variant_name}"));
                    }
                    let fields = match &variant.fields {
                        // 内部标记的空命名分支只输出 tag；与空对象的索引签名相交会排除 tag。
                        syn::Fields::Named(fields)
                            if fields.named.is_empty()
                                && metadata.tag.is_some()
                                && metadata.content.is_none() =>
                        {
                            None
                        }
                        syn::Fields::Named(fields) => Some(
                            self.fields(
                                fields,
                                &declaration.module,
                                direction,
                                FieldOptions {
                                    rename_all: variant_meta.rename_all.as_deref(),
                                    reserved: metadata
                                        .tag
                                        .as_deref()
                                        .filter(|_| metadata.content.is_none()),
                                    ..FieldOptions::default()
                                },
                            )?,
                        ),
                        syn::Fields::Unnamed(fields) => {
                            if metadata.tag.is_some() && metadata.content.is_none() {
                                return Err(
                                    "internally tagged tuple/newtype enum variants are unsupported"
                                        .into(),
                                );
                            }
                            let mut values = Vec::new();
                            for field in &fields.unnamed {
                                if !field.attrs.is_empty() {
                                    return Err(
                                        "tuple enum field attributes are unsupported".into()
                                    );
                                }
                                values.push(self.render(
                                    &field.ty,
                                    &declaration.module,
                                    direction,
                                )?);
                            }
                            Some(if values.len() == 1 {
                                values.remove(0)
                            } else {
                                format!("[{}]", values.join(", "))
                            })
                        }
                        syn::Fields::Unit => None,
                    };
                    let quoted = attrs::quote(&variant_name);
                    variants.push(match (&metadata.tag, &metadata.content, fields) {
                        (None, _, None) => quoted,
                        (None, _, Some(fields)) => format!("{{ {quoted}: {fields} }}"),
                        (Some(tag), _, None) => format!("{{ {}: {quoted} }}", attrs::quote(tag)),
                        (Some(tag), Some(content), Some(fields)) => format!(
                            "{{ {}: {quoted}; {}: {fields} }}",
                            attrs::quote(tag),
                            attrs::quote(content)
                        ),
                        (Some(tag), None, Some(fields)) => {
                            format!("({{ {}: {quoted} }} & {fields})", attrs::quote(tag))
                        }
                    });
                }
                if variants.is_empty() {
                    return Err("empty enums are unsupported".into());
                }
                variants.join(" | ")
            }
            Definition::Alias => {
                return Err(format!(
                    "{}: type aliases are unsupported ({name})",
                    declaration.source
                ))
            }
        };
        self.definitions
            .insert(export.clone(), format!("export type {export} = {body};"));
        Ok(export)
    }
    fn derive(
        &self,
        metadata: &attrs::SerdeAttrs,
        direction: Direction,
        module: &str,
        name: &str,
    ) -> Result<(), String> {
        let required = if direction == Direction::Input {
            "Deserialize"
        } else {
            "Serialize"
        };
        if !metadata.derives.contains(required) {
            return Err(format!("{name}: {required} derive is required"));
        }
        // 显式 imports 不得把同名 serde derive 换成其他宏。
        if self
            .catalog
            .imports
            .get(module)
            .and_then(|imports| imports.get(required))
            .is_some_and(|path| path != &format!("serde::{required}"))
        {
            return Err(format!("{name}: non-serde {required} import"));
        }
        Ok(())
    }
    fn fields(
        &mut self,
        fields: &syn::FieldsNamed,
        module: &str,
        direction: Direction,
        options: FieldOptions<'_>,
    ) -> Result<String, String> {
        let FieldOptions {
            rename_all,
            container_default,
            reserved,
        } = options;
        let mut output = Vec::new();
        let mut names = BTreeSet::new();
        for field in &fields.named {
            let metadata = attrs::read(&field.attrs, Location::Field)?;
            let name = metadata.rename.clone().unwrap_or(attrs::rename(
                &attrs::ident(field.ident.as_ref().ok_or("expected named field")?),
                rename_all,
                false,
            )?);
            if !names.insert(name.clone()) || reserved == Some(name.as_str()) {
                return Err(format!("duplicate/reserved serialized field: {name}"));
            }
            let option = self.option_inner(&field.ty, module)?;
            if metadata.skip_none && option.is_none() {
                return Err("skip_serializing_if Option::is_none requires Option<T>".into());
            }
            if metadata.skip_empty_vec {
                let Type::Path(path) = &field.ty else {
                    return Err("skip_serializing_if Vec::is_empty requires Vec<T>".into());
                };
                if path.qself.is_some()
                    || !matches!(
                        self.path(module, &path.path)?.as_str(),
                        "Vec" | "std::vec::Vec" | "alloc::vec::Vec"
                    )
                {
                    return Err("skip_serializing_if Vec::is_empty requires Vec<T>".into());
                }
                one_argument(path)?;
            }
            let optional = match direction {
                Direction::Input => option.is_some() || metadata.default || container_default,
                Direction::Output => metadata.skip_none || metadata.skip_empty_vec,
            };
            let ty = if direction == Direction::Output && metadata.skip_none {
                option.ok_or("missing Option payload")?
            } else {
                &field.ty
            };
            output.push(format!(
                "{}{}: {};",
                attrs::quote(&name),
                if optional { "?" } else { "" },
                self.render(ty, module, direction)?
            ));
        }
        if output.is_empty() {
            Ok("Record<string, never>".into())
        } else {
            Ok(format!("{{ {} }}", output.join(" ")))
        }
    }
}
pub(crate) fn no_generics(generics: &syn::Generics) -> Result<(), String> {
    if !generics.params.is_empty() || generics.where_clause.is_some() {
        return Err("generic DTOs/commands are unsupported".into());
    }
    Ok(())
}
fn one_argument(path: &syn::TypePath) -> Result<&Type, String> {
    if path
        .path
        .segments
        .iter()
        .take(path.path.segments.len().saturating_sub(1))
        .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return Err("generic module paths are unsupported".into());
    }
    let PathArguments::AngleBracketed(arguments) = &path
        .path
        .segments
        .last()
        .ok_or("empty type path")?
        .arguments
    else {
        return Err("expected one type argument".into());
    };
    if arguments.args.len() != 1 {
        return Err("expected exactly one type argument".into());
    }
    match arguments.args.first() {
        Some(GenericArgument::Type(ty)) => Ok(ty),
        _ => Err("only concrete type arguments are supported".into()),
    }
}

fn type_arguments(path: &syn::TypePath, count: usize) -> Result<Vec<&Type>, String> {
    if path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .take(path.path.segments.len().saturating_sub(1))
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return Err("qualified/generic module paths are unsupported".into());
    }
    let PathArguments::AngleBracketed(arguments) = &path
        .path
        .segments
        .last()
        .ok_or("empty type path")?
        .arguments
    else {
        return Err("expected concrete type arguments".into());
    };
    if arguments.args.len() != count {
        return Err(format!("expected exactly {count} type arguments"));
    }
    arguments
        .args
        .iter()
        .map(|argument| match argument {
            GenericArgument::Type(ty) => Ok(ty),
            _ => Err("only concrete type arguments are supported".into()),
        })
        .collect()
}
