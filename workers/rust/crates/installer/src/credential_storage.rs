use std::path::{Path, PathBuf};

use crate::workspace_root;

const CREDENTIAL_STORAGE_SCHEMA_VERSION: &str = "kyuubiki.credential-storage/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialStorageContract {
    pub schema_version: String,
    pub sandbox_root: String,
    pub platform_backends: Vec<CredentialPlatformBackend>,
    pub classes: Vec<CredentialClassRule>,
    pub denied_roots: Vec<String>,
    pub requirements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialPlatformBackend {
    pub platform: String,
    pub backend: String,
    pub secret_access: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialClassRule {
    pub class_id: String,
    pub storage_path: String,
    pub mutable: bool,
    pub description: String,
}

impl CredentialStorageContract {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki credential storage contract".to_string(),
            format!("schema: {}", self.schema_version),
            format!("sandbox_root: {}", self.sandbox_root),
            "platform_backends:".to_string(),
        ];

        for backend in &self.platform_backends {
            lines.push(format!(
                "  - {} => {} ({})",
                backend.platform, backend.backend, backend.secret_access
            ));
            for note in &backend.notes {
                lines.push(format!("    - {note}"));
            }
        }

        lines.push("classes:".to_string());

        for class in &self.classes {
            lines.push(format!(
                "  [{}] {} => {}",
                if class.mutable {
                    "mutable"
                } else {
                    "read-only"
                },
                class.class_id,
                class.storage_path
            ));
            lines.push(format!("    {}", class.description));
        }

        lines.push("denied_roots:".to_string());
        for root in &self.denied_roots {
            lines.push(format!("  - {root}"));
        }

        lines.push("requirements:".to_string());
        for requirement in &self.requirements {
            lines.push(format!("  - {requirement}"));
        }

        lines.join("\n")
    }
}

pub fn credential_storage_contract() -> CredentialStorageContract {
    let root = workspace_root();
    let sandbox_root = credential_sandbox_root(&root);
    CredentialStorageContract {
        schema_version: CREDENTIAL_STORAGE_SCHEMA_VERSION.to_string(),
        sandbox_root: sandbox_root.display().to_string(),
        platform_backends: vec![
            platform_backend(
                "desktop",
                "kyuubiki-sandbox-files",
                "path-backed, owner-visible, git-ignored",
                &[
                    "private material stays under .kyuubiki/credentials",
                    "installer integrity can report owned paths and residue",
                ],
            ),
            platform_backend(
                "browser",
                "in-memory-or-remote-session",
                "no durable local private-key storage",
                &[
                    "tokens should stay in memory or remote session state",
                    "browser storage may keep preferences, not secrets",
                ],
            ),
            platform_backend(
                "mobile-webview",
                "platform-secure-store-handle",
                "handle-backed, non-exporting secret access",
                &[
                    "iOS should use Keychain/Secure Enclave style storage through the shell",
                    "Android should use Keystore-backed storage through the shell",
                    "frontend receives configured state and opaque credential handles, not raw private keys",
                ],
            ),
        ],
        classes: vec![
            class_rule(
                "installer-ca",
                sandbox_root.join("installer/certificates"),
                true,
                "Local CA and node private keys owned by Installer certificate workflows.",
            ),
            class_rule(
                "remote-host-trust",
                sandbox_root.join("installer/remote-trust/known_hosts"),
                true,
                "Pinned SSH host identities for managed remote deployment.",
            ),
            class_rule(
                "integration-fixture",
                root.join("tests/integration/remote-ssh-fixture/runtime"),
                true,
                "Throwaway local-only SSH fixture keys and known-host files.",
            ),
            class_rule(
                "runtime-token",
                sandbox_root.join("runtime/tokens"),
                true,
                "Future file-backed runtime tokens; current GUI tokens should stay in memory or .env.local compatibility paths.",
            ),
        ],
        denied_roots: vec![
            "~/.ssh".to_string(),
            "~/Library/Application Support".to_string(),
            "%APPDATA%".to_string(),
            "/etc".to_string(),
            "/usr/local/etc".to_string(),
            "config/certificates".to_string(),
        ],
        requirements: vec![
            "credentials must live under the Kyuubiki sandbox root unless they are ignored integration-fixture state".to_string(),
            "mobile shells must expose opaque credential handles instead of raw secret bytes".to_string(),
            "project files may store references and fingerprints, never plaintext passwords or private keys".to_string(),
            "managed SSH must use pinned host trust before non-dev execution".to_string(),
            "credential directories must be excluded from git and visible in integrity reports".to_string(),
        ],
    }
}

pub fn credential_sandbox_root(root: &Path) -> PathBuf {
    root.join(".kyuubiki").join("credentials")
}

fn platform_backend(
    platform: &str,
    backend: &str,
    secret_access: &str,
    notes: &[&str],
) -> CredentialPlatformBackend {
    CredentialPlatformBackend {
        platform: platform.to_string(),
        backend: backend.to_string(),
        secret_access: secret_access.to_string(),
        notes: notes.iter().map(|note| (*note).to_string()).collect(),
    }
}

fn class_rule(
    class_id: &str,
    storage_path: PathBuf,
    mutable: bool,
    description: &str,
) -> CredentialClassRule {
    CredentialClassRule {
        class_id: class_id.to_string(),
        storage_path: storage_path.display().to_string(),
        mutable,
        description: description.to_string(),
    }
}
