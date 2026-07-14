use crate::native_time::utc_iso_timestamp;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const RELEASE_INDEX: &str = "releases/index.json";
const CHANNELS_PATH: &str = "deploy/update-channels.json";
const CATALOG_OUT: &str = "releases/update-catalog.json";
const DOCS_OUT: &str = "docs/update-catalog.html";
const HUB_OUT: &str = "apps/hub-gui/ui/docs/update-catalog.html";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_update_catalog(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner build-update-catalog [--self-test]");
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("update catalog self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("build-update-catalog only accepts --self-test".to_string());
    }

    let catalog = build_catalog(root)?;
    write_json(root, CATALOG_OUT, &catalog)?;
    write_text(
        root,
        DOCS_OUT,
        &render_html(
            &catalog,
            RenderOptions {
                css_href: "../apps/hub-gui/ui/docs/docs.css",
                extra_copy: "",
                kicker: "Unified update catalog",
                links: vec![
                    ("./README.md", "Back to docs readme"),
                    ("../releases/update-catalog.json", "Open JSON source"),
                ],
            },
        ),
    )?;
    write_text(
        root,
        HUB_OUT,
        &render_html(
            &catalog,
            RenderOptions {
                css_href: "./docs.css",
                extra_copy: "This is the Hub mirror for the trust-and-safety chapter's update visibility material.",
                kicker: "Unified Update Catalog Mirror · Chapter 7",
                links: vec![
                    ("./index.html", "Back to book entry"),
                    (
                        "../../../../docs/book-ch07-trust-and-safety.html",
                        "Open central chapter",
                    ),
                    ("../../../../docs/update-catalog.html", "Open source page"),
                ],
            },
        ),
    )?;
    println!("wrote {CATALOG_OUT}");
    println!("wrote {DOCS_OUT}");
    println!("wrote {HUB_OUT}");
    Ok(0)
}

fn build_catalog(root: &Path) -> RunnerResult<Value> {
    let release_index = read_json(root, RELEASE_INDEX)?;
    let channels = read_json(root, CHANNELS_PATH)?;
    let snapshots = load_snapshots(root, &release_index)?;
    let catalog_channels = build_channels(root, &channels, &release_index, &snapshots)?;
    let versions = build_versions(root, &snapshots, &catalog_channels);
    Ok(json!({
        "schema_version": "kyuubiki.update-catalog/v1",
        "generated_at": utc_iso_timestamp(),
        "shipping_version": channels.get("shipping_version").cloned().unwrap_or(Value::Null),
        "default_channel": channels.get("default_channel").cloned().unwrap_or(Value::Null),
        "line": channels.get("line").cloned().or_else(|| release_index.get("line").cloned()).unwrap_or(Value::Null),
        "source": {
            "release_index": RELEASE_INDEX,
            "channel_contract": CHANNELS_PATH,
        },
        "channels": catalog_channels,
        "versions": versions,
    }))
}

fn load_snapshots(root: &Path, release_index: &Value) -> RunnerResult<Vec<SnapshotRef>> {
    let mut snapshots = Vec::new();
    for entry in array_field(release_index, "snapshots") {
        let version = string_field(entry, "version").unwrap_or_default();
        let snapshot_path = string_field(entry, "snapshot_path").unwrap_or_default();
        if version.is_empty() || snapshot_path.is_empty() {
            continue;
        }
        snapshots.push(SnapshotRef {
            snapshot: read_json(root, &format!("releases/{snapshot_path}"))?,
            snapshot_path: format!("releases/{snapshot_path}"),
            version: version.to_string(),
        });
    }
    Ok(snapshots)
}

#[derive(Clone)]
struct SnapshotRef {
    snapshot: Value,
    snapshot_path: String,
    version: String,
}

fn build_channels(
    root: &Path,
    channels: &Value,
    release_index: &Value,
    snapshots: &[SnapshotRef],
) -> RunnerResult<Vec<Value>> {
    array_field(channels, "channels")
        .iter()
        .map(|channel| {
            let version = string_field(channel, "version").unwrap_or_default();
            let snapshot_ref = snapshots
                .iter()
                .find(|item| item.version == version)
                .ok_or_else(|| {
                    format!(
                        "missing release snapshot for channel {}: {version}",
                        string_field(channel, "id").unwrap_or_default()
                    )
                })?;
            let snapshot = &snapshot_ref.snapshot;
            Ok(json!({
                "id": channel.get("id").cloned().unwrap_or(Value::Null),
                "label": channel.get("label").cloned().unwrap_or(Value::Null),
                "tag": channel.get("tag").cloned().unwrap_or(Value::Null),
                "aliases": channel.get("aliases").cloned().unwrap_or_else(|| json!([])),
                "line": snapshot.get("line").cloned().or_else(|| channels.get("line").cloned()).or_else(|| release_index.get("line").cloned()).unwrap_or(Value::Null),
                "status": snapshot.get("status").cloned().unwrap_or(Value::Null),
                "version": snapshot.get("version").cloned().unwrap_or(Value::Null),
                "summary": snapshot.get("summary").cloned().unwrap_or(Value::Null),
                "date": snapshot.get("date").cloned().unwrap_or(Value::Null),
                "notes": channel.get("notes").cloned().unwrap_or_else(|| json!([])),
                "visible_rules": channel.get("visible_rules").cloned().unwrap_or_else(|| json!([])),
                "rollout": channel.get("rollout").cloned().unwrap_or_else(|| json!({})),
                "snapshot_path": snapshot_ref.snapshot_path,
                "docs": snapshot.get("docs").cloned().unwrap_or_else(|| json!({})),
                "product_surfaces": snapshot.get("product_surfaces").cloned().unwrap_or_else(|| json!({})),
                "desktop_artifacts": collect_artifacts(root, snapshot),
            }))
        })
        .collect()
}

fn build_versions(
    root: &Path,
    snapshots: &[SnapshotRef],
    catalog_channels: &[Value],
) -> Vec<Value> {
    snapshots
        .iter()
        .map(|snapshot_ref| {
            let snapshot = &snapshot_ref.snapshot;
            let version = string_field(snapshot, "version").unwrap_or_default();
            let bound_channels = catalog_channels
                .iter()
                .filter(|channel| string_field(channel, "version") == Some(version))
                .filter_map(|channel| channel.get("id").cloned())
                .collect::<Vec<_>>();
            let bound_tags = catalog_channels
                .iter()
                .filter(|channel| string_field(channel, "version") == Some(version))
                .flat_map(|channel| {
                    let mut tags = Vec::new();
                    if let Some(tag) = channel.get("tag").cloned() {
                        tags.push(tag);
                    }
                    tags.extend(array_field(channel, "aliases").iter().cloned());
                    tags
                })
                .collect::<Vec<_>>();
            json!({
                "version": snapshot.get("version").cloned().unwrap_or(Value::Null),
                "line": snapshot.get("line").cloned().unwrap_or(Value::Null),
                "status": snapshot.get("status").cloned().unwrap_or(Value::Null),
                "date": snapshot.get("date").cloned().unwrap_or(Value::Null),
                "summary": snapshot.get("summary").cloned().unwrap_or(Value::Null),
                "snapshot_path": snapshot_ref.snapshot_path,
                "channels": bound_channels,
                "tags": bound_tags,
                "product_surfaces": snapshot.get("product_surfaces").cloned().unwrap_or_else(|| json!({})),
                "desktop_artifacts": collect_artifacts(root, snapshot),
            })
        })
        .collect()
}

fn collect_artifacts(root: &Path, snapshot: &Value) -> Vec<Value> {
    snapshot
        .get("desktop_artifacts")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|(key, value)| {
            let relative_path = value.as_str()?;
            key.contains('_')
                .then(|| artifact_entry(root, key, relative_path))
        })
        .collect()
}

fn artifact_entry(root: &Path, key: &str, relative_path: &str) -> Value {
    let tokens = key.split('_').collect::<Vec<_>>();
    let platform_token = tokens
        .iter()
        .enumerate()
        .find(|(index, token)| *index > 0 && matches!(**token, "macos" | "linux" | "windows"))
        .map(|(index, token)| (index, *token));
    let kind = platform_token.map_or_else(
        || tokens.last().copied().unwrap_or("unknown").to_string(),
        |(index, _)| tokens[(index + 1)..].join("_"),
    );
    let platform = platform_token
        .map(|(_, token)| token.to_string())
        .unwrap_or_else(|| infer_platform(&kind).to_string());
    json!({
        "product": tokens.first().copied().unwrap_or("unknown"),
        "kind": kind,
        "platform": platform,
        "path": relative_path,
        "exists": root.join(relative_path).exists(),
    })
}

fn infer_platform(kind: &str) -> &'static str {
    match kind {
        "dmg" | "app" => "macos",
        "appimage" | "deb" | "rpm" => "linux",
        "msi" | "nsis" => "windows",
        _ => "unknown",
    }
}

struct RenderOptions<'a> {
    css_href: &'a str,
    extra_copy: &'a str,
    kicker: &'a str,
    links: Vec<(&'a str, &'a str)>,
}

fn render_html(catalog: &Value, options: RenderOptions<'_>) -> String {
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
    <title>Kyuubiki Unified Update Catalog</title>
    <link rel="stylesheet" href="{}" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">{}</div>
        <h1>{} delivery channels</h1>
        <p class="docs-copy">
          This page is generated from the shared release index and the human-owned update channel contract.
          It defines the visible, Docker-like update tags that point to concrete shipped versions.
        </p>
        {}
        <div class="docs-meta">
          <span class="docs-chip">Shipping version: {}</span>
          <span class="docs-chip">Default channel: {}</span>
          <span class="docs-chip">Schema: {}</span>
        </div>
        <div class="docs-links">
          {}
        </div>
      </section>

      <section class="docs-section">
        <article class="docs-card">
          <div class="docs-kicker">source of truth</div>
          <h2>How this stays unified</h2>
          <ul class="docs-list">
            <li><strong>channel contract</strong>: <code>{}</code></li>
            <li><strong>release registry</strong>: <code>{}</code></li>
            <li><strong>tag model</strong>: human-facing channels point to immutable shipped versions</li>
            <li><strong>installer posture</strong>: update behavior must stay visible and bounded by the installation contract</li>
          </ul>
        </article>
      </section>

      <section class="docs-grid">
        {}
      </section>
    </main>
  </body>
</html>
"#,
        escape_html(options.css_href),
        escape_html(options.kicker),
        escape_html(string_field(catalog, "line").unwrap_or_default()),
        extra_copy,
        escape_html(string_field(catalog, "shipping_version").unwrap_or_default()),
        escape_html(string_field(catalog, "default_channel").unwrap_or_default()),
        escape_html(string_field(catalog, "schema_version").unwrap_or_default()),
        render_links(&options.links),
        escape_html(
            catalog
                .pointer("/source/channel_contract")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        escape_html(
            catalog
                .pointer("/source/release_index")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        render_channels(array_field(catalog, "channels")),
    )
}

fn render_channels(channels: &[Value]) -> String {
    channels
        .iter()
        .map(|channel| {
            format!(
                "\n        <article class=\"docs-card\">\n          <div class=\"docs-kicker\">channel</div>\n          <h2>{} <code>{}</code></h2>\n          <p class=\"docs-copy\">{}</p>\n          <div class=\"docs-meta\">\n            <span class=\"docs-chip\">Version: {}</span>\n            <span class=\"docs-chip\">Status: {}</span>\n            <span class=\"docs-chip\">Aliases: {}</span>\n          </div>\n          <h3>Visible rules</h3>\n          <ul class=\"docs-list\">\n            {}\n          </ul>\n          <h3>Desktop artifact references</h3>\n          <ul class=\"docs-list\">\n            {}\n          </ul>\n        </article>",
                escape_html(string_field(channel, "label").unwrap_or_default()),
                escape_html(string_field(channel, "tag").unwrap_or_default()),
                escape_html(string_field(channel, "summary").unwrap_or_default()),
                escape_html(string_field(channel, "version").unwrap_or_default()),
                escape_html(string_field(channel, "status").unwrap_or_default()),
                escape_html(&join_string_array(array_field(channel, "aliases"))),
                render_rules(array_field(channel, "visible_rules")),
                render_artifacts(array_field(channel, "desktop_artifacts")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_rules(rules: &[Value]) -> String {
    rules
        .iter()
        .map(|rule| {
            format!(
                "<li><strong>{}</strong>: {}<br />{}</li>",
                escape_html(string_field(rule, "label").unwrap_or_default()),
                escape_html(string_field(rule, "value").unwrap_or_default()),
                escape_html(string_field(rule, "description").unwrap_or_default())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_artifacts(artifacts: &[Value]) -> String {
    artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<li><strong>{}</strong> · {} · {} · <code>{}</code> · {}</li>",
                escape_html(string_field(artifact, "product").unwrap_or_default()),
                escape_html(string_field(artifact, "platform").unwrap_or_default()),
                escape_html(string_field(artifact, "kind").unwrap_or_default()),
                escape_html(string_field(artifact, "path").unwrap_or_default()),
                if bool_field(artifact, "exists") {
                    "present"
                } else {
                    "declared"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn write_json(root: &Path, relative_path: &str, value: &Value) -> RunnerResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {relative_path}: {error}"))?;
    write_text(root, relative_path, &format!("{text}\n"))
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

fn join_string_array(items: &[Value]) -> String {
    let joined = items
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if joined.is_empty() {
        "none".to_string()
    } else {
        joined
    }
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
    let artifact = artifact_entry(
        Path::new("."),
        "hub_macos_manifest",
        "dist/macos/desktop/hub-gui/manifest.json",
    );
    if string_field(&artifact, "platform") != Some("macos")
        || string_field(&artifact, "kind") != Some("manifest")
        || string_field(&artifact, "product") != Some("hub")
    {
        return Err(format!("artifact inference drifted: {artifact}"));
    }
    let catalog = json!({
        "schema_version": "kyuubiki.update-catalog/v1",
        "shipping_version": "1.20.0",
        "default_channel": "stable",
        "line": "tamamono 1.x",
        "source": {
            "release_index": RELEASE_INDEX,
            "channel_contract": CHANNELS_PATH
        },
        "channels": [{
            "id": "stable",
            "label": "Stable",
            "tag": "tamamono:stable",
            "aliases": ["tamamono:latest"],
            "version": "1.20.0",
            "status": "current",
            "summary": "Current line.",
            "visible_rules": [{
                "label": "preflight integrity",
                "value": "required",
                "description": "Run integrity before updating."
            }],
            "desktop_artifacts": [artifact]
        }]
    });
    let html = render_html(
        &catalog,
        RenderOptions {
            css_href: "./docs.css",
            extra_copy: "mirror copy",
            kicker: "Unified Update Catalog Mirror · Chapter 7",
            links: vec![("./index.html", "Back to book entry")],
        },
    );
    for token in [
        "tamamono 1.x delivery channels",
        "Shipping version: 1.20.0",
        "preflight integrity",
        "hub",
        "declared",
    ] {
        if !html.contains(token) {
            return Err(format!("self-test html missing token: {token}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{artifact_entry, escape_html, infer_platform, run_self_test, string_field};
    use std::path::Path;

    #[test]
    fn self_test_renders_update_catalog_tokens() {
        run_self_test().unwrap();
    }

    #[test]
    fn artifact_entry_infers_platform_and_kind() {
        let item = artifact_entry(Path::new("."), "hub_macos_dmg", "dist/hub.dmg");
        assert_eq!(string_field(&item, "product"), Some("hub"));
        assert_eq!(string_field(&item, "platform"), Some("macos"));
        assert_eq!(string_field(&item, "kind"), Some("dmg"));
        assert_eq!(infer_platform("rpm"), "linux");
    }

    #[test]
    fn html_escape_covers_basic_metacharacters() {
        assert_eq!(
            escape_html("<tag & \"value\">"),
            "&lt;tag &amp; &quot;value&quot;&gt;"
        );
    }
}
