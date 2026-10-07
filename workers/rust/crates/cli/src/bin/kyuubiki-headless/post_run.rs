use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessParameterPatchReceipt, HeadlessRunReport,
    build_material_report_from_run,
};
use serde_json::Value;

use super::kyuubiki_headless_research_round::{
    PreparedResearchRound, write_research_round_evidence,
};
use super::{Flags, OutputScope, write_guarded_json_file};

pub(super) const GENERATION_FAILURE: &str = "headless artifact generation failed:";

pub(super) fn generation_error(artifact: &str, error: String) -> String {
    format!("{GENERATION_FAILURE} {artifact}: {error}")
}

pub(super) fn publish_artifacts(
    flags: &Flags,
    batch: &HeadlessExecutionBatch,
    report: &HeadlessRunReport,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
    round: Option<&PreparedResearchRound>,
) -> Result<Option<Value>, String> {
    let material_report = flags
        .material_report
        .as_deref()
        .map(|study| {
            build_material_report_from_run(study, report)
                .map_err(|error| generation_error("material report", error))
        })
        .transpose()?;
    if let (Some(material_report), Some(output)) = (&material_report, &flags.material_report_out) {
        write_guarded_json_file(flags, OutputScope::Run, output, material_report)?;
    }
    if let Some(round) = round {
        write_research_round_evidence(round, batch, report, patch_receipt, flags)?;
    }
    Ok(material_report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_headless_sdk::{
        MockHeadlessExecutor, build_template_document, execute_batch_with_executor,
        normalize_workflow_document,
    };

    #[test]
    fn material_generation_still_rejects_incomplete_retained_result_sets_without_mutating_receipts()
    {
        let batch = normalize_workflow_document(
            &build_template_document("material_heat_spreader_screening", None).unwrap(),
        )
        .unwrap();
        let report = execute_batch_with_executor(&batch, &mut MockHeadlessExecutor, false, false);
        let flags = Flags::parse(&["--material-report".into(), "heat-spreader".into()]).unwrap();
        // Corrupt only a retained result set; the normal CLI now rejects incomplete plans earlier.
        for retained in [2, 3] {
            let mut incomplete = report.clone();
            incomplete.steps.truncate(retained);
            let before = serde_json::to_value(&incomplete).unwrap();
            let error = publish_artifacts(&flags, &batch, &incomplete, None, None).unwrap_err();
            assert!(error.starts_with(GENERATION_FAILURE));
            assert_eq!(serde_json::to_value(incomplete).unwrap(), before);
        }
    }
}
