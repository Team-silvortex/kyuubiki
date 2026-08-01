use crate::native_time::utc_iso_timestamp;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

mod options;
mod source_parser;
mod translation_plan;

use options::{PlanOptions, ReportOptions};
use source_parser::{CopyValue, parse_copy_entries};
use translation_plan::build_plan;

type RunnerResult<T> = Result<T, String>;

const REPORT_SCHEMA: &str = "kyuubiki.language-pack-full-coverage/v1";
const BATCH_SCHEMA: &str = "kyuubiki.language-pack-translation-batch/v1";
const FRAGMENT_SCHEMA: &str = "kyuubiki.language-pack-fragment/v1";
const REPORT_JSON: &str = "tmp/language-pack-full-coverage.json";
const REPORT_MARKDOWN: &str = "tmp/language-pack-full-coverage.md";
const PLAN_JSON: &str = "tmp/language-pack-translation-plan.json";
const SOURCES: &[&str] = &[
    "apps/frontend/src/components/workbench/workbench-copy-en-core.ts",
    "apps/frontend/src/components/workbench/workbench-copy-en-extended.ts",
];

pub(crate) fn run_report_full_language_pack_coverage(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = ReportOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner report-full-language-pack-coverage \
             [--strict] [--strict-language <code>] \
             [--language <code> --batch <id> --template-out <path>] \
             [--apply-from <path>]"
        );
        return Ok(0);
    }

    let sources = load_sources(root)?;
    let mut packs = load_packs(root)?;
    let mut report = build_report(&sources, &packs);
    if let Some(input) = options.apply_from.as_deref() {
        apply_translation_batch(root, &sources, &report.batches, &mut packs, input)?;
        packs = load_packs(root)?;
        report = build_report(&sources, &packs);
    }
    write_report(root, &report)?;
    println!(
        "full Workbench language-pack coverage: {} keys; {}",
        report.required_keys.len(),
        if report.complete {
            "complete"
        } else {
            "incomplete"
        }
    );
    println!("reports: {REPORT_JSON}, {REPORT_MARKDOWN}");

    if let Some((language, batch, output)) = options.template_request()? {
        export_translation_batch(root, &sources, &report, &packs, language, batch, output)?;
    }
    if let Some(language) = options.strict_language.as_deref() {
        let complete = report
            .rows
            .iter()
            .any(|row| row.language == language && row.covered == row.required);
        if !complete {
            eprintln!("full language-pack coverage is incomplete for {language}");
            return Ok(1);
        }
    }
    if options.strict && !report.complete {
        eprintln!("full language-pack coverage is incomplete");
        return Ok(1);
    }
    Ok(0)
}

pub(crate) fn run_plan_workbench_language_translations(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = PlanOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner plan-workbench-language-translations \
             [--language <code>] [--next] [--json]"
        );
        return Ok(0);
    }
    let sources = load_sources(root)?;
    let packs = load_packs(root)?;
    let report = build_report(&sources, &packs);
    write_report(root, &report)?;
    let plan = build_plan(&report, options.language.as_deref())?;
    write_json(root, PLAN_JSON, &plan)?;

    if options.next {
        let next = plan.queue.first();
        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&next)
                    .map_err(|error| format!("failed to encode next translation: {error}"))?
            );
        } else if let Some(next) = next {
            println!(
                "{} {}: {}/{}; draft {}",
                next.language, next.batch, next.covered, next.required, next.draft
            );
        } else {
            println!("all Workbench language packs are complete");
        }
        return Ok(0);
    }
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&plan)
                .map_err(|error| format!("failed to encode translation plan: {error}"))?
        );
        return Ok(0);
    }
    println!(
        "language translation queue: {} incomplete batches across {} languages",
        plan.queue.len(),
        plan.incomplete_languages.len()
    );
    println!(
        "complete languages: {}",
        if plan.complete_languages.is_empty() {
            "none".to_string()
        } else {
            plan.complete_languages.join(", ")
        }
    );
    for entry in &plan.queue {
        println!(
            "{} {}: {}/{} ({} remaining)",
            entry.language, entry.batch, entry.covered, entry.required, entry.remaining
        );
    }
    Ok(0)
}

struct SourceCopy {
    relative_path: String,
    entries: BTreeMap<String, CopyValue>,
}

fn load_sources(root: &Path) -> RunnerResult<Vec<SourceCopy>> {
    SOURCES
        .iter()
        .map(|relative| {
            let text = fs::read_to_string(root.join(relative))
                .map_err(|error| format!("failed to read {relative}: {error}"))?;
            Ok(SourceCopy {
                relative_path: (*relative).to_string(),
                entries: parse_copy_entries(&text)
                    .map_err(|error| format!("failed to parse {relative}: {error}"))?,
            })
        })
        .collect()
}

struct Pack {
    language: String,
    path: PathBuf,
    payload: Value,
    overrides: Value,
    fragments: Vec<Fragment>,
}

struct Fragment {
    batch: String,
    path: PathBuf,
    payload: Value,
}

fn load_packs(root: &Path) -> RunnerResult<Vec<Pack>> {
    let directory = root.join("language-packs/workbench");
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("failed to scan {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| load_pack(&directory, path))
        .collect()
}

fn load_pack(directory: &Path, path: PathBuf) -> RunnerResult<Pack> {
    let payload = read_json(&path)?;
    let language = non_empty_string(&payload, "language", &path)?.to_string();
    let mut overrides = payload
        .get("overrides")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| format!("{}: overrides must be an object", path.display()))?;
    let mut fragments = Vec::new();
    for declaration in payload
        .get("fragments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let batch = non_empty_string(declaration, "batch", &path)?.to_string();
        let relative = non_empty_string(declaration, "path", &path)?;
        let fragment_path = safe_join(directory, relative)?;
        let fragment = read_json(&fragment_path)?;
        if fragment.get("schema_version").and_then(Value::as_str) != Some(FRAGMENT_SCHEMA)
            || fragment.get("language").and_then(Value::as_str) != Some(language.as_str())
            || fragment.get("targetSurface").and_then(Value::as_str) != Some("workbench")
            || fragment.get("batch").and_then(Value::as_str) != Some(batch.as_str())
        {
            return Err(format!(
                "invalid language-pack fragment {}",
                fragment_path.display()
            ));
        }
        let next = fragment
            .get("overrides")
            .filter(|value| value.is_object())
            .ok_or_else(|| format!("{}: overrides must be an object", fragment_path.display()))?;
        merge_values(&mut overrides, next);
        fragments.push(Fragment {
            batch,
            path: fragment_path,
            payload: fragment,
        });
    }
    Ok(Pack {
        language,
        path,
        payload,
        overrides,
        fragments,
    })
}

#[derive(Clone, Serialize)]
pub(super) struct Coverage {
    pub(super) language: String,
    pub(super) covered: usize,
}

#[derive(Clone, Serialize)]
pub(super) struct Batch {
    pub(super) id: String,
    source: String,
    pub(super) required: usize,
    keys: Vec<String>,
    pub(super) coverage: Vec<Coverage>,
}

#[derive(Serialize)]
pub(super) struct Row {
    pub(super) language: String,
    pub(super) covered: usize,
    meaningful: Vec<String>,
    #[serde(rename = "sourceMatchedCount")]
    source_matched_count: usize,
    #[serde(rename = "missingOrSourceMatch")]
    missing_or_source_match: usize,
    pub(super) required: usize,
    percent: f64,
    #[serde(rename = "percentRaw")]
    percent_raw: f64,
    missing: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct Report {
    schema_version: &'static str,
    sources: Vec<String>,
    pub(super) required_keys: Vec<String>,
    complete: bool,
    pub(super) batches: Vec<Batch>,
    pub(super) rows: Vec<Row>,
}

fn build_report(sources: &[SourceCopy], packs: &[Pack]) -> Report {
    let source_values = sources
        .iter()
        .flat_map(|source| source.entries.clone())
        .collect::<BTreeMap<_, _>>();
    let required_keys = source_values.keys().cloned().collect::<Vec<_>>();
    let rows = packs
        .iter()
        .map(|pack| build_row(pack, &required_keys, &source_values))
        .collect::<Vec<_>>();
    let batches = sources
        .iter()
        .flat_map(|source| {
            let keys = source.entries.keys().cloned().collect::<Vec<_>>();
            keys.chunks(100)
                .enumerate()
                .map(|(index, keys)| Batch {
                    id: format!(
                        "{}-{:02}",
                        source
                            .relative_path
                            .rsplit('/')
                            .next()
                            .unwrap_or_default()
                            .trim_end_matches(".ts")
                            .trim_start_matches("workbench-copy-en-"),
                        index + 1
                    ),
                    source: source.relative_path.clone(),
                    required: keys.len(),
                    keys: keys.to_vec(),
                    coverage: rows
                        .iter()
                        .map(|row| Coverage {
                            language: row.language.clone(),
                            covered: keys
                                .iter()
                                .filter(|key| row.meaningful.binary_search(key).is_ok())
                                .count(),
                        })
                        .collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Report {
        schema_version: REPORT_SCHEMA,
        sources: sources
            .iter()
            .map(|source| source.relative_path.clone())
            .collect(),
        complete: rows.iter().all(|row| row.covered == row.required),
        required_keys,
        batches,
        rows,
    }
}

fn build_row(
    pack: &Pack,
    required_keys: &[String],
    source_values: &BTreeMap<String, CopyValue>,
) -> Row {
    let meaningful = required_keys
        .iter()
        .filter(|key| is_meaningful(value_at_path(&pack.overrides, key), &source_values[*key]))
        .cloned()
        .collect::<Vec<_>>();
    let missing = required_keys
        .iter()
        .filter(|key| !has_translation(value_at_path(&pack.overrides, key), &source_values[*key]))
        .cloned()
        .collect::<Vec<_>>();
    let source_matched_count = required_keys
        .iter()
        .filter(|key| {
            is_source_translation(value_at_path(&pack.overrides, key), &source_values[*key])
        })
        .count();
    let required = required_keys.len();
    Row {
        language: pack.language.clone(),
        covered: meaningful.len(),
        source_matched_count,
        missing_or_source_match: required - meaningful.len(),
        required,
        percent: percentage(meaningful.len(), required),
        percent_raw: percentage(required - missing.len(), required),
        meaningful,
        missing,
    }
}

fn has_translation(value: Option<&Value>, source: &CopyValue) -> bool {
    match (value, source) {
        (Some(Value::String(value)), CopyValue::String(_)) => !value.trim().is_empty(),
        (Some(Value::Array(values)), CopyValue::Strings(source)) => {
            values.len() == source.len()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
        }
        _ => false,
    }
}

fn is_source_translation(value: Option<&Value>, source: &CopyValue) -> bool {
    if matches!(source, CopyValue::String(text) if text.chars().any(|value| value.is_ascii_digit()))
    {
        return false;
    }
    match (value, source) {
        (Some(Value::String(value)), CopyValue::String(source)) => value == source,
        (Some(Value::Array(values)), CopyValue::Strings(source)) => {
            values.iter().filter_map(Value::as_str).eq(source.iter())
                && values.len() == source.len()
        }
        _ => false,
    }
}

fn is_meaningful(value: Option<&Value>, source: &CopyValue) -> bool {
    has_translation(value, source) && !is_source_translation(value, source)
}

fn percentage(covered: usize, required: usize) -> f64 {
    if required == 0 {
        return 100.0;
    }
    ((covered as f64 / required as f64) * 1_000.0).round() / 10.0
}

fn value_at_path<'a>(value: &'a Value, dotted: &str) -> Option<&'a Value> {
    dotted
        .split('.')
        .try_fold(value, |current, part| current.get(part))
}

fn write_report(root: &Path, report: &Report) -> RunnerResult<()> {
    write_json(root, REPORT_JSON, report)?;
    let mut markdown = vec![
        "# Full Language-Pack Coverage".to_string(),
        String::new(),
        format!(
            "Status: **{}**. Each shipped Workbench language pack must provide a real override for all {} visible-copy keys before it can claim full coverage.",
            if report.complete {
                "complete"
            } else {
                "incomplete"
            },
            report.required_keys.len()
        ),
        String::new(),
        "## Language Coverage".into(),
        String::new(),
        "| Language | Covered | Source-matches | Required | Coverage |".into(),
        "| --- | ---: | ---: | ---: | ---: |".into(),
    ];
    markdown.extend(report.rows.iter().map(|row| {
        format!(
            "| {} | {} | {} | {} | {}% |",
            row.language, row.covered, row.source_matched_count, row.required, row.percent
        )
    }));
    markdown.extend([
        String::new(),
        "## Delivery Batches".into(),
        String::new(),
        "| Batch | Source | Keys | Lowest coverage |".into(),
        "| --- | --- | ---: | ---: |".into(),
    ]);
    markdown.extend(report.batches.iter().map(|batch| {
        let minimum = batch
            .coverage
            .iter()
            .map(|entry| entry.covered)
            .min()
            .unwrap_or_default();
        format!(
            "| {} | {} | {} | {}/{} |",
            batch.id,
            batch.source.rsplit('/').next().unwrap_or_default(),
            batch.required,
            minimum,
            batch.required
        )
    }));
    markdown.push(String::new());
    write_text(root, REPORT_MARKDOWN, &markdown.join("\n"))
}

fn export_translation_batch(
    root: &Path,
    sources: &[SourceCopy],
    report: &Report,
    packs: &[Pack],
    language: &str,
    batch_id: &str,
    output: &str,
) -> RunnerResult<()> {
    let batch = report
        .batches
        .iter()
        .find(|batch| batch.id == batch_id)
        .ok_or_else(|| format!("unknown translation batch: {batch_id}"))?;
    let pack = packs
        .iter()
        .find(|pack| pack.language == language)
        .ok_or_else(|| format!("unknown Workbench language: {language}"))?;
    let source_values = sources
        .iter()
        .flat_map(|source| source.entries.clone())
        .collect::<BTreeMap<_, _>>();
    let strings = batch
        .keys
        .iter()
        .map(|key| {
            let source = copy_value_json(&source_values[key]);
            json!({
                "key": key,
                "source": source,
                "translation": value_at_path(&pack.overrides, key).cloned().unwrap_or_else(|| {
                    if source.is_array() { Value::Array(Vec::new()) } else { Value::String(String::new()) }
                })
            })
        })
        .collect::<Vec<_>>();
    let template = json!({
        "schema_version": BATCH_SCHEMA,
        "language": language,
        "batch": batch.id,
        "source": batch.source,
        "strings": strings,
    });
    write_json(root, output, &template)?;
    println!("translation batch template: {output}");
    Ok(())
}

fn apply_translation_batch(
    root: &Path,
    sources: &[SourceCopy],
    batches: &[Batch],
    packs: &mut [Pack],
    input: &str,
) -> RunnerResult<()> {
    let input_path = safe_join(root, input)?;
    let payload = read_json(&input_path)?;
    if payload.get("schema_version").and_then(Value::as_str) != Some(BATCH_SCHEMA) {
        return Err("translation batch schema is not supported".into());
    }
    let language = non_empty_string(&payload, "language", &input_path)?;
    let batch_id = non_empty_string(&payload, "batch", &input_path)?;
    let source_path = non_empty_string(&payload, "source", &input_path)?;
    let batch = batches
        .iter()
        .find(|batch| batch.id == batch_id && batch.source == source_path)
        .ok_or_else(|| "translation batch does not match a shipped Workbench pack".to_string())?;
    let pack = packs
        .iter_mut()
        .find(|pack| pack.language == language)
        .ok_or_else(|| "translation batch does not match a shipped Workbench pack".to_string())?;
    let source_values = sources
        .iter()
        .flat_map(|source| source.entries.clone())
        .collect::<BTreeMap<_, _>>();
    let strings = payload
        .get("strings")
        .and_then(Value::as_array)
        .ok_or_else(|| "translation batch strings must be an array".to_string())?;
    let by_key = strings
        .iter()
        .filter_map(|entry| {
            entry
                .get("key")
                .and_then(Value::as_str)
                .map(|key| (key, entry))
        })
        .collect::<BTreeMap<_, _>>();
    if by_key.len() != batch.keys.len()
        || batch
            .keys
            .iter()
            .any(|key| !by_key.contains_key(key.as_str()))
    {
        return Err("translation batch keys must exactly match its declared batch".into());
    }
    for key in &batch.keys {
        let entry = by_key[key.as_str()];
        if entry.get("source") != Some(&copy_value_json(&source_values[key])) {
            return Err(format!("translation batch source drift for {key}"));
        }
        if !is_meaningful(entry.get("translation"), &source_values[key]) {
            return Err(format!(
                "translation is missing or has the wrong shape for {key}"
            ));
        }
    }
    let (target_path, target) = if let Some(fragment) = pack
        .fragments
        .iter_mut()
        .find(|fragment| fragment.batch == batch.id)
    {
        (&fragment.path, &mut fragment.payload)
    } else {
        (&pack.path, &mut pack.payload)
    };
    let overrides = target
        .get_mut("overrides")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{}: overrides must be an object", target_path.display()))?;
    for key in &batch.keys {
        set_value_at_path(overrides, key, by_key[key.as_str()]["translation"].clone())?;
    }
    target["updatedAt"] = Value::String(utc_iso_timestamp());
    write_json_path(target_path, target)?;
    let relative = target_path.strip_prefix(root).unwrap_or(target_path);
    println!(
        "applied {} translations to {}",
        batch.id,
        relative.display()
    );
    Ok(())
}

fn set_value_at_path(
    object: &mut Map<String, Value>,
    dotted: &str,
    value: Value,
) -> RunnerResult<()> {
    let mut parts = dotted.split('.').peekable();
    let mut current = object;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            current.insert(part.to_string(), value);
            return Ok(());
        }
        current = current
            .entry(part.to_string())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| format!("translation path {dotted} crosses a non-object value"))?;
    }
    Err("translation path must not be empty".into())
}

fn copy_value_json(value: &CopyValue) -> Value {
    match value {
        CopyValue::String(value) => Value::String(value.clone()),
        CopyValue::Strings(values) => {
            Value::Array(values.iter().cloned().map(Value::String).collect())
        }
    }
}

fn merge_values(base: &mut Value, next: &Value) {
    match (base, next) {
        (Value::Object(base), Value::Object(next)) => {
            for (key, value) in next {
                merge_values(base.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (base, next) => *base = next.clone(),
    }
}

fn non_empty_string<'a>(value: &'a Value, field: &str, path: &Path) -> RunnerResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{}: missing non-empty string {field}", path.display()))
}

fn safe_join(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if relative.trim().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("path must stay repository-relative: {relative}"));
    }
    Ok(root.join(path))
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(root: &Path, relative: &str, value: &impl Serialize) -> RunnerResult<()> {
    let path = safe_join(root, relative)?;
    write_json_path(&path, value)
}

fn write_json_path(path: &Path, value: &impl Serialize) -> RunnerResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    write_path(path, &format!("{text}\n"))
}

fn write_text(root: &Path, relative: &str, text: &str) -> RunnerResult<()> {
    let path = safe_join(root, relative)?;
    write_path(&path, text)
}

fn write_path(path: &Path, text: &str) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{CopyValue, has_translation, is_meaningful, percentage};
    use serde_json::json;

    #[test]
    fn classifies_translation_shape_and_source_matches() {
        let source = CopyValue::String("Run".into());
        assert!(has_translation(Some(&json!("Ausführen")), &source));
        assert!(is_meaningful(Some(&json!("Ausführen")), &source));
        assert!(!is_meaningful(Some(&json!("Run")), &source));
        let invariant = CopyValue::String("1D beam".into());
        assert!(is_meaningful(Some(&json!("1D beam")), &invariant));
        let rows = CopyValue::Strings(vec!["One".into(), "Two".into()]);
        assert!(!has_translation(Some(&json!(["Uno"])), &rows));
        assert_eq!(percentage(1, 3), 33.3);
    }
}
