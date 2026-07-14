use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

mod boundary;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/gui-runtime-capability-manifest.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.gui-runtime-capability-manifest.json";
const MANIFEST_DIR: &str = "config/gui-runtime-capabilities";
const SCHEMA_VERSION: &str = "kyuubiki.gui-runtime-capability-manifest/v1";
const UI_OWNERSHIP: &str = "product_owned_static_ui";

pub(crate) fn run_check_gui_runtime_capability_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("GUI runtime capability contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-gui-runtime-capability-contract only accepts --self-test".to_string());
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("GUI runtime capability contract check failed: {issue}");
        return Ok(1);
    }
    println!("GUI runtime capability contract check passed");
    Ok(0)
}

fn check_contracts(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    check_schema(root, &read_json(root, SCHEMA_PATH)?, &mut issues);
    check_manifest(
        root,
        &read_json(root, EXAMPLE_PATH)?,
        EXAMPLE_PATH,
        &mut issues,
    );
    check_manifest_set(root, &mut issues)?;
    check_documentation(root, &mut issues)?;
    check_frontend_capability_api(root, &mut issues)?;
    boundary::check_runtime_client_boundary(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut example = read_json(root, EXAMPLE_PATH)?;
    if let Some(binding) = example.pointer_mut("/runtime_bindings/0/headless_sdk_parity_required") {
        *binding = Value::Bool(false);
    }
    let mut issues = Vec::new();
    check_manifest(root, &example, EXAMPLE_PATH, &mut issues);
    if issues.is_empty() {
        return Err("self-test expected headless_sdk_parity_required failure".to_string());
    }

    let mut mobile = read_json(root, "config/gui-runtime-capabilities/mobile-webview.json")?;
    if let Some(target) = mobile.pointer_mut("/runtime_bindings/0/target_kind") {
        *target = Value::String("agent".to_string());
    }
    let mut mobile_issues = Vec::new();
    check_manifest(root, &mobile, "mobile-self-test", &mut mobile_issues);
    if mobile_issues.is_empty() {
        return Err("self-test expected mobile forbidden target failure".to_string());
    }
    Ok(())
}

fn check_schema(root: &Path, schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/properties/ui_ownership/const")
        .and_then(Value::as_str)
        != Some(UI_OWNERSHIP)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: ui_ownership const must be {UI_OWNERSHIP}"
        ));
    }
    if schema
        .pointer("/properties/automation_contract/properties/wasm_python_stability_required/const")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: automation contract must require WASM Python stability"
        ));
    }
    if schema
        .pointer("/properties/automation_contract/properties/user_extensible_ui_allowed/const")
        .and_then(Value::as_bool)
        != Some(false)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: GUI automation UI must remain non-user-extensible"
        ));
    }
    if !root.join(SCHEMA_PATH).exists() {
        issues.push(format!("{SCHEMA_PATH}: schema file missing"));
    }
}

fn check_manifest(root: &Path, manifest: &Value, context: &str, issues: &mut Vec<String>) {
    if field(manifest, "schema_version") != SCHEMA_VERSION {
        issues.push(format!(
            "{context}: schema_version must be {SCHEMA_VERSION}"
        ));
    }
    if field(manifest, "ui_ownership") != UI_OWNERSHIP {
        issues.push(format!("{context}: ui_ownership must be {UI_OWNERSHIP}"));
    }
    require_string(manifest, "surface_id", context, issues);
    require_string(manifest, "surface_kind", context, issues);
    let surface_kind = field(manifest, "surface_kind");
    if manifest
        .pointer("/automation_contract/wasm_python_stability_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push(format!(
            "{context}: automation_contract.wasm_python_stability_required must be true"
        ));
    }
    if manifest
        .pointer("/automation_contract/user_extensible_ui_allowed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        issues.push(format!(
            "{context}: automation_contract.user_extensible_ui_allowed must be false"
        ));
    }

    let bindings = array_field(manifest, "runtime_bindings");
    if bindings.is_empty() {
        issues.push(format!(
            "{context}: runtime_bindings must be an array with at least 1 item(s)"
        ));
    }
    let mut binding_ids = BTreeSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        let binding_context = format!("{context}#runtime_bindings/{index}");
        require_string(binding, "binding_id", &binding_context, issues);
        let binding_id = field(binding, "binding_id");
        if !binding_id.is_empty() && !binding_ids.insert(binding_id.to_string()) {
            issues.push(format!(
                "{binding_context}: duplicate binding_id {binding_id}"
            ));
        }
        require_string(binding, "target_kind", &binding_context, issues);
        require_string(binding, "binding_mode", &binding_context, issues);
        require_array(
            binding,
            "required_capabilities",
            &binding_context,
            1,
            issues,
        );
        require_array(
            binding,
            "optional_capabilities",
            &binding_context,
            0,
            issues,
        );
        if binding
            .get("headless_sdk_parity_required")
            .and_then(Value::as_bool)
            != Some(true)
        {
            issues.push(format!(
                "{binding_context}: headless_sdk_parity_required must stay true for decoupled surfaces"
            ));
        }
        let mobile_supported = binding.get("mobile_supported").and_then(Value::as_bool);
        if mobile_supported.is_none() {
            issues.push(format!(
                "{binding_context}: mobile_supported must be a boolean"
            ));
        }
        let target_kind = field(binding, "target_kind");
        if surface_kind == "mobile_webview" {
            if mobile_supported != Some(true) {
                issues.push(format!(
                    "{binding_context}: mobile WebView bindings must be mobile_supported"
                ));
            }
            if matches!(
                target_kind,
                "agent" | "mesh" | "direct_runtime" | "installer_runtime"
            ) {
                issues.push(format!(
                    "{binding_context}: mobile WebView must not bind to {target_kind}"
                ));
            }
        }
        if surface_kind == "hub"
            && matches!(
                target_kind,
                "agent" | "mesh" | "direct_runtime" | "installer_runtime"
            )
        {
            issues.push(format!(
                "{binding_context}: Hub must not bind directly to {target_kind}"
            ));
        }
        if surface_kind == "workbench" && target_kind == "installer_runtime" {
            issues.push(format!(
                "{binding_context}: Workbench must not bind directly to {target_kind}"
            ));
        }
        require_string(binding, "credential_surface", &binding_context, issues);
        require_string(binding, "notes", &binding_context, issues);
    }

    if !bindings
        .iter()
        .any(|binding| field(binding, "target_kind") == "orchestra")
    {
        issues.push(format!(
            "{context}: at least one orchestra binding is required"
        ));
    }
    if !bindings
        .iter()
        .any(|binding| binding.get("mobile_supported").and_then(Value::as_bool) == Some(true))
    {
        issues.push(format!(
            "{context}: at least one binding must support mobile WebView clients"
        ));
    }
    if surface_kind == "installer"
        && !bindings
            .iter()
            .any(|binding| field(binding, "target_kind") == "installer_runtime")
    {
        issues.push(format!(
            "{context}: Installer must expose an installer_runtime binding"
        ));
    }
    require_array(manifest, "degraded_modes", context, 1, issues);
    if !root.join(context).exists() && context.ends_with(".json") {
        issues.push(format!("{context}: manifest file missing"));
    }
}

fn check_manifest_set(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let mut surface_kinds = BTreeSet::new();
    for manifest_path in list_manifest_paths(root)? {
        let manifest = read_json(root, &manifest_path)?;
        check_manifest(root, &manifest, &manifest_path, issues);
        surface_kinds.insert(field(&manifest, "surface_kind").to_string());
    }
    for required in ["hub", "workbench", "installer", "mobile_webview"] {
        if !surface_kinds.contains(required) {
            issues.push(format!(
                "{MANIFEST_DIR}: missing product manifest for {required}"
            ));
        }
    }
    Ok(())
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    for doc in [
        "schemas/README.md",
        "docs/app-runtime-boundaries.md",
        "config/README.md",
    ] {
        let text = read_text(root, doc)?;
        for needle in [SCHEMA_PATH, EXAMPLE_PATH, MANIFEST_DIR, SCHEMA_VERSION] {
            require_contains(&text, needle, doc, issues);
        }
    }
    Ok(())
}

fn check_frontend_capability_api(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let capability = read_text(root, FRONTEND_CAPABILITY)?;
    let api_index = read_text(root, FRONTEND_API_INDEX)?;
    let tests = read_text(root, FRONTEND_CAPABILITY_TEST)?;
    for exported in [
        "GuiRuntimeCapabilityManifest",
        "listGuiRuntimeManifestCapabilities",
        "hasGuiRuntimeManifestCapability",
        "selectGuiRuntimeManifestBindings",
        "resolveWorkbenchGuiRuntimeCapabilityFromManifest",
    ] {
        require_contains(&capability, exported, FRONTEND_CAPABILITY, issues);
        require_contains(&tests, exported, FRONTEND_CAPABILITY_TEST, issues);
    }
    require_contains(
        &api_index,
        "export * from \"./gui-runtime-capabilities.ts\";",
        FRONTEND_API_INDEX,
        issues,
    );
    require_contains(
        &capability,
        "options.hostKind === \"mobile_webview\"",
        FRONTEND_CAPABILITY,
        issues,
    );
    require_contains(
        &tests,
        "mobile binding selection hides desktop-only direct mesh capabilities",
        FRONTEND_CAPABILITY_TEST,
        issues,
    );
    require_contains(
        &read_text(root, "docs/app-runtime-boundaries.md")?,
        "GUI code should select bindings by declared capability",
        "docs/app-runtime-boundaries.md",
        issues,
    );
    Ok(())
}

const FRONTEND_CAPABILITY: &str = "apps/frontend/src/lib/api/gui-runtime-capabilities.ts";
const FRONTEND_API_INDEX: &str = "apps/frontend/src/lib/api/index.ts";
const FRONTEND_CAPABILITY_TEST: &str =
    "apps/frontend/test/workflow/workbench-gui-runtime-capabilities.test.ts";

fn list_manifest_paths(root: &Path) -> RunnerResult<Vec<String>> {
    let mut paths = fs::read_dir(root.join(MANIFEST_DIR))
        .map_err(|error| format!("failed to read {MANIFEST_DIR}: {error}"))?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json"))
        .map(|name| format!("{MANIFEST_DIR}/{name}"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn require_array(
    value: &Value,
    field_name: &str,
    context: &str,
    min: usize,
    issues: &mut Vec<String>,
) {
    if value
        .get(field_name)
        .and_then(Value::as_array)
        .is_none_or(|items| items.len() < min)
    {
        issues.push(format!(
            "{context}: {field_name} must be an array with at least {min} item(s)"
        ));
    }
}

fn require_string(value: &Value, field_name: &str, context: &str, issues: &mut Vec<String>) {
    if field(value, field_name).is_empty() {
        issues.push(format!(
            "{context}: {field_name} must be a non-empty string"
        ));
    }
}

fn require_contains(text: &str, needle: &str, context: &str, issues: &mut Vec<String>) {
    if !text.contains(needle) {
        issues.push(format!("{context}: missing {needle}"));
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn array_field<'a>(value: &'a Value, field_name: &str) -> Vec<&'a Value> {
    value
        .get(field_name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn field<'a>(value: &'a Value, field_name: &str) -> &'a str {
    value
        .get(field_name)
        .and_then(Value::as_str)
        .unwrap_or_default()
}
