use super::{AuditResult, HexAdvisoryMitigation, HexOsvConfig, RunnerResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CACHE_SCHEMA: &str = "kyuubiki.hex-osv-cache/v2";
const COMMAND: &str = "OSV querybatch (Hex locked versions)";
const USER_AGENT: &str = "kyuubiki-dependency-audit/2";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct HexPackage {
    name: String,
    version: String,
    repository: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OsvBatchResponse {
    results: Vec<OsvQueryResult>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct OsvQueryResult {
    #[serde(default)]
    vulns: Vec<OsvVulnerability>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OsvVulnerability {
    id: String,
    modified: String,
}

#[derive(Deserialize, Serialize)]
struct OsvCache {
    schema_version: String,
    endpoint: String,
    ecosystem: String,
    query_repositories: Vec<String>,
    lock_sha256: String,
    fetched_at_unix_seconds: u64,
    response: OsvBatchResponse,
}

struct Evaluation {
    status: i32,
    summary: String,
    details: String,
}

enum LiveFailure {
    Transport(String),
    Invalid(String),
}

pub(super) fn run_hex_osv_audit(
    root: &Path,
    cwd: &str,
    config: &HexOsvConfig,
    mitigations: &[HexAdvisoryMitigation],
) -> AuditResult {
    let lock_path = root.join(cwd).join("mix.lock");
    let lock_text = match fs::read_to_string(&lock_path) {
        Ok(text) => text,
        Err(error) => {
            return failed_result(
                cwd,
                format!("failed to read {}: {error}", lock_path.display()),
            );
        }
    };
    let packages = match parse_mix_lock(&lock_text) {
        Ok(packages) => packages,
        Err(error) => return failed_result(cwd, error),
    };
    let blocked_packages = blocked_query_packages(&packages, &config.query_repositories);
    if !blocked_packages.is_empty() {
        return failed_result(
            cwd,
            format!(
                "Hex packages from non-approved advisory query repositories: {}",
                blocked_packages.join(", ")
            ),
        );
    }
    let lock_sha256 = sha256_hex(lock_text.as_bytes());
    let payload = query_payload(&packages, &config.ecosystem);

    match query_live(config, &payload) {
        Ok(response) => {
            let evaluation = evaluate(&packages, &response, mitigations, "OSV live");
            let cache_note = if evaluation.status == 0 {
                write_cache(root, config, &lock_sha256, response)
                    .err()
                    .map(|error| format!("; cache warning: {error}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            audit_result(cwd, evaluation, cache_note)
        }
        Err(LiveFailure::Transport(live_error)) => {
            match read_fresh_cache(root, config, &lock_sha256) {
                Ok((response, age)) => {
                    let source = format!("OSV cache, {}s old", age.as_secs());
                    let evaluation = evaluate(&packages, &response, mitigations, &source);
                    audit_result(
                        cwd,
                        evaluation,
                        format!("; live query unavailable: {live_error}"),
                    )
                }
                Err(cache_error) => failed_result(
                    cwd,
                    format!(
                        "OSV live query failed: {live_error}; usable cache unavailable: {cache_error}"
                    ),
                ),
            }
        }
        Err(LiveFailure::Invalid(error)) => {
            failed_result(cwd, format!("OSV returned an invalid response: {error}"))
        }
    }
}

fn blocked_query_packages(packages: &[HexPackage], repositories: &[String]) -> Vec<String> {
    let allowed = repositories.iter().collect::<BTreeSet<_>>();
    packages
        .iter()
        .filter(|package| !allowed.contains(&package.repository))
        .map(|package| format!("{} ({})", package.name, package.repository))
        .collect()
}

fn audit_result(cwd: &str, evaluation: Evaluation, summary_suffix: String) -> AuditResult {
    AuditResult {
        command: COMMAND.to_string(),
        cwd: cwd.to_string(),
        status: evaluation.status,
        stdout: evaluation.details,
        stderr: String::new(),
        summary: format!("{}{}", evaluation.summary, summary_suffix),
    }
}

fn failed_result(cwd: &str, error: String) -> AuditResult {
    AuditResult {
        command: COMMAND.to_string(),
        cwd: cwd.to_string(),
        status: 1,
        stdout: String::new(),
        stderr: error,
        summary: "Hex vulnerability audit incomplete".to_string(),
    }
}

fn parse_mix_lock(text: &str) -> RunnerResult<Vec<HexPackage>> {
    let marker = "{:hex, :";
    let mut packages = BTreeMap::new();
    for line in text.lines() {
        let Some(start) = line.find(marker) else {
            continue;
        };
        let rest = &line[start + marker.len()..];
        let (name, rest) = rest
            .split_once(',')
            .ok_or_else(|| "mix.lock contains a malformed Hex package entry".to_string())?;
        let name = name.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(format!(
                "mix.lock contains an invalid Hex package name {name:?}"
            ));
        }
        let version_field = rest.trim_start();
        let version_tail = version_field
            .strip_prefix('"')
            .ok_or_else(|| format!("mix.lock package {name} has no locked version"))?;
        let (version, _) = version_tail
            .split_once('"')
            .ok_or_else(|| format!("mix.lock package {name} has a malformed version"))?;
        if version.is_empty() || version.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(format!("mix.lock package {name} has an invalid version"));
        }
        let repository_tail = line
            .rsplit_once("], \"")
            .map(|(_, tail)| tail)
            .ok_or_else(|| format!("mix.lock package {name} has no repository"))?;
        let (repository, _) = repository_tail
            .split_once('"')
            .ok_or_else(|| format!("mix.lock package {name} has a malformed repository"))?;
        if repository.is_empty()
            || !repository.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
            })
        {
            return Err(format!("mix.lock package {name} has an invalid repository"));
        }
        let package = HexPackage {
            name: name.to_string(),
            version: version.to_string(),
            repository: repository.to_string(),
        };
        if let Some(previous) = packages.insert(name.to_string(), package.clone())
            && previous != package
        {
            return Err(format!(
                "mix.lock contains conflicting entries for Hex package {name}"
            ));
        }
    }
    if packages.is_empty() {
        return Err("mix.lock contains no Hex package entries".to_string());
    }
    Ok(packages.into_values().collect())
}

fn query_payload(packages: &[HexPackage], ecosystem: &str) -> Value {
    json!({
        "queries": packages.iter().map(|package| json!({
            "version": package.version,
            "package": {
                "name": package.name,
                "ecosystem": ecosystem
            }
        })).collect::<Vec<_>>()
    })
}

fn query_live(config: &HexOsvConfig, payload: &Value) -> Result<OsvBatchResponse, LiveFailure> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(config.timeout_seconds)))
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .post(&config.endpoint)
        .header("User-Agent", USER_AGENT)
        .send_json(payload)
        .map_err(|error| LiveFailure::Transport(error.to_string()))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| LiveFailure::Transport(error.to_string()))?;
    serde_json::from_str(&body).map_err(|error| LiveFailure::Invalid(error.to_string()))
}

fn evaluate(
    packages: &[HexPackage],
    response: &OsvBatchResponse,
    mitigations: &[HexAdvisoryMitigation],
    source: &str,
) -> Evaluation {
    if response.results.len() != packages.len() {
        return invalid_evaluation(format!(
            "response result count {} does not match query count {}",
            response.results.len(),
            packages.len()
        ));
    }
    if response.results.iter().any(|result| {
        result
            .next_page_token
            .as_deref()
            .is_some_and(|token| !token.is_empty())
    }) {
        return invalid_evaluation("paginated OSV response is not accepted".to_string());
    }

    let mitigation_keys = mitigations
        .iter()
        .map(|item| {
            (
                item.id.as_str(),
                item.package.as_str(),
                item.locked_version.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut observed_mitigations = BTreeSet::new();
    let mut unmitigated = Vec::new();
    let mut advisory_count = 0usize;
    for (package, result) in packages.iter().zip(&response.results) {
        let mut local_ids = BTreeSet::new();
        for advisory in &result.vulns {
            if advisory.id.trim().is_empty() || advisory.modified.trim().is_empty() {
                return invalid_evaluation(
                    "advisory id or modified timestamp is empty".to_string(),
                );
            }
            if !local_ids.insert(advisory.id.as_str()) {
                continue;
            }
            advisory_count += 1;
            let key = (
                advisory.id.as_str(),
                package.name.as_str(),
                package.version.as_str(),
            );
            if mitigation_keys.contains(&key) {
                observed_mitigations.insert(key);
            } else {
                unmitigated.push(format!(
                    "{} {}: {}",
                    package.name, package.version, advisory.id
                ));
            }
        }
    }
    let stale_mitigations = mitigation_keys
        .difference(&observed_mitigations)
        .map(|(id, package, version)| format!("{package} {version}: {id}"))
        .collect::<Vec<_>>();
    if !unmitigated.is_empty() || !stale_mitigations.is_empty() {
        let mut details = unmitigated
            .iter()
            .map(|item| format!("- unmitigated: {item}"))
            .collect::<Vec<_>>();
        details.extend(
            stale_mitigations
                .iter()
                .map(|item| format!("- stale mitigation: {item}")),
        );
        return Evaluation {
            status: 1,
            summary: format!(
                "{} unmitigated and {} stale Hex advisory exception(s); {source}",
                unmitigated.len(),
                stale_mitigations.len()
            ),
            details: details.join("\n"),
        };
    }

    Evaluation {
        status: 0,
        summary: format!(
            "{advisory_count} known vulnerability record(s), {} explicit mitigation(s), {} locked Hex package(s); {source}",
            observed_mitigations.len(),
            packages.len()
        ),
        details: String::new(),
    }
}

fn invalid_evaluation(error: String) -> Evaluation {
    Evaluation {
        status: 1,
        summary: "invalid OSV batch response".to_string(),
        details: error,
    }
}

fn write_cache(
    root: &Path,
    config: &HexOsvConfig,
    lock_sha256: &str,
    response: OsvBatchResponse,
) -> RunnerResult<()> {
    let path = root.join(&config.cache_path);
    reject_symlink_cache(root, &path)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Hex OSV cache path has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create Hex OSV cache directory: {error}"))?;
    reject_symlink_cache(root, &path)?;
    let cache = OsvCache {
        schema_version: CACHE_SCHEMA.to_string(),
        endpoint: config.endpoint.clone(),
        ecosystem: config.ecosystem.clone(),
        query_repositories: config.query_repositories.clone(),
        lock_sha256: lock_sha256.to_string(),
        fetched_at_unix_seconds: unix_seconds(SystemTime::now())?,
        response,
    };
    let encoded = serde_json::to_vec(&cache)
        .map_err(|error| format!("failed to encode Hex OSV cache: {error}"))?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temporary, encoded)
        .map_err(|error| format!("failed to write Hex OSV cache: {error}"))?;
    #[cfg(windows)]
    if path.is_file() {
        fs::remove_file(&path)
            .map_err(|error| format!("failed to replace Hex OSV cache: {error}"))?;
    }
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("failed to install Hex OSV cache: {error}")
    })
}

fn read_fresh_cache(
    root: &Path,
    config: &HexOsvConfig,
    lock_sha256: &str,
) -> RunnerResult<(OsvBatchResponse, Duration)> {
    let path = root.join(&config.cache_path);
    reject_symlink_cache(root, &path)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", config.cache_path))?;
    let cache: OsvCache =
        serde_json::from_str(&text).map_err(|error| format!("invalid Hex OSV cache: {error}"))?;
    if cache.schema_version != CACHE_SCHEMA {
        return Err("Hex OSV cache schema drifted".to_string());
    }
    if cache.endpoint != config.endpoint
        || cache.ecosystem != config.ecosystem
        || cache.query_repositories != config.query_repositories
    {
        return Err("Hex OSV cache belongs to a different advisory source".to_string());
    }
    if cache.lock_sha256 != lock_sha256 {
        return Err("Hex OSV cache belongs to a different mix.lock".to_string());
    }
    let now = unix_seconds(SystemTime::now())?;
    let age = now
        .checked_sub(cache.fetched_at_unix_seconds)
        .map(Duration::from_secs)
        .ok_or_else(|| "Hex OSV cache timestamp is in the future".to_string())?;
    if age > Duration::from_secs(config.cache_max_age_seconds) {
        return Err(format!("Hex OSV cache is stale ({}s old)", age.as_secs()));
    }
    Ok((cache.response, age))
}

fn reject_symlink_cache(root: &Path, path: &Path) -> RunnerResult<()> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "Hex OSV cache path escapes the repository root".to_string())?;
    let mut candidate = root.to_path_buf();
    for component in relative.components() {
        candidate.push(component);
        if fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!(
                "Hex OSV cache path traverses symlink {}",
                candidate.display()
            ));
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unix_seconds(time: SystemTime) -> RunnerResult<u64> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "system time is before Unix epoch".to_string())
}

pub(super) fn validate_config(config: &HexOsvConfig) -> RunnerResult<()> {
    let endpoint = config
        .endpoint
        .strip_prefix("https://")
        .ok_or_else(|| format!("{COMMAND} endpoint must use HTTPS"))?;
    let authority = endpoint.split('/').next().unwrap_or_default();
    if authority.is_empty()
        || authority.contains('@')
        || config.endpoint.contains('?')
        || config.endpoint.contains('#')
        || config
            .endpoint
            .bytes()
            .any(|byte| byte.is_ascii_whitespace())
    {
        return Err(format!("{COMMAND} endpoint is unsafe"));
    }
    if config.ecosystem != "Hex" {
        return Err(format!("{COMMAND} ecosystem must be Hex"));
    }
    let mut repositories = BTreeSet::new();
    if config.query_repositories.is_empty()
        || config.query_repositories.iter().any(|repository| {
            repository.is_empty()
                || !repository.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
                })
                || !repositories.insert(repository.as_str())
        })
    {
        return Err(format!(
            "{COMMAND} query_repositories must contain unique safe names"
        ));
    }
    let cache_path = Path::new(&config.cache_path);
    let components = cache_path.components().collect::<Vec<_>>();
    if cache_path.is_absolute()
        || components.first() != Some(&Component::Normal("tmp".as_ref()))
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
        || cache_path.extension().and_then(|value| value.to_str()) != Some("json")
    {
        return Err(format!(
            "{COMMAND} cache_path must be a relative tmp/*.json path"
        ));
    }
    if !(60..=7 * 24 * 60 * 60).contains(&config.cache_max_age_seconds) {
        return Err(format!("{COMMAND} cache age must be between 60s and 7d"));
    }
    if !(1..=120).contains(&config.timeout_seconds) {
        return Err(format!("{COMMAND} timeout must be between 1s and 120s"));
    }
    Ok(())
}

pub(super) fn run_self_test(config: &HexOsvConfig) -> RunnerResult<()> {
    validate_config(config)?;
    if config.endpoint != "https://api.osv.dev/v1/querybatch"
        || config.ecosystem != "Hex"
        || config.query_repositories != ["hexpm"]
        || config.cache_path != "tmp/dependency-audit/hex-osv-cache.json"
        || config.cache_max_age_seconds != 86_400
        || config.timeout_seconds != 20
    {
        return Err("self-test Hex OSV contract drifted".to_string());
    }
    let packages = parse_mix_lock(
        "%{\n  \"bandit\": {:hex, :bandit, \"1.12.5\", \"digest\", [:mix], [], \"hexpm\", \"outer\"}\n}",
    )?;
    if packages
        != [HexPackage {
            name: "bandit".to_string(),
            version: "1.12.5".to_string(),
            repository: "hexpm".to_string(),
        }]
    {
        return Err("self-test Hex lock parser drifted".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn package(name: &str, version: &str) -> HexPackage {
        HexPackage {
            name: name.to_string(),
            version: version.to_string(),
            repository: "hexpm".to_string(),
        }
    }

    fn response(ids: &[&str]) -> OsvBatchResponse {
        OsvBatchResponse {
            results: vec![OsvQueryResult {
                vulns: ids
                    .iter()
                    .map(|id| OsvVulnerability {
                        id: (*id).to_string(),
                        modified: "2026-08-18T00:00:00Z".to_string(),
                    })
                    .collect(),
                next_page_token: None,
            }],
        }
    }

    fn mitigation(id: &str) -> HexAdvisoryMitigation {
        HexAdvisoryMitigation {
            id: id.to_string(),
            package: "cowlib".to_string(),
            locked_version: "2.19.0".to_string(),
            status: "mitigated".to_string(),
            evidence: vec!["docs/security.md".to_string()],
        }
    }

    fn config() -> HexOsvConfig {
        HexOsvConfig {
            endpoint: "https://api.osv.dev/v1/querybatch".to_string(),
            ecosystem: "Hex".to_string(),
            query_repositories: vec!["hexpm".to_string()],
            cache_path: "tmp/dependency-audit/hex-osv-cache.json".to_string(),
            cache_max_age_seconds: 86_400,
            timeout_seconds: 20,
        }
    }

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "kyuubiki-hex-osv-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn parses_sorted_locked_hex_packages() {
        let lock = "%{\n  \"plug\": {:hex, :plug, \"1.20.3\", \"a\", [:mix], [], \"hexpm\", \"b\"},\n  \"bandit\": {:hex, :bandit, \"1.12.5\", \"c\", [:mix], [], \"hexpm\", \"d\"}\n}";
        assert_eq!(
            parse_mix_lock(lock).unwrap(),
            [package("bandit", "1.12.5"), package("plug", "1.20.3")]
        );
        assert!(parse_mix_lock("%{}").is_err());
    }

    #[test]
    fn rejects_unmitigated_and_stale_advisories() {
        let packages = [package("cowlib", "2.19.0")];
        let unmitigated = evaluate(&packages, &response(&["EEF-CVE-2026-43966"]), &[], "test");
        assert_eq!(unmitigated.status, 1);
        assert!(unmitigated.details.contains("unmitigated"));

        let stale = evaluate(
            &packages,
            &response(&[]),
            &[mitigation("EEF-CVE-2026-43966")],
            "test",
        );
        assert_eq!(stale.status, 1);
        assert!(stale.details.contains("stale mitigation"));
    }

    #[test]
    fn accepts_exact_lock_bound_mitigation() {
        let packages = [package("cowlib", "2.19.0")];
        let evaluation = evaluate(
            &packages,
            &response(&["EEF-CVE-2026-43966"]),
            &[mitigation("EEF-CVE-2026-43966")],
            "test",
        );
        assert_eq!(evaluation.status, 0);
        assert!(evaluation.summary.contains("1 explicit mitigation"));
    }

    #[test]
    fn rejects_response_shape_drift_and_pagination() {
        let packages = [package("bandit", "1.12.5")];
        let wrong_count = OsvBatchResponse { results: vec![] };
        assert_eq!(evaluate(&packages, &wrong_count, &[], "test").status, 1);

        let paginated = OsvBatchResponse {
            results: vec![OsvQueryResult {
                vulns: vec![],
                next_page_token: Some("next".to_string()),
            }],
        };
        assert_eq!(evaluate(&packages, &paginated, &[], "test").status, 1);
    }

    #[test]
    fn config_requires_https_hex_and_tmp_cache() {
        let mut config = config();
        assert!(validate_config(&config).is_ok());
        config.endpoint = "http://api.osv.dev/v1/querybatch".to_string();
        assert!(validate_config(&config).is_err());
        config.endpoint = "https://api.osv.dev/v1/querybatch?token=secret".to_string();
        assert!(validate_config(&config).is_err());
        config.endpoint = "https://api.osv.dev/v1/querybatch".to_string();
        config.cache_path = "../hex-osv-cache.json".to_string();
        assert!(validate_config(&config).is_err());
        config.cache_path = "tmp/dependency-audit/hex-osv-cache.json".to_string();
        config.query_repositories.push("hexpm".to_string());
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn blocks_unapproved_lock_repositories_before_querying() {
        let mut private = package("private_solver", "1.0.0");
        private.repository = "hexpm:research".to_string();
        assert_eq!(
            blocked_query_packages(&[private], &["hexpm".to_string()]),
            ["private_solver (hexpm:research)"]
        );
    }

    #[test]
    fn cache_is_lock_bound_fresh_and_not_future_dated() {
        let root = temporary_root("cache");
        let config = config();
        let path = root.join(&config.cache_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let now = unix_seconds(SystemTime::now()).unwrap();
        let write = |lock_sha256: &str, fetched_at_unix_seconds: u64| {
            let cache = OsvCache {
                schema_version: CACHE_SCHEMA.to_string(),
                endpoint: config.endpoint.clone(),
                ecosystem: config.ecosystem.clone(),
                query_repositories: config.query_repositories.clone(),
                lock_sha256: lock_sha256.to_string(),
                fetched_at_unix_seconds,
                response: response(&[]),
            };
            fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
        };

        write("current-lock", now);
        assert!(read_fresh_cache(&root, &config, "current-lock").is_ok());
        assert!(read_fresh_cache(&root, &config, "other-lock").is_err());

        let mut other_source = config.clone();
        other_source.endpoint = "https://osv.example.test/v1/querybatch".to_string();
        assert!(read_fresh_cache(&root, &other_source, "current-lock").is_err());
        let mut other_repositories = config.clone();
        other_repositories
            .query_repositories
            .push("hexpm:research".to_string());
        assert!(read_fresh_cache(&root, &other_repositories, "current-lock").is_err());

        write("current-lock", now + 60);
        assert!(read_fresh_cache(&root, &config, "current-lock").is_err());

        write("current-lock", now - config.cache_max_age_seconds - 1);
        assert!(read_fresh_cache(&root, &config, "current-lock").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
