use kyuubiki_installer::{
    DesktopBundleQualificationReport, DesktopBundleQualificationSummary, Platform,
    desktop_bundle_source_layout, run_desktop_bundle_qualification,
    validate_desktop_bundle_qualification_report, write_desktop_bundle_qualification_report,
};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_qualify_host(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = QualifyOptions::parse(root, args)?;
    if options.help {
        print_qualify_usage();
        return Ok(0);
    }
    let platform = Platform::current();
    ensure_bundle_source(&options.bundle_root, platform)?;
    let run_root = qualification_run_root(root)?;
    let guard = RunRootGuard(run_root.clone());
    let first_source = run_root.join("sources/first");
    let second_source = run_root.join("sources/second");
    stage_variant(
        &options.bundle_root,
        &first_source,
        platform,
        "baseline",
        &options.first_version,
    )?;
    stage_variant(
        &options.bundle_root,
        &second_source,
        platform,
        "candidate",
        &options.second_version,
    )?;
    let report = run_desktop_bundle_qualification(
        &first_source,
        &second_source,
        &run_root.join("qualification"),
        &options.first_version,
        &options.second_version,
    )?;
    validate_for_platform(&report, Some(platform.as_str()))?;
    write_desktop_bundle_qualification_report(&report, &options.output)?;
    let summary = verify_report(&options.output, Some(platform.as_str()))?;
    drop(guard);
    println!(
        "packaged desktop update qualification passed: {} -> {} -> {}, {} probes, {}",
        summary.first_version,
        summary.second_version,
        summary.first_version,
        summary.probe_count,
        display_path(root, &options.output)
    );
    Ok(0)
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(root, args)?;
    if options.help {
        print_check_usage();
        return Ok(0);
    }
    let path = options
        .report
        .unwrap_or(default_report_path(root, &development_version(root)?));
    let summary = verify_report(&path, options.require_platform.as_deref())?;
    println!(
        "packaged desktop update qualification report passed: {} -> {}, {}, {} probes, {} checks",
        summary.first_version,
        summary.second_version,
        summary.platform,
        summary.probe_count,
        summary.check_count
    );
    Ok(0)
}

struct QualifyOptions {
    help: bool,
    bundle_root: PathBuf,
    output: PathBuf,
    first_version: String,
    second_version: String,
}

impl QualifyOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let second_version = development_version(root)?;
        let first_version = previous_version(&second_version)?;
        let mut options = Self {
            help: false,
            bundle_root: default_bundle_root(root, Platform::current()),
            output: default_report_path(root, &second_version),
            first_version,
            second_version,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--bundle-root" => {
                    options.bundle_root =
                        repo_resolve(root, next_path(&mut args, "--bundle-root")?);
                }
                "--out" => {
                    options.output = repo_resolve(root, next_path(&mut args, "--out")?);
                }
                "--first-version" => {
                    options.first_version = next_string(&mut args, "--first-version")?;
                }
                "--second-version" => {
                    options.second_version = next_string(&mut args, "--second-version")?;
                }
                other => return Err(format!("unknown desktop qualification option: {other}")),
            }
        }
        validate_version_pair(&options.first_version, &options.second_version)?;
        Ok(options)
    }
}

struct CheckOptions {
    help: bool,
    report: Option<PathBuf>,
    require_platform: Option<String>,
}

impl CheckOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            report: None,
            require_platform: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--verify-report" | "--in" => {
                    options.report =
                        Some(repo_resolve(root, next_path(&mut args, "--verify-report")?));
                }
                "--require-platform" => {
                    let platform = next_string(&mut args, "--require-platform")?;
                    if !matches!(platform.as_str(), "macos" | "linux" | "windows") {
                        return Err("--require-platform must be macos, linux, or windows".into());
                    }
                    options.require_platform = Some(platform);
                }
                other => return Err(format!("unknown desktop report option: {other}")),
            }
        }
        Ok(options)
    }
}

struct RunRootGuard(PathBuf);

impl Drop for RunRootGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stage_variant(
    source_root: &Path,
    target_root: &Path,
    platform: Platform,
    variant: &str,
    package_version: &str,
) -> RunnerResult<()> {
    fs::create_dir_all(target_root)
        .map_err(|error| format!("failed to create {}: {error}", target_root.display()))?;
    for layout in desktop_bundle_source_layout(platform) {
        let source = source_root.join(layout.bundle_path);
        let target = target_root.join(layout.bundle_path);
        copy_tree(&source, &target)?;
        decorate_component(
            &target,
            layout.component_id,
            platform,
            variant,
            package_version,
        )?;
    }
    if platform == Platform::Macos {
        sign_macos_apps(target_root)?;
    }
    Ok(())
}

fn decorate_component(
    component: &Path,
    component_id: &str,
    platform: Platform,
    variant: &str,
    package_version: &str,
) -> RunnerResult<()> {
    let marker = serde_json::to_vec_pretty(&json!({
        "schema_version": "kyuubiki.desktop-bundle-qualification-marker/v1",
        "component": component_id,
        "package_version": package_version,
        "variant": variant,
    }))
    .map_err(|error| format!("failed to serialize qualification marker: {error}"))?;
    if platform == Platform::Linux {
        let mut file = OpenOptions::new()
            .append(true)
            .open(component)
            .map_err(|error| format!("failed to decorate {}: {error}", component.display()))?;
        file.write_all(b"\nKYUUBIKI_DESKTOP_QUALIFICATION\0")
            .and_then(|_| file.write_all(&marker))
            .map_err(|error| format!("failed to decorate {}: {error}", component.display()))?;
        return Ok(());
    }
    let marker_path = if platform == Platform::Macos {
        component.join("Contents/Resources/kyuubiki-update-qualification.json")
    } else {
        component.join("kyuubiki-update-qualification.json")
    };
    fs::write(&marker_path, marker)
        .map_err(|error| format!("failed to write {}: {error}", marker_path.display()))
}

fn sign_macos_apps(root: &Path) -> RunnerResult<()> {
    for layout in desktop_bundle_source_layout(Platform::Macos) {
        let app = root.join(layout.bundle_path);
        let output = Command::new("/usr/bin/codesign")
            .args(["--force", "--deep", "--options", "runtime", "--sign", "-"])
            .arg(&app)
            .output()
            .map_err(|error| format!("failed to launch codesign for {}: {error}", app.display()))?;
        if !output.status.success() {
            return Err(format!(
                "failed to sign {}: {}",
                app.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> RunnerResult<()> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("failed to inspect {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "desktop qualification source cannot contain symlinks: {}",
            source.display()
        ));
    }
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::copy(source, target).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                source.display(),
                target.display()
            )
        })?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("unsupported desktop source: {}", source.display()));
    }
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        copy_tree(&entry.path(), &target.join(entry.file_name()))?;
    }
    Ok(())
}

fn ensure_bundle_source(root: &Path, platform: Platform) -> RunnerResult<()> {
    for layout in desktop_bundle_source_layout(platform) {
        let bundle = root.join(layout.bundle_path);
        let entrypoint = root.join(layout.entrypoint);
        if !bundle.exists() || !entrypoint.is_file() {
            return Err(format!(
                "packaged desktop source misses {}: {}",
                layout.component_id,
                root.display()
            ));
        }
    }
    Ok(())
}

fn verify_report(
    path: &Path,
    require_platform: Option<&str>,
) -> RunnerResult<DesktopBundleQualificationSummary> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let report: DesktopBundleQualificationReport = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid report: {error}", path.display()))?;
    validate_for_platform(&report, require_platform)
}

fn validate_for_platform(
    report: &DesktopBundleQualificationReport,
    require_platform: Option<&str>,
) -> RunnerResult<DesktopBundleQualificationSummary> {
    let summary =
        validate_desktop_bundle_qualification_report(report).map_err(|errors| errors.join("; "))?;
    if require_platform.is_some_and(|platform| summary.platform != platform) {
        return Err(format!(
            "desktop qualification requires platform {}, report contains {}",
            require_platform.unwrap_or_default(),
            summary.platform
        ));
    }
    Ok(summary)
}

fn qualification_run_root(root: &Path) -> RunnerResult<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = root.join(format!(
        "tmp/desktop-bundle-update-qualification-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    Ok(path)
}

fn development_version(root: &Path) -> RunnerResult<String> {
    let path = root.join("docs/book-manifest.json");
    let value: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
    value
        .get("current_development_version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "book manifest misses current_development_version".into())
}

fn previous_version(version: &str) -> RunnerResult<String> {
    let [major, minor, patch] = parse_version(version)?;
    if minor == 0 && patch == 0 {
        return Err(format!(
            "cannot derive previous moxi version from {version}"
        ));
    }
    Ok(if patch > 0 {
        format!("{major}.{minor}.{}", patch - 1)
    } else {
        format!("{major}.{}.9", minor - 1)
    })
}

fn validate_version_pair(first: &str, second: &str) -> RunnerResult<()> {
    if parse_version(first)? >= parse_version(second)? {
        return Err("desktop qualification versions must be strictly increasing".into());
    }
    Ok(())
}

fn parse_version(version: &str) -> RunnerResult<[u64; 3]> {
    let parts = version
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("invalid semantic version: {version}"))?;
    parts
        .try_into()
        .map_err(|_| format!("invalid semantic version: {version}"))
}

fn default_bundle_root(root: &Path, platform: Platform) -> PathBuf {
    match platform {
        Platform::Macos => root.join("target/desktop-cache/macos/release/bundle/macos"),
        Platform::Linux => PathBuf::from("/usr/bin"),
        Platform::Windows => std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/desktop-cache/windows/release")),
    }
}

fn default_report_path(root: &Path, version: &str) -> PathBuf {
    root.join(format!(
        "releases/usability-evidence/{version}/desktop-bundle-update-operational-qualification.json"
    ))
}

fn repo_resolve(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn next_string(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn next_path(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<PathBuf> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_qualify_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-desktop-bundle-update-operational-host [--bundle-root path] [--out path] [--first-version version] [--second-version version]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-desktop-bundle-update-operational-qualification [--verify-report path] [--require-platform macos|linux|windows]"
    );
}

#[cfg(test)]
mod tests {
    use super::{previous_version, validate_version_pair};

    #[test]
    fn derives_previous_patch_across_minor_boundaries() {
        assert_eq!(previous_version("2.17.1").unwrap(), "2.17.0");
        assert_eq!(previous_version("2.17.0").unwrap(), "2.16.9");
        assert!(previous_version("2.0.0").is_err());
    }

    #[test]
    fn qualification_versions_must_increase() {
        assert!(validate_version_pair("2.16.9", "2.17.0").is_ok());
        assert!(validate_version_pair("2.17.0", "2.16.9").is_err());
        assert!(validate_version_pair("2.17", "2.17.0").is_err());
    }
}
