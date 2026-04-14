use std::{fmt, io};

#[derive(Debug)]
pub enum SdkError {
    InvalidUrl(String),
    Http(String),
    Io(io::Error),
    Json(serde_json::Error),
    Rpc(String),
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(message) | Self::Http(message) | Self::Rpc(message) => write!(f, "{message}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SdkError {}

impl From<io::Error> for SdkError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type SdkResult<T> = Result<T, SdkError>;
