use super::{
    Run, base_case_id, bool_field, case_key, field, finite, finite_opt, opt_num, ratio,
    sort_by_number_desc,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

pub(super) fn summarize_stage_rows(rows: &[Value]) -> Vec<Value> {
    let mut groups = BTreeMap::<String, StageGroup>::new();
    for row in rows {
        let key = format!(
            "{}/{}/{}",
            field(row, "matrix"),
            field(row, "profile"),
            field(row, "stage")
        );
        let group = groups.entry(key).or_insert_with(|| StageGroup::from(row));
        group.add(row);
    }
    sort_by_number_desc(
        groups
            .into_values()
            .map(StageGroup::into_value)
            .collect::<Vec<_>>(),
        "elapsed_ms_total",
    )
}

#[derive(Default)]
struct StageGroup {
    case_count: u64,
    elapsed_ms_total: f64,
    matrix: String,
    max_elapsed_ms: f64,
    non_zero_elapsed_ms_total: f64,
    non_zero_visit_count: f64,
    profile: String,
    stage: String,
    stage_share_pct_total: f64,
    stage_share_sample_count: u64,
}

impl StageGroup {
    fn from(row: &Value) -> Self {
        Self {
            matrix: field(row, "matrix").to_string(),
            profile: field(row, "profile").to_string(),
            stage: field(row, "stage").to_string(),
            ..Self::default()
        }
    }

    fn add(&mut self, row: &Value) {
        let elapsed = finite(row, "elapsed_ms").unwrap_or(0.0);
        self.case_count += 1;
        self.elapsed_ms_total += elapsed;
        self.max_elapsed_ms = self.max_elapsed_ms.max(elapsed);
        let visits = stage_non_zero_visits(row);
        if visits > 0.0 {
            self.non_zero_elapsed_ms_total += elapsed;
            self.non_zero_visit_count += visits;
        }
        if let Some(share) = finite(row, "stage_share_pct") {
            self.stage_share_pct_total += share;
            self.stage_share_sample_count += 1;
        }
    }

    fn into_value(self) -> Value {
        json!({
            "average_stage_share_pct": if self.stage_share_sample_count > 0 { json!(self.stage_share_pct_total / self.stage_share_sample_count as f64) } else { Value::Null },
            "case_count": self.case_count,
            "elapsed_ms_total": self.elapsed_ms_total,
            "matrix": self.matrix,
            "max_elapsed_ms": self.max_elapsed_ms,
            "ms_per_million_non_zero_visits": if self.non_zero_visit_count > 0.0 { json!(self.non_zero_elapsed_ms_total / (self.non_zero_visit_count / 1_000_000.0)) } else { Value::Null },
            "non_zero_elapsed_ms_total": self.non_zero_elapsed_ms_total,
            "non_zero_visit_count": self.non_zero_visit_count,
            "profile": self.profile,
            "stage": self.stage,
        })
    }
}

pub(super) fn latest_preconditioner_comparisons_for_runs(
    sorted_runs: &[Run],
    latest_cases: &[Value],
) -> Vec<Value> {
    let latest_by_case = latest_cases
        .iter()
        .map(|item| (case_key(item), item.clone()))
        .collect::<HashMap<_, _>>();
    let mut comparisons = preconditioner_comparisons(latest_cases)
        .into_iter()
        .map(|item| (comparison_key(&item), item))
        .collect::<BTreeMap<_, _>>();
    for run in sorted_runs {
        for comparison in run
            .benchmark
            .get("preconditioner_comparisons")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if !report_comparison_matches_latest(run, comparison, &latest_by_case) {
                continue;
            }
            let item = json!({
                "base_case_id": field(comparison, "base_case_id"),
                "compared": comparison.get("compared").cloned().unwrap_or_else(|| json!([])),
                "matrix": field(&run.benchmark, "matrix"),
                "profile": field(&run.benchmark, "profile"),
                "winner_iteration_reduction_pct": comparison.get("winner_iteration_reduction_pct").cloned().unwrap_or(Value::Null),
                "winner_median_ms": comparison.get("winner_median_ms").cloned().unwrap_or(Value::Null),
                "winner_preconditioner": comparison.get("winner_preconditioner").cloned().unwrap_or(Value::Null),
                "winner_solver_iterations": comparison.get("winner_solver_iterations").cloned().unwrap_or(Value::Null),
                "winner_speedup_ratio": comparison.get("winner_speedup_ratio").cloned().unwrap_or(Value::Null),
            });
            comparisons.insert(comparison_key(&item), item);
        }
    }
    sort_by_number_desc(comparisons.into_values().collect(), "winner_median_ms")
}

fn preconditioner_comparisons(cases: &[Value]) -> Vec<Value> {
    let mut groups = BTreeMap::<String, Vec<Value>>::new();
    for item in cases
        .iter()
        .filter(|case| bool_field(case, "ok") && !field(case, "solver_preconditioner").is_empty())
    {
        let key = format!(
            "{}/{}/{}",
            field(item, "matrix"),
            field(item, "profile"),
            base_case_id(field(item, "case_id"))
        );
        groups.entry(key).or_default().push(item.clone());
    }
    let mut rows = Vec::new();
    for items in groups.values() {
        let deduped = dedupe_by_preconditioner(items);
        if deduped.len() <= 1 {
            continue;
        }
        let mut sorted = deduped;
        sorted.sort_by(|left, right| {
            finite(left, "median_ms")
                .partial_cmp(&finite(right, "median_ms"))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let first = &sorted[0];
        let last = sorted.last().unwrap_or(first);
        rows.push(json!({
            "base_case_id": base_case_id(field(first, "case_id")),
            "compared": sorted.iter().map(|item| json!({
                "median_ms": item.get("median_ms").cloned().unwrap_or(Value::Null),
                "solver_iterations": item.get("solver_iterations").cloned().unwrap_or(Value::Null),
                "solver_preconditioner": item.get("solver_preconditioner").cloned().unwrap_or(Value::Null),
            })).collect::<Vec<_>>(),
            "matrix": field(first, "matrix"),
            "profile": field(first, "profile"),
            "winner_iteration_reduction_pct": iteration_reduction_pct(finite(first, "solver_iterations"), finite(last, "solver_iterations")).map(Value::from).unwrap_or(Value::Null),
            "winner_median_ms": first.get("median_ms").cloned().unwrap_or(Value::Null),
            "winner_preconditioner": field(first, "solver_preconditioner"),
            "winner_solver_iterations": first.get("solver_iterations").cloned().unwrap_or(Value::Null),
            "winner_speedup_ratio": ratio(finite(last, "median_ms"), finite(first, "median_ms")).map(Value::from).unwrap_or(Value::Null),
        }));
    }
    sort_by_number_desc(rows, "winner_median_ms")
}

pub(super) fn preconditioner_economics(comparisons: &[Value], stage_rows: &[Value]) -> Vec<Value> {
    let stage_by_case = stage_rows_by_case(stage_rows);
    comparisons
        .iter()
        .map(|comparison| {
            let compared = comparison
                .get("compared")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let winner = compared.iter().find(|item| {
                field(item, "solver_preconditioner") == field(comparison, "winner_preconditioner")
            });
            let slowest = compared.last();
            let winner_case_id = format!(
                "{}#{}",
                field(comparison, "base_case_id"),
                field(comparison, "winner_preconditioner")
            );
            let slowest_case_id = format!(
                "{}#{}",
                field(comparison, "base_case_id"),
                slowest
                    .map(|item| field(item, "solver_preconditioner"))
                    .unwrap_or("")
            );
            let winner_stages = stage_by_case
                .get(&stage_case_key(comparison, &winner_case_id))
                .cloned()
                .unwrap_or_default();
            let slowest_stages = stage_by_case
                .get(&stage_case_key(comparison, &slowest_case_id))
                .cloned()
                .unwrap_or_default();
            let elapsed_saved_ms = finite_opt(winner, "median_ms")
                .zip(finite_opt(slowest, "median_ms"))
                .map(|(winner, slowest)| slowest - winner);
            let iterations_saved = finite_opt(winner, "solver_iterations")
                .zip(finite_opt(slowest, "solver_iterations"))
                .map(|(winner, slowest)| slowest - winner);
            let winner_preconditioner_ms = winner_stages.get("solve_spd_preconditioner").copied();
            let slowest_preconditioner_ms = slowest_stages.get("solve_spd_preconditioner").copied();
            let preconditioner_extra_ms = winner_preconditioner_ms
                .zip(slowest_preconditioner_ms)
                .map(|(winner, slowest)| winner - slowest);
            json!({
                "base_case_id": field(comparison, "base_case_id"),
                "elapsed_saved_ms": opt_num(elapsed_saved_ms),
                "extra_pre_ms_per_iteration_saved": opt_num(ratio(preconditioner_extra_ms, iterations_saved)),
                "gross_non_preconditioner_saved_ms": opt_num(elapsed_saved_ms.zip(preconditioner_extra_ms).map(|(elapsed, pre)| elapsed + pre)),
                "iterations_saved": opt_num(iterations_saved),
                "matrix": field(comparison, "matrix"),
                "ms_saved_per_iteration_saved": opt_num(ratio(elapsed_saved_ms, iterations_saved)),
                "preconditioner_extra_ms": opt_num(preconditioner_extra_ms),
                "profile": field(comparison, "profile"),
                "slowest_matvec_ms": opt_num(slowest_stages.get("solve_spd_matvec").copied()),
                "slowest_matvec_ms_per_iteration": opt_num(ratio(slowest_stages.get("solve_spd_matvec").copied(), finite_opt(slowest, "solver_iterations"))),
                "slowest_preconditioner": slowest.map(|item| field(item, "solver_preconditioner")).unwrap_or(""),
                "slowest_preconditioner_ms": opt_num(slowest_preconditioner_ms),
                "slowest_pre_ms_per_iteration": opt_num(ratio(slowest_preconditioner_ms, finite_opt(slowest, "solver_iterations"))),
                "winner_matvec_ms": opt_num(winner_stages.get("solve_spd_matvec").copied()),
                "winner_matvec_ms_per_iteration": opt_num(ratio(winner_stages.get("solve_spd_matvec").copied(), finite_opt(winner, "solver_iterations"))),
                "winner_preconditioner": field(comparison, "winner_preconditioner"),
                "winner_preconditioner_ms": opt_num(winner_preconditioner_ms),
                "winner_pre_ms_per_iteration": opt_num(ratio(winner_preconditioner_ms, finite_opt(winner, "solver_iterations"))),
                "winner_speedup_ratio": comparison.get("winner_speedup_ratio").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

pub(super) fn solver_tuning_notes(economics: &[Value]) -> Vec<Value> {
    let mut notes = economics
        .iter()
        .filter_map(|item| {
            let pre = finite(item, "winner_pre_ms_per_iteration")?;
            let matvec = finite(item, "winner_matvec_ms_per_iteration")?;
            let pre_dominates = pre > matvec;
            Some(json!({
                "case_id": field(item, "base_case_id"),
                "focus": if pre_dominates { "sgs-sweep-cost" } else { "matvec-or-vector-cost" },
                "matrix": field(item, "matrix"),
                "profile": field(item, "profile"),
                "reason": if pre_dominates {
                    "winner preconditioner cost per iteration is above matvec cost per iteration"
                } else {
                    "winner matvec/vector work remains comparable to preconditioner cost"
                },
                "winner_matvec_ms_per_iteration": matvec,
                "winner_pre_ms_per_iteration": pre,
            }))
        })
        .collect::<Vec<_>>();
    notes.sort_by(|left, right| {
        let left_delta = finite(left, "winner_pre_ms_per_iteration").unwrap_or(0.0)
            - finite(left, "winner_matvec_ms_per_iteration").unwrap_or(0.0);
        let right_delta = finite(right, "winner_pre_ms_per_iteration").unwrap_or(0.0)
            - finite(right, "winner_matvec_ms_per_iteration").unwrap_or(0.0);
        right_delta
            .partial_cmp(&left_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    notes
}

pub(super) fn optimization_targets(stage_summary: &[Value]) -> Vec<Value> {
    let rows = stage_summary
        .iter()
        .map(|item| {
            let score = finite(item, "elapsed_ms_total").unwrap_or(0.0)
                * (1.0 + finite(item, "average_stage_share_pct").unwrap_or(0.0) / 100.0);
            json!({
                "average_stage_share_pct": item.get("average_stage_share_pct").cloned().unwrap_or(Value::Null),
                "case_count": item.get("case_count").cloned().unwrap_or(Value::Null),
                "elapsed_ms_total": item.get("elapsed_ms_total").cloned().unwrap_or(Value::Null),
                "focus": optimization_focus(field(item, "stage")),
                "matrix": field(item, "matrix"),
                "ms_per_million_non_zero_visits": item.get("ms_per_million_non_zero_visits").cloned().unwrap_or(Value::Null),
                "non_zero_visit_count": item.get("non_zero_visit_count").cloned().unwrap_or(Value::Null),
                "priority_score": score,
                "profile": field(item, "profile"),
                "stage": field(item, "stage"),
            })
        })
        .collect::<Vec<_>>();
    sort_by_number_desc(rows, "priority_score")
}

pub(super) fn sparse_matvec_throughput(stage_summary: &[Value]) -> Vec<Value> {
    let mut rows = stage_summary
        .iter()
        .filter(|item| {
            field(item, "stage") == "solve_spd_matvec"
                && finite(item, "ms_per_million_non_zero_visits").is_some()
        })
        .cloned()
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        finite(right, "non_zero_visit_count")
            .partial_cmp(&finite(left, "non_zero_visit_count"))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                finite(left, "ms_per_million_non_zero_visits")
                    .partial_cmp(&finite(right, "ms_per_million_non_zero_visits"))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    rows
}

fn stage_rows_by_case(stage_rows: &[Value]) -> HashMap<String, HashMap<String, f64>> {
    let mut groups = HashMap::<String, HashMap<String, f64>>::new();
    for row in stage_rows {
        groups
            .entry(format!(
                "{}/{}/{}",
                field(row, "matrix"),
                field(row, "profile"),
                field(row, "case_id")
            ))
            .or_default()
            .insert(
                field(row, "stage").to_string(),
                finite(row, "elapsed_ms").unwrap_or(0.0),
            );
    }
    groups
}

fn report_comparison_matches_latest(
    run: &Run,
    comparison: &Value,
    latest_by_case: &HashMap<String, Value>,
) -> bool {
    comparison
        .get("compared")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .all(|item| {
            let case_id = format!(
                "{}#{}",
                field(comparison, "base_case_id"),
                field(item, "solver_preconditioner")
            );
            latest_by_case
                .get(&format!(
                    "{}/{}/{}",
                    field(&run.benchmark, "matrix"),
                    field(&run.benchmark, "profile"),
                    case_id
                ))
                .is_some_and(|latest| field(latest, "run") == run.name)
        })
}

fn dedupe_by_preconditioner(items: &[Value]) -> Vec<Value> {
    let has_explicit = items
        .iter()
        .any(|item| field(item, "case_id").contains('#'));
    let mut best = BTreeMap::<String, Value>::new();
    for item in items
        .iter()
        .filter(|candidate| !has_explicit || field(candidate, "case_id").contains('#'))
    {
        let key = field(item, "solver_preconditioner").to_string();
        if best
            .get(&key)
            .is_none_or(|current| finite(item, "median_ms") < finite(current, "median_ms"))
        {
            best.insert(key, item.clone());
        }
    }
    best.into_values().collect()
}

fn stage_non_zero_visits(row: &Value) -> f64 {
    if field(row, "stage") != "solve_spd_matvec" {
        return 0.0;
    }
    finite(row, "solver_matrix_non_zero_count")
        .zip(finite(row, "solver_iterations"))
        .map_or(0.0, |(nnz, iterations)| nnz * iterations)
}

fn optimization_focus(stage: &str) -> &'static str {
    match stage {
        "solve_spd_matvec" => "sparse-matvec-throughput",
        "solve_spd_preconditioner" => "preconditioner-sweep-throughput",
        "solve_spd_dot" => "vector-reduction-throughput",
        "solve_spd_vector_update" | "solve_spd_direction_update" => "vector-update-throughput",
        "solve_spd_residual_recompute" => "residual-refresh-cost",
        "assemble_global" => "assembly-throughput",
        _ => "stage-throughput",
    }
}

fn comparison_key(item: &Value) -> String {
    format!(
        "{}/{}/{}",
        field(item, "matrix"),
        field(item, "profile"),
        field(item, "base_case_id")
    )
}

fn stage_case_key(comparison: &Value, case_id: &str) -> String {
    format!(
        "{}/{}/{}",
        field(comparison, "matrix"),
        field(comparison, "profile"),
        case_id
    )
}

fn iteration_reduction_pct(winner: Option<f64>, slowest: Option<f64>) -> Option<f64> {
    let (winner, slowest) = winner.zip(slowest)?;
    (slowest > 0.0).then_some((slowest - winner) / slowest * 100.0)
}
