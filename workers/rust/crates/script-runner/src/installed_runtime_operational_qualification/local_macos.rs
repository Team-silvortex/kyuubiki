use crate::native_time::utc_timestamp_slug;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const CAPTURE_FILES: &[&str] = &[
    "solve.json",
    "fetch-report.json",
    "detached-fetch-report.json",
    "detached-status.txt",
    "installed-digests.txt",
];

pub(super) fn run(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args
        .iter()
        .any(|arg| matches!(arg.to_string_lossy().as_ref(), "--help" | "-h"))
    {
        print_usage();
        return Ok(0);
    }
    if env::consts::OS != "macos" || env::consts::ARCH != "aarch64" {
        return Err("macOS installed Runtime qualification requires local macOS/aarch64".into());
    }
    let contract = super::load_contract(root, super::MACOS_PROFILE)?;
    let options = Options::parse(root, &contract, args)?;
    let managed = root.join("tmp").join(&options.slug);
    let capture = root.join("tmp").join(format!("{}-capture", options.slug));
    if let Err(error) = prepare_root(&managed, &capture) {
        let cleanup = merge_cleanup(
            cleanup_root(&managed, "managed root"),
            cleanup_root(&capture, "capture root"),
        );
        return Err(with_cleanup(error, cleanup));
    }

    let qualification = qualify(root, &managed, &capture, &contract.capture.package_version);
    let cleanup = cleanup_roots(&managed);
    let result = match (qualification, cleanup) {
        (Ok(()), Ok(())) => super::remote::finalize_report(
            root,
            &contract,
            super::MACOS_PROFILE,
            &options.output,
            &capture,
        ),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
    };
    let result = merge_cleanup(result, cleanup_root(&capture, "capture root"));
    result?;
    println!(
        "local macOS installed Runtime operational qualification passed: {}",
        display_path(root, &options.output)
    );
    Ok(0)
}

struct Options {
    output: PathBuf,
    slug: String,
}

impl Options {
    fn parse(root: &Path, contract: &super::Contract, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut output = super::repo_path(root, &contract.retention.report_path)?;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--out" => {
                    output = super::repo_path(root, &next_string(&mut args, "--out")?)?;
                }
                other => return Err(format!("unknown installed Runtime macOS option: {other}")),
            }
        }
        let workspace_version = super::remote::workspace_version(root)?;
        if workspace_version != contract.capture.package_version {
            return Err(format!(
                "workspace version {workspace_version} does not match macOS qualification contract {}",
                contract.capture.package_version
            ));
        }
        Ok(Self {
            output,
            slug: format!("installed-runtime-macos-{}", utc_timestamp_slug()),
        })
    }
}

fn prepare_root(managed: &Path, capture: &Path) -> RunnerResult<()> {
    if managed.exists() || capture.exists() {
        return Err("macOS qualification root already exists".to_string());
    }
    fs::create_dir_all(managed.join("home"))
        .and_then(|()| fs::create_dir_all(capture))
        .map_err(|error| format!("failed to prepare macOS qualification roots: {error}"))
}

fn qualify(root: &Path, managed: &Path, capture: &Path, version: &str) -> RunnerResult<()> {
    let target = build_rust_binaries(root)?;
    let release = managed.join("orchestra-release");
    build_orchestra_release(root, &release, version)?;
    let payload = managed.join("payload");
    let installer = target.join(executable("kyuubiki-installer"));
    stage_payload(
        root, managed, &installer, &target, &release, &payload, version,
    )?;

    let runtime = managed
        .join("home/Library/Application Support/kyuubiki/runtime/versions")
        .join(version);
    let detached_source = managed.join("source");
    fs::remove_dir_all(&payload)
        .and_then(|()| fs::remove_dir_all(&release))
        .map_err(|error| format!("failed to detach macOS staging sources: {error}"))?;
    super::host::run(vec![
        "--managed-root".into(),
        managed.as_os_str().to_owned(),
        "--runtime-root".into(),
        runtime.as_os_str().to_owned(),
        "--detached-source-root".into(),
        detached_source.as_os_str().to_owned(),
        "--out".into(),
        managed.join("capture").into_os_string(),
        "--package-version".into(),
        version.into(),
        "--platform".into(),
        "macos".into(),
        "--architecture".into(),
        "aarch64".into(),
        "--execution-host-role".into(),
        "local-macos-qualification-host".into(),
    ])?;
    copy_capture(&managed.join("capture"), capture)
}

fn build_rust_binaries(root: &Path) -> RunnerResult<PathBuf> {
    let rust = root.join("workers/rust");
    let target = cargo_target_dir(&rust);
    let mut command = Command::new("cargo");
    command.current_dir(&rust).args([
        "+1.88.0",
        "build",
        "--release",
        "-p",
        "kyuubiki-installer",
        "-p",
        "kyuubiki-script-runner",
        "-p",
        "kyuubiki-cli",
        "-p",
        "kyuubiki-desktop-runtime",
        "--bin",
        "kyuubiki-installer",
        "--bin",
        "kyuubiki-cli",
        "--bin",
        "kyuubiki-headless",
        "--bin",
        "kyuubiki-runtime",
    ]);
    super::support::run_success("build macOS qualification binaries", command)?;
    Ok(target.join("release"))
}

fn build_orchestra_release(root: &Path, release: &Path, version: &str) -> RunnerResult<()> {
    let web = root.join("apps/web");
    for (label, args) in [
        ("resolve Orchestra dependencies", vec!["deps.get"]),
        (
            "compile production Orchestra",
            vec!["compile", "--warnings-as-errors"],
        ),
    ] {
        let mut command = Command::new("mix");
        command.current_dir(&web).env("MIX_ENV", "prod").args(args);
        super::support::run_success(label, command)?;
    }
    let mut command = Command::new("mix");
    command
        .current_dir(&web)
        .env("MIX_ENV", "prod")
        .env("KYUUBIKI_RELEASE_VERSION", version)
        .args(["release", "kyuubiki_web", "--overwrite", "--path"])
        .arg(release);
    super::support::run_success("build production Orchestra release", command)?;
    Ok(())
}

fn stage_payload(
    root: &Path,
    managed: &Path,
    installer: &Path,
    target: &Path,
    release: &Path,
    payload: &Path,
    version: &str,
) -> RunnerResult<()> {
    let mut stage = Command::new(installer);
    stage
        .env("KYUUBIKI_REPO_ROOT", root)
        .args(["stage-release", "macos"])
        .arg(payload);
    super::support::run_success("stage macOS Runtime payload", stage)?;
    for name in ["kyuubiki-cli", "kyuubiki-runtime", "kyuubiki-headless"] {
        copy_executable(
            &target.join(executable(name)),
            &payload.join("bin").join(executable(name)),
        )?;
    }
    let orchestrator = payload.join("services/orchestrator");
    if orchestrator.exists() {
        fs::remove_dir_all(&orchestrator)
            .map_err(|error| format!("failed to reset staged Orchestra: {error}"))?;
    }
    copy_tree(release, &orchestrator)?;
    let frontend = payload.join("services/frontend");
    fs::create_dir_all(&frontend)
        .and_then(|()| {
            fs::write(
                frontend.join("index.html"),
                "<!doctype html><title>Kyuubiki native frontend qualification</title>\n",
            )
        })
        .map_err(|error| format!("failed to stage frontend sentinel: {error}"))?;

    let mut seal = Command::new(installer);
    seal.env("KYUUBIKI_REPO_ROOT", root)
        .arg("seal-runtime-payload")
        .arg(payload)
        .args([version, "macos"]);
    super::support::run_success("seal macOS Runtime payload", seal)?;

    let mut install = Command::new(installer);
    install
        .env_clear()
        .env("HOME", managed.join("home"))
        .env("LANG", "C.UTF-8")
        .env("PATH", "/usr/bin:/bin")
        .arg("install-runtime-payload")
        .arg(payload);
    super::support::run_success("install and activate macOS Runtime payload", install)?;
    Ok(())
}

fn copy_capture(source: &Path, target: &Path) -> RunnerResult<()> {
    for name in CAPTURE_FILES {
        fs::copy(source.join(name), target.join(name))
            .map_err(|error| format!("failed to retain macOS capture {name}: {error}"))?;
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> RunnerResult<()> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry
            .file_type()
            .map_err(|error| format!("failed to inspect release entry: {error}"))?;
        let destination = target.join(entry.file_name());
        if metadata.is_symlink() {
            return Err(format!(
                "production Orchestra release contains a symlink: {}",
                entry.path().display()
            ));
        }
        if metadata.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else if metadata.is_file() {
            fs::copy(entry.path(), &destination)
                .map_err(|error| format!("failed to copy Orchestra release file: {error}"))?;
        } else {
            return Err("production Orchestra release contains a special file".to_string());
        }
    }
    Ok(())
}

fn copy_executable(source: &Path, target: &Path) -> RunnerResult<()> {
    fs::copy(source, target).map_err(|error| {
        format!(
            "failed to copy qualification binary {}: {error}",
            source.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("failed to mark {} executable: {error}", target.display()))?;
    }
    Ok(())
}

fn cargo_target_dir(rust: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                rust.join(path)
            }
        })
        .unwrap_or_else(|| rust.join("target"))
}

fn cleanup_roots(managed: &Path) -> RunnerResult<()> {
    cleanup_root(managed, "managed root")
}

fn cleanup_root(path: &Path, label: &str) -> RunnerResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clean macOS qualification {label}: {error}"))?;
    }
    if path.exists() {
        return Err(format!("macOS qualification {label} remains after cleanup"));
    }
    Ok(())
}

fn with_cleanup(error: String, cleanup: RunnerResult<()>) -> String {
    cleanup
        .err()
        .map_or(error.clone(), |cleanup| format!("{error}; {cleanup}"))
}

fn merge_cleanup(result: RunnerResult<()>, cleanup: RunnerResult<()>) -> RunnerResult<()> {
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
    }
}

fn executable(name: &str) -> &str {
    name
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-installed-runtime-operational-macos [--out <repo-relative-report>]"
    );
}
