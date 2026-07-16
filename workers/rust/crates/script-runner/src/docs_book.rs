use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const UPDATE_CHANNELS_PATH: &str = "deploy/update-channels.json";
const BOOK_MANIFEST_PATH: &str = "docs/book-manifest.json";

const HTML_FILES: &[&str] = &[
    "docs/book.html",
    "docs/book-ch01-what-is-kyuubiki.html",
    "docs/book-ch02-moxi-line.html",
    "docs/book-ch03-architecture-boundaries.html",
    "docs/book-ch04-runtime-modes.html",
    "docs/book-ch05-workflow-and-operators.html",
    "docs/book-ch06-sdk-surfaces.html",
    "docs/book-ch07-trust-and-safety.html",
    "docs/book-ch08-reading-paths.html",
    "docs/update-catalog.html",
    "docs/installation-integrity-contract.html",
    "apps/hub-gui/ui/docs/index.html",
    "apps/hub-gui/ui/docs/current-line.html",
    "apps/hub-gui/ui/docs/operations.html",
    "apps/hub-gui/ui/docs/installation-integrity.html",
    "apps/hub-gui/ui/docs/troubleshooting.html",
    "apps/hub-gui/ui/docs/update-catalog.html",
];

const VERSION_FILES: &[&str] = &[
    "docs/book.html",
    "docs/book-manifest.json",
    "apps/hub-gui/ui/docs/index.html",
    "apps/hub-gui/ui/docs/current-line.html",
    "apps/hub-gui/ui/docs/installation-integrity.html",
    "apps/hub-gui/ui/docs/update-catalog.html",
];

const FORBIDDEN_SNIPPETS: &[&str] = &["Kyuubiki Hub Docs", "docs home", "Back to docs home"];

const ROLE_PATHS: &[&str] = &[
    "operator",
    "frontend_engineer",
    "runtime_engineer",
    "sdk_engineer",
    "llm_integrator",
];

pub(crate) fn run_check_doc_book(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-doc-book");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-doc-book does not accept positional arguments".to_string());
    }

    let shipping_version = read_json(root, UPDATE_CHANNELS_PATH)?
        .get("shipping_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "deploy/update-channels.json must declare shipping_version for docs-book validation"
                .to_string()
        })?
        .to_string();
    let manifest = read_json(root, BOOK_MANIFEST_PATH)?;
    let issues = validate_doc_book(root, &shipping_version, &manifest)?;

    if !issues.is_empty() {
        eprintln!("Kyuubiki docs-book check failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    println!("docs-book check passed for version {shipping_version}");
    println!(
        "checked {} HTML files and docs/book-manifest.json",
        HTML_FILES.len()
    );
    Ok(0)
}

pub(crate) fn run_sync_doc_book_version(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = SyncOptions::parse(args)?;
    if options.help {
        println!(
            "Usage:\n  kyuubiki-script-runner sync-doc-book-version\n  kyuubiki-script-runner sync-doc-book-version --version 2.0.0 --line \"moxi 2.0.0\""
        );
        return Ok(0);
    }

    let shipping_version = match options.version {
        Some(version) => version,
        None => read_json(root, UPDATE_CHANNELS_PATH)?
            .get("shipping_version")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "deploy/update-channels.json must declare shipping_version for docs-book sync"
                    .to_string()
            })?
            .to_string(),
    };
    let version_line = options
        .line
        .unwrap_or_else(|| format!("moxi {shipping_version}"));
    let minor_line = semver_minor(&shipping_version)?;

    let replacements = sync_replacements(&shipping_version, &version_line, &minor_line);
    let mut updated_files = Vec::new();
    for (relative_path, rules) in replacements {
        let absolute_path = root.join(relative_path);
        let original = fs::read_to_string(&absolute_path)
            .map_err(|error| format!("failed to read {}: {error}", absolute_path.display()))?;
        let next = rules
            .iter()
            .fold(original.clone(), |text, rule| rule.apply(&text));
        if next != original {
            fs::write(&absolute_path, next)
                .map_err(|error| format!("failed to write {}: {error}", absolute_path.display()))?;
            updated_files.push(relative_path);
        }
    }

    println!("synced docs-book version to {version_line}");
    if updated_files.is_empty() {
        println!("no files changed");
    } else {
        for file in updated_files {
            println!("updated {file}");
        }
    }
    Ok(0)
}

fn validate_doc_book(
    root: &Path,
    shipping_version: &str,
    manifest: &Value,
) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();

    for relative_path in HTML_FILES {
        let absolute_path = root.join(relative_path);
        if !absolute_path.is_file() {
            issues.push(format!("missing file: {relative_path}"));
            continue;
        }

        let text = fs::read_to_string(&absolute_path)
            .map_err(|error| format!("failed to read {}: {error}", absolute_path.display()))?;
        for forbidden in FORBIDDEN_SNIPPETS {
            if text.contains(forbidden) {
                issues.push(format!(
                    "{relative_path}: found forbidden legacy text \"{forbidden}\""
                ));
            }
        }

        for snippet in required_snippets(relative_path) {
            if !text.contains(snippet) {
                issues.push(format!(
                    "{relative_path}: missing required text \"{snippet}\""
                ));
            }
        }

        for href in extract_local_hrefs(&text) {
            let target = resolve_local_href(&absolute_path, &href);
            if !target.exists() {
                issues.push(format!("{relative_path}: broken href {href}"));
            }
        }
    }

    for relative_path in VERSION_FILES {
        let text = read_text(root, relative_path)?;
        if !text.contains(shipping_version) {
            issues.push(format!(
                "{relative_path}: missing shipping version {shipping_version}"
            ));
        }
    }

    let chapters = manifest
        .get("chapters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if chapters.len() != 8 {
        issues.push(format!(
            "docs/book-manifest.json: expected 8 chapters, found {}",
            chapters.len()
        ));
    }

    for chapter in chapters {
        let id = chapter
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let Some(chapter_page) = chapter.get("chapter_page").and_then(Value::as_str) else {
            issues.push(format!(
                "docs/book-manifest.json: chapter {id} missing chapter_page"
            ));
            continue;
        };
        if !root.join(chapter_page).is_file() {
            issues.push(format!(
                "docs/book-manifest.json: missing chapter page {chapter_page}"
            ));
        }
    }

    let reading_paths = manifest.get("reading_paths").and_then(Value::as_object);
    for role in ROLE_PATHS {
        let entries = reading_paths
            .and_then(|paths| paths.get(*role))
            .and_then(Value::as_array);
        if entries.is_none_or(Vec::is_empty) {
            issues.push(format!(
                "docs/book-manifest.json: reading path \"{role}\" is missing or empty"
            ));
        }
    }

    Ok(issues)
}

fn required_snippets(relative_path: &str) -> &'static [&'static str] {
    match relative_path {
        "docs/book.html" => &["Chapter 1: What Kyuubiki is", "Open chapter page"],
        "docs/book-ch08-reading-paths.html" => &["book-manifest.json", "docs/README.md"],
        "apps/hub-gui/ui/docs/index.html" => {
            &["Open central book", "Chapter entry", "Quick entry by role"]
        }
        "apps/hub-gui/ui/docs/current-line.html" => &["Mirror · Chapter 2", "Open central chapter"],
        "apps/hub-gui/ui/docs/operations.html" => &["Mirror · Chapter 4", "Open central chapter"],
        "apps/hub-gui/ui/docs/installation-integrity.html" => &[
            "Mirror · Chapter 7",
            "Open central chapter",
            "Open source page",
        ],
        "apps/hub-gui/ui/docs/troubleshooting.html" => {
            &["Troubleshooting Mirror", "Open reading paths"]
        }
        "apps/hub-gui/ui/docs/update-catalog.html" => &[
            "Mirror · Chapter 7",
            "Open central chapter",
            "Open source page",
        ],
        _ => &[],
    }
}

fn extract_local_hrefs(text: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("href=\"") {
        let after_start = &rest[start + "href=\"".len()..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        let href = &after_start[..end];
        if !href.is_empty()
            && !href.starts_with('#')
            && !href.starts_with("http://")
            && !href.starts_with("https://")
        {
            hrefs.push(href.to_string());
        }
        rest = &after_start[end + 1..];
    }
    hrefs
}

fn resolve_local_href(source_file: &Path, href: &str) -> PathBuf {
    source_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(href)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = root.join(relative_path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[derive(Debug, Default)]
struct SyncOptions {
    help: bool,
    version: Option<String>,
    line: Option<String>,
}

impl SyncOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut index = 0;
        while index < args.len() {
            let value = args[index]
                .to_str()
                .ok_or_else(|| "sync-doc-book-version received non-utf8 argument".to_string())?;
            match value {
                "--help" | "-h" => {
                    options.help = true;
                    index += 1;
                }
                "--version" => {
                    let next = args
                        .get(index + 1)
                        .and_then(|arg| arg.to_str())
                        .ok_or_else(|| "--version requires a value".to_string())?;
                    options.version = Some(next.to_string());
                    index += 2;
                }
                "--line" => {
                    let next = args
                        .get(index + 1)
                        .and_then(|arg| arg.to_str())
                        .ok_or_else(|| "--line requires a value".to_string())?;
                    options.line = Some(next.to_string());
                    index += 2;
                }
                _ => return Err(format!("Unknown argument: {value}")),
            }
        }
        Ok(options)
    }
}

struct ReplacementRule {
    prefix: &'static str,
    suffix: &'static str,
    replacement: String,
    version_kind: VersionKind,
}

#[derive(Clone, Copy)]
enum VersionKind {
    Semver,
    MinorX,
}

impl ReplacementRule {
    fn apply(&self, text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find(self.prefix) {
            output.push_str(&rest[..start]);
            let after_prefix = &rest[start + self.prefix.len()..];
            let Some(version_len) = version_token_len(after_prefix, self.version_kind) else {
                output.push_str(self.prefix);
                rest = after_prefix;
                continue;
            };
            let after_version = &after_prefix[version_len..];
            if !after_version.starts_with(self.suffix) {
                output.push_str(self.prefix);
                output.push_str(&after_prefix[..version_len]);
                rest = after_version;
                continue;
            }
            output.push_str(self.prefix);
            output.push_str(&self.replacement);
            output.push_str(self.suffix);
            rest = &after_version[self.suffix.len()..];
        }
        output.push_str(rest);
        output
    }
}

fn sync_replacements(
    shipping_version: &str,
    version_line: &str,
    minor_line: &str,
) -> Vec<(&'static str, Vec<ReplacementRule>)> {
    let display_version = version_line
        .strip_prefix("moxi ")
        .unwrap_or(version_line)
        .to_string();
    vec![
        (
            "docs/book.html",
            vec![
                semver_rule("One book for moxi ", "", &display_version),
                semver_rule("Version line: moxi ", "", &display_version),
                semver_rule("Shipping version: ", "", shipping_version),
                minor_rule("Current prep: ", "", &format!("{minor_line}.x")),
            ],
        ),
        (
            "docs/book-manifest.json",
            vec![
                semver_rule("\"version_line\": \"moxi ", "\"", &display_version),
                semver_rule("\"shipping_version\": \"", "\"", shipping_version),
            ],
        ),
        (
            "apps/hub-gui/ui/docs/index.html",
            vec![
                semver_rule("Desktop reading entry for moxi ", "", &display_version),
                semver_rule("Current line: moxi ", "", &display_version),
            ],
        ),
        (
            "apps/hub-gui/ui/docs/current-line.html",
            vec![semver_rule(">moxi ", "<", &display_version)],
        ),
    ]
}

fn semver_rule(prefix: &'static str, suffix: &'static str, replacement: &str) -> ReplacementRule {
    ReplacementRule {
        prefix,
        suffix,
        replacement: replacement.to_string(),
        version_kind: VersionKind::Semver,
    }
}

fn minor_rule(prefix: &'static str, suffix: &'static str, replacement: &str) -> ReplacementRule {
    ReplacementRule {
        prefix,
        suffix,
        replacement: replacement.to_string(),
        version_kind: VersionKind::MinorX,
    }
}

fn semver_minor(version: &str) -> RunnerResult<String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.iter().take(2).any(|part| part.is_empty()) {
        return Err(format!("invalid semantic version: {version}"));
    }
    Ok(format!("{}.{}", parts[0], parts[1]))
}

fn version_token_len(text: &str, kind: VersionKind) -> Option<usize> {
    match kind {
        VersionKind::Semver => parse_version_token_len(text, 3, false),
        VersionKind::MinorX => parse_version_token_len(text, 2, true),
    }
}

fn parse_version_token_len(text: &str, numeric_segments: usize, trailing_x: bool) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;
    for segment in 0..numeric_segments {
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return None;
        }
        if segment + 1 < numeric_segments {
            if bytes.get(index) != Some(&b'.') {
                return None;
            }
            index += 1;
        }
    }
    if trailing_x {
        if bytes.get(index) != Some(&b'.') || bytes.get(index + 1) != Some(&b'x') {
            return None;
        }
        index += 2;
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use super::{
        ReplacementRule, VersionKind, extract_local_hrefs, required_snippets, semver_minor,
    };

    #[test]
    fn extracts_local_hrefs_like_the_legacy_checker() {
        let hrefs = extract_local_hrefs(
            r##"<a href="./book.html">book</a><a href="#local">local</a><a href="https://example.invalid">remote</a>"##,
        );
        assert_eq!(hrefs, vec!["./book.html"]);
    }

    #[test]
    fn required_snippets_cover_hub_mirrors() {
        assert!(
            required_snippets("apps/hub-gui/ui/docs/index.html").contains(&"Open central book")
        );
        assert!(required_snippets("docs/book-ch08-reading-paths.html").contains(&"docs/README.md"));
    }

    #[test]
    fn replacement_rule_updates_semver_tokens() {
        let rule = ReplacementRule {
            prefix: "Current line: moxi ",
            suffix: "<",
            replacement: "2.0.0".to_string(),
            version_kind: VersionKind::Semver,
        };
        assert_eq!(
            rule.apply("Current line: moxi 1.19.0<"),
            "Current line: moxi 2.0.0<"
        );
    }

    #[test]
    fn semver_minor_keeps_major_and_minor() {
        assert_eq!(semver_minor("1.20.0").unwrap(), "1.20");
    }
}
