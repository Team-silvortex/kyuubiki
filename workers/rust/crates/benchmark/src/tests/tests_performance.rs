use super::benchmark_cases;
use crate::config::BenchmarkProfile;

#[test]
fn solver_preconditioner_all_expands_truss_cases() {
    let cases = benchmark_cases(BenchmarkProfile::Medium, "mechanical-core");
    let selected = cases
        .iter()
        .filter(|case| case.id == "truss-roof-medium")
        .collect::<Vec<_>>();
    let report = crate::runner::build_report(
        &selected,
        1,
        BenchmarkProfile::Medium,
        "mechanical-core",
        "all",
    );

    assert_eq!(report.cases.len(), 2);
    assert!(report.cases.iter().any(|case| case.id.ends_with("#jacobi")));
    assert!(report
        .cases
        .iter()
        .any(|case| case.id.ends_with("#symmetric-gauss-seidel")));
    assert_eq!(report.preconditioner_comparisons.len(), 1);
    assert_eq!(
        report.preconditioner_comparisons[0].base_case_id,
        "truss-roof-medium"
    );
    assert_eq!(report.preconditioner_comparisons[0].compared.len(), 2);
    assert!(report.preconditioner_comparisons[0].winner_speedup_ratio >= 1.0);
    let json = serde_json::to_value(&report).expect("benchmark report should serialize");
    assert!(json.get("preconditioner_comparisons").is_some());
    assert!(json["preconditioner_comparisons"][0]
        .get("winner_speedup_ratio")
        .is_some());
}

#[test]
fn solver_preconditioner_auto_uses_sgs_for_iterative_cases() {
    for (matrix, case_id) in [
        ("thermal-structural", "thermal-plane-triangle-medium"),
        ("thermal-structural", "thermal-plane-quad-medium"),
        ("thermal-core", "heat-plane-quad-medium"),
        ("mechanical-core", "truss-roof-medium"),
        ("mechanical-core", "plane-panel-medium"),
        ("mechanical-core", "plane-quad-panel-medium"),
    ] {
        let cases = benchmark_cases(BenchmarkProfile::Medium, matrix);
        let selected = cases
            .iter()
            .filter(|case| case.id == case_id)
            .collect::<Vec<_>>();
        let report =
            crate::runner::build_report(&selected, 1, BenchmarkProfile::Medium, matrix, "auto");

        assert_eq!(report.cases.len(), 1);
        assert_eq!(
            report.cases[0].solver_preconditioner.as_deref(),
            Some("symmetric-gauss-seidel"),
            "case {case_id} should use SGS in auto mode"
        );
    }
}

#[test]
fn iterative_structural_cases_expose_solver_hotspot_stages() {
    let cases = benchmark_cases(BenchmarkProfile::TenK, "mechanical-core");
    let selected = cases
        .iter()
        .filter(|case| case.id == "plane-panel-10k")
        .collect::<Vec<_>>();
    let report = crate::runner::build_report(
        &selected,
        1,
        BenchmarkProfile::TenK,
        "mechanical-core",
        "auto",
    );
    let stage_labels = report.cases[0]
        .memory_stages
        .iter()
        .map(|stage| stage.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        report.cases[0].solver_preconditioner.as_deref(),
        Some("symmetric-gauss-seidel")
    );
    assert!(
        report.cases[0].solver_matrix_non_zero_count.unwrap_or_default() > 0,
        "iterative benchmark cases should expose reduced sparse matrix nnz"
    );
    assert!(stage_labels.contains(&"solve_spd_system"));
    assert!(stage_labels.contains(&"solve_spd_matvec"));
    assert!(stage_labels.contains(&"solve_spd_preconditioner"));
    assert!(stage_labels.contains(&"solve_spd_dot"));
}

#[test]
fn plane_quad_benchmark_exposes_solver_hotspot_stages() {
    let cases = benchmark_cases(BenchmarkProfile::TenK, "mechanical-core");
    let selected = cases
        .iter()
        .filter(|case| case.id == "plane-quad-panel-10k")
        .collect::<Vec<_>>();
    let report = crate::runner::build_report(
        &selected,
        1,
        BenchmarkProfile::TenK,
        "mechanical-core",
        "auto",
    );
    let result = &report.cases[0];
    let stage_labels = result
        .memory_stages
        .iter()
        .map(|stage| stage.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        result.solver_preconditioner.as_deref(),
        Some("symmetric-gauss-seidel")
    );
    assert!(
        result.solver_matrix_non_zero_count.unwrap_or_default() > 0,
        "plane quad benchmark should expose reduced sparse matrix nnz"
    );
    assert!(stage_labels.contains(&"precompute"));
    assert!(stage_labels.contains(&"assemble_global"));
    assert!(stage_labels.contains(&"solve_spd_system"));
    assert!(stage_labels.contains(&"solve_spd_matvec"));
    assert!(stage_labels.contains(&"solve_spd_dot"));
}

#[test]
fn heat_quad_benchmark_exposes_timed_memory_stages() {
    let cases = benchmark_cases(BenchmarkProfile::TenK, "thermal-core");
    let selected = cases
        .iter()
        .filter(|case| case.id == "heat-plane-quad-10k")
        .collect::<Vec<_>>();
    let report =
        crate::runner::build_report(&selected, 1, BenchmarkProfile::TenK, "thermal-core", "auto");
    let result = &report.cases[0];
    let stage_labels = result
        .memory_stages
        .iter()
        .map(|stage| stage.label.as_str())
        .collect::<Vec<_>>();

    assert!(stage_labels.contains(&"precompute"));
    assert!(stage_labels.contains(&"assemble_global"));
    assert!(stage_labels.contains(&"reduce_system"));
    assert!(stage_labels.contains(&"solve_system"));
    assert!(stage_labels.contains(&"solve_spd_matvec"));
    assert!(stage_labels.contains(&"solve_spd_preconditioner"));
    assert!(stage_labels.contains(&"solve_spd_dot"));
    assert!(stage_labels.contains(&"assemble"));
    assert_eq!(
        result.solver_preconditioner.as_deref(),
        Some("symmetric-gauss-seidel")
    );
    assert!(
        result.solver_matrix_non_zero_count.unwrap_or_default() > 0,
        "heat quad benchmark should expose reduced sparse matrix nnz"
    );
    assert!(
        result
            .memory_stages
            .iter()
            .all(|stage| stage.elapsed_ms.is_some_and(|elapsed| elapsed >= 0.0)),
        "heat quad profile should keep elapsed timings for every memory stage"
    );
}
