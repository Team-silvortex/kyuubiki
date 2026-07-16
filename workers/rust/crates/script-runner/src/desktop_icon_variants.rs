use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::host_platform;

type RunnerResult<T> = Result<T, String>;

struct IconVariant {
    app_key: &'static str,
    label: &'static str,
    color: &'static str,
    target_app: &'static str,
    sync_runtime_asset: bool,
}

const ICON_VARIANTS: &[IconVariant] = &[
    IconVariant {
        app_key: "hub",
        label: "H",
        color: "#15C4D8",
        target_app: "hub-gui",
        sync_runtime_asset: true,
    },
    IconVariant {
        app_key: "installer",
        label: "I",
        color: "#F5A623",
        target_app: "installer-gui",
        sync_runtime_asset: true,
    },
    IconVariant {
        app_key: "workbench",
        label: "W",
        color: "#FF6B57",
        target_app: "workbench-gui",
        sync_runtime_asset: true,
    },
];

pub(crate) fn run_generate_desktop_icon_variants(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("generate-desktop-icon-variants does not accept arguments".to_string());
    }
    if host_platform().as_str() != "macos" {
        return Err("desktop icon variant generation requires macOS sips/iconutil/AppKit".into());
    }

    let asset_dir = root.join("assets/icons/app");
    let base_png = asset_dir.join("kyuubiki.png");
    let swift_script = root.join("scripts/generate_desktop_icon_badge.swift");
    for variant in ICON_VARIANTS {
        generate_variant(root, &asset_dir, &base_png, &swift_script, variant)?;
    }
    sync_shared_runtime_icons(root, &base_png)?;

    println!("generated desktop icon variants:");
    for variant in ICON_VARIANTS {
        println!(
            "  - {}",
            asset_dir
                .join(format!("kyuubiki-{}.png", variant.app_key))
                .display()
        );
    }
    Ok(0)
}

fn generate_variant(
    root: &Path,
    asset_dir: &Path,
    base_png: &Path,
    swift_script: &Path,
    variant: &IconVariant,
) -> RunnerResult<()> {
    let png_path = asset_dir.join(format!("kyuubiki-{}.png", variant.app_key));
    let icns_path = asset_dir.join(format!("kyuubiki-{}.icns", variant.app_key));
    let ico_path = asset_dir.join(format!("kyuubiki-{}.ico", variant.app_key));
    let target_dir = root.join(format!("apps/{}/src-tauri/icons", variant.target_app));

    run_status(
        root,
        "swift",
        vec![
            swift_script.as_os_str().to_os_string(),
            base_png.as_os_str().to_os_string(),
            png_path.as_os_str().to_os_string(),
            OsString::from(variant.label),
            OsString::from(variant.color),
        ],
    )?;
    generate_iconset(root, &png_path, &icns_path)?;
    generate_ico(root, &png_path, &ico_path, variant.app_key)?;

    std::fs::create_dir_all(&target_dir)
        .map_err(|error| format!("failed to create {}: {error}", target_dir.display()))?;
    std::fs::copy(&png_path, target_dir.join("kyuubiki-app.png"))
        .map_err(|error| format!("failed to copy {}: {error}", png_path.display()))?;
    std::fs::copy(&icns_path, target_dir.join("kyuubiki-app.icns"))
        .map_err(|error| format!("failed to copy {}: {error}", icns_path.display()))?;
    std::fs::copy(&ico_path, target_dir.join("kyuubiki-app.ico"))
        .map_err(|error| format!("failed to copy {}: {error}", ico_path.display()))?;
    if variant.sync_runtime_asset {
        let runtime_asset_dir = root.join(format!("apps/{}/ui/assets", variant.target_app));
        std::fs::create_dir_all(&runtime_asset_dir).map_err(|error| {
            format!("failed to create {}: {error}", runtime_asset_dir.display())
        })?;
        std::fs::copy(&png_path, runtime_asset_dir.join("kyuubiki-app.png"))
            .map_err(|error| format!("failed to copy {}: {error}", png_path.display()))?;
    }
    Ok(())
}

fn sync_shared_runtime_icons(root: &Path, base_png: &Path) -> RunnerResult<()> {
    for target in [
        "apps/frontend/public/kyuubiki.png",
        "apps/frontend/public/icon.png",
        "apps/frontend/public/apple-icon.png",
        "apps/installer-gui/ui/assets/kyuubiki-dock.png",
        "apps/workbench-gui/ui/assets/kyuubiki-dock.png",
    ] {
        let target = root.join(target);
        let parent = target
            .parent()
            .ok_or_else(|| format!("target {} has no parent directory", target.display()))?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        std::fs::copy(base_png, &target)
            .map_err(|error| format!("failed to copy {}: {error}", target.display()))?;
    }
    Ok(())
}

fn generate_iconset(root: &Path, png_path: &Path, icns_path: &Path) -> RunnerResult<()> {
    let iconset_dir = unique_temp_iconset_dir()?;
    for (size, name) in [
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
    ] {
        run_status(
            root,
            "sips",
            vec![
                OsString::from("-z"),
                OsString::from(size.to_string()),
                OsString::from(size.to_string()),
                png_path.as_os_str().to_os_string(),
                OsString::from("--out"),
                iconset_dir.join(name).into_os_string(),
            ],
        )?;
    }
    std::fs::copy(png_path, iconset_dir.join("icon_512x512@2x.png"))
        .map_err(|error| format!("failed to copy iconset source: {error}"))?;
    remove_file_if_exists(icns_path)?;
    run_status(
        root,
        "iconutil",
        vec![
            OsString::from("--convert"),
            OsString::from("icns"),
            OsString::from("--output"),
            icns_path.as_os_str().to_os_string(),
            iconset_dir.as_os_str().to_os_string(),
        ],
    )?;
    std::fs::remove_dir_all(&iconset_dir)
        .map_err(|error| format!("failed to remove {}: {error}", iconset_dir.display()))?;
    Ok(())
}

fn generate_ico(root: &Path, png_path: &Path, ico_path: &Path, app_key: &str) -> RunnerResult<()> {
    let source_png = std::env::temp_dir().join(format!("kyuubiki-{app_key}-ico.png"));
    run_status(
        root,
        "sips",
        vec![
            OsString::from("-z"),
            OsString::from("256"),
            OsString::from("256"),
            png_path.as_os_str().to_os_string(),
            OsString::from("--out"),
            source_png.as_os_str().to_os_string(),
        ],
    )?;
    run_status(
        root,
        "sips",
        vec![
            OsString::from("-s"),
            OsString::from("format"),
            OsString::from("ico"),
            source_png.as_os_str().to_os_string(),
            OsString::from("--out"),
            ico_path.as_os_str().to_os_string(),
        ],
    )?;
    remove_file_if_exists(&source_png)?;
    Ok(())
}

fn run_status<I>(root: &Path, program: &str, args: I) -> RunnerResult<()>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed"))
    }
}

fn unique_temp_iconset_dir() -> RunnerResult<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "kyuubiki-iconset-{}-{nanos}.iconset",
        std::process::id()
    ));
    std::fs::create_dir_all(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    Ok(path)
}

fn remove_file_if_exists(path: &Path) -> RunnerResult<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn print_help() {
    println!(
        "generate-desktop-icon-variants\n\n\
Usage:\n  ./scripts/kyuubiki generate-desktop-icon-variants\n\n\
Generates Hub, Installer, and Workbench desktop icon variants from \
assets/icons/app/kyuubiki.png, then syncs Tauri, runtime UI, dock, and \
frontend public icon copies. Requires macOS swift, sips, and iconutil."
    );
}
