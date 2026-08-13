pub const MODEL_ARTIFACT_MAX_BYTES_ENV: &str = "KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES";
pub const DEFAULT_MODEL_ARTIFACT_MAX_BYTES: usize = 536_870_912;

pub fn model_artifact_max_bytes() -> usize {
    configured_model_artifact_max_bytes(std::env::var(MODEL_ARTIFACT_MAX_BYTES_ENV).ok().as_deref())
}

pub fn configured_model_artifact_max_bytes(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MODEL_ARTIFACT_MAX_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_artifact_limit_is_explicit_and_fail_safe() {
        assert_eq!(
            configured_model_artifact_max_bytes(Some("1073741824")),
            1_073_741_824
        );
        assert_eq!(
            configured_model_artifact_max_bytes(Some("0")),
            DEFAULT_MODEL_ARTIFACT_MAX_BYTES
        );
        assert_eq!(
            configured_model_artifact_max_bytes(Some("not-a-size")),
            DEFAULT_MODEL_ARTIFACT_MAX_BYTES
        );
    }
}
