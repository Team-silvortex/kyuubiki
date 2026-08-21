mod agent_process;
mod probe;
mod report;
mod runtime;

use crate::remote_host::valid_ssh_alias;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_qualify_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CaptureOptions::parse(args)?;
    if options.help {
        print_capture_usage();
        return Ok(0);
    }
    report::validate_contract(root)?;
    let captured = runtime::capture(
        root,
        &options.host,
        Duration::from_secs(options.timeout_seconds),
    )?;
    let qualification = report::build_report(captured)?;
    report::validate(root, &qualification)?;
    report::write(root, &options.output, &qualification)?;
    println!(
        "distributed task recovery operational qualification passed: {}",
        options.output
    );
    Ok(0)
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        print_check_usage();
        return Ok(0);
    }
    report::validate_contract(root)?;
    if options.self_test {
        report::validator_self_test(root)?;
        println!("distributed task recovery qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options
        .report
        .unwrap_or_else(|| report::DEFAULT_REPORT.to_string());
    let qualification = report::read(root, &report_path)?;
    report::validate(root, &qualification)?;
    println!("distributed task recovery report passed: {report_path}");
    Ok(0)
}

struct CaptureOptions {
    help: bool,
    host: String,
    output: String,
    timeout_seconds: u64,
}

impl CaptureOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: report::DEFAULT_CAPTURE.to_string(),
            timeout_seconds: 90,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = required_string(&mut iter, "--host")?,
                "--out" => options.output = required_path(&mut iter, "--out")?,
                "--timeout-secs" => {
                    options.timeout_seconds = required_string(&mut iter, "--timeout-secs")?
                        .parse()
                        .map_err(|_| "--timeout-secs requires an integer".to_string())?;
                }
                other => return Err(format!("unknown distributed recovery option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        validate_relative_path(&options.output)?;
        if !(30..=300).contains(&options.timeout_seconds) {
            return Err("--timeout-secs must be between 30 and 300".to_string());
        }
        Ok(options)
    }
}

struct CheckOptions {
    help: bool,
    self_test: bool,
    report: Option<String>,
}

impl CheckOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            self_test: false,
            report: None,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--self-test" => options.self_test = true,
                "--verify-report" | "--in" => {
                    options.report = Some(required_path(&mut iter, "--verify-report")?)
                }
                other => {
                    return Err(format!(
                        "unknown distributed recovery check option: {other}"
                    ));
                }
            }
        }
        Ok(options)
    }
}

fn required_string(
    iter: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> RunnerResult<String> {
    iter.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    let value = required_string(iter, option)?;
    validate_relative_path(&value)?;
    Ok(value)
}

fn validate_relative_path(value: &str) -> RunnerResult<()> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        Err(format!("qualification path escapes repository: {value}"))
    } else {
        Ok(())
    }
}

fn print_capture_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-distributed-task-recovery-operational-remote [--host SSH_ALIAS] [--out path] [--timeout-secs seconds]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-distributed-task-recovery-operational-qualification [--self-test] [--verify-report path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_rejects_path_escape_and_ssh_option_injection() {
        assert!(CaptureOptions::parse(vec!["--host".into(), "-oProxyCommand=x".into()]).is_err());
        assert!(CaptureOptions::parse(vec!["--out".into(), "../report.json".into()]).is_err());
    }

    #[test]
    fn capture_timeout_is_bounded() {
        assert!(CaptureOptions::parse(vec!["--timeout-secs".into(), "29".into()]).is_err());
        assert!(CaptureOptions::parse(vec!["--timeout-secs".into(), "90".into()]).is_ok());
    }
}
