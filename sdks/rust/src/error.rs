use std::{fmt, io};

#[derive(Debug)]
pub enum SdkError {
    InvalidUrl(String),
    Transport(String),
    HttpStatus { status_code: u16, body: String },
    Io(io::Error),
    Json(serde_json::Error),
    Rpc { message: String, code: Option<String> },
    Timeout(String),
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(message) | Self::Transport(message) | Self::Timeout(message) => write!(f, "{message}"),
            Self::HttpStatus { status_code, body } => write!(f, "http {status_code}: {body}"),
            Self::Rpc { message, code } => match code {
                Some(code) => write!(f, "{code}: {message}"),
                None => write!(f, "{message}"),
            },
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SdkError {}

impl From<io::Error> for SdkError {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::TimedOut => Self::Timeout(value.to_string()),
            _ => Self::Io(value),
        }
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type SdkResult<T> = Result<T, SdkError>;
