use super::{Value, array_strings, normalize_case_id, number_field, string_field};
use serde_json::json;
use std::collections::BTreeMap;

pub(super) fn matrix_summaries(runs: &[Value]) -> Vec<Value> {
    let mut groups = BTreeMap::<String, MatrixSummary>::new();
    for run in runs {
        groups
            .entry(string_field(run, "matrix"))
            .or_insert_with(|| MatrixSummary::from_run(run))
            .add(run);
    }
    groups
        .into_values()
        .map(MatrixSummary::into_value)
        .collect()
}

pub(super) fn solver_strategy_summaries(runs: &[Value]) -> Vec<Value> {
    let mut groups = BTreeMap::<(String, String, String), BTreeMap<String, Value>>::new();
    for run in runs {
        if number_field(run, "case_count") != 1.0 {
            continue;
        }
        let case_ids = array_strings(run, "case_ids");
        let preconditioners = array_strings(run, "solver_preconditioners");
        if case_ids.len() != 1 || preconditioners.len() != 1 {
            continue;
        }
        let key = (
            string_field(run, "matrix"),
            string_field(run, "profile"),
            normalize_case_id(&case_ids[0]),
        );
        let preconditioner = preconditioners[0].clone();
        let metrics = case_solver_metrics(run, &case_ids[0], &preconditioner);
        groups
            .entry(key)
            .or_default()
            .entry(preconditioner)
            .or_insert_with(|| {
                json!({
                    "slug": string_field(run, "slug"),
                    "median_ms": number_field(run, "total_median_ms"),
                    "peak_rss_mib": number_field(run, "peak_rss_mib"),
                    "solver_preconditioner_reason": metrics["solver_preconditioner_reason"].clone(),
                    "solver_iterations": metrics["solver_iterations"].clone(),
                    "solver_residual_norm": metrics["solver_residual_norm"].clone(),
                })
            });
    }
    groups
        .into_iter()
        .filter(|(_, strategies)| strategies.len() > 1)
        .map(|((matrix, profile, case_id), strategies)| {
            json!({
                "matrix": matrix,
                "profile": profile,
                "case_id": case_id,
                "strategies": strategies.into_iter().map(|(preconditioner, result)| json!({
                    "preconditioner": preconditioner,
                    "slug": string_field(&result, "slug"),
                    "median_ms": number_field(&result, "median_ms"),
                    "peak_rss_mib": number_field(&result, "peak_rss_mib"),
                    "solver_preconditioner_reason": result["solver_preconditioner_reason"].clone(),
                    "solver_iterations": result["solver_iterations"].clone(),
                    "solver_residual_norm": result["solver_residual_norm"].clone(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn case_solver_metrics(run: &Value, case_id: &str, preconditioner: &str) -> Value {
    run.get("solver_case_metrics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|metrics| {
            string_field(metrics, "id") == case_id
                && string_field(metrics, "solver_preconditioner") == preconditioner
        })
        .cloned()
        .unwrap_or(Value::Null)
}

struct MatrixSummary {
    case_count: f64,
    matrix: String,
    peak_rss_mib: f64,
    run_count: f64,
    slowest_case: String,
    slowest_case_median_ms: f64,
    total_median_ms: f64,
}

impl MatrixSummary {
    fn from_run(run: &Value) -> Self {
        Self {
            case_count: 0.0,
            matrix: string_field(run, "matrix"),
            peak_rss_mib: 0.0,
            run_count: 0.0,
            slowest_case: "--".to_string(),
            slowest_case_median_ms: 0.0,
            total_median_ms: 0.0,
        }
    }

    fn add(&mut self, run: &Value) {
        self.run_count += 1.0;
        self.case_count += number_field(run, "case_count");
        self.total_median_ms += number_field(run, "total_median_ms");
        self.peak_rss_mib = self.peak_rss_mib.max(number_field(run, "peak_rss_mib"));
        if number_field(run, "total_median_ms") > self.slowest_case_median_ms {
            self.slowest_case = string_field(run, "slowest_case");
            self.slowest_case_median_ms = number_field(run, "total_median_ms");
        }
    }

    fn into_value(self) -> Value {
        json!({ "matrix": self.matrix, "run_count": self.run_count as u64, "case_count": self.case_count as u64, "total_median_ms": self.total_median_ms, "peak_rss_mib": self.peak_rss_mib, "slowest_case": self.slowest_case, "slowest_case_median_ms": self.slowest_case_median_ms })
    }
}
