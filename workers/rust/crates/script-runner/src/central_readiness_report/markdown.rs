use super::{
    CONFIG_FILES, ENDPOINTS, REPORT_SCHEMA, SCHEMA_FILES, STORAGE_SCHEMA, unsafe_text_issues,
};
use serde_json::Value;

pub(super) fn validate_markdown(markdown: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let mut required = vec![
        "# Central Readiness Report",
        "## API Surface",
        "## Schemas",
        "## Config Surface",
        "## Service Surface",
        "## Storage Contract",
        "## Language Pack Publish Contract",
        "## Publish Pipeline Contract",
        "## Runbook",
        REPORT_SCHEMA,
        STORAGE_SCHEMA,
        "kyuubiki.central-publish-pipeline/v1",
        "central-web-service",
        "orchestra-control-plane",
        "self_host_web",
        "publisher_identity",
        "download_verification",
        "unsafe_text_scan",
        "make check-central-database-readiness MODE=local BACKEND=sqlite",
        "make remote-central-database-smoke REMOTE=kyuubiki-lab",
        "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke",
    ];
    required.extend(ENDPOINTS);
    required.extend(SCHEMA_FILES);
    required.extend(CONFIG_FILES);
    for text in required {
        if !markdown.contains(text) {
            issues.push(format!("markdown missing {text}"));
        }
    }
    issues.extend(unsafe_text_issues(markdown));
    issues
}

pub(super) fn render_markdown(report: &Value) -> String {
    let mut lines = vec![
        "# Central Readiness Report".to_string(),
        String::new(),
        format!("- Schema: `{}`", str_at(report, "/schema_version")),
        format!("- Status: `{}`", str_at(report, "/status")),
        format!(
            "- Mode/backend: `{}/{}`",
            str_at(report, "/mode"),
            str_at(report, "/backend")
        ),
        format!("- Generated: `{}`", str_at(report, "/generated_at")),
        String::new(),
        "## API Surface".to_string(),
        String::new(),
        "| Endpoint | Router | Client |".to_string(),
        "| --- | --- | --- |".to_string(),
    ];
    for endpoint in array_at(report, "/api_surface/endpoints") {
        lines.push(format!(
            "| `{}` | `{}` | `{}` |",
            str_field(endpoint, "path"),
            yes(endpoint, "router_present"),
            yes(endpoint, "client_present")
        ));
    }
    render_schema_section(report, &mut lines);
    render_config_section(report, &mut lines);
    render_language_pack_section(report, &mut lines);
    render_pipeline_section(report, &mut lines);
    render_tail(report, &mut lines);
    lines.join("\n")
}

pub(super) fn markdown_fixture(report: &Value) -> String {
    render_markdown(report)
}

fn render_schema_section(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Schemas".to_string(),
        String::new(),
        "| Schema | Present |".to_string(),
        "| --- | --- |".to_string(),
    ]);
    for schema in array_at(report, "/schema_surface/schema_files") {
        lines.push(format!(
            "| `{}` | `{}` |",
            str_field(schema, "path"),
            yes(schema, "present")
        ));
    }
}

fn render_config_section(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Config Surface".to_string(),
        String::new(),
        "| Config | Present | Schema Version |".to_string(),
        "| --- | --- | --- |".to_string(),
    ]);
    for config in array_at(report, "/config_surface/config_files") {
        lines.push(format!(
            "| `{}` | `{}` | `{}` |",
            str_field(config, "path"),
            yes(config, "present"),
            yes(config, "schema_version_present")
        ));
    }
}

fn render_language_pack_section(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Language Pack Publish Contract".to_string(),
        String::new(),
        format!(
            "- Evidence: `{}`",
            array_at(report, "/language_pack_publish_contract/required_evidence")
                .into_iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!(
            "- Unsafe text scan: `{}`",
            yes_at(
                report,
                "/language_pack_publish_contract/unsafe_text_scan_present"
            )
        ),
        format!(
            "- Docs coverage: `{}`",
            yes_at(report, "/language_pack_publish_contract/docs_present")
        ),
    ]);
}

fn render_pipeline_section(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Publish Pipeline Contract".to_string(),
        String::new(),
        format!(
            "- Contract: `{}`",
            str_at(report, "/publish_pipeline_contract/schema_version")
        ),
        format!(
            "- Status: `{}`",
            str_at(report, "/publish_pipeline_contract/status")
        ),
        format!(
            "- Accepting writes: `{}`",
            yes_at(report, "/publish_pipeline_contract/accepting_writes")
        ),
        format!(
            "- Read-only guard: `{}`",
            yes_at(report, "/publish_pipeline_contract/readonly_guard_present")
        ),
        format!(
            "- Docs coverage: `{}`",
            yes_at(report, "/publish_pipeline_contract/docs_present")
        ),
        String::new(),
        "| Stage | Present |".to_string(),
        "| --- | --- |".to_string(),
    ]);
    for stage in array_at(report, "/publish_pipeline_contract/stages_present") {
        lines.push(format!(
            "| `{}` | `{}` |",
            str_field(stage, "id"),
            yes(stage, "present")
        ));
    }
}

fn render_tail(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Service Surface".to_string(),
        String::new(),
        "| Service | Module | Kind | Topology | Boundary |".to_string(),
        "| --- | --- | --- | --- | --- |".to_string(),
        format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` |",
            str_at(report, "/service_surface/id"),
            str_at(report, "/service_surface/module_id"),
            str_at(report, "/service_surface/kind"),
            yes_at(report, "/service_surface/topology_present"),
            yes_at(report, "/service_surface/boundary_documented")
        ),
        String::new(),
        "## Storage Contract".to_string(),
        String::new(),
        format!(
            "- Contract: `{}`",
            str_at(report, "/storage_contract/schema_version")
        ),
        format!(
            "- Table contract present: `{}`",
            yes_at(report, "/storage_contract/table_contract_present")
        ),
        String::new(),
        "## Runbook".to_string(),
        String::new(),
        format!(
            "- Local readiness: `{}`",
            str_at(report, "/runbook/local_readiness")
        ),
        format!(
            "- Remote dry-run: `{}`",
            str_at(report, "/runbook/remote_dry_run")
        ),
        format!(
            "- Postgres smoke: `{}`",
            str_at(report, "/runbook/postgres_smoke")
        ),
        String::new(),
    ]);
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Vec<&'a Value> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn yes(value: &Value, key: &str) -> &'static str {
    if value.get(key).and_then(Value::as_bool) == Some(true) {
        "yes"
    } else {
        "no"
    }
}

fn yes_at(value: &Value, pointer: &str) -> &'static str {
    if value.pointer(pointer).and_then(Value::as_bool) == Some(true) {
        "yes"
    } else {
        "no"
    }
}
