use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessParameterPatch, HeadlessParameterPatchReceipt,
    HeadlessResearchRoundEvidence, HeadlessResearchRoundSpec, HeadlessRunReport,
    apply_parameter_patch, build_headless_research_round_evidence,
    validate_headless_research_round_plan,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;

use super::{Flags, OutputScope};

const MAX_RESEARCH_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;
pub(super) const PREFLIGHT_FAILURE: &str = "headless research round preflight failed:";

pub(crate) struct PreparedResearchRound {
    spec: HeadlessResearchRoundSpec,
    previous: Option<HeadlessResearchRoundEvidence>,
    output_path: String,
}

pub(crate) fn apply_parameter_patch_from_flags(
    batch: &mut HeadlessExecutionBatch,
    flags: &Flags,
    scope: OutputScope,
) -> Result<Option<HeadlessParameterPatchReceipt>, String> {
    let Some(path) = flags.parameter_patch.as_deref() else {
        if flags.parameter_patch_receipt_out.is_some() {
            return Err("--parameter-patch-receipt-out requires --parameter-patch".to_string());
        }
        return Ok(None);
    };
    let patch = read_json::<HeadlessParameterPatch>(path, "headless parameter patch")?;
    let receipt = apply_parameter_patch(batch, &patch)?;
    if let Some(output_path) = flags.parameter_patch_receipt_out.as_deref() {
        write_json(
            flags,
            scope,
            output_path,
            &receipt,
            "parameter patch receipt",
        )?;
    }
    Ok(Some(receipt))
}

pub(crate) fn prepare_research_round(
    flags: &Flags,
    selected_executor: Option<&str>,
    batch: &HeadlessExecutionBatch,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
) -> Result<Option<PreparedResearchRound>, String> {
    prepare_research_round_inner(flags, selected_executor, batch, patch_receipt)
        .map_err(|error| format!("{PREFLIGHT_FAILURE} {error}"))
}

fn prepare_research_round_inner(
    flags: &Flags,
    selected_executor: Option<&str>,
    batch: &HeadlessExecutionBatch,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
) -> Result<Option<PreparedResearchRound>, String> {
    let Some(spec_path) = flags.research_round_spec.as_deref() else {
        if flags.previous_round_evidence.is_some() || flags.research_round_out.is_some() {
            return Err(
                "--previous-round-evidence and --research-round-out require --research-round-spec"
                    .to_string(),
            );
        }
        return Ok(None);
    };
    if !flags.execute
        || selected_executor != Some("service")
        || flags.execution_posture.as_deref() != Some("research")
    {
        return Err(
            "headless research round evidence requires --execute --executor service --execution-posture research"
                .to_string(),
        );
    }
    let output_path = flags.research_round_out.clone().ok_or_else(|| {
        "--research-round-spec requires --research-round-out to retain evidence".to_string()
    })?;
    let spec = read_json::<HeadlessResearchRoundSpec>(spec_path, "research round spec")?;
    let previous = flags
        .previous_round_evidence
        .as_deref()
        .map(|path| {
            read_json::<HeadlessResearchRoundEvidence>(
                path,
                "headless research round previous evidence",
            )
        })
        .transpose()?;
    if spec.iteration > 1 && previous.is_none() {
        return Err(
            "headless research round iteration 2 or later requires --previous-round-evidence"
                .to_string(),
        );
    }
    if spec.iteration > 1 && flags.parameter_patch.is_none() {
        return Err(
            "headless research round iteration 2 or later requires --parameter-patch".to_string(),
        );
    }
    validate_headless_research_round_plan(batch, &spec, patch_receipt, previous.as_ref())?;
    Ok(Some(PreparedResearchRound {
        spec,
        previous,
        output_path,
    }))
}

pub(crate) fn write_research_round_evidence(
    prepared: &PreparedResearchRound,
    batch: &HeadlessExecutionBatch,
    report: &HeadlessRunReport,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
    flags: &Flags,
) -> Result<PathBuf, String> {
    let evidence = build_headless_research_round_evidence(
        batch,
        report,
        &prepared.spec,
        patch_receipt,
        prepared.previous.as_ref(),
    )
    .map_err(|error| {
        super::kyuubiki_headless_post_run::generation_error("research round evidence", error)
    })?;
    write_json(
        flags,
        OutputScope::Run,
        &prepared.output_path,
        &evidence,
        "headless research round evidence",
    )
}

fn read_json<T: DeserializeOwned>(path: &str, label: &str) -> Result<T, String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("failed to inspect {label} {path}: {error}"))?
        .len();
    if size > MAX_RESEARCH_ARTIFACT_BYTES {
        return Err(format!(
            "{label} {path} exceeds the {MAX_RESEARCH_ARTIFACT_BYTES}-byte limit"
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {label} {path}: {error}"))?;
    if bytes.len() as u64 > MAX_RESEARCH_ARTIFACT_BYTES {
        return Err(format!(
            "{label} {path} exceeds the {MAX_RESEARCH_ARTIFACT_BYTES}-byte limit"
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {label} {path}: {error}"))
}

fn write_json<T: Serialize>(
    flags: &Flags,
    scope: OutputScope,
    path: &str,
    value: &T,
    label: &str,
) -> Result<PathBuf, String> {
    super::write_guarded_json_file(flags, scope, path, value)
        .map_err(|error| format!("{error} ({label})"))
}
