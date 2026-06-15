use crate::manifest::{DiscoveredOperatorPackage, discover_operator_packages};
use crate::{
    current_platform_library_file_name, expand_platform_library_template,
};
use crate::OperatorRegistry;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorPackageLoadPlan {
    pub package_root: PathBuf,
    pub manifest_path: PathBuf,
    pub entrypoint_path: PathBuf,
    pub entrypoint_candidates: Vec<PathBuf>,
    pub manifest: crate::OperatorPackageManifest,
}

pub trait OperatorPackageActivator {
    fn activate_package(
        &self,
        plan: &OperatorPackageLoadPlan,
        registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError>;
}

#[derive(Debug)]
pub enum OperatorPackageLoadError {
    Manifest(crate::OperatorManifestError),
    Activation { package_id: String, message: String },
}

impl Display for OperatorPackageLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(f),
            Self::Activation {
                package_id,
                message,
            } => {
                write!(f, "failed to activate operator package {package_id}: {message}")
            }
        }
    }
}

impl std::error::Error for OperatorPackageLoadError {}

impl From<crate::OperatorManifestError> for OperatorPackageLoadError {
    fn from(value: crate::OperatorManifestError) -> Self {
        Self::Manifest(value)
    }
}

pub fn build_operator_package_load_plan(
    discovered: DiscoveredOperatorPackage,
) -> OperatorPackageLoadPlan {
    let entrypoint_candidates = resolve_entrypoint_candidates(
        &discovered.package_root,
        &discovered.manifest.entrypoint,
    );
    let entrypoint_path = select_entrypoint_path(&entrypoint_candidates);
    OperatorPackageLoadPlan {
        package_root: discovered.package_root,
        manifest_path: discovered.manifest_path,
        entrypoint_path,
        entrypoint_candidates,
        manifest: discovered.manifest,
    }
}

pub fn activate_discovered_operator_packages(
    discovered: Vec<DiscoveredOperatorPackage>,
    activator: &impl OperatorPackageActivator,
    registry: &mut OperatorRegistry,
) -> Result<Vec<OperatorPackageLoadPlan>, OperatorPackageLoadError> {
    let mut activated = Vec::new();
    for package in discovered {
        let plan = build_operator_package_load_plan(package);
        activator.activate_package(&plan, registry)?;
        activated.push(plan);
    }
    Ok(activated)
}

pub fn discover_and_activate_operator_packages(
    root: impl AsRef<Path>,
    activator: &impl OperatorPackageActivator,
    registry: &mut OperatorRegistry,
) -> Result<Vec<OperatorPackageLoadPlan>, OperatorPackageLoadError> {
    let discovered = discover_operator_packages(root)?;
    activate_discovered_operator_packages(discovered, activator, registry)
}

fn resolve_entrypoint_candidates(package_root: &Path, entrypoint: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let expanded = expand_entrypoint_template(entrypoint);
    push_unique_candidate(&mut candidates, package_root, &expanded);

    for variant in derive_platform_entrypoint_variants(&expanded) {
        push_unique_candidate(&mut candidates, package_root, &variant);
    }

    candidates
}

fn select_entrypoint_path(candidates: &[PathBuf]) -> PathBuf {
    candidates
        .iter()
        .find(|candidate| candidate.exists())
        .cloned()
        .or_else(|| candidates.first().cloned())
        .unwrap_or_default()
}

fn expand_entrypoint_template(entrypoint: &str) -> String {
    expand_platform_library_template(entrypoint)
}

fn derive_platform_entrypoint_variants(entrypoint: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let file_name = Path::new(entrypoint)
        .file_name()
        .and_then(|value| value.to_str());
    let Some(file_name) = file_name else {
        return variants;
    };

    let stem = file_name
        .strip_prefix("lib")
        .and_then(|trimmed| trimmed.rsplit_once('.').map(|(value, _)| value))
        .or_else(|| file_name.rsplit_once('.').map(|(value, _)| value))
        .unwrap_or(file_name);
    let parent = Path::new(entrypoint).parent().unwrap_or_else(|| Path::new(""));

    for variant_name in current_platform_library_names(stem) {
        let variant_path = if parent.as_os_str().is_empty() {
            PathBuf::from(variant_name)
        } else {
            parent.join(variant_name)
        };
        if let Some(text) = variant_path.to_str() {
            variants.push(text.to_string());
        }
    }

    variants
}

fn current_platform_library_names(stem: &str) -> Vec<String> {
    vec![current_platform_library_file_name(stem)]
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, package_root: &Path, candidate: &str) {
    let candidate = PathBuf::from(candidate);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        package_root.join(candidate)
    };
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OperatorPackageActivator, OperatorPackageLoadError, build_operator_package_load_plan,
        discover_and_activate_operator_packages,
    };
    use crate::{
        OperatorDescriptorBuilder, OperatorRegistry, OperatorRunRequest, OperatorRunResult,
        OperatorSdkError, current_platform_library_file_name, operator_summary_result,
        partial_validation, read_operator_package_manifest,
    };
    use kyuubiki_protocol::{OperatorDescriptor, OperatorKind};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StaticHandler {
        descriptor: OperatorDescriptor,
        package_id: String,
    }

    impl crate::OperatorHandler for StaticHandler {
        fn descriptor(&self) -> &OperatorDescriptor {
            &self.descriptor
        }

        fn run(
            &self,
            _request: OperatorRunRequest,
        ) -> Result<OperatorRunResult, OperatorSdkError> {
            Ok(operator_summary_result(
                self.descriptor.id.clone(),
                serde_json::json!({ "package_id": self.package_id }),
            ))
        }
    }

    struct TestActivator;

    impl OperatorPackageActivator for TestActivator {
        fn activate_package(
            &self,
            plan: &super::OperatorPackageLoadPlan,
            registry: &mut OperatorRegistry,
        ) -> Result<(), OperatorPackageLoadError> {
            for operator in &plan.manifest.operators {
                registry
                    .register(StaticHandler {
                        descriptor: OperatorDescriptorBuilder::new(
                            operator.operator_id.clone(),
                            OperatorKind::Extract,
                            "multi_domain",
                            operator.operator_id.replace('.', "_"),
                        )
                        .summary(format!("Loaded from {}", plan.manifest.package_id))
                        .validation(partial_validation("loader_test"))
                        .build(),
                        package_id: plan.manifest.package_id.clone(),
                    })
                    .map_err(|error| OperatorPackageLoadError::Activation {
                        package_id: plan.manifest.package_id.clone(),
                        message: error.to_string(),
                    })?;
            }
            Ok(())
        }
    }

    #[test]
    fn builds_load_plan_with_resolved_entrypoint() {
        let package_root = temp_dir("load-plan");
        let manifest_path = package_root.join(crate::OPERATOR_PACKAGE_MANIFEST_FILE);
        fs::write(
            &manifest_path,
            serde_json::json!({
                "schema_version": crate::OPERATOR_PACKAGE_SCHEMA_VERSION,
                "package_id": "operator.example.loader",
                "package_version": "0.1.0",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/{lib_prefix}operator_example_loader.{lib_extension}",
                "operators": [
                    {
                        "operator_id": "extract.example_loader",
                        "kind": "extract",
                        "entry_symbol": "register_operator"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write manifest");
        let manifest = read_operator_package_manifest(&manifest_path).expect("read manifest");

        let plan = build_operator_package_load_plan(crate::DiscoveredOperatorPackage {
            package_root: package_root.clone(),
            manifest_path: manifest_path.clone(),
            manifest,
        });

        assert_eq!(
            plan.entrypoint_path,
            package_root.join("target/debug").join(current_platform_library_file_name(
                "operator_example_loader",
            ))
        );
        assert!(!plan.entrypoint_candidates.is_empty());
    }

    #[test]
    fn discovers_and_activates_operator_packages() {
        let root = temp_dir("load-activate");
        let package_dir = root.join("operator-alpha");
        fs::create_dir_all(&package_dir).expect("create package dir");
        fs::write(
            package_dir.join(crate::OPERATOR_PACKAGE_MANIFEST_FILE),
            serde_json::json!({
                "schema_version": crate::OPERATOR_PACKAGE_SCHEMA_VERSION,
                "package_id": "operator.alpha",
                "package_version": "0.1.0",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_alpha.dylib",
                "operators": [
                    {
                        "operator_id": "extract.alpha",
                        "kind": "extract",
                        "entry_symbol": "register_alpha"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write manifest");

        let mut registry = OperatorRegistry::new();
        let plans =
            discover_and_activate_operator_packages(&root, &TestActivator, &mut registry)
                .expect("packages should activate");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].manifest.package_id, "operator.alpha");
        let result = registry
            .run(OperatorRunRequest {
                operator_id: "extract.alpha".to_string(),
                input: serde_json::json!({}),
                context: Default::default(),
            })
            .expect("registered operator should run");
        assert_eq!(result.summary["package_id"].as_str(), Some("operator.alpha"));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kyuubiki-loader-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
