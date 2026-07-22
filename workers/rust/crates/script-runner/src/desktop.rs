use std::ffi::OsString;
use std::path::Path;

use crate::{RepoPaths, RunnerResult, run_installer, run_with_env};

#[derive(Clone, Copy)]
pub enum DesktopApp {
    Hub,
    Installer,
    Workbench,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
    Windows,
}

enum DesktopTarget {
    All,
    Platform(Platform),
}

pub fn run_desktop_dev(paths: &RepoPaths, app: DesktopApp) -> RunnerResult<u8> {
    prepare_desktop_assets(paths)?;
    run_desktop_tauri_command(
        paths,
        app,
        desktop_app_dir(paths, app),
        "npm",
        ["run", "tauri:dev"].map(OsString::from),
    )
}

pub fn run_desktop_build(
    paths: &RepoPaths,
    app: DesktopApp,
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    let platform = parse_desktop_platform(rest.first())?;
    let Some(platform) = platform else {
        prepare_desktop_assets(paths)?;
        return run_host_desktop_build_unprepared(paths, app);
    };

    if platform == host_platform() {
        prepare_desktop_assets(paths)?;
        run_host_desktop_build_unprepared(paths, app)
    } else {
        println!(
            "{} native bundle build is host-bound; staging {} release manifests instead.",
            desktop_app_name(app),
            platform.as_str()
        );
        run_installer(
            paths,
            "stage-release",
            vec![OsString::from(platform.as_str())],
        )
    }
}

pub fn run_package_desktop(paths: &RepoPaths, rest: Vec<OsString>) -> RunnerResult<u8> {
    match parse_desktop_target(rest.first())? {
        DesktopTarget::All => {
            for platform in Platform::all() {
                let stage = stage_desktop_platform(paths, platform)?;
                if stage != 0 {
                    return Ok(stage);
                }
            }
            run_desktop_build_host(paths)
        }
        DesktopTarget::Platform(platform) => {
            let stage = stage_desktop_platform(paths, platform)?;
            if stage != 0 {
                return Ok(stage);
            }
            if platform == host_platform() {
                run_desktop_build_host(paths)
            } else {
                println!(
                    "{} desktop release staged; native bundles must be built on a {} host.",
                    platform.as_str(),
                    platform.as_str()
                );
                Ok(0)
            }
        }
    }
}

pub fn run_desktop_stage(paths: &RepoPaths, rest: Vec<OsString>) -> RunnerResult<u8> {
    match parse_desktop_target(rest.first())? {
        DesktopTarget::All => {
            for platform in Platform::all() {
                let status = stage_desktop_platform(paths, platform)?;
                if status != 0 {
                    return Ok(status);
                }
            }
            Ok(0)
        }
        DesktopTarget::Platform(platform) => stage_desktop_platform(paths, platform),
    }
}

pub fn run_desktop_build_host(paths: &RepoPaths) -> RunnerResult<u8> {
    prepare_desktop_assets(paths)?;
    for app in desktop_apps() {
        let status = run_host_desktop_build_unprepared(paths, app)?;
        if status != 0 {
            return Ok(status);
        }
    }
    Ok(0)
}

pub fn run_desktop_release(paths: &RepoPaths, rest: Vec<OsString>) -> RunnerResult<u8> {
    let stage = run_desktop_stage(paths, rest.clone())?;
    if stage != 0 {
        return Ok(stage);
    }
    let build = run_desktop_build_host(paths)?;
    if build != 0 {
        return Ok(build);
    }
    run_desktop_verify(paths, rest)
}

pub fn run_desktop_status(paths: &RepoPaths, rest: Vec<OsString>) -> RunnerResult<u8> {
    match parse_desktop_target(rest.first())? {
        DesktopTarget::All => {
            for platform in Platform::all() {
                print_desktop_status(paths, platform);
            }
        }
        DesktopTarget::Platform(platform) => print_desktop_status(paths, platform),
    }
    Ok(0)
}

pub fn run_desktop_verify(paths: &RepoPaths, rest: Vec<OsString>) -> RunnerResult<u8> {
    match parse_desktop_target(rest.first())? {
        DesktopTarget::All => {
            let status = run_installer(paths, "cross-platform-audit", Vec::new())?;
            if status != 0 {
                return Ok(status);
            }
            for platform in Platform::all() {
                verify_desktop_stage(paths, platform)?;
                verify_desktop_icons(paths, platform)?;
            }
        }
        DesktopTarget::Platform(platform) => {
            verify_desktop_stage(paths, platform)?;
            verify_desktop_icons(paths, platform)?;
        }
    }
    Ok(0)
}

pub fn host_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

impl Platform {
    pub fn all() -> [Self; 3] {
        [Self::Macos, Self::Linux, Self::Windows]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Macos => "macos",
            Platform::Windows => "windows",
        }
    }

    pub fn bundle_subdirs(self) -> &'static [&'static str] {
        match self {
            Self::Macos => &["macos", "dmg"],
            Self::Linux => &["appimage", "deb", "rpm"],
            Self::Windows => &["msi", "nsis"],
        }
    }
}

pub fn desktop_target_cache_dir(root: &Path, platform: Platform) -> std::path::PathBuf {
    root.join("target")
        .join("desktop-cache")
        .join(platform.as_str())
}

fn prepare_desktop_assets(paths: &RepoPaths) -> RunnerResult<()> {
    let status = crate::desktop_shared_sync::run_sync_desktop_shared(&paths.root)?;
    if status != 0 {
        return Err(format!(
            "desktop shared asset preparation failed with status {status}"
        ));
    }
    Ok(())
}

fn run_host_desktop_build_unprepared(paths: &RepoPaths, app: DesktopApp) -> RunnerResult<u8> {
    run_desktop_tauri_command(
        paths,
        app,
        desktop_app_dir(paths, app),
        "npm",
        ["run", "tauri:build"].map(OsString::from),
    )
}

fn run_desktop_tauri_command<I>(
    paths: &RepoPaths,
    app: DesktopApp,
    cwd: &Path,
    program: &str,
    args: I,
) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let target_dir = desktop_target_cache_dir(&paths.root, host_platform());
    let target_dir_text = target_dir
        .to_str()
        .ok_or_else(|| format!("desktop target path is not UTF-8: {}", target_dir.display()))?;

    println!(
        "{} uses shared {} Cargo target cache: {}",
        desktop_app_name(app),
        host_platform().as_str(),
        target_dir.display()
    );
    run_with_env(cwd, program, args, &[("CARGO_TARGET_DIR", target_dir_text)])
}

fn stage_desktop_platform(paths: &RepoPaths, platform: Platform) -> RunnerResult<u8> {
    run_installer(
        paths,
        "stage-release",
        vec![OsString::from(platform.as_str())],
    )
}

fn print_desktop_status(paths: &RepoPaths, platform: Platform) {
    let dist_root = paths.root.join("dist").join(platform.as_str());
    let cache_root = desktop_target_cache_dir(&paths.root, platform);
    println!("platform: {}", platform.as_str());
    println!(
        "  runtime stage: {}",
        if dist_root.exists() {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "  shared Cargo cache: {} ({})",
        cache_root.display(),
        if cache_root.exists() {
            "present"
        } else {
            "missing"
        }
    );
    for app in desktop_apps() {
        let app_name = desktop_app_name(app);
        let manifest = dist_root
            .join("desktop")
            .join(app_name)
            .join("manifest.json");
        println!(
            "  {app_name}: manifest {}, icons {}",
            if manifest.is_file() {
                "present"
            } else {
                "missing"
            },
            desktop_icon_status(paths, app, platform)
        );
    }
}

fn verify_desktop_icons(paths: &RepoPaths, platform: Platform) -> RunnerResult<()> {
    for app in desktop_apps() {
        let icon_dir = desktop_app_dir(paths, app).join("src-tauri/icons");
        for extension in required_icon_extensions(platform) {
            let found = has_icon_extension(&icon_dir, extension)?;
            if !found {
                return Err(format!(
                    "missing .{extension} icon for {} on {}",
                    desktop_app_name(app),
                    platform.as_str()
                ));
            }
        }
    }
    println!("desktop icon verification passed for {}", platform.as_str());
    Ok(())
}

fn verify_desktop_stage(paths: &RepoPaths, platform: Platform) -> RunnerResult<()> {
    let desktop_root = paths
        .root
        .join("dist")
        .join(platform.as_str())
        .join("desktop");
    if !desktop_root.is_dir() {
        return Err(format!(
            "missing desktop release directory for {}: {}",
            platform.as_str(),
            desktop_root.display()
        ));
    }
    for app in desktop_apps() {
        let manifest = desktop_root
            .join(desktop_app_name(app))
            .join("manifest.json");
        if !manifest.is_file() {
            return Err(format!(
                "missing {} manifest for {}: {}",
                desktop_app_name(app),
                platform.as_str(),
                manifest.display()
            ));
        }
    }
    println!(
        "desktop stage verification passed for {}",
        platform.as_str()
    );
    Ok(())
}

fn desktop_icon_status(paths: &RepoPaths, app: DesktopApp, platform: Platform) -> String {
    let icon_dir = desktop_app_dir(paths, app).join("src-tauri/icons");
    let missing: Vec<&str> = required_icon_extensions(platform)
        .iter()
        .copied()
        .filter(|extension| !has_icon_extension(&icon_dir, extension).unwrap_or(false))
        .collect();

    if missing.is_empty() {
        "ready".to_string()
    } else {
        format!("missing .{}", missing.join(" ."))
    }
}

fn has_icon_extension(icon_dir: &Path, extension: &str) -> RunnerResult<bool> {
    Ok(std::fs::read_dir(icon_dir)
        .map_err(|error| format!("failed to scan {}: {error}", icon_dir.display()))?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        }))
}

fn desktop_app_dir(paths: &RepoPaths, app: DesktopApp) -> &Path {
    match app {
        DesktopApp::Hub => &paths.hub_gui,
        DesktopApp::Installer => &paths.installer_gui,
        DesktopApp::Workbench => &paths.workbench_gui,
    }
}

fn desktop_app_name(app: DesktopApp) -> &'static str {
    match app {
        DesktopApp::Hub => "hub-gui",
        DesktopApp::Installer => "installer-gui",
        DesktopApp::Workbench => "workbench-gui",
    }
}

fn desktop_apps() -> [DesktopApp; 3] {
    [
        DesktopApp::Hub,
        DesktopApp::Installer,
        DesktopApp::Workbench,
    ]
}

fn required_icon_extensions(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Macos => &["png", "icns"],
        Platform::Linux => &["png"],
        Platform::Windows => &["png", "ico"],
    }
}

fn parse_desktop_target(value: Option<&OsString>) -> RunnerResult<DesktopTarget> {
    match value.and_then(|value| value.to_str()) {
        Some("all") => Ok(DesktopTarget::All),
        other => parse_desktop_platform_text(other).map(DesktopTarget::Platform),
    }
}

fn parse_desktop_platform(value: Option<&OsString>) -> RunnerResult<Option<Platform>> {
    match value.and_then(|value| value.to_str()) {
        None | Some("") => Ok(None),
        Some(value) => parse_desktop_platform_text(Some(value)).map(Some),
    }
}

fn parse_desktop_platform_text(value: Option<&str>) -> RunnerResult<Platform> {
    match value {
        None | Some("") => Ok(host_platform()),
        Some("macos") => Ok(Platform::Macos),
        Some("linux") => Ok(Platform::Linux),
        Some("windows") => Ok(Platform::Windows),
        Some(other) => Err(format!(
            "unsupported desktop platform `{other}`; expected macos, linux, windows, or all"
        )),
    }
}
