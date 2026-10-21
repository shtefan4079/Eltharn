use el_extension_api::launch::ExtensionFlags;
use el_extension_api::{ExtensionId, PackageVersion, Sha256Digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_FILE_BYTES: usize = 262_144;
const MAX_SETTINGS_BYTES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackagePin {
    pub id: ExtensionId,
    pub version: PackageVersion,
    pub content_id: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderSelection {
    pub package: PackagePin,
    pub settings: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub flags: ExtensionFlags,
    #[serde(default)]
    pub mods: Vec<PackagePin>,
    #[serde(default)]
    pub shader: Option<ShaderSelection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ExtensionPreferences {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_active_profile")]
    pub active_profile: String,
    #[serde(default = "default_profiles")]
    pub profiles: Vec<ExtensionProfile>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_active_profile() -> String {
    "default".to_owned()
}

fn default_profiles() -> Vec<ExtensionProfile> {
    vec![ExtensionProfile {
        id: "default".to_owned(),
        name: "Default".to_owned(),
        flags: ExtensionFlags::default(),
        mods: Vec::new(),
        shader: None,
    }]
}

impl Default for ExtensionPreferences {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            active_profile: default_active_profile(),
            profiles: default_profiles(),
        }
    }
}

fn valid_profile_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn check_pin(pin: &PackagePin) -> Result<(), String> {
    if pin.id.is_reserved_namespace() {
        return Err("reserved extension namespace".to_owned());
    }
    Ok(())
}

fn check_shader(sel: &ShaderSelection) -> Result<(), String> {
    check_pin(&sel.package)?;
    if !sel.settings.is_object() {
        return Err("shader settings must be an object".to_owned());
    }
    let n = serde_json::to_vec(&sel.settings)
        .map_err(|e| e.to_string())?
        .len();
    if n > MAX_SETTINGS_BYTES {
        return Err("shader settings too large".to_owned());
    }
    Ok(())
}

fn check_profile(p: &ExtensionProfile) -> Result<(), String> {
    if !valid_profile_id(&p.id) {
        return Err(format!("invalid profile id {:?}", p.id));
    }
    let trimmed = p.name.trim();
    if trimmed.is_empty() || p.name.chars().count() > 64 {
        return Err("invalid profile name".to_owned());
    }
    if p.name.chars().any(|c| c.is_control()) {
        return Err("invalid profile name".to_owned());
    }
    if p.mods.len() > 256 {
        return Err("too many mods".to_owned());
    }
    for i in 0..p.mods.len() {
        check_pin(&p.mods[i])?;
        for other in p.mods.iter().take(i) {
            if other.id == p.mods[i].id {
                return Err("duplicate mod package".to_owned());
            }
        }
    }
    if let Some(sel) = &p.shader {
        check_shader(sel)?;
    }
    Ok(())
}

impl ExtensionPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported schema_version".to_owned());
        }
        if self.profiles.is_empty() || self.profiles.len() > 16 {
            return Err("profiles must contain 1..=16 entries".to_owned());
        }
        for p in &self.profiles {
            check_profile(p)?;
        }
        for i in 1..self.profiles.len() {
            if self.profiles[..i]
                .iter()
                .any(|p| p.id == self.profiles[i].id)
            {
                return Err("duplicate profile id".to_owned());
            }
        }
        if !self.profiles.iter().any(|p| p.id == self.active_profile) {
            return Err("unknown active_profile".to_owned());
        }
        Ok(())
    }

    pub fn active(&self) -> Result<&ExtensionProfile, String> {
        self.validate()?;
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile)
            .ok_or_else(|| "unknown active_profile".to_owned())
    }

    pub fn active_mut(&mut self) -> Result<&mut ExtensionProfile, String> {
        self.validate()?;
        let idx = self
            .profiles
            .iter()
            .position(|p| p.id == self.active_profile)
            .ok_or_else(|| "unknown active_profile".to_owned())?;
        Ok(&mut self.profiles[idx])
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err("preferences file too large".to_owned());
        }
        let value = crate::extension_json::strict_json(bytes, MAX_FILE_BYTES)?;
        let parsed: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn base_recovery(&self) -> Result<ExtensionProfile, String> {
        let active = self.active()?.clone();
        if !active.mods.is_empty() {
            return Err("recovery refuses to strip mods".to_owned());
        }
        let mut out = active;
        out.flags = ExtensionFlags::default();
        out.shader = None;
        Ok(out)
    }
}
