use super::{RunnerResult, array, field, finite};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(super) fn write_markdown(path: &Path, summary: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut lines = vec![
        "# Remote Material Benchmark Summary".to_string(),
        String::new(),
        format!("Generated: {}", field(summary, "generated_at_utc")),
        String::new(),
        format!("Runs: {}", summary.get("run_count").unwrap_or(&Value::Null)),
        String::new(),
        format!(
            "Cases: {}",
            summary.get("case_count").unwrap_or(&Value::Null)
        ),
        String::new(),
        "## Latest Cases".to_string(),
        String::new(),
        "| Matrix | Profile | Case | Median ms | RSS MiB | DOF | Iterations | Residual |"
            .to_string(),
        "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |".to_string(),
    ];
    for item in array(summary, "latest_cases") {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            field(item, "matrix"),
            field(item, "profile"),
            field(item, "case_id"),
            fixed(item, "median_ms", 2),
            fixed(item, "peak_rss_mib", 1),
            item.get("dof_count").unwrap_or(&Value::Null),
            item.get("solver_iterations").unwrap_or(&Value::Null),
            item.get("residual_norm").unwrap_or(&Value::Null),
        ));
    }
    lines.extend([
        String::new(),
        "## Latest Stage Hotspots".to_string(),
        String::new(),
        "| Rank | Matrix | Profile | Case | Stage | Elapsed ms | Share |".to_string(),
        "| ---: | --- | --- | --- | --- | ---: | ---: |".to_string(),
    ]);
    for (index, item) in array(summary, "latest_stage_hotspots").iter().enumerate() {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {}% |",
            index + 1,
            field(item, "matrix"),
            field(item, "profile"),
            field(item, "case_id"),
            field(item, "stage"),
            fixed(item, "elapsed_ms", 2),
            fixed(item, "stage_share_pct", 1),
        ));
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn fixed(value: &Value, key: &str, digits: usize) -> String {
    finite(value, key).map_or_else(|| "n/a".to_string(), |value| format!("{value:.digits$}"))
}
