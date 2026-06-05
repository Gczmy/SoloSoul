#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub active_session_key: Option<Vec<u8>>,
}
