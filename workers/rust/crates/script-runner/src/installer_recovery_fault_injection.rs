use kyuubiki_installer::{
    InstallerJournalReplayFaultInjectionReport, run_installer_journal_replay_fault_injection,
};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const REPORT_SCHEMA: &str = "kyuubiki.installer-journal-replay-fault-injection/v1";

#[derive(Default)]
struct Options {
    out: Option<PathBuf>,
    verify_report: Option<PathBuf>,
    self_test: bool,
}

pub(crate) fn run_check_installer_recovery_fault_injection(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(root, args)?;
    if let Some(path) = options.verify_report {
        validate_report(&read_report(&path)?)?;
        println!(
            "Installer journal replay fault injection report passed: {}",
            path.display()
        );
        return Ok(0);
    }

    let temporary_root = root.join("tmp/installer-journal-replay-fault-injection");
    fs::create_dir_all(&temporary_root)
        .map_err(|error| format!("failed to create Installer probe root: {error}"))?;
    let report = run_installer_journal_replay_fault_injection(&temporary_root)?;
    validate_report(&report)?;

    if options.self_test {
        let mut tampered = report;
        tampered.process_loss_replay.completed_step_replayed = true;
        if validate_report(&tampered).is_ok() {
            return Err(
                "Installer recovery self-test accepted replayed completed work".to_string(),
            );
        }
        println!("Installer journal replay fault injection self-test passed");
        return Ok(0);
    }

    let path = options
        .out
        .unwrap_or_else(|| root.join("tmp/installer-journal-replay-fault-injection.json"));
    write_report(&path, &report)?;
    println!(
        "Installer journal replay fault injection passed: {}",
        path.display()
    );
    Ok(0)
}

fn parse_options(root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => options.out = Some(repo_path(root, required_path(&mut iter, "--out")?)?),
            "--verify-report" => {
                options.verify_report = Some(repo_path(
                    root,
                    required_path(&mut iter, "--verify-report")?,
                )?)
            }
            "--self-test" => options.self_test = true,
            other => return Err(format!("unknown Installer recovery argument: {other}")),
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    if options.self_test && (options.out.is_some() || options.verify_report.is_some()) {
        return Err("--self-test cannot be combined with report paths".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

fn repo_path(root: &Path, relative: String) -> RunnerResult<PathBuf> {
    let path = Path::new(&relative);
    if path.is_absolute() || path.components().any(|part| part.as_os_str() == "..") {
        return Err(format!(
            "Installer recovery report path escapes repository: {relative}"
        ));
    }
    Ok(root.join(path))
}

fn validate_report(report: &InstallerJournalReplayFaultInjectionReport) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA || report.status != "passed" {
        return Err("Installer recovery report summary is invalid".to_string());
    }
    if report.scenario_count != 2 {
        return Err("Installer recovery report must contain two scenarios".to_string());
    }
    let replay = &report.process_loss_replay;
    if replay.status != "passed"
        || replay.interrupted_step_id != "sync-artifacts"
        || replay.resume_step_id != "sync-artifacts"
        || replay.completed_before_loss != ["policy-check", "bootstrap-workspace"]
        || replay.completed_step_replayed
        || replay.interrupted_attempt_before != 1
        || replay.interrupted_attempt_after != 2
        || replay.final_status != "completed"
        || replay.pending_count != 0
        || !replay.journal_digest_valid
        || !replay.power_loss_sidecar_recovered
        || !replay.partial_next_ignored
        || !replay.probe_state_cleaned
    {
        return Err("Installer process-loss replay evidence is incomplete".to_string());
    }
    let tamper = &report.digest_tamper_recovery;
    if tamper.status != "passed"
        || !tamper.digest_tamper_rejected
        || tamper.error_class != "journal_digest_mismatch"
        || !tamper.valid_journal_preserved
        || !tamper.probe_state_cleaned
    {
        return Err("Installer digest-tamper recovery evidence is incomplete".to_string());
    }
    Ok(())
}

fn write_report(
    path: &Path,
    report: &InstallerJournalReplayFaultInjectionReport,
) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create report directory: {error}"))?;
    }
    let mut bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("failed to encode Installer recovery report: {error}"))?;
    bytes.push(b'\n');
    fs::write(path, bytes)
        .map_err(|error| format!("failed to write report {}: {error}", path.display()))
}

fn read_report(path: &Path) -> RunnerResult<InstallerJournalReplayFaultInjectionReport> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "failed to read Installer recovery report {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid Installer recovery report {}: {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_escape() {
        let error = repo_path(Path::new("/repo"), "../report.json".to_string()).unwrap_err();
        assert!(error.contains("escapes repository"));
    }

    #[test]
    fn rejects_replayed_completed_step() {
        let mut report = run_installer_journal_replay_fault_injection(&std::env::temp_dir())
            .expect("native Installer recovery probe should run");
        report.process_loss_replay.completed_step_replayed = true;
        let error = validate_report(&report).unwrap_err();
        assert!(error.contains("process-loss replay evidence"));
    }
}
