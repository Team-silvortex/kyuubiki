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
    report::validate_contract(root, false)?;
    let (journey, cleanup) = runtime::capture(
        root,
        &options.host,
        &options.postgres_image,
        Duration::from_secs(options.timeout_seconds),
    )?;
    let qualification = report::build_report(journey, cleanup)?;
    report::validate(root, &qualification)?;
    report::write(root, &options.output, &qualification)?;
    println!(
        "two-Orchestra PostgreSQL takeover qualification passed: {}",
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
    report::validate_contract(root, false)?;
    if options.self_test {
        report::validator_self_test(root)?;
        println!("Orchestra takeover operational qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options
        .report
        .unwrap_or_else(|| report::DEFAULT_REPORT.to_string());
    let qualification = report::read(root, &report_path)?;
    report::validate(root, &qualification)?;
    println!("Orchestra takeover operational report passed: {report_path}");
    Ok(0)
}

struct CaptureOptions {
    help: bool,
    host: String,
    output: String,
    postgres_image: String,
    timeout_seconds: u64,
}

impl CaptureOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: report::DEFAULT_CAPTURE.to_string(),
            postgres_image: env::var("KYUUBIKI_POSTGRES_QUALIFICATION_IMAGE")
                .unwrap_or_else(|_| "postgres:16-alpine".to_string()),
            timeout_seconds: 120,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = required_string(&mut iter, "--host")?,
                "--out" => options.output = required_path(&mut iter, "--out")?,
                "--postgres-image" => {
                    options.postgres_image = required_string(&mut iter, "--postgres-image")?
                }
                "--timeout-secs" => {
                    options.timeout_seconds = required_string(&mut iter, "--timeout-secs")?
                        .parse()
                        .map_err(|_| "--timeout-secs requires an integer".to_string())?;
                }
                other => return Err(format!("unknown Orchestra takeover option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        if !valid_image_reference(&options.postgres_image) {
            return Err("PostgreSQL image must be a portable container reference".to_string());
        }
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
                other => return Err(format!("unknown Orchestra takeover check option: {other}")),
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
    let path = Path::new(&value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!("qualification path escapes repository: {value}"));
    }
    Ok(value)
}

fn valid_image_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'_' | b'-' | b':')
        })
}

fn print_capture_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-orchestra-takeover-operational-remote [--host SSH_ALIAS] [--out path] [--postgres-image image] [--timeout-secs seconds]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-orchestra-takeover-operational-qualification [--self-test] [--verify-report path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_options_reject_injection_and_path_escape() {
        assert!(CaptureOptions::parse(vec!["--host".into(), "-oProxyCommand=x".into()]).is_err());
        assert!(CaptureOptions::parse(vec!["--out".into(), "../report.json".into()]).is_err());
        assert!(CaptureOptions::parse(vec!["--postgres-image".into(), "pg;id".into()]).is_err());
    }

    #[test]
    fn capture_timeout_is_bounded() {
        assert!(CaptureOptions::parse(vec!["--timeout-secs".into(), "29".into()]).is_err());
        assert!(CaptureOptions::parse(vec!["--timeout-secs".into(), "120".into()]).is_ok());
    }
}
