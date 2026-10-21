#![allow(dead_code)]

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatTrustLevel {
    Secure,
    Modified,
    NotSecure,
}

impl ChatTrustLevel {
    pub fn evaluate(has_signature: bool, has_expired: bool, content_modified: bool) -> Self {
        if !has_signature || has_expired {
            ChatTrustLevel::NotSecure
        } else if content_modified {
            ChatTrustLevel::Modified
        } else {
            ChatTrustLevel::Secure
        }
    }

    pub fn is_not_secure(self) -> bool {
        self == ChatTrustLevel::NotSecure
    }

    pub fn serialized_name(self) -> &'static str {
        match self {
            ChatTrustLevel::Secure => "secure",
            ChatTrustLevel::Modified => "modified",
            ChatTrustLevel::NotSecure => "not_secure",
        }
    }
}
