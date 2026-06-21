use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::certificates_types::CertificateMetadata;

pub(crate) fn inspect_certificate(path: &Path) -> Result<CertificateMetadata, String> {
    let output = Command::new("openssl")
        .arg("x509")
        .arg("-in")
        .arg(path)
        .arg("-noout")
        .arg("-serial")
        .arg("-fingerprint")
        .arg("-sha256")
        .arg("-subject")
        .arg("-enddate")
        .output()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("openssl failed to inspect {}", path.display())
        } else {
            stderr
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(CertificateMetadata {
        serial: find_metadata_value(&stdout, "serial=").unwrap_or_default(),
        fingerprint: find_metadata_value(&stdout, "sha256 fingerprint=")
            .or_else(|| find_metadata_value(&stdout, "SHA256 Fingerprint="))
            .unwrap_or_default(),
        subject: find_metadata_value(&stdout, "subject=").unwrap_or_default(),
        not_after: find_metadata_value(&stdout, "notAfter=").unwrap_or_default(),
    })
}

pub(crate) fn ensure_openssl_available() -> Result<(), String> {
    let output = Command::new("openssl")
        .arg("version")
        .output()
        .map_err(|error| format!("failed to run openssl: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("openssl is required for certificate management".to_string())
    }
}

pub(crate) fn run_command(command: &mut Command, label: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to {label}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() {
            format!("failed to {label}: {stdout}")
        } else {
            format!("failed to {label}: {stderr}")
        })
    }
}

pub(crate) fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn find_metadata_value(text: &str, prefix: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix(prefix)
            .map(|value| value.trim().to_string())
    })
}
