use super::*;

pub(super) fn validate_contract(
    root: &Path,
    contract: &QualificationContract,
    manifest: &CoverageManifest,
) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("benchmark qualification schemas are invalid".into());
    }
    let schema: Value = read_json(root, REPORT_SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("benchmark qualification report schema const drifted".into());
    }
    let expected_modules = [
        "runtime-agent-cli",
        "runtime-engine-solver",
        "verification-evidence",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let modules = contract
        .required_modules
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if modules != expected_modules || modules.len() != contract.required_modules.len() {
        return Err("benchmark qualification module set drifted".into());
    }
    if manifest.schema_version != "kyuubiki.benchmark-profile-coverage/v1" {
        return Err("benchmark coverage manifest schema drifted".into());
    }
    for path in contract
        .source_files
        .iter()
        .chain([&contract.coverage_manifest, &contract.direct_mesh_baseline])
    {
        if !repo_path(root, path)?.is_file() {
            return Err(format!(
                "benchmark qualification source does not exist: {path}"
            ));
        }
    }
    if contract.source_files.len() < 6
        || contract.source_files.iter().collect::<BTreeSet<_>>().len()
            != contract.source_files.len()
        || contract.current_runs.len() < 2
        || contract.min_retained_runs < 50
        || contract.min_resolved_failures == 0
        || contract.min_direct_mesh_repeats < 3
        || contract.one_million_node_threshold < 1_000_000
    {
        return Err("benchmark qualification depth is below policy".into());
    }
    let mut run_ids = BTreeSet::new();
    for spec in &contract.current_runs {
        if spec.id.is_empty()
            || !run_ids.insert(spec.id.as_str())
            || spec.repeat < 3
            || spec.profile.is_empty()
            || spec.report_profile.is_empty()
            || spec.matrix.is_empty()
            || spec.case_id.is_empty()
        {
            return Err(format!("benchmark current run {} is invalid", spec.id));
        }
    }
    let requirements = contract
        .profile_requirements
        .iter()
        .map(|item| (item.profile.as_str(), item.expected_case_count))
        .collect::<BTreeMap<_, _>>();
    if requirements.get("five_hundred_k") != Some(&19)
        || requirements.get("one_million") != Some(&39)
        || requirements.len() != contract.profile_requirements.len()
    {
        return Err("benchmark profile requirements drifted".into());
    }
    let expected = expected_one_million_cases(manifest)?;
    if expected.len() != 6 || expected.values().map(Vec::len).sum::<usize>() != 39 {
        return Err("one-million benchmark manifest must define 39 cases in 6 matrices".into());
    }
    Ok(())
}

pub(super) fn validate_report(
    root: &Path,
    contract: &QualificationContract,
    manifest: &CoverageManifest,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.status != "passed"
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.source_tree_sha256 != source_tree_digest(root, &contract.source_files)?
        || report.limitations != LIMITATIONS
    {
        return Err("benchmark qualification report header drifted".into());
    }
    validate_current_runs(contract, &report.current_runs)?;
    validate_scale_archive(contract, manifest, &report.scale_archive)?;
    validate_direct_mesh(root, contract, &report.direct_mesh)?;
    let expected = build_summary(
        contract,
        &report.current_runs,
        &report.scale_archive,
        &report.direct_mesh,
    );
    if !summary_matches(&report.summary, &expected) {
        return Err("benchmark qualification summary drifted".into());
    }
    let rendered = serde_json::to_string(report)
        .map_err(|error| format!("failed to inspect benchmark report: {error}"))?;
    if rendered.contains("/Users/")
        || rendered.contains("/home/")
        || rendered.contains("\\\\Users\\\\")
    {
        return Err("benchmark qualification report leaks host paths".into());
    }
    Ok(())
}

fn validate_current_runs(
    contract: &QualificationContract,
    runs: &[CurrentRunEvidence],
) -> RunnerResult<()> {
    if runs.len() != contract.current_runs.len() {
        return Err("benchmark current run count drifted".into());
    }
    for spec in &contract.current_runs {
        let run = runs
            .iter()
            .find(|run| run.id == spec.id)
            .ok_or_else(|| format!("benchmark report misses current run {}", spec.id))?;
        let case = &run.case;
        if run.route != ROUTE
            || run.args != benchmark_args(spec)
            || run.status != "passed"
            || run.launch_elapsed_ms == 0
            || !is_digest(&run.stdout_sha256)
            || run.repeat != spec.repeat
            || run.profile != spec.report_profile
            || run.matrix != spec.matrix
            || case.id != spec.case_id
            || case.family.is_empty()
            || !case.ok
            || case.repeat != spec.repeat
            || case.node_count < spec.min_node_count
            || !valid_timings(case)
        {
            return Err(format!("benchmark current run {} drifted", spec.id));
        }
    }
    Ok(())
}

fn valid_timings(case: &CurrentCaseEvidence) -> bool {
    [
        case.min_ms,
        case.median_ms,
        case.mean_ms,
        case.p95_ms,
        case.max_ms,
    ]
    .iter()
    .all(|value| value.is_finite() && *value >= 0.0)
        && case.min_ms <= case.median_ms
        && case.median_ms <= case.p95_ms
        && case.p95_ms <= case.max_ms
}

fn validate_scale_archive(
    contract: &QualificationContract,
    manifest: &CoverageManifest,
    archive: &ScaleArchiveEvidence,
) -> RunnerResult<()> {
    if archive.schema_version != "kyuubiki.benchmark-profile-index/v1"
        || !is_digest(&archive.source_index_sha256)
        || archive.gate_status != "pass"
        || archive.retained_run_count < contract.min_retained_runs
        || archive.resolved_failure_count < contract.min_resolved_failures
        || archive.failed_run_count < archive.resolved_failure_count
        || archive.unresolved_failure_count != 0
        || archive.profiles.len() != contract.profile_requirements.len()
    {
        return Err("benchmark scale archive header drifted".into());
    }
    for requirement in &contract.profile_requirements {
        let profile = archive
            .profiles
            .iter()
            .find(|item| item.profile == requirement.profile)
            .ok_or_else(|| format!("benchmark report misses profile {}", requirement.profile))?;
        if profile.expected_case_count != requirement.expected_case_count
            || profile.covered_case_count != requirement.expected_case_count
            || profile.missing_case_count != 0
            || profile.below_scale_threshold_case_count != 0
            || (requirement.require_scale_qualified
                && profile.scale_qualified_covered_case_count != requirement.expected_case_count)
        {
            return Err(format!("benchmark profile {} drifted", requirement.profile));
        }
    }
    let expected = expected_one_million_cases(manifest)?
        .into_iter()
        .flat_map(|(matrix, cases)| cases.into_iter().map(move |case| (matrix.clone(), case)))
        .collect::<BTreeSet<_>>();
    let actual = archive
        .one_million_cases
        .iter()
        .map(|item| (item.matrix.clone(), item.case_id.clone()))
        .collect::<BTreeSet<_>>();
    if actual != expected || actual.len() != archive.one_million_cases.len() {
        return Err("one-million benchmark case set drifted".into());
    }
    for case in &archive.one_million_cases {
        if case.node_count < contract.one_million_node_threshold
            || case.source_slug.is_empty()
            || case.source_slug.contains('/')
            || case.run_case_count == 0
            || case.run_repeat == 0
            || !case.run_total_median_ms.is_finite()
            || case.run_total_median_ms <= 0.0
            || !case.run_peak_rss_mib.is_finite()
            || case.run_peak_rss_mib <= 0.0
        {
            return Err(format!(
                "one-million benchmark case {}/{} drifted",
                case.matrix, case.case_id
            ));
        }
    }
    Ok(())
}

fn validate_direct_mesh(
    root: &Path,
    contract: &QualificationContract,
    direct_mesh: &DirectMeshEvidence,
) -> RunnerResult<()> {
    if direct_mesh.baseline_path != contract.direct_mesh_baseline
        || direct_mesh.baseline_sha256 != sha256_file(root, &contract.direct_mesh_baseline)?
        || direct_mesh.repeat < contract.min_direct_mesh_repeats
        || direct_mesh.run_count < contract.min_direct_mesh_repeats
        || direct_mesh.subtest_sample_count < contract.min_direct_mesh_repeats * 2
        || !direct_mesh.elapsed_mean_s.is_finite()
        || direct_mesh.elapsed_mean_s <= 0.0
        || !direct_mesh.peak_rss_mean_kib.is_finite()
        || direct_mesh.peak_rss_mean_kib <= 0.0
        || direct_mesh.comparator_status != "passed"
        || !is_digest(&direct_mesh.comparison_sha256)
    {
        return Err("direct mesh benchmark evidence drifted".into());
    }
    Ok(())
}

fn summary_matches(left: &QualificationSummary, right: &QualificationSummary) -> bool {
    left.module_count == right.module_count
        && left.current_run_count == right.current_run_count
        && left.current_repeat_count == right.current_repeat_count
        && left.five_hundred_k_case_count == right.five_hundred_k_case_count
        && left.one_million_case_count == right.one_million_case_count
        && left.one_million_matrix_count == right.one_million_matrix_count
        && left.resolved_failure_count == right.resolved_failure_count
        && left.direct_mesh_repeat_count == right.direct_mesh_repeat_count
}
