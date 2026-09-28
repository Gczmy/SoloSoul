#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nested {
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    #[serde(rename = "explicit-id")]
    pub id: String,
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    #[serde(default)]
    pub sequence: u32,
    pub nested: Vec<Nested>,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ready,
    HTTPReady,
    #[serde(rename = "completed")]
    Done,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum Event {
    Idle,
    Updated(Payload),
    Pair(String, u32),
    #[serde(rename_all = "camelCase")]
    Failed {
        error_message: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Internal {
    Idle,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum External {
    Idle,
    Text(String),
    Pair(String, bool),
    Record { value: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Floating {
    pub score: f64,
    pub weight: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Empty {}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EmptyInternal {
    Empty {},
    Unit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Array {
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub value: String,
}
