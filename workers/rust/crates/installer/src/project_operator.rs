//! Local project intent, independent of accounts and Agent lifecycle. Never loads libraries.
use crate::operator_package_store::{operator_package_store_root, verify_managed_operator_package};
use fs2::FileExt;
use kyuubiki_operator_sdk::OperatorPackageManifest;
use kyuubiki_protocol::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const SCHEMA: &str = "kyuubiki.project-operators/v1";
const LIMIT: u64 = 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Project {
    schema_version: String,
    project_id: String,
    scope: String,
    packages: Vec<ProjectOperatorPin>,
}

pub fn project_operator(request: ProjectOperatorRequest) -> Result<ProjectOperatorStatus, String> {
    project_operator_in(&operator_package_store_root()?, request)
}

pub fn project_operator_in(
    root: &Path,
    request: ProjectOperatorRequest,
) -> Result<ProjectOperatorStatus, String> {
    if !valid_project_operator_id(&request.project_id)
        || request.scope.len() != 64
        || !request
            .scope
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("project_operator_invalid_project".into());
    }
    let storage = Storage::open(root, &request.project_id, &request.scope)?;
    let mut project = storage.read(&request.project_id, &request.scope)?;
    let mut task = None;
    let mut deployment = None;
    match request.operation {
        ProjectOperatorOperation::List => (),
        ProjectOperatorOperation::ReadDeployment { package, offset } => {
            if !project.packages.contains(&package) {
                return Err("project_operator_not_enabled".into());
            }
            deployment = Some(crate::operator_deployment::export_chunk(
                root, &package, offset,
            )?);
        }
        ProjectOperatorOperation::Enable { package } => {
            verify_pin(root, &package)?;
            if let Some(old) = project
                .packages
                .iter()
                .find(|p| p.package_id == package.package_id)
            {
                if old != &package {
                    return Err("project_operator_disable_before_replacement".into());
                }
            } else {
                if project.packages.len() >= 64 {
                    return Err("project_operator_limit".into());
                }
                if project.packages.iter().any(|p| {
                    p.operators.iter().any(|o| {
                        package
                            .operators
                            .iter()
                            .any(|n| n.operator_id == o.operator_id)
                    })
                }) {
                    return Err("project_operator_id_conflict".into());
                }
                project.packages.push(package);
                storage.write(&project)?;
            }
        }
        ProjectOperatorOperation::Disable { package } => {
            package.validate()?;
            if project
                .packages
                .iter()
                .any(|p| p.package_id == package.package_id && p != &package)
            {
                return Err("project_operator_selection_changed".into());
            }
            project.packages.retain(|p| p != &package);
            storage.write(&project)?;
        }
        ProjectOperatorOperation::Prepare {
            package,
            operator_id,
            task_id,
            input,
            config,
        } => {
            if !project.packages.contains(&package) {
                return Err("project_operator_not_enabled".into());
            }
            verify_pin(root, &package)?;
            task = Some(build_project_operator_task(
                &request.project_id,
                &package,
                &operator_id,
                &task_id,
                input,
                config,
            )?);
        }
    }
    let exporting = deployment.is_some();
    let inventory = if exporting {
        Ok(vec![])
    } else {
        inventory(root)
    };
    let installed_error = inventory.as_ref().err().cloned();
    Ok(ProjectOperatorStatus {
        schema_version: SCHEMA.into(),
        project_id: request.project_id,
        scope: request.scope,
        packages_root: root.join("packages").display().to_string(),
        installed: inventory.unwrap_or_default(),
        installed_error,
        enabled: if exporting {
            vec![]
        } else {
            project
                .packages
                .into_iter()
                .map(|package| {
                    let state = if verify_pin(root, &package).is_ok() {
                        "ready"
                    } else {
                        "unavailable_or_changed"
                    };
                    ProjectOperatorState {
                        package,
                        state: state.into(),
                    }
                })
                .collect()
        },
        task,
        deployment,
    })
}
pub(crate) fn pin_at(root: &Path, id: &str) -> Result<ProjectOperatorPin, String> {
    if !valid_operator_component(id) {
        return Err("project_operator_invalid_pin".into());
    }
    safe(&root.join("packages"), true)?;
    let path = root.join("packages").join(id);
    let manifest_path = path.join("kyuubiki-operator.json");
    safe(&path, true)?;
    safe(&manifest_path, false)?;
    if fs::metadata(&manifest_path)
        .map_err(|_| "project_operator_package_unavailable")?
        .len()
        > LIMIT
    {
        return Err("project_operator_limit".into());
    }
    let receipt = verify_managed_operator_package(&path)
        .map_err(|_| "project_operator_package_unavailable")?;
    let mut bytes = Vec::new();
    File::open(&manifest_path)
        .map_err(|_| "project_operator_package_unavailable")?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "project_operator_package_unavailable")?;
    if bytes.len() as u64 > LIMIT
        || format!("{:x}", Sha256::digest(&bytes)) != receipt.manifest_sha256
    {
        return Err("project_operator_selection_changed".into());
    }
    let manifest: OperatorPackageManifest =
        serde_json::from_slice(&bytes).map_err(|_| "project_operator_package_unavailable")?;
    // Recheck after reading the exported operator table, before accepting it as project metadata.
    if verify_managed_operator_package(&path).map_err(|_| "project_operator_package_unavailable")?
        != receipt
        || receipt.package_id != id
    {
        return Err("project_operator_selection_changed".into());
    }
    let pin = ProjectOperatorPin {
        package_id: receipt.package_id,
        package_version: receipt.package_version,
        manifest_sha256: receipt.manifest_sha256,
        entrypoint_sha256: receipt.entrypoint_sha256,
        operators: manifest
            .operators
            .into_iter()
            .map(|o| ProjectOperatorEntry {
                operator_id: o.operator_id,
                kind: o.kind,
            })
            .collect(),
    };
    pin.validate()?;
    Ok(pin)
}
fn verify_pin(root: &Path, pin: &ProjectOperatorPin) -> Result<(), String> {
    pin.validate()?;
    if &pin_at(root, &pin.package_id)? != pin {
        return Err("project_operator_selection_changed".into());
    }
    Ok(())
}
fn inventory(root: &Path) -> Result<Vec<ProjectOperatorPin>, String> {
    let path = root.join("packages");
    safe(&path, true)?;
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(_) => return Err("project_operator_package_unavailable".into()),
    };
    let mut pins = Vec::new();
    for entry in entries.take(65) {
        if pins.len() == 64 {
            return Err("project_operator_limit".into());
        }
        let entry = entry.map_err(|_| "project_operator_package_unavailable")?;
        pins.push(pin_at(
            root,
            entry
                .file_name()
                .to_str()
                .ok_or("project_operator_invalid_pin")?,
        )?);
    }
    pins.sort_by(|a, b| a.package_id.cmp(&b.package_id));
    Ok(pins)
}
fn safe(path: &Path, dir: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if !m.file_type().is_symlink() && if dir { m.is_dir() } else { m.is_file() } => {
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("project_operator_unsafe_path".into()),
    }
}
fn options() -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o
}
struct Storage {
    #[cfg(unix)]
    root: PathBuf,
    path: PathBuf,
    _lock: File,
}
impl Storage {
    fn open(root: &Path, project: &str, scope: &str) -> Result<Self, String> {
        safe(root, true)?;
        fs::create_dir_all(root).map_err(|_| "project_operator_storage_unavailable")?;
        let root = root
            .canonicalize()
            .map_err(|_| "project_operator_storage_unavailable")?;
        let root = root.join("projects");
        safe(&root, true)?;
        fs::create_dir_all(&root).map_err(|_| "project_operator_storage_unavailable")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| "project_operator_storage_unavailable")?;
        }
        let lock = root.join("project-operators.lock");
        safe(&lock, false)?;
        let lock = options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock)
            .map_err(|_| "project_operator_storage_unavailable")?;
        lock.try_lock_exclusive()
            .map_err(|_| "project_operator_busy")?;
        let path = root.join(format!(
            "{:x}.json",
            Sha256::digest(
                serde_json::to_vec(&(scope, project))
                    .map_err(|_| "project_operator_invalid_project")?
            )
        ));
        Ok(Self {
            #[cfg(unix)]
            root,
            path,
            _lock: lock,
        })
    }
    fn read(&self, id: &str, scope: &str) -> Result<Project, String> {
        safe(&self.path, false)?;
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Project {
                    schema_version: SCHEMA.into(),
                    project_id: id.into(),
                    scope: scope.into(),
                    packages: vec![],
                });
            }
            Err(_) => return Err("project_operator_storage_unavailable".into()),
        };
        let mut bytes = Vec::new();
        file.take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "project_operator_storage_unavailable")?;
        if bytes.len() as u64 > LIMIT {
            return Err("project_operator_limit".into());
        }
        let project: Project =
            serde_json::from_slice(&bytes).map_err(|_| "project_operator_storage_corrupt")?;
        let mut ids = std::collections::HashSet::new();
        if project.schema_version != SCHEMA
            || project.project_id != id
            || project.scope != scope
            || project.packages.len() > 64
            || project
                .packages
                .iter()
                .any(|p| p.validate().is_err() || !ids.insert(&p.package_id))
        {
            return Err("project_operator_storage_corrupt".into());
        }
        Ok(project)
    }
    fn write(&self, project: &Project) -> Result<(), String> {
        let stage = self.path.with_extension("pending");
        safe(&stage, false)?;
        safe(&self.path, false)?;
        if stage.exists() {
            fs::remove_file(&stage).map_err(|_| "project_operator_storage_unavailable")?;
        }
        let bytes = serde_json::to_vec(project).map_err(|_| "project_operator_storage_corrupt")?;
        if bytes.len() as u64 > LIMIT {
            return Err("project_operator_limit".into());
        }
        let result = (|| -> std::io::Result<()> {
            let mut f = options().write(true).create_new(true).open(&stage)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&stage, &self.path)?;
            #[cfg(unix)]
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        result.map_err(|_| "project_operator_storage_unavailable".into())
    }
}
pub fn project_operator_command(args: Vec<String>) -> Result<String, String> {
    let root = match args.as_slice() {
        [] => operator_package_store_root()?,
        [flag, root] if flag == "--store-root" => PathBuf::from(root),
        _ => {
            return Err(
                "usage: project-operator [--store-root <path>] (JSON request on stdin)".into(),
            );
        }
    };
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(131073)
        .read_to_end(&mut bytes)
        .map_err(|_| "project_operator_invalid_request")?;
    if bytes.len() > 131072 {
        return Err("project_operator_limit".into());
    }
    let request = serde_json::from_slice(&bytes).map_err(|_| "project_operator_invalid_request")?;
    serde_json::to_string(&project_operator_in(&root, request)?)
        .map_err(|_| "project_operator_invalid_response".into())
}
