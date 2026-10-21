use crate::ProtocolId;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAssetManifest {
    pub format_version: u32,
    pub compatibility_version: String,
    pub source_version: String,
    pub asset_index: String,
    pub index_sha1: String,
    pub client_sha1: String,
}
impl PreparedAssetManifest {
    pub fn validate(&self, expected_compatibility: &str) -> Result<(), String> {
        if !matches!(self.format_version, 1 | 2) {
            return Err(format!(
                "Unsupported prepared asset format {}",
                self.format_version
            ));
        }
        for name in [
            &self.compatibility_version,
            &self.source_version,
            &self.asset_index,
        ] {
            ProtocolId::new(name.clone()).map_err(|e| e.to_string())?;
        }
        if self.compatibility_version != expected_compatibility {
            return Err(format!(
                "Expected asset compatibility {expected_compatibility}, received {}",
                self.compatibility_version
            ));
        }
        if self.source_version != self.compatibility_version {
            return Err("Prepared assets substitute a different source version".into());
        }
        for hash in [&self.index_sha1, &self.client_sha1] {
            if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("Prepared assets contain an invalid SHA-1".into());
            }
        }
        Ok(())
    }
    pub fn client_archive_name(&self) -> String {
        if self.format_version == 1 {
            format!("client-{}.jar", self.source_version)
        } else {
            format!(
                "client-{}-{}.jar",
                self.source_version,
                self.client_sha1.to_ascii_lowercase()
            )
        }
    }
    pub fn index_file_name(&self) -> String {
        if self.format_version == 1 {
            format!("{}.json", self.asset_index)
        } else {
            format!(
                "{}-{}.json",
                self.asset_index,
                self.index_sha1.to_ascii_lowercase()
            )
        }
    }
}
