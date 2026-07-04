use crate::{credential_storage_contract, installation_integrity_report};

#[test]
fn credential_storage_contract_keeps_credentials_in_kyuubiki_sandbox() {
    let contract = credential_storage_contract();
    assert_eq!(contract.schema_version, "kyuubiki.credential-storage/v1");
    assert!(contract.sandbox_root.contains(".kyuubiki"));
    assert!(contract.sandbox_root.contains("credentials"));
    assert!(contract.platform_backends.iter().any(|backend| {
        backend.platform == "mobile-webview" && backend.backend == "platform-secure-store-handle"
    }));
    assert!(
        contract
            .classes
            .iter()
            .any(|rule| rule.class_id == "installer-ca" && rule.storage_path.contains(".kyuubiki"))
    );
    assert!(contract.denied_roots.iter().any(|root| root == "~/.ssh"));
    assert!(contract.render().contains("opaque credential handles"));
}

#[test]
fn installation_integrity_reports_component_protocol() {
    let report = installation_integrity_report();
    assert_eq!(
        report.component_protocol.schema_version,
        "kyuubiki.component-integrity/v1"
    );
    assert_eq!(
        report.component_protocol.covered_required_path_count,
        report.component_protocol.required_path_count
    );
    assert!(
        report.version_checks.iter().all(|check| check.ok),
        "version checks should all align after brand metadata is covered"
    );
    assert!(
        report
            .component_protocol
            .components
            .iter()
            .any(|component| component.id == "installer.core")
    );
    assert!(
        report
            .component_protocol
            .components
            .iter()
            .any(|component| component.id == "runtime.state")
    );
    assert!(
        report
            .component_protocol
            .issues
            .iter()
            .all(|issue| !issue.message.contains("outside owned paths"))
    );
    assert!(
        report
            .render()
            .contains("component_protocol: kyuubiki.component-integrity/v1")
    );
    assert!(report.render().contains("required_path_coverage:"));
}
