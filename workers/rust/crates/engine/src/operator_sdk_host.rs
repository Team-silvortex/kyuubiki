use crate::operator_sdk_runtime::{built_in_operator_registry, BuiltInOperatorRegistryKind};
use kyuubiki_operator_sdk::{
    OperatorPackageActivator, OperatorPackageLoadError, OperatorPackageLoadPlan,
    OperatorRegistrationEntrypoint, OperatorRegistry,
};
use libloading::Library;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalOperatorHostConfig {
    pub registry_kind: BuiltInOperatorRegistryKind,
    pub packages_root: PathBuf,
    pub trust_policy: ExternalOperatorTrustPolicy,
}

impl ExternalOperatorHostConfig {
    pub fn new(
        registry_kind: BuiltInOperatorRegistryKind,
        packages_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            registry_kind,
            packages_root: packages_root.into(),
            trust_policy: ExternalOperatorTrustPolicy::default(),
        }
    }

    pub fn with_trust_policy(mut self, trust_policy: ExternalOperatorTrustPolicy) -> Self {
        self.trust_policy = trust_policy;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalOperatorTrustPolicy {
    pub allowed_package_ids: Option<BTreeSet<String>>,
    pub allowed_runtimes: BTreeSet<String>,
    pub allow_absolute_entrypoints: bool,
    pub require_entrypoint_within_package_root: bool,
}

impl Default for ExternalOperatorTrustPolicy {
    fn default() -> Self {
        let mut allowed_runtimes = BTreeSet::new();
        allowed_runtimes.insert("rust_crate".to_string());
        Self {
            allowed_package_ids: None,
            allowed_runtimes,
            allow_absolute_entrypoints: false,
            require_entrypoint_within_package_root: true,
        }
    }
}

impl ExternalOperatorTrustPolicy {
    pub fn allow_package_ids(package_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut policy = Self::default();
        policy.allowed_package_ids = Some(
            package_ids
                .into_iter()
                .map(Into::into)
                .collect::<BTreeSet<_>>(),
        );
        policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalOperatorLoadReport {
    pub registry_kind: BuiltInOperatorRegistryKind,
    pub packages_root: PathBuf,
    pub activated_packages: Vec<OperatorPackageLoadPlan>,
}

#[derive(Debug)]
pub enum ExternalOperatorHostError {
    Policy { package_id: String, message: String },
    Activation(OperatorPackageLoadError),
}

impl Display for ExternalOperatorHostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy {
                package_id,
                message,
            } => {
                write!(
                    f,
                    "external operator package {package_id} rejected by host policy: {message}"
                )
            }
            Self::Activation(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ExternalOperatorHostError {}

impl From<OperatorPackageLoadError> for ExternalOperatorHostError {
    fn from(value: OperatorPackageLoadError) -> Self {
        Self::Activation(value)
    }
}

pub fn built_in_registry_with_external_packages(
    config: &ExternalOperatorHostConfig,
    activator: &impl OperatorPackageActivator,
) -> Result<(OperatorRegistry, ExternalOperatorLoadReport), ExternalOperatorHostError> {
    let mut registry = built_in_operator_registry(config.registry_kind);
    let activated_packages =
        discover_activate_and_validate_operator_packages(config, activator, &mut registry)?;
    Ok((
        registry,
        ExternalOperatorLoadReport {
            registry_kind: config.registry_kind,
            packages_root: config.packages_root.clone(),
            activated_packages,
        },
    ))
}

fn discover_activate_and_validate_operator_packages(
    config: &ExternalOperatorHostConfig,
    activator: &impl OperatorPackageActivator,
    registry: &mut OperatorRegistry,
) -> Result<Vec<OperatorPackageLoadPlan>, ExternalOperatorHostError> {
    let discovered = kyuubiki_operator_sdk::discover_operator_packages(&config.packages_root)
        .map_err(kyuubiki_operator_sdk::OperatorPackageLoadError::Manifest)
        .map_err(ExternalOperatorHostError::from)?;
    let mut activated = Vec::new();
    for package in discovered {
        let plan = kyuubiki_operator_sdk::build_operator_package_load_plan(package);
        validate_load_plan_against_policy(&plan, &config.trust_policy)?;
        activator.activate_package(&plan, registry)?;
        activated.push(plan);
    }
    Ok(activated)
}

fn validate_load_plan_against_policy(
    plan: &OperatorPackageLoadPlan,
    trust_policy: &ExternalOperatorTrustPolicy,
) -> Result<(), ExternalOperatorHostError> {
    let normalized_package_root = normalize_path(&plan.package_root);
    let normalized_entrypoint_path = normalize_path(&plan.entrypoint_path);

    if let Some(allowed_package_ids) = &trust_policy.allowed_package_ids {
        if !allowed_package_ids.contains(&plan.manifest.package_id) {
            return Err(ExternalOperatorHostError::Policy {
                package_id: plan.manifest.package_id.clone(),
                message: "package_id is not present in the host allowlist".to_string(),
            });
        }
    }

    if !trust_policy
        .allowed_runtimes
        .contains(&plan.manifest.runtime)
    {
        return Err(ExternalOperatorHostError::Policy {
            package_id: plan.manifest.package_id.clone(),
            message: format!(
                "runtime {} is not allowed by the host policy",
                plan.manifest.runtime
            ),
        });
    }

    if !trust_policy.allow_absolute_entrypoints
        && Path::new(&plan.manifest.entrypoint).is_absolute()
    {
        return Err(ExternalOperatorHostError::Policy {
            package_id: plan.manifest.package_id.clone(),
            message: "absolute entrypoint paths are disabled by the host policy".to_string(),
        });
    }

    if trust_policy.require_entrypoint_within_package_root
        && !normalized_entrypoint_path.starts_with(&normalized_package_root)
    {
        return Err(ExternalOperatorHostError::Policy {
            package_id: plan.manifest.package_id.clone(),
            message: format!(
                "resolved entrypoint {} escapes package root {}",
                normalized_entrypoint_path.display(),
                normalized_package_root.display()
            ),
        });
    }

    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

pub struct DynamicOperatorHostSession {
    registry: OperatorRegistry,
    loaded_libraries: Vec<Library>,
    report: ExternalOperatorLoadReport,
}

impl DynamicOperatorHostSession {
    pub fn registry(&self) -> &OperatorRegistry {
        &self.registry
    }

    pub fn report(&self) -> &ExternalOperatorLoadReport {
        &self.report
    }

    pub fn loaded_library_count(&self) -> usize {
        self.loaded_libraries.len()
    }
}

#[derive(Default)]
pub struct DynamicLibraryOperatorActivator {
    loaded_libraries: Mutex<Vec<Library>>,
}

impl DynamicLibraryOperatorActivator {
    pub fn into_loaded_libraries(self) -> Vec<Library> {
        self.loaded_libraries
            .into_inner()
            .expect("dynamic library activator lock should not be poisoned")
    }
}

impl OperatorPackageActivator for DynamicLibraryOperatorActivator {
    fn activate_package(
        &self,
        plan: &OperatorPackageLoadPlan,
        registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError> {
        let library = unsafe { Library::new(&plan.entrypoint_path) }.map_err(|error| {
            OperatorPackageLoadError::Activation {
                package_id: plan.manifest.package_id.clone(),
                message: format!(
                    "failed to open dynamic library {}: {}",
                    plan.entrypoint_path.display(),
                    error
                ),
            }
        })?;

        for operator in &plan.manifest.operators {
            let entry_symbol = operator.entry_symbol.as_bytes();
            let register = unsafe { library.get::<OperatorRegistrationEntrypoint>(entry_symbol) }
                .map_err(|error| OperatorPackageLoadError::Activation {
                package_id: plan.manifest.package_id.clone(),
                message: format!(
                    "failed to resolve symbol {} in {}: {}",
                    operator.entry_symbol,
                    plan.entrypoint_path.display(),
                    error
                ),
            })?;
            unsafe { register(registry) }.map_err(|error| {
                OperatorPackageLoadError::Activation {
                    package_id: plan.manifest.package_id.clone(),
                    message: error.to_string(),
                }
            })?;
        }

        self.loaded_libraries
            .lock()
            .expect("dynamic library activator lock should not be poisoned")
            .push(library);
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DeferredDynamicLoadActivator;

impl OperatorPackageActivator for DeferredDynamicLoadActivator {
    fn activate_package(
        &self,
        plan: &OperatorPackageLoadPlan,
        _registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError> {
        Err(OperatorPackageLoadError::Activation {
            package_id: plan.manifest.package_id.clone(),
            message: format!(
                "runtime host has not enabled dynamic loading for entrypoint {}",
                plan.entrypoint_path.display()
            ),
        })
    }
}

pub fn load_external_operator_packages_with_deferred_host(
    registry_kind: BuiltInOperatorRegistryKind,
    packages_root: impl AsRef<Path>,
) -> Result<(OperatorRegistry, ExternalOperatorLoadReport), ExternalOperatorHostError> {
    built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(registry_kind, packages_root.as_ref()),
        &DeferredDynamicLoadActivator,
    )
}

pub fn load_external_operator_packages_with_dynamic_host(
    registry_kind: BuiltInOperatorRegistryKind,
    packages_root: impl AsRef<Path>,
) -> Result<DynamicOperatorHostSession, ExternalOperatorHostError> {
    let activator = DynamicLibraryOperatorActivator::default();
    let (registry, report) = built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(registry_kind, packages_root.as_ref()),
        &activator,
    )?;
    Ok(DynamicOperatorHostSession {
        registry,
        loaded_libraries: activator.into_loaded_libraries(),
        report,
    })
}
