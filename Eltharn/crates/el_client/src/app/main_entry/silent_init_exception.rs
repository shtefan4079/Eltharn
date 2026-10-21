use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct SilentInitError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl SilentInitError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_cause(message: impl Into<String>, cause: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            message: message.into(),
            source: Some(cause),
        }
    }
}

impl fmt::Display for SilentInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SilentInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
