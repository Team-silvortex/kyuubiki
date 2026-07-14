use super::{Run, RunnerResult, array, build_summary, field};
use serde_json::{Value, json};

pub(super) fn run_self_test() -> RunnerResult<String> {
    let summary = build_summary(&[
        fake_run(
            "run-001",
            "mechanical-core",
            "400k",
            vec![
                fake_case("truss-400k", 100.0, "symmetric-gauss-seidel"),
                fake_case("panel-400k", 200.0, "symmetric-gauss-seidel"),
                fake_case("panel-400k#jacobi", 300.0, "jacobi"),
                fake_case(
                    "panel-400k#symmetric-gauss-seidel",
                    220.0,
                    "symmetric-gauss-seidel",
                ),
            ],
            vec![fake_comparison("panel-400k", 220.0, 300.0)],
        ),
        fake_run(
            "run-002",
            "mechanical-core",
            "400k",
            vec![fake_case("panel-400k#jacobi", 330.0, "jacobi")],
            Vec::new(),
        ),
    ]);
    let latest_ids = array(&summary, "latest_cases")
        .iter()
        .map(|item| field(item, "case_id").to_string())
        .collect::<Vec<_>>();
    if latest_ids
        != vec![
            "panel-400k",
            "panel-400k#jacobi",
            "panel-400k#symmetric-gauss-seidel",
            "truss-400k",
        ]
    {
        return Err(format!("self-test latest ids mismatch: {latest_ids:?}"));
    }
    assert_ptr(
        &summary,
        "/latest_stage_summary/0/stage",
        "solve_spd_matvec",
    )?;
    assert_num(&summary, "/latest_stage_summary/0/case_count", 4.0)?;
    assert_num(&summary, "/latest_stage_summary/0/elapsed_ms_total", 425.0)?;
    assert_num(
        &summary,
        "/latest_preconditioner_economics/0/iterations_saved",
        110.0,
    )?;
    assert_ptr(
        &summary,
        "/latest_solver_tuning_notes/0/focus",
        "matvec-or-vector-cost",
    )?;
    Ok("remote material benchmark summary self-test passed".to_string())
}

fn fake_run(
    name: &str,
    matrix: &str,
    profile: &str,
    cases: Vec<Value>,
    comparisons: Vec<Value>,
) -> Run {
    Run {
        benchmark: json!({ "cases": cases, "matrix": matrix, "preconditioner_comparisons": comparisons, "profile": profile, "repeat": 1 }),
        name: name.to_string(),
        research: None,
    }
}

fn fake_comparison(base_case_id: &str, sgs_ms: f64, jacobi_ms: f64) -> Value {
    json!({
        "base_case_id": base_case_id,
        "compared": [
            { "median_ms": sgs_ms, "solver_iterations": sgs_ms, "solver_preconditioner": "symmetric-gauss-seidel" },
            { "median_ms": jacobi_ms, "solver_iterations": jacobi_ms, "solver_preconditioner": "jacobi" }
        ],
        "winner_iteration_reduction_pct": (jacobi_ms - sgs_ms) / jacobi_ms * 100.0,
        "winner_median_ms": sgs_ms,
        "winner_preconditioner": "symmetric-gauss-seidel",
        "winner_solver_iterations": sgs_ms,
        "winner_speedup_ratio": jacobi_ms / sgs_ms,
    })
}

fn fake_case(id: &str, median_ms: f64, solver_preconditioner: &str) -> Value {
    json!({
        "dof_count": 1000,
        "id": id,
        "median_ms": median_ms,
        "memory_stages": [
            { "elapsed_ms": median_ms / 2.0, "label": "solve_spd_system", "rss_kib": 1024 },
            { "elapsed_ms": median_ms / 2.0, "label": "solve_spd_matvec", "rss_kib": 1024 },
            { "elapsed_ms": median_ms / 4.0, "label": "solve_spd_preconditioner", "rss_kib": 1024 }
        ],
        "ok": true,
        "peak_rss_kib": 2048,
        "solver_iterations": median_ms,
        "solver_matrix_non_zero_count": 1000,
        "solver_preconditioner": solver_preconditioner,
        "solver_residual_norm": 1.0e-10,
    })
}

fn assert_ptr(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    let actual = value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{pointer}: expected {expected:?}, got {actual:?}"))
    }
}

fn assert_num(value: &Value, pointer: &str, expected: f64) -> RunnerResult<()> {
    let actual = value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    if (actual - expected).abs() < 1e-9 {
        Ok(())
    } else {
        Err(format!("{pointer}: expected {expected}, got {actual}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{build_summary, fake_case, fake_comparison, fake_run};

    #[test]
    fn summary_self_test_fixture_matches_expected_latest_ids() {
        let summary = build_summary(&[fake_run(
            "run-001",
            "mechanical-core",
            "400k",
            vec![
                fake_case("panel-400k#jacobi", 300.0, "jacobi"),
                fake_case(
                    "panel-400k#symmetric-gauss-seidel",
                    220.0,
                    "symmetric-gauss-seidel",
                ),
            ],
            vec![fake_comparison("panel-400k", 220.0, 300.0)],
        )]);
        assert_eq!(summary["case_count"], 2);
        assert_eq!(
            summary["latest_preconditioner_comparisons"][0]["winner_preconditioner"],
            "symmetric-gauss-seidel"
        );
    }
}
