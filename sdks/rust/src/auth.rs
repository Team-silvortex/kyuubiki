use crate::{SdkError, SdkResult};
use std::fmt;

const MAX_AUTH_HEADER_NAME_BYTES: usize = 128;
const MAX_AUTH_HEADER_VALUE_BYTES: usize = 8 * 1024;

#[derive(Clone)]
pub struct KyuubikiAuth {
    pub header_name: String,
    pub header_value: String,
}

impl fmt::Debug for KyuubikiAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KyuubikiAuth")
            .field("header_name", &self.header_name)
            .field("header_value", &"[REDACTED]")
            .finish()
    }
}

impl KyuubikiAuth {
    pub fn access_token(token: impl Into<String>) -> Self {
        Self {
            header_name: "x-kyuubiki-token".into(),
            header_value: token.into(),
        }
    }

    pub fn validate(&self) -> SdkResult<()> {
        let valid_name = !self.header_name.is_empty()
            && self.header_name.len() <= MAX_AUTH_HEADER_NAME_BYTES
            && self
                .header_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
        if !valid_name {
            return Err(SdkError::Transport(
                "invalid authentication header name".into(),
            ));
        }
        let valid_value = !self.header_value.is_empty()
            && self.header_value.len() <= MAX_AUTH_HEADER_VALUE_BYTES
            && self
                .header_value
                .bytes()
                .all(|byte| byte.is_ascii_graphic());
        if !valid_value {
            return Err(SdkError::Transport(
                "invalid authentication header value".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn append_http_header(&self, request: &mut String) -> SdkResult<()> {
        self.validate()?;
        request.push_str(&format!("{}: {}\r\n", self.header_name, self.header_value));
        Ok(())
    }
}
