use super::RunnerResult;
use crate::qualification_support::repo_path;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

pub(super) fn source_tree_digest(
    root: &Path,
    source_files: &[String],
    source_roots: &[String],
) -> RunnerResult<String> {
    let mut files = BTreeSet::new();
    for relative in source_files {
        collect_file(root, &repo_path(root, relative)?, &mut files)?;
    }
    for relative in source_roots {
        let source_root = repo_path(root, relative)?;
        let metadata = fs::symlink_metadata(&source_root)
            .map_err(|error| format!("failed to inspect source root {relative}: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "benchmark qualification source root is not a real directory: {relative}"
            ));
        }
        collect_directory(root, &source_root, &mut files)?;
    }
    if files.is_empty() {
        return Err("benchmark qualification source tree is empty".into());
    }

    let mut hasher = Sha256::new();
    for relative in files {
        hasher.update(b"file\0");
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(
            fs::read(repo_path(root, &relative)?)
                .map_err(|error| format!("failed to hash {relative}: {error}"))?,
        );
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_directory(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<String>,
) -> RunnerResult<()> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "failed to read source root {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read source entry under {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "benchmark qualification source tree contains a symlink: {}",
                portable_relative(root, &path)?
            ));
        }
        if file_type.is_dir() {
            collect_directory(root, &path, files)?;
        } else if file_type.is_file() {
            collect_file(root, &path, files)?;
        } else {
            return Err(format!(
                "benchmark qualification source tree contains an unsupported entry: {}",
                portable_relative(root, &path)?
            ));
        }
    }
    Ok(())
}

fn collect_file(root: &Path, path: &Path, files: &mut BTreeSet<String>) -> RunnerResult<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect source file {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "benchmark qualification source is not a real file: {}",
            portable_relative(root, path)?
        ));
    }
    files.insert(portable_relative(root, path)?);
    Ok(())
}

fn portable_relative(root: &Path, path: &Path) -> RunnerResult<String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("source path escapes repository: {}", path.display()))?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| format!("source path is not UTF-8: {}", path.display()))?,
            ),
            Component::CurDir => {}
            _ => {
                return Err(format!(
                    "source path escapes repository: {}",
                    path.display()
                ));
            }
        }
    }
    if parts.is_empty() {
        return Err("source path cannot be the repository root".into());
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::source_tree_digest;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn recursively_hashes_source_roots_in_stable_order() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-benchmark-source-tree-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src/nested")).expect("fixture tree should be created");
        fs::write(root.join("Cargo.toml"), "[package]\n").expect("manifest should be written");
        fs::write(root.join("src/lib.rs"), "mod nested;\n").expect("source should be written");
        fs::write(root.join("src/nested/mod.rs"), "pub fn value() {}\n")
            .expect("nested source should be written");

        let first = source_tree_digest(&root, &["Cargo.toml".into()], &["src".into()])
            .expect("source tree should hash");
        let reordered = source_tree_digest(
            &root,
            &["src/lib.rs".into(), "Cargo.toml".into()],
            &["src/nested".into(), "src".into()],
        )
        .expect("overlapping roots should deduplicate");
        assert_eq!(first, reordered);

        fs::write(
            root.join("src/nested/mod.rs"),
            "pub fn value() { panic!() }\n",
        )
        .expect("nested source should change");
        let changed = source_tree_digest(&root, &["Cargo.toml".into()], &["src".into()])
            .expect("changed source tree should hash");
        assert_ne!(first, changed);
        fs::remove_dir_all(root).expect("fixture tree should be removed");
    }
}
