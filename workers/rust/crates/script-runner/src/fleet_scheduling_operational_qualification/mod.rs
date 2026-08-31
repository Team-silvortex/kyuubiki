mod agent_process;
mod probe;
mod remote;
mod report;
mod runtime;

use crate::remote_host::valid_ssh_alias;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_qualify_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = RemoteOptions::parse(args)?;
    if options.help {
        print_remote_usage();
        return Ok(0);
    }
    report::validate_contract(root)?;
    let output = root.join(&options.output);
    let status = remote::capture(
        root,
        &options.host,
        &output,
        &options.package_version,
        Duration::from_secs(options.timeout_seconds),
    )?;
    if status == 0 {
        println!(
            "remote Installer-managed fleet scheduling qualification passed: {}",
            options.output
        );
    }
    Ok(status)
}

pub(crate) fn run_capture_host(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = HostOptions::parse(args)?;
    if options.help {
        print_host_usage();
        return Ok(0);
    }
    report::validate_contract(root)?;
    let captured = runtime::capture_host(
        root,
        &options.agent_binary,
        &options.work_root,
        &options.package_version,
        Duration::from_secs(options.timeout_seconds),
    )?;
    let qualification = report::build(captured)?;
    report::validate(root, &qualification)?;
    report::write_path(&options.output, &qualification)?;
    println!(
        "fleet scheduling host capture passed: {}",
        options.output.display()
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
        println!("fleet scheduling operational qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let path = options
        .report
        .unwrap_or_else(|| report::DEFAULT_REPORT.to_string());
    let qualification = report::read(root, &path)?;
    report::validate(root, &qualification)?;
    println!("fleet scheduling operational qualification report passed: {path}");
    Ok(0)
}

struct RemoteOptions {
    help: bool,
    host: String,
    output: String,
    package_version: String,
    timeout_seconds: u64,
}

impl RemoteOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: report::DEFAULT_CAPTURE.to_string(),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            timeout_seconds: 120,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = required_string(&mut iter, "--host")?,
                "--out" => options.output = required_relative_path(&mut iter, "--out")?,
                "--package-version" => {
                    options.package_version = required_string(&mut iter, "--package-version")?
                }
                "--timeout-secs" => {
                    options.timeout_seconds = parse_timeout(&mut iter, "--timeout-secs")?
                }
                other => return Err(format!("unknown fleet scheduling capture option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        validate_version(&options.package_version)?;
        Ok(options)
    }
}

struct HostOptions {
    help: bool,
    agent_binary: PathBuf,
    work_root: PathBuf,
    output: PathBuf,
    package_version: String,
    timeout_seconds: u64,
}

impl HostOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut help = false;
        let mut agent_binary = None;
        let mut work_root = None;
        let mut output = None;
        let mut package_version = None;
        let mut timeout_seconds = 120;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => help = true,
                "--agent-binary" => {
                    agent_binary = Some(required_absolute_path(&mut iter, "--agent-binary")?)
                }
                "--work-root" => {
                    work_root = Some(required_absolute_path(&mut iter, "--work-root")?)
                }
                "--out" => output = Some(required_absolute_path(&mut iter, "--out")?),
                "--package-version" => {
                    package_version = Some(required_string(&mut iter, "--package-version")?)
                }
                "--timeout-secs" => timeout_seconds = parse_timeout(&mut iter, "--timeout-secs")?,
                other => return Err(format!("unknown fleet scheduling host option: {other}")),
            }
        }
        if help {
            return Ok(Self {
                help,
                agent_binary: PathBuf::new(),
                work_root: PathBuf::new(),
                output: PathBuf::new(),
                package_version: String::new(),
                timeout_seconds,
            });
        }
        let package_version = package_version.ok_or("--package-version is required")?;
        validate_version(&package_version)?;
        let work_root = work_root.ok_or("--work-root is required")?;
        let output = output.ok_or("--out is required")?;
        if output.starts_with(&work_root) {
            return Err(
                "fleet scheduling report must live outside the disposable work root".into(),
            );
        }
        Ok(Self {
            help,
            agent_binary: agent_binary.ok_or("--agent-binary is required")?,
            work_root,
            output,
            package_version,
            timeout_seconds,
        })
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
                    options.report = Some(required_relative_path(&mut iter, "--verify-report")?)
                }
                other => return Err(format!("unknown fleet scheduling check option: {other}")),
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

fn required_relative_path(
    iter: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> RunnerResult<String> {
    let value = required_string(iter, option)?;
    let path = Path::new(&value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        Err(format!("{option} must remain repository-relative"))
    } else {
        Ok(value)
    }
}

fn required_absolute_path(
    iter: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> RunnerResult<PathBuf> {
    let path = PathBuf::from(required_string(iter, option)?);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(format!("{option} requires an absolute managed path"))
    }
}

fn parse_timeout(iter: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<u64> {
    let timeout = required_string(iter, option)?
        .parse::<u64>()
        .map_err(|_| format!("{option} requires an integer"))?;
    if (30..=300).contains(&timeout) {
        Ok(timeout)
    } else {
        Err(format!("{option} must be between 30 and 300"))
    }
}

fn validate_version(version: &str) -> RunnerResult<()> {
    let mut parts = version.split('.');
    if parts.clone().count() == 3
        && parts.all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err("fleet scheduling package version must use major.minor.patch".to_string())
    }
}

fn print_remote_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-fleet-scheduling-operational-remote [--host SSH_ALIAS] [--out path] [--package-version version] [--timeout-secs seconds]"
    );
}

fn print_host_usage() {
    println!(
        "usage: kyuubiki-script-runner capture-fleet-scheduling-operational-host --agent-binary path --work-root path --out path --package-version version [--timeout-secs seconds]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-fleet-scheduling-operational-qualification [--self-test] [--verify-report path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_options_reject_path_and_ssh_injection() {
        assert!(RemoteOptions::parse(vec!["--host".into(), "-oProxyCommand=x".into()]).is_err());
        assert!(RemoteOptions::parse(vec!["--out".into(), "../report.json".into()]).is_err());
    }

    #[test]
    fn timeout_and_versions_are_bounded() {
        assert!(RemoteOptions::parse(vec!["--timeout-secs".into(), "29".into()]).is_err());
        assert!(validate_version("2.19.0").is_ok());
        assert!(validate_version("2.19.0-dev").is_err());
    }
}
