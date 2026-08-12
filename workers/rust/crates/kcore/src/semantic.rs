use std::collections::{HashMap, HashSet};

use kyuubiki_headless_sdk::{
    HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION, HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION,
    HeadlessExecutionBatch, HeadlessParameterPatch, HeadlessResearchRoundEvidence,
    HeadlessRunReport, apply_parameter_patch, headless_batch_content_sha256,
    verify_headless_research_round_evidence,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::canonical;
use crate::model::{Artifact, Manifest};

pub const HEADLESS_RESEARCH_CONTRACT_NAME: &str = "headless-research-round";
pub const RESEARCH_BATCH_ROLE: &str = "workflow.execution-batch";
pub const RESEARCH_PATCH_ROLE: &str = "workflow.parameter-patch";
pub const RESEARCH_RUN_ROLE: &str = "evidence.execution-run";
pub const RESEARCH_ROUND_ROLE: &str = "evidence.research-round";
pub const RESEARCH_SERIES_KIND: &str = "research-round-series";

const JSON_MEDIA_TYPE: &str = "application/json";
const MAX_SEMANTIC_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SEMANTIC_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_RESEARCH_SERIES_ROUNDS: usize = 4_096;

#[derive(Clone, Debug, Default, Serialize)]
pub struct SemanticVerification {
    pub contract_count: usize,
    pub research_round_count: usize,
}

struct ResearchArtifacts {
    batches: HashMap<String, HeadlessExecutionBatch>,
    reports: HashMap<String, HeadlessRunReport>,
    evidence_by_id: HashMap<String, HeadlessResearchRoundEvidence>,
    evidence_id_by_digest: HashMap<String, String>,
    patches: HashMap<String, HeadlessParameterPatch>,
}

pub(crate) fn verify<F>(manifest: &Manifest, mut read: F) -> Result<SemanticVerification, String>
where
    F: FnMut(&Artifact, u64) -> Result<Vec<u8>, String>,
{
    let binding = manifest
        .contracts
        .iter()
        .find(|binding| binding.name == HEADLESS_RESEARCH_CONTRACT_NAME);
    let has_round_evidence = manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.role == RESEARCH_ROUND_ROLE);
    let Some(binding) = binding else {
        if manifest.kind == RESEARCH_SERIES_KIND || has_round_evidence {
            return Err(format!(
                "headless research KCore requires the {HEADLESS_RESEARCH_CONTRACT_NAME} contract"
            ));
        }
        return Ok(SemanticVerification::default());
    };
    if manifest.kind != RESEARCH_SERIES_KIND {
        return Err(format!(
            "{HEADLESS_RESEARCH_CONTRACT_NAME} contract requires kind {RESEARCH_SERIES_KIND}"
        ));
    }
    if binding.schema_version != HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported {HEADLESS_RESEARCH_CONTRACT_NAME} contract: {}",
            binding.schema_version
        ));
    }
    if !manifest.entrypoints.contains(&binding.artifact_id) {
        return Err(format!(
            "{HEADLESS_RESEARCH_CONTRACT_NAME} contract artifact must be an entrypoint: {}",
            binding.artifact_id
        ));
    }
    validate_semantic_budget(manifest)?;

    let artifacts = load_research_artifacts(manifest, &mut read)?;
    let latest = artifacts
        .evidence_by_id
        .get(&binding.artifact_id)
        .ok_or_else(|| {
            format!(
                "{HEADLESS_RESEARCH_CONTRACT_NAME} contract does not target a {RESEARCH_ROUND_ROLE} artifact"
            )
        })?;
    let research_round_count = verify_research_chain(latest, &artifacts)?;
    if research_round_count != artifacts.evidence_by_id.len()
        || research_round_count != artifacts.batches.len()
        || research_round_count != artifacts.reports.len()
        || research_round_count.saturating_sub(1) != artifacts.patches.len()
    {
        return Err(
            "headless research KCore contains orphaned or missing round artifacts".to_string(),
        );
    }
    Ok(SemanticVerification {
        contract_count: 1,
        research_round_count,
    })
}

fn validate_semantic_budget(manifest: &Manifest) -> Result<(), String> {
    let mut total = 0_u64;
    let mut round_count = 0_usize;
    for artifact in &manifest.artifacts {
        if !is_research_role(&artifact.role) {
            continue;
        }
        total = total
            .checked_add(artifact.byte_length)
            .ok_or_else(|| "headless research semantic byte count overflow".to_string())?;
        if total > MAX_SEMANTIC_TOTAL_BYTES {
            return Err(format!(
                "headless research semantic artifacts exceed {MAX_SEMANTIC_TOTAL_BYTES} bytes"
            ));
        }
        if artifact.role == RESEARCH_ROUND_ROLE {
            round_count += 1;
        }
    }
    if round_count == 0 || round_count > MAX_RESEARCH_SERIES_ROUNDS {
        return Err(format!(
            "headless research KCore must contain 1..={MAX_RESEARCH_SERIES_ROUNDS} rounds"
        ));
    }
    Ok(())
}

fn is_research_role(role: &str) -> bool {
    matches!(
        role,
        RESEARCH_BATCH_ROLE | RESEARCH_PATCH_ROLE | RESEARCH_RUN_ROLE | RESEARCH_ROUND_ROLE
    )
}

fn load_research_artifacts<F>(
    manifest: &Manifest,
    read: &mut F,
) -> Result<ResearchArtifacts, String>
where
    F: FnMut(&Artifact, u64) -> Result<Vec<u8>, String>,
{
    let mut loaded = ResearchArtifacts {
        batches: HashMap::new(),
        reports: HashMap::new(),
        evidence_by_id: HashMap::new(),
        evidence_id_by_digest: HashMap::new(),
        patches: HashMap::new(),
    };
    for artifact in &manifest.artifacts {
        match artifact.role.as_str() {
            RESEARCH_BATCH_ROLE => {
                require_json_artifact(artifact, "kyuubiki.headless-execution-batch", "v1")?;
                let batch = read_strict(artifact, read)?;
                let digest = headless_batch_content_sha256(&batch)?;
                insert_unique(&mut loaded.batches, digest, batch, RESEARCH_BATCH_ROLE)?;
            }
            RESEARCH_RUN_ROLE => {
                require_json_artifact(artifact, "kyuubiki.headless-execution-run", "v1")?;
                let (value, report) = read_strict_value(artifact, read)?;
                let digest = canonical_json_sha256(&value);
                insert_unique(&mut loaded.reports, digest, report, RESEARCH_RUN_ROLE)?;
            }
            RESEARCH_ROUND_ROLE => {
                require_json_artifact(artifact, "kyuubiki.headless-research-round-evidence", "v1")?;
                let (value, evidence) = read_strict_value(artifact, read)?;
                let digest = canonical_json_sha256(&value);
                if loaded
                    .evidence_id_by_digest
                    .insert(digest, artifact.id.clone())
                    .is_some()
                {
                    return Err("duplicate headless research round evidence content".to_string());
                }
                loaded.evidence_by_id.insert(artifact.id.clone(), evidence);
            }
            RESEARCH_PATCH_ROLE => {
                require_json_artifact(artifact, "kyuubiki.headless-parameter-patch", "v1")?;
                let patch: HeadlessParameterPatch = read_strict(artifact, read)?;
                if patch.schema_version != HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION {
                    return Err(format!(
                        "artifact {} has an unsupported parameter patch schema",
                        artifact.id
                    ));
                }
                if loaded
                    .patches
                    .insert(patch.patch_id.clone(), patch)
                    .is_some()
                {
                    return Err("duplicate headless parameter patch_id".to_string());
                }
            }
            _ => {}
        }
    }
    Ok(loaded)
}

fn verify_research_chain(
    latest: &HeadlessResearchRoundEvidence,
    artifacts: &ResearchArtifacts,
) -> Result<usize, String> {
    let mut current = latest;
    let mut seen = HashSet::new();
    let mut count = 0_usize;
    loop {
        let current_value = serde_json::to_value(current)
            .map_err(|error| format!("failed to encode research evidence: {error}"))?;
        let current_digest = canonical_json_sha256(&current_value);
        if !seen.insert(current_digest) {
            return Err("headless research KCore contains a lineage cycle".to_string());
        }
        let batch = artifacts
            .batches
            .get(&current.batch_content_sha256)
            .ok_or_else(|| {
                format!(
                    "research round {} is missing its execution batch",
                    current.round_id
                )
            })?;
        let report = artifacts
            .reports
            .get(&current.run_report_sha256)
            .ok_or_else(|| {
                format!(
                    "research round {} is missing its execution report",
                    current.round_id
                )
            })?;
        let previous = current
            .previous_round
            .as_ref()
            .map(|link| {
                let id = artifacts
                    .evidence_id_by_digest
                    .get(&link.evidence_sha256)
                    .ok_or_else(|| {
                        format!(
                            "research round {} is missing previous evidence {}",
                            current.round_id, link.round_id
                        )
                    })?;
                artifacts
                    .evidence_by_id
                    .get(id)
                    .ok_or_else(|| "headless research evidence index is inconsistent".to_string())
            })
            .transpose()?;
        verify_headless_research_round_evidence(batch, report, current, previous)?;
        verify_patch_transition(current, batch, previous, artifacts)?;
        count += 1;
        let Some(previous) = previous else {
            break;
        };
        current = previous;
    }
    if count as u64 != latest.iteration {
        return Err(format!(
            "headless research KCore chain length {count} does not match latest iteration {}",
            latest.iteration
        ));
    }
    Ok(count)
}

fn verify_patch_transition(
    current: &HeadlessResearchRoundEvidence,
    current_batch: &HeadlessExecutionBatch,
    previous: Option<&HeadlessResearchRoundEvidence>,
    artifacts: &ResearchArtifacts,
) -> Result<(), String> {
    let Some(previous) = previous else {
        if current.patch_receipt.is_some() {
            return Err("initial KCore research round cannot carry a parameter patch".to_string());
        }
        return Ok(());
    };
    let receipt = current
        .patch_receipt
        .as_ref()
        .ok_or_else(|| "later KCore research round is missing its patch receipt".to_string())?;
    let patch = artifacts.patches.get(&receipt.patch_id).ok_or_else(|| {
        format!(
            "research round {} is missing parameter patch {}",
            current.round_id, receipt.patch_id
        )
    })?;
    let mut rebuilt = artifacts
        .batches
        .get(&previous.batch_content_sha256)
        .cloned()
        .ok_or_else(|| {
            format!(
                "research round {} is missing the previous execution batch",
                current.round_id
            )
        })?;
    let rebuilt_receipt = apply_parameter_patch(&mut rebuilt, patch)?;
    if rebuilt_receipt != *receipt || rebuilt != *current_batch {
        return Err(format!(
            "research round {} parameter patch does not reconstruct its execution batch",
            current.round_id
        ));
    }
    Ok(())
}

fn require_json_artifact(
    artifact: &Artifact,
    expected_schema: &str,
    expected_version: &str,
) -> Result<(), String> {
    let schema_ref = artifact.schema_ref.as_ref().ok_or_else(|| {
        format!(
            "research profile artifact {} requires schema_ref",
            artifact.id
        )
    })?;
    if artifact.media_type != JSON_MEDIA_TYPE
        || artifact.encoding.as_deref() != Some("json")
        || schema_ref.schema != expected_schema
        || schema_ref.version != expected_version
    {
        return Err(format!(
            "research profile artifact {} has an invalid media type, encoding, or schema_ref",
            artifact.id
        ));
    }
    Ok(())
}

fn read_strict<T, F>(artifact: &Artifact, read: &mut F) -> Result<T, String>
where
    T: DeserializeOwned + Serialize,
    F: FnMut(&Artifact, u64) -> Result<Vec<u8>, String>,
{
    read_strict_value(artifact, read).map(|(_, value)| value)
}

fn read_strict_value<T, F>(artifact: &Artifact, read: &mut F) -> Result<(Value, T), String>
where
    T: DeserializeOwned + Serialize,
    F: FnMut(&Artifact, u64) -> Result<Vec<u8>, String>,
{
    let bytes = read(artifact, MAX_SEMANTIC_ARTIFACT_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("artifact {} is invalid JSON: {error}", artifact.id))?;
    let decoded: T = serde_json::from_value(value.clone())
        .map_err(|error| format!("artifact {} violates its schema: {error}", artifact.id))?;
    let normalized = serde_json::to_value(&decoded)
        .map_err(|error| format!("failed to normalize artifact {}: {error}", artifact.id))?;
    if normalized != value {
        return Err(format!(
            "artifact {} contains unknown, missing, or non-canonical fields",
            artifact.id
        ));
    }
    Ok((value, decoded))
}

fn insert_unique<T>(
    values: &mut HashMap<String, T>,
    key: String,
    value: T,
    role: &str,
) -> Result<(), String> {
    if values.insert(key, value).is_some() {
        Err(format!("duplicate {role} semantic identity"))
    } else {
        Ok(())
    }
}

fn canonical_json_sha256(value: &Value) -> String {
    canonical::sha256(canonical::json(value).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Integrity, Producer};
    use std::collections::BTreeMap;

    fn artifact(id: String, byte_length: u64) -> Artifact {
        let sha256 = "a".repeat(64);
        Artifact {
            id,
            role: RESEARCH_ROUND_ROLE.to_string(),
            media_type: JSON_MEDIA_TYPE.to_string(),
            object_path: Manifest::object_path(&sha256),
            byte_length,
            sha256,
            name: None,
            schema_ref: None,
            encoding: Some("json".to_string()),
            shape: vec![],
            unit: None,
            metadata: BTreeMap::new(),
        }
    }

    fn manifest(artifacts: Vec<Artifact>) -> Manifest {
        Manifest {
            schema_version: "kyuubiki.kcore/v1".to_string(),
            format: "kcore".to_string(),
            format_version: 1,
            core_id: "research-budget".to_string(),
            title: "Research budget".to_string(),
            kind: RESEARCH_SERIES_KIND.to_string(),
            producer: Producer {
                name: "test".to_string(),
                version: "1".to_string(),
                runtime: None,
            },
            artifacts,
            contracts: vec![],
            entrypoints: vec![],
            integrity: Integrity {
                algorithm: "sha256".to_string(),
                core_digest_sha256: "a".repeat(64),
            },
            created_at: None,
            provenance: Value::Null,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn semantic_budget_rejects_excessive_declared_bytes_without_reading_objects() {
        let manifest = manifest(vec![artifact(
            "round-1".to_string(),
            MAX_SEMANTIC_TOTAL_BYTES + 1,
        )]);
        assert!(
            validate_semantic_budget(&manifest)
                .expect_err("semantic byte budget")
                .contains("exceed")
        );
    }

    #[test]
    fn semantic_budget_rejects_excessive_round_count() {
        let artifacts = (0..=MAX_RESEARCH_SERIES_ROUNDS)
            .map(|index| artifact(format!("round-{index}"), 0))
            .collect();
        assert!(
            validate_semantic_budget(&manifest(artifacts))
                .expect_err("semantic round budget")
                .contains("1..=")
        );
    }
}
