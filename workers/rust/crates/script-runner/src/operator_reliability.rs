use crate::operator_qualification_evidence_kits::load_qualification_evidence_kits;
use crate::operator_reliability_rules::{
    is_below_minimum_coverage_level, qualification_evidence_errors,
    qualification_evidence_kit_errors, qualification_roadmap_errors,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

mod extract;
mod release_records;

use extract::{physics_coverage_templates, workflow_operator_ids};
use release_records::validate_qualification_release_records;

const MANIFEST_PATH: &str = "config/operator-reliability-manifest.json";
const ROADMAP_PATH: &str = "config/operator-qualification-roadmap.json";
const SHARD_SCHEMA_VERSION: &str = "kyuubiki.operator-reliability-shard/v1";
const MANIFEST_SCHEMA_VERSION: &str = "kyuubiki.operator-reliability-manifest/v1";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_operator_reliability(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner check-operator-reliability");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-operator-reliability does not accept arguments".to_string());
    }
    match validate(root) {
        Ok(summary) => {
            println!("{summary}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("operator reliability check failed: {issue}");
            Ok(1)
        }
    }
}

fn validate(root: &Path) -> RunnerResult<String> {
    let manifest = load_manifest(root)?;
    require(
        field(&manifest, "schema_version") == MANIFEST_SCHEMA_VERSION,
        "unexpected schema_version",
    )?;
    require(
        field(&manifest, "coverage_matrix") == "physics-coverage",
        "coverage_matrix must be physics-coverage",
    )?;
    let allowed = ["smoke", "baseline", "review", "qualification"];
    for level in string_array(manifest.get("levels")) {
        require(allowed.contains(&level), &format!("unknown level {level}"))?;
    }
    let minimum = field(&manifest, "minimum_coverage_level");
    require(
        allowed.contains(&minimum),
        &format!("unknown minimum_coverage_level {minimum}"),
    )?;

    let expected_templates = physics_coverage_templates(root)?;
    let workflow_ids = workflow_operator_ids(root)?;
    let mut seen_templates = HashSet::new();
    let mut seen_operators = HashSet::new();
    let mut operator_levels = HashMap::new();
    let mut level_counts: HashMap<String, usize> = HashMap::new();

    for entry in array(&manifest, "operators") {
        let context = context(entry);
        let level = field(entry, "coverage_level");
        require(
            allowed.contains(&level),
            &format!("{context}: unknown coverage_level {level}"),
        )?;
        require(
            !is_below_minimum_coverage_level(level, minimum),
            &format!("{context}: coverage_level {level} is below manifest minimum {minimum}"),
        )?;
        require(
            field(value(entry, "evidence"), "benchmark_matrix")
                == field(&manifest, "coverage_matrix"),
            &format!("{context}: benchmark_matrix must match manifest coverage_matrix"),
        )?;
        require(
            value(entry, "evidence")
                .get("headless_workflow")
                .and_then(Value::as_bool)
                == Some(true),
            &format!("{context}: headless_workflow must be true for physics coverage"),
        )?;
        require(
            expected_templates.contains(field(entry, "benchmark_template")),
            &format!("{context}: benchmark_template is not in physics-coverage"),
        )?;
        require(
            workflow_ids.contains(field(entry, "operator_id")),
            &format!("{context}: operator_id is not exported by workflow_payloads"),
        )?;
        require(
            seen_templates.insert(field(entry, "benchmark_template").to_string()),
            &format!(
                "{context}: duplicate benchmark_template {}",
                field(entry, "benchmark_template")
            ),
        )?;
        require(
            seen_operators.insert(field(entry, "operator_id").to_string()),
            &format!("{context}: duplicate operator_id"),
        )?;
        operator_levels.insert(field(entry, "operator_id").to_string(), level.to_string());
        *level_counts.entry(level.to_string()).or_default() += 1;
        validate_entry_evidence(root, entry, &context)?;
    }

    let missing = expected_templates
        .iter()
        .filter(|template| !seen_templates.contains(*template))
        .cloned()
        .collect::<Vec<_>>();
    require(
        missing.is_empty(),
        &format!(
            "physics-coverage templates missing from manifest: {}",
            missing.join(", ")
        ),
    )?;
    validate_roadmap(root, &manifest, &seen_operators, &operator_levels)?;
    Ok(format!(
        "operator reliability manifest ok: {} operators, {}",
        array(&manifest, "operators").len(),
        ["smoke", "baseline", "review", "qualification"]
            .iter()
            .filter_map(|level| level_counts
                .get(*level)
                .map(|count| format!("{level}={count}")))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn load_manifest(root: &Path) -> RunnerResult<Value> {
    let manifest = read_json(root, MANIFEST_PATH)?;
    if manifest
        .get("operators")
        .and_then(Value::as_array)
        .is_some()
    {
        return Ok(manifest);
    }
    let shards = string_array(manifest.get("shards"));
    require(
        !shards.is_empty(),
        "manifest must declare operators or non-empty shards",
    )?;
    let mut operators = Vec::new();
    let mut seen = HashSet::new();
    for shard_path in shards {
        require(
            seen.insert(shard_path.to_string()),
            &format!("duplicate shard path {shard_path}"),
        )?;
        let shard = read_json(root, shard_path)?;
        require(
            field(&shard, "schema_version") == SHARD_SCHEMA_VERSION,
            &format!("{shard_path}: unexpected shard schema_version"),
        )?;
        require(
            !field(&shard, "domain").is_empty(),
            &format!("{shard_path}: missing domain"),
        )?;
        require(
            !array(&shard, "operators").is_empty(),
            &format!("{shard_path}: operators must be non-empty"),
        )?;
        for entry in array(&shard, "operators") {
            require(
                field(entry, "domain") == field(&shard, "domain"),
                &format!(
                    "{shard_path}: {} domain must match shard domain",
                    field(entry, "operator_id")
                ),
            )?;
            operators.push(entry.clone());
        }
    }
    let mut next = manifest.as_object().cloned().unwrap_or_default();
    next.insert("operators".to_string(), Value::Array(operators));
    Ok(Value::Object(next))
}

fn validate_entry_evidence(root: &Path, entry: &Value, context: &str) -> RunnerResult<()> {
    for path in string_array(value(entry, "evidence").get("tests")) {
        ensure_file_contains(root, path, None, context)?;
    }
    if !field(value(entry, "evidence"), "accuracy_baseline").is_empty() {
        let tests = string_array(value(entry, "evidence").get("tests"));
        ensure_file_contains(
            root,
            tests.first().copied().unwrap_or_default(),
            Some(field(value(entry, "evidence"), "accuracy_baseline")),
            context,
        )?;
    }
    if matches!(field(entry, "coverage_level"), "review" | "qualification") {
        validate_review(root, entry, context)?;
    }
    if field(entry, "coverage_level") == "qualification" {
        first_error(qualification_evidence_errors(entry), context)?;
        for path in string_array(value(value(entry, "evidence"), "qualification").get("tests")) {
            ensure_file_contains(root, path, None, context)?;
        }
    }
    require(
        !array(entry, "limits").is_empty(),
        &format!("{context}: limits must be non-empty"),
    )
}

fn validate_review(root: &Path, entry: &Value, context: &str) -> RunnerResult<()> {
    let review = value(value(entry, "evidence"), "review");
    require(
        review.is_object(),
        &format!("{context}: review-level operators must declare evidence.review"),
    )?;
    for field_name in ["assumptions", "boundary_checks", "diagnostics", "tests"] {
        require(
            !array(review, field_name).is_empty(),
            &format!("{context}: evidence.review.{field_name} must be non-empty"),
        )?;
    }
    for path in string_array(review.get("tests")) {
        ensure_file_contains(root, path, None, context)?;
    }
    validate_stokes(root, entry, review, context)?;
    validate_plane(root, entry, review, context)
}

fn validate_stokes(root: &Path, entry: &Value, review: &Value, context: &str) -> RunnerResult<()> {
    if !["solve.stokes_flow_quad_2d", "solve.stokes_flow_triangle_2d"]
        .contains(&field(entry, "operator_id"))
    {
        return Ok(());
    }
    validate_refs(
        root,
        review,
        "scope_notes",
        &[
            "CFD Stokes Screening Scope",
            "Stokes-only",
            "screening",
            "Navier-Stokes",
        ],
        context,
    )?;
    validate_refs_filtered(
        root,
        review,
        "tolerance_notes",
        |reference| reference.ends_with(".md#cfd-stokes-divergence-tolerance"),
        &["CFD Stokes Divergence Tolerance", "1e-10", "divergence"],
        context,
    )?;
    validate_refs_filtered(
        root,
        review,
        "tolerance_notes",
        |reference| reference.ends_with(".json"),
        &[
            "stokes_screening_divergence",
            "1e-10",
            "Navier-Stokes",
            "mesh-convergence",
        ],
        context,
    )?;
    for limit in [
        "stokes_only",
        "screening_only",
        "screening_divergence_tolerance_1e-10",
    ] {
        require(
            string_array(entry.get("limits")).contains(&limit),
            &format!("{context}: limits must include {limit}"),
        )?;
    }
    Ok(())
}

fn validate_plane(root: &Path, entry: &Value, review: &Value, context: &str) -> RunnerResult<()> {
    let operator = field(entry, "operator_id");
    if operator.contains("electrostatic_plane") || operator.contains("magnetostatic_plane") {
        validate_refs(
            root,
            review,
            "scope_notes",
            &[
                "Electromagnetic Plane Review Scope",
                "single-patch",
                "orientation",
                "qualification",
            ],
            context,
        )?;
        validate_refs(
            root,
            review,
            "material_notes",
            &[
                "Electromagnetic Plane Material",
                "linear material",
                "permittivity",
                "permeability",
                "stored energy",
            ],
            context,
        )?;
        for limit in ["linear_material", "two_dimensional"] {
            require(
                string_array(entry.get("limits")).contains(&limit),
                &format!("{context}: limits must include {limit}"),
            )?;
        }
    }
    if operator.contains("thermal_plane") || operator.contains("heat_plane") {
        validate_refs(
            root,
            review,
            "scope_notes",
            &[
                "Thermal Plane Review Scope",
                "mesh convergence",
                "boundary coverage",
                "qualification",
            ],
            context,
        )?;
        validate_refs(
            root,
            review,
            "material_notes",
            &[
                "Thermal Plane Material",
                "linear",
                "conductivity",
                "thermal expansion",
                "material-card",
            ],
            context,
        )?;
        let required = if field(entry, "domain") == "thermal" {
            vec!["steady_state", "linear_conductivity"]
        } else {
            vec!["linear_plane_stress"]
        };
        for limit in required {
            require(
                string_array(entry.get("limits")).contains(&limit),
                &format!("{context}: limits must include {limit}"),
            )?;
        }
    }
    Ok(())
}

fn validate_refs(
    root: &Path,
    review: &Value,
    field_name: &str,
    needles: &[&str],
    context: &str,
) -> RunnerResult<()> {
    let refs = string_array(review.get(field_name));
    require(
        !refs.is_empty(),
        &format!("{context}: evidence.review.{field_name} must be non-empty"),
    )?;
    for reference in refs {
        let path = reference.split('#').next().unwrap_or(reference);
        for needle in needles {
            ensure_file_contains(root, path, Some(needle), context)?;
        }
    }
    Ok(())
}

fn validate_refs_filtered(
    root: &Path,
    review: &Value,
    field_name: &str,
    predicate: impl Fn(&str) -> bool,
    needles: &[&str],
    context: &str,
) -> RunnerResult<()> {
    let refs = string_array(review.get(field_name))
        .into_iter()
        .filter(|reference| predicate(reference))
        .collect::<Vec<_>>();
    require(
        !refs.is_empty(),
        &format!("{context}: evidence.review.{field_name} must be non-empty"),
    )?;
    for reference in refs {
        let path = reference.split('#').next().unwrap_or(reference);
        for needle in needles {
            ensure_file_contains(root, path, Some(needle), context)?;
        }
    }
    Ok(())
}

fn validate_roadmap(
    root: &Path,
    manifest: &Value,
    seen: &HashSet<String>,
    levels: &HashMap<String, String>,
) -> RunnerResult<()> {
    let roadmap = read_json(root, ROADMAP_PATH)?;
    let borrowed_seen = seen.iter().map(String::as_str).collect::<HashSet<_>>();
    let borrowed_levels = levels
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<HashMap<_, _>>();
    first_error(
        qualification_roadmap_errors(&roadmap, manifest, &borrowed_seen, &borrowed_levels),
        "qualification roadmap",
    )?;
    let kits = load_qualification_evidence_kits(root)?;
    first_error(
        qualification_evidence_kit_errors(&kits, &roadmap, manifest),
        "qualification evidence kits",
    )?;
    let make_sources = make_target_sources(root)?;
    for kit in array(&kits, "kits") {
        for requirement in array(kit, "artifact_requirements") {
            if !field(requirement, "artifact_path").is_empty() {
                ensure_file_contains(
                    root,
                    field(requirement, "artifact_path"),
                    None,
                    &format!(
                        "qualification evidence kit {}:{}",
                        field(kit, "candidate_id"),
                        field(requirement, "artifact_id")
                    ),
                )?;
            }
            let command = field(requirement, "artifact_command");
            if !command.is_empty() && !make_sources.contains(command) {
                return Err(format!(
                    "qualification evidence kit {}:{}: artifact_command is not discoverable in Make target sources",
                    field(kit, "candidate_id"),
                    field(requirement, "artifact_id")
                ));
            }
            let check_command = field(requirement, "artifact_check_command");
            if !check_command.is_empty() && !make_sources.contains(check_command) {
                return Err(format!(
                    "qualification evidence kit {}:{}: artifact_check_command is not discoverable in Make target sources",
                    field(kit, "candidate_id"),
                    field(requirement, "artifact_id")
                ));
            }
        }
    }
    validate_qualification_release_records(root, &roadmap, &kits)?;
    Ok(())
}

fn make_target_sources(root: &Path) -> RunnerResult<String> {
    let mut text = read_text(root, "Makefile")?;
    for entry in
        fs::read_dir(root.join("make")).map_err(|error| format!("failed to read make: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to read make entry: {error}"))?;
        if entry.path().extension().and_then(|value| value.to_str()) == Some("mk") {
            text.push('\n');
            text.push_str(
                &fs::read_to_string(entry.path())
                    .map_err(|error| format!("failed to read make file: {error}"))?,
            );
        }
    }
    Ok(text)
}

fn ensure_file_contains(
    root: &Path,
    relative: &str,
    needle: Option<&str>,
    context: &str,
) -> RunnerResult<()> {
    require(
        !relative.is_empty(),
        &format!("{context}: evidence file path must be non-empty"),
    )?;
    let text = read_text(root, relative)
        .map_err(|_| format!("{context}: evidence file does not exist: {relative}"))?;
    if let Some(needle) = needle {
        require(
            text.contains(needle),
            &format!("{context}: evidence file {relative} does not contain {needle}"),
        )?;
    }
    Ok(())
}

fn first_error(errors: Vec<String>, context: &str) -> RunnerResult<()> {
    if let Some(error) = errors.into_iter().next() {
        Err(format!("{context}: {error}"))
    } else {
        Ok(())
    }
}

fn context(entry: &Value) -> String {
    [
        field(entry, "operator_id"),
        field(entry, "benchmark_template"),
        "unknown operator",
    ]
    .into_iter()
    .find(|value| !value.is_empty())
    .unwrap_or("unknown operator")
    .to_string()
}

fn read_json(root: &Path, relative: &str) -> RunnerResult<Value> {
    serde_json::from_str(&read_text(root, relative)?)
        .map_err(|error| format!("{relative}: invalid json: {error}"))
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn require(condition: bool, message: &str) -> RunnerResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn value<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
