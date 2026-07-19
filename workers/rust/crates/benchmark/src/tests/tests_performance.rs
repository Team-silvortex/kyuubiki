use super::benchmark_cases;
use crate::config::BenchmarkProfile;
use crate::models::select_cases;
use crate::runner_preconditioner::{effective_preconditioner, preconditioner_selection_reason};

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

    assert_eq!(report.cases.len(), 3);
    assert!(report.cases.iter().any(|case| case.id.ends_with("#jacobi")));
    assert!(report
        .cases
        .iter()
        .any(|case| case.id.ends_with("#symmetric-gauss-seidel")));
    assert!(report.cases.iter().any(|case| case.id.ends_with("#ic0")));
    assert_eq!(report.preconditioner_comparisons.len(), 1);
    assert_eq!(
        report.preconditioner_comparisons[0].base_case_id,
        "truss-roof-medium"
    );
    assert_eq!(report.preconditioner_comparisons[0].compared.len(), 3);
    assert!(report.preconditioner_comparisons[0].winner_speedup_ratio >= 1.0);
    let json = serde_json::to_value(&report).expect("benchmark report should serialize");
    assert!(json.get("preconditioner_comparisons").is_some());
    assert!(json["preconditioner_comparisons"][0]
        .get("winner_speedup_ratio")
        .is_some());
}

#[test]
fn solver_preconditioner_auto_uses_evidence_backed_iterative_strategies() {
    for (matrix, case_id) in [
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

    let cases = benchmark_cases(BenchmarkProfile::Medium, "thermal-structural");
    let selected = cases
        .iter()
        .filter(|case| case.id == "thermal-plane-triangle-medium")
        .collect::<Vec<_>>();
    let report = crate::runner::build_report(
        &selected,
        1,
        BenchmarkProfile::Medium,
        "thermal-structural",
        "auto",
    );
    assert_eq!(
        report.cases[0].solver_preconditioner.as_deref(),
        Some("ic0"),
        "thermal-plane triangles should use IC(0) in auto mode"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn thermal_quad_auto_uses_ic0_only_at_the_validated_scale() {
    let medium = benchmark_cases(BenchmarkProfile::Medium, "thermal-structural");
    let medium_quad = medium
        .iter()
        .find(|case| case.id == "thermal-plane-quad-medium")
        .expect("medium thermal-plane quad case");
    assert_eq!(
        effective_preconditioner(medium_quad, "auto"),
        "symmetric-gauss-seidel"
    );
    assert_eq!(
        preconditioner_selection_reason(medium_quad, "auto"),
        "auto-iterative-sgs"
    );

    let one_million = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
    let large_quad = one_million
        .iter()
        .find(|case| case.id == "thermal-plane-quad-1m")
        .expect("one-million thermal-plane quad case");
    assert_eq!(effective_preconditioner(large_quad, "auto"), "ic0");
    assert_eq!(
        preconditioner_selection_reason(large_quad, "auto"),
        "auto-large-thermal-plane-quad-ic0"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn one_million_truss_auto_uses_ic0() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "mechanical-core");
    let truss = cases
        .iter()
        .find(|case| case.id == "truss-roof-1m")
        .expect("one-million truss case");

    assert_eq!(effective_preconditioner(truss, "auto"), "ic0");
    assert_eq!(
        preconditioner_selection_reason(truss, "auto"),
        "auto-large-truss-ic0"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn one_million_plane_quad_auto_uses_ic0() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "mechanical-core");
    let panel = cases
        .iter()
        .find(|case| case.id == "plane-quad-panel-1m")
        .expect("one-million structural quad panel");

    assert_eq!(effective_preconditioner(panel, "auto"), "ic0");
    assert_eq!(
        preconditioner_selection_reason(panel, "auto"),
        "auto-large-plane-quad-ic0"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn one_million_plane_triangle_auto_uses_ic0() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "mechanical-core");
    let panel = cases
        .iter()
        .find(|case| case.id == "plane-panel-1m")
        .expect("one-million structural triangle panel");

    assert_eq!(effective_preconditioner(panel, "auto"), "ic0");
    assert_eq!(
        preconditioner_selection_reason(panel, "auto"),
        "auto-large-plane-triangle-ic0"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn one_million_thermal_truss_auto_uses_ic0() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
    let truss = cases
        .iter()
        .find(|case| case.id == "thermal-truss-2d-1m")
        .expect("one-million thermal truss case");

    assert_eq!(effective_preconditioner(truss, "auto"), "ic0");
    assert_eq!(
        preconditioner_selection_reason(truss, "auto"),
        "auto-large-thermal-truss-ic0"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn one_million_frame_cases_auto_use_ic0() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
    for (case_id, reason) in [
        ("frame-2d-1m", "auto-large-frame-2d-ic0"),
        ("frame-3d-1m", "auto-large-frame-3d-ic0"),
        ("thermal-frame-2d-1m", "auto-large-thermal-frame-2d-ic0"),
        ("thermal-frame-3d-1m", "auto-large-thermal-frame-3d-ic0"),
    ] {
        let case = cases
            .iter()
            .find(|case| case.id == case_id)
            .unwrap_or_else(|| panic!("{case_id} should exist"));
        assert_eq!(effective_preconditioner(case, "auto"), "ic0");
        assert_eq!(preconditioner_selection_reason(case, "auto"), reason);
    }
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn case_selection_uses_exact_ids() {
    let cases = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
    let selected = select_cases(&cases, Some("frame-2d-1m"));

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "frame-2d-1m");
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
    assert!(
        result.hotspot_label.is_some(),
        "heat quad profile should expose the dominant benchmark hotspot"
    );
    assert!(
        result.hotspot_elapsed_ms.unwrap_or_default() >= 0.0,
        "hotspot should include elapsed time"
    );
    assert!(
        result
            .hotspot_hint
            .as_deref()
            .unwrap_or_default()
            .contains("bound")
            || result.hotspot_hint.as_deref().unwrap_or_default().contains("inspect"),
        "hotspot should include an optimization hint"
    );
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn thermal_structural_500k_frame_and_truss_cases_are_profile_scaled() {
    let cases = benchmark_cases(BenchmarkProfile::FiveHundredK, "thermal-structural");

    for case_id in [
        "thermal-truss-2d-500k",
        "thermal-truss-3d-500k",
        "frame-2d-500k",
        "thermal-frame-2d-500k",
        "frame-3d-500k",
        "thermal-frame-3d-500k",
    ] {
        let case = cases
            .iter()
            .find(|case| case.id == case_id)
            .unwrap_or_else(|| panic!("{case_id} should be in thermal-structural 500k"));
        let (_, _, dofs) = crate::runner_shape::workload_shape(&case.workload);

        assert!(
            dofs >= 500_000,
            "{case_id} should be profile-scaled, got {dofs} dofs"
        );
    }
}

#[test]
#[ignore = "large-scale workload generation runs in test-rust-scale-profiles"]
fn dry_run_shape_report_exposes_selected_case_scale_without_solving() {
    let cases = benchmark_cases(BenchmarkProfile::FiveHundredK, "thermal-structural");
    let selected = cases
        .iter()
        .filter(|case| case.id == "thermal-truss-2d-500k")
        .collect::<Vec<_>>();
    let report = crate::shape_report::build_shape_report(
        &selected,
        BenchmarkProfile::FiveHundredK,
        "thermal-structural",
    );

    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.cases[0].id, "thermal-truss-2d-500k");
    assert!(report.cases[0].node_count >= 250_000);
    assert!(report.cases[0].element_count >= 500_000);
    assert!(report.cases[0].dof_count >= 500_000);
}
