use super::MaterialResearchBundleArtifactChecksums;
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct RawArtifacts<'a> {
    #[serde(borrow)]
    initial_exploration: &'a RawValue,
    #[serde(borrow)]
    next_round_execution_plan: &'a RawValue,
    #[serde(borrow)]
    next_exploration: &'a RawValue,
    #[serde(borrow)]
    chain: &'a RawValue,
}

pub(super) fn verify_raw_artifacts(
    text: &str,
    checksums: &MaterialResearchBundleArtifactChecksums,
) -> Result<(), String> {
    let raw: RawArtifacts<'_> = serde_json::from_str(text)
        .map_err(|error| format!("invalid material research artifact JSON: {error}"))?;
    for (name, artifact, expected) in [
        (
            "initial_exploration",
            raw.initial_exploration,
            &checksums.initial_exploration_sha256,
        ),
        (
            "next_round_execution_plan",
            raw.next_round_execution_plan,
            &checksums.next_round_execution_plan_sha256,
        ),
        (
            "next_exploration",
            raw.next_exploration,
            &checksums.next_exploration_sha256,
        ),
        ("chain", raw.chain, &checksums.chain_sha256),
    ] {
        let actual = raw_artifact_sha256(artifact.get());
        if &actual != expected {
            return Err(format!(
                "artifact_checksums.{name}_sha256: checksum mismatch for {name}; declared {expected}, computed {actual}"
            ));
        }
    }
    Ok(())
}

fn raw_artifact_sha256(text: &str) -> String {
    // v1 hashes compact original JSON plus LF, not sorted/re-encoded JSON.
    let bytes = text.as_bytes();
    let mut hasher = Sha256::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
            if start < index {
                hasher.update(&bytes[start..index]);
            }
            start = index + 1;
        }
    }
    hasher.update(&bytes[start..]);
    hasher.update(b"\n");
    format!("{:x}", hasher.finalize())
}
