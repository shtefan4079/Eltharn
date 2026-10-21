use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceLocation {
    pub namespace: String,
    pub path: String,
}

impl ResourceLocation {
    pub const DEFAULT_NAMESPACE: &'static str = "minecraft";

    pub fn new(namespace: &str, path: &str) -> Self {
        if !Self::is_valid_namespace(namespace) {
            panic!(
                "Non [a-z0-9_.-] character in namespace of identifier: {}:{}",
                namespace, path
            );
        }
        if !Self::is_valid_path(path) {
            panic!(
                "Non [a-z0-9/._-] character in path of location: {}:{}",
                namespace, path
            );
        }
        ResourceLocation {
            namespace: namespace.to_string(),
            path: path.to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.split_once(':') {
            Some((namespace, path)) => ResourceLocation::new(namespace, path),
            None => ResourceLocation::new(Self::DEFAULT_NAMESPACE, s),
        }
    }

    pub fn try_new(namespace: &str, path: &str) -> Option<Self> {
        if Self::is_valid_namespace(namespace) && Self::is_valid_path(path) {
            Some(ResourceLocation {
                namespace: namespace.to_string(),
                path: path.to_string(),
            })
        } else {
            None
        }
    }

    pub fn try_from_string(s: &str) -> Option<Self> {
        match s.split_once(':') {
            Some((namespace, path)) => Self::try_new(namespace, path),
            None => Self::try_new(Self::DEFAULT_NAMESPACE, s),
        }
    }

    pub fn is_valid_namespace(namespace: &str) -> bool {
        if namespace == ".." {
            return false;
        }
        namespace.chars().all(|c| {
            c == '_' || c == '-' || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '.'
        })
    }

    pub fn is_valid_path(path: &str) -> bool {
        path.chars().all(|c| {
            c == '_'
                || c == '-'
                || (c >= 'a' && c <= 'z')
                || (c >= '0' && c <= '9')
                || c == '/'
                || c == '.'
        })
    }
}

impl fmt::Display for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl<'de> serde::Deserialize<'de> for ResourceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ResourceLocation::try_from_string(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid resource location: {s}")))
    }
}

impl serde::Serialize for ResourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
