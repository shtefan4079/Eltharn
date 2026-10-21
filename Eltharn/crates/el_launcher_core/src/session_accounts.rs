use crate::StoredAccountV1;
use el_protocol_api::{ClientConnectRequest, Uuid};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};
pub const MAX_ACCOUNTS: usize = 256;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountHandle(String);
impl AccountHandle {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Offline,
    Microsoft,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccountSummary {
    pub handle: AccountHandle,
    pub name: String,
    pub uuid: String,
    pub kind: AccountKind,
    pub online_ready: bool,
    pub selected: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldSwitchGuard {
    pub player: bool,
    pub level: bool,
    pub connection: bool,
    pub camera_entity: bool,
    pub game_mode: bool,
    pub integrated_server: bool,
}
impl WorldSwitchGuard {
    pub fn is_main_menu(self) -> bool {
        !(self.player
            || self.level
            || self.connection
            || self.camera_entity
            || self.game_mode
            || self.integrated_server)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginMode {
    Offline,
    Online,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountError {
    InvalidNamespace,
    InvalidName,
    InvalidUuid,
    AccountLimit,
    MissingAccount,
    DuplicateAccount,
    WorldActive,
    AuthenticationRequired,
    IdentityMismatch,
    StalePreparation,
    CounterExhausted,
}
impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "account selection {:?}", self)
    }
}
impl std::error::Error for AccountError {}
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialHandle(String);
impl CredentialHandle {
    pub fn from_engine_reference(reference: String) -> Result<Self, AccountError> {
        if reference.is_empty() || reference.len() > 160 || reference.chars().any(char::is_control)
        {
            return Err(AccountError::AuthenticationRequired);
        }
        Ok(Self(reference))
    }
    pub fn engine_reference(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for CredentialHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CredentialHandle(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineAuthenticatedSession {
    pub name: String,
    pub uuid: Uuid,
    pub credential: CredentialHandle,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedSession {
    pub account: AccountHandle,
    pub name: String,
    pub uuid: Uuid,
    pub mode: LoginMode,
    pub credential: Option<CredentialHandle>,
}
#[derive(Clone, Debug)]
pub struct PreparedSwitch {
    namespace: u64,
    revision: u64,
    target: AccountHandle,
    mode: LoginMode,
}
struct Account {
    handle: AccountHandle,
    name: String,
    uuid: Uuid,
    kind: AccountKind,
    authenticated: Option<EngineAuthenticatedSession>,
}
pub struct SessionAccountRegistry {
    namespace: u64,
    revision: u64,
    next_handle: u64,
    accounts: BTreeMap<AccountHandle, Account>,
    order: Vec<AccountHandle>,
    selected: Option<SelectedSession>,
}
impl fmt::Debug for SessionAccountRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionAccountRegistry")
            .field("accounts", &self.accounts.len())
            .field("selected", &self.selected.as_ref().map(|s| &s.account))
            .finish()
    }
}
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.encode_utf16().count() <= 16
        && !name.chars().any(char::is_control)
}
fn offline_uuid(name: &str) -> Uuid {
    ClientConnectRequest::offline("account-identity-only", name).uuid
}
impl SessionAccountRegistry {
    pub fn new(namespace: u64) -> Result<Self, AccountError> {
        if namespace == 0 {
            return Err(AccountError::InvalidNamespace);
        }
        Ok(Self {
            namespace,
            revision: 0,
            next_handle: 1,
            accounts: BTreeMap::new(),
            order: vec![],
            selected: None,
        })
    }
    fn bump(&mut self) -> Result<(), AccountError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(AccountError::CounterExhausted)?;
        Ok(())
    }
    fn insert(
        &mut self,
        name: String,
        uuid: Uuid,
        kind: AccountKind,
    ) -> Result<AccountHandle, AccountError> {
        if !valid_name(&name) {
            return Err(AccountError::InvalidName);
        }
        if self.accounts.len() >= MAX_ACCOUNTS {
            return Err(AccountError::AccountLimit);
        }
        if self
            .accounts
            .values()
            .any(|a| a.name == name && a.kind == kind)
        {
            return Err(AccountError::DuplicateAccount);
        }
        let next = self
            .next_handle
            .checked_add(1)
            .ok_or(AccountError::CounterExhausted)?;
        self.bump()?;
        let handle = AccountHandle(format!(
            "account:{:016x}:{:016x}",
            self.namespace, self.next_handle
        ));
        self.next_handle = next;
        self.accounts.insert(
            handle.clone(),
            Account {
                handle: handle.clone(),
                name,
                uuid,
                kind,
                authenticated: None,
            },
        );
        self.order.push(handle.clone());
        Ok(handle)
    }
    pub fn import_legacy(
        &mut self,
        record: &StoredAccountV1,
    ) -> Result<AccountHandle, AccountError> {
        let kind = if record.is_microsoft {
            AccountKind::Microsoft
        } else {
            AccountKind::Offline
        };
        let uuid = if kind == AccountKind::Offline {
            offline_uuid(&record.name)
        } else {
            record.uuid.parse().map_err(|_| AccountError::InvalidUuid)?
        };
        self.insert(record.name.clone(), uuid, kind)
    }
    pub fn add_offline(&mut self, name: String) -> Result<AccountHandle, AccountError> {
        self.insert(name.clone(), offline_uuid(&name), AccountKind::Offline)
    }
    pub fn remove(&mut self, handle: &AccountHandle) -> Result<(), AccountError> {
        if !self.accounts.contains_key(handle) {
            return Err(AccountError::MissingAccount);
        }
        self.bump()?;
        self.accounts.remove(handle);
        self.order.retain(|id| id != handle);
        Ok(())
    }
    pub fn rename_offline(
        &mut self,
        handle: &AccountHandle,
        name: String,
    ) -> Result<(), AccountError> {
        if !valid_name(&name) {
            return Err(AccountError::InvalidName);
        }
        let account = self
            .accounts
            .get(handle)
            .ok_or(AccountError::MissingAccount)?;
        if account.kind != AccountKind::Offline {
            return Err(AccountError::AuthenticationRequired);
        }
        if self
            .accounts
            .values()
            .any(|a| a.handle != *handle && a.kind == AccountKind::Offline && a.name == name)
        {
            return Err(AccountError::DuplicateAccount);
        }
        self.bump()?;
        let account = self
            .accounts
            .get_mut(handle)
            .ok_or(AccountError::MissingAccount)?;
        account.uuid = offline_uuid(&name);
        account.name = name;
        Ok(())
    }
    pub fn move_account(
        &mut self,
        handle: &AccountHandle,
        index: usize,
    ) -> Result<(), AccountError> {
        if index >= self.order.len() || !self.accounts.contains_key(handle) {
            return Err(AccountError::MissingAccount);
        }
        self.bump()?;
        self.order.retain(|h| h != handle);
        self.order.insert(index, handle.clone());
        Ok(())
    }
    pub fn install_authenticated(
        &mut self,
        handle: &AccountHandle,
        session: EngineAuthenticatedSession,
    ) -> Result<(), AccountError> {
        let account = self
            .accounts
            .get(handle)
            .ok_or(AccountError::MissingAccount)?;
        if account.kind != AccountKind::Microsoft
            || account.uuid != session.uuid
            || !valid_name(&session.name)
        {
            return Err(AccountError::IdentityMismatch);
        }
        self.bump()?;
        let account = self
            .accounts
            .get_mut(handle)
            .ok_or(AccountError::MissingAccount)?;
        account.name = session.name.clone();
        account.authenticated = Some(session);
        Ok(())
    }
    pub fn invalidate_authenticated(&mut self, handle: &AccountHandle) -> Result<(), AccountError> {
        if !self.accounts.contains_key(handle) {
            return Err(AccountError::MissingAccount);
        }
        self.bump()?;
        self.accounts.get_mut(handle).unwrap().authenticated = None;
        Ok(())
    }
    pub fn list(&self) -> Vec<AccountSummary> {
        self.order
            .iter()
            .filter_map(|id| self.accounts.get(id))
            .map(|a| AccountSummary {
                handle: a.handle.clone(),
                name: a.name.clone(),
                uuid: a.uuid.to_string(),
                kind: a.kind,
                online_ready: a.authenticated.is_some(),
                selected: self
                    .selected
                    .as_ref()
                    .is_some_and(|s| s.account == a.handle),
            })
            .collect()
    }
    pub fn prepare_switch(
        &self,
        target: &AccountHandle,
        mode: LoginMode,
        guard: WorldSwitchGuard,
    ) -> Result<PreparedSwitch, AccountError> {
        if !guard.is_main_menu() {
            return Err(AccountError::WorldActive);
        }
        let account = self
            .accounts
            .get(target)
            .ok_or(AccountError::MissingAccount)?;
        if mode == LoginMode::Online
            && account.kind == AccountKind::Microsoft
            && account.authenticated.is_none()
        {
            return Err(AccountError::AuthenticationRequired);
        }
        let mode = if account.kind == AccountKind::Offline {
            LoginMode::Offline
        } else {
            mode
        };
        Ok(PreparedSwitch {
            namespace: self.namespace,
            revision: self.revision,
            target: target.clone(),
            mode,
        })
    }
    pub fn commit_switch(
        &mut self,
        prepared: PreparedSwitch,
        guard: WorldSwitchGuard,
    ) -> Result<SelectedSession, AccountError> {
        if !guard.is_main_menu() {
            return Err(AccountError::WorldActive);
        }
        if prepared.namespace != self.namespace || prepared.revision != self.revision {
            return Err(AccountError::StalePreparation);
        }
        let account = self
            .accounts
            .get(&prepared.target)
            .ok_or(AccountError::MissingAccount)?;
        let selected = match prepared.mode {
            LoginMode::Offline => SelectedSession {
                account: account.handle.clone(),
                name: account.name.clone(),
                uuid: offline_uuid(&account.name),
                mode: LoginMode::Offline,
                credential: None,
            },
            LoginMode::Online => {
                let session = account
                    .authenticated
                    .as_ref()
                    .ok_or(AccountError::AuthenticationRequired)?;
                SelectedSession {
                    account: account.handle.clone(),
                    name: session.name.clone(),
                    uuid: session.uuid,
                    mode: LoginMode::Online,
                    credential: Some(session.credential.clone()),
                }
            }
        };
        self.bump()?;
        self.selected = Some(selected.clone());
        Ok(selected)
    }
    pub fn selected(&self) -> Option<&SelectedSession> {
        self.selected.as_ref()
    }
    pub fn invalidate_preparations(&mut self) -> Result<(), AccountError> {
        self.bump()
    }
}
