use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const CONTRACT_PATH: &str = "deploy/installation-integrity-contract.json";
const CHANNELS_PATH: &str = "deploy/update-channels.json";
const DOCS_OUT: &str = "docs/installation-integrity-contract.html";
const HUB_OUT: &str = "apps/hub-gui/ui/docs/installation-integrity.html";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_installation_integrity_docs(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner build-installation-integrity-docs [--self-test]");
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("installation integrity docs self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("build-installation-integrity-docs only accepts --self-test".to_string());
    }

    let mut contract = read_json(root, CONTRACT_PATH)?;
    let channels = read_json(root, CHANNELS_PATH)?;
    merge_shipping_version(&mut contract, &channels);

    let docs_html = render_html(
        &contract,
        RenderOptions {
            css_href: "../apps/hub-gui/ui/docs/docs.css",
            extra_copy: "",
            json_href: "../deploy/installation-integrity-contract.json",
            kicker: "Installation Contract",
            source_href: None,
        },
    );
    let hub_html = render_html(
        &contract,
        RenderOptions {
            css_href: "./docs.css",
            extra_copy: "This is the Hub mirror for the trust-and-safety chapter's installation integrity material.",
            json_href: "../../../../deploy/installation-integrity-contract.json",
            kicker: "Installation Contract Mirror · Chapter 7",
            source_href: Some("../../../../docs/installation-integrity-contract.html"),
        },
    );

    write_text(root, DOCS_OUT, &docs_html)?;
    write_text(root, HUB_OUT, &hub_html)?;
    println!("wrote {DOCS_OUT}");
    println!("wrote {HUB_OUT}");
    Ok(0)
}

struct RenderOptions<'a> {
    css_href: &'a str,
    extra_copy: &'a str,
    json_href: &'a str,
    kicker: &'a str,
    source_href: Option<&'a str>,
}

fn render_html(contract: &Value, options: RenderOptions<'_>) -> String {
    let version = escape_html(string_field(contract, "shipping_version").unwrap_or("unknown"));
    let product_line =
        escape_html(string_field(contract, "product_line").unwrap_or("tamamono 1.x"));
    let schema_version = escape_html(
        string_field(contract, "schema_version").unwrap_or("kyuubiki.installation-contract/v1"),
    );
    let extra_copy = if options.extra_copy.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"docs-copy\">\n          {}\n        </p>",
            escape_html(options.extra_copy)
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Installation Integrity Contract</title>
    <link rel="stylesheet" href="{}" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">{}</div>
        <h1>{} installation integrity</h1>
        <p class="docs-copy">
          Generated from <code>deploy/installation-integrity-contract.json</code>.
          This page exposes the same contract used by the desktop installer integrity report and repair workflow.
        </p>
        {}
        <div class="docs-meta">
          <span class="docs-chip">Shipping version: {}</span>
          <span class="docs-chip">Schema: {}</span>
        </div>
        <div class="docs-links">
          {}
        </div>
      </section>

      <section class="docs-stack">
        <article class="docs-card">
          <h2>Visible behavior contract</h2>
          <p class="docs-copy">
            Every repair-side behavior should be operator-visible. Some rules are intentionally read-only,
            but none of them should be hidden.
          </p>
          <div class="docs-stack">
            {}
          </div>
        </article>

        <article class="docs-card">
          <h2>Required install layout</h2>
          <ul class="docs-list">
            {}
          </ul>
        </article>

        <article class="docs-card">
          <h2>Protected paths</h2>
          <ul class="docs-list">
            {}
          </ul>
        </article>

        <article class="docs-card">
          <h2>Cleanup allowlist</h2>
          <ul class="docs-list">
            {}
          </ul>
        </article>
      </section>
    </main>
  </body>
</html>
"#,
        escape_html(options.css_href),
        escape_html(options.kicker),
        product_line,
        extra_copy,
        version,
        schema_version,
        render_links(&doc_links(&options)),
        render_rule_cards(array_field(contract, "visible_rules")),
        render_required_layout(array_field(contract, "required_layout")),
        render_string_list(array_field(contract, "protected_paths")),
        render_string_list(array_field(contract, "removable_patterns")),
    )
}

fn doc_links<'a>(options: &'a RenderOptions<'a>) -> Vec<(&'a str, &'a str)> {
    if let Some(source_href) = options.source_href {
        vec![
            ("./index.html", "Back to book entry"),
            (
                "../../../../docs/book-ch07-trust-and-safety.html",
                "Open central chapter",
            ),
            (source_href, "Open source page"),
            (options.json_href, "Open JSON source"),
        ]
    } else {
        vec![
            ("./README.md", "Back to docs readme"),
            (options.json_href, "Open JSON source"),
        ]
    }
}

fn render_links(links: &[(&str, &str)]) -> String {
    links
        .iter()
        .map(|(href, label)| {
            format!(
                "<a class=\"docs-link\" href=\"{}\">{}</a>",
                escape_html(href),
                escape_html(label)
            )
        })
        .collect::<Vec<_>>()
        .join("\n          ")
}

fn render_rule_cards(rules: &[Value]) -> String {
    rules
        .iter()
        .map(|rule| {
            format!(
                "<article class=\"docs-card\">\n    <div class=\"docs-kicker\">{}</div>\n    <h3>{}</h3>\n    <p class=\"docs-copy\">{}</p>\n    <p class=\"docs-copy\"><strong>Value:</strong> <code>{}</code></p>\n    <p class=\"docs-copy\"><strong>Mode:</strong> {}</p>\n  </article>",
                escape_html(string_field(rule, "category").unwrap_or("rule")),
                escape_html(string_field(rule, "label").unwrap_or("Untitled rule")),
                escape_html(string_field(rule, "description").unwrap_or("")),
                escape_html(string_field(rule, "value").unwrap_or("")),
                if bool_field(rule, "editable") { "Editable" } else { "Read-only" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_required_layout(entries: &[Value]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "<li><strong>{}</strong>: <code>{}</code>{}</li>",
                escape_html(string_field(entry, "label").unwrap_or("")),
                escape_html(string_field(entry, "path").unwrap_or("")),
                if bool_field(entry, "required") {
                    " (required)"
                } else {
                    " (optional)"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_string_list(entries: &[Value]) -> String {
    entries
        .iter()
        .filter_map(Value::as_str)
        .map(|item| format!("<li><code>{}</code></li>", escape_html(item)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn merge_shipping_version(contract: &mut Value, channels: &Value) {
    if contract.get("shipping_version").is_none()
        && let Some(version) = channels.get("shipping_version").cloned()
        && let Some(object) = contract.as_object_mut()
    {
        object.insert("shipping_version".to_string(), version);
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn write_text(root: &Path, relative_path: &str, text: &str) -> RunnerResult<()> {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn run_self_test() -> RunnerResult<()> {
    let mut contract = serde_json::json!({
        "schema_version": "kyuubiki.installation-contract/v1",
        "product_line": "tamamono 1.x",
        "visible_rules": [{
            "category": "cleanup",
            "label": "visible cleanup",
            "description": "All cleanup is visible.",
            "value": "allowlisted",
            "editable": false
        }],
        "required_layout": [{
            "label": "runtime root",
            "path": "Library/Application Support/Kyuubiki",
            "required": true
        }],
        "protected_paths": ["tmp/data"],
        "removable_patterns": ["dist/macos"]
    });
    merge_shipping_version(
        &mut contract,
        &serde_json::json!({ "shipping_version": "1.20.0" }),
    );
    let html = render_html(
        &contract,
        RenderOptions {
            css_href: "./docs.css",
            extra_copy: "mirror copy",
            json_href: "../../../../deploy/installation-integrity-contract.json",
            kicker: "Installation Contract Mirror · Chapter 7",
            source_href: Some("../../../../docs/installation-integrity-contract.html"),
        },
    );
    for token in [
        "tamamono 1.x installation integrity",
        "Shipping version: 1.20.0",
        "Open source page",
        "visible cleanup",
        "Read-only",
    ] {
        if !html.contains(token) {
            return Err(format!("self-test html missing token: {token}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{escape_html, run_self_test};

    #[test]
    fn self_test_renders_expected_contract_tokens() {
        run_self_test().unwrap();
    }

    #[test]
    fn html_escape_covers_basic_metacharacters() {
        assert_eq!(
            escape_html("<tag & \"value\">"),
            "&lt;tag &amp; &quot;value&quot;&gt;"
        );
    }
}
