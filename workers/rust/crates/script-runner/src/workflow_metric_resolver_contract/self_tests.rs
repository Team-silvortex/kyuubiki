use super::quality_payload::{
    compare_quality_payload_keys, compare_quality_term_entry_schemas,
    rust_quality_payload_keys_from_source, rust_quality_term_entry_schemas_from_source,
    web_quality_payload_keys, web_quality_term_entry_schema,
};
use super::quality_terms::{
    compare_quality_term_signatures, compare_quality_terms,
    rust_quality_term_signatures_from_source, rust_quality_terms_from_source,
    web_quality_term_signatures, web_quality_terms,
};
use super::{
    RunnerResult, compare_metric_fields, compare_quality_mirrors, rust_metric_fields,
    rust_quality_mirrors_from_source, web_metric_fields, web_quality_mirrors,
};

pub(super) fn run_self_test() -> RunnerResult<()> {
    super::quality_domains::run_self_test()?;
    check_resolver_fields()?;
    check_quality_mirrors()?;
    check_quality_terms_and_signatures()?;
    check_quality_payload_keys()?;
    check_quality_term_entry_schemas()
}

fn check_resolver_fields() -> RunnerResult<()> {
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
    )
}

fn check_quality_mirrors() -> RunnerResult<()> {
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
    )
}

fn check_quality_terms_and_signatures() -> RunnerResult<()> {
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

fn check_quality_payload_keys() -> RunnerResult<()> {
    let rust_payload_source = r#"
        "dynamic_quality_contract": "kyuubiki.dynamic_quality_score/v1",
        "dynamic_quality_score": score,
        "dynamic_quality_peak_frequency_hz": metric_value(object, "peak_frequency_hz"),
"#;
    let web_payload_source = r##"
@domains %{
  "transform.score_dynamic_quality" => %{
    id: "dynamic",
    terms: []
  }
}
def supported_operator_ids, do: Map.keys(@domains)
"#{id}_quality_contract" => "kyuubiki.#{id}_quality_score/v1",
"#{id}_quality_score" => score,
{"dynamic_quality_peak_frequency_hz", "peak_frequency_hz"}
"##;
    let rust_payload_keys = rust_quality_payload_keys_from_source(rust_payload_source);
    let web_payload_keys = web_quality_payload_keys(web_payload_source);
    let issues = compare_quality_payload_keys(&rust_payload_keys, &web_payload_keys);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality payload fixtures drifted: {issues:?}"
        ));
    }

    let missing_web_payload = web_payload_source.replace(
        "{\"dynamic_quality_peak_frequency_hz\", \"peak_frequency_hz\"}",
        "",
    );
    expect_issue(
        compare_quality_payload_keys(
            &rust_payload_keys,
            &web_quality_payload_keys(&missing_web_payload),
        ),
        "quality payload keys missing from Web runtime: dynamic_quality_peak_frequency_hz",
    )
}

fn check_quality_term_entry_schemas() -> RunnerResult<()> {
    let rust_term_entry_source = r#"
fn score_quality_term() {
    serde_json::json!({
        "field": term.field,
        "label": term.label,
        "value": value,
        "target": target,
        "weight": weight,
        "goal": "min",
        "penalty": penalty,
        "status": "ok",
    })
    serde_json::json!({
        "field": term.field,
        "label": term.label,
        "target": target,
        "weight": weight,
        "penalty": 0.0,
        "status": "missing",
    })
}
fn compact_quality_term() {
    serde_json::json!({
        "field": term.get("field"),
        "label": term.get("label"),
        "status": term.get("status"),
        "penalty": term.get("penalty"),
    })
}
"#;
    let web_term_entry_source = r#"
defp score_term(payload, config, term) do
  %{
    "field" => field,
    "label" => label,
    "value" => value,
    "target" => target,
    "weight" => weight,
    "goal" => "min",
    "penalty" => penalty,
    "status" => "ok"
  }

  %{
    "field" => field,
    "label" => label,
    "target" => target,
    "weight" => weight,
    "penalty" => 0.0,
    "status" => "missing"
  }
end

defp compact_quality_term(term) do
  %{
    "field" => Map.get(term, "field"),
    "label" => Map.get(term, "label"),
    "status" => Map.get(term, "status"),
    "penalty" => Map.get(term, "penalty")
  }
end
"#;
    let rust_term_entry_schemas =
        rust_quality_term_entry_schemas_from_source("cfd", rust_term_entry_source);
    let web_term_entry_schema = web_quality_term_entry_schema(web_term_entry_source);
    let issues =
        compare_quality_term_entry_schemas(&rust_term_entry_schemas, &web_term_entry_schema);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality term entry fixtures drifted: {issues:?}"
        ));
    }

    let missing_rust_goal = rust_term_entry_source.replace("        \"goal\": \"min\",\n", "");
    expect_issue(
        compare_quality_term_entry_schemas(
            &rust_quality_term_entry_schemas_from_source("cfd", &missing_rust_goal),
            &web_term_entry_schema,
        ),
        "quality term entry keys missing from Rust cfd:present: goal",
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
