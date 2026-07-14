use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod render;

type RunnerResult<T> = Result<T, String>;

const DEFAULT_TMP_ROOT: &str = "tmp";

pub(crate) fn run_build_nightly_artifact_overview(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let lanes = [
        read_direct_mesh_lane(&options.tmp_root)?,
        read_workflow_mesh_lane(&options.tmp_root)?,
        read_workflow_catalog_lane(&options.tmp_root)?,
        read_standard_lane(&options.tmp_root)?,
        read_benchmark_profile_lane(&options.tmp_root)?,
    ]
    .into_iter()
    .filter(Value::is_object)
    .collect::<Vec<_>>();
    let root = display_path(repo_root, &options.tmp_root);
    let payload = json!({
        "schema_version": "kyuubiki.nightly-artifact-overview/v1",
        "root": root,
        "generated_at_unix_s": unix_seconds_now(),
        "lanes": lanes,
    });

    fs::create_dir_all(&options.tmp_root).map_err(|error| {
        format!(
            "failed to create tmp root {}: {error}",
            options.tmp_root.display()
        )
    })?;
    write_json(&options.tmp_root.join("nightly-overview.json"), &payload)?;
    let empty_lanes = Vec::new();
    let payload_lanes = payload["lanes"].as_array().unwrap_or(&empty_lanes);
    fs::write(
        options.tmp_root.join("README.md"),
        render::render_readme(&root, payload_lanes),
    )
    .map_err(|error| format!("failed to write tmp README.md: {error}"))?;
    fs::write(
        options.tmp_root.join("nightly-overview.html"),
        render::render_html(&root, payload_lanes),
    )
    .map_err(|error| format!("failed to write nightly-overview.html: {error}"))?;
    println!("{root}");
    Ok(0)
}

struct Options {
    tmp_root: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        tmp_root: repo_root.join(DEFAULT_TMP_ROOT),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-nightly-artifact-overview [--tmp-root tmp]"
                );
                return Ok(options);
            }
            "--tmp-root" => options.tmp_root = path_arg(repo_root, &mut iter, "--tmp-root")?,
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    Ok(options)
}

fn read_standard_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let index_path = tmp_root.join("standard-benchmark/index.json");
    if !index_path.exists() {
        return Ok(Value::Null);
    }
    let payload = read_json(&index_path)?;
    let Some(run) = payload.pointer("/retained_runs/0") else {
        return Ok(json!({
            "id": "standard-benchmark",
            "title": "Standard benchmark nightly",
            "summary": "No retained standard benchmark runs yet.",
            "generatedAtUnixS": int_at(Some(&payload), "/generated_at_unix_s"),
            "links": ["standard-benchmark/index.html", "standard-benchmark/index.json", "standard-benchmark/README.md"],
        }));
    };
    Ok(json!({
        "id": "standard-benchmark",
        "title": "Standard benchmark nightly",
        "summary": format!("Latest retained run `{}` on profile `{}`.", str_at(run, "/slug"), str_at(run, "/profile")),
        "generatedAtUnixS": generated_at(run, &payload),
        "links": ["standard-benchmark/index.html", "standard-benchmark/index.json", format!("standard-benchmark/{}", str_at(run, "/merged_report"))],
        "detail": str_at(run, "/headline"),
    }))
}

fn read_direct_mesh_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let Some(run) = latest_run(&tmp_root.join("direct-mesh-benchmark-container"))? else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "id": "direct-mesh-docker",
        "title": "Direct-mesh Docker nightly",
        "summary": format!("Latest run `{}` from the LAN Docker regression harness.", run.slug),
        "generatedAtUnixS": run.generated_at_unix_s,
        "links": [
            format!("direct-mesh-benchmark-container/{}/summary.json", run.slug),
            format!("direct-mesh-benchmark-container/{}/compare.json", run.slug),
            format!("direct-mesh-benchmark-container/{}/compare.md", run.slug),
        ],
    }))
}

fn read_workflow_catalog_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let Some(run) = latest_run(&tmp_root.join("workflow-catalog-benchmark"))? else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "id": "workflow-catalog",
        "title": "Workflow catalog nightly",
        "summary": format!("Latest run `{}` from the orchestrated composite workflow regression path.", run.slug),
        "generatedAtUnixS": run.generated_at_unix_s,
        "links": [
            format!("workflow-catalog-benchmark/{}/summary.json", run.slug),
            format!("workflow-catalog-benchmark/{}/compare.json", run.slug),
            format!("workflow-catalog-benchmark/{}/compare.md", run.slug),
        ],
    }))
}

fn read_workflow_mesh_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let index_path = tmp_root.join("workflow-mesh-regression/index.json");
    if !index_path.exists() {
        return Ok(Value::Null);
    }
    let payload = read_json(&index_path)?;
    let Some(run) = payload.pointer("/retained_runs/0") else {
        return Ok(json!({
            "id": "workflow-mesh",
            "title": "Workflow mesh nightly",
            "summary": "No retained workflow mesh runs yet.",
            "generatedAtUnixS": int_at(Some(&payload), "/generated_at_unix_s"),
            "links": ["workflow-mesh-regression/index.html", "workflow-mesh-regression/index.json", "workflow-mesh-regression/README.md"],
        }));
    };
    let slug = str_at(run, "/slug");
    Ok(json!({
        "id": "workflow-mesh",
        "title": "Workflow mesh nightly",
        "summary": format!("Latest run `{slug}` from the remote distributed workflow mesh regression trio."),
        "generatedAtUnixS": generated_at(run, &payload),
        "links": [
            format!("workflow-mesh-regression/{slug}/summary.json"),
            format!("workflow-mesh-regression/{slug}/README.md"),
            format!("workflow-mesh-regression/{slug}/run.log"),
            "workflow-mesh-regression/index.html".to_string(),
            "workflow-mesh-regression/index.json".to_string(),
        ],
    }))
}

fn read_benchmark_profile_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let index_path = tmp_root.join("benchmark-profile/index.json");
    if !index_path.exists() {
        return Ok(Value::Null);
    }
    let payload = read_json(&index_path)?;
    let Some(run) = payload.pointer("/retained_runs/0") else {
        return Ok(json!({
            "id": "benchmark-profile",
            "title": "Benchmark profile exploration",
            "summary": "No retained exploratory benchmark profile runs yet.",
            "generatedAtUnixS": int_at(Some(&payload), "/generated_at_unix_s"),
            "links": ["benchmark-profile/index.json", "benchmark-profile/README.md"],
            "detail": format!("Gate {}.", str_at(&payload, "/gate/status")),
        }));
    };
    Ok(json!({
        "id": "benchmark-profile",
        "title": "Benchmark profile exploration",
        "summary": format!("Latest profile run `{}` for matrix `{}` on profile `{}`.", str_at(run, "/slug"), str_at(run, "/matrix"), str_at(run, "/profile")),
        "generatedAtUnixS": generated_at(run, &payload),
        "links": [
            "benchmark-profile/index.json".to_string(),
            "benchmark-profile/README.md".to_string(),
            format!("benchmark-profile/{}", str_at(run, "/files/summary_json")),
            format!("benchmark-profile/{}", str_at(run, "/files/readme_md")),
        ],
        "detail": format!(
            "Gate {}; cases {}; total median {:.3} ms; peak RSS {:.1} MiB.",
            str_at(&payload, "/gate/status"),
            int_at(Some(run), "/case_count"),
            number_at(Some(run), "/total_median_ms"),
            number_at(Some(run), "/peak_rss_mib")
        ),
    }))
}

struct RunDir {
    generated_at_unix_s: u64,
    slug: String,
}

fn latest_run(root: &Path) -> RunnerResult<Option<RunDir>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(None),
    };
    let mut runs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", root.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to stat {}: {error}", entry.path().display()))?;
        if !file_type.is_dir() {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());
        runs.push(RunDir {
            generated_at_unix_s: modified,
            slug: entry.file_name().to_string_lossy().into_owned(),
        });
    }
    runs.sort_by(|left, right| right.generated_at_unix_s.cmp(&left.generated_at_unix_s));
    Ok(runs.into_iter().next())
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = iter
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))?;
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn int_at(value: Option<&Value>, pointer: &str) -> i64 {
    value
        .and_then(|item| item.pointer(pointer))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn generated_at(run: &Value, payload: &Value) -> i64 {
    run["generated_at_unix_s"]
        .as_i64()
        .unwrap_or_else(|| int_at(Some(payload), "/generated_at_unix_s"))
}

fn number_at(value: Option<&Value>, pointer: &str) -> f64 {
    value
        .and_then(|item| item.pointer(pointer))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
