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
const VERSION_LINE: &str = "tamamono 1.x";
const SURFACES: &[&str] = &["workbench", "hub"];

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
        "Validated {} language packs for {VERSION_LINE} {}.",
        validator.validated_pack_count, validator.target_app_version
    );
    Ok(0)
}

struct Validator<'a> {
    root: &'a Path,
    errors: Vec<String>,
    target_app_version: String,
    validated_pack_count: usize,
}

impl<'a> Validator<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            errors: Vec::new(),
            target_app_version: String::new(),
            validated_pack_count: 0,
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
        for surface in SURFACES {
            let packs = packs_by_surface.get(*surface).cloned().unwrap_or_default();
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
            for discovered in self.discover_pack_files(surface) {
                if !referenced_paths.contains(&discovered) {
                    self.fail(format!(
                        "language-packs/{discovered}: discovered pack is not listed in catalog.json"
                    ));
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::looks_like_iso_datetime;

    #[test]
    fn accepts_release_timestamp_shape() {
        assert!(looks_like_iso_datetime("2026-07-13T00:00:00.000Z"));
        assert!(!looks_like_iso_datetime("2026-07-13"));
    }
}
