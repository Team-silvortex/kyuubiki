mod quality_terms;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use quality_terms::{
    compare_quality_signature_coverage, compare_quality_term_signatures, compare_quality_terms,
    rust_quality_term_signatures, rust_quality_term_signatures_from_source, rust_quality_terms,
    rust_quality_terms_from_source, web_quality_term_signatures, web_quality_terms,
};

type RunnerResult<T> = Result<T, String>;

const RUST_RESOLVER_PATH: &str = "workers/rust/crates/engine/src/workflow_metric_resolver.rs";
const WEB_RESOLVER_PATH: &str = "apps/web/lib/kyuubiki_web/workflow_domain_metric_resolver.ex";
const WEB_QUALITY_RUNTIME_PATH: &str =
    "apps/web/lib/kyuubiki_web/workflow_domain_quality_runtime.ex";
const WEB_RESOLVER_TEST_PATH: &str =
    "apps/web/test/kyuubiki_web/workflow_domain_metric_resolver_test.exs";
const WEB_QUALITY_RUNTIME_TEST_PATH: &str =
    "apps/web/test/kyuubiki_web/workflow_domain_quality_runtime_test.exs";

const RUST_QUALITY_SOURCES: &[(&str, &str)] = &[
    (
        "dynamic",
        "workers/rust/crates/engine/src/dynamic_quality.rs",
    ),
    (
        "structural",
        "workers/rust/crates/engine/src/structural_quality.rs",
    ),
    (
        "thermal",
        "workers/rust/crates/engine/src/thermal_quality.rs",
    ),
    (
        "electrostatic",
        "workers/rust/crates/engine/src/electrostatic_quality.rs",
    ),
    (
        "magnetostatic",
        "workers/rust/crates/engine/src/magnetostatic_quality.rs",
    ),
    (
        "acoustic",
        "workers/rust/crates/engine/src/acoustic_quality.rs",
    ),
    ("modal", "workers/rust/crates/engine/src/modal_quality.rs"),
    (
        "transport",
        "workers/rust/crates/engine/src/transport_quality.rs",
    ),
    ("cfd", "workers/rust/crates/engine/src/cfd_diagnostics.rs"),
];

const REQUIRED_CONTRACT_FIELDS: &[&str] = &[
    "peak_frequency_hz",
    "max_displacement",
    "max_velocity",
    "max_acceleration",
    "max_force",
    "max_stress",
    "mass",
    "stiffness_margin",
    "thermal_temperature_max",
    "thermal_flux_peak_magnitude",
    "thermo_temperature_delta_max",
    "thermo_stress_peak",
    "thermal_total_energy",
    "electrostatic_field_peak_magnitude",
    "electrostatic_peak_energy_density",
    "electrostatic_flux_peak_magnitude",
    "electrostatic_total_stored_energy",
    "electrostatic_potential_span",
    "magnetostatic_field_peak_magnitude",
    "magnetostatic_flux_peak_magnitude",
    "magnetostatic_energy_density_peak",
    "magnetostatic_current_density_sum",
    "magnetostatic_total_stored_energy",
    "max_sound_pressure_level_db",
    "max_acoustic_intensity",
    "max_pressure_amplitude",
    "total_damping_loss",
    "min_frequency_hz",
    "max_frequency_hz",
    "total_mass",
    "frequency_span_hz",
    "mode_1_participation_norm",
    "cfd_divergence_error_peak",
    "cfd_reynolds_number_peak",
    "cfd_viscous_dissipation_total",
    "cfd_velocity_span",
    "cfd_pressure_span",
    "velocity_magnitude",
    "pressure",
    "divergence_error",
    "reynolds_number",
    "viscous_dissipation",
    "transport_total_flux_peak_magnitude",
    "transport_peclet_peak",
    "transport_concentration_span",
    "transport_source_sum",
];

pub(crate) fn run_check_workflow_metric_resolver_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("workflow metric resolver contract self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-workflow-metric-resolver-contract only accepts --self-test".to_string());
    }

    let summary = check_contract(root)?;
    if let Some(issue) = summary.issues.first() {
        eprintln!("workflow metric resolver contract failed: {issue}");
        return Ok(1);
    }
    println!(
        "workflow metric resolver contract passed: {} shared field(s), {} quality mirror(s), {} quality term(s), {} quality signature(s)",
        summary.shared_field_count,
        summary.quality_mirror_count,
        summary.quality_term_count,
        summary.quality_signature_count
    );
    Ok(0)
}

struct ContractSummary {
    shared_field_count: usize,
    quality_mirror_count: usize,
    quality_term_count: usize,
    quality_signature_count: usize,
    issues: Vec<String>,
}

fn check_contract(root: &Path) -> RunnerResult<ContractSummary> {
    let rust_source = read_text(root, RUST_RESOLVER_PATH)?;
    let web_source = read_text(root, WEB_RESOLVER_PATH)?;
    let web_quality_source = read_text(root, WEB_QUALITY_RUNTIME_PATH)?;
    let web_test_source = read_text(root, WEB_RESOLVER_TEST_PATH)?;
    let web_quality_test_source = read_text(root, WEB_QUALITY_RUNTIME_TEST_PATH)?;
    let rust_fields = rust_metric_fields(&rust_source);
    let web_fields = web_metric_fields(&web_source);
    let mut issues = compare_metric_fields(&rust_fields, &web_fields);
    let rust_quality_mirrors = rust_quality_mirrors(root)?;
    let web_quality_mirrors = web_quality_mirrors(&web_quality_source);
    issues.extend(compare_quality_mirrors(
        &rust_quality_mirrors,
        &web_quality_mirrors,
    ));
    let rust_quality_terms = rust_quality_terms(root)?;
    let web_quality_terms = web_quality_terms(&web_quality_source);
    issues.extend(compare_quality_terms(
        &rust_quality_terms,
        &web_quality_terms,
    ));
    let rust_quality_signatures = rust_quality_term_signatures(root)?;
    let web_quality_signatures = web_quality_term_signatures(&web_quality_source);
    issues.extend(compare_quality_signature_coverage(
        "Rust",
        &rust_quality_terms,
        &rust_quality_signatures,
    ));
    issues.extend(compare_quality_signature_coverage(
        "Web",
        &web_quality_terms,
        &web_quality_signatures,
    ));
    issues.extend(compare_quality_term_signatures(
        &rust_quality_signatures,
        &web_quality_signatures,
    ));
    check_required_fields("Rust resolver", &rust_fields, &mut issues);
    check_required_fields("Web resolver", &web_fields, &mut issues);
    check_test_anchors(
        &rust_source,
        RUST_RESOLVER_PATH,
        &[
            "direct_metric_value_takes_precedence_over_aliases",
            "resolves_domain_aliases_and_bounds_derived_spans",
            "resolves_modal_modes_cfd_values_and_generic_spans",
        ],
        &mut issues,
    );
    check_test_anchors(
        &web_test_source,
        WEB_RESOLVER_TEST_PATH,
        &[
            "direct metric values take precedence over aliases",
            "resolves domain aliases and bounds-derived spans",
            "resolves modal modes, CFD values, and generic spans",
        ],
        &mut issues,
    );
    check_test_anchors(
        &web_quality_test_source,
        WEB_QUALITY_RUNTIME_TEST_PATH,
        &["scores optional energy terms and emits domain metric mirrors"],
        &mut issues,
    );
    let shared_field_count = rust_fields.intersection(&web_fields).count();
    let quality_mirror_count = rust_quality_mirrors
        .intersection(&web_quality_mirrors)
        .count();
    let quality_term_count = rust_quality_terms.intersection(&web_quality_terms).count();
    let quality_signature_count = rust_quality_signatures
        .keys()
        .filter(|key| web_quality_signatures.contains_key(*key))
        .count();
    Ok(ContractSummary {
        shared_field_count,
        quality_mirror_count,
        quality_term_count,
        quality_signature_count,
        issues,
    })
}

fn rust_metric_fields(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(first_quoted_string_with_tail)
        .filter_map(|(value, tail)| tail.trim_start().starts_with("=>").then_some(value))
        .collect()
}

fn web_metric_fields(source: &str) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("@dynamic_amplitude_fields") {
            fields.extend(tilde_word_list(trimmed));
        }
        if web_metric_function_head(trimmed) {
            if let Some((value, _tail)) = first_quoted_string_with_tail(trimmed) {
                fields.insert(value);
            }
        }
    }
    fields
}

fn web_metric_function_head(line: &str) -> bool {
    [
        "def metric_value(",
        "defp dynamic_alias_value(",
        "defp domain_alias_value(",
        "defp derived_dynamic_value(",
        "defp derived_domain_value(",
        "defp frequency_entry_number(",
        "defp transient_node_field(",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn compare_metric_fields(rust: &BTreeSet<String>, web: &BTreeSet<String>) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push(format!(
            "{RUST_RESOLVER_PATH}: no resolver fields extracted"
        ));
    }
    if web.is_empty() {
        issues.push(format!("{WEB_RESOLVER_PATH}: no resolver fields extracted"));
    }
    let missing_from_web = rust.difference(web).cloned().collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "missing from Web resolver: {}",
            missing_from_web.join(", ")
        ));
    }
    let missing_from_rust = web.difference(rust).cloned().collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "missing from Rust resolver: {}",
            missing_from_rust.join(", ")
        ));
    }
    issues
}

fn rust_quality_mirrors(root: &Path) -> RunnerResult<BTreeSet<(String, String)>> {
    let mut mirrors = BTreeSet::new();
    for (_domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        mirrors.extend(rust_quality_mirrors_from_source(&source));
    }
    Ok(mirrors)
}

fn rust_quality_mirrors_from_source(source: &str) -> BTreeSet<(String, String)> {
    source
        .lines()
        .filter(|line| {
            line.contains("numeric_field(object,") || line.contains("metric_value(object,")
        })
        .filter_map(|line| {
            let (output_field, tail) = first_quoted_string_with_tail(line)?;
            if !output_field.contains("_quality_") {
                return None;
            }
            let (metric_field, _tail) = first_quoted_string_with_tail(tail)?;
            Some((output_field, metric_field))
        })
        .collect()
}

fn web_quality_mirrors(source: &str) -> BTreeSet<(String, String)> {
    source
        .lines()
        .filter(|line| line.trim_start().starts_with("{\""))
        .filter_map(two_quoted_strings)
        .filter(|(output_field, _metric_field)| output_field.contains("_quality_"))
        .collect()
}

fn compare_quality_mirrors(
    rust: &BTreeSet<(String, String)>,
    web: &BTreeSet<(String, String)>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality mirror contract extracted no fields".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality mirror contract extracted no fields".to_string());
    }
    let missing_from_web = rust.difference(web).cloned().collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality mirrors missing from Web runtime: {}",
            format_mirror_pairs(&missing_from_web)
        ));
    }
    let missing_from_rust = web.difference(rust).cloned().collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality mirrors missing from Rust engine: {}",
            format_mirror_pairs(&missing_from_rust)
        ));
    }
    issues
}

fn format_mirror_pairs(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(output_field, metric_field)| format!("{output_field}->{metric_field}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn check_required_fields(label: &str, fields: &BTreeSet<String>, issues: &mut Vec<String>) {
    for field in REQUIRED_CONTRACT_FIELDS {
        if !fields.contains(*field) {
            issues.push(format!("{label}: missing required contract field {field}"));
        }
    }
}

fn check_test_anchors(source: &str, path: &str, anchors: &[&str], issues: &mut Vec<String>) {
    for anchor in anchors {
        if !source.contains(anchor) {
            issues.push(format!("{path}: missing resolver test anchor {anchor}"));
        }
    }
}

fn first_quoted_string_with_tail(line: &str) -> Option<(String, &str)> {
    let start = line.find('"')?;
    let after_start = &line[start + 1..];
    let end = after_start.find('"')?;
    Some((after_start[..end].to_string(), &after_start[end + 1..]))
}

fn two_quoted_strings(line: &str) -> Option<(String, String)> {
    let (first, tail) = first_quoted_string_with_tail(line)?;
    let (second, _tail) = first_quoted_string_with_tail(tail)?;
    Some((first, second))
}

fn tilde_word_list(line: &str) -> Vec<String> {
    let Some(start) = line.find("~w(") else {
        return Vec::new();
    };
    let after_start = &line[start + 3..];
    let Some(end) = after_start.find(')') else {
        return Vec::new();
    };
    after_start[..end]
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative)).map_err(|error| format!("{relative}: {error}"))
}

fn run_self_test() -> RunnerResult<()> {
    let rust = r#"
fn domain_alias_field(field: &str) {
    match field {
        "max_stress" => Some(1.0),
        "max_velocity" => Some(2.0),
        "velocity_magnitude" => Some(3.0),
        "frequency_span_hz" => Some(4.0),
        _ => None,
    }
}
"#;
    let web = r#"
@dynamic_amplitude_fields ~w(max_velocity)
def metric_value(payload, "frequency_span_hz"), do: payload
defp domain_alias_value(payload, "max_stress"), do: payload
defp domain_alias_value(payload, "velocity_magnitude"), do: payload
"#;
    let rust_fields = rust_metric_fields(rust);
    let web_fields = web_metric_fields(web);
    let issues = compare_metric_fields(&rust_fields, &web_fields);
    if !issues.is_empty() {
        return Err(format!("self-test matching fixtures drifted: {issues:?}"));
    }

    let missing_web_fields = web.replace(
        "defp domain_alias_value(payload, \"max_stress\"), do: payload",
        "",
    );
    expect_issue(
        compare_metric_fields(&rust_fields, &web_metric_fields(&missing_web_fields)),
        "missing from Web resolver: max_stress",
    )?;

    let extra_web_field =
        format!("{web}\ndefp domain_alias_value(payload, \"extra_metric\"), do: payload\n");
    expect_issue(
        compare_metric_fields(&rust_fields, &web_metric_fields(&extra_web_field)),
        "missing from Rust resolver: extra_metric",
    )?;

    let rust_mirror = r#"
        "thermal_quality_total_energy": numeric_field(object, "thermal_total_energy"),
        "cfd_quality_velocity_span": metric_value(object, "cfd_velocity_span"),
"#;
    let web_mirror = r#"
      {"thermal_quality_total_energy", "thermal_total_energy"},
      {"cfd_quality_velocity_span", "cfd_velocity_span"}
"#;
    let rust_mirrors = rust_quality_mirrors_from_source(rust_mirror);
    let web_mirrors = web_quality_mirrors(web_mirror);
    let issues = compare_quality_mirrors(&rust_mirrors, &web_mirrors);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching mirror fixtures drifted: {issues:?}"
        ));
    }

    let missing_web_mirror = web_mirror.replace(
        "{\"thermal_quality_total_energy\", \"thermal_total_energy\"},",
        "",
    );
    expect_issue(
        compare_quality_mirrors(&rust_mirrors, &web_quality_mirrors(&missing_web_mirror)),
        "quality mirrors missing from Web runtime: thermal_quality_total_energy->thermal_total_energy",
    )?;

    let extra_web_mirror =
        format!("{web_mirror}\n      {{\"extra_quality_metric\", \"extra_metric\"}}\n");
    expect_issue(
        compare_quality_mirrors(&rust_mirrors, &web_quality_mirrors(&extra_web_mirror)),
        "quality mirrors missing from Rust engine: extra_quality_metric->extra_metric",
    )?;

    let rust_structural_term_source = r#"
        QualityTerm {
            field: "max_stress",
            label: "Maximum stress",
            target: 1.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }
"#;
    let rust_thermal_term_source = r#"
        "thermal_total_energy" => Some(QualityTerm {
            field: "thermal_total_energy",
            label: "Total thermal energy",
            target: 1.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }),
"#;
    let web_term_source = r#"
@domains %{
  "transform.score_structural_quality" => %{
    id: "structural",
    terms: [
      {"max_stress", "Maximum stress", 1.0, 1.0, :min}
    ]
  }
}
def supported_operator_ids, do: Map.keys(@domains)
defp extra_quality_term("thermal", "thermal_total_energy"),
  do: {"thermal_total_energy", "Total thermal energy", 1.0, 1.0, :min}
"#;
    let mut rust_terms = rust_quality_terms_from_source("structural", rust_structural_term_source);
    rust_terms.extend(rust_quality_terms_from_source(
        "thermal",
        rust_thermal_term_source,
    ));
    let web_terms = web_quality_terms(web_term_source);
    let issues = compare_quality_terms(&rust_terms, &web_terms);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality term fixtures drifted: {issues:?}"
        ));
    }

    let missing_web_term = web_term_source.replace(
        "defp extra_quality_term(\"thermal\", \"thermal_total_energy\"),",
        "defp extra_quality_term(\"thermal\", \"unused_term\"),",
    );
    expect_issue(
        compare_quality_terms(&rust_terms, &web_quality_terms(&missing_web_term)),
        "quality score terms missing from Web runtime: thermal:thermal_total_energy",
    )?;

    let extra_web_term = format!(
        "{web_term_source}\ndefp extra_quality_term(\"cfd\", \"cfd_extra_metric\"), do: {{\"cfd_extra_metric\", \"Extra\", 1.0, 1.0, :min}}\n"
    );
    expect_issue(
        compare_quality_terms(&rust_terms, &web_quality_terms(&extra_web_term)),
        "quality score terms missing from Rust engine: cfd:cfd_extra_metric",
    )?;

    let mut rust_signatures =
        rust_quality_term_signatures_from_source("structural", rust_structural_term_source);
    rust_signatures.extend(rust_quality_term_signatures_from_source(
        "thermal",
        rust_thermal_term_source,
    ));
    let web_signatures = web_quality_term_signatures(web_term_source);
    let issues = compare_quality_term_signatures(&rust_signatures, &web_signatures);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality signature fixtures drifted: {issues:?}"
        ));
    }

    let drifted_web_signature = web_term_source.replace(
        "{\"max_stress\", \"Maximum stress\", 1.0, 1.0, :min}",
        "{\"max_stress\", \"Maximum stress\", 2.0, 1.0, :min}",
    );
    expect_issue(
        compare_quality_term_signatures(
            &rust_signatures,
            &web_quality_term_signatures(&drifted_web_signature),
        ),
        "quality score term signature drift for structural:max_stress",
    )
}

fn expect_issue(issues: Vec<String>, expected: &str) -> RunnerResult<()> {
    if issues.iter().any(|issue| issue.contains(expected)) {
        Ok(())
    } else {
        Err(format!(
            "self-test expected issue containing {expected:?}, got {issues:?}"
        ))
    }
}
