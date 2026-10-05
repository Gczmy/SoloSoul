//! PDF结构诊断只保存有界分类、数量、几何和frame身份，不返回HTML/文本/URL。
use super::{identifier, url_class, Outcome};
use serde_json::{json, Value};
use std::collections::BTreeSet;
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn number(v: &Value, max: u64) -> bool {
    v.as_u64().is_some_and(|n| n <= max)
}
fn geometry(v: &Value, embed: bool) -> bool {
    let keys = if embed {
        &["kind", "sourceClass", "visible", "width", "height"][..]
    } else {
        &["visible", "width", "height"][..]
    };
    exact(v, keys)
        && v["visible"].is_boolean()
        && number(&v["width"], 16384)
        && number(&v["height"], 16384)
        && (!embed
            || (matches!(v["kind"].as_str(), Some("embed" | "object" | "iframe"))
                && matches!(
                    v["sourceClass"].as_str(),
                    Some("owned-pdf" | "component-extension" | "blank" | "other")
                )))
}
pub(super) fn valid(v: &Value) -> bool {
    if !exact(
        v,
        &[
            "schemaVersion",
            "scope",
            "documentReadyState",
            "visibilityState",
            "scannedElements",
            "openShadowRoots",
            "truncated",
            "counts",
            "customTags",
            "embeds",
            "canvases",
        ],
    ) || v["schemaVersion"] != 1
        || v["scope"] != "windows-native-sdk-pdf-structure"
        || !matches!(
            v["documentReadyState"].as_str(),
            Some("loading" | "interactive" | "complete")
        )
        || !matches!(v["visibilityState"].as_str(), Some("visible" | "hidden"))
        || !number(&v["scannedElements"], 512)
        || !number(&v["openShadowRoots"], 8)
        || !v["truncated"].is_boolean()
    {
        return false;
    }
    let c = &v["counts"];
    let kinds = [
        "embed",
        "object",
        "iframe",
        "canvas",
        "pdfViewer",
        "customElements",
    ];
    if !exact(c, &kinds)
        || kinds
            .iter()
            .any(|k| !number(&c[*k], v["scannedElements"].as_u64().unwrap()))
    {
        return false;
    }
    let n = |key: &str| c[key].as_u64().unwrap();
    if n("embed") + n("object") + n("iframe") + n("canvas") > v["scannedElements"].as_u64().unwrap()
        || n("pdfViewer") > n("customElements")
        || v["openShadowRoots"].as_u64().unwrap() > v["scannedElements"].as_u64().unwrap()
    {
        return false;
    }
    let (Some(tags), Some(embeds), Some(canvases)) = (
        v["customTags"].as_array(),
        v["embeds"].as_array(),
        v["canvases"].as_array(),
    ) else {
        return false;
    };
    if tags.len() > 32
        || embeds.len() > 8
        || canvases.len() > 8
        || tags.len() as u64 > n("customElements")
        || embeds.len() as u64 > n("embed") + n("object") + n("iframe")
        || canvases.len() as u64 > n("canvas")
    {
        return false;
    }
    let mut seen = BTreeSet::new();
    for tag in tags {
        let Some(tag) = tag.as_str() else {
            return false;
        };
        if tag.is_empty()
            || tag.len() > 64
            || !tag.as_bytes()[0].is_ascii_lowercase()
            || !tag.contains('-')
            || !tag
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !seen.insert(tag)
        {
            return false;
        }
    }
    embeds.iter().all(|e| geometry(e, true)) && canvases.iter().all(|c| geometry(c, false))
}
pub(super) fn frames(raw: &Value, expected: &Value, pdf: &str) -> Outcome<Value> {
    fn walk(
        tree: &Value,
        parent: Option<&str>,
        pdf: &str,
        depth: usize,
        result: &mut Vec<Value>,
        seen: &mut BTreeSet<String>,
    ) -> Outcome<()> {
        let f = &tree["frame"];
        let url = f["url"].as_str().ok_or("pdf-session-tree-invalid")?;
        if depth > 3
            || result.len() >= 8
            || !identifier(&f["id"])
            || url.len() > 4096
            || parent.is_some_and(|id| f["parentId"] != id)
        {
            return Err("pdf-session-tree-invalid");
        }
        let id = f["id"].as_str().unwrap();
        if !seen.insert(id.to_owned()) {
            return Err("pdf-session-tree-invalid");
        }
        let loader = match f.get("loaderId") {
            None | Some(Value::Null) => Value::Null,
            Some(Value::String(s)) if s.is_empty() => Value::Null,
            Some(v) if identifier(v) => v.clone(),
            _ => return Err("pdf-session-tree-invalid"),
        };
        result.push(
            json!({"id":id,"parentId":parent,"loaderId":loader,"urlClass":url_class(url,pdf)}),
        );
        if let Some(children) = tree.get("childFrames") {
            let children = children.as_array().ok_or("pdf-session-tree-invalid")?;
            if children.len() > 8 {
                return Err("pdf-session-tree-invalid");
            }
            for child in children {
                walk(child, Some(id), pdf, depth + 1, result, seen)?;
            }
        }
        Ok(())
    }
    let mut result = Vec::new();
    walk(
        &raw["frameTree"],
        None,
        pdf,
        0,
        &mut result,
        &mut BTreeSet::new(),
    )?;
    if ["id", "loaderId", "urlClass"]
        .iter()
        .any(|k| result[0][*k] != expected[*k])
    {
        return Err("pdf-session-tree-invalid");
    }
    Ok(json!(result))
}
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn empty() -> Value {
        json!({"schemaVersion":1,"scope":"windows-native-sdk-pdf-structure","documentReadyState":"complete","visibilityState":"visible","scannedElements":0,"openShadowRoots":0,"truncated":false,"counts":{"embed":0,"object":0,"iframe":0,"canvas":0,"pdfViewer":0,"customElements":0},"customTags":[],"embeds":[],"canvases":[]})
    }
    #[test]
    fn structure_rejects_payload_keys_inconsistent_counts_and_limits() {
        assert!(valid(&empty()));
        for (k, v) in [
            ("scannedElements", json!(513)),
            ("openShadowRoots", json!(9)),
            ("documentReadyState", json!("PRIVATE")),
            ("visibilityState", json!(null)),
            ("truncated", json!(0)),
            ("privateText", json!("PRIVATE")),
        ] {
            let mut x = empty();
            x[k] = v;
            assert!(!valid(&x));
        }
        let mut x = empty();
        x["counts"]["iframe"] = json!(1);
        assert!(!valid(&x));
    }
    #[test]
    fn structure_rejects_unbounded_or_duplicate_tags_and_raw_sources() {
        let mut x = empty();
        x["scannedElements"] = json!(4);
        x["counts"]["customElements"] = json!(2);
        x["customTags"] = json!(["pdf-viewer"]);
        assert!(valid(&x));
        for tags in [
            json!(["pdf-viewer", "pdf-viewer"]),
            json!(["private text"]),
            json!(["script"]),
            json!(["-bad"]),
        ] {
            x["customTags"] = tags;
            assert!(!valid(&x));
        }
        x["customTags"] = json!([]);
        x["counts"]["iframe"] = json!(1);
        x["embeds"] = json!([{"kind":"iframe","sourceClass":"owned-pdf","visible":true,"width":10,"height":10}]);
        assert!(valid(&x));
        x["embeds"][0]["src"] = json!("PRIVATE");
        assert!(!valid(&x));
    }
    #[test]
    fn frame_tree_keeps_only_identity_and_source_class() {
        let raw = json!({"frameTree":{"frame":{"id":"R","loaderId":"L","url":"PDF","name":"PRIVATE"},"childFrames":[{"frame":{"id":"C","parentId":"R","url":"http://private.invalid/secret","name":"PRIVATE"}}]}});
        let expected = json!({"id":"R","loaderId":"L","urlClass":"owned-pdf"});
        let proof = frames(&raw, &expected, "PDF").unwrap();
        assert_eq!(proof[1]["loaderId"], Value::Null);
        assert_eq!(proof[1]["urlClass"], "other");
        assert!(!proof.to_string().contains("private"));
        assert!(!proof.to_string().contains("PRIVATE"));
        for (k, v) in [
            ("id", json!("R")),
            ("parentId", json!("FOREIGN")),
            ("loaderId", json!(false)),
        ] {
            let mut bad = raw.clone();
            bad["frameTree"]["childFrames"][0]["frame"][k] = v;
            assert!(frames(&bad, &expected, "PDF").is_err());
        }
    }
    #[test]
    fn frame_tree_rejects_budget_and_root_replacement() {
        let expected = json!({"id":"R","loaderId":"L","urlClass":"owned-pdf"});
        let mut raw = json!({"frameTree":{"frame":{"id":"R","loaderId":"L","url":"PDF"}}});
        assert!(frames(&raw, &expected, "PDF").is_ok());
        raw["frameTree"]["frame"]["loaderId"] = json!("REPLACED");
        assert!(frames(&raw, &expected, "PDF").is_err());
        raw["frameTree"]["frame"]["loaderId"] = json!("L");
        raw["frameTree"]["childFrames"] = json!((0..8)
            .map(|n| json!({"frame":{"id":format!("C{n}"),"parentId":"R","url":"PDF"}}))
            .collect::<Vec<_>>());
        assert!(frames(&raw, &expected, "PDF").is_err());
    }
}
