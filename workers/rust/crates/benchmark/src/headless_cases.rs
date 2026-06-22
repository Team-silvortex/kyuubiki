use crate::models::{BenchmarkCase, BenchmarkWorkload};

pub(crate) const HEADLESS_SDK_MATRIX: &str = "headless-sdk";

pub(crate) fn headless_sdk_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            id: "headless-action-manifest".to_string(),
            family: "headless_sdk_manifest",
            workload: BenchmarkWorkload::HeadlessActionManifest,
        },
        BenchmarkCase {
            id: "direct-fem-manifest".to_string(),
            family: "headless_sdk_direct_fem",
            workload: BenchmarkWorkload::DirectFemManifest,
        },
    ]
}

pub(crate) fn is_headless_sdk_matrix(matrix: &str) -> bool {
    matrix == HEADLESS_SDK_MATRIX
}

#[cfg(test)]
mod tests {
    use super::{HEADLESS_SDK_MATRIX, headless_sdk_cases, is_headless_sdk_matrix};

    #[test]
    fn exposes_headless_sdk_matrix_cases() {
        let cases = headless_sdk_cases();

        assert_eq!(cases.len(), 2);
        assert!(is_headless_sdk_matrix(HEADLESS_SDK_MATRIX));
        assert_eq!(cases[0].id, "headless-action-manifest");
        assert_eq!(cases[1].id, "direct-fem-manifest");
    }
}
