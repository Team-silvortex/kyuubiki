use crate::{
    complete_remote_deployment_step, default_remote_deployment_plan,
    prepare_remote_deployment_resume, remote_deployment_journal_digest,
    remote_deployment_journal_for_plan, run_installer_journal_replay_fault_injection,
    start_remote_deployment_step, verify_remote_deployment_journal,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn journal_state_machine_tracks_attempts_and_revision() {
    let plan = default_remote_deployment_plan();
    let mut journal = remote_deployment_journal_for_plan(&plan, "test-agent");
    start_remote_deployment_step(&plan, &mut journal, "policy-check")
        .expect("first step should start");
    assert_eq!(journal.revision, 1);
    assert_eq!(journal.status, "running");
    complete_remote_deployment_step(&plan, &mut journal, "policy-check")
        .expect("running step should complete");
    assert_eq!(journal.revision, 2);
    assert_eq!(journal.status, "ready");
    assert_eq!(journal.records[0].attempt, 1);
    verify_remote_deployment_journal(&plan, &journal).expect("advanced journal should verify");
}

#[test]
fn journal_rejects_out_of_order_and_semantically_tampered_steps() {
    let plan = default_remote_deployment_plan();
    let mut journal = remote_deployment_journal_for_plan(&plan, "test-agent");
    let error = start_remote_deployment_step(&plan, &mut journal, "sync-artifacts")
        .expect_err("later step must not start before its predecessors");
    assert!(error.contains("out of order"));

    journal.records[0].local_record_path = "../outside.jsonl".to_string();
    journal.journal_digest = remote_deployment_journal_digest(&journal);
    let error = verify_remote_deployment_journal(&plan, &journal)
        .expect_err("a re-signed path change must still fail plan verification");
    assert!(error.contains("mismatches its plan"));
}

#[test]
fn replay_marks_running_step_interrupted_without_replaying_completed_prefix() {
    let plan = default_remote_deployment_plan();
    let mut journal = remote_deployment_journal_for_plan(&plan, "test-agent");
    start_remote_deployment_step(&plan, &mut journal, "policy-check").expect("start step");
    complete_remote_deployment_step(&plan, &mut journal, "policy-check").expect("complete step");
    start_remote_deployment_step(&plan, &mut journal, "bootstrap-workspace")
        .expect("start interrupted step");

    let (recovered, resume) =
        prepare_remote_deployment_resume(&plan, &journal).expect("prepare replay");
    assert_eq!(recovered.status, "interrupted");
    assert_eq!(recovered.records[0].status, "completed");
    assert_eq!(recovered.records[0].attempt, 1);
    assert_eq!(
        resume.resume_step_id.as_deref(),
        Some("bootstrap-workspace")
    );
    assert_eq!(resume.completed_step_ids, vec!["policy-check"]);
    assert_eq!(resume.reason_code, "resume_from_first_incomplete_step");
}

#[test]
fn native_fault_injection_recovers_partial_commit_and_rejects_tamper() {
    let root = unique_test_root();
    let report = run_installer_journal_replay_fault_injection(&root)
        .expect("fault-injection probe should close both recovery scenarios");
    assert_eq!(report.status, "passed");
    assert_eq!(report.scenario_count, 2);
    assert_eq!(report.process_loss_replay.resume_step_id, "sync-artifacts");
    assert!(!report.process_loss_replay.completed_step_replayed);
    assert_eq!(report.process_loss_replay.interrupted_attempt_after, 2);
    assert!(report.process_loss_replay.power_loss_sidecar_recovered);
    assert!(report.process_loss_replay.partial_next_ignored);
    assert!(report.process_loss_replay.probe_state_cleaned);
    assert!(report.digest_tamper_recovery.digest_tamper_rejected);
    assert!(report.digest_tamper_recovery.valid_journal_preserved);
    assert!(report.digest_tamper_recovery.probe_state_cleaned);
    fs::remove_dir(&root).expect("probe parent should be empty after scenario cleanup");
    assert!(!root.exists());
}

fn unique_test_root() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kyuubiki-installer-replay-test-{}-{timestamp}",
        std::process::id()
    ))
}
