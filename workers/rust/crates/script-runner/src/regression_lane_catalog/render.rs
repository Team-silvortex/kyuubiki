use serde_json::Value;

pub(super) fn render_readme(root: &str, lanes: &[Value]) -> String {
    let overall = worst_gate_status(&enforceable_gate_statuses(lanes));
    let mut lines = vec![
        "# Regression Lane Catalog".to_string(),
        String::new(),
        format!("- Root: `{root}`"),
        format!("- Lanes indexed: `{}`", lanes.len()),
        format!("- Overall gate status: `{overall}`"),
        String::new(),
        "| Lane | Category | Status | Gate | Generated (unix) | Summary |".to_string(),
        "| --- | --- | --- | --- | ---: | --- |".to_string(),
    ];
    for lane in lanes {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | {} |",
            str_at(lane, "/id"),
            str_at(lane, "/category"),
            str_at(lane, "/status"),
            str_at(lane, "/gate/status"),
            int_at(lane, "/generated_at_unix_s"),
            str_at(lane, "/summary"),
        ));
    }
    lines.push(String::new());
    format!("{}\n", lines.join("\n").trim_end())
}

pub(super) fn render_html(root: &str, lanes: &[Value]) -> String {
    let overall = worst_gate_status(&enforceable_gate_statuses(lanes));
    let cards = lanes.iter().map(render_card).collect::<Vec<_>>().join("\n");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Regression Lane Catalog</title>
    <link rel="stylesheet" href="../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Regression Catalog</div>
        <h1>Unified regression lane view</h1>
        <p class="docs-copy">
          This page normalizes the latest retained outputs across direct-mesh, workflow-catalog, and workflow-mesh lanes into a shared read model.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Root: {}</span>
          <span class="docs-chip">Lanes indexed: {}</span>
          <span class="docs-chip">Overall gate: {}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="./regression-lane-catalog.json">Open JSON catalog</a>
          <a class="docs-link" href="./regression-lane-catalog.md">Open Markdown catalog</a>
          <a class="docs-link" href="./nightly-overview.html">Open nightly overview</a>
        </div>
      </section>
      <section class="docs-grid">
        {}
      </section>
    </main>
  </body>
</html>
"#,
        escape_html(root),
        lanes.len(),
        escape_html(overall),
        cards
    )
}

fn render_card(lane: &Value) -> String {
    let reasons = lane
        .pointer("/gate/reasons")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .map(|items| {
            let joined = items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" | ");
            format!(
                r#"<p class="docs-copy"><strong>Gate reasons:</strong> {}</p>"#,
                escape_html(&joined)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<article class="docs-card">
        <div class="docs-kicker">{}</div>
        <h2>{}</h2>
        <p class="docs-copy">{}</p>
        <div class="docs-meta">
          <span class="docs-chip">Lane: {}</span>
          <span class="docs-chip">Status: {}</span>
          <span class="docs-chip">Gate: {}</span>
          <span class="docs-chip">Scope: {}</span>
          <span class="docs-chip">Generated at unix: {}</span>
        </div>
        {}
        <h3>Metrics</h3>
        <ul class="docs-list">
          {}
        </ul>
        <h3>Artifacts</h3>
        <ul class="docs-list">
          {}
        </ul>
      </article>"#,
        escape_html(str_at(lane, "/category")),
        escape_html(str_at(lane, "/title")),
        escape_html(str_at(lane, "/summary")),
        escape_html(str_at(lane, "/id")),
        escape_html(str_at(lane, "/status")),
        escape_html(str_at(lane, "/gate/status")),
        escape_html(lane["gate_scope"].as_str().unwrap_or("enforced")),
        int_at(lane, "/generated_at_unix_s"),
        reasons,
        render_metrics(lane),
        render_links(lane),
    )
}

fn render_metrics(lane: &Value) -> String {
    lane["metrics"]
        .as_array()
        .map(|metrics| {
            metrics
                .iter()
                .map(render_metric)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn render_metric(metric: &Value) -> String {
    let baseline = if metric["baseline"].is_null() {
        String::new()
    } else {
        format!(
            " · baseline <code>{}</code>",
            escape_html(&value_text(&metric["baseline"]))
        )
    };
    let delta = if metric["delta_pct"].is_null() {
        String::new()
    } else {
        format!(
            " · delta <code>{}%</code>",
            escape_html(&value_text(&metric["delta_pct"]))
        )
    };
    format!(
        "<li><code>{}</code>: <code>{}</code> {}{}{}</li>",
        escape_html(str_at(metric, "/name")),
        escape_html(&value_text(&metric["value"])),
        escape_html(str_at(metric, "/unit")),
        baseline,
        delta,
    )
}

fn render_links(lane: &Value) -> String {
    lane["links"]
        .as_array()
        .map(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .map(|item| format!("<li><code>{}</code></li>", escape_html(item)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn enforceable_gate_statuses(lanes: &[Value]) -> Vec<String> {
    lanes
        .iter()
        .filter(|lane| lane["gate_scope"].as_str() != Some("advisory"))
        .map(|lane| {
            lane.pointer("/gate/status")
                .or_else(|| lane.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("pass")
                .to_string()
        })
        .collect()
}

fn worst_gate_status(statuses: &[String]) -> &'static str {
    if statuses.iter().any(|status| status == "fail") {
        "fail"
    } else if statuses.iter().any(|status| status == "warn") {
        "warn"
    } else {
        "pass"
    }
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn int_at(value: &Value, pointer: &str) -> i64 {
    value.pointer(pointer).and_then(Value::as_i64).unwrap_or(0)
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
