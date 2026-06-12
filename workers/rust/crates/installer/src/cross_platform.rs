use std::fs;
use std::path::PathBuf;

use crate::{
    Platform, build_desktop_app_manifest, build_launch_manifest, build_release_manifest,
    expected_release_script_contents, workspace_root,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossPlatformAuditIssue {
    pub platform: String,
    pub target: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossPlatformAuditReport {
    pub workspace: String,
    pub checked_platforms: Vec<String>,
    pub issues: Vec<CrossPlatformAuditIssue>,
}

impl CrossPlatformAuditReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki cross-platform audit".to_string(),
            format!("workspace: {}", self.workspace),
            format!("platforms: {}", self.checked_platforms.join(", ")),
            format!("issues: {}", self.issues.len()),
        ];

        if self.issues.is_empty() {
            lines.push("[ok] release scaffold parity is aligned".to_string());
            return lines.join("\n");
        }

        for issue in &self.issues {
            lines.push(format!(
                "[issue] {} {} -> {}",
                issue.platform, issue.target, issue.detail
            ));
        }

        lines.join("\n")
    }
}

pub fn cross_platform_audit_report() -> CrossPlatformAuditReport {
    let root = workspace_root();
    let platforms = [Platform::Macos, Platform::Linux, Platform::Windows];
    let checked_platforms = platforms
        .iter()
        .map(|platform| platform.as_str().to_string())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();

    for platform in platforms {
        audit_platform(&root, platform, &mut issues);
    }

    CrossPlatformAuditReport {
        workspace: root.display().to_string(),
        checked_platforms,
        issues,
    }
}

fn audit_platform(root: &PathBuf, platform: Platform, issues: &mut Vec<CrossPlatformAuditIssue>) {
    let platform_name = platform.as_str().to_string();
    let release_dir = root.join("dist").join(platform.as_str());

    if !release_dir.exists() {
        issues.push(CrossPlatformAuditIssue {
            platform: platform_name,
            target: "dist".to_string(),
            detail: format!("missing release directory {}", release_dir.display()),
        });
        return;
    }

    compare_text_file(
        issues,
        platform,
        "manifests/release-manifest.json",
        build_release_manifest(root, &release_dir, platform),
    );
    compare_text_file(
        issues,
        platform,
        "manifests/launch.json",
        build_launch_manifest(root, platform),
    );
    compare_text_file(
        issues,
        platform,
        "config/.env.example",
        load_workspace_env_example(root),
    );

    for app in ["hub-gui", "installer-gui", "workbench-gui"] {
        compare_text_file(
            issues,
            platform,
            &format!("desktop/{app}/manifest.json"),
            build_desktop_app_manifest(app, platform),
        );
    }

    for (relative, expected) in expected_release_script_contents(platform) {
        compare_text_file(issues, platform, &relative, expected);
    }
}

fn compare_text_file(
    issues: &mut Vec<CrossPlatformAuditIssue>,
    platform: Platform,
    relative: &str,
    expected: String,
) {
    let root = workspace_root();
    let path = root.join("dist").join(platform.as_str()).join(relative);
    let platform_name = platform.as_str().to_string();

    match fs::read_to_string(&path) {
        Ok(actual) => {
            if actual != expected {
                issues.push(CrossPlatformAuditIssue {
                    platform: platform_name,
                    target: relative.to_string(),
                    detail: "content drift from current release template".to_string(),
                });
            }
        }
        Err(_) => issues.push(CrossPlatformAuditIssue {
            platform: platform_name,
            target: relative.to_string(),
            detail: "missing required release file".to_string(),
        }),
    }
}

fn load_workspace_env_example(root: &PathBuf) -> String {
    fs::read_to_string(root.join(".env.example")).unwrap_or_default()
}
