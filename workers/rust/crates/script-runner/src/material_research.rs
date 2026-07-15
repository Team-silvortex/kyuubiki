use crate::{
    material_card_contract, material_research_bundle, material_research_bundle_build,
    material_research_bundle_contract, material_research_bundle_index, material_research_example,
    material_study_sdk_examples, remote_material_health, remote_material_research_example,
    remote_material_stage_health, remote_material_summary,
};
use std::ffi::OsString;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_material_research_command(
    root: &Path,
    command: &str,
    args: Vec<OsString>,
) -> Option<RunnerResult<u8>> {
    Some(match command {
        "check-material-card-contract" => {
            material_card_contract::run_check_material_card_contract(root, args)
        }
        "check-material-research-bundle-contract" => {
            material_research_bundle_contract::run_check_material_research_bundle_contract(
                root, args,
            )
        }
        "check-material-research-bundle" => {
            material_research_bundle::run_check_material_research_bundle(root, args)
        }
        "build-material-research-bundle" => {
            material_research_bundle_build::run_build_material_research_bundle(root, args)
        }
        "build-material-research-bundle-index" => {
            material_research_bundle_index::run_build_material_research_bundle_index(root, args)
        }
        "capture-material-research-example" => {
            material_research_example::run_capture_material_research_example(root, args)
        }
        "check-material-research-example" => {
            material_research_example::run_check_material_research_example(root, args)
        }
        "check-material-study-sdk-examples" => {
            material_study_sdk_examples::run_check_material_study_sdk_examples(root, args)
        }
        "check-remote-material-preconditioner-health" => {
            remote_material_health::run_check_remote_material_preconditioner_health(root, args)
        }
        "check-remote-material-stage-health" => {
            remote_material_stage_health::run_check_remote_material_stage_health(root, args)
        }
        "build-remote-material-benchmark-summary" => {
            remote_material_summary::run_build_remote_material_benchmark_summary(root, args)
        }
        "remote-material-research-example" => {
            remote_material_research_example::run_remote_material_research_example(root, args)
        }
        _ => return None,
    })
}
