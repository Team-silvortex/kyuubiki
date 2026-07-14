use serde_json::Value;

pub(super) fn render_readme(root: &str, lanes: &[Value]) -> String {
    let mut lines = vec![
        "# Tmp Workspace Map".to_string(),
        String::new(),
        format!("- Root: `{root}`"),
        "- Purpose: disposable local runtime state, benchmark artifacts, and nightly comparison outputs.".to_string(),
        "- Cross-lane regression catalog: `regression-lane-catalog.json`, `regression-lane-catalog.md`, `regression-lane-catalog.html`.".to_string(),
        "- Gate report: `regression-gate-report.json`, `regression-gate-report.md`.".to_string(),
        String::new(),
        "## Nightly lanes".to_string(),
        String::new(),
    ];
    for lane in lanes {
        lines.push(format!("- `{}`", str_at(lane, "/id")));
        lines.push(format!("  {}", str_at(lane, "/summary")));
        lines.push(format!(
            "  Generated at unix: `{}`",
            int_at(lane, "/generatedAtUnixS")
        ));
        lines.push(format!("  Links: {}", rendered_links(lane)));
    }
    lines.push(String::new());
    format!("{}\n", lines.join("\n").trim_end())
}

pub(super) fn render_html(root: &str, lanes: &[Value]) -> String {
    let cards = lanes.iter().map(render_card).collect::<Vec<_>>().join("\n");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Nightly Artifact Overview</title>
    <link rel="stylesheet" href="../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Tmp Nightly Overview</div>
        <h1>Local nightly artifact map</h1>
        <p class="docs-copy">
          This page indexes the latest local outputs for the current self-hosted nightly regression lanes.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Root: {}</span>
          <span class="docs-chip">Lanes indexed: {}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="./README.md">Open tmp README</a>
          <a class="docs-link" href="./nightly-overview.json">Open JSON index</a>
          <a class="docs-link" href="./regression-lane-catalog.html">Open regression catalog</a>
          <a class="docs-link" href="./regression-gate-report.json">Open gate report</a>
          <a class="docs-link" href="../docs/testing-and-ci.md">Open testing guide</a>
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
        cards
    )
}

fn render_card(lane: &Value) -> String {
    let detail = str_at(lane, "/detail");
    let detail_html = if detail.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="docs-copy"><strong>Detail:</strong> <code>{}</code></p>"#,
            escape_html(detail)
        )
    };
    format!(
        r#"<article class="docs-card">
        <div class="docs-kicker">nightly lane</div>
        <h2>{}</h2>
        <p class="docs-copy">{}</p>
        <div class="docs-meta">
          <span class="docs-chip">Lane: {}</span>
          <span class="docs-chip">Generated at unix: {}</span>
        </div>
        {}
        <ul class="docs-list">
          {}
        </ul>
      </article>"#,
        escape_html(str_at(lane, "/title")),
        escape_html(str_at(lane, "/summary")),
        escape_html(str_at(lane, "/id")),
        int_at(lane, "/generatedAtUnixS"),
        detail_html,
        list_links(lane),
    )
}

fn rendered_links(lane: &Value) -> String {
    lane["links"]
        .as_array()
        .map(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .map(|item| format!("`{item}`"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn list_links(lane: &Value) -> String {
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

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn int_at(value: &Value, pointer: &str) -> i64 {
    value.pointer(pointer).and_then(Value::as_i64).unwrap_or(0)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
