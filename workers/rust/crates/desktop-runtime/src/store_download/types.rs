use super::Result;
use serde::{Deserialize, Serialize};

pub const MAX_FILE: u64 = 64 * 1024 * 1024;
pub const MAX_PACKAGE: u64 = 256 * 1024 * 1024;
pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(
            &value
                .split('.')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase()
                .as_str(),
        )
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DownloadSelection {
    pub kind: String,
    pub asset_id: String,
    pub version: String,
    pub platform: String,
    pub key_id: String,
    pub key_sha256: String,
}
impl DownloadSelection {
    pub fn validate(&self) -> Result<()> {
        if ![
            "operator",
            "workflow_template",
            "frontend_dsl_template",
            "language_pack",
        ]
        .contains(&self.kind.as_str())
            || !segment(&self.asset_id)
            || !segment(&self.version)
            || !self.version.starts_with(|c: char| c.is_ascii_digit())
            || ![
                "macos-aarch64",
                "macos-x86_64",
                "linux-aarch64",
                "linux-x86_64",
                "windows-aarch64",
                "windows-x86_64",
                "any",
            ]
            .contains(&self.platform.as_str())
            || !segment(&self.key_id)
            || !digest(&self.key_sha256)
        {
            return Err("invalid_selection");
        }
        Ok(())
    }
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub(super) struct Signature {
    pub algorithm: String,
    pub scheme: String,
    pub signed_payload: String,
    pub key_id: String,
    pub value: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub(super) struct ArtifactFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub media_type: String,
    pub signature: Signature,
}
#[derive(Deserialize)]
pub(super) struct Artifact {
    pub platform: String,
    pub files: Vec<ArtifactFile>,
}
#[derive(Deserialize)]
pub(super) struct Release {
    pub kind: String,
    pub asset_id: String,
    pub version: String,
    pub status: String,
    pub artifacts: Vec<Artifact>,
}
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Quota {
    pub limit: Option<u64>,
    pub used: u64,
    pub remaining: Option<u64>,
    pub unlimited: bool,
    pub reset_at: u64,
}
#[derive(Deserialize)]
pub(super) struct Entitlements {
    pub subject: String,
    pub download_kinds: Vec<String>,
    pub metered_kinds: Vec<String>,
    pub quota: Quota,
    pub block_reasons: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct QuotaOutcome {
    pub event: String,
    pub charged: bool,
    pub units: u8,
    pub remaining: Option<u64>,
    pub unlimited: bool,
}
#[derive(Deserialize)]
pub(super) struct GrantFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub ticket_expires_at: u64,
    pub download_url: String,
}
#[derive(Deserialize)]
pub(super) struct Grant {
    pub grant_id: String,
    pub kind: String,
    pub asset_id: String,
    pub version: String,
    pub platform: String,
    pub status: String,
    pub created_at: u64,
    pub renew_until: u64,
    pub ticket_expires_at: u64,
    pub files: Vec<GrantFile>,
    pub quota: QuotaOutcome,
}
impl Drop for GrantFile {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.download_url.zeroize();
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub format: String,
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub selection: DownloadSelection,
    pub files: Vec<ArtifactFile>,
    pub quota: Quota,
    pub metered: bool,
    pub state: String,
    pub attempted: bool,
    pub replay_until: Option<u64>,
    pub grant_id: Option<String>,
    pub outcome: Option<QuotaOutcome>,
    pub error: Option<String>,
    #[serde(default)]
    pub installation: Option<String>,
}
#[derive(Serialize)]
pub struct DownloadView {
    pub id: String,
    pub issuer: String,
    pub selection: DownloadSelection,
    pub state: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub received_bytes: u64,
    pub metered: bool,
    pub remaining: Option<u64>,
    pub unlimited: bool,
    pub charged: Option<bool>,
    pub error: Option<String>,
    pub directory: Option<String>,
    pub installation: Option<String>,
}
impl Record {
    pub fn view(&self) -> DownloadView {
        DownloadView {
            id: self.id.clone(),
            issuer: self.issuer.clone(),
            selection: self.selection.clone(),
            state: self.state.clone(),
            file_count: self.files.len(),
            total_bytes: self.files.iter().map(|file| file.size_bytes).sum(),
            received_bytes: 0,
            metered: self.metered,
            remaining: self
                .outcome
                .as_ref()
                .map_or(self.quota.remaining, |q| q.remaining),
            unlimited: self
                .outcome
                .as_ref()
                .map_or(self.quota.unlimited, |q| q.unlimited),
            charged: self.outcome.as_ref().map(|q| q.charged),
            error: self.error.clone(),
            directory: None,
            installation: self.installation.clone(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.selection.validate()?;
        if self.format != "kyuubiki.store-download/v1"
            || self.id.len() != 64
            || !digest(&self.id)
            || self.subject.is_empty()
            || self.subject.len() > 256
            || self.issuer.len() > 2048
            || self
                .error
                .as_ref()
                .is_some_and(|v| v.len() > 64 || !segment(v))
            || self.grant_id.as_ref().is_some_and(|v| !grant_id(v))
            || self.attempted != self.replay_until.is_some()
            || (self.grant_id.is_some() != self.outcome.is_some())
            || self.installation.as_deref().is_some_and(|v| {
                ![
                    "installing",
                    "installed",
                    "install_failed",
                    "variant_staging",
                    "variant_staged",
                    "variant_stage_failed",
                ]
                .contains(&v)
                    || self.state != "downloaded"
                    || (v.starts_with("variant_")
                        && (self.selection.kind != "operator"
                            || !matches!(
                                self.selection.platform.as_str(),
                                "linux-x86_64" | "linux-aarch64"
                            )))
            })
            || ![
                "prepared",
                "authorizing",
                "downloading",
                "downloaded",
                "failed",
                "cancelled",
            ]
            .contains(&self.state.as_str())
        {
            return Err("download_record_invalid");
        }
        validate_files(&self.files)
    }
}
pub(super) fn grant_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
pub(super) fn validate_files(files: &[ArtifactFile]) -> Result<()> {
    if files.is_empty() || files.len() > 32 {
        return Err("artifact_mismatch");
    }
    let mut paths = Vec::<String>::new();
    let mut total = 0u64;
    for file in files {
        let path = file.path.to_ascii_lowercase();
        if file.path.len() > 512
            || !file.path.split('/').all(segment)
            || !digest(&file.sha256)
            || file.size_bytes == 0
            || file.size_bytes > MAX_FILE
            || file.media_type.len() > 200
            || paths.iter().any(|p| {
                p == &path
                    || p.starts_with(&(path.clone() + "/"))
                    || path.starts_with(&(p.clone() + "/"))
            })
        {
            return Err("artifact_mismatch");
        }
        paths.push(path);
        total += file.size_bytes;
    }
    if total > MAX_PACKAGE {
        return Err("package_too_large");
    }
    Ok(())
}
