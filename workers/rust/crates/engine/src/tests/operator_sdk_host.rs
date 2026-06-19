use crate::{
    built_in_registry_with_external_packages, load_external_operator_packages_with_deferred_host,
    load_external_operator_packages_with_dynamic_host, BuiltInOperatorRegistryKind,
    ExternalOperatorHostConfig, ExternalOperatorTrustPolicy,
};
use kyuubiki_operator_sdk::{
    current_platform_library_file_name, current_platform_library_path, partial_validation,
    OperatorDescriptorBuilder, OperatorHandler, OperatorPackageActivator, OperatorPackageLoadError,
    OperatorPackageLoadPlan, OperatorRegistry, OperatorSdkError,
};
use kyuubiki_protocol::{OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct StaticExternalOperator {
    descriptor: kyuubiki_protocol::OperatorDescriptor,
    package_id: String,
}

impl OperatorHandler for StaticExternalOperator {
    fn descriptor(&self) -> &kyuubiki_protocol::OperatorDescriptor {
        &self.descriptor
    }

    fn run(&self, _request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError> {
        Ok(OperatorRunResult {
            operator_id: self.descriptor.id.clone(),
            summary: serde_json::json!({
                "package_id": self.package_id,
                "source": "external_local",
            }),
            artifacts: Vec::new(),
        })
    }
}

struct TestActivator;

impl OperatorPackageActivator for TestActivator {
    fn activate_package(
        &self,
        plan: &OperatorPackageLoadPlan,
        registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError> {
        for operator in &plan.manifest.operators {
            registry
                .register(StaticExternalOperator {
                    descriptor: OperatorDescriptorBuilder::new(
                        operator.operator_id.clone(),
                        OperatorKind::Extract,
                        "multi_domain",
                        operator.operator_id.replace('.', "_"),
                    )
                    .summary(format!("Loaded from {}", plan.manifest.package_id))
                    .validation(partial_validation("engine_host_test"))
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
fn loads_external_local_package_into_built_in_registry() {
    let packages_root = temp_dir("external-host-success");
    let package_dir = packages_root.join("operator-alpha");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.alpha",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}operator_alpha.{lib_extension}",
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

    let (registry, report) = built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root),
        &TestActivator,
    )
    .expect("external package should load");

    assert_eq!(report.activated_packages.len(), 1);
    assert_eq!(
        report.activated_packages[0].manifest.package_id,
        "operator.alpha"
    );

    let result = registry
        .run(OperatorRunRequest {
            operator_id: "extract.alpha".to_string(),
            input: serde_json::json!({}),
            context: OperatorRunContext::default(),
        })
        .expect("external operator should run");
    assert_eq!(
        result.summary["package_id"].as_str(),
        Some("operator.alpha")
    );
    assert_eq!(result.summary["source"].as_str(), Some("external_local"));
}

#[test]
fn deferred_host_reports_dynamic_loading_as_not_enabled() {
    let packages_root = temp_dir("external-host-deferred");
    let package_dir = packages_root.join("operator-beta");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.beta",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}operator_beta.{lib_extension}",
            "operators": [
                {
                    "operator_id": "extract.beta",
                    "kind": "extract",
                    "entry_symbol": "register_beta"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let error = match load_external_operator_packages_with_deferred_host(
        BuiltInOperatorRegistryKind::Extract,
        &packages_root,
    ) {
        Ok(_) => panic!("default host should reject dynamic activation"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("runtime host has not enabled dynamic loading"));
    assert!(error.to_string().contains("operator.beta"));
}

#[test]
fn dynamic_host_reports_missing_library_path() {
    let packages_root = temp_dir("external-host-missing-library");
    let package_dir = packages_root.join("operator-gamma");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.gamma",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}operator_gamma.{lib_extension}",
            "operators": [
                {
                    "operator_id": "extract.gamma",
                    "kind": "extract",
                    "entry_symbol": "register_gamma"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let error = match load_external_operator_packages_with_dynamic_host(
        BuiltInOperatorRegistryKind::Extract,
        &packages_root,
    ) {
        Ok(_) => panic!("dynamic host should fail when library is missing"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("failed to open dynamic library"));
    assert!(error.to_string().contains("operator.gamma"));
}

#[test]
fn host_policy_rejects_non_allowlisted_package_id() {
    let packages_root = temp_dir("external-host-policy-allowlist");
    let package_dir = packages_root.join("operator-delta");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.delta",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}operator_delta.{lib_extension}",
            "operators": [
                {
                    "operator_id": "extract.delta",
                    "kind": "extract",
                    "entry_symbol": "register_delta"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let error = match built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root)
            .with_trust_policy(ExternalOperatorTrustPolicy::allow_package_ids([
                "operator.alpha",
            ])),
        &TestActivator,
    ) {
        Ok(_) => panic!("host policy should reject unknown package id"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("allowlist"));
    assert!(error.to_string().contains("operator.delta"));
}

#[test]
fn host_policy_rejects_disallowed_runtime() {
    let packages_root = temp_dir("external-host-policy-runtime");
    let package_dir = packages_root.join("operator-epsilon");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.epsilon",
            "package_version": "0.1.0",
            "runtime": "python_wasm",
            "entrypoint": "target/debug/{lib_prefix}operator_epsilon.{lib_extension}",
            "operators": [
                {
                    "operator_id": "extract.epsilon",
                    "kind": "extract",
                    "entry_symbol": "register_epsilon"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let error = match built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root),
        &TestActivator,
    ) {
        Ok(_) => panic!("host policy should reject disallowed runtime"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("runtime python_wasm is not allowed"));
    assert!(error.to_string().contains("operator.epsilon"));
}

#[test]
fn host_policy_rejects_entrypoint_outside_package_root() {
    let packages_root = temp_dir("external-host-policy-root");
    let package_dir = packages_root.join("operator-zeta");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.zeta",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": "../outside/{lib_prefix}operator_zeta.{lib_extension}",
            "operators": [
                {
                    "operator_id": "extract.zeta",
                    "kind": "extract",
                    "entry_symbol": "register_zeta"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let error = match built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root),
        &TestActivator,
    ) {
        Ok(_) => panic!("host policy should reject escaping entrypoint"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("escapes package root"));
    assert!(error.to_string().contains("operator.zeta"));
}

#[test]
fn host_policy_can_allow_absolute_entrypoints_for_trusted_packages() {
    let packages_root = temp_dir("external-host-policy-absolute");
    let package_dir = packages_root.join("operator-eta");
    fs::create_dir_all(&package_dir).expect("create package dir");
    let trusted_library =
        current_platform_library_path(packages_root.join("trusted"), "operator_eta");
    fs::create_dir_all(trusted_library.parent().expect("trusted parent"))
        .expect("create trusted parent");
    fs::write(&trusted_library, b"placeholder").expect("write placeholder library");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.eta",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": trusted_library,
            "operators": [
                {
                    "operator_id": "extract.eta",
                    "kind": "extract",
                    "entry_symbol": "register_eta"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let mut runtimes = BTreeSet::new();
    runtimes.insert("rust_crate".to_string());
    let trust_policy = ExternalOperatorTrustPolicy {
        allowed_package_ids: Some(BTreeSet::from(["operator.eta".to_string()])),
        allowed_runtimes: runtimes,
        allow_absolute_entrypoints: true,
        require_entrypoint_within_package_root: false,
    };

    let (registry, report) = built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root)
            .with_trust_policy(trust_policy),
        &TestActivator,
    )
    .expect("trusted absolute entrypoint should be accepted");

    assert!(registry.describe("extract.eta").is_some());
    assert_eq!(report.activated_packages.len(), 1);
}

#[test]
#[ignore = "requires prebuilt operator template cdylib"]
fn loads_prebuilt_template_cdylib_through_dynamic_host() {
    let packages_root = temp_dir("external-host-template-dylib");
    let template_dylib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates/operator-crate-template/target/debug")
        .join(current_platform_library_file_name(
            "kyuubiki_operator_template",
        ));
    assert!(
        template_dylib.exists(),
        "template cdylib must be prebuilt before running this test"
    );

    let package_dir = packages_root.join("operator-template");
    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "package_id": "operator.template.summary",
            "package_version": "0.1.0",
            "runtime": "rust_crate",
            "entrypoint": template_dylib,
            "operators": [
                {
                    "operator_id": "extract.template_summary",
                    "kind": "extract",
                    "entry_symbol": "register_template_operator"
                }
            ]
        })
        .to_string(),
    )
    .expect("write manifest");

    let trust_policy = ExternalOperatorTrustPolicy {
        allowed_package_ids: Some(BTreeSet::from(["operator.template.summary".to_string()])),
        allowed_runtimes: BTreeSet::from(["rust_crate".to_string()]),
        allow_absolute_entrypoints: true,
        require_entrypoint_within_package_root: false,
    };
    let activator = crate::DynamicLibraryOperatorActivator::default();
    let (registry, report) = built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, &packages_root)
            .with_trust_policy(trust_policy),
        &activator,
    )
    .expect("dynamic host should load prebuilt template library");

    let result = registry
        .run(OperatorRunRequest {
            operator_id: "extract.template_summary".to_string(),
            input: serde_json::json!({ "values": [2.0, 4.0, 8.0] }),
            context: OperatorRunContext::default(),
        })
        .expect("template operator should run");

    assert_eq!(report.activated_packages.len(), 1);
    assert_eq!(result.summary["count"].as_u64(), Some(3));
    assert_eq!(result.summary["sum"].as_f64(), Some(14.0));
    assert_eq!(result.summary["max"].as_f64(), Some(8.0));
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kyuubiki-engine-{label}-{unique}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
