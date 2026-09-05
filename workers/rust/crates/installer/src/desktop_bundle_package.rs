use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use kyuubiki_platform::Platform;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DESKTOP_BUNDLE_SET_SCHEMA_VERSION: &str = "kyuubiki.desktop-bundle-set/v1";
const MANIFEST_PATH: &str = "manifests/desktop-bundle-set.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleSetManifest {
    pub schema_version: String,
    pub version: String,
    pub platform: String,
    pub payload_sha256: String,
    pub components: Vec<DesktopBundleComponent>,
    pub files: Vec<DesktopBundleFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleComponent {
    pub id: String,
    pub bundle_path: String,
    pub entrypoint: String,
    pub content_sha256: String,
    pub file_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub executable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopBundleSourceLayout {
    pub component_id: &'static str,
    pub bundle_path: &'static str,
    pub entrypoint: &'static str,
}

pub fn prepare_desktop_bundle_set(
    source_root: &Path,
    package_root: &Path,
    version: &str,
    platform: Platform,
) -> Result<DesktopBundleSetManifest, String> {
    validate_version(version)?;
    reject_symlink(source_root, "desktop bundle source")?;
    reject_symlink(package_root, "desktop bundle package")?;
    if !source_root.is_dir() {
        return Err(format!(
            "desktop bundle source is unavailable: {}",
            source_root.display()
        ));
    }
    if package_root.exists()
        && fs::read_dir(package_root)
            .map_err(|error| format!("failed to read {}: {error}", package_root.display()))?
            .next()
            .is_some()
    {
        return Err("desktop bundle package root must be empty".to_string());
    }
    fs::create_dir_all(package_root.join("payload"))
        .map_err(|error| format!("failed to create desktop package payload: {error}"))?;
    for definition in desktop_bundle_source_layout(platform) {
        let source = source_root.join(definition.bundle_path);
        let target = package_root.join("payload").join(definition.bundle_path);
        copy_tree(&source, &target)?;
    }
    seal_desktop_bundle_set(package_root, version, platform)
}

pub fn seal_desktop_bundle_set(
    package_root: &Path,
    version: &str,
    platform: Platform,
) -> Result<DesktopBundleSetManifest, String> {
    validate_version(version)?;
    reject_symlink(package_root, "desktop bundle package")?;
    let definitions = desktop_bundle_source_layout(platform);
    let mut files = collect_package_files(package_root)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files.is_empty() {
        return Err("desktop bundle package contains no payload files".to_string());
    }
    let components = definitions
        .iter()
        .map(|definition| component_record(definition, &files, platform))
        .collect::<Result<Vec<_>, _>>()?;
    let payload_sha256 = payload_digest(&components, &files);
    let manifest = DesktopBundleSetManifest {
        schema_version: DESKTOP_BUNDLE_SET_SCHEMA_VERSION.to_string(),
        version: version.to_string(),
        platform: platform.as_str().to_string(),
        payload_sha256,
        components,
        files,
    };
    write_manifest(package_root, &manifest)?;
    verify_desktop_bundle_set(package_root, platform)
}

pub fn verify_desktop_bundle_set(
    package_root: &Path,
    platform: Platform,
) -> Result<DesktopBundleSetManifest, String> {
    reject_symlink(package_root, "desktop bundle package")?;
    let manifest_path = package_root.join(MANIFEST_PATH);
    let manifest: DesktopBundleSetManifest = read_json(&manifest_path)?;
    if manifest.schema_version != DESKTOP_BUNDLE_SET_SCHEMA_VERSION {
        return Err("unsupported desktop bundle set schema".to_string());
    }
    validate_version(&manifest.version)?;
    if manifest.platform != platform.as_str() {
        return Err(format!(
            "desktop bundle set targets {}, current operation requires {}",
            manifest.platform,
            platform.as_str()
        ));
    }
    validate_manifest_shape(&manifest, platform)?;
    let mut actual_files = collect_package_files(package_root)?;
    actual_files.sort_by(|left, right| left.path.cmp(&right.path));
    if actual_files != manifest.files {
        return Err("desktop bundle file inventory or content digest drifted".to_string());
    }
    let expected_components = desktop_bundle_source_layout(platform)
        .iter()
        .map(|definition| component_record(definition, &actual_files, platform))
        .collect::<Result<Vec<_>, _>>()?;
    if expected_components != manifest.components {
        return Err("desktop bundle component digest or entrypoint drifted".to_string());
    }
    if payload_digest(&manifest.components, &manifest.files) != manifest.payload_sha256 {
        return Err("desktop bundle payload digest mismatch".to_string());
    }
    Ok(manifest)
}

pub(crate) fn copy_verified_desktop_bundle_set(
    source: &Path,
    target: &Path,
    platform: Platform,
) -> Result<DesktopBundleSetManifest, String> {
    let manifest = verify_desktop_bundle_set(source, platform)?;
    reject_symlink(target, "desktop bundle target")?;
    if target.exists() {
        return Err(format!(
            "desktop bundle target already exists: {}",
            target.display()
        ));
    }
    copy_tree(source, target)?;
    let installed = verify_desktop_bundle_set(target, platform)?;
    if installed != manifest {
        return Err("copied desktop bundle set changed during staging".to_string());
    }
    Ok(installed)
}

fn component_record(
    definition: &DesktopBundleSourceLayout,
    files: &[DesktopBundleFile],
    platform: Platform,
) -> Result<DesktopBundleComponent, String> {
    let bundle_path = format!("payload/{}", definition.bundle_path);
    let entrypoint = format!("payload/{}", definition.entrypoint);
    let component_files = files
        .iter()
        .filter(|file| {
            file.path == bundle_path || file.path.starts_with(&format!("{bundle_path}/"))
        })
        .collect::<Vec<_>>();
    if component_files.is_empty() {
        return Err(format!(
            "desktop component `{}` has no payload files",
            definition.component_id
        ));
    }
    let entry = files
        .iter()
        .find(|file| file.path == entrypoint)
        .ok_or_else(|| {
            format!(
                "desktop component `{}` entrypoint is missing",
                definition.component_id
            )
        })?;
    if platform != Platform::Windows && !entry.executable {
        return Err(format!(
            "desktop component `{}` entrypoint is not executable",
            definition.component_id
        ));
    }
    Ok(DesktopBundleComponent {
        id: definition.component_id.to_string(),
        bundle_path,
        entrypoint,
        content_sha256: component_digest(&component_files),
        file_count: component_files.len(),
    })
}

fn validate_manifest_shape(
    manifest: &DesktopBundleSetManifest,
    platform: Platform,
) -> Result<(), String> {
    if !valid_sha256(&manifest.payload_sha256) {
        return Err("desktop bundle payload digest is invalid".to_string());
    }
    let expected = desktop_bundle_source_layout(platform);
    if manifest.components.len() != expected.len() || manifest.files.is_empty() {
        return Err("desktop bundle set must contain exactly three components".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for component in &manifest.components {
        checked_relative(&component.bundle_path)?;
        checked_relative(&component.entrypoint)?;
        if !ids.insert(component.id.as_str())
            || component.file_count == 0
            || !valid_sha256(&component.content_sha256)
        {
            return Err("desktop bundle component metadata is invalid".to_string());
        }
    }
    for file in &manifest.files {
        checked_relative(&file.path)?;
        if !file.path.starts_with("payload/")
            || !paths.insert(file.path.as_str())
            || !valid_sha256(&file.sha256)
        {
            return Err("desktop bundle file metadata is invalid".to_string());
        }
    }
    let actual = manifest
        .components
        .iter()
        .map(|component| component.id.as_str())
        .collect::<BTreeSet<_>>();
    let required = expected
        .iter()
        .map(|definition| definition.component_id)
        .collect::<BTreeSet<_>>();
    if actual != required {
        return Err("desktop bundle component set is incomplete".to_string());
    }
    Ok(())
}

fn collect_package_files(package_root: &Path) -> Result<Vec<DesktopBundleFile>, String> {
    let mut paths = Vec::new();
    collect_files(package_root, package_root, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .filter(|path| path != Path::new(MANIFEST_PATH))
        .map(|relative| {
            if !relative.starts_with("payload") {
                return Err(format!(
                    "desktop bundle package contains unmanaged file: {}",
                    relative.display()
                ));
            }
            let full = package_root.join(&relative);
            Ok(DesktopBundleFile {
                path: portable_path(&relative)?,
                sha256: sha256_file(&full)?,
                size_bytes: full
                    .metadata()
                    .map_err(|error| format!("failed to inspect {}: {error}", full.display()))?
                    .len(),
                executable: is_executable(&full),
            })
        })
        .collect()
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    reject_symlink(current, "desktop bundle path")?;
    if current.is_file() {
        output.push(
            current
                .strip_prefix(root)
                .map_err(|_| "desktop bundle path escaped its package".to_string())?
                .to_path_buf(),
        );
        return Ok(());
    }
    if !current.is_dir() {
        return Err(format!(
            "desktop bundle path is not a regular file or directory: {}",
            current.display()
        ));
    }
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("failed to read {}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to read {}: {error}", current.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        collect_files(root, &entry.path(), output)?;
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    reject_symlink(source, "desktop bundle copy source")?;
    if source.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::copy(source, target).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                source.display(),
                target.display()
            )
        })?;
        return Ok(());
    }
    if !source.is_dir() {
        return Err(format!(
            "desktop bundle copy source is unavailable: {}",
            source.display()
        ));
    }
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        copy_tree(&entry.path(), &target.join(entry.file_name()))?;
    }
    Ok(())
}

fn component_digest(files: &[&DesktopBundleFile]) -> String {
    let mut digest = Sha256::new();
    for file in files {
        hash_file_record(&mut digest, file);
    }
    format!("{:x}", digest.finalize())
}

fn payload_digest(components: &[DesktopBundleComponent], files: &[DesktopBundleFile]) -> String {
    let mut digest = Sha256::new();
    digest.update(DESKTOP_BUNDLE_SET_SCHEMA_VERSION.as_bytes());
    digest.update([0]);
    for component in components {
        for value in [
            component.id.as_str(),
            component.bundle_path.as_str(),
            component.entrypoint.as_str(),
            component.content_sha256.as_str(),
        ] {
            digest.update(value.as_bytes());
            digest.update([0]);
        }
        digest.update(component.file_count.to_le_bytes());
    }
    for file in files {
        hash_file_record(&mut digest, file);
    }
    format!("{:x}", digest.finalize())
}

fn hash_file_record(digest: &mut Sha256, file: &DesktopBundleFile) {
    digest.update(file.path.as_bytes());
    digest.update([0]);
    digest.update(file.sha256.as_bytes());
    digest.update(file.size_bytes.to_le_bytes());
    digest.update([u8::from(file.executable)]);
}

pub fn desktop_bundle_source_layout(platform: Platform) -> [DesktopBundleSourceLayout; 3] {
    match platform {
        Platform::Macos => [
            definition(
                "hub",
                "Kyuubiki Hub.app",
                "Kyuubiki Hub.app/Contents/MacOS/kyuubiki-hub-gui",
            ),
            definition(
                "installer",
                "Kyuubiki Installer.app",
                "Kyuubiki Installer.app/Contents/MacOS/kyuubiki-installer-gui",
            ),
            definition(
                "workbench",
                "Kyuubiki Workbench.app",
                "Kyuubiki Workbench.app/Contents/MacOS/kyuubiki-workbench-gui",
            ),
        ],
        Platform::Linux => [
            definition("hub", "kyuubiki-hub-gui", "kyuubiki-hub-gui"),
            definition(
                "installer",
                "kyuubiki-installer-gui",
                "kyuubiki-installer-gui",
            ),
            definition(
                "workbench",
                "kyuubiki-workbench-gui",
                "kyuubiki-workbench-gui",
            ),
        ],
        Platform::Windows => [
            definition("hub", "Kyuubiki Hub", "Kyuubiki Hub/kyuubiki-hub-gui.exe"),
            definition(
                "installer",
                "Kyuubiki Installer",
                "Kyuubiki Installer/kyuubiki-installer-gui.exe",
            ),
            definition(
                "workbench",
                "Kyuubiki Workbench",
                "Kyuubiki Workbench/kyuubiki-workbench-gui.exe",
            ),
        ],
    }
}

const fn definition(
    id: &'static str,
    bundle_path: &'static str,
    entrypoint: &'static str,
) -> DesktopBundleSourceLayout {
    DesktopBundleSourceLayout {
        component_id: id,
        bundle_path,
        entrypoint,
    }
}

fn validate_version(version: &str) -> Result<(), String> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || part.parse::<u64>().is_err())
    {
        return Err(format!("desktop bundle version must be semver: {version}"));
    }
    Ok(())
}

fn checked_relative(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!("desktop bundle path is not portable: {value}"));
    }
    Ok(path)
}

fn portable_path(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| format!("desktop bundle path is not UTF-8: {}", path.display()))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    checked_relative(&text)?;
    Ok(text)
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && metadata.file_type().is_symlink()
    {
        return Err(format!("{label} cannot be a symlink: {}", path.display()));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

fn write_manifest(root: &Path, manifest: &DesktopBundleSetManifest) -> Result<(), String> {
    let path = root.join(MANIFEST_PATH);
    let parent = path
        .parent()
        .ok_or_else(|| "desktop bundle manifest has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("failed to serialize desktop bundle manifest: {error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn manifest_by_component(
    manifest: &DesktopBundleSetManifest,
) -> BTreeMap<&str, &DesktopBundleComponent> {
    manifest
        .components
        .iter()
        .map(|component| (component.id.as_str(), component))
        .collect()
}
