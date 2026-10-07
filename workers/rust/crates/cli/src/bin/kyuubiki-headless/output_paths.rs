use super::Flags;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub(super) const PATH_CONFLICT: &str = "headless output path conflict:";

#[derive(Clone, Copy)]
pub(super) enum OutputScope {
    BatchInput,
    Transform,
    Run,
}

pub(super) fn validate_paths(flags: &Flags, scope: OutputScope) -> Result<(), String> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    if let Some(input) = flags.positional.first() {
        inputs.push(("workflow input", input.as_str()));
    }
    add_path(&mut inputs, "--parameter-patch", &flags.parameter_patch);
    add_path(
        &mut outputs,
        "--parameter-patch-receipt-out",
        &flags.parameter_patch_receipt_out,
    );
    match scope {
        OutputScope::BatchInput => {}
        OutputScope::Transform => add_path(&mut outputs, "--out", &flags.out),
        OutputScope::Run => {
            add_path(
                &mut inputs,
                "--research-round-spec",
                &flags.research_round_spec,
            );
            add_path(
                &mut inputs,
                "--previous-round-evidence",
                &flags.previous_round_evidence,
            );
            add_path(&mut outputs, "--report-out", &flags.report_out);
            add_path(
                &mut outputs,
                "--material-report-out",
                &flags.material_report_out,
            );
            add_path(
                &mut outputs,
                "--research-round-out",
                &flags.research_round_out,
            );
        }
    }
    check_artifact_paths(&inputs, &outputs)
}

pub(super) fn write_guarded_json_file<T: Serialize>(
    flags: &Flags,
    scope: OutputScope,
    path: &str,
    value: &T,
) -> Result<PathBuf, String> {
    super::kyuubiki_headless_report_io::write_json_file_checked(path, value, || {
        validate_paths(flags, scope)
    })
}

fn add_path<'a>(
    paths: &mut Vec<(&'static str, &'a str)>,
    role: &'static str,
    value: &'a Option<String>,
) {
    if let Some(path) = value {
        paths.push((role, path));
    }
}

fn check_artifact_paths(inputs: &[(&str, &str)], outputs: &[(&str, &str)]) -> Result<(), String> {
    if outputs.is_empty() {
        return Ok(());
    }
    let inputs = resolve_paths(inputs)?;
    let outputs = resolve_paths(outputs)?;
    for (index, output) in outputs.iter().enumerate() {
        let collision = inputs
            .iter()
            .find(|other| output.identity.aliases(&other.identity))
            .or_else(|| {
                outputs[..index]
                    .iter()
                    .find(|other| output.identity.conflicts_with_output(&other.identity))
            });
        if let Some(other) = collision {
            return Err(format!(
                "{PATH_CONFLICT} {} ({}) conflicts with {} ({}); select distinct output files separate from inputs and output-file ancestors",
                output.role, output.original, other.role, other.original
            ));
        }
    }
    Ok(())
}

struct ArtifactPath<'a> {
    role: &'a str,
    original: &'a str,
    identity: PathIdentity,
}

fn resolve_paths<'a>(paths: &[(&'a str, &'a str)]) -> Result<Vec<ArtifactPath<'a>>, String> {
    paths
        .iter()
        .map(|(role, path)| {
            let identity = PathIdentity::resolve(Path::new(path)).map_err(|error| {
                format!("{PATH_CONFLICT} cannot resolve {role} ({path}): {error}")
            })?;
            Ok(ArtifactPath {
                role,
                original: path,
                identity,
            })
        })
        .collect()
}

struct PathIdentity {
    physical: PathBuf,
    #[cfg(unix)]
    file_id: Option<(u64, u64)>,
}

impl PathIdentity {
    fn resolve(path: &Path) -> io::Result<Self> {
        let physical = resolve_physical_path(path)?;
        #[cfg(unix)]
        let file_id = {
            use std::os::unix::fs::MetadataExt;
            match fs::metadata(&physical) {
                Ok(metadata) => Some((metadata.dev(), metadata.ino())),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            }
        };
        Ok(Self {
            physical,
            #[cfg(unix)]
            file_id,
        })
    }

    fn aliases(&self, other: &Self) -> bool {
        if self.physical == other.physical {
            return true;
        }
        #[cfg(unix)]
        if self.file_id.is_some() && self.file_id == other.file_id {
            return true;
        }
        // Conservatively reserve ASCII case-only variants on the common case-insensitive hosts.
        #[cfg(any(windows, target_os = "macos"))]
        if let (Some(left), Some(right)) = (self.physical.to_str(), other.physical.to_str()) {
            return left.eq_ignore_ascii_case(right);
        }
        false
    }

    fn conflicts_with_output(&self, other: &Self) -> bool {
        if self.aliases(other)
            || self.physical.starts_with(&other.physical)
            || other.physical.starts_with(&self.physical)
        {
            return true;
        }
        #[cfg(any(windows, target_os = "macos"))]
        return ascii_component_prefix(&self.physical, &other.physical)
            || ascii_component_prefix(&other.physical, &self.physical);
        #[cfg(not(any(windows, target_os = "macos")))]
        false
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn ascii_component_prefix(prefix: &Path, path: &Path) -> bool {
    let mut components = path.components();
    prefix.components().all(|left| {
        components.next().is_some_and(|right| {
            left == right
                || match (left.as_os_str().to_str(), right.as_os_str().to_str()) {
                    (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
                    _ => false,
                }
        })
    })
}

fn resolve_physical_path(path: &Path) -> io::Result<PathBuf> {
    let mut ancestor = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut suffix = Vec::<OsString>::new();
    loop {
        match fs::canonicalize(&ancestor) {
            Ok(mut physical) => {
                // Resolve existing symlinks before normalizing the still-missing suffix.
                for part in suffix.iter().rev() {
                    if part == ".." {
                        physical.pop();
                    } else {
                        physical.push(part);
                    }
                }
                return Ok(physical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if fs::symlink_metadata(&ancestor)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "unresolved symbolic link",
                    ));
                }
                match ancestor.components().next_back() {
                    Some(Component::Normal(name)) => suffix.push(name.to_owned()),
                    Some(Component::ParentDir) => suffix.push("..".into()),
                    _ => return Err(error),
                }
                if !ancestor.pop() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
#[path = "output_paths_tests.rs"]
mod tests;
