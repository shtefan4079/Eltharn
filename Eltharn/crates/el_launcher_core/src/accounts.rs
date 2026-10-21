use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAccountV1 {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub is_microsoft: bool,
    pub refresh_token: Option<String>,
}
impl std::fmt::Debug for StoredAccountV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredAccountV1")
            .field("name", &self.name)
            .field("uuid", &self.uuid)
            .field("is_microsoft", &self.is_microsoft)
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedAccounts {
    Current(Vec<StoredAccountV1>),
    MigratedLegacyNames(Vec<StoredAccountV1>),
}
pub fn decode_legacy_accounts(json: &str) -> Option<DecodedAccounts> {
    if let Ok(accounts) = serde_json::from_str::<Vec<StoredAccountV1>>(json) {
        return Some(DecodedAccounts::Current(accounts));
    }
    serde_json::from_str::<Vec<String>>(json)
        .ok()
        .map(migrate_legacy_names)
        .map(DecodedAccounts::MigratedLegacyNames)
}
pub fn encode_legacy_accounts(accounts: &[StoredAccountV1]) -> Result<String, serde_json::Error> {
    serde_json::to_string(accounts)
}
pub fn migrate_legacy_names(names: Vec<String>) -> Vec<StoredAccountV1> {
    names.into_iter().map(make_legacy_offline_account).collect()
}
pub fn legacy_offline_uuid(name: &str) -> String {
    el_protocol_api::ClientConnectRequest::offline("account-identity-only", name)
        .uuid
        .to_string()
}
pub fn make_legacy_offline_account(name: String) -> StoredAccountV1 {
    StoredAccountV1 {
        uuid: legacy_offline_uuid(&name),
        name,
        access_token: "null".to_string(),
        is_microsoft: false,
        refresh_token: None,
    }
}
pub fn normalize_active_account_index(
    accounts: &[StoredAccountV1],
    configured_index: usize,
) -> usize {
    if configured_index < accounts.len() {
        configured_index
    } else {
        0
    }
}
pub fn add_offline_account_legacy(
    accounts: &mut Vec<StoredAccountV1>,
    active_account: &mut usize,
    name: String,
) -> Option<usize> {
    if name.is_empty() || accounts.iter().any(|account| account.name == name) {
        return None;
    }
    accounts.push(make_legacy_offline_account(name));
    let index = accounts.len() - 1;
    *active_account = index;
    Some(index)
}
pub fn select_account(
    accounts: &[StoredAccountV1],
    active_account: &mut usize,
    index: usize,
) -> bool {
    if index >= accounts.len() {
        return false;
    }
    *active_account = index;
    true
}
pub fn delete_account(
    accounts: &mut Vec<StoredAccountV1>,
    active_account: &mut usize,
    index: usize,
) -> Option<StoredAccountV1> {
    if index >= accounts.len() {
        return None;
    }
    let removed = accounts.remove(index);
    if accounts.is_empty() {
        *active_account = 0;
    } else if *active_account >= accounts.len() {
        *active_account = accounts.len().saturating_sub(1);
    } else if index < *active_account {
        *active_account = (*active_account).saturating_sub(1);
    }
    Some(removed)
}
pub fn upsert_account_by_name_legacy(
    accounts: &mut Vec<StoredAccountV1>,
    active_account: &mut usize,
    account: StoredAccountV1,
) -> usize {
    let name = account.name.clone();
    if let Some(index) = accounts.iter().position(|stored| stored.name == name) {
        accounts[index] = account;
    } else {
        accounts.push(account);
    }
    let selected_index = accounts
        .iter()
        .position(|stored| stored.name == name)
        .unwrap_or(0);
    *active_account = selected_index;
    selected_index
}
