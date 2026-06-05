#[derive(Debug, Clone, Default)]
pub struct VaultRuntimeState {
    pub is_unlocked: bool,
    pub current_account_id: Option<String>,
}
