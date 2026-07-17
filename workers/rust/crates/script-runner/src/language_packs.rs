use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const PACK_SCHEMA: &str = "kyuubiki.language-pack/v1";
const CATALOG_SCHEMA: &str = "kyuubiki.language-pack-catalog/v1";
const LOCALE_TARGET_SCHEMA: &str = "kyuubiki.localization-mainstream-locales/v1";
const VERSION_LINE: &str = "moxi 2.x";
const SURFACES: &[&str] = &["workbench", "hub"];
const HUB_REQUIRED_OVERRIDE_PATHS: &[&str] = &[
    "nav.projects",
    "nav.runtimes",
    "nav.deploy",
    "nav.observe",
    "nav.tools",
    "sections.projects.title",
    "sections.projects.copy",
    "sections.runtimes.title",
    "sections.runtimes.copy",
    "sections.deploy.title",
    "sections.deploy.copy",
    "shell.language",
    "shell.actionStatus",
    "shell.idle",
    "shell.openWorkbench",
    "shell.startLocal",
    "shell.validateEnv",
];
const WORKBENCH_REQUIRED_OVERRIDE_PATHS: &[&str] = &[
    "title",
    "subtitle",
    "rail.study",
    "rail.model",
    "rail.workflow",
    "rail.store",
    "rail.library",
    "rail.system",
    "sections.study",
    "sections.model",
    "sections.workflow",
    "sections.store",
    "sections.library",
    "sections.system",
    "workflowBuilderPage",
    "workflowRunsPage",
    "workflowCatalogTitle",
    "workflowTemplateChainLibraryLabel",
    "languagePacksTitle",
    "languagePacksHint",
    "languagePacksEmptyLabel",
    "languagePackName",
    "languagePackVersion",
    "languagePackSourceImported",
    "languagePackSourceDownloaded",
    "languagePackDownloadTemplate",
    "languagePackExportInstalled",
    "languagePackImport",
    "languagePackRemove",
    "languagePackCatalogTitle",
    "languagePackCatalogHint",
    "languagePackCatalogAction",
];

pub(crate) fn run_validate_language_packs(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if !args.is_empty() {
        return Err("validate-language-packs does not accept arguments".to_string());
    }
    let mut validator = Validator::new(root);
    validator.run();
    if !validator.errors.is_empty() {
        eprintln!("Language pack validation failed:");
        for error in validator.errors {
            eprintln!("- {error}");
        }
        return Ok(1);
    }
    println!(
        "Validated {} language packs for {VERSION_LINE} {}; coverage {}.",
        validator.validated_pack_count, validator.target_app_version, validator.coverage_summary
    );
    Ok(0)
}

struct Validator<'a> {
    root: &'a Path,
    errors: Vec<String>,
    target_app_version: String,
    validated_pack_count: usize,
    coverage_summary: String,
}

impl<'a> Validator<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            errors: Vec::new(),
            target_app_version: String::new(),
            validated_pack_count: 0,
            coverage_summary: "not evaluated".to_string(),
        }
    }

    fn run(&mut self) {
        self.target_app_version = self.read_current_release_version();
        let locale_target = self.validate_locale_target();
        let mut referenced_paths = BTreeSet::new();
        let mut seen_ids = BTreeSet::new();
        let mut packs_by_surface = SURFACES
            .iter()
            .map(|surface| ((*surface).to_string(), Vec::<Value>::new()))
            .collect::<BTreeMap<_, _>>();

        let Some(catalog) = self.read_json("language-packs/catalog.json") else {
            return;
        };
        if !catalog.is_object() {
            self.fail("language-packs/catalog.json: catalog must be a JSON object");
            return;
        }
        self.validate_safe_text(&catalog, "language-packs/catalog.json");
        self.validate_catalog_header(&catalog);
        for (index, entry) in catalog
            .get("packs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let label = format!("language-packs/catalog.json:packs[{index}]");
            if !entry.is_object() {
                self.fail(format!("{label}: entry must be an object"));
                continue;
            }
            for field in ["id", "surface", "language", "name", "path", "status"] {
                self.validate_string(entry, field, &label);
            }
            let surface = string_field(entry, "surface");
            if !SURFACES.contains(&surface) {
                self.fail(format!("{label}: surface must be workbench or hub"));
            }
            let id = string_field(entry, "id");
            if !seen_ids.insert(id.to_string()) {
                self.fail(format!("{label}: duplicate id {id}"));
            }
            let path = string_field(entry, "path");
            referenced_paths.insert(path.to_string());
            let Some(pack) = self.validate_pack(path, surface) else {
                continue;
            };
            self.validated_pack_count += 1;
            packs_by_surface
                .entry(surface.to_string())
                .or_default()
                .push(pack.clone());
            for (field, pack_value) in [
                ("id", string_field(&pack, "id")),
                ("language", string_field(&pack, "language")),
                ("name", string_field(&pack, "name")),
            ] {
                if string_field(entry, field) != pack_value {
                    self.fail(format!(
                        "{label}: {field} does not match pack {field} {pack_value}"
                    ));
                }
            }
        }
        self.run_frontend_catalog_test();
        self.validate_surface_coverage(&locale_target, &referenced_paths, &packs_by_surface);
    }

    fn validate_catalog_header(&mut self, catalog: &Value) {
        if string_field(catalog, "schema_version") != CATALOG_SCHEMA {
            self.fail(format!(
                "language-packs/catalog.json: schema_version must be {CATALOG_SCHEMA}"
            ));
        }
        if string_field(catalog, "line") != VERSION_LINE {
            self.fail(format!(
                "language-packs/catalog.json: line must be {VERSION_LINE}"
            ));
        }
        if string_field(catalog, "shipping_version") != self.target_app_version {
            self.fail(format!(
                "language-packs/catalog.json: shipping_version must be {}",
                self.target_app_version
            ));
        }
        self.validate_timestamp(catalog.get("updatedAt"), "language-packs/catalog.json");
        if !catalog.get("packs").and_then(Value::as_array).is_some() {
            self.fail("language-packs/catalog.json: packs must be an array");
        }
    }

    fn validate_locale_target(&mut self) -> BTreeMap<String, Locale> {
        let Some(target) =
            self.read_json("config/localization/mainstream-language-pack-locales.json")
        else {
            return BTreeMap::new();
        };
        if !target.is_object() {
            self.fail("config/localization/mainstream-language-pack-locales.json: target must be a JSON object");
            return BTreeMap::new();
        }
        self.validate_safe_text(
            &target,
            "config/localization/mainstream-language-pack-locales.json",
        );
        if string_field(&target, "schema_version") != LOCALE_TARGET_SCHEMA {
            self.fail(format!(
                "config/localization/mainstream-language-pack-locales.json: schema_version must be {LOCALE_TARGET_SCHEMA}"
            ));
        }
        if string_field(&target, "line") != VERSION_LINE {
            self.fail(format!(
                "config/localization/mainstream-language-pack-locales.json: line must be {VERSION_LINE}"
            ));
        }
        self.validate_timestamp(
            target.get("updatedAt"),
            "config/localization/mainstream-language-pack-locales.json",
        );
        let locales = target
            .get("locales")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if locales.is_empty() {
            self.fail("config/localization/mainstream-language-pack-locales.json: locales must be an array");
        }
        if target.get("target_count").and_then(Value::as_u64) != Some(locales.len() as u64) {
            self.fail("config/localization/mainstream-language-pack-locales.json: target_count must match locales length");
        }
        let mut locale_target = BTreeMap::new();
        for (index, locale) in locales.iter().enumerate() {
            let label = format!(
                "config/localization/mainstream-language-pack-locales.json:locales[{index}]"
            );
            if !locale.is_object() {
                self.fail(format!("{label}: locale must be an object"));
                continue;
            }
            for field in ["language", "englishName", "nativeName"] {
                self.validate_string(locale, field, &label);
            }
            let language = string_field(locale, "language").to_string();
            if locale_target.contains_key(&language) {
                self.fail(format!("{label}: duplicate language {language}"));
            }
            locale_target.insert(
                language,
                Locale {
                    english_name: string_field(locale, "englishName").to_string(),
                },
            );
        }
        locale_target
    }

    fn validate_pack(&mut self, relative_path: &str, expected_surface: &str) -> Option<Value> {
        let full_path = format!("language-packs/{relative_path}");
        let pack = self.read_json(&full_path)?;
        if !pack.is_object() {
            self.fail(format!("{full_path}: pack must be a JSON object"));
            return None;
        }
        self.validate_safe_text(&pack, &full_path);
        for field in [
            "schema_version",
            "id",
            "language",
            "targetSurface",
            "name",
            "version",
            "versionLine",
            "targetAppVersion",
            "source",
            "updatedAt",
        ] {
            self.validate_string(&pack, field, &full_path);
        }
        if string_field(&pack, "schema_version") != PACK_SCHEMA {
            self.fail(format!("{full_path}: schema_version must be {PACK_SCHEMA}"));
        }
        if string_field(&pack, "targetSurface") != expected_surface {
            self.fail(format!(
                "{full_path}: targetSurface must be {expected_surface}"
            ));
        }
        if string_field(&pack, "versionLine") != VERSION_LINE {
            self.fail(format!("{full_path}: versionLine must be {VERSION_LINE}"));
        }
        if string_field(&pack, "targetAppVersion") != self.target_app_version
            || string_field(&pack, "version") != self.target_app_version
        {
            self.fail(format!(
                "{full_path}: version and targetAppVersion must be {}",
                self.target_app_version
            ));
        }
        if !matches!(string_field(&pack, "source"), "downloaded" | "imported") {
            self.fail(format!(
                "{full_path}: source must be imported or downloaded"
            ));
        }
        if !pack.get("overrides").is_some_and(Value::is_object) {
            self.fail(format!("{full_path}: overrides must be an object"));
        }
        self.validate_timestamp(pack.get("updatedAt"), &full_path);
        Some(pack)
    }

    fn validate_surface_coverage(
        &mut self,
        locale_target: &BTreeMap<String, Locale>,
        referenced_paths: &BTreeSet<String>,
        packs_by_surface: &BTreeMap<String, Vec<Value>>,
    ) {
        let mut summaries = Vec::new();
        for surface in SURFACES {
            let packs = packs_by_surface.get(*surface).cloned().unwrap_or_default();
            let required_paths = required_override_paths(surface);
            let mut covered_total = 0usize;
            let mut required_total = 0usize;
            let languages = packs
                .iter()
                .map(|pack| string_field(pack, "language"))
                .collect::<BTreeSet<_>>();
            if packs.len() != locale_target.len() {
                self.fail(format!(
                    "language-packs/catalog.json: {surface} must ship exactly {} mainstream language packs",
                    locale_target.len()
                ));
            }
            if languages.len() != packs.len() {
                self.fail(format!("language-packs/catalog.json: {surface} language packs must use unique language tags"));
            }
            for (language, locale) in locale_target {
                let Some(pack) = packs
                    .iter()
                    .find(|entry| string_field(entry, "language") == language)
                else {
                    self.fail(format!("language-packs/catalog.json: {surface} is missing mainstream language {language}"));
                    continue;
                };
                let expected_name = format!(
                    "{} {} Core",
                    locale.english_name,
                    if *surface == "hub" {
                        "Hub"
                    } else {
                        "Workbench"
                    }
                );
                if string_field(pack, "name") != expected_name {
                    self.fail(format!(
                        "language-packs/{surface}/{language}: name must be {expected_name}"
                    ));
                }
            }
            for pack in &packs {
                let missing = missing_override_paths(pack, required_paths);
                covered_total += required_paths.len().saturating_sub(missing.len());
                required_total += required_paths.len();
                if !missing.is_empty() {
                    self.fail(format!(
                        "language-packs/{surface}/{}: missing override coverage {}",
                        string_field(pack, "language"),
                        missing.join(", ")
                    ));
                }
            }
            summaries.push(format!("{surface} {covered_total}/{required_total}"));
            for discovered in self.discover_pack_files(surface) {
                if !referenced_paths.contains(&discovered) {
                    self.fail(format!(
                        "language-packs/{discovered}: discovered pack is not listed in catalog.json"
                    ));
                }
            }
        }
        self.coverage_summary = summaries.join(", ");
    }

    fn discover_pack_files(&mut self, surface: &str) -> Vec<String> {
        let surface_dir = self.root.join("language-packs").join(surface);
        if !surface_dir.exists() {
            self.fail(format!(
                "language-packs/{surface}: missing surface directory"
            ));
            return Vec::new();
        }
        let mut files = fs::read_dir(&surface_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|entry| entry.ends_with(".json"))
            .map(|entry| format!("{surface}/{entry}"))
            .collect::<Vec<_>>();
        files.sort();
        files
    }

    fn run_frontend_catalog_test(&mut self) {
        let output = Command::new("node")
            .args([
                "./scripts/test-unit.mjs",
                "workflow/workbench-language-pack-catalog",
            ])
            .current_dir(self.root.join("apps/frontend"))
            .output();
        match output {
            Ok(output) if output.status.success() => {}
            Ok(output) => self.fail(format!(
                "apps/frontend/test/workflow/workbench-language-pack-catalog.test.ts failed\n{}\n{}",
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            ).trim().to_string()),
            Err(error) => self.fail(format!("apps/frontend/test/workflow/workbench-language-pack-catalog.test.ts failed\n{error}")),
        }
    }

    fn read_current_release_version(&mut self) -> String {
        let Some(channels) = self.read_json("deploy/update-channels.json") else {
            return String::new();
        };
        let version = string_field(&channels, "shipping_version");
        if version.is_empty() {
            self.fail("deploy/update-channels.json must declare shipping_version for language pack validation");
        }
        version.to_string()
    }

    fn read_json(&mut self, relative_path: &str) -> Option<Value> {
        if !is_safe_repo_relative_path(relative_path) {
            self.fail(format!(
                "{relative_path}: path must be repository-relative and stay inside language pack validation roots"
            ));
            return None;
        }
        let path = self.root.join(relative_path);
        match fs::read_to_string(&path).and_then(|text| {
            serde_json::from_str(&text)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        }) {
            Ok(value) => Some(value),
            Err(error) => {
                self.fail(format!("{relative_path}: {error}"));
                None
            }
        }
    }

    fn validate_string(&mut self, value: &Value, field: &str, relative_path: &str) {
        if string_field(value, field).is_empty() {
            self.fail(format!(
                "{relative_path}: missing non-empty string field {field}"
            ));
        }
    }

    fn validate_timestamp(&mut self, value: Option<&Value>, relative_path: &str) {
        let Some(text) = value.and_then(Value::as_str) else {
            self.fail(format!(
                "{relative_path}: updatedAt must be an ISO date-time string"
            ));
            return;
        };
        if !looks_like_iso_datetime(text) {
            self.fail(format!(
                "{relative_path}: updatedAt must be an ISO date-time string"
            ));
        }
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    fn validate_safe_text(&mut self, value: &Value, relative_path: &str) {
        for issue in unsafe_language_pack_text_issues(value, relative_path) {
            self.fail(issue);
        }
    }
}

#[derive(Clone)]
struct Locale {
    english_name: String,
}

fn string_field<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
}

fn looks_like_iso_datetime(value: &str) -> bool {
    value.len() >= 20 && value.contains('T') && value.ends_with('Z')
}

fn is_safe_repo_relative_path(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.starts_with('/')
        && !looks_like_windows_absolute(value)
        && !value.split(['/', '\\']).any(|part| part == "..")
}

fn looks_like_windows_absolute(value: &str) -> bool {
    value.len() > 2
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\')
        && value.as_bytes()[0].is_ascii_alphabetic()
}

fn required_override_paths(surface: &str) -> &'static [&'static str] {
    match surface {
        "hub" => HUB_REQUIRED_OVERRIDE_PATHS,
        "workbench" => WORKBENCH_REQUIRED_OVERRIDE_PATHS,
        _ => &[],
    }
}

fn value_at_dotted_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn missing_override_paths(pack: &Value, required_paths: &[&str]) -> Vec<String> {
    let Some(overrides) = pack.get("overrides") else {
        return required_paths
            .iter()
            .map(|entry| (*entry).to_string())
            .collect();
    };
    required_paths
        .iter()
        .filter(|path| {
            value_at_dotted_path(overrides, path)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        })
        .map(|entry| (*entry).to_string())
        .collect()
}

fn unsafe_language_pack_text_issues(value: &Value, label: &str) -> Vec<String> {
    let mut issues = Vec::new();
    collect_unsafe_language_pack_text(value, label, &mut issues);
    issues
}

fn collect_unsafe_language_pack_text(value: &Value, label: &str, issues: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            for (needle, reason) in [
                ("<", "html angle bracket"),
                (">", "html angle bracket"),
                ("javascript:", "javascript url"),
                ("data:text/html", "html data url"),
                ("onerror=", "inline event handler"),
                ("onclick=", "inline event handler"),
                ("innerhtml", "html injection sink"),
                ("document.cookie", "browser secret access"),
                ("localstorage", "browser storage access"),
                ("eval(", "script evaluation"),
            ] {
                if lower.contains(needle) {
                    issues.push(format!("{label}: unsafe language-pack text: {reason}"));
                    break;
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_unsafe_language_pack_text(item, &format!("{label}[{index}]"), issues);
            }
        }
        Value::Object(object) => {
            for (key, item) in object {
                collect_unsafe_language_pack_text(item, &format!("{label}.{key}"), issues);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Validator, is_safe_repo_relative_path, looks_like_iso_datetime,
        unsafe_language_pack_text_issues,
    };
    use serde_json::json;

    #[test]
    fn accepts_release_timestamp_shape() {
        assert!(looks_like_iso_datetime("2026-07-13T00:00:00.000Z"));
        assert!(!looks_like_iso_datetime("2026-07-13"));
    }

    #[test]
    fn language_pack_fuzz_smoke_rejects_hostile_paths() {
        for path in [
            "",
            "/tmp/pack.json",
            "../pack.json",
            "workbench/../../secret.json",
            "C:\\Users\\secrets\\pack.json",
            "C:/Users/secrets/pack.json",
        ] {
            assert!(
                !is_safe_repo_relative_path(path),
                "hostile language pack path must fail: {path}"
            );
        }
        assert!(is_safe_repo_relative_path(
            "language-packs/workbench/en.json"
        ));

        let root = std::path::Path::new("/tmp/kyuubiki-language-pack-fuzz");
        let mut validator = Validator::new(root);
        validator.target_app_version = "2.0.0".to_string();
        assert!(
            validator
                .validate_pack("../secret.json", "workbench")
                .is_none()
        );
        assert!(
            validator
                .errors
                .iter()
                .any(|error| error.contains("path must be repository-relative"))
        );

        for payload in [
            json!({"overrides": {"title": "<script>alert(1)</script>"}}),
            json!({"overrides": {"title": "javascript:alert(1)"}}),
            json!({"overrides": {"title": "onclick=steal()"}}),
            json!({"overrides": {"title": "localStorage.token"}}),
        ] {
            assert!(
                !unsafe_language_pack_text_issues(&payload, "language-packs/fuzz.json").is_empty(),
                "hostile language pack text must fail: {payload}"
            );
        }
    }
}
