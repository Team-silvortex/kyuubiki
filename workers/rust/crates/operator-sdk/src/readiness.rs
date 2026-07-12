use crate::{OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION, OperatorPackageManifest};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorPortDescriptor, OperatorValidationStatus,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorSdkReadinessSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorSdkReadinessIssue {
    pub severity: OperatorSdkReadinessSeverity,
    pub code: &'static str,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorSdkReadinessReport {
    pub ok: bool,
    pub issues: Vec<OperatorSdkReadinessIssue>,
}

impl OperatorSdkReadinessReport {
    fn from_issues(issues: Vec<OperatorSdkReadinessIssue>) -> Self {
        let ok = issues
            .iter()
            .all(|issue| issue.severity != OperatorSdkReadinessSeverity::Error);
        Self { ok, issues }
    }
}

pub fn operator_descriptor_readiness(
    descriptor: &OperatorDescriptor,
) -> OperatorSdkReadinessReport {
    let mut issues = Vec::new();
    check_required(
        &mut issues,
        &descriptor.id,
        "descriptor.id",
        "descriptor id must not be empty",
    );
    check_required(
        &mut issues,
        &descriptor.version,
        "descriptor.version",
        "descriptor version must not be empty",
    );
    check_required(
        &mut issues,
        &descriptor.domain,
        "descriptor.domain",
        "descriptor domain must not be empty",
    );
    check_required(
        &mut issues,
        &descriptor.family,
        "descriptor.family",
        "descriptor family must not be empty",
    );
    check_required(
        &mut issues,
        &descriptor.summary,
        "descriptor.summary",
        "descriptor summary must not be empty",
    );
    check_schema_ref(
        &mut issues,
        &descriptor.input_schema.schema,
        "input_schema.schema",
    );
    check_schema_ref(
        &mut issues,
        &descriptor.input_schema.version,
        "input_schema.version",
    );
    check_schema_ref(
        &mut issues,
        &descriptor.output_schema.schema,
        "output_schema.schema",
    );
    check_schema_ref(
        &mut issues,
        &descriptor.output_schema.version,
        "output_schema.version",
    );
    check_tags(&mut issues, descriptor);
    check_ports(&mut issues, &descriptor.inputs, "input");
    check_ports(&mut issues, &descriptor.outputs, "output");
    check_operator_flow_shape(&mut issues, descriptor);
    check_validation(&mut issues, descriptor);
    OperatorSdkReadinessReport::from_issues(issues)
}

pub fn operator_package_manifest_readiness(
    manifest: &OperatorPackageManifest,
) -> OperatorSdkReadinessReport {
    let mut issues = Vec::new();
    if manifest.schema_version != OPERATOR_PACKAGE_SCHEMA_VERSION {
        push_error(
            &mut issues,
            "manifest_schema_mismatch",
            "manifest.schema_version",
            format!(
                "expected schema_version {} but found {}",
                OPERATOR_PACKAGE_SCHEMA_VERSION, manifest.schema_version
            ),
        );
    }
    if manifest.sdk_api_version != OPERATOR_SDK_API_VERSION {
        push_error(
            &mut issues,
            "sdk_api_mismatch",
            "manifest.sdk_api_version",
            format!(
                "expected sdk_api_version {} but found {}",
                OPERATOR_SDK_API_VERSION, manifest.sdk_api_version
            ),
        );
    }
    check_required(
        &mut issues,
        &manifest.package_id,
        "manifest.package_id",
        "package_id must not be empty",
    );
    check_required(
        &mut issues,
        &manifest.package_version,
        "manifest.package_version",
        "package_version must not be empty",
    );
    check_required(
        &mut issues,
        &manifest.minimum_host_version,
        "manifest.minimum_host_version",
        "minimum_host_version must not be empty",
    );
    check_required(
        &mut issues,
        &manifest.validation_notes,
        "manifest.validation_notes",
        "validation_notes must not be empty",
    );
    check_required(
        &mut issues,
        &manifest.entrypoint,
        "manifest.entrypoint",
        "entrypoint must not be empty",
    );
    if manifest.runtime != "rust_crate" {
        push_error(
            &mut issues,
            "manifest_runtime_not_rust_crate",
            "manifest.runtime",
            "operator SDK packages must use runtime rust_crate".to_string(),
        );
    }
    if manifest.validation_status == OperatorValidationStatus::Unverified {
        push_warning(
            &mut issues,
            "manifest_unverified",
            "manifest.validation_status",
            "package validation_status is unverified".to_string(),
        );
    }
    check_manifest_operators(&mut issues, manifest);
    OperatorSdkReadinessReport::from_issues(issues)
}

pub fn operator_package_descriptor_readiness(
    manifest: &OperatorPackageManifest,
    descriptors: &[OperatorDescriptor],
) -> OperatorSdkReadinessReport {
    let mut issues = operator_package_manifest_readiness(manifest).issues;
    for descriptor in descriptors {
        issues.extend(operator_descriptor_readiness(descriptor).issues);
    }

    let descriptor_ids = descriptors
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<BTreeSet<_>>();
    let manifest_entries = manifest
        .operators
        .iter()
        .map(|entry| entry.operator_id.as_str())
        .collect::<BTreeSet<_>>();

    for operator_id in &manifest_entries {
        if !descriptor_ids.contains(operator_id) {
            push_error(
                &mut issues,
                "manifest_operator_missing_descriptor",
                format!("manifest.operator.{operator_id}"),
                "manifest operator has no matching descriptor".to_string(),
            );
        }
    }
    for descriptor_id in &descriptor_ids {
        if !manifest_entries.contains(descriptor_id) {
            push_warning(
                &mut issues,
                "descriptor_missing_manifest_operator",
                format!("descriptor.{descriptor_id}"),
                "descriptor is not listed in the package manifest".to_string(),
            );
        }
    }

    OperatorSdkReadinessReport::from_issues(issues)
}

fn check_required(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    value: &str,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    if value.trim().is_empty() {
        push_error(issues, "required_field_empty", subject, message);
    }
}

fn check_schema_ref(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    value: &str,
    subject: impl Into<String>,
) {
    if value.trim().is_empty() {
        push_error(
            issues,
            "schema_ref_empty",
            subject,
            "schema references must include schema and version".to_string(),
        );
    }
}

fn check_tags(issues: &mut Vec<OperatorSdkReadinessIssue>, descriptor: &OperatorDescriptor) {
    if descriptor.capability_tags.is_empty() {
        push_error(
            issues,
            "capability_tags_empty",
            format!("descriptor.{}", descriptor.id),
            "capability_tags must contain at least one stable tag".to_string(),
        );
        return;
    }

    let mut seen = BTreeSet::new();
    for tag in &descriptor.capability_tags {
        if tag.trim().is_empty() {
            push_error(
                issues,
                "capability_tag_empty",
                format!("descriptor.{}", descriptor.id),
                "capability_tags must not contain blank tags".to_string(),
            );
        } else if !seen.insert(tag) {
            push_warning(
                issues,
                "capability_tag_duplicate",
                format!("descriptor.{}", descriptor.id),
                format!("capability tag {tag} is duplicated"),
            );
        }
    }
}

fn check_ports(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    ports: &[OperatorPortDescriptor],
    direction: &'static str,
) {
    let mut seen = BTreeSet::new();
    for port in ports {
        let subject = format!("{direction}_port.{}", port.id);
        check_required(issues, &port.id, &subject, "port id must not be empty");
        check_required(
            issues,
            &port.artifact_type,
            &subject,
            "port artifact_type must not be empty",
        );
        check_required(
            issues,
            &port.description,
            &subject,
            "port description must not be empty",
        );
        if !port.id.trim().is_empty() && !seen.insert(port.id.as_str()) {
            push_error(
                issues,
                "port_id_duplicate",
                subject,
                format!("duplicate {direction} port id {}", port.id),
            );
        }
    }
}

fn check_operator_flow_shape(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    descriptor: &OperatorDescriptor,
) {
    if descriptor.outputs.is_empty() {
        push_error(
            issues,
            "operator_outputs_empty",
            format!("descriptor.{}", descriptor.id),
            "operators must expose at least one output port".to_string(),
        );
    }
    let requires_input = matches!(
        descriptor.kind,
        OperatorKind::Solver
            | OperatorKind::Transform
            | OperatorKind::Extract
            | OperatorKind::WorkflowBridge
    );
    if requires_input && descriptor.inputs.is_empty() {
        push_error(
            issues,
            "operator_inputs_empty",
            format!("descriptor.{}", descriptor.id),
            "solver, transform, extract, and workflow_bridge operators need at least one input port"
                .to_string(),
        );
    }
}

fn check_validation(issues: &mut Vec<OperatorSdkReadinessIssue>, descriptor: &OperatorDescriptor) {
    if descriptor.validation.baseline_cases.is_empty() {
        push_error(
            issues,
            "validation_baseline_cases_empty",
            format!("descriptor.{}", descriptor.id),
            "validation baseline_cases must contain at least one case id".to_string(),
        );
    }
    if descriptor.validation.smoke_paths.is_empty() {
        push_error(
            issues,
            "validation_smoke_paths_empty",
            format!("descriptor.{}", descriptor.id),
            "validation smoke_paths must contain at least one execution path".to_string(),
        );
    }
    if descriptor.validation.baseline_status == OperatorValidationStatus::Unverified {
        push_warning(
            issues,
            "validation_unverified",
            format!("descriptor.{}", descriptor.id),
            "descriptor baseline_status is unverified".to_string(),
        );
    }
}

fn check_manifest_operators(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    manifest: &OperatorPackageManifest,
) {
    if manifest.operators.is_empty() {
        push_error(
            issues,
            "manifest_operators_empty",
            "manifest.operators",
            "operators must contain at least one entry".to_string(),
        );
        return;
    }

    let mut seen = BTreeMap::new();
    for entry in &manifest.operators {
        let subject = format!("manifest.operator.{}", entry.operator_id);
        check_required(
            issues,
            &entry.operator_id,
            &subject,
            "operator_id must not be empty",
        );
        check_required(issues, &entry.kind, &subject, "kind must not be empty");
        check_required(
            issues,
            &entry.entry_symbol,
            &subject,
            "entry_symbol must not be empty",
        );
        if entry.operator_id.trim().is_empty() {
            continue;
        }
        if seen.insert(entry.operator_id.as_str(), true).is_some() {
            push_error(
                issues,
                "manifest_operator_duplicate",
                subject,
                format!("duplicate operator_id {}", entry.operator_id),
            );
        }
    }
}

fn push_error(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    code: &'static str,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(OperatorSdkReadinessIssue {
        severity: OperatorSdkReadinessSeverity::Error,
        code,
        subject: subject.into(),
        message: message.into(),
    });
}

fn push_warning(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    code: &'static str,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(OperatorSdkReadinessIssue {
        severity: OperatorSdkReadinessSeverity::Warning,
        code,
        subject: subject.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::{
        OperatorSdkReadinessSeverity, operator_descriptor_readiness,
        operator_package_descriptor_readiness, operator_package_manifest_readiness,
    };
    use crate::{
        OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION, OperatorDescriptorBuilder,
        OperatorPackageManifest, OperatorPackageOperatorEntry, operator_port,
        operator_port_with_dataset, verified_validation,
    };
    use kyuubiki_protocol::{OperatorKind, OperatorValidationStatus};

    #[test]
    fn accepts_ready_descriptor() {
        let descriptor = ready_descriptor();
        let report = operator_descriptor_readiness(&descriptor);
        assert!(report.ok, "{:?}", report.issues);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn rejects_descriptor_without_ports_and_validation_evidence() {
        let descriptor = OperatorDescriptorBuilder::new(
            "solve.incomplete",
            OperatorKind::Solver,
            "mechanical",
            "incomplete",
        )
        .summary("Incomplete solver used to prove readiness reporting.")
        .capability_tags(["mechanical"])
        .build();

        let report = operator_descriptor_readiness(&descriptor);
        assert!(!report.ok);
        assert!(has_issue(&report, "operator_inputs_empty"));
        assert!(has_issue(&report, "operator_outputs_empty"));
    }

    #[test]
    fn accepts_ready_manifest_and_descriptor_pair() {
        let descriptor = ready_descriptor();
        let manifest = ready_manifest();
        let report = operator_package_descriptor_readiness(&manifest, &[descriptor]);
        assert!(report.ok, "{:?}", report.issues);
    }

    #[test]
    fn reports_manifest_descriptor_mismatch() {
        let manifest = ready_manifest();
        let report = operator_package_descriptor_readiness(&manifest, &[]);
        assert!(!report.ok);
        assert!(has_issue(&report, "manifest_operator_missing_descriptor"));
    }

    #[test]
    fn warns_on_unverified_manifest_without_blocking_authoring() {
        let mut manifest = ready_manifest();
        manifest.validation_status = OperatorValidationStatus::Unverified;
        let report = operator_package_manifest_readiness(&manifest);
        assert!(report.ok);
        assert_eq!(
            report.issues[0].severity,
            OperatorSdkReadinessSeverity::Warning
        );
        assert!(has_issue(&report, "manifest_unverified"));
    }

    fn ready_descriptor() -> kyuubiki_protocol::OperatorDescriptor {
        OperatorDescriptorBuilder::new(
            "extract.temperature_peak",
            OperatorKind::Extract,
            "thermal",
            "temperature_peak",
        )
        .summary("Extract peak temperature from a thermal result field.")
        .capability_tags(["thermal", "postprocess"])
        .input_port(operator_port_with_dataset(
            "result",
            "result/thermal_field",
            "Thermal result field",
            "thermal_field",
        ))
        .output_port(operator_port(
            "summary",
            "artifact/json",
            "Peak-temperature summary",
        ))
        .validation(verified_validation("temperature_peak_baseline"))
        .build()
    }

    fn ready_manifest() -> OperatorPackageManifest {
        OperatorPackageManifest {
            schema_version: OPERATOR_PACKAGE_SCHEMA_VERSION.to_string(),
            sdk_api_version: OPERATOR_SDK_API_VERSION.to_string(),
            package_id: "operator.example.temperature_peak".to_string(),
            package_version: "0.1.0".to_string(),
            minimum_host_version: "1.15.0".to_string(),
            validation_status: OperatorValidationStatus::Partial,
            validation_notes: "Readiness test fixture.".to_string(),
            runtime: "rust_crate".to_string(),
            entrypoint: "target/debug/liboperator_example_temperature_peak.dylib".to_string(),
            operators: vec![OperatorPackageOperatorEntry {
                operator_id: "extract.temperature_peak".to_string(),
                kind: "extract".to_string(),
                entry_symbol: "register_operator".to_string(),
            }],
        }
    }

    fn has_issue(report: &super::OperatorSdkReadinessReport, code: &str) -> bool {
        report.issues.iter().any(|issue| issue.code == code)
    }
}
