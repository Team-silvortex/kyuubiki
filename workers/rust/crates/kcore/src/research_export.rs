use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::export::{ExportReport, export_spec};
use crate::model::{
    ContractBinding, EXPORT_SCHEMA_VERSION, ExportArtifact, ExportSpec, Producer, SchemaReference,
};
use crate::semantic::{
    HEADLESS_RESEARCH_CONTRACT_NAME, MAX_RESEARCH_SERIES_ROUNDS, RESEARCH_BATCH_ROLE,
    RESEARCH_PATCH_ROLE, RESEARCH_ROUND_ROLE, RESEARCH_RUN_ROLE, RESEARCH_SERIES_KIND,
};
use kyuubiki_headless_sdk::HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION;

pub const HEADLESS_RESEARCH_SERIES_SCHEMA_VERSION: &str =
    "kyuubiki.kcore-headless-research-series/v1";

const MAX_SERIES_SPEC_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SOURCE_PATH_BYTES: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessResearchSeriesRound {
    pub batch: String,
    pub run_report: String,
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_patch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessResearchSeriesSpec {
    pub schema_version: String,
    pub core_id: String,
    pub title: String,
    pub producer: Producer,
    pub rounds: Vec<HeadlessResearchSeriesRound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

pub fn export_research_series(spec_path: &Path, output: &Path) -> Result<ExportReport, String> {
    let spec_path = existing_regular_file(spec_path, "research series spec")?;
    let spec = read_series_spec(&spec_path)?;
    export_research_series_spec(
        spec,
        spec_path.parent().unwrap_or_else(|| Path::new(".")),
        output,
    )
}

pub fn export_research_series_spec(
    spec: HeadlessResearchSeriesSpec,
    source_base: &Path,
    output: &Path,
) -> Result<ExportReport, String> {
    export_spec(build_research_export_spec(spec)?, source_base, output)
}

pub fn build_research_export_spec(spec: HeadlessResearchSeriesSpec) -> Result<ExportSpec, String> {
    validate_series_spec(&spec)?;
    let latest_evidence_id = round_artifact_id(spec.rounds.len(), "evidence");
    let mut artifacts = Vec::with_capacity(spec.rounds.len().saturating_mul(4));
    for (index, round) in spec.rounds.into_iter().enumerate() {
        let ordinal = index + 1;
        artifacts.push(json_artifact(
            round_artifact_id(ordinal, "batch"),
            RESEARCH_BATCH_ROLE,
            round.batch,
            "kyuubiki.headless-execution-batch",
        ));
        artifacts.push(json_artifact(
            round_artifact_id(ordinal, "run"),
            RESEARCH_RUN_ROLE,
            round.run_report,
            "kyuubiki.headless-execution-run",
        ));
        artifacts.push(json_artifact(
            round_artifact_id(ordinal, "evidence"),
            RESEARCH_ROUND_ROLE,
            round.evidence,
            "kyuubiki.headless-research-round-evidence",
        ));
        if let Some(parameter_patch) = round.parameter_patch {
            artifacts.push(json_artifact(
                round_artifact_id(ordinal, "patch"),
                RESEARCH_PATCH_ROLE,
                parameter_patch,
                "kyuubiki.headless-parameter-patch",
            ));
        }
    }
    Ok(ExportSpec {
        schema_version: EXPORT_SCHEMA_VERSION.to_string(),
        core_id: spec.core_id,
        title: spec.title,
        kind: RESEARCH_SERIES_KIND.to_string(),
        producer: spec.producer,
        artifacts,
        contracts: vec![ContractBinding {
            name: HEADLESS_RESEARCH_CONTRACT_NAME.to_string(),
            schema_version: HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION.to_string(),
            artifact_id: latest_evidence_id.clone(),
            purpose: Some("self-contained Headless research lineage".to_string()),
        }],
        entrypoints: vec![latest_evidence_id],
        created_at: spec.created_at,
        provenance: spec.provenance,
        metadata: spec.metadata,
    })
}

fn validate_series_spec(spec: &HeadlessResearchSeriesSpec) -> Result<(), String> {
    if spec.schema_version != HEADLESS_RESEARCH_SERIES_SCHEMA_VERSION {
        return Err(format!(
            "unsupported Headless research series schema: {}",
            spec.schema_version
        ));
    }
    if spec.rounds.is_empty() || spec.rounds.len() > MAX_RESEARCH_SERIES_ROUNDS {
        return Err(format!(
            "Headless research series must contain 1..={MAX_RESEARCH_SERIES_ROUNDS} rounds"
        ));
    }
    for (index, round) in spec.rounds.iter().enumerate() {
        let ordinal = index + 1;
        validate_source(&round.batch, ordinal, "batch")?;
        validate_source(&round.run_report, ordinal, "run_report")?;
        validate_source(&round.evidence, ordinal, "evidence")?;
        match (ordinal, &round.parameter_patch) {
            (1, Some(_)) => {
                return Err(
                    "Headless research series round 1 cannot declare parameter_patch".to_string(),
                );
            }
            (2.., None) => {
                return Err(format!(
                    "Headless research series round {ordinal} requires parameter_patch"
                ));
            }
            (_, Some(path)) => validate_source(path, ordinal, "parameter_patch")?,
            _ => {}
        }
    }
    Ok(())
}

fn validate_source(source: &str, ordinal: usize, field: &str) -> Result<(), String> {
    if source.trim().is_empty()
        || source.len() > MAX_SOURCE_PATH_BYTES
        || source.chars().any(char::is_control)
    {
        return Err(format!(
            "Headless research series round {ordinal} {field} path is invalid"
        ));
    }
    Ok(())
}

fn json_artifact(id: String, role: &str, source: String, schema: &str) -> ExportArtifact {
    ExportArtifact {
        id,
        role: role.to_string(),
        media_type: "application/json".to_string(),
        source,
        name: None,
        schema_ref: Some(SchemaReference {
            schema: schema.to_string(),
            version: "v1".to_string(),
        }),
        encoding: Some("json".to_string()),
        shape: vec![],
        unit: None,
        metadata: BTreeMap::new(),
    }
}

fn round_artifact_id(ordinal: usize, suffix: &str) -> String {
    format!("round-{ordinal:06}-{suffix}")
}

fn read_series_spec(path: &Path) -> Result<HeadlessResearchSeriesSpec, String> {
    let size = path
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
        .len();
    if size > MAX_SERIES_SPEC_BYTES {
        return Err(format!(
            "research series spec exceeds {MAX_SERIES_SPEC_BYTES} bytes: {}",
            path.display()
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid research series spec {}: {error}", path.display()))
}

fn existing_regular_file(path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}
