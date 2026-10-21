use el_extension_api::PackageVersion;
use std::{fmt, str::FromStr};
#[derive(Clone, Debug)]
pub struct Requirement {
    original: String,
    parsed: semver::VersionReq,
}
const OPS: [&str; 7] = ["<=", ">=", "=", "<", ">", "^", "~"];
impl FromStr for Requirement {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() || s.len() > 1024 {
            return Err("requirement must be 1..1024 bytes".into());
        }
        if !s.is_ascii() || s.bytes().any(|b| b < 0x20 || b == 0x7F) {
            return Err("only printable ASCII, spaces only".into());
        }
        if s.bytes().filter(|&b| b == b',').count() > 16 {
            return Err("too many commas (max 16)".into());
        }
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() > 16 {
            return Err("too many comparators (max 16)".into());
        }
        let mut norm: Vec<String> = Vec::with_capacity(parts.len());
        for raw in parts {
            let t = raw.trim_matches(' ');
            if t.is_empty() {
                return Err("empty comparator".into());
            }
            let op = OPS
                .iter()
                .find(|o| t.starts_with(*o))
                .ok_or("operator (= > >= < <= ^ ~) required")?;
            let rest = t[op.len()..].trim_start_matches(' ');
            if rest.is_empty() || rest.contains(' ') {
                return Err(format!(
                    "bad version near `{t}`: only spaces around operator allowed"
                ));
            }
            if rest.contains('|') || rest.contains('!') || rest.contains('*') {
                return Err(format!("forbidden `||`/`!=`/wildcard in `{t}`"));
            }
            let pv = rest
                .parse::<PackageVersion>()
                .map_err(|e| format!("bad version `{rest}`: {e:?}"))?;
            if !pv.build().is_empty() {
                return Err(format!("build metadata forbidden in `{t}`"));
            }
            norm.push(format!("{op}{rest}"));
        }
        let joined = norm.join(", ");
        let parsed =
            semver::VersionReq::from_str(&joined).map_err(|e| format!("semver req: {e}"))?;
        Ok(Self {
            original: s.to_string(),
            parsed,
        })
    }
}
impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.original)
    }
}
impl serde::Serialize for Requirement {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.original)
    }
}
impl<'de> serde::Deserialize<'de> for Requirement {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Requirement;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("requirement string")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Requirement, E> {
                Requirement::from_str(v).map_err(E::custom)
            }
        }
        d.deserialize_str(V)
    }
}
impl Requirement {
    pub fn matches(&self, v: &PackageVersion) -> bool {
        self.parsed.matches(v.as_semver())
    }
}
