use kyuubiki_headless_sdk::{
    KyuubikiSession, ModelApprovalVerifier, ModelCollaborationSession, ModelHeadlessPlan,
    ModelPlanApproval, ModelResearchExecutionStatus, ModelWorkflowProposal, SdkError, SdkResult,
    SessionModelActionDispatcher, build_model_headless_plan, execute_model_headless_plan,
};
use serde::de::DeserializeOwned;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("model research execution failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        return Err(
            "usage: execute_model_research_plan SESSION.json PROPOSAL.json APPROVAL.json".into(),
        );
    }

    let session: ModelCollaborationSession = read_json(&arguments[0])?;
    let proposal: ModelWorkflowProposal = read_json(&arguments[1])?;
    let approval: ModelPlanApproval = read_json(&arguments[2])?;
    let plan = build_model_headless_plan(&session, &proposal)?;
    let base_url = env::var("KYUUBIKI_BASE_URL")
        .map_err(|_| "KYUUBIKI_BASE_URL must name the configured control plane")?;
    let token = env::var("KYUUBIKI_ACCESS_TOKEN").ok();
    let approval_verifier = EnvironmentApprovalVerifier {
        approval_id: env::var("KYUUBIKI_APPROVAL_ID")
            .map_err(|_| "KYUUBIKI_APPROVAL_ID must confirm the reviewed approval")?,
        authority: env::var("KYUUBIKI_APPROVAL_AUTHORITY")
            .map_err(|_| "KYUUBIKI_APPROVAL_AUTHORITY must name the caller authority")?,
    };
    let headless = KyuubikiSession::from_control_plane(&base_url, token)?;
    let dispatcher = SessionModelActionDispatcher::new(&headless);
    let receipt =
        execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &approval_verifier)?;

    println!("{}", serde_json::to_string_pretty(&receipt)?);
    if receipt.status == ModelResearchExecutionStatus::Failed {
        std::process::exit(2);
    }
    Ok(())
}

struct EnvironmentApprovalVerifier {
    approval_id: String,
    authority: String,
}

impl ModelApprovalVerifier for EnvironmentApprovalVerifier {
    fn verify_model_approval(
        &self,
        _plan: &ModelHeadlessPlan,
        approval: &ModelPlanApproval,
    ) -> SdkResult<()> {
        if approval.approval_id == self.approval_id && approval.authority == self.authority {
            Ok(())
        } else {
            Err(SdkError::Validation {
                errors: vec![
                    "approval file does not match caller-owned approval environment".to_string(),
                ],
            })
        }
    }
}

fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}
