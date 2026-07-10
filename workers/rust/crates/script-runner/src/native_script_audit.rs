use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellScriptKind {
    NativeShim,
    TinyLauncher,
    MigrationTarget,
}

#[derive(Debug, Eq, PartialEq)]
struct ShellScriptRecord {
    relative_path: String,
    kind: ShellScriptKind,
}

pub(crate) fn run_native_script_audit(
    root: &Path,
    host_label: &str,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("native-script-audit only accepts --self-test".to_string());
    }

    let records = collect_shell_script_records(root)?;
    print_report(root, host_label, &records);
    Ok(0)
}

fn collect_shell_script_records(root: &Path) -> RunnerResult<Vec<ShellScriptRecord>> {
    let mut records = Vec::new();
    for entry in ["deploy", "dist", "scripts"] {
        let scan_root = root.join(entry);
        if scan_root.exists() {
            collect_shell_script_records_in(root, &scan_root, &mut records)?;
        }
    }
    records.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(records)
}

fn collect_shell_script_records_in(
    root: &Path,
    dir: &Path,
    records: &mut Vec<ShellScriptRecord>,
) -> RunnerResult<()> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("failed to scan {}: {error}", dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed to read directory entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_shell_script_records_in(root, &path, records)?;
            continue;
        }
        if !is_shell_script(&path) {
            continue;
        }
        let relative = relative_path(root, &path);
        records.push(ShellScriptRecord {
            kind: classify_shell_script(&relative),
            relative_path: relative,
        });
    }
    Ok(())
}

fn is_shell_script(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("sh" | "bash" | "zsh")
    ) || first_line(path)
        .map(|line| {
            line.starts_with("#!")
                && (line.contains("/sh")
                    || line.contains(" sh")
                    || line.contains(" bash")
                    || line.contains("/bash")
                    || line.contains(" zsh")
                    || line.contains("/zsh"))
        })
        .unwrap_or(false)
}

fn first_line(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let first_line = bytes.split(|byte| *byte == b'\n').next()?;
    Some(String::from_utf8_lossy(first_line).to_string())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn classify_shell_script(relative_path: &str) -> ShellScriptKind {
    if is_native_shim(relative_path) {
        ShellScriptKind::NativeShim
    } else if relative_path == "scripts/kyuubiki" {
        ShellScriptKind::TinyLauncher
    } else {
        ShellScriptKind::MigrationTarget
    }
}

fn is_native_shim(relative_path: &str) -> bool {
    matches!(
        relative_path,
        "scripts/run-direct-mesh-benchmark-container.sh"
            | "scripts/run-direct-mesh-benchmark-regression.sh"
            | "scripts/run-benchmark-profile-remote.sh"
            | "scripts/run-standard-benchmark-regression.sh"
            | "scripts/run-workflow-catalog-benchmark-regression.sh"
            | "scripts/run-workflow-mesh-regression-remote.sh"
            | "scripts/run-workflow-mesh-regression.sh"
    )
}

fn print_report(root: &Path, host_label: &str, records: &[ShellScriptRecord]) {
    println!("native script migration audit");
    println!("  host: {host_label}");
    println!("  root: {}", root.display());
    println!("  native runner: kyuubiki-script-runner");
    println!("  remote seam: workers/rust/crates/script-runner/src/remote_host.rs");
    println!("  host tool boundary:");
    for tool in host_tool_boundary() {
        println!("  - {tool}");
    }
    println!("  remaining shell scripts and shims:");
    for record in records {
        println!(
            "  - {} ({})",
            record.relative_path,
            shell_script_kind_label(record.kind)
        );
    }
    println!("  summary:");
    for kind in [
        ShellScriptKind::TinyLauncher,
        ShellScriptKind::NativeShim,
        ShellScriptKind::MigrationTarget,
    ] {
        let count = records.iter().filter(|record| record.kind == kind).count();
        println!("    {}: {count}", shell_script_kind_label(kind));
    }
}

fn host_tool_boundary() -> &'static [&'static str] {
    &[
        "cargo", "docker", "mix", "node", "npm", "python3", "rsync", "scp", "ssh",
    ]
}

fn shell_script_kind_label(kind: ShellScriptKind) -> &'static str {
    match kind {
        ShellScriptKind::NativeShim => "native shim",
        ShellScriptKind::TinyLauncher => "tiny launcher",
        ShellScriptKind::MigrationTarget => "migration target",
    }
}

fn run_self_test() {
    assert_eq!(
        classify_shell_script("scripts/run-workflow-mesh-regression.sh"),
        ShellScriptKind::NativeShim
    );
    assert_eq!(
        classify_shell_script("scripts/kyuubiki"),
        ShellScriptKind::TinyLauncher
    );
    assert_eq!(
        classify_shell_script("deploy/legacy-sync.sh"),
        ShellScriptKind::MigrationTarget
    );
    assert!(host_tool_boundary().contains(&"ssh"));
    assert!(host_tool_boundary().contains(&"docker"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_native_shims_and_migration_targets() {
        assert_eq!(
            classify_shell_script("scripts/run-benchmark-profile-remote.sh"),
            ShellScriptKind::NativeShim
        );
        assert_eq!(
            classify_shell_script("scripts/kyuubiki"),
            ShellScriptKind::TinyLauncher
        );
        assert_eq!(
            classify_shell_script("scripts/custom.sh"),
            ShellScriptKind::MigrationTarget
        );
    }

    #[test]
    fn detects_shell_scripts_by_extension_or_shebang() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/plain-node"), "#!/usr/bin/env node\n").unwrap();
        fs::write(root.join("scripts/plain-shell"), "#!/usr/bin/env sh\n").unwrap();
        fs::write(root.join("scripts/wrapper.zsh"), "#!/usr/bin/env zsh\n").unwrap();

        let records = collect_shell_script_records(&root).unwrap();
        let paths: Vec<_> = records
            .iter()
            .map(|record| record.relative_path.as_str())
            .collect();

        assert_eq!(paths, vec!["scripts/plain-shell", "scripts/wrapper.zsh"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_known_host_tool_boundary() {
        assert_eq!(
            host_tool_boundary(),
            &[
                "cargo", "docker", "mix", "node", "npm", "python3", "rsync", "scp", "ssh",
            ]
        );
    }

    fn unique_temp_dir() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "kyuubiki-native-script-audit-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }
}
