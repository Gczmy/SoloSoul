//! Fixed settings UI traversal, dark frame capture and real preference/sidebar restoration.
//! Available only after the parent owned surface request and exact scene request match.
use super::super::{parse_frame_policy, theme_source, validate_observer};
use super::{bytes, exact, number, publish, Capture, Outcome};
use serde_json::{json, Value};

const FILE: &str = "native-perf-surface-appearance-request.json";
const SCRIPT: &str = include_str!("sdk_surface_appearance.js");
const HOME: &str = "http://tauri.localhost/";
const SETTINGS: &str = "http://tauri.localhost/settings";
const APPEARANCE: &str = "http://tauri.localhost/settings/appearance";

fn request(value: &Value, parent: &Value) -> Outcome<bool> {
    let collapsed = match value["scene"].as_str() {
        Some("dark-expanded") => false,
        Some("dark-collapsed") => true,
        _ => return Err("appearance-scene-request-invalid"),
    };
    let mut expected = parent.clone();
    expected["scope"] = json!("windows-native-appearance-scene-request");
    expected["scene"] = value["scene"].clone();
    if expected != *value {
        return Err("appearance-scene-request-invalid");
    }
    Ok(collapsed)
}
pub(super) fn requested(c: &Capture, parent: &Value) -> Outcome<Option<Value>> {
    let path = c.config.root.join(FILE);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("appearance-scene-request-invalid"),
        Ok(_) => {}
    }
    let value = serde_json::from_slice(&bytes(&path, 4096)?)
        .map_err(|_| "appearance-scene-request-invalid")?;
    request(&value, parent)?;
    Ok(Some(value))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Settings,
    Appearance,
    Dark,
    Original,
    Home,
    Toggle,
    Recover,
    Done,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Appearance => "appearance",
            Self::Dark => "dark",
            Self::Original => "original",
            Self::Home => "home",
            Self::Toggle => "toggle",
            Self::Recover => "recover",
            Self::Done => "done",
        }
    }
    fn parse(value: &Value) -> Outcome<Self> {
        match value.as_str() {
            Some("settings") => Ok(Self::Settings),
            Some("appearance") => Ok(Self::Appearance),
            Some("dark") => Ok(Self::Dark),
            Some("original") => Ok(Self::Original),
            Some("home") => Ok(Self::Home),
            Some("toggle") => Ok(Self::Toggle),
            Some("done") => Ok(Self::Done),
            _ => Err("appearance-scene-probe-invalid"),
        }
    }
    fn source(self) -> &'static str {
        match self {
            Self::Settings | Self::Toggle | Self::Done => HOME,
            Self::Appearance => SETTINGS,
            _ => APPEARANCE,
        }
    }
    fn after(self) -> &'static str {
        match self {
            Self::Settings => SETTINGS,
            Self::Appearance => APPEARANCE,
            Self::Home | Self::Toggle => HOME,
            _ => APPEARANCE,
        }
    }
    fn text_input(self) -> u64 {
        u64::from(matches!(self, Self::Dark | Self::Original))
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Preference {
    Light,
    Dark,
    System,
}
impl Preference {
    fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
    fn parse(value: &Value) -> Outcome<Self> {
        match value.as_str() {
            Some("light") => Ok(Self::Light),
            Some("dark") => Ok(Self::Dark),
            Some("system") => Ok(Self::System),
            _ => Err("appearance-scene-preference-invalid"),
        }
    }
}
fn probe(raw: &Value, run: &str, clock: &Value, minimum: f64) -> Outcome<Value> {
    let p = &raw["result"]["value"];
    if raw.get("exceptionDetails").is_some()
        || raw["result"]["type"] != "object"
        || !exact(
            p,
            &[
                "schemaVersion",
                "scope",
                "action",
                "href",
                "timeOriginMs",
                "atMs",
                "theme",
                "platform",
                "expanded",
                "viewport",
                "focused",
                "visibility",
                "layout",
                "settled",
                "target",
                "checked",
                "radios",
                "inputTrust",
                "observer",
            ],
        )
        || p["schemaVersion"] != 1
        || p["scope"] != "windows-native-appearance-probe"
        || p["timeOriginMs"] != *clock
        || !number(&p["atMs"], minimum, 300_000.0)
        || !matches!(p["theme"].as_str(), Some("light" | "dark"))
        || p["platform"] != "windows"
        || !p["expanded"].is_boolean()
        || p["focused"] != true
        || p["visibility"] != "visible"
        || p["settled"] != true
        || !exact(&p["inputTrust"], &["pointer", "text", "untrusted"])
        || p["inputTrust"]["pointer"]
            .as_u64()
            .is_none_or(|n| !(10..=32).contains(&n))
        || p["inputTrust"]["text"]
            .as_u64()
            .is_none_or(|n| !(3..=5).contains(&n))
        || p["inputTrust"]["untrusted"] != 0
    {
        return Err("appearance-scene-probe-invalid");
    }
    let action = Action::parse(&p["action"])?;
    if p["href"] != action.source() {
        return Err("appearance-scene-route-invalid");
    }
    let viewport = p["viewport"]
        .as_array()
        .ok_or("appearance-scene-probe-invalid")?;
    let layout = p["layout"]
        .as_array()
        .ok_or("appearance-scene-probe-invalid")?;
    if viewport.len() != 3
        || !number(&viewport[0], 1.0, 16384.0)
        || !number(&viewport[1], 1.0, 16384.0)
        || !number(&viewport[2], 0.25, 8.0)
        || layout.len() != 3
        || layout.iter().any(|r| {
            !r.as_array().is_some_and(|r| {
                r.len() == 4
                    && r[..2].iter().all(|v| number(v, -16384.0, 16384.0))
                    && r[2..].iter().all(|v| number(v, 1.0, 16384.0))
            })
        })
    {
        return Err("appearance-scene-probe-invalid");
    }
    if action == Action::Done {
        if !p["target"].is_null() {
            return Err("appearance-scene-probe-invalid");
        }
    } else if !exact(&p["target"], &["x", "y", "actionable"])
        || p["target"]["actionable"] != true
        || !number(&p["target"]["x"], 0.0, viewport[0].as_f64().unwrap() - 1.0)
        || !number(&p["target"]["y"], 0.0, viewport[1].as_f64().unwrap() - 1.0)
    {
        return Err("appearance-scene-probe-invalid");
    }
    if p["href"] == APPEARANCE {
        let selected = Preference::parse(&p["checked"])?;
        let expected = match selected {
            Preference::Light => json!([true, false, false]),
            Preference::Dark => json!([false, true, false]),
            Preference::System => json!([false, false, true]),
        };
        if p["radios"] != expected {
            return Err("appearance-scene-preference-invalid");
        }
    } else if !p["checked"].is_null() || p["radios"] != json!([]) {
        return Err("appearance-scene-preference-invalid");
    }
    validate_observer(&p["observer"], run, clock)?;
    Ok(p.clone())
}
struct Spec {
    action: Action,
    theme: Option<&'static str>,
    checked: Option<Preference>,
    original: Option<Preference>,
    verified: bool,
}
impl Spec {
    fn fixed(action: Action, theme: &'static str, checked: Option<Preference>) -> Self {
        Self {
            action,
            theme: Some(theme),
            checked,
            original: None,
            verified: false,
        }
    }
}
async fn observe(c: &mut Capture, spec: &Spec, minimum: f64) -> Outcome<Value> {
    let clock = c.time_origin.clone().ok_or("surface-binding-unavailable")?;
    let parameters = json!({"action":spec.action.name(),"theme":spec.theme,"checked":spec.checked.map(Preference::name),"original":spec.original.map(Preference::name),"verified":spec.verified});
    let raw=c.protocol("Runtime.evaluate",json!({"expression":SCRIPT.replace("__SCENE_PROBE__",&parameters.to_string()),"returnByValue":true,"awaitPromise":true})).await?;
    let mut p = probe(&raw, &c.config.run_id, &clock, minimum)?;
    p["preferenceWrites"] = crate::native_perf::preference_trace::snapshot(&c.config.run_id)
        .ok_or("appearance-preference-observation-unavailable")?;
    if (spec.action != Action::Recover && p["action"] != spec.action.name())
        || spec.theme.is_some_and(|t| p["theme"] != t)
        || spec.checked.is_some_and(|t| p["checked"] != t.name())
    {
        return Err("appearance-scene-state-invalid");
    }
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    let source = p["href"].as_str().ok_or("appearance-scene-route-invalid")?;
    if !theme_source(source) {
        return Err("appearance-scene-route-invalid");
    }
    let frame = parse_frame_policy(&tree, source, true)?;
    let anchor = c.frame.as_ref().ok_or("surface-binding-unavailable")?;
    if frame["mainFrameId"] != anchor["mainFrameId"] || frame["loaderId"] != anchor["loaderId"] {
        return Err("surface-document-replaced");
    }
    Ok(p)
}
fn transition(before: &Value, after: &Value) -> Outcome<()> {
    let action = Action::parse(&before["action"])?;
    if matches!(action, Action::Done | Action::Recover)
        || after["href"] != action.after()
        || after["viewport"] != before["viewport"]
        || after["inputTrust"]["pointer"].as_u64()
            != before["inputTrust"]["pointer"]
                .as_u64()
                .and_then(|n| n.checked_add(1))
        || after["inputTrust"]["text"].as_u64()
            != before["inputTrust"]["text"]
                .as_u64()
                .and_then(|n| n.checked_add(action.text_input()))
        || (action == Action::Toggle
            && after["expanded"].as_bool() != before["expanded"].as_bool().map(|b| !b))
        || (action != Action::Toggle && after["expanded"] != before["expanded"])
    {
        return Err("appearance-scene-transition-invalid");
    }
    super::super::ipc_delta(&before["observer"], &after["observer"])?;
    Ok(())
}
async fn click(c: &mut Capture, before: &Value) -> Outcome<()> {
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol("Input.dispatchMouseEvent",json!({"type":kind,"x":before["target"]["x"],"y":before["target"]["y"],"button":"left","clickCount":1})).await?;
    }
    Ok(())
}
struct Scene {
    initial: Value,
    steps: Vec<Value>,
    original: Option<Preference>,
    collapsed: bool,
}
impl Scene {
    fn last(&self) -> &Value {
        self.steps.last().unwrap_or(&self.initial)
    }
}
async fn advance(c: &mut Capture, state: &mut Scene, spec: Spec) -> Outcome<()> {
    click(c, state.last()).await?;
    let after = observe(c, &spec, state.last()["atMs"].as_f64().unwrap()).await?;
    transition(state.last(), &after)?;
    state.steps.push(after);
    Ok(())
}
async fn prepare(c: &mut Capture, state: &mut Scene, req: &Value) -> Outcome<()> {
    advance(c, state, Spec::fixed(Action::Appearance, "light", None)).await?;
    advance(c, state, Spec::fixed(Action::Dark, "light", None)).await?;
    let original = Preference::parse(&state.last()["checked"])?;
    if original == Preference::Dark {
        return Err("appearance-scene-initial-preference-invalid");
    }
    state.original = Some(original);
    advance(
        c,
        state,
        Spec::fixed(Action::Home, "dark", Some(Preference::Dark)),
    )
    .await?;
    advance(c, state, Spec::fixed(Action::Toggle, "dark", None)).await?;
    if state.collapsed {
        advance(c, state, Spec::fixed(Action::Toggle, "dark", None)).await?;
    }
    if state.last()["expanded"] != !state.collapsed {
        return Err("appearance-scene-sidebar-invalid");
    }
    publish(
        &c.config.root,
        "native-perf-surface-appearance-prepared.json",
        &json!({"schemaVersion":1,"scope":"windows-native-appearance-prepared","request":req,"binding":c.frame,"initial":state.initial,"steps":state.steps,"originalPreference":state.original.map(Preference::name),"performanceMetrics":null}),
    )
}
async fn restore(c: &mut Capture, state: &Scene, req: &Value) -> Outcome<()> {
    let mut verified = false;
    let mut steps = Vec::new();
    let mut minimum = state.last()["atMs"].as_f64().unwrap();
    let mut previous: Option<Value> = None;
    for _ in 0..7 {
        let spec = Spec {
            action: Action::Recover,
            theme: None,
            checked: None,
            original: state.original,
            verified,
        };
        let p = observe(c, &spec, minimum).await?;
        if let Some(before) = &previous {
            transition(before, &p)?;
        }
        if p["href"] == APPEARANCE
            && state.original.is_some_and(|t| p["checked"] == t.name())
            && p["theme"] == "light"
        {
            verified = true;
        }
        let action = Action::parse(&p["action"])?;
        steps.push(p.clone());
        if action == Action::Done {
            if p["theme"] != "light"
                || p["expanded"] != true
                || p["viewport"] != state.initial["viewport"]
                || p["layout"][0] != state.initial["layout"][0]
                || (state.original.is_some() && !verified)
            {
                return Err("appearance-scene-restore-invalid");
            }
            return publish(
                &c.config.root,
                "native-perf-surface-appearance-restored.json",
                &json!({"schemaVersion":1,"scope":"windows-native-appearance-restored","request":req,"binding":c.frame,"steps":steps,"originalPreference":state.original.map(Preference::name),"preferenceVerified":verified,"performanceMetrics":null}),
            );
        }
        if !matches!(
            action,
            Action::Toggle
                | Action::Settings
                | Action::Appearance
                | Action::Original
                | Action::Home
        ) {
            return Err("appearance-scene-recovery-action-invalid");
        }
        click(c, &p).await?;
        minimum = p["atMs"].as_f64().unwrap();
        previous = Some(p);
    }
    Err("appearance-scene-restore-budget-exceeded")
}
pub(super) async fn capture(
    c: &mut Capture,
    parent: &Value,
    req: &Value,
    minimum: f64,
) -> Outcome<()> {
    let collapsed = request(req, parent)?;
    let before_calls = c.calls.len();
    let initial = observe(c, &Spec::fixed(Action::Settings, "light", None), minimum).await?;
    if initial["expanded"] != true
        || c.last.as_ref().map(|p| &p["inputTrust"]) != Some(&initial["inputTrust"])
    {
        return Err("appearance-scene-initial-state-invalid");
    }
    let mut state = Scene {
        initial,
        steps: Vec::new(),
        original: None,
        collapsed,
    };
    c.theme_scene = true;
    c.step = "appearance-scene";
    let captured = async {
        prepare(c, &mut state, req).await?;
        super::capture_frame(c, parent, minimum).await
    }
    .await;
    // A failed input/capture still gets a bounded real-UI restoration attempt.
    let restored = restore(c, &state, req).await;
    c.theme_scene = false;
    publish(
        &c.config.root,
        "native-perf-surface-appearance-result.json",
        &json!({"schemaVersion":1,"scope":"windows-native-appearance-result","request":req,"captureError":captured.as_ref().err(),"restoreError":restored.as_ref().err(),"success":captured.is_ok()&&restored.is_ok(),"callsBefore":before_calls,"callsAfter":c.calls.len(),"performanceMetrics":null}),
    )?;
    captured?;
    restored
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_request_is_exact_bound_and_only_has_two_dark_scenes() {
        let parent = json!({"schemaVersion":1,"scope":"windows-native-surface-request","runId":"run","pid":1,"browserPid":2,"token":"0123456789abcdef0123456789abcdef"});
        let mut req = parent.clone();
        req["scope"] = json!("windows-native-appearance-scene-request");
        req["scene"] = json!("dark-expanded");
        assert!(!request(&req, &parent).unwrap());
        req["scene"] = json!("dark-collapsed");
        assert!(request(&req, &parent).unwrap());
        for (key, value) in [
            ("pid", json!(3)),
            ("browserPid", json!(3)),
            ("runId", json!("old")),
            ("token", json!("old")),
            ("scene", json!("system")),
            ("expression", json!("arbitrary")),
        ] {
            let mut bad = req.clone();
            bad[key] = value;
            assert!(request(&bad, &parent).is_err());
        }
    }
    fn sample() -> Value {
        json!({"result":{"type":"object","value":{"schemaVersion":1,"scope":"windows-native-appearance-probe","action":"dark","href":APPEARANCE,"timeOriginMs":10,"atMs":100,"theme":"light","platform":"windows","expanded":true,"viewport":[1200,800,1],"focused":true,"visibility":"visible","layout":[[0,0,186,800],[186,0,1014,48],[186,48,1014,752]],"settled":true,"target":{"x":500,"y":150,"actionable":true},"checked":"system","radios":[false,false,true],"inputTrust":{"pointer":12,"text":3,"untrusted":0},"observer":{"schemaVersion":1,"scope":"windows-native-tauri-invoke-observer","runId":"run","valid":true,"invalidReasons":[],"total":0,"observedCount":0,"maxEvents":16384,"timeOriginMs":10,"installedAtMs":0,"commands":[]}}}})
    }
    #[test]
    fn appearance_probe_rejects_foreign_route_clock_payload_input_and_radio_disagreement() {
        assert!(probe(&sample(), "run", &json!(10), 99.0).is_ok());
        for (key, value) in [
            ("href", json!("http://tauri.localhost/settings/account")),
            ("timeOriginMs", json!(11)),
            ("atMs", json!(98)),
            ("target", json!({"x":1200,"y":150,"actionable":true})),
            ("focused", json!(false)),
            ("settled", json!(false)),
            ("radios", json!([true, true, false])),
            ("inputTrust", json!({"pointer":12,"text":3,"untrusted":1})),
            ("payload", json!("secret")),
        ] {
            let mut bad = sample();
            bad["result"]["value"][key] = value;
            assert!(probe(&bad, "run", &json!(10), 99.0).is_err());
        }
    }
    #[test]
    fn radio_transition_requires_both_real_pointer_and_radio_input_events() {
        let before = sample()["result"]["value"].clone();
        let mut after = before.clone();
        after["action"] = json!("home");
        after["theme"] = json!("dark");
        after["checked"] = json!("dark");
        after["radios"] = json!([false, true, false]);
        after["inputTrust"] = json!({"pointer":13,"text":4,"untrusted":0});
        assert!(transition(&before, &after).is_ok());
        for (key, value) in [
            ("inputTrust", json!({"pointer":13,"text":3,"untrusted":0})),
            ("inputTrust", json!({"pointer":12,"text":4,"untrusted":0})),
            ("viewport", json!([1000, 800, 1])),
            ("expanded", json!(false)),
            ("href", json!(HOME)),
        ] {
            let mut bad = after.clone();
            bad[key] = value;
            assert!(transition(&before, &bad).is_err());
        }
    }
}
