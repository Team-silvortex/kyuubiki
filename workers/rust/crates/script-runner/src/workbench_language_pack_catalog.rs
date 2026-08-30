use crate::language_packs::{looks_like_iso_datetime, unsafe_language_pack_text_issues};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path};

type RunnerResult<T> = Result<T, String>;

const PACKS_DIRECTORY: &str = "language-packs/workbench";
const OUTPUT_PATH: &str =
    "apps/frontend/src/components/workbench/workbench-language-pack-catalog-data.ts";
const OUTPUT_DIRECTORY: &str =
    "apps/frontend/src/components/workbench/workbench-language-pack-data";
const FRAGMENT_SCHEMA: &str = "kyuubiki.language-pack-fragment/v1";

pub(crate) fn run_build_workbench_language_pack_catalog(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.help {
        println!("usage: kyuubiki-script-runner build-workbench-language-pack-catalog [--check]");
        return Ok(0);
    }
    let expected = render_catalog(&read_workbench_packs(root)?)?;
    let output_path = root.join(OUTPUT_PATH);
    if options.check {
        let actual = fs::read_to_string(&output_path)
            .map_err(|error| format!("failed to read {OUTPUT_PATH}: {error}"))?;
        if actual != expected.index || !generated_modules_match(root, &expected.modules)? {
            return Err(
                "Workbench language-pack catalog is stale; run make build-workbench-language-pack-catalog"
                    .to_string(),
            );
        }
        println!("Workbench language-pack catalog is synchronized.");
        return Ok(0);
    }

    fs::write(&output_path, expected.index)
        .map_err(|error| format!("failed to write {OUTPUT_PATH}: {error}"))?;
    write_generated_modules(root, &expected.modules)?;
    println!("Built Workbench language-pack catalog from language-packs/workbench.");
    Ok(0)
}

struct Options {
    check: bool,
    help: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            check: false,
            help: false,
        };
        for arg in args {
            match arg.to_string_lossy().as_ref() {
                "--check" => options.check = true,
                "--help" | "-h" => options.help = true,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(options)
    }
}

fn read_workbench_packs(root: &Path) -> RunnerResult<Vec<(String, Value)>> {
    let packs_root = root.join(PACKS_DIRECTORY);
    let mut paths = fs::read_dir(&packs_root)
        .map_err(|error| format!("failed to read {PACKS_DIRECTORY}: {error}"))?
        .map(|entry| {
            entry
                .map_err(|error| format!("failed to inspect {PACKS_DIRECTORY}: {error}"))
                .map(|entry| entry.path())
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(Path::to_str)
                .unwrap_or(PACKS_DIRECTORY);
            let pack = read_json(&path, relative)?;
            let language = non_empty_string(&pack, "language", relative)?.to_string();
            let overrides = pack
                .get("overrides")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| format!("invalid language-pack root for {language}"))?;
            let fragments = match pack.get("fragments") {
                None => Vec::new(),
                Some(Value::Array(items)) => items.clone(),
                Some(_) => return Err(format!("invalid language-pack root for {language}")),
            };
            let mut merged = Value::Object(overrides);
            let mut fragment_ids = BTreeSet::new();
            for fragment in fragments {
                let next = read_fragment(&packs_root, &language, &fragment, &mut fragment_ids)?;
                merge_overrides(&mut merged, next);
            }
            Ok((language, merged))
        })
        .collect()
}

fn read_fragment(
    packs_root: &Path,
    language: &str,
    fragment: &Value,
    fragment_ids: &mut BTreeSet<String>,
) -> RunnerResult<Value> {
    let batch = non_empty_string(fragment, "batch", language)?.to_string();
    let relative_path = non_empty_string(fragment, "path", language)?;
    if !safe_fragment_path(relative_path) {
        return Err(format!(
            "invalid language-pack fragment declaration for {language}"
        ));
    }
    if !fragment_ids.insert(batch.clone()) {
        return Err(format!(
            "duplicate {language} language-pack fragment {batch}"
        ));
    }
    let absolute_path = packs_root.join(relative_path);
    let payload = read_json(&absolute_path, relative_path)?;
    if payload.get("schema_version").and_then(Value::as_str) != Some(FRAGMENT_SCHEMA)
        || !payload.get("overrides").is_some_and(Value::is_object)
    {
        return Err(format!("invalid language-pack fragment {relative_path}"));
    }
    if payload.get("language").and_then(Value::as_str) != Some(language)
        || payload.get("targetSurface").and_then(Value::as_str) != Some("workbench")
        || payload.get("batch").and_then(Value::as_str) != Some(batch.as_str())
    {
        return Err(format!("fragment identity mismatch: {relative_path}"));
    }
    let timestamp = payload
        .get("updatedAt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !looks_like_iso_datetime(timestamp)
        || !unsafe_language_pack_text_issues(&payload, relative_path).is_empty()
    {
        return Err(format!(
            "unsafe or stale language-pack fragment {relative_path}"
        ));
    }
    Ok(payload.get("overrides").cloned().unwrap_or_default())
}

fn merge_overrides(base: &mut Value, next: Value) {
    let (Some(base), Value::Object(next)) = (base.as_object_mut(), next) else {
        return;
    };
    for (key, next_value) in next {
        if let Some(base_value) = base.get_mut(&key)
            && base_value.is_object()
            && next_value.is_object()
        {
            merge_overrides(base_value, next_value);
        } else {
            base.insert(key, next_value);
        }
    }
}

struct RenderedCatalog {
    index: String,
    modules: Vec<(String, String)>,
}

fn render_catalog(packs: &[(String, Value)]) -> RunnerResult<RenderedCatalog> {
    let mut lines = vec![
        "// Generated from language-packs/workbench/*.json. Do not edit by hand.".to_string(),
        "type WorkbenchLanguagePackOverrides = Record<string, unknown>;".to_string(),
        "type WorkbenchLanguagePackLoader = () => Promise<WorkbenchLanguagePackOverrides>;".to_string(),
        String::new(),
        "const WORKBENCH_TRANSLATED_LANGUAGE_PACK_LOADERS: Record<string, WorkbenchLanguagePackLoader> = {".to_string(),
    ];
    let mut modules = Vec::with_capacity(packs.len());
    for (language, overrides) in packs {
        let language_literal = serde_json::to_string(language)
            .map_err(|error| format!("failed to encode language id: {error}"))?;
        let filename = language_module_filename(language);
        lines.push(format!(
            "  {language_literal}: () => import(\"./workbench-language-pack-data/{filename}\").then((module) => module.default),"
        ));

        let payload = serde_json::to_string(overrides)
            .map_err(|error| format!("failed to encode language overrides: {error}"))?;
        modules.push((
            format!("{filename}.ts"),
            format!(
                "// Generated from language-packs/workbench/{filename}.json. Do not edit by hand.\nconst overrides: Record<string, unknown> = {payload};\n\nexport default overrides;\n"
            ),
        ));
    }
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push(
        "const workbenchLanguagePackCache = new Map<string, Promise<WorkbenchLanguagePackOverrides>>();"
            .to_string(),
    );
    lines.push(String::new());
    lines.push(
        "export function loadWorkbenchTranslatedLanguagePackOverrides(language: string): Promise<WorkbenchLanguagePackOverrides | null> {"
            .to_string(),
    );
    lines
        .push("  const loader = WORKBENCH_TRANSLATED_LANGUAGE_PACK_LOADERS[language];".to_string());
    lines.push("  if (!loader) return Promise.resolve(null);".to_string());
    lines.push("  const cached = workbenchLanguagePackCache.get(language);".to_string());
    lines.push("  if (cached) return cached;".to_string());
    lines.push("  const pending = loader().catch((error) => {".to_string());
    lines.push("    workbenchLanguagePackCache.delete(language);".to_string());
    lines.push("    throw error;".to_string());
    lines.push("  });".to_string());
    lines.push("  workbenchLanguagePackCache.set(language, pending);".to_string());
    lines.push("  return pending;".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    Ok(RenderedCatalog {
        index: lines.join("\n"),
        modules,
    })
}

fn language_module_filename(language: &str) -> String {
    let mut filename = String::new();
    for character in language.chars() {
        if character.is_ascii_alphanumeric() {
            filename.push(character.to_ascii_lowercase());
        } else if !filename.ends_with('-') {
            filename.push('-');
        }
    }
    filename.trim_matches('-').to_string()
}

fn generated_modules_match(root: &Path, expected: &[(String, String)]) -> RunnerResult<bool> {
    let output_directory = root.join(OUTPUT_DIRECTORY);
    let expected_names = expected
        .iter()
        .map(|(filename, _)| filename.as_str())
        .collect::<BTreeSet<_>>();
    let actual_names = fs::read_dir(&output_directory)
        .map_err(|error| format!("failed to read {OUTPUT_DIRECTORY}: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|filename| filename.ends_with(".ts"))
        .collect::<BTreeSet<_>>();
    if actual_names.len() != expected_names.len()
        || actual_names
            .iter()
            .any(|filename| !expected_names.contains(filename.as_str()))
    {
        return Ok(false);
    }
    for (filename, contents) in expected {
        let actual = fs::read_to_string(output_directory.join(filename))
            .map_err(|error| format!("failed to read {OUTPUT_DIRECTORY}/{filename}: {error}"))?;
        if actual != *contents {
            return Ok(false);
        }
    }
    Ok(true)
}

fn write_generated_modules(root: &Path, modules: &[(String, String)]) -> RunnerResult<()> {
    let output_directory = root.join(OUTPUT_DIRECTORY);
    fs::create_dir_all(&output_directory)
        .map_err(|error| format!("failed to create {OUTPUT_DIRECTORY}: {error}"))?;
    let expected_names = modules
        .iter()
        .map(|(filename, _)| filename.as_str())
        .collect::<BTreeSet<_>>();
    for entry in fs::read_dir(&output_directory)
        .map_err(|error| format!("failed to read {OUTPUT_DIRECTORY}: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to inspect {OUTPUT_DIRECTORY}: {error}"))?;
        let Some(filename) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if filename.ends_with(".ts") && !expected_names.contains(filename.as_str()) {
            fs::remove_file(entry.path()).map_err(|error| {
                format!("failed to remove stale {OUTPUT_DIRECTORY}/{filename}: {error}")
            })?;
        }
    }
    for (filename, contents) in modules {
        fs::write(output_directory.join(filename), contents)
            .map_err(|error| format!("failed to write {OUTPUT_DIRECTORY}/{filename}: {error}"))?;
    }
    Ok(())
}

fn read_json(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse {label}: {error}"))
}

fn non_empty_string<'a>(value: &'a Value, field: &str, label: &str) -> RunnerResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("invalid language-pack fragment declaration for {label}"))
}

fn safe_fragment_path(relative_path: &str) -> bool {
    let path = Path::new(relative_path);
    !relative_path.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::{
        OUTPUT_DIRECTORY, OUTPUT_PATH, generated_modules_match, read_workbench_packs,
        render_catalog,
    };
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn native_catalog_matches_retained_generated_output() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let packs = read_workbench_packs(&root).expect("language packs should load");
        let expected = render_catalog(&packs).expect("catalog should render");
        let actual =
            fs::read_to_string(root.join(OUTPUT_PATH)).expect("generated catalog should load");
        assert_eq!(actual, expected.index);
        assert!(
            generated_modules_match(&root, &expected.modules)
                .expect("generated language-pack modules should load"),
            "{OUTPUT_DIRECTORY} must match the source packs"
        );
    }

    #[test]
    fn build_path_stays_native() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let legacy_name = ["build-workbench-language-pack-catalog", ".mjs"].concat();
        assert!(
            !root.join("scripts").join(&legacy_name).exists(),
            "legacy Node catalog generator must stay deleted"
        );
        for relative in ["make/build.mk", "scripts/validate-language-packs.mjs"] {
            let source = fs::read_to_string(root.join(relative)).expect("build caller should load");
            assert!(
                !source.contains(&legacy_name),
                "{relative} must use the native catalog generator"
            );
        }
    }
}
