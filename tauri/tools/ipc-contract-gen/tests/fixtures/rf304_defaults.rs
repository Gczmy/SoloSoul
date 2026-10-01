#[derive(serde::Serialize, serde::Deserialize)]
pub struct Request {
    #[serde(default = "default_locale")]
    pub locale: String,
    pub nullable: Option<String>,
}
fn default_locale() -> String {
    "en-US".into()
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default = "default_container")]
pub struct Container {
    pub locale: String,
    pub count: u32,
}
fn default_container() -> Container {
    Container {
        locale: default_locale(),
        count: 7,
    }
}
