#![allow(dead_code)]

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProfilePublicKey {
    pub expires_at: std::time::SystemTime,
    pub key_bytes: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ProfileKeyPair {
    pub private_key_pem: String,
    pub public_key: ProfilePublicKey,
    pub refreshed_after: std::time::SystemTime,
}

impl ProfileKeyPair {
    pub fn due_refresh(&self) -> bool {
        false
    }
}

pub trait ProfileKeyPairManager {
    fn prepare_key_pair(&self) -> Pin<Box<dyn Future<Output = Option<ProfileKeyPair>> + Send>>;
    fn should_refresh_key_pair(&self) -> bool;
}

pub struct EmptyProfileKeyPairManager;

impl ProfileKeyPairManager for EmptyProfileKeyPairManager {
    fn prepare_key_pair(&self) -> Pin<Box<dyn Future<Output = Option<ProfileKeyPair>> + Send>> {
        Box::pin(async { None })
    }

    fn should_refresh_key_pair(&self) -> bool {
        false
    }
}

pub struct AccountProfileKeyPairManager {
    pub profile_id: Uuid,
    pub profile_key_pair_path: std::path::PathBuf,
}

impl AccountProfileKeyPairManager {
    pub fn new(profile_id: Uuid, game_directory: &Path) -> Self {
        let profile_key_pair_path = game_directory
            .join("profilekeys")
            .join(format!("{}.json", profile_id));
        Self {
            profile_id,
            profile_key_pair_path,
        }
    }
}

impl ProfileKeyPairManager for AccountProfileKeyPairManager {
    fn prepare_key_pair(&self) -> Pin<Box<dyn Future<Output = Option<ProfileKeyPair>> + Send>> {
        Box::pin(async { None })
    }

    fn should_refresh_key_pair(&self) -> bool {
        false
    }
}
