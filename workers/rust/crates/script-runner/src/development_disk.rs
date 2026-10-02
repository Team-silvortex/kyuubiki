use fs2::FileExt;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

const CARGO_ROOTS: &[&str] = &[
    "workers/rust/target",
    "sdks/rust/target",
    "target/desktop-cache/macos",
    "target/desktop-cache/linux",
    "target/desktop-cache/windows",
    "apps/hub-gui/src-tauri/target",
    "apps/installer-gui/src-tauri/target",
    "apps/workbench-gui/src-tauri/target",
];
const FRONTEND_CACHE: &str = "apps/frontend/.next/cache";

#[derive(Default)]
struct Options {
    apply: bool,
    test_binaries: bool,
}

#[derive(Debug)]
struct Candidate {
    path: PathBuf,
    directory: bool,
    bytes: u64,
}

pub(crate) fn run(root: &Path, args: Vec<OsString>) -> Result<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: dev-disk [--apply] [--test-binaries]\n\
            Preview regenerable incremental, Rust object and frontend build caches.\n\
            --apply removes the previewed caches; --test-binaries also removes hashed Cargo executables.\n\
            Dependency libraries, top-level tools, exports and installed apps are retained."
        );
        return Ok(0);
    }
    let options = parse_options(&args)?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    // Hold Cargo's own output locks throughout both inspection and deletion.
    let _locks = if options.apply {
        lock_builds(&root)?
    } else {
        Vec::new()
    };
    let candidates = plan(&root, options.test_binaries)?;
    if options.apply {
        ensure_no_tracked_outputs(&root)?;
    }
    let total: u64 = candidates.iter().map(|item| item.bytes).sum();
    let count = candidates.len();
    if options.apply {
        apply(&root, &candidates)?;
    }
    println!(
        "Development cache {}: {count} entries, {:.2} GiB logical bytes (hard links may overlap).",
        if options.apply { "removed" } else { "preview" },
        total as f64 / 1_073_741_824.0
    );
    println!(
        "Scope: debug incremental caches, *.rcgu.o, {FRONTEND_CACHE}{}.",
        if options.test_binaries {
            ", hashed Cargo executables"
        } else {
            ""
        }
    );
    Ok(0)
}

fn parse_options(args: &[OsString]) -> Result<Options> {
    let mut options = Options::default();
    for arg in args {
        match arg.to_str() {
            Some("--apply") => options.apply = true,
            Some("--test-binaries") => options.test_binaries = true,
            _ => {
                return Err(format!(
                    "unsupported dev-disk option: {}",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(options)
}

fn lock_builds(root: &Path) -> Result<Vec<File>> {
    let mut locks = Vec::new();
    for cargo_root in CARGO_ROOTS {
        let profile = root.join(cargo_root).join("debug");
        if !checked_exists(root, &profile)? {
            continue;
        }
        let path = profile.join(".cargo-lock");
        checked_exists(root, &path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("cannot open build lock: {e}"))?;
        file.try_lock_exclusive().map_err(|e| {
            format!("build cache is in use at {cargo_root}; retry after Cargo finishes: {e}")
        })?;
        locks.push(file);
    }
    Ok(locks)
}

fn plan(root: &Path, binaries: bool) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    for cargo_root in CARGO_ROOTS {
        let profile = root.join(cargo_root).join("debug");
        if !checked_exists(root, &profile)? {
            continue;
        }
        let incremental = profile.join("incremental");
        add_directory(root, incremental, &mut candidates)?;
        let deps = profile.join("deps");
        if !checked_exists(root, &deps)? {
            continue;
        }
        for entry in fs::read_dir(&deps).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let object = name.ends_with(".rcgu.o");
            let binary = binaries && is_cargo_binary(name, &entry.path())?;
            if object || binary {
                candidates.push(Candidate {
                    path: entry.path(),
                    directory: false,
                    bytes: entry.metadata().map_err(|e| e.to_string())?.len(),
                });
            }
        }
    }
    add_directory(root, root.join(FRONTEND_CACHE), &mut candidates)?;
    Ok(candidates)
}

fn is_cargo_binary(name: &str, path: &Path) -> Result<bool> {
    let base = name.strip_suffix(".exe").unwrap_or(name);
    let Some((_, hash)) = base.rsplit_once('-') else {
        return Ok(false);
    };
    if hash.len() != 16 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Ok(false);
    }
    // Cargo emits a dep-info companion for linked binaries. Libraries and metadata
    // have their own extensions and must survive this cleanup.
    let companion = path.with_file_name(format!("{base}.d"));
    match fs::symlink_metadata(companion) {
        Ok(meta) => Ok(meta.is_file()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

fn add_directory(root: &Path, path: PathBuf, candidates: &mut Vec<Candidate>) -> Result<()> {
    if checked_exists(root, &path)? {
        if !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err(format!(
                "cache directory is not a directory: {}",
                path.display()
            ));
        }
        candidates.push(Candidate {
            bytes: tree_bytes(&path)?,
            path,
            directory: true,
        });
    }
    Ok(())
}

fn checked_exists(root: &Path, path: &Path) -> Result<bool> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "cache path escaped repository")?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "cache path contains a symlink: {}",
                    current.display()
                ));
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(true)
}

fn tree_bytes(path: &Path) -> Result<u64> {
    let mut bytes = 0;
    let mut pending = vec![path.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if meta.is_dir() {
                pending.push(entry.path());
            } else if meta.is_file() {
                bytes += meta.len();
            }
        }
    }
    Ok(bytes)
}

fn ensure_no_tracked_outputs(root: &Path) -> Result<()> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["ls-files", "--cached", "-z", "--"]);
    for cargo_root in CARGO_ROOTS {
        command.arg(format!("{cargo_root}/debug"));
    }
    command.arg(FRONTEND_CACHE);
    let output = command
        .output()
        .map_err(|e| format!("cannot check tracked output protection: {e}"))?;
    if !output.status.success() {
        return Err("cannot check tracked output protection in this repository".into());
    }
    if !output.stdout.is_empty() {
        return Err("tracked files exist in a managed cache; cleanup refused".into());
    }
    Ok(())
}

fn apply(root: &Path, candidates: &[Candidate]) -> Result<()> {
    for item in candidates {
        if !checked_exists(root, &item.path)? {
            continue;
        }
        let result = if item.directory {
            fs::remove_dir_all(&item.path)
        } else {
            fs::remove_file(&item.path)
        };
        result.map_err(|e| format!("failed to remove cache {}: {e}", item.path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "development_disk_tests.rs"]
mod tests;
