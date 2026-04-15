#[derive(Clone, Debug)]
pub struct KyuubikiAuth {
    pub header_name: String,
    pub header_value: String,
}

impl KyuubikiAuth {
    pub fn access_token(token: impl Into<String>) -> Self {
        Self {
            header_name: "x-kyuubiki-token".into(),
            header_value: token.into(),
        }
    }
}
