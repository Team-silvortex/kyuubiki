use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use kyuubiki_platform::desktop_preferences_dir;
use serde::Deserialize;
use serde_json::Value;

use crate::workspace_root;

const SOURCE_MODE_ENV: &str = "KYUUBIKI_DESKTOP_SOURCE_MODE";
const RUNTIME_ROOT_ENV: &str = "KYUUBIKI_RUNTIME_ROOT";
const SERVICE_MANIFEST: &str = "manifests/service-launch.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeOrigin {
    Development,
    Installed,
}

pub(crate) struct RuntimePaths {
    pub root: PathBuf,
    pub run: PathBuf,
    pub hot: PathBuf,
    pub origin: RuntimeOrigin,
    services: HashMap<String, ServiceLaunchEntry>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedService {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
struct ServiceLaunchManifest {
    schema_version: String,
    services: Vec<ServiceLaunchEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct ServiceLaunchEntry {
    id: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: String,
}

pub(crate) fn runtime_paths() -> Result<RuntimePaths, String> {
    if let Some(root) = env::var_os(RUNTIME_ROOT_ENV).map(PathBuf::from) {
        return installed_paths(root);
    }
    if source_mode_enabled() {
        return Ok(development_paths(workspace_root()));
    }

    installed_paths(installed_runtime_root()?)
}

pub(crate) fn runtime_bin_dirs(root: &Path) -> Vec<PathBuf> {
    let manifest = root.join("manifests").join("embedded-runtimes.json");
    fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("runtimes").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .flat_map(|runtime| {
            runtime
                .get("bin_dirs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|entry| entry.as_str().map(|path| root.join(path)))
        .filter(|path| path.is_dir())
        .collect()
}

pub(crate) fn resolve_development_command(root: &Path, name: &str) -> Result<PathBuf, String> {
    let candidates = command_names(name);
    let mut dirs = runtime_bin_dirs(root);
    if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".cargo/bin"));
    }
    dirs.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }
    for dir in dirs {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err(format!(
        "development runtime command `{name}` was not found"
    ))
}

impl RuntimePaths {
    pub fn is_development(&self) -> bool {
        self.origin == RuntimeOrigin::Development
    }

    pub fn origin_label(&self) -> &'static str {
        match self.origin {
            RuntimeOrigin::Development => "development-source",
            RuntimeOrigin::Installed => "installer-managed",
        }
    }

    pub fn service(
        &self,
        id: &str,
        replacements: &[(&str, String)],
    ) -> Result<ResolvedService, String> {
        let entry = self
            .services
            .get(id)
            .ok_or_else(|| format!("installed runtime does not declare service `{id}`"))?;
        let replace = |value: &str| {
            replacements
                .iter()
                .fold(value.to_string(), |text, (key, value)| {
                    text.replace(&format!("{{{key}}}"), value)
                })
        };
        let command = checked_relative_path(&self.root, &replace(&entry.command), "command")?;
        if !command.is_file() {
            return Err(format!(
                "installed runtime service `{id}` is missing executable {}; open Kyuubiki Installer and repair the runtime",
                command.display()
            ));
        }
        let cwd = checked_relative_path(&self.root, &replace(&entry.cwd), "cwd")?;
        if !cwd.is_dir() {
            return Err(format!(
                "installed runtime service `{id}` is missing working directory {}; open Kyuubiki Installer and repair the runtime",
                cwd.display()
            ));
        }
        Ok(ResolvedService {
            command,
            args: entry.args.iter().map(|arg| replace(arg)).collect(),
            cwd,
        })
    }
}

fn source_mode_enabled() -> bool {
    env::var(SOURCE_MODE_ENV)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        || cfg!(debug_assertions)
}

fn installed_runtime_root() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir("kyuubiki")?
        .join("runtime")
        .join("current"))
}

fn development_paths(root: PathBuf) -> RuntimePaths {
    let run = root.join("tmp").join("run");
    RuntimePaths {
        hot: run.join("hot"),
        root,
        run,
        origin: RuntimeOrigin::Development,
        services: HashMap::new(),
    }
}

fn installed_paths(root: PathBuf) -> Result<RuntimePaths, String> {
    let manifest_path = root.join(SERVICE_MANIFEST);
    let bytes = fs::read(&manifest_path).map_err(|error| {
        format!(
            "installer-managed runtime is unavailable at {}: {error}; open Kyuubiki Installer and install or repair the runtime",
            root.display()
        )
    })?;
    let manifest: ServiceLaunchManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    if manifest.schema_version != "kyuubiki.service-launch/v1" {
        return Err(format!(
            "unsupported service launch schema `{}` in {}",
            manifest.schema_version,
            manifest_path.display()
        ));
    }
    let mut services = HashMap::new();
    for entry in manifest.services {
        if entry.id.trim().is_empty() || services.insert(entry.id.clone(), entry).is_some() {
            return Err(format!(
                "service launch manifest contains an empty or duplicate service id: {}",
                manifest_path.display()
            ));
        }
    }
    for required in ["agent", "orchestrator", "frontend"] {
        if !services.contains_key(required) {
            return Err(format!(
                "service launch manifest is missing required service `{required}`: {}",
                manifest_path.display()
            ));
        }
    }
    let run = root.join("run");
    Ok(RuntimePaths {
        hot: run.join("hot"),
        root,
        run,
        origin: RuntimeOrigin::Installed,
        services,
    })
}

fn checked_relative_path(root: &Path, relative: &str, kind: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "installed runtime {kind} must stay inside the runtime root: {relative}"
        ));
    }
    let joined = root.join(path);
    if joined.exists() {
        let canonical_root = root.canonicalize().map_err(|error| {
            format!("failed to resolve runtime root {}: {error}", root.display())
        })?;
        let canonical_path = joined.canonicalize().map_err(|error| {
            format!(
                "failed to resolve installed runtime {kind} {}: {error}",
                joined.display()
            )
        })?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(format!(
                "installed runtime {kind} resolves outside the runtime root: {relative}"
            ));
        }
        return Ok(canonical_path);
    }
    Ok(joined)
}

fn command_names(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::{checked_relative_path, installed_paths};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "kyuubiki-runtime-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn installed_layout_requires_all_service_entries() {
        let root = fixture_root("missing-service");
        fs::create_dir_all(root.join("manifests")).unwrap();
        fs::write(
            root.join("manifests/service-launch.json"),
            r#"{"schema_version":"kyuubiki.service-launch/v1","services":[]}"#,
        )
        .unwrap();
        let error = installed_paths(root.clone()).err().unwrap();
        fs::remove_dir_all(root).unwrap();
        assert!(error.contains("missing required service"));
    }

    #[test]
    fn launch_paths_cannot_escape_runtime_root() {
        let root = fixture_root("escape");
        assert!(checked_relative_path(&root, "../node", "command").is_err());
        assert!(checked_relative_path(&root, "/usr/bin/node", "command").is_err());
        assert_eq!(
            checked_relative_path(&root, "bin/worker", "command").unwrap(),
            root.join("bin/worker")
        );
    }

    #[test]
    fn installed_services_resolve_only_inside_the_declared_root() {
        let root = fixture_root("service-resolution");
        for relative in ["manifests", "bin", "services/frontend"] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }
        fs::write(root.join("bin/service"), "fixture").unwrap();
        fs::write(
            root.join("manifests/service-launch.json"),
            r#"{
              "schema_version":"kyuubiki.service-launch/v1",
              "services":[
                {"id":"agent","command":"bin/service","args":["agent","{port}"],"cwd":"."},
                {"id":"orchestrator","command":"bin/service","args":[],"cwd":"."},
                {"id":"frontend","command":"bin/service","args":[],"cwd":"services/frontend"}
              ]
            }"#,
        )
        .unwrap();
        let paths = installed_paths(root.clone()).unwrap();
        let agent = paths
            .service("agent", &[("port", "5001".to_string())])
            .unwrap();
        assert_eq!(
            agent.command,
            root.join("bin/service").canonicalize().unwrap()
        );
        assert_eq!(agent.args, ["agent", "5001"]);
        fs::remove_dir_all(root).unwrap();
    }
}
