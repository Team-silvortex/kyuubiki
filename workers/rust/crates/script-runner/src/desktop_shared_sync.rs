use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use self::asset_io::{SyncStats, copy_changed, mirror_tree, prune_entries, write_changed};
use crate::RunnerResult;

#[path = "desktop_asset_io.rs"]
mod asset_io;

#[cfg(test)]
#[path = "desktop_build_tests.rs"]
mod tests;

const DESKTOP_APPS: [(&str, &str); 3] = [
    ("hub-gui", "hub"),
    ("installer-gui", "hub"),
    ("workbench-gui", "workbench"),
];
const SHARED_UI_FILES: [&str; 7] = [
    "desktop-shell.css",
    "desktop-shell-runtime-mesh.css",
    "language-pack-loader.js",
    "platform.js",
    "runtime-status-model.js",
    "runtime-status-summary.js",
    "tauri-bridge.js",
];
const INSTALLER_PRIMARY_BUTTON_CSS: &str = "\n.desktop-shell-button-primary {\n  background: linear-gradient(180deg, rgba(255, 174, 72, 0.28), rgba(79, 84, 93, 0.96));\n  border-color: rgba(255, 174, 72, 0.34);\n}\n";

pub(crate) fn run_sync_desktop_shared(root: &Path) -> RunnerResult<u8> {
    let stats = with_generated_ui(root, |generated| {
        let mut stats = SyncStats::default();
        let canonical = root.join("apps/desktop-shared/ui");
        for file in SHARED_UI_FILES.iter().filter(|file| file.ends_with(".js")) {
            copy_changed(&generated.join(file), &canonical.join(file), &mut stats)?;
        }
        remove_if_exists(&canonical.join("runtime-status-types.js"))?;
        sync_shared_assets(root, &mut stats)?;
        Ok(stats)
    })?;
    println!(
        "synced desktop shared assets to {} ({} checked, {} written, {} stale entries removed)",
        DESKTOP_APPS
            .iter()
            .map(|(app, surface)| format!("{app}:{surface}"))
            .collect::<Vec<_>>()
            .join(", "),
        stats.checked,
        stats.written,
        stats.removed
    );
    Ok(0)
}

pub(crate) fn run_check_desktop_shared(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if !args.is_empty() {
        return Err("check-desktop-shared does not accept arguments".to_string());
    }

    with_generated_ui(root, |generated| verify_shared_assets(root, generated))?;
    println!("desktop shared asset synchronization check passed");
    Ok(0)
}

fn with_generated_ui<T>(
    root: &Path,
    action: impl FnOnce(&Path) -> RunnerResult<T>,
) -> RunnerResult<T> {
    let temporary_ui_dir = root.join("tmp").join(format!(
        "desktop-shared-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    asset_io::ensure_directory(&root.join("tmp"))?;
    fs::create_dir(&temporary_ui_dir)
        .map_err(|error| format!("create {}: {error}", temporary_ui_dir.display()))?;
    let result = (|| {
        compile_desktop_shared_typescript(root, &temporary_ui_dir)?;
        action(&temporary_ui_dir)
    })();
    let cleanup = remove_dir_if_exists(&temporary_ui_dir);
    let value = result?;
    cleanup?;
    Ok(value)
}

fn compile_desktop_shared_typescript(root: &Path, out_dir: &Path) -> RunnerResult<()> {
    let desktop_shared_dir = root.join("apps/desktop-shared");
    let tsc = tsc_bin(root);
    let args = vec![
        OsString::from("-p"),
        desktop_shared_dir.join("tsconfig.json").into_os_string(),
        OsString::from("--outDir"),
        out_dir.into(),
    ];
    let status = crate::run_command(&desktop_shared_dir, tsc.to_string_lossy().as_ref(), args)?;
    if status != 0 {
        return Err(format!(
            "desktop shared TypeScript compile failed with status {status}"
        ));
    }
    remove_if_exists(&out_dir.join("runtime-status-types.js"))?;
    Ok(())
}

fn verify_shared_assets(root: &Path, generated_ui_dir: &Path) -> RunnerResult<()> {
    let shared_ui_dir = root.join("apps/desktop-shared/ui");
    for file in SHARED_UI_FILES
        .iter()
        .copied()
        .filter(|file| file.ends_with(".js"))
    {
        assert_same_file(
            &generated_ui_dir.join(file),
            &shared_ui_dir.join(file),
            "generated desktop shared asset",
        )?;
    }

    let brand_source = root.join("assets/brand/brand.json");
    for (app, surface) in DESKTOP_APPS {
        verify_app_shared_assets(root, app, surface, &shared_ui_dir, &brand_source)?;
    }
    Ok(())
}

fn verify_app_shared_assets(
    root: &Path,
    app: &str,
    language_surface: &str,
    shared_ui_dir: &Path,
    brand_source: &Path,
) -> RunnerResult<()> {
    let app_ui_dir = root.join("apps").join(app).join("ui");
    let shared_target_dir = app_ui_dir.join("shared");
    assert_directory_entries(&shared_target_dir, &SHARED_UI_FILES)?;
    for file in SHARED_UI_FILES {
        let source = shared_ui_dir.join(file);
        let target = shared_target_dir.join(file);
        if app == "installer-gui" && file == "desktop-shell.css" {
            let mut expected = fs::read(&source)
                .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
            expected.extend_from_slice(INSTALLER_PRIMARY_BUTTON_CSS.as_bytes());
            assert_file_bytes(&target, &expected, "installer shared stylesheet")?;
        } else {
            assert_same_file(&source, &target, "desktop shared mirror")?;
        }
    }
    assert_same_file(
        brand_source,
        &app_ui_dir.join("assets/brand.json"),
        "desktop brand mirror",
    )?;
    verify_language_pack_mirror(root, app, language_surface)
}

fn verify_language_pack_mirror(root: &Path, app: &str, surface: &str) -> RunnerResult<()> {
    let source_root = root.join("language-packs");
    let target_root = root.join("apps").join(app).join("ui/language-packs");
    assert_directory_entries(&target_root, &["catalog.json", surface])?;
    assert_same_file(
        &source_root.join("catalog.json"),
        &target_root.join("catalog.json"),
        "language pack catalog mirror",
    )?;
    assert_same_tree(
        &source_root.join(surface),
        &target_root.join(surface),
        "language pack surface mirror",
    )
}

fn sync_shared_assets(root: &Path, stats: &mut SyncStats) -> RunnerResult<()> {
    let brand_source = root.join("assets/brand/brand.json");
    let shared_ui_dir = root.join("apps/desktop-shared/ui");
    for (app, language_surface) in DESKTOP_APPS {
        let shared_target_dir = root.join("apps").join(app).join("ui/shared");
        for file in SHARED_UI_FILES {
            if app == "installer-gui" && file == "desktop-shell.css" {
                let mut bytes = fs::read(shared_ui_dir.join(file))
                    .map_err(|error| format!("read shared stylesheet: {error}"))?;
                bytes.extend_from_slice(INSTALLER_PRIMARY_BUTTON_CSS.as_bytes());
                write_changed(&shared_target_dir.join(file), &bytes, stats)?;
            } else {
                copy_changed(
                    &shared_ui_dir.join(file),
                    &shared_target_dir.join(file),
                    stats,
                )?;
            }
        }
        prune_entries(
            &shared_target_dir,
            &SHARED_UI_FILES.map(OsString::from),
            stats,
        )?;
        copy_changed(
            &brand_source,
            &root.join("apps").join(app).join("ui/assets/brand.json"),
            stats,
        )?;
        sync_language_packs(root, app, language_surface, stats)?;
    }
    Ok(())
}

fn sync_language_packs(
    root: &Path,
    app: &str,
    surface: &str,
    stats: &mut SyncStats,
) -> RunnerResult<()> {
    let source_root = root.join("language-packs");
    let target_root = root.join("apps").join(app).join("ui/language-packs");
    copy_changed(
        &source_root.join("catalog.json"),
        &target_root.join("catalog.json"),
        stats,
    )?;
    mirror_tree(
        &source_root.join(surface),
        &target_root.join(surface),
        stats,
    )?;
    prune_entries(
        &target_root,
        &["catalog.json".into(), surface.into()],
        stats,
    )
}

fn tsc_bin(root: &Path) -> PathBuf {
    let binary = if cfg!(windows) { "tsc.cmd" } else { "tsc" };
    root.join("apps/frontend/node_modules/.bin").join(binary)
}

fn assert_same_file(source: &Path, target: &Path, label: &str) -> RunnerResult<()> {
    let expected = fs::read(source)
        .map_err(|error| format!("failed to read {label} {}: {error}", source.display()))?;
    assert_file_bytes(target, &expected, label)
}

fn assert_file_bytes(target: &Path, expected: &[u8], label: &str) -> RunnerResult<()> {
    let actual = fs::read(target)
        .map_err(|error| format!("failed to read {label} {}: {error}", target.display()))?;
    if actual != expected {
        return Err(format!("{label} differs: {}", target.display()));
    }
    Ok(())
}

fn assert_directory_entries(target: &Path, expected: &[&str]) -> RunnerResult<()> {
    let mut actual = fs::read_dir(target)
        .map_err(|error| format!("failed to read {}: {error}", target.display()))?
        .map(|entry| {
            entry
                .map(|value| value.file_name().to_string_lossy().into_owned())
                .map_err(|error| format!("failed to read {} entry: {error}", target.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected = expected
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(format!(
            "directory entries differ for {}: expected {:?}, got {:?}",
            target.display(),
            expected,
            actual
        ));
    }
    Ok(())
}

fn assert_same_tree(source: &Path, target: &Path, label: &str) -> RunnerResult<()> {
    let source_files = files_in_tree(source)?;
    let target_files = files_in_tree(target)?;
    if source_files != target_files {
        return Err(format!("{label} file set differs: {}", target.display()));
    }
    for relative in source_files {
        assert_same_file(&source.join(&relative), &target.join(&relative), label)?;
    }
    Ok(())
}

fn files_in_tree(root: &Path) -> RunnerResult<Vec<PathBuf>> {
    fn collect(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> RunnerResult<()> {
        for entry in fs::read_dir(current)
            .map_err(|error| format!("failed to read {}: {error}", current.display()))?
        {
            let entry = entry
                .map_err(|error| format!("failed to read {} entry: {error}", current.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
            if file_type.is_dir() {
                collect(root, &path, files)?;
            } else if file_type.is_file() {
                files.push(
                    path.strip_prefix(root)
                        .map_err(|error| {
                            format!("failed to relativize {}: {error}", path.display())
                        })?
                        .to_path_buf(),
                );
            } else {
                return Err(format!("unsupported mirror entry: {}", path.display()));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn remove_if_exists(target: &Path) -> RunnerResult<()> {
    match fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", target.display())),
    }
}

fn remove_dir_if_exists(target: &Path) -> RunnerResult<()> {
    match fs::remove_dir_all(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", target.display())),
    }
}
