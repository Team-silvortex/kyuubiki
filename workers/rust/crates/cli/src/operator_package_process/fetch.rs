//! Supervised Installer download; never activates packages or loads plugin code.
use super::{POLICY, Policy, ProcessError, invoke_until, wire};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use zeroize::Zeroize;

#[derive(Serialize, Deserialize)]
pub(super) struct FetchRequest {
    central_url: String,
    package_id: String,
    package_version: String,
    store_root: PathBuf,
    bearer_token: Option<String>,
}

impl Drop for FetchRequest {
    fn drop(&mut self) {
        self.bearer_token.zeroize();
    }
}

pub(crate) fn fetch_deadline() -> Instant {
    let policy = POLICY.get().copied().unwrap_or_else(Policy::default);
    Instant::now() + Duration::from_millis(policy.fetch_ms)
}

pub(crate) fn fetch_package(
    central_url: &str,
    package_id: &str,
    package_version: &str,
    store_root: &Path,
    bearer_token: Option<&str>,
    deadline: Instant,
) -> Result<(), ProcessError> {
    super::check(deadline).map_err(fetch_error)?;
    // The parent owns this directory, so a killed downloader cannot leave global temp files.
    let temporary = DownloadTemporary::create(store_root)?;
    let request = FetchRequest {
        central_url: central_url.into(),
        package_id: package_id.into(),
        package_version: package_version.into(),
        store_root: store_root.into(),
        bearer_token: bearer_token.map(str::to_owned),
    };
    match invoke_until(wire::Request::Fetch(request), deadline, Some(&temporary.0))
        .map_err(fetch_error)?
    {
        wire::Reply::Fetched => Ok(()),
        _ => Err(fetch_error(ProcessError::protocol(
            "unexpected package download reply",
        ))),
    }
}

fn fetch_error(mut error: ProcessError) -> ProcessError {
    error.code = match error.code {
        "operator_process_cancelled" => "operator_package_fetch_cancelled",
        "operator_process_timeout" => "operator_package_fetch_timeout",
        _ => "operator_package_fetch_failed",
    };
    if error.code == "operator_package_fetch_timeout" {
        error.message =
            "Package download exceeded its wall-clock budget; terminated without retry".into();
    }
    error
}

pub(super) fn execute(request: FetchRequest) -> Result<wire::Reply, (wire::FailureKind, String)> {
    // Independently hold the cache alive even if the Agent exits before this process does.
    let _session = crate::operator_package_generation_session::acquire_download_session_lease(
        &request.store_root,
    )
    .map_err(|error| (wire::FailureKind::Fetch, error))?;
    kyuubiki_installer::fetch_operator_package_into(
        &request.central_url,
        &request.package_id,
        &request.package_version,
        &request.store_root,
        request.bearer_token.as_deref(),
    )
    .map_err(|error| (wire::FailureKind::Fetch, error))?;
    Ok(wire::Reply::Fetched)
}

pub(super) fn configure_download_environment(command: &mut Command, temporary: &Path) {
    for name in ["TMPDIR", "TMP", "TEMP"] {
        command.env(name, temporary);
    }
    // Preserve download proxy routing, but only in the downloader, not native operator hosts.
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

struct DownloadTemporary(PathBuf);
impl DownloadTemporary {
    fn create(store_root: &Path) -> Result<Self, ProcessError> {
        let root = store_root.join(".fetch-temporary");
        fs::create_dir(&root).map_err(|error| fetch_error(ProcessError::protocol(error)))?;
        Ok(Self(root))
    }
}
impl Drop for DownloadTemporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_failure_mapping_preserves_cancellation_and_deadline_identity() {
        for (from, to) in [
            (
                "operator_process_cancelled",
                "operator_package_fetch_cancelled",
            ),
            ("operator_process_timeout", "operator_package_fetch_timeout"),
            ("operator_process_crashed", "operator_package_fetch_failed"),
        ] {
            assert_eq!(
                fetch_error(ProcessError {
                    code: from,
                    message: "test".into()
                })
                .code,
                to
            );
        }
    }

    #[test]
    fn downloader_defaults_expose_total_budget_and_no_activation() {
        let snapshot = super::super::snapshot();
        let fetch = &snapshot["package_fetch"];
        assert_eq!(fetch["timeout_ms"], 120_000);
        assert_eq!(fetch["loads_operator_code"], false);
        assert_eq!(fetch["automatic_retry"], false);
        assert_eq!(fetch["temporary_storage"], "unpublished_generation");
        assert_eq!(
            fetch["session_recovery"]["session_lease"],
            "shared_agent_and_downloader"
        );
        assert_eq!(fetch["session_recovery"]["catalog_wait_ms"], 2000);
        assert_eq!(fetch["session_recovery"]["automatic_replay"], false);
        assert_eq!(
            fetch["session_recovery"]["initialization"],
            "persist_intent_before_directory_creation"
        );
        assert_eq!(
            fetch["session_recovery"]["invalid_initialization"],
            "retain_and_report_without_guessing"
        );
    }
}
