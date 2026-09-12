use crate::{
    Platform, default_remote_artifact_delivery_manifest, default_remote_deployment_dry_run,
    default_remote_deployment_journal, default_remote_deployment_plan,
    default_remote_host_trust_plan, default_remote_ssh_fixture_plan,
    default_remote_ssh_fixture_report, remote_deployment_roadmap, unified_update_plan,
};

#[test]
fn remote_deployment_roadmap_marks_ssh_wrapper_as_pilot() {
    let roadmap = remote_deployment_roadmap();
    assert_eq!(
        roadmap.schema_version,
        "kyuubiki.remote-deployment-roadmap/v1"
    );
    assert!(roadmap.current_maturity.contains("pilot"));
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "deployment-plan" && stage.status == "started")
    );
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "remote-journal" && stage.status == "started")
    );
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "artifact-delivery" && stage.status == "started")
    );
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "dry-run-preflight" && stage.status == "started")
    );
    assert!(
        roadmap
            .stages
            .iter()
            .any(|stage| stage.id == "integration-tests" && stage.status == "started")
    );
    assert!(roadmap.render().contains("Policy-bounded SSH transport"));
}

#[test]
fn remote_deployment_plan_has_retry_safe_step_contract() {
    let plan = default_remote_deployment_plan();
    assert_eq!(plan.schema_version, "kyuubiki.remote-deployment-plan/v1");
    assert!(plan.steps.iter().any(|step| step.id == "policy-check"));
    assert!(plan.steps.iter().any(|step| step.id == "verify-integrity"));
    assert!(plan.steps.iter().any(|step| step.id == "health-check"));
    assert!(
        plan.steps
            .iter()
            .all(|step| !step.idempotency_key.is_empty() && !step.failure_class.is_empty())
    );
    assert!(plan.render().contains("rollback_hint:"));
}

#[test]
fn remote_deployment_journal_matches_plan_steps() {
    let plan = default_remote_deployment_plan();
    let journal = default_remote_deployment_journal();
    assert_eq!(
        journal.schema_version,
        "kyuubiki.remote-deployment-journal/v2"
    );
    assert_eq!(journal.revision, 0);
    assert_eq!(journal.status, "pending");
    assert_eq!(journal.plan_digest.len(), 64);
    assert_eq!(journal.journal_digest.len(), 64);
    assert_eq!(journal.records.len(), plan.steps.len());
    assert!(
        journal
            .records
            .iter()
            .all(|record| record.status == "pending")
    );
    assert!(journal.records.iter().all(|record| {
        record
            .local_record_path
            .starts_with(".kyuubiki/remote-journal/")
    }));
    assert!(journal.render().contains("remote deployment journal"));
}

#[test]
fn default_remote_artifact_delivery_matches_published_platform_artifacts() {
    let plan = unified_update_plan(None).expect("default update catalog should be valid");
    let platform = Platform::current().as_str();
    let declared: Vec<_> = plan
        .artifacts
        .iter()
        .filter(|artifact| artifact.platform == platform)
        .map(|artifact| artifact.path.as_str())
        .collect();
    let result = default_remote_artifact_delivery_manifest();
    if declared.is_empty() {
        assert_eq!(
            result.unwrap_err(),
            format!(
                "no remote-deliverable artifacts declared for {platform} on channel {}",
                plan.target_channel
            )
        );
        return;
    }
    let manifest = result.expect("published platform artifacts should be deliverable");
    assert_eq!(manifest.platform, platform);
    assert_eq!(manifest.channel, plan.target_channel);
    assert_eq!(manifest.target_version, plan.target_version);
    assert!(
        manifest
            .artifacts
            .iter()
            .map(|artifact| artifact.source_path.as_str())
            .eq(declared)
    );
}

#[test]
fn remote_deployment_dry_run_summarizes_readiness_without_ssh() {
    let report = default_remote_deployment_dry_run();
    assert_eq!(
        report.schema_version,
        "kyuubiki.remote-deployment-dry-run/v1"
    );
    assert_eq!(report.plan.steps.len(), report.journal.records.len());
    assert!(matches!(
        report.status.as_str(),
        "ready_for_preflight" | "blocked"
    ));
    assert!(
        report
            .render()
            .contains("does not open SSH sessions or mutate hosts")
    );
    assert!(!report.next_actions.is_empty());
}

#[test]
fn remote_ssh_fixture_keeps_command_shape_local_and_secret_free() {
    let report = default_remote_ssh_fixture_report();
    assert_eq!(report.schema_version, "kyuubiki.remote-ssh-fixture/v1");
    assert_eq!(report.fixture_target, "kyuubiki-fixture@fixture.local");
    assert!(
        report
            .commands
            .iter()
            .any(|command| command.program == "ssh")
    );
    assert!(
        report
            .commands
            .iter()
            .any(|command| command.program == "scp")
    );
    assert!(report.checks.iter().all(|check| check.ok));
    assert!(report.render().contains("does not open network sockets"));
}

#[test]
fn remote_host_trust_plan_separates_dev_accept_new_from_pinned_managed_mode() {
    let plan = default_remote_host_trust_plan();
    assert_eq!(plan.schema_version, "kyuubiki.remote-host-trust/v1");
    assert_eq!(plan.current_mode, "dev-accept-new");
    assert_eq!(plan.target_mode, "pinned-known-host");
    assert!(plan.options.iter().any(|option| {
        option.phase == "dev"
            && option.key == "StrictHostKeyChecking"
            && option.value == "accept-new"
    }));
    assert!(plan.options.iter().any(|option| {
        option.phase == "managed" && option.key == "StrictHostKeyChecking" && option.value == "yes"
    }));
    assert!(plan.render().contains("does not write known_hosts files"));
}

#[test]
fn remote_deployment_roadmap_marks_host_trust_as_started() {
    let roadmap = remote_deployment_roadmap();
    let host_trust = roadmap
        .stages
        .iter()
        .find(|stage| stage.id == "host-trust")
        .expect("host trust stage should exist");
    assert_eq!(host_trust.status, "started");
    assert!(
        host_trust
            .exit_criteria
            .iter()
            .any(|criterion| criterion.contains("managed pinned-host path"))
    );
}

#[test]
fn remote_ssh_fixture_plan_points_to_ignored_runtime_state() {
    let plan = default_remote_ssh_fixture_plan();
    assert_eq!(plan.schema_version, "kyuubiki.remote-ssh-fixture-plan/v1");
    assert_eq!(plan.bind_address, "127.0.0.1:2222");
    assert!(plan.manual_only);
    assert!(
        plan.compose_file
            .ends_with("remote-ssh-fixture/compose.yaml")
    );
    assert_eq!(plan.run_script, "scripts/run-remote-ssh-fixture.sh");
    assert!(
        plan.ignored_runtime_paths
            .iter()
            .all(|path| path.contains("remote-ssh-fixture/runtime"))
    );
    assert!(plan.render().contains("does not start containers"));
}
