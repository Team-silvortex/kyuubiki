use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "docs/ui-automation-contract.json";
const HTML_PATH: &str = "docs/ui-automation-contract.html";
const TS_CONTRACT_PATH: &str =
    "apps/frontend/src/components/workbench/workbench-ui-automation-contract.ts";

pub(crate) fn run_check_ui_automation_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("UI automation contract self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-ui-automation-contract only accepts --self-test".to_string());
    }
    let issues = audit(root)?;
    if !issues.is_empty() {
        eprintln!("UI automation contract drift detected:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }
    println!("UI automation contract ok");
    Ok(0)
}

fn audit(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    let contract = read_json(root, CONTRACT_PATH)?;
    let ts_contract = read_text(root, TS_CONTRACT_PATH)?;
    let html_contract = read_text(root, HTML_PATH)?;

    if string_field(&contract, "name") != "Kyuubiki Workbench UI Automation Contract" {
        issues.push("docs/ui-automation-contract.json has an unexpected contract name".to_string());
    }
    if contract.get("productOwned").and_then(Value::as_bool) != Some(true)
        || contract.get("userExtensible").and_then(Value::as_bool) != Some(false)
    {
        issues.push(
            "UI automation contract must remain product-owned and non-user-extensible".to_string(),
        );
    }
    if contract.get("contractVersion").and_then(Value::as_i64) != Some(1) {
        issues.push(format!(
            "expected contractVersion 1, got {}",
            contract
                .get("contractVersion")
                .cloned()
                .unwrap_or(Value::Null)
        ));
    }
    require_contains(
        &mut issues,
        HTML_PATH,
        &html_contract,
        string_field(&contract, "version"),
        "contract version line",
    );
    let rules = string_array(&contract, "rules");
    if !rules
        .iter()
        .any(|rule| rule.contains("stable data-* selectors"))
    {
        issues.push(
            "contract rules must require stable selectors instead of visual text".to_string(),
        );
    }
    if !rules.iter().any(|rule| rule.contains("product-owned ids")) {
        issues.push("contract rules must mark automation labels as product-owned ids".to_string());
    }
    require_contains(
        &mut issues,
        TS_CONTRACT_PATH,
        &ts_contract,
        "WORKBENCH_UI_AUTOMATION_CONTRACT_VERSION = 1",
        "matching contract version",
    );

    for selector in required_selectors() {
        if contract
            .pointer(&format!("/selectors/{}", escape_pointer(selector.key)))
            .and_then(Value::as_str)
            != Some(selector.value)
        {
            issues.push(format!(
                "docs/ui-automation-contract.json selector {} must equal {}",
                selector.key, selector.value
            ));
        }
        require_contains(
            &mut issues,
            TS_CONTRACT_PATH,
            &ts_contract,
            selector.ts_needle.unwrap_or(selector.value),
            &format!("selector {}", selector.key),
        );
        require_contains(
            &mut issues,
            HTML_PATH,
            &html_contract,
            selector.value,
            &format!("documented selector {}", selector.key),
        );
        for file in selector.implementation_files {
            require_contains(
                &mut issues,
                file,
                &read_text(root, file)?,
                selector.implementation_needle,
                &format!("implementation for {}", selector.key),
            );
        }
    }
    Ok(issues)
}

fn run_self_test() -> RunnerResult<()> {
    let selectors = required_selectors()
        .into_iter()
        .map(|selector| (selector.key.to_string(), json!(selector.value)))
        .collect::<serde_json::Map<_, _>>();
    let contract = json!({
        "name": "Kyuubiki Workbench UI Automation Contract",
        "productOwned": true,
        "userExtensible": false,
        "contractVersion": 1,
        "selectors": selectors,
        "rules": [
            "Automation must target stable data-* selectors.",
            "Accessible labels are product-owned ids."
        ]
    });
    if contract
        .pointer("/selectors/railButton(section)")
        .and_then(Value::as_str)
        != Some("workbench-rail:${section}")
    {
        return Err("self-test selector lookup failed".to_string());
    }
    if contract.pointer("/selectors/missing").is_some() {
        return Err("self-test missing selector lookup failed".to_string());
    }
    if !required_selectors()
        .iter()
        .any(|selector| selector.key == "loadedModelState")
    {
        return Err("self-test selector table missing loadedModelState".to_string());
    }
    Ok(())
}

fn require_contains(issues: &mut Vec<String>, file: &str, text: &str, needle: &str, label: &str) {
    if !text.contains(needle) {
        issues.push(format!("{file} is missing {label}: {needle}"));
    }
}

fn required_selectors() -> Vec<RequiredSelector> {
    vec![
        RequiredSelector::new(
            "shell",
            "[data-workbench-shell=\"root\"]",
            &["apps/frontend/src/components/workbench/workbench-shell-frame.tsx"],
            "data-workbench-shell=\"root\"",
        ),
        RequiredSelector::new(
            "sidebar",
            "[data-workbench-panel=\"sidebar\"]",
            &["apps/frontend/src/components/workbench/workbench-sidebar-panel.tsx"],
            "data-workbench-panel=\"sidebar\"",
        ),
        RequiredSelector::with_ts(
            "railButton(section)",
            "workbench-rail:${section}",
            "workbench-rail:",
            &["apps/frontend/src/components/workbench/workbench-app-rail.tsx"],
            "workbench-rail:",
        ),
        RequiredSelector::new(
            "loadedModelState",
            "[data-workbench-state=\"loaded-model\"]",
            &["apps/frontend/src/components/workbench/workbench-main-shell-mount.tsx"],
            "data-workbench-state=\"loaded-model\"",
        ),
        RequiredSelector::with_ts(
            "libraryTab(tab)",
            "workbench-library-tab:${tab}",
            "workbench-library-tab:",
            &["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
            "workbench-library-tab:",
        ),
        RequiredSelector::with_ts(
            "sampleDomain(domain)",
            "workbench-sample-domain:${domain}",
            "workbench-sample-domain:",
            &["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
            "workbench-sample-domain:",
        ),
        RequiredSelector::with_ts(
            "sample(sampleId)",
            "workbench-sample:${sampleId}",
            "workbench-sample:",
            &["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
            "workbench-sample:",
        ),
        RequiredSelector::new(
            "runtimePanel",
            "[data-workbench-runtime=\"panel\"]",
            &["apps/frontend/src/components/workbench/system/workbench-system-runtime-panel.tsx"],
            "data-workbench-runtime=\"panel\"",
        ),
        RequiredSelector::new(
            "controlWindow",
            "[data-workbench-control-window=\"root\"]",
            &[
                "apps/frontend/src/components/workbench/system/workbench-system-control-mode-window.tsx",
            ],
            "data-workbench-control-window=\"root\"",
        ),
    ]
}

struct RequiredSelector {
    key: &'static str,
    value: &'static str,
    ts_needle: Option<&'static str>,
    implementation_files: &'static [&'static str],
    implementation_needle: &'static str,
}

impl RequiredSelector {
    fn new(
        key: &'static str,
        value: &'static str,
        implementation_files: &'static [&'static str],
        implementation_needle: &'static str,
    ) -> Self {
        Self {
            key,
            value,
            ts_needle: None,
            implementation_files,
            implementation_needle,
        }
    }

    fn with_ts(
        key: &'static str,
        value: &'static str,
        ts_needle: &'static str,
        implementation_files: &'static [&'static str],
        implementation_needle: &'static str,
    ) -> Self {
        Self {
            key,
            value,
            ts_needle: Some(ts_needle),
            implementation_files,
            implementation_needle,
        }
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn string_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{escape_pointer, required_selectors};

    #[test]
    fn selector_table_contains_loaded_state() {
        assert!(
            required_selectors()
                .iter()
                .any(|selector| selector.key == "loadedModelState")
        );
    }

    #[test]
    fn json_pointer_escape_handles_slashes() {
        assert_eq!(escape_pointer("a/b"), "a~1b");
    }
}
