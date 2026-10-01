//! Runtime 泛型只属于 Host 注入；不向 JSON 契约引入泛型或未知占位。
use super::attrs;
use super::types::Renderer;
use syn::visit::{self, Visit};
use syn::{GenericArgument, GenericParam, Generics, PathArguments, Type, TypeParamBound};

pub(crate) fn parameter(
    renderer: &Renderer<'_>,
    module: &str,
    generics: &Generics,
) -> Result<Option<String>, String> {
    if generics.params.is_empty() && generics.where_clause.is_none() {
        return Ok(None);
    }
    let unsupported =
        || "only a single tauri::Runtime injection parameter is supported".to_string();
    if generics.params.len() != 1 || generics.where_clause.is_some() {
        return Err(unsupported());
    }
    let GenericParam::Type(parameter) = &generics.params[0] else {
        return Err(unsupported());
    };
    if !parameter.attrs.is_empty() || parameter.default.is_some() || parameter.bounds.len() != 1 {
        return Err(unsupported());
    }
    let TypeParamBound::Trait(bound) = &parameter.bounds[0] else {
        return Err(unsupported());
    };
    if bound.paren_token.is_some()
        || bound.lifetimes.is_some()
        || !matches!(bound.modifier, syn::TraitBoundModifier::None)
        || bound
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
        || renderer.path(module, &bound.path)? != "tauri::Runtime"
    {
        return Err(unsupported());
    }
    Ok(Some(attrs::ident(&parameter.ident)))
}

pub(crate) fn contains(ty: &Type, parameter: &str) -> bool {
    struct Finder<'a> {
        parameter: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if path
                .segments
                .first()
                .is_some_and(|segment| attrs::ident(&segment.ident) == self.parameter)
            {
                self.found = true;
            }
            visit::visit_path(self, path);
        }
    }
    let mut finder = Finder {
        parameter,
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

pub(crate) fn injection(
    renderer: &Renderer<'_>,
    module: &str,
    ty: &Type,
    parameter: &str,
) -> Result<bool, String> {
    let Type::Path(path) = ty else {
        return Ok(false);
    };
    if path.qself.is_some() {
        return Ok(false);
    }
    let name = renderer.path(module, &path.path)?;
    if !matches!(
        name.as_str(),
        "tauri::AppHandle" | "tauri::Window" | "tauri::WebviewWindow" | "tauri::Webview"
    ) {
        return Ok(false);
    }
    let Some(last) = path.path.segments.last() else {
        return Ok(false);
    };
    if path
        .path
        .segments
        .iter()
        .take(path.path.segments.len() - 1)
        .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return Ok(false);
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Ok(false);
    };
    if arguments.args.len() != 1 {
        return Ok(false);
    }
    let GenericArgument::Type(Type::Path(arg)) = &arguments.args[0] else {
        return Ok(false);
    };
    Ok(arg.qself.is_none()
        && arg.path.leading_colon.is_none()
        && arg.path.segments.len() == 1
        && attrs::ident(&arg.path.segments[0].ident) == parameter
        && matches!(arg.path.segments[0].arguments, PathArguments::None))
}
